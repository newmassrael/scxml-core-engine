// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0084, ARCHITECTURE.md "Engine Threading": a Rust engine is confined to
// its owner by the type system, and what another thread has for it enters
// through its inbox. The per-runtime conformance case of the ADR's D3, with the
// generated C++ engine's (WhatAnotherThreadPostsEntersThroughTheInboxAotTest)
// and the Kotlin engine's beside it:
//
//   * many threads post endings and external events while the owner runs
//     macrosteps, and what the machine takes, and what its host is told, are
//     those of a single-threaded replay of the same posts;
//   * an ending posted for a start that was cancelled or started again is
//     refused by the token check, as a direct one is;
//   * an ending that names the machine's own session is applied by it, and one
//     naming a session the tree does not hold is refused when it is posted;
//   * a post is taken before the run starts and applied at its first boundary,
//     and one taken after the run was stopped is counted as unseen;
//   * a post made before the owner puts an event of its own on the external
//     queue is taken before that event (W3C test 201's shape);
//   * an ending that names a child session is posted to that child's inbox
//     (ARCHITECTURE.md "Engine Threading", item 3): the child takes it before
//     what its parent sends it afterwards, the root's waker is called, the
//     child applies it at the parent's next drive, a child resumed by a
//     restore is in the directory with its own children, and a child that
//     has ended is not.
//
// Fixture: tests/integration/inbox_probe.scxml, generated with the host-invoker
// declaration `x-sce-host` into src/integration/host_processor/ by
// scripts/regen_host_processor.sh. `taken` is what the machine took, in order,
// as `ping:<data>;`, `done:<data>;` and `failed:<data>;`. The child sessions'
// cases drive sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml,
// as a_child_sessions_host_invocation_is_served_by_the_trees_host.rs does, and
// static_invoke_ending_order.scxml beside it.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use sce_rust_runtime::saved_state::{SavedInvokes, SavedState};
use sce_rust_runtime::{
    BoundedInbox, Engine, EngineInbox, HostInvokeEnding, HostInvokeEvent, HostInvokeResponse,
    IScriptEngine, InboxInput, InboxInputOf, InboxRefusal, InboxWaker, SceClock, StatePolicy,
    StdHal,
};
use sce_rust_tests::integration::host_processor::{
    InboxProbeEvent as Event, InboxProbePolicy as Policy,
};
use sce_rust_tests::integration::static_datamodel::static_invoke_ending_order_sm::{
    StaticInvokeEndingOrderActions, StaticInvokeEndingOrderEvent, StaticInvokeEndingOrderPolicy,
};
use sce_rust_tests::integration::static_datamodel::static_invoke_tree_sm::{
    StaticInvokeTreePersist, StaticInvokeTreePolicy,
};

/// The type the fixture was generated for (`HOST_INVOKER` in
/// scripts/regen_host_processor.sh).
const TYPE: &str = "x-sce-host";
/// The fixture's one host-run invocation.
const JOB: &str = "job";

/// The session id the child sessions' cases give their root, as
/// a_child_sessions_host_invocation_is_served_by_the_trees_host.rs does.
const ROOT: &str = "fixture-root";
/// The session `worker` is in both trees: the root's id and the id its first
/// start is started under (`<state>.<start>.<invoke>`).
const WORKER: &str = "fixture-root.working.1.worker";
/// The standard host type `worker` asks.
const CLOCK: &str = "sce-host-clock-now";
/// A completion of the clock `worker`'s result record reads.
const NOW: &str = r#"{"error":"","utc_ms":1700000000000,"monotonic_ms":5}"#;
/// The wall-clock moment a tree is saved at.
const SAVED_AT_MS: u64 = 1_700_000_000_000;

type Input = InboxInputOf<Policy>;
type TreeInput = InboxInputOf<StaticInvokeTreePolicy>;

/// One machine and its host's half: what the invoker was asked, in order, as
/// `start:<token>` and `cancel:<token>`.
struct Machine {
    engine: Engine<Policy>,
    script_engine: Arc<dyn IScriptEngine>,
    host: Arc<Mutex<Vec<String>>>,
}

