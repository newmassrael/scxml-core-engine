// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0084, ARCHITECTURE.md "Engine Threading": an engine is confined to
// its owner, and what another thread has for it enters through its inbox. These
// cases hold the inbox itself to its contract on a machine written by hand, so
// that what they read is the runtime and not a generated document:
//
//   * a post is applied at the owner's next macrostep boundary, behind what the
//     owner queued, and not before;
//   * a post the inbox took wakes the owner through the policy's HAL and through
//     the waker the host installed;
//   * a post to the inbox of an engine that is gone is refused with its input;
//   * a posted event's payload reaches the machine with it, and comes back
//     with it when the post is refused;
//   * a post taken after the run was stopped is counted as unseen, as a direct
//     raise would be, and not held where nobody reads it;
//   * the bare-metal inbox a deployment declares refuses a post when full, gives
//     the input back and counts the refusal, and the engine it is attached to
//     applies what it took;
//   * an engine holds one inbox, its own or a bounded one, and refuses the
//     other.
//
// The machine: one state, `s`, whose two targetless transitions take `ping` and
// `pong` and record, in order, the name of each event they took, and the
// payload each event carried.
//
// The generated-machine case, many threads against a single-threaded replay, is
// `backends/rust/tests/tests/what_another_thread_posts_enters_through_the_inbox.rs`.

// The engine's own inbox, `EngineInbox`, is the `std` profile's.
#![cfg(feature = "std")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use sce_rust_runtime::helpers::hierarchy::BoundedVec;
use sce_rust_runtime::{
    state_scratch, state_words, BoundedInbox, DocumentTables, Engine, EngineInbox, EngineState,
    EventId, EventWithMetadata, Hal, HistoryId, HostInvokeEnding, InboxInput, InboxRefusal,
    InboxWaker, NoHistory, RunStore, StateId, StateKind, StatePolicy, StateRow, Target,
    TransitionId, TransitionRow, Workspace,
};

// ─────────────────────────────────────────────────────────────
// A HAL that counts its wakes
// ─────────────────────────────────────────────────────────────

/// How many times `CountingHal::wake` was called in this process.
static WAKES: AtomicUsize = AtomicUsize::new(0);

/// `WAKES` is one count for the whole binary, and cargo runs its tests on
/// parallel threads. Every case that posts holds this lock, so the case that
/// reads the count reads its own posts alone. `into_inner` keeps one failing
/// case from turning every later one into a poisoned-lock panic.
static POSTING: Mutex<()> = Mutex::new(());

fn posting() -> MutexGuard<'static, ()> {
    POSTING.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Debug, Clone, Copy, Default)]
struct CountingHal;

impl Hal for CountingHal {
    fn now_ticks_ms() -> u64 {
        0
    }

    fn wake() {
        WAKES.fetch_add(1, Ordering::SeqCst);
    }

    fn irq_save<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        f()
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
    Pong,
}

/// `s` on `ping` and on `pong`, both targetless, by `TransitionId`.
const S_PING: TransitionId = TransitionId(0);
const S_PONG: TransitionId = TransitionId(1);

/// What the machine took, in order, and the payload each event carried: `D`,
/// the machine's payload type, is `()` but in the case that posts a payload.
struct InboxPolicy<D = ()> {
    taken: Vec<&'static str>,
    payloads: Vec<D>,
    /// The session's id: the empty text of a policy written by hand that binds
    /// none, but in the case that holds a tree of two sessions.
    id: &'static str,
}

impl InboxPolicy {
    fn new() -> Self {
        Self::carrying()
    }

    /// The machine of the session `id`.
    fn named(id: &'static str) -> Self {
        Self {
            id,
            ..Self::carrying()
        }
    }
}

impl<D> InboxPolicy<D> {
    fn carrying() -> Self {
        Self {
            taken: Vec::new(),
            payloads: Vec::new(),
            id: "",
        }
    }
}

