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
// # What the policy answers, and what it does not
//
// The policy answers what only the document knows: its structure, which of a
// state's transitions an event enables, what a transition's content is, what a
// state's onentry and onexit do, what a history recorded. What W3C SCXML
// Appendix D does with those answers — the walk from each atomic state up
// through its ancestors, the ordered set, conflict removal, the exit set, the
// entry set — is microstep.go, written once for every machine. Every instance
// method below answers for ONE state or ONE transition.
//
// # History identity
//
// A <history> is named by a HistoryID the policy issues, not by a type of its
// own: Engine[S, E] is the type a host names, and a third type parameter on it
// would change that for every host to carry a fact only the policy uses.
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

	// InitialState returns the state the engine names before Initialize enters
	// the initial configuration (§scxml-3.2).
	//
	// Not what Initialize enters: that is the document's initial transition,
	// GetDocumentInitialTargets, which may name several states and a <history>.
	InitialState() S

	// IsFinalState returns whether state is a <final> state (§scxml-3.7).
	IsFinalState(state S) bool

	// GetParent returns the parent of state in the document hierarchy. The bool
	// return indicates whether a parent exists (false for root children).
	GetParent(state S) (S, bool)

	// IsCompoundState returns whether state is a compound state: a <state> with
	// child states (§scxml-3.3). A <parallel> answers false — this is Appendix
	// D's isCompoundState, and it decides which ancestors can be a transition's
	// domain.
	IsCompoundState(state S) bool

	// IsParallelState returns whether state is a <parallel> state (§scxml-3.4).
	IsParallelState(state S) bool

	// GetChildStates is §scxml-D-getChildStates: state's <state>, <parallel>
	// and <final> children, in document order — for a <parallel>, its regions.
	GetChildStates(state S) []S

	// GetInitialTargets is a compound state's initial transition target, as
	// written (§scxml-3.3) — one entry per token of `initial` or of the
	// <initial> element's transition, or the first child state when the
	// document names none. Empty for every state that is not compound.
	GetInitialTargets(state S) []EntryTarget[S, HistoryID]

	// GetDocumentInitialTargets is the target of the document's own initial
	// transition, as written (§scxml-3.2) — what Initialize enters, from the
	// <scxml> element.
	GetDocumentInitialTargets() []EntryTarget[S, HistoryID]

	// GetHistoryParent is the state a <history> is declared in (§scxml-3.10).
	GetHistoryParent(history HistoryID) S

	// GetHistoryDefaultTargets is a <history>'s default transition target, as
	// written — its default stored state configuration (§scxml-3.10.2).
	GetHistoryDefaultTargets(history HistoryID) []EntryTarget[S, HistoryID]

	// GetDocumentOrder returns the document order index of state (W3C SCXML Appendix D).
	// Document order is also entry order, and its reverse is exit order.
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
	// structural, and a default could only ever answer false. A policy that
	// had not emitted the table would then report every recorded configuration
	// as unknown — which a caller reads as "this run has no history" rather
	// than as a policy that is incomplete. Mirrors GetEventFromName, which is
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
	// (§scxml-3.13). The engine passes it to FirstEnabledTransition when it
	// selects eventless transitions.
	NullEvent() E

	// ================================================================
	// Run-time state the entry procedures read
	// ================================================================

	// HistoryValue reports what history recorded when its parent was last
	// exited, and false before that ever happened (§scxml-3.10).
	HistoryValue(history HistoryID) ([]S, bool)

	// ================================================================
	// Instance methods -- generated executable content
	//
	// These mirror Rust policy methods that take &mut Engine<Self> as a parameter.
	// Generated code mutates the policy via the receiver and calls engine methods
	// through the engine parameter. Each answers for ONE state or ONE
	// transition: which states a microstep exits and enters, and in which
	// order, is the engine's Appendix D procedure, not the policy's.
	// ================================================================

	// BindCurrentEvent binds the event whose transitions are about to be
	// selected as the _event their guards read (§scxml-5.10).
	//
	// Called once per selection, before the first guard runs, and with the
	// NullEvent for an eventless selection — which has no event of its own, so
	// a policy binds nothing for it. A document whose guards never read _event
	// has nothing to bind.
	BindCurrentEvent(event E, engine *Engine[S, E])

	// FirstEnabledTransition is Appendix D selectTransitions, the half only the
	// document can answer: the first of state's own transitions, in document
	// order, that event enables and whose guard holds. The NullEvent asks for
	// eventless transitions.
	//
	// The only place a guard is evaluated. The engine walks the atomic states
	// and their ancestors and keeps the ordered set.
	FirstEnabledTransition(state S, event E, engine *Engine[S, E]) (EnabledTransition[S, HistoryID], bool)

	// ExecuteTransitionContent executes one transition's executable content
	// (§scxml-3.13 — run between the microstep's exits and its entries).
	//
	// transitionIndex is the one FirstEnabledTransition reported for source.
	ExecuteTransitionContent(source S, transitionIndex int, engine *Engine[S, E])

	// ExecuteEntryActions enters state (§scxml-3.8): adds it to the
	// configuration, runs its <onentry>, and its initial transition's content
	// when isDefaultEntry. May raise internal events via engine.Raise(),
	// schedule delayed sends, and defer <invoke> starts until the macrostep
	// ends (§scxml-6.4).
	//
	// One state, and nothing below it: which states a microstep enters is the
	// engine's Appendix D entry set, entered front to back, so this neither
	// enters regions nor descends to an initial child. isDefaultEntry is the
	// entry set's statesForDefaultEntry answer — a compound state entered only
	// as an ANCESTOR of a deeper target was not entered by default, and its
	// initial transition content does not run (pinned by
	// integration_resources/ancestor_entry_is_not_default_entry/). For a
	// <final>, this is also what the appendix does on entering one.
	ExecuteEntryActions(state S, engine *Engine[S, E], isDefaultEntry bool)

	// ExecuteExitActions exits state (§scxml-3.9): records its histories, runs
	// its <onexit>, cancels its invocations and removes it from the
	// configuration.
	//
	// One state, and nothing below it: the engine's Appendix D exit set already
	// holds every active descendant, in exit order, ahead of this state.
	// configurationBeforeExit is the configuration as it stood before the
	// microstep's first exit, which every history of the microstep is recorded
	// from (§scxml-3.10).
	ExecuteExitActions(state S, engine *Engine[S, E], configurationBeforeExit []S)

	// ExecuteHistoryDefaultContent runs a <history>'s default transition
	// content (§scxml-3.10.2), after its parent's onentry (and after the
	// parent's own initial content) when the history was taken with nothing
	// recorded.
	ExecuteHistoryDefaultContent(history HistoryID, engine *Engine[S, E])

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

	// HasChildTick returns whether the policy supports child-tick for nested invokes.
	HasChildTick() bool

	// ================================================================
	// Optional instance methods
	//
	// Every generated policy implements all of these; one a document does not
	// need is emitted as a no-op, under the feature flag above that says so.
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

	// PopulateEventMetadata populates pending event metadata fields from an event's
	// metadata (§scxml-5.10).
	PopulateEventMetadata(meta *EventMetadata)

	// LiftTypedPayload binds the typed `_event.data` view the natively lowered
	// guards read (NL→IR Item C1 Path A), for the event being dequeued.
	//
	// The typed carrier `PopulateEventMetadata` reads is filled by ONE
	// producer: the generated `Raise<Event>` inject seam. Every other producer
	// fills `EventMetadata.Data`, so the fields are lifted out of that here and
	// the same guard answers the same way whichever producer sent the event —
	// including one on the far side of an invoke boundary.
	//
	// A non-nil error says the data cannot be read as this event's schema. The
	// engine then raises error.execution and the guard does not fire, which is
	// what §scxml-3.13 gives for a guard that cannot be evaluated, and what
	// the script engine gives for the same guard on the same data. A policy
	// with no typed payloads returns nil.
	LiftTypedPayload(event E, meta *EventMetadata) error

	// ClearEventMetadata clears pending event metadata after transition processing.
	ClearEventMetadata()
}