/// What the invoker does on a start, handed the inbox of the machine it serves,
/// and what it answers.
type OnStart = Box<dyn FnMut(&EngineInbox<Policy>) -> Option<HostInvokeResponse> + Send>;

impl Machine {
    /// Built and served, not yet run. The invoker answers nothing on a start,
    /// so the work runs past the call.
    fn built() -> Self {
        Self::built_starting_with(Box::new(|_: &EngineInbox<Policy>| None))
    }

    /// Built and served, not yet run, with an invoker that does `on_start` on
    /// each start and answers what it returns.
    fn built_starting_with(mut on_start: OnStart) -> Self {
        let script_engine: Arc<dyn IScriptEngine> = Arc::new(sce_rust_lua::LuaEngine::new());
        let mut engine = Engine::new(Policy::new(Arc::clone(&script_engine)));
        let inbox = engine.inbox();
        let host: Arc<Mutex<Vec<String>>> = Arc::default();
        let log = Arc::clone(&host);
        engine.register_invoker(TYPE, move |event: HostInvokeEvent| {
            let (line, answer) = match event {
                HostInvokeEvent::Start(request) => {
                    (format!("start:{}", request.token), on_start(&inbox))
                }
                HostInvokeEvent::Cancel(cancel) => (format!("cancel:{}", cancel.token), None),
            };
            log.lock().expect("the host's log").push(line);
            answer
        });
        Self {
            engine,
            script_engine,
            host,
        }
    }

    /// Entered and driven once on the calling thread, which owns it.
    fn booted() -> Self {
        let mut machine = Self::built();
        machine.boot();
        machine
    }

    fn boot(&mut self) {
        self.engine
            .initialize()
            .expect("the machine's script session is established");
        self.engine.step();
    }

    /// What the machine took, in order.
    fn taken(&self) -> String {
        sce_rust_runtime::helpers::datamodel_read::read_string(
            &*self.script_engine,
            self.engine.policy().session_id.as_deref(),
            "taken",
        )
        .expect("the fixture declares `taken` as a string")
    }

    /// What the host was asked, in order.
    fn host_log(&self) -> Vec<String> {
        self.host.lock().expect("the host's log").clone()
    }

    /// The token of the latest start of `job`.
    fn latest_token(&self) -> u64 {
        self.host_log()
            .iter()
            .rev()
            .find_map(|line| line.strip_prefix("start:"))
            .map(|token| token.parse::<u64>().expect("a token is a number"))
            .expect("`job` never started")
    }
}

/// `text` as the JSON string the machine reads back as `text` in `_event.data`.
fn quoted(text: &str) -> String {
    format!("\"{text}\"")
}

/// An ending of start `token` of `job` in the session of the tree whose id is
/// `session_id`, or of the machine whose inbox takes it when that is `None`: a
/// completion when `done`, a failure otherwise, with `data` as `_event.data`.
fn ending_of(session_id: Option<&str>, token: u64, done: bool, data: &str) -> Input {
    InboxInput::host_invoke_ending(
        session_id,
        HostInvokeEnding {
            processor_type: TYPE,
            invoke_id: JOB,
            token,
            done,
            data,
            origin: None,
        },
    )
}

/// An ending of start `token` of `job` of the machine whose inbox takes it.
fn ending(token: u64, done: bool, data: &str) -> Input {
    ending_of(None, token, done, data)
}

/// `ping` with `data`, as the document's own event.
fn ping(data: &str) -> Input {
    InboxInput::external(Event::Ping, data)
}

/// `ping` with `data`, named by text, as a host that knows no member names it.
fn ping_by_name(data: &str) -> Input {
    InboxInput::external_by_name("ping", data)
}

/// Post `input` from a thread of its own, which does not own `engine`; whether
/// the inbox took it. Joined before it returns, so the post happens before what
/// the caller does next.
fn post_from_another_thread(engine: &Engine<Policy>, input: Input) -> bool {
    let inbox = engine.inbox();
    std::thread::spawn(move || inbox.post(input).is_ok())
        .join()
        .expect("the posting thread")
}

/// One post of the run below, with what the machine records if it takes it.
struct Post {
    input: Input,
    recorded: String,
    is_ending: bool,
}

