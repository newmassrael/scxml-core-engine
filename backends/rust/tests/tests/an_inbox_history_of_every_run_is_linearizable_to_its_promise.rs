// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0147, D3: the door of docs/adr/0084 is a concurrent object, and every
// run of it a history. These cases run the Rust engine's inboxes on real threads,
// record every post each poster made and every item the owner put on a session's
// external queue, with the end of each session's run, and hold the record
// linearizable to the promise docs/formal/InboxService.tla states, with the
// checker every runtime's history is judged by (sce_build::inbox_history):
//
//   * the `std` profile's own inboxes of a tree of two sessions, a root and a
//     child that joined its tree, posted to from several threads while the
//     owner drives both, puts events of its own on their queues and ends them;
//     posts naming a session the tree does not hold are refused;
//   * the bare-metal profile's bounded inbox a deployment declares, attached
//     to one session, refusing posts when it holds its capacity;
//   * a recorded run with one of the owner's enqueues taken out, and one with
//     two inputs of one poster entered in the reverse of their posts, are
//     refused, so the cases above fail when the record or the runtime loses
//     an input or reorders two.
//
// What enters a session's external queue is told by the queue itself: the
// machine below names `RecordingQueue` as its external queue type (a policy
// supplies its queues, docs/adr/0076), and the engine hands it each item's
// payload at the moment the item is put on it, whatever put it there: the drain
// at a macrostep boundary, the drain before an event of the owner's own, the
// owner's own raise, or the drain where a run that has ended applies what its
// inbox still holds. The payload says which item it is and whether the owner
// raised it itself, so the record names every item by the value its poster or
// the owner gave it. The drain, the order it keeps and the inboxes are the
// runtime's; only the queue's storage is this file's.
//
// Neither Rust inbox closes when a run ends (backends/rust/runtime/src/inbox.rs),
// so every history here says `closes: false`: a post taken after a session's
// end is applied where the owner next reaches the end of its loop, and its
// enqueue is recorded there.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use sce_build::inbox_history::{
    check, Delivery, History, OwnerCall, OwnerOperation, Post, Refusal, Source, Verdict,
};
use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::{
    state_scratch, state_words, BoundedInbox, DocumentTables, Engine, EngineInbox, EngineState,
    EventId, EventQueue, EventQueueMut, EventQueueStorage, EventWithMetadata, HistoryId,
    HostInvokeEnding, InboxInput, InboxRefusal, NoHistory, QueuedEvent, RunStore, StateId,
    StateKind, StatePolicy, StateRow, StdHal, Target, TransitionId, TransitionRow, Workspace,
};

// ─────────────────────────────────────────────────────────────
// The clock and the record
// ─────────────────────────────────────────────────────────────

/// The one clock every thread of a run reads, immediately before and
/// immediately after each operation it records. One read-modify-write chain on
/// one atomic orders the readings with the calls between them, so "returned
/// before invoked" in the record means it in the run, and no two readings are
/// equal.
struct Clock(AtomicU64);

impl Clock {
    fn new() -> Self {
        Clock(AtomicU64::new(0))
    }

    fn tick(&self) -> u64 {
        self.0.fetch_add(1, Ordering::AcqRel) + 1
    }
}

/// What the owner thread records: its operations, in its program order, read
/// from the run's clock. Installed on the owner thread for the length of a run;
/// the posters never touch a session's external queue, so they never reach it.
struct Recording {
    clock: Arc<Clock>,
    owner: Vec<OwnerOperation>,
}

thread_local! {
    static RECORDING: RefCell<Option<Recording>> = const { RefCell::new(None) };
}

/// Record the owner's operations on this thread from now on, against `clock`.
fn start_recording(clock: Arc<Clock>) {
    RECORDING.with(|recording| {
        *recording.borrow_mut() = Some(Recording {
            clock,
            owner: Vec::new(),
        });
    });
}

/// The owner's operations this thread recorded since `start_recording`, and
/// no recording from now on.
fn finish_recording() -> Vec<OwnerOperation> {
    RECORDING.with(|recording| {
        recording
            .borrow_mut()
            .take()
            .expect("the owner records from the start of the run")
            .owner
    })
}

