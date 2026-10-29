// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! - **Instance methods** (`execute_entry_actions()`,
//!   `first_enabled_transition()`, etc.) mirror C++ non-static policy methods.
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

use crate::event::{EventMetadata, EventWithMetadata};
use crate::hal::Hal;
use crate::helpers::event_queue::EventQueueLike;
use crate::helpers::hierarchy::Bounded;
use crate::helpers::microstep::Buffers;
use crate::helpers::run_store::RunStorage;
use crate::helpers::tables::{DocumentTables, EventId, HistoryId, StateId, TransitionId};
use crate::Engine;

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
    /// [`Target::History`](crate::helpers::tables::Target::History), and the
    /// entry procedures dereference it to what it recorded or to its default.
    type History: Copy + Eq + Debug + 'static;

    /// Typed event payload for EventSchema native lowering.
    ///
    /// `()` for schemaless documents — the dynamic `_event.data` baseline keeps
    /// riding in [`EventMetadata::data`](crate::EventMetadata) as before. For a
    /// document that imports `EventSchema`s, generated code sets this to the
    /// per-document payload sum (`<Doc>Payload`) so that `_event.data.<field>`
    /// guards lower to native field reads with no script engine — the no_std MCU
    /// value path. The payload rides with its event through the queues in
    /// [`EventWithMetadata::payload`](crate::EventWithMetadata) and is copied to
    /// the policy at dispatch via
    /// [`populate_event_payload`](StatePolicy::populate_event_payload).
    ///
    /// `Default` supplies the slot for payloadless/schemaless events (the queues
    /// are homogeneous in `Self::Payload`); `Clone`/`Debug` mirror the
    /// `EventWithMetadata` derives; `'static` because payloads carry no borrows.
    type Payload: Clone + Default + Debug + 'static;

    /// HAL impl bound to this policy (SCE Protocol-Synthesis RFC §synth-5-J-2 line 1984).
    ///
    /// Determines which [`Hal`] impl the [`Engine`] dispatches `ticks` /
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

    /// Event-queue type backing this machine's W3C SCXML Appendix D
    /// `internalQueue` / `externalQueue` (the [`Engine`] holds
    /// one of each).
    ///
    /// Carries the machine's FIFO **depth** under `--no-default-features`: generated
    /// code emits `EventQueueManager<EventWithMetadata<Self::Event,
    /// Self::Payload>, N>` where `N` is the per-document capacity resolved from
    /// `<scxml sce:capacity="N">` / deploy `default_event_queue_capacity`
    /// (emitted as `EVENT_QUEUE_CAPACITY`), defaulting to
    /// [`MAX_EVENT_QUEUE_DEPTH`](crate::MAX_EVENT_QUEUE_DEPTH) when the document
    /// declares no capacity. Under the std build the depth is inert
    /// (`VecDeque`, unbounded — the spec's unbounded `Queue`), so the same
    /// emission compiles on both runtime profiles.
    ///
    /// This is the SSOT for the no_std queue size: the bound lives in the
    /// machine (its authored capacity), not in a runtime-crate global. Mirrors
    /// the per-machine [`Payload`](StatePolicy::Payload) / [`Hal`](StatePolicy::Hal)
    /// associated types; like them it has no default (stable Rust forbids
    /// default associated types) and is emitted by every generated policy.
    type EventQueue: EventQueueLike<EventWithMetadata<Self::Event, Self::Payload>> + Default;

    /// Storage for the delayed-send scheduler's per-entry cancel key
    /// (`send_id`), backing §scxml-6.3 `<cancel sendid>`.
    ///
    /// The scheduler keeps a `send_id` per pending entry purely so `<cancel>`
    /// can find and drop the matching one; the id is read only by
    /// [`Engine::cancel_event`](crate::Engine::cancel_event) and never reaches a
    /// fired event's metadata (the timer drain passes an empty `send_id`). A
    /// document with no `<cancel>` therefore never reads it, so generated code
    /// emits the zero-size [`ElidedSendId`](crate::ElidedSendId) and the no_std
    /// scheduler ring sheds its per-entry `heapless::String<256>`
    /// (~264 B × [`MAX_SCHEDULED_EVENTS`](crate::MAX_SCHEDULED_EVENTS)); a
    /// document that cancels emits [`SceString`](crate::SceString) (load-bearing
    /// on both profiles). The choice is behaviour-preserving under std either
    /// way, so the one emission compiles on both runtime profiles.
    ///
    /// Mirrors the per-machine [`EventQueue`](StatePolicy::EventQueue) sizing
    /// lever; like it (and [`Hal`](StatePolicy::Hal) / [`Payload`](StatePolicy::Payload))
    /// it has no default — stable Rust forbids default associated types, so
    /// every generated policy emits it.
    type ScheduledSendId: crate::ScheduledSendIdLike;

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
    const TABLES: &'static DocumentTables;

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

    /// The buffers one call into Appendix D's procedures borrows, of the sizes
    /// this document needs (docs/adr/0056): `Workspace<STATES, ENABLED, WALK,
    /// DEFAULTS>`.
    type Workspace: Buffers + Default;

    /// The configuration and history records the engine keeps, of the sizes
    /// this document needs: `RunStore<CONFIGURATION, RECORDS, HISTORIES>`.
    type RunStore: RunStorage;

    /// The configuration as [`Engine::get_active_states`] returns it (§scxml-3.4):
    /// `BoundedVec<State, N>` with `N` the most states one configuration holds,
    /// a `Vec` under std and a `heapless::Vec` of exactly that capacity without
    /// it.
    type Configuration: Bounded<Self::State>;

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
    /// during [`Engine::initialize`](crate::Engine::initialize) and the generated code
    /// will lazy-initialize a script session against the `IScriptEngine` instance
    /// the policy received via `Policy::new(script_engine)` (Engine DI Parity RFC).
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
    /// [`unattended_scheduler_steps`](crate::Engine::unattended_scheduler_steps)).
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

    // ──────────────────────────────────────────────
    // Static metadata methods (C++ `constexpr static`)
    //
    // These encode the SCXML document structure at compile time — state
    // hierarchy, document order, event name tables. They take no `&self`
    // because the data is baked into the generated source.
    // ──────────────────────────────────────────────

    /// The state the engine names before [`Engine::initialize`] enters the
    /// initial configuration, and the state enum's `Default` (§scxml-3.2).
    ///
    /// Not what `initialize` enters: that is the document's initial transition
    /// (§scxml-3.2), the row [`DocumentTables::initial`] names, which may name
    /// several states and a `<history>`.
    fn initial_state() -> Self::State;

    /// The state numbered `id`.
    fn state_of(id: StateId) -> Self::State {
        Self::STATES[id.index()]
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
        Self::state_of(Self::TABLES.history(Self::history_id(history)).parent)
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
    /// `Engine::complete_host_invoke` (a plain label: that method does not
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
    /// Generated code produces an `Event::Null` variant. The engine passes this
    /// to [`first_enabled_transition`](StatePolicy::first_enabled_transition)
    /// when it selects eventless transitions.
    fn null_event() -> Self::Event;

    // ──────────────────────────────────────────────
    // Instance methods — generated executable content
    //
    // These mirror C++ policy methods that take `Engine&` as a parameter.
    // Generated code mutates the policy via `&mut self` and calls engine
    // methods through the `engine` parameter. Each answers for ONE state or
    // ONE transition: which states a microstep exits and enters, and in which
    // order, is the engine's Appendix D procedure, not the policy's.
    // ──────────────────────────────────────────────

    /// §scxml-5.10: bind the event whose transitions are about to be selected
    /// as the `_event` their guards read.
    ///
    /// Called once per selection, before the first guard runs, and with the
    /// [`null_event`](StatePolicy::null_event) for an eventless selection —
    /// which has no event of its own, so a policy binds nothing for it. The
    /// default binds nothing: a document whose guards never read `_event` has
    /// nothing to bind.
    fn bind_current_event(&mut self, _event: Self::Event, _engine: &mut Engine<Self>) {}

    /// Appendix D selectTransitions, the half only the document can answer:
    /// the first of `state`'s own transitions, in document order, that `event`
    /// enables and whose guard holds. The [`null_event`](StatePolicy::null_event)
    /// asks for eventless transitions.
    ///
    /// Ports C++ `firstEnabledTransition(State, Event, Engine&)`. The only
    /// place a guard is evaluated. The engine walks the atomic states and
    /// their ancestors and keeps the ordered set. The answer is the
    /// transition's row in [`TABLES`](StatePolicy::TABLES), which holds its
    /// source, targets and type.
    fn first_enabled_transition(
        &mut self,
        state: Self::State,
        event: Self::Event,
        engine: &mut Engine<Self>,
    ) -> Option<TransitionId>;

    /// Execute one transition's executable content (§scxml-3.13 — run between
    /// the microstep's exits and its entries).
    ///
    /// `transition` is one `first_enabled_transition` reported, and one whose
    /// row says it has content. Ports C++ `executeTransitionActions(State, int,
    /// Engine&)`.
    fn execute_transition_content(&mut self, transition: TransitionId, engine: &mut Engine<Self>);

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
    fn execute_entry_actions(
        &mut self,
        state: Self::State,
        engine: &mut Engine<Self>,
        is_default_entry: bool,
    );

    /// Exit `state` (§scxml-3.9): run its `<onexit>` and cancel its
    /// invocations. The engine has recorded its histories already (§scxml-3.10)
    /// and removes it from the configuration afterwards.
    ///
    /// Ports C++ `executeExitActions(State, Engine&, const vector<State>&)`.
    /// One state, and nothing below it: the engine's Appendix D exit set
    /// already holds every active descendant, in exit order, ahead of this
    /// state.
    fn execute_exit_actions(&mut self, state: Self::State, engine: &mut Engine<Self>);

    /// §scxml-3.10.2: a `<history>`'s default transition content, run after
    /// its parent's onentry (and after the parent's own initial content) when
    /// the history was taken with nothing recorded.
    ///
    /// The default runs nothing, which is right for a history whose default
    /// transition has no content and for a document with no history at all.
    fn execute_history_default_content(
        &mut self,
        _history: Self::History,
        _engine: &mut Engine<Self>,
    ) {
    }

    // ──────────────────────────────────────────────
    // Optional instance methods (default no-op; overridden when the
    // corresponding feature flag is `true`)
    // ──────────────────────────────────────────────

    /// Initialize the datamodel via the script engine (§scxml-5.3).
    ///
    /// Generated only when `NEEDS_DATA_MODEL_INIT` is `true`. Called from
    /// [`Engine::initialize`](crate::Engine::initialize) before any state entry.
    fn initialize_data_model(&mut self, _engine: &mut Engine<Self>) {}

    /// Execute any pending `<invoke>` elements deferred during entry (§scxml-6.4).
    ///
    /// Generated only when `HAS_INVOKE_SUPPORT` is `true`.
    fn execute_pending_invokes(&mut self, _engine: &mut Engine<Self>) {}

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
    fn restart_invoke(&mut self, _invoke_id: &str, _engine: &mut Engine<Self>) {}

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

    /// Judge `child`, a running child a saved format-2 state holds, against
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
        _engine: &mut Engine<Self>,
        _wall_now_ms: u64,
    ) -> Result<(), crate::saved_state::StateRefusal> {
        Err(crate::saved_state::StateRefusal::new(format!(
            "invokes names '{}', and the document starts no child session to resume",
            child.id
        )))
    }

    /// Ask `engine`'s factory of children's routers for the router of each
    /// running child a restore resumed and no factory has been asked for yet
    /// ([`Engine::start_owed_child_mesh`]; docs/adr/0043, docs/adr/0045). The
    /// engine calls it when that factory is registered or installed.
    ///
    /// Generated only when the document has an `<invoke type="scxml">` and
    /// saves. A policy with no children to resume has none to ask for, which
    /// is what the default says.
    #[cfg(feature = "std")]
    fn start_owed_child_meshes(&mut self, _engine: &mut Engine<Self>) {}

    /// Hand `invokers`, the tree's host invokers, to every child this session
    /// holds ([`Engine::adopt_host_invokers`]; docs/adr/0045).
    ///
    /// Generated when the document has an `<invoke type="scxml">`. A policy
    /// with no children has none to hand them to, which is what the default
    /// says.
    #[cfg(feature = "std")]
    fn adopt_host_invokers_in_children(&mut self, _invokers: &crate::host_processor::TreeInvokers) {
    }

    /// Route `ending`, a host's ending of a host-run invocation of session
    /// `session_id`, to the child that is that session or holds it in its
    /// tree ([`Engine::route_host_invoke_end`]; docs/adr/0045). `None` when
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
        _engine: &mut Engine<Self>,
    ) {
    }

    /// Copy the typed payload of the event being dispatched into the policy
    /// (EventSchema native lowering).
    ///
    /// The runtime calls this at dispatch — alongside
    /// [`populate_event_metadata`](StatePolicy::populate_event_metadata) — for
    /// every dequeued event, handing the [`EventWithMetadata::payload`](crate::EventWithMetadata)
    /// that rode with the event. Schemaless documents leave the default no-op
    /// (`Self::Payload = ()`, nothing to bind); a document importing
    /// `EventSchema`s overrides this to store the payload in a typed
    /// `pending_payload` field that `_event.data.<field>` guards read natively.
    fn populate_event_payload(&mut self, _payload: &Self::Payload) {}

    /// Lift the typed `_event.data` view out of the data an event carries
    /// (EventSchema native lowering).
    ///
    /// The payload [`populate_event_payload`](StatePolicy::populate_event_payload)
    /// binds is filled by ONE producer: the generated `raise_<event>` inject
    /// seam. Every other producer — `<send>` with `<param>`, namelist or
    /// `<content>`, an invoke forwarding an event either way, autoforward,
    /// BasicHTTP, mesh — fills [`EventMetadata::data`](crate::EventMetadata),
    /// the wire §scxml-5.10 describes and §scxml-B-2-8-1 reads. This reads
    /// the schema's fields out of that when no typed payload rode with the
    /// event, so the same guard answers the same way whichever producer sent
    /// it — including one on the far side of an invoke boundary.
    ///
    /// `Err` says the data cannot be read as this event's schema. The engine
    /// raises `error.execution` and the guard does not fire, which is what W3C
    /// SCXML 3.13 gives for a guard that cannot be evaluated, and what the
    /// script engine gives for the same guard on the same data.
    ///
    /// ⚠ `std` only, because [`EventMetadata::data`](crate::EventMetadata) is:
    /// a `no_std` build has no wire to lift from and no script engine to
    /// disagree with, so there the typed carrier is the whole channel.
    #[cfg(feature = "std")]
    fn lift_event_payload(
        &mut self,
        _event: Self::Event,
        _data: &str,
    ) -> Result<(), crate::event_payload::PayloadRefusal> {
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
        _engine: &mut Engine<Self>,
    ) {
    }

    /// Tick child state machines (§scxml-6.4).
    ///
    /// Generated only when `HAS_CHILD_TICK` is `true`. Called from
    /// [`Engine::tick`](crate::Engine::tick) to propagate scheduler ticks to children.
    fn tick_children(&mut self, _engine: &mut Engine<Self>) {}

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
    /// [`Engine::follow_clock`](crate::Engine::follow_clock).
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
    fn deliver_to_invocation(
        &mut self,
        _invoke_id: &str,
        _event_name: &str,
        _event_data: &str,
        _engine: &mut Engine<Self>,
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

    /// This session's own id (`_sessionid`; docs/adr/0045): the one its host
    /// gave or the runtime drew when it was created, the one its parent gave a
    /// child, or the one a restore bound. Every generated policy has one from
    /// its construction on, a `no_std` one included; the empty text is the
    /// default of a policy written by hand that binds none.
    fn own_session_id(&self) -> &str {
        ""
    }

    /// Bind this session to `session_id`, the id a format-2 saved state holds
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
    /// the delivery, as for [`deliver_to_invocation`](Self::deliver_to_invocation).
    fn deliver_to_child_session(
        &mut self,
        _child_session_id: &str,
        _event_name: &str,
        _event_data: &str,
        _engine: &mut Engine<Self>,
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

/// A policy whose engine can be built in a constant expression
/// (`Engine::new_const`, present on the bare-metal profile), so that a bare-metal
/// program holds it in a `static` and runs no constructor at start-up and
/// copies nothing onto its stack.
///
/// A separate trait and not two more items of [`StatePolicy`], because a
/// constant needs a value written as one: `StatePolicy::initial_state` is a
/// function, and a trait function cannot be called in a constant expression on
/// the toolchain this crate supports. A policy that is built from run-time
/// values (a datamodel that reads its initial values from a script, a session id
/// drawn from the operating system) has no such constants and does not
/// implement this; it is built by [`Engine::new`](crate::Engine::new) or in its
/// slot by [`Engine::new_in_place`](crate::Engine::new_in_place).
pub trait ConstStatePolicy: StatePolicy {
    /// [`StatePolicy::initial_state`], as a constant.
    const INITIAL_STATE: Self::State;

    /// An empty [`StatePolicy::EventQueue`], as a constant: what
    /// `EventQueue::default()` returns.
    const EMPTY_EVENT_QUEUE: Self::EventQueue;
}