/// What producer `producer` posts as its `k`th input: external events, typed and
/// by name, and now and then an ending of `token`, completed or failed. Each data
/// names its producer and its place, so the record says which post it came from.
fn post_of(producer: usize, k: usize, token: u64) -> Post {
    let label = format!("p{producer}-{k}");
    let data = quoted(&label);
    match k % 7 {
        3 => Post {
            input: ending(token, true, &data),
            recorded: format!("done:{label};"),
            is_ending: true,
        },
        5 => Post {
            input: ending(token, false, &data),
            recorded: format!("failed:{label};"),
            is_ending: true,
        },
        1 => Post {
            input: ping_by_name(&data),
            recorded: format!("ping:{label};"),
            is_ending: false,
        },
        _ => Post {
            input: ping(&data),
            recorded: format!("ping:{label};"),
            is_ending: false,
        },
    }
}

/// What the machine must take from `posted`, in the order the inbox took it:
/// every event, and of the endings only the first, since it ends `job` and the
/// token check refuses the rest.
fn expected_from(posted: &[Post]) -> String {
    let mut ended = false;
    let mut expected = String::new();
    for post in posted {
        if post.is_ending {
            if ended {
                continue;
            }
            ended = true;
        }
        expected.push_str(&post.recorded);
    }
    expected
}

/// What the owner sleeps on: the posts the waker counted, and the producers
/// still running.
struct Wakes {
    count: Mutex<usize>,
    woken: Condvar,
}

// Eight threads post sixty inputs each while the owner, woken by the waker the
// host installed, drives the machine. The order the inbox took the posts in is
// recorded under one lock with the post, so it is the order of the inbox itself;
// the machine must take them in that order, and a machine of its own replaying
// the same posts on one thread must end where this one did.
#[test]
fn posts_from_many_threads_during_macrosteps_are_taken_as_a_single_threaded_replay_takes_them() {
    const PRODUCERS: usize = 8;
    const PER_PRODUCER: usize = 60;

    let mut machine = Machine::booted();
    let token = machine.latest_token();

    let wakes = Arc::new(Wakes {
        count: Mutex::new(0),
        woken: Condvar::new(),
    });
    let counter = Arc::clone(&wakes);
    let waker: InboxWaker = Arc::new(move || {
        *counter.count.lock().expect("the wake count") += 1;
        counter.woken.notify_one();
    });
    machine.engine.inbox().wake_with(Some(waker));

    let order: Mutex<Vec<Post>> = Mutex::new(Vec::new());
    let refused: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let running = AtomicUsize::new(PRODUCERS);
    let released = AtomicBool::new(false);

    std::thread::scope(|scope| {
        for producer in 0..PRODUCERS {
            let inbox = machine.engine.inbox();
            let (order, refused, running, released, wakes) =
                (&order, &refused, &running, &released, &wakes);
            scope.spawn(move || {
                while !released.load(Ordering::Acquire) {
                    std::thread::yield_now();
                }
                for k in 0..PER_PRODUCER {
                    let post = post_of(producer, k, token);
                    let mut posted = order.lock().expect("the order of the posts");
                    match inbox.post(post.input.clone()) {
                        Ok(()) => posted.push(post),
                        Err(_) => refused
                            .lock()
                            .expect("the refused posts")
                            .push(post.recorded),
                    }
                }
                running.fetch_sub(1, Ordering::AcqRel);
                // Taken and released so the owner, if it is waiting, sees the
                // count of running producers change.
                drop(wakes.count.lock().expect("the wake count"));
                wakes.woken.notify_one();
            });
        }
        released.store(true, Ordering::Release);

        // The owner: asleep until a post wakes it, then a macrostep, for as long
        // as any producer runs. Bounded, so a lost wake shows as a slow case and
        // not a hung one.
        let mut seen = 0;
        while running.load(Ordering::Acquire) > 0 {
            {
                let count = wakes.count.lock().expect("the wake count");
                let (count, _) = wakes
                    .woken
                    .wait_timeout_while(count, Duration::from_millis(100), |count| {
                        *count == seen && running.load(Ordering::Acquire) > 0
                    })
                    .expect("the wake count");
                seen = *count;
            }
            machine.engine.step();
        }
    });
    // What the last producer posted after the owner's last macrostep.
    machine.engine.step();

    let posted = order.into_inner().expect("the order of the posts");
    let refused = refused.into_inner().expect("the refused posts");
    assert!(
        refused.is_empty(),
        "an engine's own inbox refused {} posts",
        refused.len()
    );
    assert_eq!(posted.len(), PRODUCERS * PER_PRODUCER);
    assert_eq!(
        *wakes.count.lock().expect("the wake count"),
        posted.len(),
        "the waker was not called once for each post the inbox took"
    );
    assert_eq!(
        machine.taken(),
        expected_from(&posted),
        "the machine did not take the posts in the order the inbox took them"
    );
    // The first ending ended `job`, so the host was told of its start and nothing
    // else.
    assert_eq!(
        machine.host_log(),
        [format!("start:{token}")],
        "an ending was not applied, or a refused one was"
    );

    let mut replay = Machine::booted();
    assert_eq!(
        replay.latest_token(),
        token,
        "the replay's start has another token, so the run's endings are not its"
    );
    let inbox = replay.engine.inbox();
    for post in &posted {
        inbox
            .post(post.input.clone())
            .expect("the replay's inbox refused a post");
    }
    replay.engine.step();
    assert_eq!(
        replay.taken(),
        machine.taken(),
        "the run differs from a single-threaded replay of its posts"
    );
    assert_eq!(
        replay.host_log(),
        machine.host_log(),
        "the invocation differs from a single-threaded replay of the posts"
    );
}