/// End `session` by `end`, the owner's call that ends its run, recorded with
/// the clock read immediately around it.
fn record_end(session: u64, end: impl FnOnce()) {
    let clock = RECORDING.with(|recording| {
        Arc::clone(
            &recording
                .borrow()
                .as_ref()
                .expect("the owner records from the start of the run")
                .clock,
        )
    });
    let invoked = clock.tick();
    end();
    let returned = clock.tick();
    RECORDING.with(|recording| {
        recording
            .borrow_mut()
            .as_mut()
            .expect("the owner records from the start of the run")
            .owner
            .push(OwnerOperation {
                call: OwnerCall::End { session },
                invoked,
                returned,
            });
    });
}

/// The payload of every event of the machine: the item it is, a value no other
/// input or own event of the run has, and whether the owner raised it itself
/// (an own event) rather than a poster posting it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Tag {
    value: u64,
    own: bool,
}

/// The external queue of session `SESSION`: the runtime's own queue, which
/// records each item at the moment the engine puts it there, with the clock read
/// immediately before and after. The engine hands a queue an event's payload
/// and then its number (`EngineState::enqueue_queued`), and nothing but the
/// owner reads the queue, so the item has entered when its payload has.
#[derive(Default)]
struct RecordingQueue<const SESSION: u64> {
    inner: EventQueue<Tag>,
}

impl<const SESSION: u64> EventQueueStorage<Tag> for RecordingQueue<SESSION> {
    const EMPTY: Self = RecordingQueue {
        inner: EventQueue::new(),
    };

    fn events(&mut self) -> EventQueueMut<'_> {
        EventQueueStorage::events(&mut self.inner)
    }

    fn has_events(&self) -> bool {
        EventQueueStorage::has_events(&self.inner)
    }

    fn queued(&self) -> impl Iterator<Item = &QueuedEvent> {
        EventQueueStorage::queued(&self.inner)
    }

    fn push_payload(&mut self, payload: Tag) {
        RECORDING.with(|recording| {
            let mut recording = recording.borrow_mut();
            let Some(recording) = recording.as_mut() else {
                EventQueueStorage::push_payload(&mut self.inner, payload);
                return;
            };
            let invoked = recording.clock.tick();
            EventQueueStorage::push_payload(&mut self.inner, payload);
            let returned = recording.clock.tick();
            recording.owner.push(OwnerOperation {
                call: OwnerCall::Enqueue {
                    session: SESSION,
                    item: payload.value,
                    source: if payload.own {
                        Source::Own
                    } else {
                        Source::Posted
                    },
                },
                invoked,
                returned,
            });
        });
    }

    fn pop_payload(&mut self) -> Option<Tag> {
        EventQueueStorage::pop_payload(&mut self.inner)
    }
}

// ─────────────────────────────────────────────────────────────
// The machine
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum St {
    S,
}

// Declared in the order of `EVENTS`, so a variant's discriminant is its
// `EventId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Ev {
    Null,
    Ping,
}

/// `s` on `ping`, targetless.
const S_PING: TransitionId = TransitionId(0);

/// The session ids of the two sessions of the tree, by the number the history
/// names each by: the root, 0, and the child, 1.
const SESSION_IDS: [&str; 2] = ["inbox-history-root", "inbox-history-child"];
/// The number the history names the session the tree does not hold by.
const ABSENT: u64 = 2;
/// The id a post that names the session the tree does not hold gives.
const ABSENT_ID: &str = "inbox-history-absent";

/// The machine of session `SESSION` of the tree: one state, `s`, whose one
/// targetless transition takes `ping`. What it takes is no part of the
/// record; the queue it takes it from is.
struct HistoryPolicy<const SESSION: u64>;