impl<D: Clone + Default + std::fmt::Debug + 'static> StatePolicy for InboxPolicy<D> {
    type State = St;
    type Event = Ev;
    type History = NoHistory;
    type Payload = D;
    // docs/adr/0124: the payload an event carried, read where the engine
    // holds it.
    type EventData<'e> = &'e D;
    type Hal = CountingHal;
    type InternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    type ExternalQueue = sce_rust_runtime::EventQueue<Self::Payload>;
    // It reads no person, and holds none (docs/adr/0116).
    type Principal = sce_rust_runtime::NoPrincipal;
    type ScheduledSendId = sce_rust_runtime::SceString;
    // No delayed send, no `<cancel>`, no `<donedata>`.
    type Scheduler = sce_rust_runtime::NoDelayedSends;
    type DoneData = sce_rust_runtime::NoDoneData;
    // One state and no history: one transition is enabled at a time.
    type Workspace = Workspace<{ state_scratch(1) }, 1, 6, 0>;
    type RunStore = RunStore<{ state_words(1) }, 0, 0>;
    type Configuration = BoundedVec<St, 1>;
    type StartFailure = core::convert::Infallible;

    // One child of the `<scxml>` element with two targetless transitions that
    // have content; the document's initial transition, last, enters it.
    const TABLES: &'static DocumentTables<'static> = &DocumentTables {
        states: &[StateRow::new(None, StateKind::Atomic, StateId(0), None)],
        histories: &[],
        transitions: &[
            // s: ping, targetless
            TransitionRow::new(Some(StateId(0)), Target::NONE, false, true),
            // s: pong, targetless
            TransitionRow::new(Some(StateId(0)), Target::NONE, false, true),
            // <scxml initial="s">
            TransitionRow::new(None, Target::state(StateId(0)), false, false),
        ],
        targets: &[],
        initial: TransitionId(2),
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
    const EVENTS: &'static [Ev] = &[Ev::Null, Ev::Ping, Ev::Pong];

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
            Ev::Pong => "pong",
        }
    }

    fn get_event_from_name(name: &str) -> Option<Self::Event> {
        match name {
            "ping" => Some(Ev::Ping),
            "pong" => Some(Ev::Pong),
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

    // A session of a tree is named by its own id.
    fn own_session_id(&self) -> &str {
        self.id
    }

    // `_is_default_entry` is unused: this policy has no compound states.
    fn event_data<'a>(
        held: &'a sce_rust_runtime::CurrentEventData<'_, Self>,
    ) -> Self::EventData<'a> {
        match held {
            sce_rust_runtime::CurrentEventData::Queued(payload) => payload,
            sce_rust_runtime::CurrentEventData::Lent(data) => *data,
        }
    }

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
            (St::S, Ev::Pong) => Some(S_PONG),
            _ => None,
        }
    }

    // The content of each transition records the payload its event carried,
    // as the engine hands it over (docs/adr/0124).
    fn execute_transition_content(
        &mut self,
        transition: TransitionId,
        event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) {
        if transition == S_PING {
            self.taken.push("ping");
        } else if transition == S_PONG {
            self.taken.push("pong");
        }
        self.payloads.push(event_data.clone());
    }
}

fn running() -> Engine<InboxPolicy> {
    let mut engine = Engine::new(InboxPolicy::new());
    let Ok(()) = engine.initialize();
    engine
}

fn ping() -> InboxInput<Ev> {
    InboxInput::external(Ev::Ping, "")
}

/// `ping`, carrying `payload`, for the machine whose payload type is `u32`.
fn ping_carrying(payload: u32) -> InboxInput<Ev, u32> {
    let mut event: EventWithMetadata<Ev, u32> = EventWithMetadata::new(Ev::Ping);
    event.payload = payload;
    InboxInput::ExternalEvent(event)
}

// ─────────────────────────────────────────────────────────────
// Cases
// ─────────────────────────────────────────────────────────────

// The handle is what crosses threads; the engine stays `Send` with an inbox in it.
#[test]
fn the_inbox_handle_crosses_threads_and_the_engine_stays_send() {
    fn assert_send_sync<T: Send + Sync>() {}
    fn assert_send<T: Send>() {}
    assert_send_sync::<EngineInbox<InboxPolicy>>();
    assert_send_sync::<BoundedInbox<InboxPolicy, 4, CountingHal>>();
    assert_send::<Engine<InboxPolicy>>();
}

#[test]
fn a_post_is_applied_at_the_next_boundary_behind_what_the_owner_queued() {
    let _posting = posting();
    let mut engine = running();
    engine.raise_external(Ev::Pong, "");

    let inbox = engine.inbox();
    let taken = std::thread::spawn(move || inbox.post(ping()).is_ok())
        .join()
        .expect("the posting thread");
    assert!(
        taken,
        "an engine's own inbox takes every post while the engine lives"
    );
    assert!(
        engine.policy().taken.is_empty(),
        "a post was applied before the owner reached a macrostep boundary"
    );

    engine.step();
    assert_eq!(
        engine.policy().taken,
        ["pong", "ping"],
        "the posted event did not join the external queue behind what the owner raised"
    );
}