// What a cancelled process sends is ignored, and a restarted `<invoke>` is a
// different process under the same id (the engine's token check). A posted
// ending is judged when the owner applies it, by the same check a direct one
// meets.
#[test]
fn a_posted_ending_of_a_cancelled_or_restarted_start_is_refused_as_a_direct_one_is() {
    let mut machine = Machine::booted();
    let first = machine.latest_token();
    machine.engine.process_event(Event::Leave);
    machine.engine.process_event(Event::Go);
    let second = machine.latest_token();
    assert_ne!(first, second, "a restart reused the first start's token");

    assert!(
        post_from_another_thread(&machine.engine, ending(first, true, &quoted("stale"))),
        "the inbox refused a post; the token is the owner's to judge"
    );
    machine.engine.step();
    assert_eq!(
        machine.taken(),
        "",
        "the cancelled start's ending was taken"
    );

    assert!(post_from_another_thread(
        &machine.engine,
        ending(second, true, &quoted("fresh"))
    ));
    assert_eq!(
        machine.taken(),
        "",
        "a post was applied before the owner reached a macrostep boundary"
    );
    machine.engine.step();
    assert_eq!(machine.taken(), "done:fresh;");

    // Ended once, so a later ending of the same start finds nothing to end.
    assert!(post_from_another_thread(
        &machine.engine,
        ending(second, false, &quoted("late"))
    ));
    machine.engine.step();
    assert_eq!(machine.taken(), "done:fresh;", "the same start ended twice");
    assert_eq!(
        machine.host_log(),
        [
            format!("start:{first}"),
            format!("cancel:{first}"),
            format!("start:{second}"),
        ]
    );
}

// An ending that names a session of the tree is resolved when it is posted: the
// root's own id reaches the root, and an id no session of the tree has is
// refused then, with the input, and ends nothing.
#[test]
fn a_posted_ending_naming_a_session_of_the_tree_is_applied_in_that_session() {
    let mut machine = Machine::booted();
    let token = machine.latest_token();
    let session = machine
        .engine
        .policy()
        .session_id
        .clone()
        .expect("a generated root has a session id from its construction");

    let inbox = machine.engine.inbox();
    let elsewhere = ending_of(Some("no-such-session"), token, true, &quoted("elsewhere"));
    let answer = std::thread::spawn(move || inbox.post(elsewhere).err())
        .join()
        .expect("the posting thread");
    assert!(
        matches!(answer, Some(InboxRefusal::UnknownSession(_))),
        "a post naming a session the tree does not hold was answered {answer:?}"
    );
    machine.engine.step();
    assert_eq!(
        machine.taken(),
        "",
        "an ending for a session the tree does not hold was applied"
    );

    assert!(post_from_another_thread(
        &machine.engine,
        ending_of(Some(session.as_str()), token, true, &quoted("here"))
    ));
    machine.engine.step();
    assert_eq!(machine.taken(), "done:here;");
    assert_eq!(machine.host_log(), [format!("start:{token}")]);
}