impl<const SESSION: u64> StatePolicy for HistoryPolicy<SESSION> {
    type State = St;
    type Event = Ev;
    type History = NoHistory;
    type Payload = Tag;
    // docs/adr/0124: the payload an event carried, read where the engine
    // holds it.
    type EventData<'e> = &'e Tag;
    type Hal = StdHal;
    type InternalQueue = EventQueue<Tag>;
    type ExternalQueue = RecordingQueue<SESSION>;
    type ScheduledSendId = sce_rust_runtime::SceString;
    // No delayed send, no `<cancel>`, no `<donedata>`.
    type Scheduler = sce_rust_runtime::NoDelayedSends;
    type DoneData = sce_rust_runtime::NoDoneData;
    // One state and no history: one transition is enabled at a time.
    type Workspace = Workspace<{ state_scratch(1) }, 1, 6, 0>;
    type RunStore = RunStore<{ state_words(1) }, 0, 0>;
    type Configuration = BoundedVec<St, 1>;
    type StartFailure = core::convert::Infallible;

    // One child of the `<scxml>` element with a targetless transition that has
    // content; the document's initial transition, last, enters it.
    const TABLES: &'static DocumentTables<'static> = &DocumentTables {
        states: &[StateRow::new(None, StateKind::Atomic, StateId(0), None)],
        histories: &[],
        transitions: &[
            // s: ping, targetless
            TransitionRow::new(Some(StateId(0)), Target::NONE, false, true),
            // <scxml initial="s">
            TransitionRow::new(None, Target::state(StateId(0)), false, false),
        ],
        targets: &[],
        initial: TransitionId(1),
        default_state: StateId(St::S as u16),
        null_event: EventId(0),
        error_events: &[],
        // Appendix D's sets computed as the appendix writes them, which serves
        // any document (docs/adr/0125).
        walk: &sce_rust_runtime::GENERAL_WALK,
        entry_rows: &[],
        entry_paths: &[],
        history: None,
        parallel: None,
        candidates: None,
        selection: None,
    };
    const STATES: &'static [St] = &[St::S];
    const HISTORIES: &'static [NoHistory] = &[];
    const EVENTS: &'static [Ev] = &[Ev::Null, Ev::Ping];

    fn state_id(state: Self::State) -> StateId {
        StateId(state as u16)
    }

    fn history_id(history: Self::History) -> HistoryId {
        match history {}
    }

    fn event_id(event: Self::Event) -> EventId {
        EventId(event as u16)
    }

    fn null_event() -> Self::Event {
        Ev::Null
    }

    fn get_event_name(event: Self::Event) -> &'static str {
        match event {
            Ev::Null => "",
            Ev::Ping => "ping",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "ping" => Some(Ev::Ping),
            _ => None,
        }
    }

    fn get_state_name(_state: Self::State) -> &'static str {
        "s"
    }

    fn get_state_from_name(name: &str) -> Option<Self::State> {
        match name {
            "s" => Some(St::S),
            _ => None,
        }
    }

    fn event_data<'a>(
        held: &'a sce_rust_runtime::CurrentEventData<'_, Self>,
    ) -> Self::EventData<'a> {
        match held {
            sce_rust_runtime::CurrentEventData::Queued(payload) => payload,
            sce_rust_runtime::CurrentEventData::Lent(data) => *data,
        }
    }

    // A session of the tree is named by its own id, so the child is entered in
    // the tree's directory under one the root does not share.
    fn own_session_id(&self) -> &str {
        SESSION_IDS[SESSION as usize]
    }

    // `_is_default_entry` is unused: this policy has no compound states.
    fn execute_entry_actions(
        &mut self,
        _state: StateId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
        _is_default_entry: bool,
    ) {
    }

    fn execute_exit_actions(
        &mut self,
        _state: StateId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) {
    }

    fn first_enabled_transition(
        &mut self,
        state: StateId,
        event: EventId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) -> Option<TransitionId> {
        match (Self::state_of(state), Self::event_of(event)) {
            (St::S, Ev::Ping) => Some(S_PING),
            _ => None,
        }
    }

    fn execute_transition_content(
        &mut self,
        _transition: TransitionId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) {
    }
}

/// `ping`, carrying the input `value` a poster gives it.
fn posted(value: u64) -> InboxInput<Ev, Tag> {
    let mut event: EventWithMetadata<Ev, Tag> = EventWithMetadata::new(Ev::Ping);
    event.payload = Tag { value, own: false };
    InboxInput::ExternalEvent(event)
}

