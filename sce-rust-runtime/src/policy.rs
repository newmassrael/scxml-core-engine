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

use std::fmt::Debug;
use std::hash::Hash;

use crate::event::EventWithMetadata;
use crate::hal::Hal;
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

    /// State enum type generated per SCXML document (W3C SCXML 3.3).
    ///
    /// Must be `Copy` (enums are tiny), `Eq`/`Hash` for active-state sets,
    /// `Debug` for logging, `'static` because it has no references.
    type State: Copy + Eq + Hash + Debug + 'static;

    /// Event enum type generated per SCXML document (W3C SCXML 3.12).
    type Event: Copy + Eq + Hash + Debug + 'static;

    /// HAL impl bound to this policy (watching-zenoh RFC §5.J.2 line 1984).
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

    // ──────────────────────────────────────────────
    // Feature flags (C++ `static constexpr bool HAS_...`)
    //
    // The engine branches on these at compile time via `if P::HAS_X { ... }`.
    // Rust's const propagation eliminates the branch entirely when false,
    // matching C++ `if constexpr`.
    // ──────────────────────────────────────────────

    /// Whether the SCXML document contains any `<parallel>` states (W3C SCXML 3.4).
    const HAS_PARALLEL_STATES: bool = false;

    /// Whether ECMAScript expression evaluation is required (guards, assigns, etc.).
    ///
    /// When `true`, the engine will call [`initialize_data_model`](StatePolicy::initialize_data_model)
    /// during [`Engine::initialize`](crate::Engine::initialize) and the generated code
    /// will lazy-initialize a script session via [`ScriptEngineProvider`](crate::ScriptEngineProvider).
    const NEEDS_SCRIPT_ENGINE: bool = false;

    /// Whether the document has any `<datamodel>` variables requiring script-engine initialization.
    ///
    /// When `true`, [`initialize_data_model`](StatePolicy::initialize_data_model) is called
    /// before entering the initial configuration (W3C SCXML 5.3).
    const NEEDS_DATA_MODEL_INIT: bool = false;

    /// Whether the document has any static `<invoke>` children (W3C SCXML 6.4).
    ///
    /// When `true`, [`execute_pending_invokes`](StatePolicy::execute_pending_invokes)
    /// is called after macrostep completion to start deferred child state machines.
    const HAS_INVOKE_SUPPORT: bool = false;

    /// Whether the document's children receive parent events via `<finalize>` (W3C SCXML 6.5).
    ///
    /// When `true`, [`execute_finalize_for_child_event`](StatePolicy::execute_finalize_for_child_event)
    /// runs before each parent event is routed to the parent's transitions.
    const HAS_FINALIZE: bool = false;

    /// Whether the document autoforward child events to any invokes (W3C SCXML 6.4.6).
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

    /// The initial state of the root `<scxml>` element (W3C SCXML 3.3).
    fn initial_state() -> Self::State;

    /// Whether `state` is a `<final>` state (W3C SCXML 3.7).
    fn is_final_state(state: Self::State) -> bool;

    /// The parent of `state` in the document hierarchy, or `None` if it's a root child.
    fn get_parent(state: Self::State) -> Option<Self::State>;

    /// Whether `state` is a compound state (has children, W3C SCXML 3.3).
    fn is_compound_state(state: Self::State) -> bool;

    /// Whether `state` is a `<parallel>` state (W3C SCXML 3.4).
    ///
    /// Only meaningful when `HAS_PARALLEL_STATES` is `true`; the default returns `false`.
    fn is_parallel_state(_state: Self::State) -> bool {
        false
    }

    /// The child regions of a parallel `state` (W3C SCXML 3.4).
    ///
    /// Only meaningful when `HAS_PARALLEL_STATES` is `true`; the default returns an empty slice.
    fn get_parallel_regions(_state: Self::State) -> &'static [Self::State] {
        &[]
    }

    /// Whether `desc` is a (proper or improper) descendant of `anc` in the hierarchy.
    ///
    /// Used by W3C SCXML 3.12 LCA calculation and W3C 3.13 internal transition detection.
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
    /// Only meaningful when `HAS_PARALLEL_STATES` is `true`; required for `In()` predicate
    /// and `_state.active` queries. Default returns `""`.
    fn get_state_name(_state: Self::State) -> &'static str {
        ""
    }

    /// Sentinel event value for eventless transition dispatch (W3C SCXML 3.13).
    ///
    /// Generated code produces an `Event::Null` variant. The engine passes this
    /// to `process_transition()` when checking eventless transitions.
    fn null_event() -> Self::Event;

    /// Get initial children of a compound state (W3C SCXML 3.6).
    /// Returns the resolved initial child state(s) for deep initial targets.
    fn get_initial_children(_state: Self::State) -> Vec<Self::State> {
        Vec::new()
    }

    /// Get initial child considering history (W3C SCXML 3.11).
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

    /// W3C SCXML 3.13: Was the most recently taken transition of type `internal`?
    ///
    /// Set by `process_transition` as a side effect and consumed by the engine's
    /// `handle_hierarchical_transition` to decide LCA behavior.
    fn last_transition_is_internal(&self) -> bool;

    /// Set the "last transition is internal" flag.
    fn set_last_transition_is_internal(&mut self, value: bool);

    /// W3C SCXML 3.13: Was the most recently taken transition targetless (no `target` attribute)?
    fn last_transition_is_targetless(&self) -> bool;

    /// Set the "last transition is targetless" flag.
    fn set_last_transition_is_targetless(&mut self, value: bool);

    /// W3C SCXML 3.4: The actual source state of the last transition.
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

    /// Execute `<onentry>` actions for `state` (W3C SCXML 3.7).
    ///
    /// Ports C++ `executeEntryActions(State, Engine&)`. May:
    /// - raise internal events via `engine.raise(...)`
    /// - schedule delayed sends via `engine.schedule_event(...)`
    /// - mutate datamodel variables on `self`
    /// - defer invokes via `InvokeHelper` (Phase 4)
    fn execute_entry_actions(&mut self, state: Self::State, engine: &mut Engine<Self>);

    /// Execute `<onexit>` actions for `state` (W3C SCXML 3.8).
    ///
    /// Ports C++ `executeExitActions(State, Engine&, const vector<State>&)`.
    /// The `pre_transition_active` slice captures the active configuration
    /// before the transition began, for history state recording (W3C SCXML 3.11).
    fn execute_exit_actions(
        &mut self,
        state: Self::State,
        engine: &mut Engine<Self>,
        pre_transition_active: &[Self::State],
    );

    /// Evaluate guards and take a matching transition (W3C SCXML 3.13).
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
    /// (W3C SCXML 3.13 — executed between exit and entry).
    ///
    /// Ports C++ `executeTransitionActions(Engine&)`.
    fn execute_transition_actions(&mut self, engine: &mut Engine<Self>);

    // ──────────────────────────────────────────────
    // Optional instance methods (default no-op; overridden when the
    // corresponding feature flag is `true`)
    // ──────────────────────────────────────────────

    /// Initialize the datamodel via the script engine (W3C SCXML 5.3).
    ///
    /// Generated only when `NEEDS_DATA_MODEL_INIT` is `true`. Called from
    /// [`Engine::initialize`](crate::Engine::initialize) before any state entry.
    fn initialize_data_model(&mut self, _engine: &mut Engine<Self>) {}

    /// Execute any pending `<invoke>` elements deferred during entry (W3C SCXML 6.4).
    ///
    /// Generated only when `HAS_INVOKE_SUPPORT` is `true`.
    fn execute_pending_invokes(&mut self, _engine: &mut Engine<Self>) {}

    /// Execute `<finalize>` handlers for child events (W3C SCXML 6.5).
    ///
    /// Generated only when `HAS_FINALIZE` is `true`. Called from the engine's
    /// external queue processing, before the event is routed to transitions.
    fn execute_finalize_for_child_event(
        &mut self,
        _event: &EventWithMetadata<Self::Event>,
        _engine: &mut Engine<Self>,
    ) {
    }

    /// Get active states for parallel state machines (W3C SCXML 3.4).
    ///
    /// Generated only when `HAS_ACTIVE_STATES` is `true`.
    fn get_active_states(&self) -> Vec<Self::State> {
        Vec::new()
    }

    /// Forward external events to autoforward children (W3C SCXML 6.4.6).
    ///
    /// Generated only when `HAS_AUTOFORWARD` is `true`.
    fn forward_to_autoforward_children(&mut self, _event_name: &str, _engine: &mut Engine<Self>) {}

    /// Tick child state machines (W3C SCXML 6.4).
    ///
    /// Generated only when `HAS_CHILD_TICK` is `true`. Called from
    /// [`Engine::tick`](crate::Engine::tick) to propagate scheduler ticks to children.
    fn tick_children(&mut self, _engine: &mut Engine<Self>) {}

    /// Set the `nextEventIsExternal_` flag (W3C SCXML 5.10.1).
    ///
    /// Generated only when `HAS_EXTERNAL_EVENT_FLAG` is `true`. Used by the engine's
    /// `raise_external` to mark the next processed event as external for `_event.type`.
    fn set_next_event_is_external(&mut self, _value: bool) {}

    /// W3C SCXML 5.10: Populate pending event metadata fields from an event's metadata.
    ///
    /// Ports C++ `EventMetadataHelper::populatePolicyFromMetadata`. Called by the engine
    /// before dispatching each event from the internal/external queues. Generated code
    /// stores the metadata in `pending_event_*` struct fields so that `process_transition`
    /// can pass them to `set_current_event_in_script_engine`.
    fn populate_event_metadata(&mut self, _metadata: &crate::event::EventMetadata) {}

    /// W3C SCXML 5.10: Clear pending event metadata after transition processing.
    ///
    /// Ports C++ `EventMetadataHelper::clearPolicyMetadata`. Called by the engine
    /// after each event dispatch cycle to reset metadata for the next event.
    fn clear_event_metadata(&mut self) {}
}