/// A bounded inbox for the case of its own session; each case declares its own,
/// as cargo runs the cases on parallel threads.
static BOUNDED_OWN: BoundedInbox<Policy, 4, StdHal> = BoundedInbox::new();
/// A bounded inbox attached to one engine and then to another.
static BOUNDED_MOVED: BoundedInbox<Policy, 4, StdHal> = BoundedInbox::new();

/// The session id `machine`'s root was built under.
fn session_of(machine: &Machine) -> String {
    machine
        .engine
        .policy()
        .session_id
        .clone()
        .expect("a generated root has a session id from its construction")
}

// A bounded inbox reaches only the engine it is attached to, and resolves an
// ending's address when it is posted, as the engine's own inbox does: an ending
// naming that engine's session is taken and applied, and one naming any other
// session is refused then, with the input, and not taken to be dropped later.
#[test]
fn a_bounded_inbox_takes_an_ending_naming_its_engines_session_and_refuses_another() {
    let mut machine = Machine::booted();
    let token = machine.latest_token();
    let session = session_of(&machine);
    machine.engine.attach_inbox(&BOUNDED_OWN);

    let answer = BOUNDED_OWN.post(ending_of(
        Some("no-such-session"),
        token,
        true,
        &quoted("elsewhere"),
    ));
    assert!(
        matches!(answer, Err(InboxRefusal::UnknownSession(_))),
        "a bounded inbox answered {answer:?} to an ending naming a session it does not reach"
    );
    let answer = BOUNDED_OWN.post(ending_of(
        Some(session.as_str()),
        token,
        true,
        &quoted("here"),
    ));
    assert!(
        answer.is_ok(),
        "a bounded inbox answered {answer:?} to an ending naming its engine's session"
    );
    machine.engine.step();
    assert_eq!(
        machine.taken(),
        "done:here;",
        "the ending the bounded inbox took was not applied, or the refused one was"
    );
}

// Attaching is the owner's operation that tells a bounded inbox whose session
// it serves: attached to another engine, it takes an ending naming that
// engine's session, and refuses one naming the engine it served before.
#[test]
fn a_bounded_inbox_attached_to_another_engine_answers_for_that_engines_session() {
    let mut first = Machine::booted();
    first.engine.attach_inbox(&BOUNDED_MOVED);
    let before = session_of(&first);
    drop(first);

    let mut second = Machine::booted();
    second.engine.attach_inbox(&BOUNDED_MOVED);
    let token = second.latest_token();
    let now = session_of(&second);
    assert_ne!(before, now, "two machines were built under one drawn id");

    let answer = BOUNDED_MOVED.post(ending_of(
        Some(before.as_str()),
        token,
        true,
        &quoted("before"),
    ));
    assert!(
        matches!(answer, Err(InboxRefusal::UnknownSession(_))),
        "a bounded inbox answered {answer:?} to an ending naming the engine it served before"
    );
    let answer = BOUNDED_MOVED.post(ending_of(Some(now.as_str()), token, true, &quoted("now")));
    assert!(
        answer.is_ok(),
        "a bounded inbox answered {answer:?} to an ending naming the engine it serves now"
    );
    second.engine.step();
    assert_eq!(second.taken(), "done:now;");
}

// The inbox is open from the moment the engine is built, so what is posted before
// the run starts is applied at its first boundary. It stays open after the run is
// stopped, and what is posted then is applied where the owner next reaches the
// end of its loop, as a direct raise would be: counted as unseen, not dropped.
#[test]
fn a_post_before_the_run_is_taken_by_it_and_one_after_the_stop_is_counted_as_unseen() {
    let mut machine = Machine::built();
    assert!(
        post_from_another_thread(&machine.engine, ping(&quoted("early"))),
        "the inbox of an engine not yet run refused a post"
    );
    machine.boot();
    assert_eq!(
        machine.taken(),
        "ping:early;",
        "what was posted before the run was not taken by it"
    );

    machine.engine.stop();
    let unseen = machine.engine.unseen_external_events();
    assert!(
        post_from_another_thread(&machine.engine, ping(&quoted("late"))),
        "the inbox of a stopped engine refused a post"
    );
    machine.engine.step();
    assert_eq!(machine.engine.unseen_external_events(), unseen + 1);
    assert_eq!(machine.engine.last_unseen_event(), Some(Event::Ping));
}