/// An ending of a host-run invocation of the session the tree does not hold:
/// what a post naming that session posts. No run here starts an invocation,
/// so were it taken it would raise nothing, and its input would never enter.
fn naming_the_absent_session() -> InboxInput<Ev, Tag> {
    InboxInput::host_invoke_ending(
        Some(ABSENT_ID),
        HostInvokeEnding {
            processor_type: "x-inbox-history",
            invoke_id: "none",
            token: 0,
            done: true,
            data: "",
            origin: None,
        },
    )
}

/// Raise the own event `value` on `engine`, as the host's own raise on the
/// owner does.
fn raise_own<const SESSION: u64>(engine: &mut Engine<HistoryPolicy<SESSION>>, value: u64) {
    engine.raise_external_typed(Ev::Ping, Tag { value, own: true });
}

/// What the inbox answered a post, as the history says it. The engine of every
/// session a run posts to lives until its posters are joined, so no post is
/// refused because its engine is gone.
fn delivery<I>(answer: Result<(), InboxRefusal<I>>) -> Delivery {
    match answer {
        Ok(()) => Delivery::Taken,
        Err(InboxRefusal::Full(_)) => Delivery::Refused(Refusal::Full),
        Err(InboxRefusal::UnknownSession(_)) => Delivery::Refused(Refusal::Unknown),
        Err(InboxRefusal::EngineGone(_)) => panic!(
            "a post was refused because its engine was gone, while the run keeps every engine \
             until its posters are joined"
        ),
    }
}

/// Post `input` with `post`, recorded as a post of input `value` to
/// `session` with the clock read immediately around it.
fn record_post<I>(
    clock: &Clock,
    session: u64,
    value: u64,
    post: impl FnOnce() -> Result<(), InboxRefusal<I>>,
) -> Post {
    let invoked = clock.tick();
    let answer = post();
    let returned = clock.tick();
    Post {
        session,
        input: value,
        outcome: delivery(answer),
        invoked,
        returned,
    }
}

// ─────────────────────────────────────────────────────────────
// The runs
// ─────────────────────────────────────────────────────────────

/// Threads that post, beside the owner.
const POSTERS: usize = 4;
/// Posts each poster makes in one run.
const PER_POSTER: u64 = 120;
/// Own events the owner raises at most on each session in one run.
const OWN_PER_SESSION: u64 = 40;
/// Runs of each kind of inbox: the schedules that matter are a small fraction
/// of those a machine gives.
const RUNS: usize = 4;
/// The first value of the owner's own events, past every input a poster posts.
const OWN_BASE: u64 = 1_000_000;

/// The input poster `poster` posts as its `k`th, `posts` being how many each
/// poster makes: a value no other post of the run has.
fn input_of(poster: usize, k: u64, posts: u64) -> u64 {
    poster as u64 * posts + k + 1
}

