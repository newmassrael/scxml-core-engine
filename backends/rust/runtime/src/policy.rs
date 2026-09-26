// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
//! once for every machine. It reads a policy as a [`Document`] (see
//! [`PolicyDocument`]), and the engine hands it the rest.
//!
//! ## Static vs Instance Methods
//!
//! - **Static methods** (`is_final_state()`, `get_child_states()`, etc.) mirror
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
use crate::helpers::hierarchy::{self, StateChain};
use crate::helpers::microstep::{Document, EnabledTransition, EntryTarget};
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
    /// [`NoHistory`](crate::helpers::microstep::NoHistory) for a document that
    /// declares none.
    ///
    /// A history is not a state — it is never in a configuration — so it is not
    /// a variant of [`State`](StatePolicy::State): target lists name one as an
    /// [`EntryTarget::History`], and the entry procedures dereference it to what
    /// it recorded or to its default.
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
    /// Carries the machine's FIFO **depth** under `--features=no_std`: generated
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

    /// Whether the policy exposes `activeStates_` tracking (required for parallel states).
    const HAS_ACTIVE_STATES: bool = false;

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
    /// Not what `initialize` enters: that is the document's initial transition,
    /// [`get_document_initial_targets`](StatePolicy::get_document_initial_targets),
    /// which may name several states and a `<history>`.
    fn initial_state() -> Self::State;

    /// Whether `state` is a `<final>` state (§scxml-3.7).
    fn is_final_state(state: Self::State) -> bool;

    /// The parent of `state` in the document hierarchy, or `None` if it's a root child.
    fn get_parent(state: Self::State) -> Option<Self::State>;

    /// Whether `state` is a compound state: a `<state>` with child states
    /// (§scxml-3.3). A `<parallel>` answers `false` — this is Appendix D's
    /// `isCompoundState`, and it decides which ancestors can be a transition's
    /// domain.
    fn is_compound_state(state: Self::State) -> bool;

    /// Whether `state` is a `<parallel>` state (§scxml-3.4).
    ///
    /// Only meaningful when `HAS_PARALLEL_STATES` is `true`; the default returns `false`.
    fn is_parallel_state(_state: Self::State) -> bool {
        false
    }

    /// §scxml-D-getChildStates: `state`'s `<state>`, `<parallel>` and `<final>`
    /// children, in document order — for a `<parallel>`, its regions.
    fn get_child_states(state: Self::State) -> &'static [Self::State];

    /// §scxml-3.3: a compound state's initial transition target, as written —
    /// one entry per token of `initial` or of the `<initial>` element's
    /// transition, or the first child state when the document names none.
    /// Empty for every state that is not compound.
    fn get_initial_targets(
        state: Self::State,
    ) -> &'static [EntryTarget<Self::State, Self::History>];

    /// §scxml-3.2: the target of the document's own initial transition, as
    /// written — what [`Engine::initialize`] enters, from the `<scxml>`
    /// element.
    fn get_document_initial_targets() -> &'static [EntryTarget<Self::State, Self::History>];

    /// §scxml-3.10: the state a `<history>` is declared in.
    fn get_history_parent(history: Self::History) -> Self::State;

    /// §scxml-3.10.2: a `<history>`'s default transition target, as written —
    /// its default stored state configuration.
    fn get_history_default_targets(
        history: Self::History,
    ) -> &'static [EntryTarget<Self::State, Self::History>];

    /// Document order index of `state` (W3C SCXML Appendix D).
    ///
    /// Document order is also entry order, and its reverse is exit order.
    fn get_document_order(state: Self::State) -> u32;

    /// Human-readable name of `event` (e.g., `"error.execution"`, `"done.state.s1"`).
    ///
    /// Used for `_event.name` population, logging, and HTTP send payloads.
    fn get_event_name(event: Self::Event) -> &'static str;

    /// Reverse lookup: `Some(event)` if `name` matches a known event, else `None`.
    ///
    /// Used by `raiseExternal(const std::string&)` overload and child invoke autoforward.
    fn get_event_from_name(name: &str) -> Option<Self::Event>;

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
    /// C++ `StateNamingPolicy` concept in `sce/include/core/StatePolicyConcepts.h`.
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
    /// [`Engine::enter_at`](crate::Engine::enter_at) takes a
    /// [`StateChain`] of `Self::State`. (Bare label: `StateChain` is imported
    /// above, so an explicit target is redundant and `rustdoc-links` rejects
    /// it.)
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
    // Run-time state the entry procedures read
    // ──────────────────────────────────────────────

    /// §scxml-3.10: what `history` recorded when its parent was last exited;
    /// `None` before that ever happened.
    fn history_value(&self, history: Self::History) -> Option<&[Self::State]>;

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
    /// their ancestors and keeps the ordered set.
    fn first_enabled_transition(
        &mut self,
        state: Self::State,
        event: Self::Event,
        engine: &mut Engine<Self>,
    ) -> Option<EnabledTransition<Self::State, Self::History>>;

    /// Execute one transition's executable content (§scxml-3.13 — run between
    /// the microstep's exits and its entries).
    ///
    /// `transition_index` is the one `first_enabled_transition` reported for
    /// `source`. Ports C++ `executeTransitionActions(State, int, Engine&)`.
    fn execute_transition_content(
        &mut self,
        source: Self::State,
        transition_index: usize,
        engine: &mut Engine<Self>,
    );

    /// Enter `state` (§scxml-3.8): add it to the configuration, run its
    /// `<onentry>`, and its initial transition's content when
    /// `is_default_entry`.
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

    /// Exit `state` (§scxml-3.9): record its histories, run its `<onexit>`,
    /// cancel its invocations, remove it from the configuration.
    ///
    /// Ports C++ `executeExitActions(State, Engine&, const vector<State>&)`.
    /// One state, and nothing below it: the engine's Appendix D exit set
    /// already holds every active descendant, in exit order, ahead of this
    /// state. `configuration_before_exit` is the configuration as it stood
    /// before the microstep's first exit, which every history of the microstep
    /// is recorded from (§scxml-3.10).
    fn execute_exit_actions(
        &mut self,
        state: Self::State,
        engine: &mut Engine<Self>,
        configuration_before_exit: &[Self::State],
    );

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
    #[cfg(not(feature = "no_std"))]
    fn lift_event_payload(
        &mut self,
        _event: Self::Event,
        _data: &str,
    ) -> Result<(), crate::event_payload::PayloadRefusal> {
        Ok(())
    }

    /// Get active states for parallel state machines (§scxml-3.4).
    ///
    /// Generated only when `HAS_ACTIVE_STATES` is `true`.
    ///
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: returns the cfg-conditional
    /// [`StateChain`] alias — `Vec` under std, a `heapless::Vec` bounded by
    /// [`MAX_HIERARCHY_DEPTH`](crate::helpers::hierarchy::MAX_HIERARCHY_DEPTH)
    /// under no_std. The default no-op returns an empty chain.
    fn get_active_states(&self) -> StateChain<Self::State> {
        hierarchy::new_chain()
    }

    /// Put the active state set back (§scxml-3.4), for
    /// [`Engine::enter_at`](crate::Engine::enter_at).
    ///
    /// Generated wherever [`get_active_states`](Self::get_active_states) is, and
    /// for the same reason: a machine that keeps its own active set is the only
    /// machine that can be handed one. `enter_at` calls this exactly when
    /// [`has_active_states`](crate::helpers::state_policy_concepts::has_active_states)
    /// holds, which is the same condition the generator emits both under.
    ///
    /// The default is a tripwire rather than a no-op, unlike its reading
    /// sibling. The sibling's empty chain is a truthful answer for a machine
    /// with no parallel regions — there is no active set beyond the hierarchy
    /// walk. Silently discarding a *write* has no such reading: the caller
    /// would be told the configuration was restored while the policy still held
    /// the one it was constructed with, which is the "looks fine, is elsewhere"
    /// outcome `enter_at` exists to refuse. Reaching it means the policy
    /// declares `HAS_ACTIVE_STATES` without generating this override, so it is a
    /// generator defect and says so — the same discipline as
    /// [`push_chain`](crate::helpers::hierarchy::push_chain)'s capacity
    /// tripwire.
    fn set_active_states(&mut self, _states: StateChain<Self::State>) {
        panic!(
            "policy declares HAS_ACTIVE_STATES but did not generate set_active_states \
             (generator bug): Engine::enter_at cannot restore this machine's active set"
        );
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

/// A policy, as the document Appendix D's procedures read: its static tables
/// are the structure, and its recorded histories are the one piece of run-time
/// state the entry procedures need.
///
/// A borrow of the policy alone, never of the engine holding it, so a
/// generated hook — which runs with the engine borrowed mutably — can hand
/// [`helpers::microstep`](crate::helpers::microstep) its own `self`: the
/// completion check a generated `<final>` makes for the `<parallel>` it may
/// complete (`is_in_final_state`) asks the transcription through this. A
/// wrapper rather than an implementation on every policy, because the two
/// traits name their associated types alike and a generated policy spelling
/// `Self::State` must keep meaning exactly one of them.
pub struct PolicyDocument<'p, P: StatePolicy>(pub &'p P);

impl<P: StatePolicy> Document for PolicyDocument<'_, P> {
    type State = P::State;
    type History = P::History;

    fn parent_of(&self, state: P::State) -> Option<P::State> {
        P::get_parent(state)
    }

    fn is_compound(&self, state: P::State) -> bool {
        P::is_compound_state(state)
    }

    fn is_parallel(&self, state: P::State) -> bool {
        P::is_parallel_state(state)
    }

    fn is_final(&self, state: P::State) -> bool {
        P::is_final_state(state)
    }

    fn child_states(&self, state: P::State) -> &'static [P::State] {
        P::get_child_states(state)
    }

    fn initial_targets(&self, state: P::State) -> &'static [EntryTarget<P::State, P::History>] {
        P::get_initial_targets(state)
    }

    fn history_parent(&self, history: P::History) -> P::State {
        P::get_history_parent(history)
    }

    fn history_value(&self, history: P::History) -> Option<&[P::State]> {
        self.0.history_value(history)
    }

    fn history_default_targets(
        &self,
        history: P::History,
    ) -> &'static [EntryTarget<P::State, P::History>] {
        P::get_history_default_targets(history)
    }

    fn document_order(&self, state: P::State) -> u32 {
        P::get_document_order(state)
    }
}