// W3C test 201's shape (docs/adr/0084, D2 as amended at step 8). Inside the
// owner's macrostep a host call waits for a thread of its own to post `ping`,
// and the owner then puts an event of its own on the external queue: the
// `done.invoke.job` of the start the host completed at once. The post arrived
// first, so it is taken first, and not after the event it came before.
#[test]
fn a_post_made_before_the_owner_queues_its_own_event_is_taken_before_it() {
    let mut machine = Machine::built_starting_with(Box::new(|inbox: &EngineInbox<Policy>| {
        let inbox = inbox.clone();
        let posted = std::thread::spawn(move || inbox.post(ping(&quoted("posted"))).is_ok())
            .join()
            .expect("the posting thread");
        assert!(posted, "the inbox refused a post");
        Some(HostInvokeResponse {
            done_data: Some(quoted("own")),
            ..HostInvokeResponse::default()
        })
    }));
    machine.boot();
    assert_eq!(
        machine.taken(),
        "ping:posted;done:own;",
        "the owner's own event was taken ahead of a post that arrived before it"
    );
}

// A host's own raise on the owner puts an event of the owner's on the external
// queue as well: what another thread posted before it is taken before it.
#[test]
fn a_post_made_before_the_hosts_own_raise_is_taken_before_it() {
    let mut machine = Machine::booted();
    assert!(post_from_another_thread(
        &machine.engine,
        ping(&quoted("posted"))
    ));
    machine
        .engine
        .raise_external(Event::Ping, &quoted("raised"));
    machine.engine.step();
    assert_eq!(
        machine.taken(),
        "ping:posted;ping:raised;",
        "the host's raise was taken ahead of a post that arrived before it"
    );
}

/// The tokens of the starts of the clock, in order.
type Starts = Arc<Mutex<Vec<u64>>>;

/// Serve the clock on `engine`, the root of a tree: each start's token is
/// recorded and none is answered, so each invocation keeps running.
fn serve_the_clock<P: StatePolicy>(engine: &mut Engine<P>) -> Starts {
    let starts = Starts::default();
    let seen = Arc::clone(&starts);
    engine.register_invoker(CLOCK, move |event: HostInvokeEvent| {
        if let HostInvokeEvent::Start(request) = event {
            seen.lock().expect("the starts").push(request.token);
        }
        None
    });
    starts
}

/// The token of the one start of the clock so far.
fn the_one_start(starts: &Starts) -> u64 {
    let starts = starts.lock().expect("the starts");
    assert_eq!(starts.len(), 1, "one start of the clock: {starts:?}");
    starts[0]
}

/// `static_invoke_tree` built under `ROOT`, served, and driven until `worker`
/// has started its clock, with the token of that start: `go`, which the root
/// forwards to `worker` in one tick and `worker` takes in the next, arming its
/// `report` 5 s on, entering `second` and starting the clock.
fn tree_with_its_clock_running() -> (Engine<StaticInvokeTreePolicy>, u64) {
    let mut engine =
        Engine::new(StaticInvokeTreePolicy::new_with_session_id(ROOT).expect("an Nmtoken"));
    engine.set_clock(SceClock::Manual(0));
    let starts = serve_the_clock(&mut engine);
    let Ok(()) = engine.initialize();
    engine.raise_external_by_name("go", "");
    engine.tick();
    engine.tick();
    let token = the_one_start(&starts);
    (engine, token)
}

/// The completion of start `token` of the clock of session `session_id`.
fn clock_ending(session_id: &str, token: u64) -> TreeInput {
    InboxInput::host_invoke_ending(
        Some(session_id),
        HostInvokeEnding {
            processor_type: CLOCK,
            invoke_id: "clock",
            token,
            done: true,
            data: NOW,
            origin: None,
        },
    )
}