/// One run of the `std` profile's inboxes: a root and a child of its tree,
/// each with its own inbox, posted to by `POSTERS` threads through the handles
/// of both, which now and then post an input naming a session the tree does
/// not hold. The owner drives both sessions while the posters run, raises
/// events of its own on each, and ends the child once half the posts are made;
/// once the posters are joined it applies what is left and ends the root.
fn record_std_run(clock: &Arc<Clock>) -> History {
    start_recording(Arc::clone(clock));
    let clock: &Clock = clock;
    let mut root = Engine::new(HistoryPolicy::<0>);
    let mut child = Engine::new(HistoryPolicy::<1>);
    // The child joins the root's tree before its first macrostep, as a parent
    // hands its tree to each child it starts.
    root.share_tree_inboxes_with(&mut child);
    let Ok(()) = root.initialize();
    let Ok(()) = child.initialize();

    let root_inbox: EngineInbox<HistoryPolicy<0>> = root.inbox();
    let child_inbox: EngineInbox<HistoryPolicy<1>> = child.inbox();
    // An event of the owner's own on each session before any post, so every run
    // enters one on each.
    raise_own(&mut root, OWN_BASE);
    raise_own(&mut child, OWN_BASE + 1);

    let made = AtomicUsize::new(0);
    let running = AtomicUsize::new(POSTERS);
    let total = POSTERS * PER_POSTER as usize;

    let posters: Vec<Vec<Post>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..POSTERS)
            .map(|poster| {
                let (root_inbox, child_inbox) = (root_inbox.clone(), child_inbox.clone());
                let (made, running) = (&made, &running);
                scope.spawn(move || {
                    let mut posts = Vec::new();
                    for k in 0..PER_POSTER {
                        let value = input_of(poster, k, PER_POSTER);
                        posts.push(match k % 5 {
                            4 => record_post(clock, ABSENT, value, || {
                                root_inbox.post(naming_the_absent_session())
                            }),
                            1 | 3 => {
                                record_post(clock, 1, value, || child_inbox.post(posted(value)))
                            }
                            _ => record_post(clock, 0, value, || root_inbox.post(posted(value))),
                        });
                        made.fetch_add(1, Ordering::AcqRel);
                        std::thread::yield_now();
                    }
                    running.fetch_sub(1, Ordering::AcqRel);
                    posts
                })
            })
            .collect();

        // The owner: a macrostep of each session, an own event on each now and
        // then, and the child's end once half the posts are made.
        let mut own = OWN_BASE + 2;
        let (mut root_own, mut child_own) = (0u64, 0u64);
        let mut child_ended = false;
        let mut round = 0u64;
        while running.load(Ordering::Acquire) > 0 {
            root.step();
            child.step();
            round += 1;
            if round % 3 == 0 && root_own < OWN_PER_SESSION {
                raise_own(&mut root, own);
                own += 1;
                root_own += 1;
            }
            if round % 3 == 1 && !child_ended && child_own < OWN_PER_SESSION {
                raise_own(&mut child, own);
                own += 1;
                child_own += 1;
            }
            if !child_ended && made.load(Ordering::Acquire) * 2 >= total {
                record_end(1, || child.stop());
                // A child that has ended leaves its tree's directory, as the
                // generated parent has it leave, and what its inbox still
                // holds is applied then.
                child.leave_tree_inboxes();
                child_ended = true;
            }
            std::thread::yield_now();
        }
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a posting thread"))
            .collect()
    });

    // What the last posts left in either inbox, applied, and the ends of the
    // sessions not yet ended.
    root.step();
    child.step();
    if child.is_running() {
        record_end(1, || child.stop());
        child.step();
    }
    record_end(0, || root.stop());
    root.step();

    History {
        sessions: vec![0, 1],
        closes: false,
        capacity: None,
        owner: finish_recording(),
        posters,
    }
}

/// The capacity of the bounded inbox below.
const CAPACITY: usize = 4;

/// The bounded inbox one run after another is attached to; a `static`, as a
/// deployment declares it.
static BOUNDED: BoundedInbox<HistoryPolicy<0>, CAPACITY, StdHal> = BoundedInbox::new();