#[test]
fn a_post_wakes_the_owner_through_the_hal_and_the_installed_waker() {
    let _posting = posting();
    let mut engine = running();
    let inbox = engine.inbox();
    let installed = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&installed);
    let waker: InboxWaker = Arc::new(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    inbox.wake_with(Some(waker));
    let hal_before = WAKES.load(Ordering::SeqCst);

    for _ in 0..2 {
        let poster = inbox.clone();
        let taken = std::thread::spawn(move || poster.post(ping()).is_ok())
            .join()
            .expect("the posting thread");
        assert!(
            taken,
            "an engine's own inbox takes every post while the engine lives"
        );
    }
    assert_eq!(
        WAKES.load(Ordering::SeqCst) - hal_before,
        2,
        "the policy's HAL was not woken once for each post the inbox took"
    );
    assert_eq!(
        installed.load(Ordering::SeqCst),
        2,
        "the installed waker was not called once for each post the inbox took"
    );

    inbox.wake_with(None);
    inbox
        .post(ping())
        .expect("an engine's own inbox takes every post while the engine lives");
    assert_eq!(
        installed.load(Ordering::SeqCst),
        2,
        "a removed waker was still called"
    );
    assert_eq!(WAKES.load(Ordering::SeqCst) - hal_before, 3);

    engine.step();
    assert_eq!(engine.policy().taken, ["ping", "ping", "ping"]);
}

#[test]
fn a_post_to_the_inbox_of_an_engine_that_is_gone_is_refused_with_its_input() {
    let _posting = posting();
    let engine = running();
    let inbox = engine.inbox();
    drop(engine);

    match inbox.post(ping()) {
        Err(InboxRefusal::EngineGone(InboxInput::ExternalEvent(refused))) => {
            assert_eq!(
                refused.event,
                Ev::Ping,
                "the refusal gave back another input"
            );
        }
        other => panic!("a post to the inbox of a dropped engine answered {other:?}"),
    }
}

// The inbox holds what was posted without the machine's types, so a payload
// rides beside its event: it reaches the machine with the event, and comes
// back with the event when the post is refused.
#[test]
fn a_posted_payload_rides_with_its_event_and_comes_back_with_a_refusal() {
    let _posting = posting();
    let mut engine = Engine::new(InboxPolicy::<u32>::carrying());
    let Ok(()) = engine.initialize();
    let inbox = engine.inbox();

    let poster = inbox.clone();
    let taken = std::thread::spawn(move || poster.post(ping_carrying(7)).is_ok())
        .join()
        .expect("the posting thread");
    assert!(
        taken,
        "an engine's own inbox takes every post while the engine lives"
    );
    engine.step();
    assert_eq!(engine.policy().taken, ["ping"]);
    assert_eq!(
        engine.policy().payloads,
        [7],
        "the posted event reached the machine without the payload it carried"
    );

    drop(engine);
    match inbox.post(ping_carrying(9)) {
        Err(InboxRefusal::EngineGone(InboxInput::ExternalEvent(refused))) => {
            assert_eq!(
                refused.event,
                Ev::Ping,
                "the refusal gave back another event"
            );
            assert_eq!(
                refused.payload, 9,
                "the refusal gave back the event without its payload"
            );
        }
        other => panic!("a post to the inbox of a dropped engine answered {other:?}"),
    }
}

// A post taken after the run was stopped is applied where the owner next reaches
// the end of its loop, as `raise_external` made then would be: counted as unseen.
#[test]
fn a_post_after_the_run_was_stopped_is_counted_as_unseen() {
    let _posting = posting();
    let mut engine = running();
    engine.stop();

    engine
        .inbox()
        .post(ping())
        .expect("the inbox stays open for the engine's life");
    engine.step();

    assert!(
        engine.policy().taken.is_empty(),
        "a stopped machine took an event"
    );
    assert_eq!(
        engine.unseen_external_events(),
        1,
        "the post was neither taken nor counted: it was dropped in silence"
    );
    assert_eq!(engine.last_unseen_event(), Some(Ev::Ping));
}

