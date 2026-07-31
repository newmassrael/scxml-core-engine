// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

// StatePolicy is the contract that generated state machine code implements.
//
// Go equivalent of Rust's StatePolicy trait from backends/rust/runtime/src/policy.rs.
// Uses type parameters for State and Event types. Generated code produces a
// concrete struct implementing StatePolicy[S, E] per SCXML source file.
//
// The Rust version uses associated types and const flags. In Go, we use an
// interface with methods. Feature flags (HasParallelStates, NeedsScriptEngine,
// etc.) are methods returning bool -- the Go compiler cannot const-fold these
// like Rust, but the generated implementations return constant values.
//
// # Static vs Instance Methods
//
// Rust distinguishes static methods (no &self) from instance methods (&mut self).
// Go interfaces only have receiver methods. All methods here take the policy
// receiver. For "static" metadata methods like InitialState() or IsFinalState(),
// the generated implementation ignores the receiver and returns constant data.
type StatePolicy[S comparable, E comparable] interface {
	// ================================================================
	// Static metadata methods (C++ constexpr static equivalents)
	//
	// These encode the SCXML document structure. Generated code returns
	// constant data from these methods.
	// ================================================================

	// InitialState returns the initial state of the root <scxml> element (W3C SCXML 3.3).
	InitialState() S

	// IsFinalState returns whether state is a <final> state (W3C SCXML 3.7).
	IsFinalState(state S) bool

	// GetParent returns the parent of state in the document hierarchy. The bool
	// return indicates whether a parent exists (false for root children).
	GetParent(state S) (S, bool)

	// IsCompoundState returns whether state is a compound state (has children, W3C SCXML 3.3).
	IsCompoundState(state S) bool

	// IsParallelState returns whether state is a <parallel> state (W3C SCXML 3.4).
	// Only meaningful when HasParallelStates() returns true.
	IsParallelState(state S) bool

	// GetParallelRegions returns the child regions of a parallel state (W3C SCXML 3.4).
	// Only meaningful when HasParallelStates() returns true.
	GetParallelRegions(state S) []S

	// IsDescendantOf returns whether desc is a (proper or improper) descendant
	// of anc in the hierarchy. Used by W3C SCXML 3.12 LCA calculation.
	IsDescendantOf(desc, anc S) bool

	// GetDocumentOrder returns the document order index of state (W3C SCXML Appendix D).
	// Used for deterministic exit ordering and optimal transition set selection.
	GetDocumentOrder(state S) int

	// GetEventName returns the human-readable name of event (e.g., "error.execution").
	// Used for _event.name population, logging, and HTTP send payloads.
	GetEventName(event E) string

	// GetEventFromName performs reverse lookup: returns (event, true) if name
	// matches a known event, or (zero, false) otherwise. Used by RaiseExternalByName
	// and child invoke autoforward.
	GetEventFromName(name string) (E, bool)

	// GetStateName returns the human-readable name of state (e.g., "s0", "passingState").
	// Required for In() predicate and _state.active queries.
	GetStateName(state S) string

	// NullEvent returns the sentinel event value for eventless transition dispatch
	// (W3C SCXML 3.13). Generated code produces a Null variant.
	NullEvent() E

	// GetInitialChildren returns the initial children of a compound state (W3C SCXML 3.6).
	// Returns the resolved initial child state(s) for deep initial targets.
	GetInitialChildren(state S) []S

	// GetInitialOrHistoryChild returns the initial child considering history
	// (W3C SCXML 3.11). Non-static: checks history before returning initial child.
	GetInitialOrHistoryChild(state S) S

	// ================================================================
	// Required mutable field accessors
	//
	// These replace the Rust accessors for last_transition_is_internal_,
	// last_transition_is_targetless_, and last_transition_source_state_.
	// Generated code emits these as trivial getters/setters over struct fields.
	// ================================================================

	// LastTransitionIsInternal returns whether the most recently taken transition
	// was of type internal (W3C SCXML 3.13).
	LastTransitionIsInternal() bool

	// SetLastTransitionIsInternal sets the "last transition is internal" flag.
	SetLastTransitionIsInternal(value bool)

	// LastTransitionIsTargetless returns whether the most recently taken transition
	// was targetless (no target attribute).
	LastTransitionIsTargetless() bool

	// SetLastTransitionIsTargetless sets the "last transition is targetless" flag.
	SetLastTransitionIsTargetless(value bool)

	// LastTransitionSourceState returns the actual source state of the last transition.
	LastTransitionSourceState() S

	// SetLastTransitionSourceState sets the last transition source state.
	SetLastTransitionSourceState(state S)

	// ================================================================
	// Instance methods -- generated executable content
	//
	// These mirror Rust policy methods that take &mut Engine<Self> as a parameter.
	// Generated code mutates the policy via the receiver and calls engine methods
	// through the engine parameter.
	// ================================================================

	// ExecuteEntryActions executes <onentry> actions for state (W3C SCXML 3.7).
	// May raise internal events via engine.Raise(), schedule delayed sends, etc.
	ExecuteEntryActions(state S, engine *Engine[S, E])

	// ExecuteExitActions executes <onexit> actions for state (W3C SCXML 3.8).
	// The preTransitionActive slice captures the active configuration before the
	// transition began, for history state recording (W3C SCXML 3.11).
	ExecuteExitActions(state S, engine *Engine[S, E], preTransitionActive []S)

	// ProcessTransition evaluates guards and takes a matching transition (W3C SCXML 3.13).
	// The currentState parameter is an in/out pointer: the engine passes its current
	// state; generated code updates it to the transition's target if a transition is taken.
	// Returns true if a transition was taken.
	ProcessTransition(currentState *S, event E, engine *Engine[S, E]) bool

	// ExecuteTransitionActions executes transition action blocks for the
	// currently-matched transition (W3C SCXML 3.13 -- between exit and entry).
	ExecuteTransitionActions(engine *Engine[S, E])

	// ================================================================
	// Feature flags (Rust associated const bool equivalents)
	//
	// In Rust these are const flags enabling compile-time branch elimination.
	// In Go they are regular methods. Generated implementations return constant
	// values, enabling the compiler to potentially inline and optimize.
	// ================================================================

	// HasParallelStates returns whether the SCXML document contains any <parallel> states.
	HasParallelStates() bool

	// NeedsScriptEngine returns whether ECMAScript expression evaluation is required.
	NeedsScriptEngine() bool

	// NeedsDataModelInit returns whether the document has <datamodel> variables
	// requiring script-engine initialization.
	NeedsDataModelInit() bool

	// HasInvokeSupport returns whether the document has any static <invoke> children
	// (W3C SCXML 6.4).
	HasInvokeSupport() bool

	// HasFinalize returns whether the document's children receive parent events
	// via <finalize> (W3C SCXML 6.5).
	HasFinalize() bool

	// HasAutoforward returns whether the document autoforwards child events to
	// any invokes (W3C SCXML 6.4.1).
	HasAutoforward() bool

	// HasActiveStates returns whether the policy exposes activeStates_ tracking.
	HasActiveStates() bool

	// HasExternalEventFlag returns whether the policy has a nextEventIsExternal_ flag.
	HasExternalEventFlag() bool

	// HasChildTick returns whether the policy supports child-tick for nested invokes.
	HasChildTick() bool

	// ================================================================
	// Optional instance methods (default no-op behavior)
	//
	// Generated code overrides these when the corresponding feature flag is true.
	// For Go, the generated struct embeds a DefaultPolicyBehavior to get default
	// implementations, then overrides only what it needs.
	// ================================================================

	// InitializeDataModel initializes the datamodel via the script engine (W3C SCXML 5.3).
	InitializeDataModel(engine *Engine[S, E])

	// ExecutePendingInvokes executes any pending <invoke> elements deferred during
	// entry (W3C SCXML 6.4).
	ExecutePendingInvokes(engine *Engine[S, E])

	// ExecuteFinalizeForChildEvent executes <finalize> handlers for child events
	// (W3C SCXML 6.5).
	ExecuteFinalizeForChildEvent(event *EventWithMetadata[E], engine *Engine[S, E])

	// GetActiveStates returns the active states for parallel state machines (W3C SCXML 3.4).
	GetActiveStates() []S

	// ForwardToAutoforwardChildren forwards external events to autoforward children
	// (W3C SCXML 6.4.1).
	ForwardToAutoforwardChildren(eventName string, metadata EventMetadata, engine *Engine[S, E])

	// TickChildren ticks child state machines (W3C SCXML 6.4).
	TickChildren(engine *Engine[S, E])

	// SetNextEventIsExternal sets the nextEventIsExternal_ flag (W3C SCXML 5.10.1).
	SetNextEventIsExternal(value bool)

	// PopulateEventMetadata populates pending event metadata fields from an event's
	// metadata (W3C SCXML 5.10).
	PopulateEventMetadata(meta *EventMetadata)

	// ClearEventMetadata clears pending event metadata after transition processing.
	ClearEventMetadata()
}