/// One run of the bare-metal profile's bounded inbox, attached to one session.
/// A first poster posts one input more than the inbox holds while the owner
/// stands between two operations, so one post is refused as full; then
/// `POSTERS` threads post while the owner drives the session and raises own
/// events, and the session ends once half the posts are made. Each refused post
/// is answered and the poster goes on with its next input.
fn record_bounded_run(clock: &Arc<Clock>) -> History {
    start_recording(Arc::clone(clock));
    let clock: &Clock = clock;
    let mut engine = Engine::new(HistoryPolicy::<0>);
    // Attached before the run starts, as a deployment hands its inbox to its
    // engine before anything asks for one of its own.
    engine.attach_inbox(&BOUNDED);
    let Ok(()) = engine.initialize();
    engine.step();

    let filler_posts = CAPACITY as u64 + 1;
    let mut posters = vec![std::thread::scope(|scope| {
        scope
            .spawn(|| {
                (0..filler_posts)
                    .map(|k| {
                        let value = input_of(0, k, filler_posts);
                        record_post(clock, 0, value, || BOUNDED.post(posted(value)))
                    })
                    .collect::<Vec<Post>>()
            })
            .join()
            .expect("the posting thread")
    })];
    // Past every input the first poster posted.
    let base = filler_posts + 1;
    // An event of the owner's own behind what the first poster's posts left in
    // the inbox, so every run enters one.
    raise_own(&mut engine, OWN_BASE);

    let made = AtomicUsize::new(0);
    let running = AtomicUsize::new(POSTERS);
    let total = POSTERS * PER_POSTER as usize;
    let more: Vec<Vec<Post>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..POSTERS)
            .map(|poster| {
                let (made, running) = (&made, &running);
                scope.spawn(move || {
                    let mut posts = Vec::new();
                    for k in 0..PER_POSTER {
                        let value = base + input_of(poster, k, PER_POSTER);
                        posts.push(if k % 7 == 6 {
                            record_post(clock, ABSENT, value, || {
                                BOUNDED.post(naming_the_absent_session())
                            })
                        } else {
                            record_post(clock, 0, value, || BOUNDED.post(posted(value)))
                        });
                        made.fetch_add(1, Ordering::AcqRel);
                        std::thread::yield_now();
                    }
                    running.fetch_sub(1, Ordering::AcqRel);
                    posts
                })
            })
            .collect();

        let mut own = OWN_BASE + 1;
        let mut ended = false;
        let mut round = 0u64;
        while running.load(Ordering::Acquire) > 0 {
            engine.step();
            round += 1;
            if !ended && round % 3 == 0 && own < OWN_BASE + OWN_PER_SESSION {
                raise_own(&mut engine, own);
                own += 1;
            }
            if !ended && made.load(Ordering::Acquire) * 2 >= total {
                record_end(0, || engine.stop());
                ended = true;
            }
            std::thread::yield_now();
        }
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a posting thread"))
            .collect()
    });
    posters.extend(more);

    engine.step();
    if engine.is_running() {
        record_end(0, || engine.stop());
    }
    // What was taken after the end, applied where the owner reaches the end of
    // its loop again.
    engine.step();

    History {
        sessions: vec![0],
        closes: false,
        capacity: Some(CAPACITY as u64),
        owner: finish_recording(),
        posters,
    }
}

// ─────────────────────────────────────────────────────────────
// Judging a run
// ─────────────────────────────────────────────────────────────

/// `history`, sent through the JSON form every runtime writes its runs in
/// (tests/forge/conformance/inbox_history.schema.json) and read back, so this
/// lane's runs are judged in the form `sce-codegen check-inbox-history` reads.
fn through_the_wire(history: &History) -> History {
    History::from_json(&history.to_json())
        .expect("a recorded history reads back from its JSON form")
}

/// The owner's enqueues of inputs posts took, in its program order: each one's
/// place among the owner's operations, its session and its item.
fn posted_enqueues(history: &History) -> Vec<(usize, u64, u64)> {
    history
        .owner
        .iter()
        .enumerate()
        .filter_map(|(at, op)| match op.call {
            OwnerCall::Enqueue {
                session,
                item,
                source: Source::Posted,
            } => Some((at, session, item)),
            _ => None,
        })
        .collect()
}

/// The inputs the posts of `history` took.
fn taken_inputs(history: &History) -> Vec<u64> {
    history
        .posters
        .iter()
        .flatten()
        .filter(|post| post.outcome == Delivery::Taken)
        .map(|post| post.input)
        .collect()
}

/// Hold `history`, a run of the inbox named `what`, to the promise: every
/// input a post took entered once, and the whole record is linearizable.
fn assert_run_is_linearizable(what: &str, history: &History) {
    let read = through_the_wire(history);
    let mut taken = taken_inputs(&read);
    let mut entered: Vec<u64> = posted_enqueues(&read)
        .into_iter()
        .map(|(_, _, item)| item)
        .collect();
    taken.sort_unstable();
    entered.sort_unstable();
    assert_eq!(
        entered, taken,
        "{what}: the inputs that entered the sessions' queues are not the inputs the posts took"
    );
    match check(&read) {
        Verdict::Linearizable => {}
        other => panic!(
            "{what}: the history is not linearizable to docs/formal/InboxService.tla: {other:?}\n{}",
            read.to_json()
        ),
    }
}