// docs/adr/0084, D2 as amended at step 10: the tree keeps a directory from the id
// of each of its sessions to that session's inbox, the root's as well as each
// child's (docs/adr/0147, D2). An ending that names the root, posted through a
// child's handle, is looked up there and taken for the root; once the root's
// engine is gone its inbox is named no longer, and the same post is refused as
// naming a session the tree does not hold.
#[test]
fn the_roots_inbox_is_in_its_trees_directory_while_its_engine_lives() {
    let _posting = posting();
    let root = Engine::new(InboxPolicy::named("root"));
    let mut child = Engine::new(InboxPolicy::named("child"));
    root.share_tree_inboxes_with(&mut child);
    let _root_inbox = root.inbox();
    let child_inbox = child.inbox();
    let ending = || {
        InboxInput::host_invoke_ending(
            Some("root"),
            HostInvokeEnding {
                processor_type: "x-sce-host",
                invoke_id: "job",
                token: 1,
                done: true,
                data: "",
                origin: None,
            },
        )
    };
    let taken = child_inbox.post(ending());
    assert!(
        taken.is_ok(),
        "an ending naming the root, posted through its child's handle, was answered {taken:?}"
    );

    drop(root);
    match child_inbox.post(ending()) {
        Err(InboxRefusal::UnknownSession(_)) => {}
        other => panic!(
            "an ending naming a root whose engine is gone, posted through its child's handle, \
             was answered {other:?}"
        ),
    }
}

/// Two inputs, so the third post finds it full.
static FULL_AT_TWO: BoundedInbox<InboxPolicy, 2, CountingHal> = BoundedInbox::new();

#[test]
fn a_bounded_inbox_refuses_a_post_when_full_and_its_engine_applies_what_it_took() {
    let _posting = posting();
    let hal_before = WAKES.load(Ordering::SeqCst);

    let taken = std::thread::spawn(|| FULL_AT_TWO.post(ping()).is_ok())
        .join()
        .expect("the posting thread");
    assert!(taken, "an empty inbox of two takes a post");
    FULL_AT_TWO
        .post(InboxInput::external(Ev::Pong, ""))
        .expect("an inbox of two holding one takes a post");
    match FULL_AT_TWO.post(ping()) {
        Err(InboxRefusal::Full(InboxInput::ExternalEvent(refused))) => {
            assert_eq!(
                refused.event,
                Ev::Ping,
                "the refusal gave back another input"
            );
        }
        other => panic!("a post to a full inbox answered {other:?}"),
    }
    assert_eq!(
        FULL_AT_TWO.refused_posts(),
        1,
        "the refusal was not counted"
    );
    assert_eq!(
        WAKES.load(Ordering::SeqCst) - hal_before,
        2,
        "the named HAL was not woken once for each post the inbox took, and only for those"
    );

    // Attached before the run starts, so what it holds is applied at the run's
    // first boundary.
    let mut engine = Engine::new(InboxPolicy::new());
    engine.attach_inbox(&FULL_AT_TWO);
    let Ok(()) = engine.initialize();
    assert_eq!(
        engine.policy().taken,
        ["ping", "pong"],
        "the attached inbox was not drained, or not in the order it took the posts"
    );

    FULL_AT_TWO
        .post(ping())
        .expect("an inbox the owner drained takes a post again");
    engine.step();
    assert_eq!(engine.policy().taken, ["ping", "pong", "ping"]);
}

/// Bounded inboxes for the cases below, one each, as cargo runs them on
/// parallel threads.
static AFTER_ITS_OWN: BoundedInbox<InboxPolicy, 2, CountingHal> = BoundedInbox::new();
static BEFORE_ITS_OWN: BoundedInbox<InboxPolicy, 2, CountingHal> = BoundedInbox::new();

// A session has one inbox (docs/adr/0084, D2; docs/adr/0147, D1): two, drained
// one after the other, would take a post to the second ahead of an earlier post
// to the first. So an engine whose own inbox was made refuses a bounded one...
#[test]
#[should_panic(expected = "a session has one inbox")]
fn an_engine_whose_own_inbox_was_made_refuses_a_bounded_one() {
    let mut engine = Engine::new(InboxPolicy::new());
    let _own = engine.inbox();
    engine.attach_inbox(&AFTER_ITS_OWN);
}

// ...and an engine a bounded inbox is attached to makes no inbox of its own.
#[test]
#[should_panic(expected = "a session has one inbox")]
fn an_engine_a_bounded_inbox_is_attached_to_makes_no_inbox_of_its_own() {
    let mut engine = Engine::new(InboxPolicy::new());
    engine.attach_inbox(&BEFORE_ITS_OWN);
    let _own = engine.inbox();
}