// PolicyDocument is a policy as microstep.go's procedures read the document:
// its static tables, and what each <history> recorded.
//
// The engine drives the procedures through it, and generated code asks two of
// them directly — whether a <parallel> has completed (IsInFinalState) and what
// a <history> records as its parent exits (RecordedHistory) — so both read the
// one transcription rather than a copy written into each machine.
type PolicyDocument[S comparable, E comparable] struct {
	Policy StatePolicy[S, E]
}

// ParentOf implements Document.
func (d PolicyDocument[S, E]) ParentOf(state S) (S, bool) { return d.Policy.GetParent(state) }

// IsCompound implements Document.
func (d PolicyDocument[S, E]) IsCompound(state S) bool { return d.Policy.IsCompoundState(state) }

// IsParallel implements Document.
func (d PolicyDocument[S, E]) IsParallel(state S) bool { return d.Policy.IsParallelState(state) }

// IsFinal implements Document.
func (d PolicyDocument[S, E]) IsFinal(state S) bool { return d.Policy.IsFinalState(state) }

// ChildStates implements Document.
func (d PolicyDocument[S, E]) ChildStates(state S) []S { return d.Policy.GetChildStates(state) }

// InitialTargets implements Document.
func (d PolicyDocument[S, E]) InitialTargets(state S) []EntryTarget[S, HistoryID] {
	return d.Policy.GetInitialTargets(state)
}

// HistoryParent implements Document.
func (d PolicyDocument[S, E]) HistoryParent(history HistoryID) S {
	return d.Policy.GetHistoryParent(history)
}

// HistoryValue implements Document.
func (d PolicyDocument[S, E]) HistoryValue(history HistoryID) ([]S, bool) {
	return d.Policy.HistoryValue(history)
}

// HistoryDefaultTargets implements Document.
func (d PolicyDocument[S, E]) HistoryDefaultTargets(history HistoryID) []EntryTarget[S, HistoryID] {
	return d.Policy.GetHistoryDefaultTargets(history)
}

// DocumentOrder implements Document.
func (d PolicyDocument[S, E]) DocumentOrder(state S) int { return d.Policy.GetDocumentOrder(state) }

// IsInFinalState is Appendix D's isInFinalState over this document and
// configuration — see the package function of the same name.
func (d PolicyDocument[S, E]) IsInFinalState(state S, configuration []S) bool {
	return IsInFinalState[S, HistoryID](d, state, configuration)
}

// RecordedHistory is what a <history> of parent records as parent exits —
// see the package function of the same name.
func (d PolicyDocument[S, E]) RecordedHistory(parent S, deep bool, configurationBeforeExit []S) []S {
	return RecordedHistory[S, HistoryID](d, parent, deep, configurationBeforeExit)
}
