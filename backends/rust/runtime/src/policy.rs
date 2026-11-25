// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! The [`StatePolicy`] trait: the contract generated state machine code implements.
//!
//! This is the Rust analog of the C++ `StatePolicy` concept defined across
//! `sce/include/core/StatePolicyConcepts.h` and `sce/include/static/StaticExecutionEngine.h`.
//! Generated code produces a struct implementing `StatePolicy` per SCXML file, and
//! the runtime [`Engine<P>`](crate::Engine) is parameterized on that struct.
//!
//! ## Design
//!
//! The C++ engine uses CRTP (template-based policy pattern) for zero-overhead
//! compile-time polymorphism. Rust uses generics with trait bounds to achieve the
//! same effect. Generated code becomes fully monomorphized — no dynamic dispatch.
//!
//! ## What the policy answers, and what it does not
//!
//! The policy answers what only the document knows: its structure, which of a
//! state's transitions an event enables, what a transition's content is, what a
//! state's onentry and onexit do, what a history recorded. What W3C SCXML
//! Appendix D does with those answers — which transitions an event selects,
//! which survive preemption, which states a microstep exits and enters and in
//! which order — is [`helpers::microstep`](crate::helpers::microstep), written
//! once for every program. It reads the policy's
//! [`TABLES`](StatePolicy::TABLES) and asks for the rest through the engine
//! (docs/adr/0064).
//!
//! ## Static vs Instance Methods
//!
//! - **Static methods** (`is_final_state()`, `get_event_name()`, etc.) mirror
//!   C++ `constexpr static` methods. They encode compile-time SCXML document
//!   structure (state hierarchy, initial and history targets, document order).
//! - **Instance methods** (`execute_entry_actions()`, `guard_holds()`,
//!   etc.) mirror C++ non-static policy methods.
//!   They hold mutable datamodel state and read/write through `&mut self`.
//!
//! ## Optional Features
//!
//! Optional capabilities (datamodel init, invoke support, finalize, parallel states)
//! are signaled via associated `const bool` flags (`NEEDS_SCRIPT_ENGINE`,
//! `HAS_PARALLEL_STATES`, `NEEDS_DATA_MODEL_INIT`, etc.). Default implementations
//! of the corresponding methods are no-ops, so the generator only overrides what
//! the SCXML document actually uses. The engine branches on these `const` flags at
//! compile time, yielding zero runtime overhead for unused features.

use core::fmt::Debug;
use core::hash::Hash;

use crate::engine::StepFunctionForm;
use crate::event::{EventMetadata, EventWithMetadata};
use crate::hal::Hal;
use crate::helpers::event_queue::EventQueueStorage;
use crate::helpers::hierarchy::Bounded;
use crate::helpers::microstep::Buffers;
use crate::helpers::run_store::RunStorage;
use crate::helpers::tables::{DocumentTables, EventId, HistoryId, StateId, TransitionId};
use crate::EngineState;

/// A text or byte string of data a host lent that is past the bound its schema
/// states (docs/adr/0124, D2): the field, the bytes it holds and the bound.
/// A value of every profile and of no allocation, so a policy answers the door
/// of lent data in one signature (`StatePolicy::admit_event_data`); a `std`
/// build words it as the lift of the same field from the wire refuses it
/// (`event_payload::lent_refusal`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LentBreach {
    /// The field, as its schema names it.
    pub field: &'static str,
    /// The bytes it holds.
    pub held: usize,
    /// The bound its type declares.
    pub bound: usize,
}

impl LentBreach {
    /// `held` bytes of `field` within `bound`, or the breach.
    pub const fn within(field: &'static str, held: usize, bound: usize) -> Result<(), Self> {
        if held <= bound {
            Ok(())
        } else {
            Err(Self { field, held, bound })
        }
    }
}

/// What the engine holds as the data of the event being processed, from the
/// moment it takes the event to the moment it takes the next one
/// (docs/adr/0124). Read by every hook through
/// [`StatePolicy::event_data`].
pub enum CurrentEventData<'d, P: StatePolicy> {
    /// The payload the event carried through its queue, owned by the engine
    /// while the event is `_event`.
    Queued(P::Payload),
    /// The data a host lent with the event it processes now
    /// ([`Engine::process_event_with`](crate::Engine::process_event_with)),
    /// borrowed for that call alone.
    Lent(P::EventData<'d>),
}

