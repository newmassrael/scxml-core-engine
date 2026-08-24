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

	// InitialState returns the initial state of the root <scxml> element (§scxml-3.3).
	InitialState() S

	// IsFinalState returns whether state is a <final> state (§scxml-3.7).
	IsFinalState(state S) bool

	// GetParent returns the parent of state in the document hierarchy. The bool
	// return indicates whether a parent exists (false for root children).
	GetParent(state S) (S, bool)

	// IsCompoundState returns whether state is a compound state (has children, §scxml-3.3).
	IsCompoundState(state S) bool

	// IsParallelState returns whether state is a <parallel> state (§scxml-3.4).
	// Only meaningful when HasParallelStates() returns true.
	IsParallelState(state S) bool

	// GetParallelRegions returns the child regions of a parallel state (§scxml-3.4).
	// Only meaningful when HasParallelStates() returns true.
	GetParallelRegions(state S) []S

	// IsDescendantOf returns whether desc is a (proper or improper) descendant
	// of anc in the hierarchy. Used by §scxml-3.12 LCA calculation.
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

	// GetStateFromName is the reverse lookup: (state, true) if name matches a
	// state this document declares, or (zero, false) otherwise.
	//
	// Required (no default) for the reason GetStateName is: the mapping is
	// structural, and a default could only ever answer false. A policy that had
	// not emitted the table would then report every recorded configuration as
	// unknown — which a caller reads as "this run has no history" rather than
	// as a policy that is incomplete. Mirrors GetEventFromName, which is
	// required for the same reason on the event side.
	//
	// This is what lets a configuration cross a process. A host can only record
	// state NAMES — a journal, a wire, a file — while Engine.EnterAt takes a
	// []S. Without the reverse, a recorded configuration cannot be turned back
	// into the argument that door asks for and resuming degrades to replaying
	// from the initial state. A consumer-side table would age silently the
	// moment the document gained a state; only the generator writes one that
	// ages with the document.
	//
	// The round trip is an identity: GetStateFromName(GetStateName(s)) is
	// (s, true) for every state of the document, and a name the document does
	// not carry is (zero, false) rather than a guess.
	GetStateFromName(name string) (S, bool)

	// NullEvent returns the sentinel event value for eventless transition dispatch
	// (§scxml-3.13). Generated code produces a Null variant.
	NullEvent() E

	// GetInitialChildren returns the initial children of a compound state (§scxml-3.6).
	// Returns the resolved initial child state(s) for deep initial targets.
	GetInitialChildren(state S) []S

	// GetInitialOrHistoryChild returns the initial child considering history
	// (§scxml-3.11). Non-static: checks history before returning initial child.
	GetInitialOrHistoryChild(state S) S

	// ================================================================
	// Required mutable field accessors
	//
	// These replace the Rust accessors for last_transition_is_internal_,
	// last_transition_is_targetless_, and last_transition_source_state_.
	// Generated code emits these as trivial getters/setters over struct fields.
	// ================================================================

	// LastTransitionIsInternal returns whether the most recently taken transition
	// was of type internal (§scxml-3.13).
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

	// ExecuteEntryActions executes <onentry> actions for state (§scxml-3.7)
	// and gives state the descendants Appendix D says it is owed.
	//
	// pathChild is what tells Appendix D's two entry functions apart, and it is
	// the whole of the difference between them:
	//
	//   nil          state is the entry TARGET, so addDescendantStatesToEnter
	//                applies: a compound state takes its default initial child
	//                and a <parallel> takes every region.
	//   non-nil      state is merely an ANCESTOR on the way to a deeper target,
	//                and *pathChild is the one of its children the entry set
	//                already holds. addAncestorStatesToEnter adds it WITHOUT
	//                its default; the single exception is a <parallel>, whose
	//                OTHER regions still take theirs because nothing is
	//                entering inside them.
	//
	// Answering both with the nil behaviour is what leaves two children of one
	// compound state active at once — measured 2026-08-15 across five backends,
	// and pinned by integration_resources/ancestor_entry_is_not_default_entry/.
	// May raise internal events via engine.Raise(), schedule delayed sends, etc.
	ExecuteEntryActions(state S, engine *Engine[S, E], pathChild *S)

	// ExecuteExitActions executes <onexit> actions for state (§scxml-3.8).
	// The preTransitionActive slice captures the active configuration before the
	// transition began, for history state recording (§scxml-3.11).
	ExecuteExitActions(state S, engine *Engine[S, E], preTransitionActive []S)

	// ProcessTransition evaluates guards and takes a matching transition (§scxml-3.13).
	// The currentState parameter is an in/out pointer: the engine passes its current
	// state; generated code updates it to the transition's target if a transition is taken.
	// Returns true if a transition was taken.
	ProcessTransition(currentState *S, event E, engine *Engine[S, E]) bool

	// ExecuteTransitionActions executes transition action blocks for the
	// currently-matched transition (§scxml-3.13 -- between exit and entry).
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
	// (§scxml-6.4).
	HasInvokeSupport() bool

	// HasFinalize returns whether the document's children receive parent events
	// via <finalize> (§scxml-6.5).
	HasFinalize() bool

	// HasAutoforward returns whether the document autoforwards child events to
	// any invokes (§scxml-6.4.1).
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

	// InitializeDataModel initializes the datamodel via the script engine (§scxml-5.3).
	InitializeDataModel(engine *Engine[S, E])

	// ExecutePendingInvokes executes any pending <invoke> elements deferred during
	// entry (§scxml-6.4).
	ExecutePendingInvokes(engine *Engine[S, E])

	// ExecuteFinalizeForChildEvent executes <finalize> handlers for child events
	// (§scxml-6.5).
	ExecuteFinalizeForChildEvent(event *EventWithMetadata[E], engine *Engine[S, E])

	// GetActiveStates returns the active states for parallel state machines (§scxml-3.4).
	GetActiveStates() []S

	// SetActiveStates hands a machine that keeps its own active set that set
	// back (§scxml-3.4). The write half of GetActiveStates, and the only caller
	// is Engine.EnterAt: the ordinary entry and exit paths grow the set one
	// state at a time as they walk, while a restore is handed the whole
	// configuration at once.
	//
	// A no-op on a policy whose HasActiveStates is false — such a machine's
	// configuration is the parent walk from its current state, which EnterAt
	// restores by setting that state.
	SetActiveStates(states []S)

	// ForwardToAutoforwardChildren forwards external events to autoforward children
	// (§scxml-6.4.1).
	ForwardToAutoforwardChildren(eventName string, metadata EventMetadata, engine *Engine[S, E])

	// TickChildren ticks child state machines (§scxml-6.4).
	TickChildren(engine *Engine[S, E])

	// SetNextEventIsExternal sets the nextEventIsExternal_ flag (§scxml-5.10.1).
	SetNextEventIsExternal(value bool)

	// PopulateEventMetadata populates pending event metadata fields from an event's
	// metadata (§scxml-5.10).
	PopulateEventMetadata(meta *EventMetadata)

	// ClearEventMetadata clears pending event metadata after transition processing.
	ClearEventMetadata()
}