/// Whether `history` holds what the run was meant to exercise: posts taken,
/// posts refused for `reason`, own events entered and every session ended.
fn assert_run_exercised(what: &str, history: &History, reason: Refusal) {
    let posts: Vec<&Post> = history.posters.iter().flatten().collect();
    assert!(
        posts.iter().any(|post| post.outcome == Delivery::Taken),
        "{what}: no post was taken"
    );
    assert!(
        posts
            .iter()
            .any(|post| post.outcome == Delivery::Refused(reason)),
        "{what}: no post was refused as {reason:?}"
    );
    assert!(
        history.owner.iter().any(|op| matches!(
            op.call,
            OwnerCall::Enqueue {
                source: Source::Own,
                ..
            }
        )),
        "{what}: the owner put no event of its own on a queue"
    );
    for &session in &history.sessions {
        assert!(
            history
                .owner
                .iter()
                .any(|op| op.call == OwnerCall::End { session }),
            "{what}: session {session} never ended"
        );
    }
}

#[test]
fn every_run_of_a_trees_own_inboxes_is_linearizable_to_the_promise() {
    for run in 0..RUNS {
        let clock = Arc::new(Clock::new());
        let history = record_std_run(&clock);
        let what = format!("std run {run}");
        assert_run_exercised(&what, &history, Refusal::Unknown);
        assert_run_is_linearizable(&what, &history);
    }
}

#[test]
fn every_run_of_a_bounded_inbox_is_linearizable_to_the_promise() {
    for run in 0..RUNS {
        let clock = Arc::new(Clock::new());
        let history = record_bounded_run(&clock);
        let what = format!("bounded run {run}");
        assert_run_exercised(&what, &history, Refusal::Full);
        assert!(
            history
                .posters
                .iter()
                .flatten()
                .any(|post| post.outcome == Delivery::Refused(Refusal::Unknown)),
            "{what}: no post naming a session the inbox does not reach was refused"
        );
        assert_run_is_linearizable(&what, &history);
    }
}

// The checker refuses a recorded run of this lane that lost an enqueue: the
// recording drops nothing the cases above would then pass on.
#[test]
fn a_recorded_run_with_an_enqueue_left_out_is_refused() {
    let clock = Arc::new(Clock::new());
    let mut history = record_std_run(&clock);
    let (at, _, item) = *posted_enqueues(&history)
        .last()
        .expect("the run entered an input a post took");
    history.owner.remove(at);
    let verdict = check(&through_the_wire(&history));
    assert!(
        matches!(verdict, Verdict::NotLinearizable { .. }),
        "the run with input {item}'s enqueue left out, an input taken and never entered, was \
         judged {verdict:?}"
    );
}

// The checker refuses a recorded run of this lane whose owner entered two inputs
// of one session in the reverse of the order one poster posted them in: the
// order the runtime keeps is what the cases above hold.
#[test]
fn a_recorded_run_with_two_inputs_of_one_poster_entered_in_reverse_is_refused() {
    let clock = Arc::new(Clock::new());
    let mut history = record_std_run(&clock);
    // Which poster posted `input`.
    let poster_of = |history: &History, input: u64| {
        history
            .posters
            .iter()
            .position(|posts| posts.iter().any(|post| post.input == input))
    };
    let enqueues = posted_enqueues(&history);
    let (first, second) = enqueues
        .iter()
        .enumerate()
        .find_map(|(i, &(at, session, item))| {
            enqueues[i + 1..]
                .iter()
                .find(|&&(_, other_session, other)| {
                    other_session == session
                        && poster_of(&history, other) == poster_of(&history, item)
                })
                .map(|&(other_at, _, _)| (at, other_at))
        })
        .expect("one poster's posts to one session entered at least twice");
    let swap = |op: &mut OwnerOperation, item: u64| {
        if let OwnerCall::Enqueue { item: entered, .. } = &mut op.call {
            *entered = item;
        }
    };
    let (a, b) = match (history.owner[first].call, history.owner[second].call) {
        (OwnerCall::Enqueue { item: a, .. }, OwnerCall::Enqueue { item: b, .. }) => (a, b),
        _ => unreachable!("both places are enqueues"),
    };
    swap(&mut history.owner[first], b);
    swap(&mut history.owner[second], a);
    let verdict = check(&through_the_wire(&history));
    assert!(
        matches!(verdict, Verdict::NotLinearizable { .. }),
        "the run with inputs {a} and {b} of one poster entered in reverse was judged {verdict:?}"
    );
}