/// The contract that generated state machine policies must satisfy.
///
/// Ports the C++ `StatePolicy` concept from `sce/include/core/StatePolicyConcepts.h`.
/// Generated code produces one struct implementing `StatePolicy` per SCXML source file.
pub trait StatePolicy: Sized + 'static {
    // ──────────────────────────────────────────────
    // Associated types (C++ `using State = ...; using Event = ...;`)
    // ──────────────────────────────────────────────

    /// State enum type generated per SCXML document (§scxml-3.3).
    ///
    /// Must be `Copy` (enums are tiny), `Eq`/`Hash` for active-state sets,
    /// `Debug` for logging, `'static` because it has no references.
    type State: Copy + Eq + Hash + Debug + 'static;

    /// Event enum type generated per SCXML document (§scxml-3.12).
    type Event: Copy + Eq + Hash + Debug + 'static;

    /// `<history>` enum type generated per SCXML document (§scxml-3.10), or
    /// [`NoHistory`](crate::helpers::tables::NoHistory) for a document that
    /// declares none.
    ///
    /// A history is not a state — it is never in a configuration — so it is not
    /// a variant of [`State`](StatePolicy::State): target lists name one as a
    /// [`Target::history`](crate::helpers::tables::Target::history), and the
    /// entry procedures dereference it to what it recorded or to its default.
    type History: Copy + Eq + Debug + 'static;

    /// Typed event payload for EventSchema native lowering: the data of an
    /// event as a queue owns it.
    ///
    /// `()` for schemaless documents — the dynamic `_event.data` baseline keeps
    /// riding in [`EventMetadata::data`](crate::EventMetadata) as before. For a
    /// document that imports `EventSchema`s, generated code sets this to the
    /// per-document payload sum (`<Doc>Payload`) so that `_event.data.<field>`
    /// guards lower to native field reads with no script engine — the no_std MCU
    /// value path. The payload rides with its event through the queues in
    /// [`EventWithMetadata::payload`](crate::EventWithMetadata), and the engine
    /// holds it beside the turn while its event is `_event`: what reads it reads
    /// it through [`EventData`](StatePolicy::EventData), handed to each hook
    /// (docs/adr/0124).
    ///
    /// `Default` supplies the slot for payloadless/schemaless events (the queues
    /// are homogeneous in `Self::Payload`); `Clone`/`Debug` mirror the
    /// `EventWithMetadata` derives; `'static` because a queued event outlives
    /// the call that queued it, so its payload carries no borrows.
    type Payload: Clone + Default + Debug + 'static;

    /// The data of the event being processed, as every reader of it reads it:
    /// a guard, an `<sce:arg>`, the content of a transition (docs/adr/0124).
    ///
    /// One value whether the data came owned through a queue, as a
    /// [`Payload`](StatePolicy::Payload), or was lent by the host for one call
    /// ([`Engine::process_event_with`](crate::Engine::process_event_with)): a
    /// borrow of it for `'e`, read through each schema's read interface. A
    /// generated policy spells `<Machine>EventData<'e>`, with a `None` for an
    /// event that carries no data; a policy whose events carry none spells
    /// `()`. No default: stable Rust has no defaults for associated types.
    ///
    /// It is an argument of every hook that runs executable content, and never
    /// stored: the policy keeps no lifetime, and the borrow checker proves that
    /// nothing a hook keeps outlives the event.
    type EventData<'e>: Copy
    where
        Self: 'e;

    /// The data the engine holds for the event being processed, as
    /// [`EventData`](StatePolicy::EventData): the payload a queue handed over,
    /// read in place, or the data a host lent, as it is (docs/adr/0124).
    ///
    /// A policy whose events carry no data answers `()`:
    /// `fn event_data<'a>(_held: &'a CurrentEventData<'_, Self>) -> Self::EventData<'a> {}`.
    fn event_data<'a>(held: &'a CurrentEventData<'_, Self>) -> Self::EventData<'a>;

    /// HAL impl bound to this policy (SCE Protocol-Synthesis RFC §synth-5-J-2 line 1984).
    ///
    /// Determines which [`Hal`] impl the [`Engine`](crate::Engine) dispatches `ticks` /
    /// `wake` / `irq-save` calls through. Generated code emits
    /// `type Hal = sce_rust_runtime::StdHal;` per policy under the default
    /// host backend; the future `sce-codegen generate -l rust --no-std` flag
    /// (Atomic B-β) will emit a different HAL type for no_std consumers.
    ///
    /// No default is provided: stable Rust forbids default associated types
    /// (`#![feature(associated_type_defaults)]` is nightly-only), and the
    /// explicit per-policy emission keeps every generated state machine's
    /// HAL target self-declaring rather than implicit.
    type Hal: Hal;

    /// The queue backing this machine's W3C SCXML Appendix D `internalQueue`:
    /// what the machine raises for itself during a macrostep (docs/adr/0076).
    ///
    /// Carries the queue's FIFO **depth** under `--no-default-features`:
    /// generated code emits `EventQueue<Self::Payload, N>`. Every event this
    /// queue can hold is raised by the document or by the engine for it, so
    /// the generator proves `N` from the document when it can (the most one
    /// microstep can enqueue, times the microsteps that can run before the
    /// first internal event is taken), and states in the generated policy
    /// which case gave it. `0` is a depth: a document that raises nothing for
    /// itself keeps no internal slots. When the generator proves no bound, the
    /// depth follows the [`ExternalQueue`](StatePolicy::ExternalQueue) rule.
    ///
    /// Under the std build the depth is inert (`VecDeque`, unbounded — the
    /// spec's unbounded `Queue`), so the same emission compiles on both
    /// runtime profiles. Like the other per-machine associated types it has no
    /// default (stable Rust forbids default associated types) and is emitted
    /// by every generated policy.
    type InternalQueue: EventQueueStorage<Self::Payload>;

    /// The internal-queue depth the generator proved from the document, when it
    /// proved one (docs/adr/0076): the same number as
    /// [`InternalQueue`](StatePolicy::InternalQueue)'s depth.
    ///
    /// A build with debug assertions holds every raise to it, on both profiles.
    /// Under std the queue is unbounded, so without this a bound too small for
    /// the document would pass every test and stop only a bare-metal machine;
    /// with it, each test run of a machine with a proven bound checks the
    /// proof. `None`, the default, asks nothing.
    const INTERNAL_QUEUE_BOUND: Option<usize> = None;

    /// The queue backing this machine's W3C SCXML Appendix D `externalQueue`:
    /// what the host and other sessions deliver.
    ///
    /// Carries the queue's FIFO **depth** under `--no-default-features`:
    /// generated code emits `EventQueue<Self::Payload, N>` where `N` is the
    /// per-document capacity resolved from `<scxml sce:capacity="N">` / deploy
    /// `default_event_queue_capacity` (emitted as `EVENT_QUEUE_CAPACITY`),
    /// defaulting to [`MAX_EVENT_QUEUE_DEPTH`](crate::MAX_EVENT_QUEUE_DEPTH)
    /// when the document declares no capacity. No document can state the rate
    /// at which its host delivers events, so this depth is the host's
    /// statement and not something the generator proves (docs/adr/0076).
    /// Inert under std, like [`InternalQueue`](StatePolicy::InternalQueue).
    /// The queues hold their events as numbers, so the engine drives every
    /// machine's queues with one copy of its code (docs/adr/0068).
    type ExternalQueue: EventQueueStorage<Self::Payload>;

    /// Storage for the delayed-send scheduler's per-entry cancel key
    /// (`send_id`), backing §scxml-6.3 `<cancel sendid>`.
    ///
    /// The scheduler keeps a `send_id` per pending entry purely so `<cancel>`
    /// can find and drop the matching one; the id is read only by
    /// [`EngineState::cancel_event`](crate::EngineState::cancel_event) and never reaches a
    /// fired event's metadata (the timer drain passes an empty `send_id`). A
    /// document with no `<cancel>` therefore never reads it, so generated code
    /// emits the zero-size [`ElidedSendId`](crate::ElidedSendId) and the no_std
    /// scheduler ring sheds its per-entry `heapless::String<256>`
    /// (~264 B × [`MAX_SCHEDULED_EVENTS`](crate::MAX_SCHEDULED_EVENTS)); a
    /// document that cancels emits [`SceString`](crate::SceString) (load-bearing
    /// on both profiles). The choice is behaviour-preserving under std either
    /// way, so the one emission compiles on both runtime profiles.
    ///
    /// The key of the [`Scheduler`](StatePolicy::Scheduler) when that is a
    /// [`PullScheduler`](crate::PullScheduler): generated code writes
    /// `PullScheduler<Self::ScheduledSendId>`. A machine that keeps no
    /// delayed-send queue keeps no key either, and its policy still names one
    /// (the zero-size `ElidedSendId`, since such a document has no `<cancel>`).
    ///
    /// Mirrors the per-machine [`ExternalQueue`](StatePolicy::ExternalQueue)
    /// sizing lever; like it (and [`Hal`](StatePolicy::Hal) / [`Payload`](StatePolicy::Payload))
    /// it has no default — stable Rust forbids default associated types, so
    /// every generated policy emits it.
    type ScheduledSendId: crate::ScheduledSendIdLike;

    /// The delayed-send queue the [`Engine`](crate::Engine) keeps for this machine, chosen by
    /// what the document uses.
    ///
    /// - [`NoDelayedSends`](crate::NoDelayedSends), zero-size, for a document
    ///   with no `<send delay>`, no `<send delayexpr>`, no `<cancel>` and no
    ///   host-run deadline: nothing can be armed, so no queue is kept. Under
    ///   `--no-default-features` the bounded ring it replaces is
    ///   [`MAX_SCHEDULED_EVENTS`](crate::MAX_SCHEDULED_EVENTS) entries of
    ///   capacity nobody could fill.
    /// - [`PullScheduler`](crate::PullScheduler) keyed by
    ///   [`ScheduledSendId`](StatePolicy::ScheduledSendId) for every other
    ///   document.
    ///
    /// The same emission compiles on both runtime profiles, and the choice is
    /// behaviour-preserving on both: a document the zero-size queue is chosen
    /// for has nothing to arm on it. Emitted by every generated policy, like
    /// [`ScheduledSendId`](StatePolicy::ScheduledSendId).
    type Scheduler: crate::SchedulerStorage;

    /// Where the payload of a top-level `<final>`'s `<donedata>` waits for the
    /// parent that lifts it onto `done.invoke.<id>`
    /// ([`EngineState::donedata_at_final`]).
    ///
    /// [`SceString`](crate::SceString) for a document with such a `<donedata>`,
    /// and the zero-size [`NoDoneData`](crate::NoDoneData), which reads as the
    /// empty string, for one without: under `--no-default-features` the string
    /// is a buffer of the runtime's string bound that such a document could
    /// never fill. Emitted by every generated policy, like
    /// [`Scheduler`](StatePolicy::Scheduler).
    type DoneData: crate::DoneDataStorage;

    // ──────────────────────────────────────────────
    // The document, as Appendix D's procedures read it (docs/adr/0064)
    //
    // The procedures are compiled once for every program, so the document
    // reaches them as tables and its states, histories and events as numbers.
    // The generator writes all of these; like the associated types above they
    // have no default.
    // ──────────────────────────────────────────────

    /// The document's tables: its states in document order, its histories, its
    /// transitions and the bounds its buffers are sized by.
    const TABLES: &'static DocumentTables<'static>;

    /// The states, by [`StateId`]: `STATES[state_id(s).index()] == s`.
    const STATES: &'static [Self::State];

    /// The histories, by [`HistoryId`]; empty for a document with none.
    const HISTORIES: &'static [Self::History];

    /// The events, by [`EventId`]: `EVENTS[event_id(e).index()] == e`.
    const EVENTS: &'static [Self::Event];

    /// The number of `state` in [`TABLES`](StatePolicy::TABLES): its position in
    /// document order. The generated state enum's discriminant.
    fn state_id(state: Self::State) -> StateId;

    /// The number of `history` in [`TABLES`](StatePolicy::TABLES).
    fn history_id(history: Self::History) -> HistoryId;

    /// The number of `event`. The generated event enum's discriminant.
    fn event_id(event: Self::Event) -> EventId;

    /// The form this document is printed in (docs/adr/0129): `None` for the
    /// shared interpreter, whose main loop and Appendix D procedures are one
    /// copy in the program and read [`TABLES`](StatePolicy::TABLES);
    /// `Some(StepFunctionForm::of())` for a document of the static class the
    /// build printed as its own step function, whose policy implements
    /// [`StepFunction`](crate::StepFunction). The build that generates the
    /// machines of an image chooses, document by document; a policy that says
    /// nothing is interpreted.
    const STEP_FUNCTION: Option<StepFunctionForm<Self>> = None;

    /// The buffers one call into Appendix D's procedures borrows, of the sizes
    /// this document needs (docs/adr/0056): `Workspace<STATES, ENABLED, WALK,
    /// DEFAULTS>`.
    type Workspace: Buffers + Default;

    /// The configuration and history records the engine keeps, of the sizes
    /// this document needs: `RunStore<CONFIGURATION, RECORDS, HISTORIES>`.
    type RunStore: RunStorage;

    /// The configuration as [`EngineState::get_active_states`] returns it (§scxml-3.4):
    /// `BoundedVec<State, N>` with `N` the most states one configuration holds,
    /// a `Vec` under std and a `heapless::Vec` of exactly that capacity without
    /// it.
    type Configuration: Bounded<Self::State>;

    /// Why this machine's start can be refused (docs/adr/0096):
    /// [`ScriptSessionFailure`](crate::ScriptSessionFailure) for a machine with a
    /// script engine, whose session may not be established, and
    /// [`core::convert::Infallible`] for one without, whose start cannot be
    /// refused that way, so a host starts it with
    /// `engine.initialize().unwrap_or_else(|never| match never {})`.
    ///
    /// [`Engine::initialize`](crate::Engine::initialize) returns it as it is; the
    /// refusals that hold every kind of failure ([`RootStartRefusal`](crate::RootStartRefusal),
    /// [`ConfigurationRejection`](crate::ConfigurationRejection)) hold it as a
    /// [`ScriptSessionFailure`](crate::ScriptSessionFailure), which is what the
    /// `Into` bound is for.
    type StartFailure: Into<crate::ScriptSessionFailure> + Debug;

    // ──────────────────────────────────────────────
    // Feature flags (C++ `static constexpr bool HAS_...`)
    //
    // The engine branches on these at compile time via `if P::HAS_X { ... }`.
    // Rust's const propagation eliminates the branch entirely when false,
    // matching C++ `if constexpr`.
    // ──────────────────────────────────────────────

    /// Whether the SCXML document contains any `<parallel>` states (§scxml-3.4).
    const HAS_PARALLEL_STATES: bool = false;

    /// Whether ECMAScript expression evaluation is required (guards, assigns, etc.).
    ///
    /// When `true`, the engine will call [`initialize_data_model`](StatePolicy::initialize_data_model)
    /// during [`Engine::initialize`](crate::Engine::initialize), which establishes
    /// the machine's script session against the `IScriptEngine` instance the
    /// policy received via `Policy::new(script_engine)` (Engine DI Parity RFC).
    /// That is the one place the session is made (docs/adr/0096).
    const NEEDS_SCRIPT_ENGINE: bool = false;

    /// Whether the document has any `<datamodel>` variables requiring script-engine initialization.
    ///
    /// When `true`, [`initialize_data_model`](StatePolicy::initialize_data_model) is called
    /// before entering the initial configuration (§scxml-5.3).
    const NEEDS_DATA_MODEL_INIT: bool = false;

    /// Whether driving this machine requires [`Engine::tick`](crate::Engine::tick)
    /// rather than [`Engine::step`](crate::Engine::step) alone.
    ///
    /// `true` when the document carries a delayed `<send>` — which the spec's
    /// send section routes through the event scheduler — or an `<invoke>`d
    /// child that does. `step` runs a macrostep and never consults
    /// the scheduler, so a host that only ever calls it gets no delayed event,
    /// no error and no warning. That silence is what this constant exists to
    /// end: a consumer can branch on it to decide whether its driving loop
    /// needs a clock, and the engine counts the macrosteps taken while nothing
    /// polled the scheduler (see
    /// [`unattended_scheduler_steps`](crate::EngineState::unattended_scheduler_steps)).
    ///
    /// The same fact reaches CLI consumers as the generate manifest's
    /// `needs_event_scheduler`; before this constant, a `build.rs` consumer of
    /// [`compile_scxml`](https://docs.rs/sce-build) had no route to it at all,
    /// because that function returns `()`.
    const NEEDS_EVENT_SCHEDULER: bool = false;

    /// Whether the document sends to its parent session — a literal
    /// `<send target="#_parent">` (§scxml-6.2.4). Started by a host rather
    /// than invoked, such a machine runs, and each such send raises
    /// `error.communication`; a host that wants that refused instead starts
    /// it with [`initialize_as_root`](crate::Engine::initialize_as_root),
    /// which reads this constant. The generate manifest's `needs_parent`.
    const NEEDS_PARENT: bool = false;

    /// Whether the document has any static `<invoke>` children (§scxml-6.4).
    ///
    /// When `true`, [`execute_pending_invokes`](StatePolicy::execute_pending_invokes)
    /// is called after macrostep completion to start deferred child state machines.
    const HAS_INVOKE_SUPPORT: bool = false;

    /// Whether the document's children receive parent events via `<finalize>` (§scxml-6.5).
    ///
    /// When `true`, [`execute_finalize_for_child_event`](StatePolicy::execute_finalize_for_child_event)
    /// runs before each parent event is routed to the parent's transitions.
    const HAS_FINALIZE: bool = false;

    /// Whether the document autoforward child events to any invokes (§scxml-6.4.1).
    const HAS_AUTOFORWARD: bool = false;

    /// Whether the policy supports child-tick for nested invokes.
    const HAS_CHILD_TICK: bool = false;

    /// Whether the document hands an `<invoke>` to a host invoker: a type the
    /// host declared it runs, or a Mesh request
    /// ([`HOST_INVOKE_IDS`](StatePolicy::HOST_INVOKE_IDS) names them).
    ///
    /// When `false` the engine starts no host-run invocation, so none can end,
    /// and the code that starts one, ends one or lets its deadline pass is
    /// left out of the machine: a program pays for that code only in the
    /// machines whose documents use it. A start asked for anyway is answered
    /// as one no invoker ran, and an ending is refused as one for a start that
    /// is not running.
    ///
    /// `true` by default, so a policy written by hand keeps every host-run
    /// path; the generator writes `false` for a document with no such
    /// `<invoke>`.
    const HAS_HOST_INVOKES: bool = true;

    // ──────────────────────────────────────────────
    // Static metadata methods (C++ `constexpr static`)
    //
    // These encode the SCXML document structure at compile time — state
    // hierarchy, document order, event name tables. They take no `&self`
    // because the data is baked into the generated source.
    // ──────────────────────────────────────────────

    /// The state the engine names before
    /// [`Engine::initialize`](crate::Engine::initialize) enters the
    /// initial configuration, and the state enum's `Default` (§scxml-3.2).
    ///
    /// Not what `initialize` enters: that is the document's initial transition
    /// (§scxml-3.2), the row [`DocumentTables::initial`] names, which may name
    /// several states and a `<history>`.
    ///
    /// The state [`DocumentTables::default_state`] numbers: the tables are the
    /// one place it is written, so the engine (which holds the number) and
    /// this answer cannot disagree.
    fn initial_state() -> Self::State {
        Self::state_of(Self::TABLES.default_state)
    }

    /// The state numbered `id`.
    ///
    /// The engine holds a state as the number its tables give it, so a number
    /// past `STATES` is a generator fault, which stops at the one cold call
    /// every row read past its table shares: the read is a compare and a
    /// branch, with no bounds check that formats the index.
    fn state_of(id: StateId) -> Self::State {
        match Self::STATES.get(id.index()) {
            Some(state) => *state,
            None => crate::helpers::tables::row_missing(),
        }
    }

    /// The event numbered `id`: where a behaviour hook, which is handed the
    /// number (docs/adr/0123), needs the event as this machine's own value, to
    /// read its name or a typed payload.
    fn event_of(id: EventId) -> Self::Event {
        match Self::EVENTS.get(id.index()) {
            Some(event) => *event,
            None => crate::helpers::tables::row_missing(),
        }
    }

    /// The `<history>` numbered `id`: where a behaviour hook, which is handed
    /// the number (docs/adr/0123), needs it as this machine's own value.
    fn history_of(id: HistoryId) -> Self::History {
        match Self::HISTORIES.get(id.index()) {
            Some(history) => *history,
            None => crate::helpers::tables::row_missing(),
        }
    }

    /// Whether `state` is a `<final>` state (§scxml-3.7).
    fn is_final_state(state: Self::State) -> bool {
        Self::TABLES.is_final(Self::state_id(state))
    }

    /// The parent of `state` in the document hierarchy, or `None` if it's a root child.
    fn get_parent(state: Self::State) -> Option<Self::State> {
        Self::TABLES
            .parent(Self::state_id(state))
            .map(Self::state_of)
    }

    /// Whether `state` is a compound state: a `<state>` with child states
    /// (§scxml-3.3). A `<parallel>` answers `false` — this is Appendix D's
    /// `isCompoundState`, and it decides which ancestors can be a transition's
    /// domain.
    fn is_compound_state(state: Self::State) -> bool {
        Self::TABLES.is_compound(Self::state_id(state))
    }

    /// Whether `state` is a `<parallel>` state (§scxml-3.4).
    fn is_parallel_state(state: Self::State) -> bool {
        Self::TABLES.is_parallel(Self::state_id(state))
    }

    /// §scxml-3.10: the state a `<history>` is declared in.
    fn get_history_parent(history: Self::History) -> Self::State {
        Self::state_of(Self::TABLES.history(Self::history_id(history)).parent())
    }

    /// Document order index of `state` (W3C SCXML Appendix D): its number.
    ///
    /// Document order is also entry order, and its reverse is exit order.
    fn get_document_order(state: Self::State) -> u32 {
        u32::from(Self::state_id(state).0)
    }

    /// Human-readable name of `event` (e.g., `"error.execution"`, `"done.state.s1"`).
    ///
    /// Used for `_event.name` population, logging, and HTTP send payloads.
    fn get_event_name(event: Self::Event) -> &'static str;

    /// Reverse lookup: `Some(event)` if `name` matches a known event, else `None`.
    ///
    /// Used by `raiseExternal(const std::string&)` overload and child invoke autoforward.
    fn get_event_from_name(name: &str) -> Option<Self::Event>;

    /// The event an event that arrives BY NAME is delivered as (§scxml-3.12.1).
    ///
    /// An event arrives by name from outside the document — a child's
    /// autoforward, a host — so the names it can arrive under are open, while
    /// the enum holds only the names the document writes. A transition's
    /// descriptor matches an event by whole tokens: `request` matches
    /// `request.new` whether or not the document writes `request.new`. Every
    /// descriptor that matches an arriving name is a token prefix of it, so is
    /// one of the document's names, so is a prefix of the LONGEST of those —
    /// which therefore matches exactly what the arriving name would. That is the
    /// event this answers: `name` itself when the document writes it, else the
    /// longest token prefix of `name` it does, else the wildcard member — the
    /// one `event="*"` stands for, which the exact table keeps under the name
    /// `*` (no document can write that as an event's name) — when the document
    /// has such a transition, else `None`: no transition the document has could
    /// match it, and it is dropped.
    ///
    /// Provided, not generated: the rule is one, and a generated copy per
    /// document is a copy that can age. [`get_event_from_name`](Self::get_event_from_name)
    /// stays the exact table.
    fn resolve_event_by_name(name: &str) -> Option<Self::Event> {
        let mut candidate = name;
        loop {
            if let Some(event) = Self::get_event_from_name(candidate) {
                return Some(event);
            }
            match candidate.rfind('.') {
                Some(at) => candidate = &candidate[..at],
                None => return Self::get_event_from_name("*"),
            }
        }
    }

    /// The ids of the `<invoke>`s this document hands to a host invoker
    /// (§scxml-6.4.1), emitted when the build declared one.
    ///
    /// Their `done.invoke.<id>` is accepted only through
    /// `EngineState::complete_host_invoke` (a plain label: that method does not
    /// exist on the no_std profile, where this constant still does), the one
    /// path that knows the invocation is still running; the engine
    /// refuses one raised any other way. Empty — nothing refused — for a
    /// document with no host-run invoke, whose `done.invoke` events all come
    /// from SCXML children.
    const HOST_INVOKE_IDS: &'static [&'static str] = &[];

    /// Human-readable name of `state` (e.g., `"s0"`, `"passingState"`).
    ///
    /// Required (no default): the State→id mapping is structural and external
    /// consumers (trace recorders, post-mortem analyzers, the generated
    /// `In()` predicate callback) need it regardless of whether the SM uses
    /// parallel states. Mirrors `get_event_name` (also required) — see the
    /// C++ `StateNamingPolicyTrait` in `sce/include/core/StatePolicyConcepts.h`.
    fn get_state_name(state: Self::State) -> &'static str;

    /// Reverse lookup: `Some(state)` if `name` matches a known state, else `None`.
    ///
    /// Required (no default) for the reason `get_state_name` is: the mapping is
    /// structural, and a default could only ever answer `None`. A policy that
    /// had not emitted the table would then report every recorded configuration
    /// as unknown — which a caller reads as "this run has no history" rather
    /// than as a policy that is incomplete. Mirrors `get_event_from_name`,
    /// which is required for the same reason on the event side.
    ///
    /// This is what lets a configuration cross a process. A host can only
    /// record state *names* — a journal, a wire, a file — while
    /// [`Engine::enter_at`](crate::Engine::enter_at) takes a slice of
    /// `Self::State`.
    /// Without the reverse, a recorded configuration cannot be turned back into
    /// the argument that door asks for and resuming degrades to replaying from
    /// the initial state. A consumer-side table would age silently the moment
    /// the document gained a state; only the generator writes one that ages
    /// with the document.
    ///
    /// The round trip is an identity: `get_state_from_name(get_state_name(s))`
    /// is `Some(s)` for every state of the document, and a name the document
    /// does not carry is `None` rather than a guess.
    fn get_state_from_name(name: &str) -> Option<Self::State>;

    /// Sentinel event value for eventless transition dispatch (§scxml-3.13).
    ///
    /// Generated code produces an `Event::Null` variant. Its number is the
    /// tables' [`null_event`](DocumentTables::null_event), the event the
    /// engine selects eventless transitions with.
    fn null_event() -> Self::Event;

    // ──────────────────────────────────────────────
    // Instance methods — generated executable content
    //
    // These mirror C++ policy methods that take `Engine&` as a parameter.
    // Generated code mutates the policy via `&mut self` and calls engine
    // methods through the `engine` parameter, the engine's `EngineState`:
    // everything of the engine but the policy, borrowed apart from it, so the
    // two references a hook holds are to two places (docs/adr/0131). A method
    // that needs the policy is `Engine`'s and is not reachable from a hook,
    // where the policy is `self` already. Each answers for ONE state or
    // ONE transition: which states a microstep exits and enters, and in which
    // order, is the engine's Appendix D procedure, not the policy's.
    //
    // The hooks the procedures call are handed the numbers the procedures
    // speak — a `StateId`, an `EventId`, a `HistoryId`, a `TransitionId`
    // — the numbers `TABLES` is indexed by, so structure and behaviour are
    // one numbering (docs/adr/0123). The engine converts nothing on the way
    // in; a hook that needs the machine's own value converts where it needs
    // it, with `state_of`, `event_of` or `history_of`.
    //
    // Every hook that runs a guard or executable content is handed
    // `event_data`, the data of the event being processed
    // ([`EventData`](StatePolicy::EventData)), as an argument: one signature
    // for every machine, whether that data was queued or lent, and nothing
    // the policy keeps can hold it past the call (docs/adr/0124).
    // ──────────────────────────────────────────────

    /// §scxml-5.10: bind the event whose transitions are about to be selected
    /// as the `_event` their guards read.
    ///
    /// Called once per selection, before the first guard runs, and with the
    /// tables' [`null_event`](DocumentTables::null_event) for an eventless
    /// selection — which has no event of its own, so a policy binds nothing
    /// for it. The default binds nothing: a document whose guards never read
    /// `_event` has nothing to bind.
    fn bind_current_event(&mut self, _event: EventId, _engine: &mut EngineState<Self>) {}

    /// Appendix D selectTransitions, the half only the document can answer,
    /// for tables written by hand: the first of `state`'s own transitions, in
    /// document order, that `event` enables and whose guard holds. The tables'
    /// [`null_event`](DocumentTables::null_event) asks for eventless
    /// transitions.
    ///
    /// Ports C++ `firstEnabledTransition(State, Event, Engine&)`. The engine
    /// walks the atomic states and their ancestors and keeps the ordered set.
    /// The state, the event and the answer are numbers of
    /// [`TABLES`](StatePolicy::TABLES): the answer is the transition's row,
    /// which holds its source, targets and type. `event_data` is `event`'s
    /// data, which a guard reads as `_event.data`.
    ///
    /// Asked only when [`TABLES`](StatePolicy::TABLES) carry no
    /// [`selection`](DocumentTables::selection) rows. A generated machine's
    /// tables carry them, and one selection reads them for every machine of a
    /// program, asking the policy [`guard_holds`](StatePolicy::guard_holds)
    /// instead (docs/adr/0127), so a generated policy does not write this
    /// method. A policy whose tables carry no rows writes it; one that does not
    /// stops at its first selection, with a message that says so.
    fn first_enabled_transition(
        &mut self,
        _state: StateId,
        _event: EventId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) -> Option<TransitionId> {
        selection_unanswered()
    }

    /// Whether the guard of `transition` holds: a written transition of the
    /// document that the event being selected for enables.
    ///
    /// The selection rows of [`TABLES`](StatePolicy::TABLES) ask it for each
    /// transition the event enables, in the order its state's transitions are
    /// written, until one holds, after
    /// [`bind_current_event`](StatePolicy::bind_current_event) bound the
    /// event, and never for a transition the event does not enable
    /// (docs/adr/0127). So it is the only place a generated machine evaluates
    /// a guard, and a guard runs exactly where the document's own selection
    /// would have run it. `event_data` is the event's data, which a guard
    /// reads as `_event.data`.
    ///
    /// The default holds: a document with no guard writes none, and the
    /// policy of tables with no selection rows is never asked.
    fn guard_holds(
        &mut self,
        _transition: TransitionId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) -> bool {
        true
    }

    /// Whether a transition of the document takes something before the
    /// microstep exits anything ([`take_transition`](StatePolicy::take_transition)):
    /// a branch of a test component (docs/SCE_TEST_PROFILE.md §4.4, P4 step 1).
    /// The engine asks the policy only where this holds, so the policy of a
    /// statechart in a build that also holds a test component is never asked.
    ///
    /// Only with the crate's `test-component` feature, as is
    /// [`take_transition`](StatePolicy::take_transition): a build without it
    /// has no take step at all, and the policy of a test component, which
    /// names both, compiles only where the feature is on.
    #[cfg(feature = "test-component")]
    const TAKES_TRANSITIONS: bool = false;

    /// Take one selected transition before the microstep exits any state: a
    /// branch of a test component removes what it matched from the round's
    /// snapshot, writes its record in the test log, and holds a message it
    /// took as the payload its content reads (docs/SCE_TEST_PROFILE.md §4.4,
    /// P4 step 1, §12.3). Asked only where
    /// [`TAKES_TRANSITIONS`](StatePolicy::TAKES_TRANSITIONS) holds; the
    /// default takes nothing. Only with the crate's `test-component` feature.
    #[cfg(feature = "test-component")]
    fn take_transition(&mut self, _transition: TransitionId, _engine: &mut EngineState<Self>) {}

    /// Execute one transition's executable content (§scxml-3.13 — run between
    /// the microstep's exits and its entries).
    ///
    /// `transition` is one the selection returned, and one whose row says it
    /// has content. Ports C++ `executeTransitionActions(State, int,
    /// Engine&)`. `event_data` is the data of the event that selected it.
    fn execute_transition_content(
        &mut self,
        transition: TransitionId,
        event_data: Self::EventData<'_>,
        engine: &mut EngineState<Self>,
    );

    /// Enter `state` (§scxml-3.8): run its `<onentry>`, and its initial
    /// transition's content when `is_default_entry`. The engine has added it to
    /// the configuration already.
    ///
    /// Ports C++ `executeEntryActions(State, Engine&, bool)`. May:
    /// - raise internal events via `engine.raise(...)`
    /// - schedule delayed sends via `engine.schedule_event(...)`
    /// - mutate datamodel variables on `self`
    /// - defer `<invoke>` starts until the configuration is stable (§scxml-6.4)
    ///
    /// One state, and nothing below it: which states a microstep enters is the
    /// engine's Appendix D entry set, entered front to back, so this neither
    /// enters regions nor descends to an initial child. `is_default_entry` is
    /// the entry set's `statesForDefaultEntry` answer — a compound state
    /// entered only as an ANCESTOR of a deeper target was not entered by
    /// default, and its initial transition content does not run. For a
    /// `<final>`, this is also what the appendix does on entering one.
    /// `event_data` is the data of the event being processed, if any.
    fn execute_entry_actions(
        &mut self,
        state: StateId,
        event_data: Self::EventData<'_>,
        engine: &mut EngineState<Self>,
        is_default_entry: bool,
    );

    /// Exit `state` (§scxml-3.9): run its `<onexit>` and cancel its
    /// invocations. The engine has recorded its histories already (§scxml-3.10)
    /// and removes it from the configuration afterwards.
    ///
    /// Ports C++ `executeExitActions(State, Engine&, const vector<State>&)`.
    /// One state, and nothing below it: the engine's Appendix D exit set
    /// already holds every active descendant, in exit order, ahead of this
    /// state. `event_data` is the data of the event being processed, if any.
    fn execute_exit_actions(
        &mut self,
        state: StateId,
        event_data: Self::EventData<'_>,
        engine: &mut EngineState<Self>,
    );

    /// §scxml-3.10.2: a `<history>`'s default transition content, run after
    /// its parent's onentry (and after the parent's own initial content) when
    /// the history was taken with nothing recorded.
    ///
    /// The default runs nothing, which is right for a history whose default
    /// transition has no content and for a document with no history at all.
    fn execute_history_default_content(
        &mut self,
        _history: HistoryId,
        _event_data: Self::EventData<'_>,
        _engine: &mut EngineState<Self>,
    ) {
    }

    // ──────────────────────────────────────────────
    // Optional instance methods (default no-op; overridden when the
    // corresponding feature flag is `true`)
    // ──────────────────────────────────────────────

    /// Initialize the datamodel via the script engine (§scxml-5.3).
    ///
    /// Generated only when `NEEDS_DATA_MODEL_INIT` is `true`. Called from
    /// [`Engine::initialize`](crate::Engine::initialize) and
    /// [`Engine::enter_at`](crate::Engine::enter_at) before any state entry.
    /// It establishes the script session first, and a session that cannot be
    /// established is the error: the policy releases what the failed steps made,
    /// and the engine refuses the start (docs/adr/0096).
    fn initialize_data_model(
        &mut self,
        _engine: &mut EngineState<Self>,
    ) -> Result<(), Self::StartFailure> {
        Ok(())
    }

    /// Execute any pending `<invoke>` elements deferred during entry (§scxml-6.4).
    ///
    /// Generated only when `HAS_INVOKE_SUPPORT` is `true`.
    fn execute_pending_invokes(&mut self, _engine: &mut EngineState<Self>) {}

    /// The state whose `<invoke>` the document gives the id `invoke_id`
    /// (§scxml-6.4), or `None` when it has no such invoke — what a restore
    /// judges a saved invocation against.
    ///
    /// Generated only when `HAS_INVOKE_SUPPORT` is `true` and the document
    /// saves. `std` only, as the saved state is.
    #[cfg(feature = "std")]
    fn invoke_owner(_invoke_id: &str) -> Option<Self::State> {
        None
    }

    /// The state whose `<invoke>` the document gives the id `invoke_id` and the
    /// host-run `type` `processor_type`, or `None` when it has no such invoke —
    /// what a restore judges a saved host-run invocation against.
    ///
    /// Generated only when the document has an `<invoke>` a declared host
    /// invoker serves and saves. `std` only, as the saved state is.
    #[cfg(feature = "std")]
    fn host_invoke_owner(_processor_type: &str, _invoke_id: &str) -> Option<Self::State> {
        None
    }

    /// The ids of the `<invoke>`s whose child session is running (§scxml-6.4),
    /// in document order. A child that has ended is not running, though the
    /// policy may keep it until its state exits: a restore starts every id
    /// listed here again, and an invocation already complete must not run twice.
    ///
    /// Generated only when `HAS_INVOKE_SUPPORT` is `true` and the document
    /// saves. `std` only, as the saved state is.
    #[cfg(feature = "std")]
    fn running_invokes(&self) -> Vec<&'static str> {
        Vec::new()
    }

    /// Defer the start of the `<invoke>` `invoke_id`, as entering its state
    /// defers it (§scxml-6.4): the next
    /// [`execute_pending_invokes`](StatePolicy::execute_pending_invokes) starts
    /// its child. A restore calls this for each running invocation it was given
    /// and then runs them together.
    ///
    /// Generated only when `HAS_INVOKE_SUPPORT` is `true` and the document
    /// saves. `std` only, as the saved state is.
    #[cfg(feature = "std")]
    fn restart_invoke(&mut self, _invoke_id: &str, _engine: &mut EngineState<Self>) {}

    /// How many times each `<invoke>` that has started in this session has
    /// started, by the id the document gives it, in document order
    /// (docs/adr/0045): what a saved state carries so that a start after a
    /// restore is numbered on.
    ///
    /// Generated only when the document has an `<invoke type="scxml">` and
    /// saves. `std` only, as the saved state is.
    #[cfg(feature = "std")]
    fn invoke_starts(&self) -> Vec<(&'static str, u64)> {
        Vec::new()
    }

    /// Judge `child`, a running child a saved tree of sessions holds, against
    /// the document the `<invoke>` it names starts there — its shape, its
    /// revisions, every value, its own running children — building nothing
    /// (docs/adr/0045, section 3, step 1).
    ///
    /// Generated only when the document has an `<invoke type="scxml">` and
    /// saves. The default refuses: a document with no child to resume has none.
    #[cfg(feature = "std")]
    fn judge_child(
        child: &crate::saved_state::SavedChild,
    ) -> Result<(), crate::saved_state::StateRefusal> {
        Err(crate::saved_state::StateRefusal::new(format!(
            "invokes names '{}', and the document starts no child session to resume",
            child.id
        )))
    }

    /// Build the child `child` holds as starting it would build it — its link
    /// to this session, the clock, its host — and restore it from its saved
    /// session in place of initializing it (docs/adr/0045, section 3, step 3):
    /// no `<onentry>`, no `<data>`, no argument evaluated, nothing sent.
    ///
    /// Generated only when the document has an `<invoke type="scxml">` and
    /// saves. The default refuses, as [`judge_child`](Self::judge_child) does.
    #[cfg(feature = "std")]
    fn resume_child(
        &mut self,
        child: &crate::saved_state::SavedChild,
        _engine: &mut EngineState<Self>,
        _wall_now_ms: u64,
    ) -> Result<(), crate::saved_state::StateRefusal> {
        Err(crate::saved_state::StateRefusal::new(format!(
            "invokes names '{}', and the document starts no child session to resume",
            child.id
        )))
    }

    /// Ask `engine`'s factory of children's routers for the router of each
    /// running child a restore resumed and no factory has been asked for yet
    /// ([`EngineState::start_owed_child_mesh`]; docs/adr/0043, docs/adr/0045). The
    /// engine calls it when that factory is registered or installed.
    ///
    /// Generated only when the document has an `<invoke type="scxml">` and
    /// saves. A policy with no children to resume has none to ask for, which
    /// is what the default says.
    #[cfg(feature = "std")]
    fn start_owed_child_meshes(&mut self, _engine: &mut EngineState<Self>) {}

    /// Hand `invokers`, the tree's host invokers, to every child this session
    /// holds ([`Engine::adopt_host_invokers`](crate::Engine::adopt_host_invokers);
    /// docs/adr/0045).
    ///
    /// Generated when the document has an `<invoke type="scxml">`. A policy
    /// with no children has none to hand them to, which is what the default
    /// says.
    #[cfg(feature = "std")]
    fn adopt_host_invokers_in_children(&mut self, _invokers: &crate::host_processor::TreeInvokers) {
    }

    /// Hand `tree`, the inboxes of the tree, to every child this session
    /// holds ([`Engine::adopt_tree_inboxes`](crate::Engine::adopt_tree_inboxes);
    /// ARCHITECTURE.md "Engine
    /// Threading", item 3).
    ///
    /// Generated when the document has an `<invoke type="scxml">`. A policy
    /// with no children has none to hand it to, which is what the default
    /// says.
    #[cfg(feature = "std")]
    fn adopt_tree_inboxes_in_children(&mut self, _tree: &crate::inbox::TreeInboxes) {}

    /// Route `ending`, a host's ending of a host-run invocation of session
    /// `session_id`, to the child that is that session or holds it in its
    /// tree ([`Engine::route_host_invoke_end`](crate::Engine::route_host_invoke_end);
    /// docs/adr/0045). `None` when
    /// no child is or holds that session.
    ///
    /// Generated when the document has an `<invoke type="scxml">`. A policy
    /// with no children holds no other session, which is what the default
    /// says.
    #[cfg(feature = "std")]
    fn route_host_invoke_end_to_child(
        &mut self,
        _session_id: &str,
        _ending: crate::host_processor::HostInvokeEnding<'_>,
    ) -> Option<Result<(), crate::host_processor::HostInvokeEndRefusal>> {
        None
    }

    /// Execute `<finalize>` handlers for child events (§scxml-6.5).
    ///
    /// Generated only when `HAS_FINALIZE` is `true`. Called from the engine's
    /// external queue processing, before the event is routed to transitions.
    fn execute_finalize_for_child_event(
        &mut self,
        _event: &EventWithMetadata<Self::Event, Self::Payload>,
        _engine: &mut EngineState<Self>,
    ) {
    }

    /// Lift the typed `_event.data` view out of the data an event carries
    /// (EventSchema native lowering), into `payload`, the payload its queue
    /// handed over, which the engine then holds while the event is `_event`
    /// (docs/adr/0124).
    ///
    /// A typed payload is filled by ONE producer: the generated `raise_<event>`
    /// inject seam, and a `payload` it filled is left as it is. Every other
    /// producer — `<send>` with `<param>`, namelist or `<content>`, an invoke
    /// forwarding an event either way, autoforward, BasicHTTP, mesh — fills [`EventMetadata::data`](crate::EventMetadata),
    /// the wire §scxml-5.10 describes and §scxml-B-2-8-1 reads. This reads
    /// the schema's fields out of that when no typed payload rode with the
    /// event, so the same guard answers the same way whichever producer sent
    /// it — including one on the far side of an invoke boundary.
    ///
    /// `Err` says the data cannot be read as this event's schema. The engine
    /// refuses the delivery whole (docs/adr/0086, section 4.2): no transition
    /// is selected for it, and an `error.execution` carrying the refusal is
    /// taken in its place.
    ///
    /// ⚠ `std` only, because [`EventMetadata::data`](crate::EventMetadata) is:
    /// a `no_std` build has no wire to lift from and no script engine to
    /// disagree with, so there the typed carrier is the whole channel.
    #[cfg(feature = "std")]
    fn lift_event_payload(
        _event: Self::Event,
        _data: &str,
        _payload: &mut Self::Payload,
    ) -> Result<(), crate::event_payload::PayloadRefusal> {
        Ok(())
    }

    /// The door of data a host lends with its event
    /// ([`Engine::process_event_with`](crate::Engine::process_event_with);
    /// docs/adr/0124, D2): hold it to what a `std` build's `lift_event_payload`
    /// (no item of a `no_std` build, so not a link) holds the same data to when
    /// a wire carries it, the bounds of its texts
    /// and byte strings among them. Data that enters a machine keeps one
    /// contract whichever route it takes.
    ///
    /// `Err` refuses the delivery whole, exactly as a lift's refusal does: an
    /// `error.execution` carrying the refusal is taken in its place, or, in a
    /// document that declares none, it is dropped and counted. The default
    /// admits everything, which is right for a policy whose data has no bound
    /// its type does not already hold.
    ///
    /// One signature on every profile (docs/adr/0053: a crate built without
    /// `std` still compiles where another crate of the build turns it on): the
    /// breach is a value of no allocation, which a `std` build words as the
    /// lift's refusal and a `no_std` build, which has no wire and no refusal to
    /// answer with, takes as a delivery that carries no data, as it takes any
    /// delivery whose typed payload is not there.
    fn admit_event_data(_event_data: Self::EventData<'_>) -> Result<(), LentBreach> {
        Ok(())
    }

    /// Forward external events to autoforward children (§scxml-6.4.1).
    ///
    /// Generated only when `HAS_AUTOFORWARD` is `true`.
    ///
    /// §scxml-6.4 requires an *exact copy* of the source event to reach the
    /// child, so the metadata travels alongside the name: the child must see
    /// the same `_event.data`, `_event.origin`, `_event.sendid`,
    /// `_event.origintype` and `_event.invokeid` the parent saw. The name is
    /// passed separately because it is the only identity the two machines
    /// share — the child's `Event` enum is an unrelated type. Under `no_std`
    /// every metadata field except `event_type` is elided from
    /// [`EventMetadata`], so nothing extra crosses on MCU targets by
    /// construction.
    fn forward_to_autoforward_children(
        &mut self,
        _event_name: &str,
        _metadata: &EventMetadata,
        _engine: &mut EngineState<Self>,
    ) {
    }

    /// Tick child state machines (§scxml-6.4).
    ///
    /// Generated only when `HAS_CHILD_TICK` is `true`. Called from
    /// [`Engine::tick`](crate::Engine::tick) to propagate scheduler ticks to children.
    fn tick_children(&mut self, _engine: &mut EngineState<Self>) {}

    /// §scxml-6.4: the nearest deadline of any running child, in milliseconds
    /// from now, or `None` when no child has one.
    ///
    /// Generated only when `HAS_CHILD_TICK` is `true`. A child is ticked by its
    /// parent ([`tick_children`](Self::tick_children)), so a moment a child
    /// needs is a moment of this machine's clock too, and
    /// [`Engine::time_until_next_scheduled_ms`](crate::Engine::time_until_next_scheduled_ms)
    /// counts it: a host that walks time by that answer must not step over a
    /// timer the child armed. A child that has ended answers nothing, and a
    /// child's own children count through the child's own answer.
    fn next_child_deadline_ms(&self) -> Option<u64> {
        None
    }

    /// §scxml-6.4: move every running child's host-owned clock to `clock`,
    /// the parent's, as the parent opens a turn — see
    /// [`EngineState::follow_clock`](crate::EngineState::follow_clock).
    ///
    /// A policy that invokes nothing has no child to move, which is what the
    /// default says.
    fn follow_clock_in_children(&mut self, _clock: crate::SceClock) {}

    /// §scxml-6.2 + §scxml-6.4: deliver a delayed `<send target="#_<invokeid>">`
    /// whose wait is over to that invocation, by name — a child need not
    /// declare every event its parent sends.
    ///
    /// Answers whether the invocation was there to take it: one whose session
    /// has reached its final state has ended, and the engine reports the send
    /// it could not deliver (§scxml-C-1). A policy with no invocations has
    /// none, which is what the default says.
    ///
    /// The child takes the event at once, and what it sends its parent while it
    /// does, and its ending, reach this session within the delivery, on
    /// `engine` (docs/adr/0045): nothing a delivery produced waits in the
    /// child's buffer for a later drive.
    ///
    /// `sender` is the id of the session that sent the event, this one: the
    /// child reads the location it publishes as `_event.origin`
    /// (docs/adr/0090, section 2).
    fn deliver_to_invocation(
        &mut self,
        _invoke_id: &str,
        _event_name: &str,
        _event_data: &str,
        _sender: &str,
        _engine: &mut EngineState<Self>,
    ) -> bool {
        false
    }

    /// §scxml-6.2 + §scxml-6.4: deliver a delayed `<send target="#_parent">`
    /// whose wait is over to the session that invoked this one, with the
    /// payload an immediate one carries.
    ///
    /// Answers whether that session was there to take it. A policy that was
    /// not invoked has no parent, which is what the default says.
    fn deliver_to_parent(&mut self, _event_name: &str, _event_data: &str) -> bool {
        false
    }

    /// §scxml-C-1: whether a session invoked this one — asked when a
    /// `<send targetexpr>` names `#_parent`, so a session with none hears
    /// error.communication when the send is made, delayed or not.
    ///
    /// A policy that was not invoked has no parent, which is what the
    /// default says.
    fn has_parent_session(&self) -> bool {
        false
    }

    /// Whether `session_id` names the session that invoked this one. The location a session publishes is a target, so a reply sent to
    /// the origin of an event the parent sent — its location — reaches the
    /// parent as `#_parent` does (docs/adr/0090).
    ///
    /// A policy that was not invoked has no parent, which is what the
    /// default says.
    fn is_parent_session(&self, _session_id: &str) -> bool {
        false
    }

    /// This session's own id (`_sessionid`; docs/adr/0045): the one its host
    /// gave or the runtime drew when it was created, the one its parent gave a
    /// child, or the one a restore bound. Every generated policy has one from
    /// its construction on, a `no_std` one included; the empty text is the
    /// default of a policy written by hand that binds none.
    fn own_session_id(&self) -> &str {
        ""
    }

    /// Bind this session to `session_id`, the id a saved tree of sessions holds
    /// for it, in place of the one it was created with: a restored session
    /// carries on, so every child's id and a peer's `#_scxml_<id>` address
    /// still name it (docs/adr/0045). A policy that holds no id has none to
    /// bind, which is what the default says.
    #[cfg(feature = "std")]
    fn bind_session_id(&mut self, _session_id: &str) {}

    /// §scxml-C-1: deliver a send addressed to one of this machine's children
    /// by the session id its published location names.
    ///
    /// Answers whether that child was there to take it. A policy with no
    /// children has none, which is what the default says. What the child sends
    /// its parent as it takes the event reaches this session on `engine` within
    /// the delivery, as for [`deliver_to_invocation`](Self::deliver_to_invocation),
    /// and `sender` is what it is there.
    fn deliver_to_child_session(
        &mut self,
        _child_session_id: &str,
        _event_name: &str,
        _event_data: &str,
        _sender: &str,
        _engine: &mut EngineState<Self>,
    ) -> bool {
        false
    }

    /// §scxml-6.4 + §scxml-C-1: whether `#_<invokeid>` names an invocation
    /// that is running now — one that has not started, or whose session has
    /// reached its final state, is not there to address, and a send naming it
    /// is reported when it is made.
    fn is_invocation_running(&self, _invoke_id: &str) -> bool {
        false
    }

    /// §scxml-C-1: whether a session id names a child of this machine that is
    /// running now — the test [`deliver_to_child_session`](Self::deliver_to_child_session) makes.
    fn is_child_session_running(&self, _child_session_id: &str) -> bool {
        false
    }

    /// §scxml-5.10: Populate pending event metadata fields from an event's metadata.
    ///
    /// Ports C++ `EventMetadataHelper::populatePolicyFromMetadata`. Called by the engine
    /// before dispatching each event from the internal/external queues. Generated code
    /// stores the metadata in `pending_event_*` struct fields so that `process_transition`
    /// can pass them to `set_current_event_in_script_engine`.
    fn populate_event_metadata(&mut self, _metadata: &crate::event::EventMetadata) {}

    /// §scxml-5.10: Clear pending event metadata after transition processing.
    ///
    /// Ports C++ `EventMetadataHelper::clearPolicyMetadata`. Called by the engine
    /// after each event dispatch cycle to reset metadata for the next event.
    fn clear_event_metadata(&mut self) {}
}

/// A policy asked [`StatePolicy::first_enabled_transition`] when its tables
/// carry no selection rows and it does not write the method: a policy written
/// by hand that is missing a method its tables need. No formatted message and
/// no argument, for the reason a table's missing row has none.
#[cold]
#[inline(never)]
fn selection_unanswered() -> ! {
    panic!(
        "a policy whose tables carry no selection rows answers first_enabled_transition itself, \
         and this one does not"
    )
}