/// Post `input` to the inbox of `engine`, a root, from a thread of its own,
/// joined before it returns: the refusal, if the inbox refused it.
fn post_to_the_tree(
    engine: &Engine<StaticInvokeTreePolicy>,
    input: TreeInput,
) -> Option<InboxRefusal<TreeInput>> {
    let inbox = engine.inbox();
    std::thread::spawn(move || inbox.post(input).err())
        .join()
        .expect("the posting thread")
}

// An ending posted for a child session goes to the child's inbox, and the child
// applies it at its own boundaries. The tick that releases the child's due
// `report` drives the child too, so the child takes `done.invoke.clock` in that
// drive or the next and tells its parent `stamped`.
#[test]
fn an_ending_for_a_child_session_posted_before_the_childs_report_is_applied() {
    let (mut engine, token) = tree_with_its_clock_running();
    let refused = post_to_the_tree(&engine, clock_ending(WORKER, token));
    assert!(
        refused.is_none(),
        "the inbox refused an ending for `worker`: {refused:?}"
    );

    // The report comes due, and the root takes it in this turn.
    engine.advance_time_ms(5_000);
    assert_eq!(
        engine.policy().reports(),
        1,
        "the child's report did not reach the root"
    );
    engine.tick();
    assert_eq!(
        engine.policy().stamps(),
        1,
        "the ending posted for the child session was never applied"
    );
}

/// What `beforeQuestion` does, put in place by the test once the machine runs.
type HostAct = Box<dyn FnMut() + Send>;

/// The host of `static_invoke_ending_order`: its one action, `beforeQuestion`,
/// runs what the test put in the slot it shares with it, if anything.
struct BeforeQuestion(Arc<Mutex<Option<HostAct>>>);

impl StaticInvokeEndingOrderActions for BeforeQuestion {
    fn before_question(&mut self) {
        if let Some(act) = self.0.lock().expect("the host action").as_mut() {
            act();
        }
    }
}

// docs/adr/0084, D2 as amended at step 10. Inside the parent's macrostep that
// takes `ask`, the host posts the ending of `worker`'s `clock`, naming `worker`,
// from a thread of its own (`beforeQuestion`), and then the parent sends
// `question` to `worker`. The post came first, so `worker` takes
// `done.invoke.clock` before `question`, and the parent hears `stamped` before
// `answer`.
//
// Fixture: sce-build/tests/fixtures/static_datamodel/static_invoke_ending_order.scxml.
#[test]
fn an_ending_posted_for_a_child_before_its_parents_send_to_it_is_taken_first() {
    let act: Arc<Mutex<Option<HostAct>>> = Arc::default();
    let mut engine = Engine::new(
        StaticInvokeEndingOrderPolicy::new_with_session_id(ROOT, BeforeQuestion(Arc::clone(&act)))
            .expect("an Nmtoken"),
    );
    engine.set_clock(SceClock::Manual(0));
    let starts = serve_the_clock(&mut engine);
    let Ok(()) = engine.initialize();
    // Driven until `worker` has started its clock, a bounded number of times,
    // so a start that never comes fails the case rather than hangs it.
    for _ in 0..8 {
        if !starts.lock().expect("the starts").is_empty() {
            break;
        }
        engine.tick();
    }
    let token = the_one_start(&starts);

    let inbox = engine.inbox();
    *act.lock().expect("the host action") = Some(Box::new(move || {
        let inbox = inbox.clone();
        let ending = InboxInput::host_invoke_ending(
            Some(WORKER),
            HostInvokeEnding {
                processor_type: CLOCK,
                invoke_id: "clock",
                token,
                done: true,
                data: NOW,
                origin: None,
            },
        );
        let taken = std::thread::spawn(move || inbox.post(ending).is_ok())
            .join()
            .expect("the posting thread");
        assert!(taken, "the inbox refused the ending posted for `worker`");
    }));

    engine.raise_external(StaticInvokeEndingOrderEvent::Ask, "");
    for _ in 0..4 {
        engine.tick();
    }

    let heard = (engine.policy().stamped_at(), engine.policy().answer_at());
    assert_eq!(
        heard,
        (1, 2),
        "the parent heard `stamped` and `answer` in the places {heard:?}, so `worker` took \
         the parent's `question` before the ending the host had posted for it first"
    );
}

