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
//! ## Static vs Instance Methods
//!
//! - **Static methods** (`initial_state()`, `is_final_state()`, etc.) mirror C++
//!   `constexpr static` methods. They encode compile-time SCXML document structure
//!   (state hierarchy, parent map, document order).
//! - **Instance methods** (`execute_entry_actions()`, `process_transition()`, etc.)
//!   mirror C++ non-static policy methods. They hold mutable datamodel state and
//!   read/write through `&mut self`.
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
use crate::Engine;

/// The contract that generated state machine policies must satisfy.
///
/// Ports the C++ `StatePolicy` concept from `sce/include/core/StatePolicyConcepts.h`.
/// Generated code produces one struct implementing `StatePolicy` per SCXML source file.
///
/// ## Required Members (via accessors)
///
/// The C++ version uses `static_assert(requires(p) { p.lastTransitionIsInternal_ })` to
/// enforce member presence. Rust uses accessor methods: [`last_transition_is_internal`](StatePolicy::last_transition_is_internal),
/// [`last_transition_is_targetless`](StatePolicy::last_transition_is_targetless),
/// [`last_transition_source_state`](StatePolicy::last_transition_source_state). Generated
/// code emits these as trivial getters over struct fields.
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

    /// Whether the policy has a `nextEventIsExternal_` flag for `_event.type` classification.
    const HAS_EXTERNAL_EVENT_FLAG: bool = false;

    /// Whether the policy supports child-tick for nested invokes.
    const HAS_CHILD_TICK: bool = false;

    // ──────────────────────────────────────────────
    // Static metadata methods (C++ `constexpr static`)
    //
    // These encode the SCXML document structure at compile time — state
    // hierarchy, document order, event name tables. They take no `&self`
    // because the data is baked into the generated source.
    // ──────────────────────────────────────────────

    /// The initial state of the root `<scxml>` element (§scxml-3.2).
    fn initial_state() -> Self::State;

    /// Whether `state` is a `<final>` state (§scxml-3.7).
    fn is_final_state(state: Self::State) -> bool;

    /// The parent of `state` in the document hierarchy, or `None` if it's a root child.
    fn get_parent(state: Self::State) -> Option<Self::State>;

    /// Whether `state` is a compound state (has children, §scxml-3.3).
    fn is_compound_state(state: Self::State) -> bool;

    /// Whether `state` is a `<parallel>` state (§scxml-3.4).
    ///
    /// Only meaningful when `HAS_PARALLEL_STATES` is `true`; the default returns `false`.
    fn is_parallel_state(_state: Self::State) -> bool {
        false
    }

    /// The child regions of a parallel `state` (§scxml-3.4).
    ///
    /// Only meaningful when `HAS_PARALLEL_STATES` is `true`; the default returns an empty slice.
    fn get_parallel_regions(_state: Self::State) -> &'static [Self::State] {
        &[]
    }

    /// Whether `desc` is a (proper or improper) descendant of `anc` in the hierarchy.
    ///
    /// Used by §scxml-3.12 LCA calculation and W3C 3.13 internal transition detection.
    fn is_descendant_of(desc: Self::State, anc: Self::State) -> bool;

    /// Document order index of `state` (W3C SCXML Appendix D).
    ///
    /// Used for deterministic exit ordering and optimal transition set selection.
    fn get_document_order(state: Self::State) -> u32;

    /// Human-readable name of `event` (e.g., `"error.execution"`, `"done.state.s1"`).
    ///
    /// Used for `_event.name` population, logging, and HTTP send payloads.
    fn get_event_name(event: Self::Event) -> &'static str;

    /// Reverse lookup: `Some(event)` if `name` matches a known event, else `None`.
    ///
    /// Used by `raiseExternal(const std::string&)` overload and child invoke autoforward.
    fn get_event_from_name(name: &str) -> Option<Self::Event>;

    /// Human-readable name of `state` (e.g., `"s0"`, `"passingState"`).
    ///
    /// Required (no default): the State→id mapping is structural and external
    /// consumers (trace recorders, post-mortem analyzers, the generated
    /// `In()` predicate callback) need it regardless of whether the SM uses
    /// parallel states. Mirrors `get_event_name` (also required) — see the
    /// C++ `StateNamingPolicy` concept in `sce/include/core/StatePolicyConcepts.h`.
    fn get_state_name(state: Self::State) -> &'static str;

    /// Sentinel event value for eventless transition dispatch (§scxml-3.13).
    ///
    /// Generated code produces an `Event::Null` variant. The engine passes this
    /// to `process_transition()` when checking eventless transitions.
    fn null_event() -> Self::Event;

    /// Get initial children of a compound state (§scxml-3.6).
    /// Returns the resolved initial child state(s) for deep initial targets.
    ///
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: returns the bounded
    /// [`StateChain<Self::State>`](crate::helpers::hierarchy::StateChain) — aliased
    /// to `Vec<Self::State>` under std (ABI-preserving — existing generated
    /// overrides keep emitting `Vec<...>` which is the same type via the alias)
    /// and to `heapless::Vec<Self::State, MAX_HIERARCHY_DEPTH=16>` under no_std.
    /// The default no-op returns an empty chain via
    /// [`new_chain`](crate::helpers::hierarchy::new_chain). Reuses the
    /// existing `MAX_HIERARCHY_DEPTH` invariant — no new capacity constant
    /// (D-1 lockin preserved beyond `MAX_SCHEDULED_EVENTS` / `MAX_EVENT_QUEUE_DEPTH`).
    fn get_initial_children(_state: Self::State) -> StateChain<Self::State> {
        hierarchy::new_chain()
    }

    /// Get initial child considering history (§scxml-3.11).
    /// Non-static: checks history before returning initial child.
    fn get_initial_or_history_child(&self, state: Self::State) -> Self::State {
        state
    }

    // ──────────────────────────────────────────────
    // Required mutable field accessors
    //
    // C++ uses `static_assert(requires(p) { p.lastTransitionIsInternal_ })` to
    // enforce member presence directly on struct fields. Rust exposes these as
    // read/write methods; generated code emits trivial inline getters.
    // ──────────────────────────────────────────────

    /// §scxml-3.13: Was the most recently taken transition of type `internal`?
    ///
    /// Set by `process_transition` as a side effect and consumed by the engine's
    /// `handle_hierarchical_transition` to decide LCA behavior.
    fn last_transition_is_internal(&self) -> bool;

    /// Set the "last transition is internal" flag.
    fn set_last_transition_is_internal(&mut self, value: bool);

    /// §scxml-3.13: Was the most recently taken transition targetless (no `target` attribute)?
    fn last_transition_is_targetless(&self) -> bool;

    /// Set the "last transition is targetless" flag.
    fn set_last_transition_is_targetless(&mut self, value: bool);

    /// §scxml-3.4: The actual source state of the last transition.
    ///
    /// For parallel states, differs from the engine's `current_state` when the
    /// transition originated from an inactive ancestor.
    fn last_transition_source_state(&self) -> Self::State;

    /// Set the last transition source state.
    fn set_last_transition_source_state(&mut self, state: Self::State);

    // ──────────────────────────────────────────────
    // Instance methods — generated executable content
    //
    // These mirror C++ policy methods that take `Engine&` as a parameter.
    // Generated code mutates the policy via `&mut self` and calls engine
    // methods through the `engine` parameter.
    // ──────────────────────────────────────────────

    /// Execute `<onentry>` actions for `state` (§scxml-3.8).
    ///
    /// Ports C++ `executeEntryActions(State, Engine&)`. May:
    /// - raise internal events via `engine.raise(...)`
    /// - schedule delayed sends via `engine.schedule_event(...)`
    /// - mutate datamodel variables on `self`
    /// - defer `<invoke>` starts until the configuration is stable (§scxml-6.4)
    fn execute_entry_actions(&mut self, state: Self::State, engine: &mut Engine<Self>);

    /// Execute `<onexit>` actions for `state` (§scxml-3.9).
    ///
    /// Ports C++ `executeExitActions(State, Engine&, const vector<State>&)`.
    /// The `pre_transition_active` slice captures the active configuration
    /// before the transition began, for history state recording (§scxml-3.11).
    fn execute_exit_actions(
        &mut self,
        state: Self::State,
        engine: &mut Engine<Self>,
        pre_transition_active: &[Self::State],
    );

    /// Evaluate guards and take a matching transition (§scxml-3.13).
    ///
    /// Ports C++ `processTransition(State&, Event, Engine&) -> bool`.
    ///
    /// The `current_state` parameter is an in/out: the engine passes its current
    /// state; generated code updates it to the transition's target if a transition
    /// is taken. Returns `true` if a transition was taken, `false` otherwise.
    ///
    /// For the eventless-transition code path, the engine passes a sentinel event
    /// value (typically `Event::default()` if `Default` is implemented, or a
    /// generator-reserved "null" variant).
    fn process_transition(
        &mut self,
        current_state: &mut Self::State,
        event: Self::Event,
        engine: &mut Engine<Self>,
    ) -> bool;

    /// Execute transition action blocks for the currently-matched transition
    /// (§scxml-3.13 — executed between exit and entry).
    ///
    /// Ports C++ `executeTransitionActions(Engine&)`.
    fn execute_transition_actions(&mut self, engine: &mut Engine<Self>);

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

    /// Get active states for parallel state machines (§scxml-3.4).
    ///
    /// Generated only when `HAS_ACTIVE_STATES` is `true`.
    ///
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: return type matches the cfg-conditional
    /// [`StateChain`] alias — see [`get_initial_children`](Self::get_initial_children)
    /// above for the std/no_std mapping rationale. The default no-op returns
    /// an empty chain.
    fn get_active_states(&self) -> StateChain<Self::State> {
        hierarchy::new_chain()
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

    /// Set the `nextEventIsExternal_` flag (§scxml-5.10.1).
    ///
    /// Generated only when `HAS_EXTERNAL_EVENT_FLAG` is `true`. Used by the engine's
    /// `raise_external` to mark the next processed event as external for `_event.type`.
    fn set_next_event_is_external(&mut self, _value: bool) {}

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
