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
//   * an ending posted for a session of the tree, named by its id, is applied
//     as `end_host_invoke_in` applies it;
//   * a post is taken before the run starts and applied at its first boundary,
//     and one taken after the run was stopped is counted as unseen;
//   * a post made before the owner puts an event of its own on the external
//     queue is taken before that event (W3C test 201's shape), and an ending
//     for a child session posted then is still applied.
//
// Fixture: tests/integration/inbox_probe.scxml, generated with the host-invoker
// declaration `x-sce-host` into src/integration/host_processor/ by
// scripts/regen_host_processor.sh. `taken` is what the machine took, in order,
// as `ping:<data>;`, `done:<data>;` and `failed:<data>;`. The child session's
// case drives sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml,
// as a_child_sessions_host_invocation_is_served_by_the_trees_host.rs does.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use sce_rust_runtime::{
    Engine, EngineInbox, HostInvokeEnding, HostInvokeEvent, HostInvokeResponse, IScriptEngine,
    InboxInput, InboxInputOf, InboxWaker, SceClock,
};
use sce_rust_tests::integration::host_processor::{
    InboxProbeEvent as Event, InboxProbePolicy as Policy,
};
use sce_rust_tests::integration::static_datamodel::static_invoke_ending_order_sm::{
    StaticInvokeEndingOrderActions, StaticInvokeEndingOrderEvent, StaticInvokeEndingOrderPolicy,
};
use sce_rust_tests::integration::static_datamodel::static_invoke_tree_sm::StaticInvokeTreePolicy;

/// The type the fixture was generated for (`HOST_INVOKER` in
/// scripts/regen_host_processor.sh).
const TYPE: &str = "x-sce-host";
/// The fixture's one host-run invocation.
const JOB: &str = "job";

type Input = InboxInputOf<Policy>;

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
        self.engine.initialize();
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
        },
    )
}

/// An ending of start `token` of `job` of the machine whose inbox takes it.
fn ending(token: u64, done: bool, data: &str) -> Input {
    ending_of(None, token, done, data)
}

/// `ping` with `data`, as the document's own event.
fn ping(data: &str) -> Input {
    InboxInput::external(Event::Ping, data, "")
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

// An ending that names a session of the tree is applied as `end_host_invoke_in`
// applies it: the root's own id reaches the root, and an id no session of the
// tree has ends nothing.
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

    assert!(post_from_another_thread(
        &machine.engine,
        ending_of(Some("no-such-session"), token, true, &quoted("elsewhere"))
    ));
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
        .raise_external(Event::Ping, &quoted("raised"), "");
    machine.engine.step();
    assert_eq!(
        machine.taken(),
        "ping:posted;ping:raised;",
        "the host's raise was taken ahead of a post that arrived before it"
    );
}

// An ending for a child session is applied through the root's policy, which
// the root's own code holds while it raises a child's report on the root; the
// drain that report asks for runs inside that code. The ending waits for the
// root's macrostep boundary and is applied there, so the child takes its
// `done.invoke.clock` at its next drive and tells its parent `stamped`.
#[test]
fn an_ending_for_a_child_session_posted_before_the_childs_report_is_applied() {
    // The ids and the completion
    // a_child_sessions_host_invocation_is_served_by_the_trees_host.rs uses.
    const ROOT: &str = "fixture-root";
    const WORKER: &str = "fixture-root.working.1.worker";
    const CLOCK: &str = "sce-host-clock-now";
    const NOW: &str = r#"{"error":"","utc_ms":1700000000000,"monotonic_ms":5}"#;

    let mut engine =
        Engine::new(StaticInvokeTreePolicy::new_with_session_id(ROOT).expect("an Nmtoken"));
    engine.set_clock(SceClock::Manual(0));
    let tokens: Arc<Mutex<Vec<u64>>> = Arc::default();
    let seen = Arc::clone(&tokens);
    engine.register_invoker(CLOCK, move |event: HostInvokeEvent| {
        if let HostInvokeEvent::Start(request) = event {
            seen.lock().expect("the starts").push(request.token);
        }
        None
    });
    engine.initialize();
    // `go`, which the root forwards to `worker` in one tick and `worker` takes in
    // the next: it arms its `report` 5 s on, enters `second` and starts the clock.
    engine.raise_external_by_name("go", "");
    engine.tick();
    engine.tick();
    let token = {
        let tokens = tokens.lock().expect("the starts");
        assert_eq!(tokens.len(), 1, "one start of the clock: {tokens:?}");
        tokens[0]
    };

    let inbox = engine.inbox();
    let ending = InboxInput::host_invoke_ending(
        Some(WORKER),
        HostInvokeEnding {
            processor_type: CLOCK,
            invoke_id: "clock",
            token,
            done: true,
            data: NOW,
        },
    );
    assert!(
        std::thread::spawn(move || inbox.post(ending).is_ok())
            .join()
            .expect("the posting thread"),
        "the inbox refused a post"
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
    const ROOT: &str = "fixture-root";
    const WORKER: &str = "fixture-root.working.1.worker";
    const CLOCK: &str = "sce-host-clock-now";
    const NOW: &str = r#"{"error":"","utc_ms":1700000000000,"monotonic_ms":5}"#;

    let act: Arc<Mutex<Option<HostAct>>> = Arc::default();
    let mut engine = Engine::new(
        StaticInvokeEndingOrderPolicy::new_with_session_id(ROOT, BeforeQuestion(Arc::clone(&act)))
            .expect("an Nmtoken"),
    );
    engine.set_clock(SceClock::Manual(0));
    // The clock records each start's token and does not answer, so the
    // invocation keeps running.
    let tokens: Arc<Mutex<Vec<u64>>> = Arc::default();
    let seen = Arc::clone(&tokens);
    engine.register_invoker(CLOCK, move |event: HostInvokeEvent| {
        if let HostInvokeEvent::Start(request) = event {
            seen.lock().expect("the starts").push(request.token);
        }
        None
    });
    engine.initialize();
    // Driven until `worker` has started its clock, a bounded number of times,
    // so a start that never comes fails the case rather than hangs it.
    for _ in 0..8 {
        if !tokens.lock().expect("the starts").is_empty() {
            break;
        }
        engine.tick();
    }
    let token = {
        let tokens = tokens.lock().expect("the starts");
        assert_eq!(tokens.len(), 1, "one start of the clock: {tokens:?}");
        tokens[0]
    };

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
            },
        );
        let taken = std::thread::spawn(move || inbox.post(ending).is_ok())
            .join()
            .expect("the posting thread");
        assert!(taken, "the inbox refused the ending posted for `worker`");
    }));

    engine.raise_external(StaticInvokeEndingOrderEvent::Ask, "", "");
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