// One owner drives the tree, and a post to any of its sessions wakes it through
// the waker the host installed on the root. The owner, woken by the post for
// `worker` alone, drives the tree as it always does, and that drive reaches
// `worker`, which applies the ending at its own boundary and tells its parent
// `stamped`.
#[test]
fn an_ending_posted_for_a_child_session_wakes_the_roots_owner_and_its_drive_applies_it() {
    let (mut engine, token) = tree_with_its_clock_running();
    let wakes = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&wakes);
    let waker: InboxWaker = Arc::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    engine.inbox().wake_with(Some(waker));

    let refused = post_to_the_tree(&engine, clock_ending(WORKER, token));
    assert!(
        refused.is_none(),
        "the inbox refused an ending for `worker`: {refused:?}"
    );
    assert_eq!(
        wakes.load(Ordering::SeqCst),
        1,
        "a post for a child session did not call the waker the host installed on the root"
    );
    assert_eq!(
        engine.policy().stamps(),
        0,
        "the ending was applied before the owner drove the tree"
    );
    engine.tick();
    assert_eq!(
        engine.policy().stamps(),
        1,
        "the owner's drive did not reach the child whose inbox held the post"
    );
}

// A child that has ended has left the tree's directory, so an ending posted for
// it is refused when it is posted, and the refusal says so, rather than being
// taken and dropped.
#[test]
fn an_ending_for_a_child_session_that_has_ended_is_refused_when_it_is_posted() {
    let (mut engine, token) = tree_with_its_clock_running();
    // `rest` leaves `working`, which stops `worker` and cancels its clock.
    engine.raise_external_by_name("rest", "");
    engine.tick();

    let refused = post_to_the_tree(&engine, clock_ending(WORKER, token));
    assert!(
        matches!(refused, Some(InboxRefusal::UnknownSession(_))),
        "an ending for a child session that has ended was answered {refused:?}"
    );
}

// A restore resumes `worker`, and the grandchild `worker` runs, before the host
// holds the tree. Each enters the restored tree's directory: `worker` when the
// root resumes it, and the grandchild when `worker` is handed the root's tree.
// An ending posted for either is taken, and `worker` applies its own.
#[test]
fn a_restored_trees_resumed_sessions_are_in_its_directory() {
    let (engine, _) = tree_with_its_clock_running();
    let saved = engine
        .save_at(SAVED_AT_MS)
        .unwrap_or_else(|refusal| panic!("saves with the clock running: {refusal}"));
    let SavedInvokes::Resumed(children) = &saved.invokes else {
        panic!("a format-2 tree");
    };
    let SavedInvokes::Resumed(grandchildren) = &children[0].session.invokes else {
        panic!("a format-2 child");
    };
    let grandchild = format!("{WORKER}.{}", grandchildren[0].invoke_id);

    let mut restored = Engine::<StaticInvokeTreePolicy>::restore_with(
        StaticInvokeTreePolicy::new(),
        &SavedState::from_json(&saved.to_json()).expect("reads"),
        SceClock::Manual(0),
        SAVED_AT_MS,
    )
    .unwrap_or_else(|refusal| panic!("restores: {refusal}"));
    let starts = serve_the_clock(&mut restored);
    // The parent's first drive of `worker` starts its clock again.
    restored.tick();
    let token = the_one_start(&starts);

    let refused = post_to_the_tree(&restored, clock_ending(&grandchild, token));
    assert!(
        refused.is_none(),
        "an ending for the resumed grandchild '{grandchild}' was refused: {refused:?}"
    );
    let refused = post_to_the_tree(&restored, clock_ending(WORKER, token));
    assert!(
        refused.is_none(),
        "an ending for the resumed `worker` was refused: {refused:?}"
    );
    restored.tick();
    assert_eq!(
        restored.policy().stamps(),
        1,
        "the resumed `worker` did not apply the ending posted for it"
    );
}
