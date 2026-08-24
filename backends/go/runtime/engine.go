// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

import (
	"log"
	"sort"
	"time"
)

// Engine is the SCXML execution engine.
//
// Generic over State (S) and Event (E) types. Takes a StatePolicy[S, E]
// interface that encodes the state machine structure. Matches Rust Engine<P>
// from backends/rust/runtime/src/engine.rs and C++ StaticExecutionEngine<StatePolicy>.
//
// # Threading Model
//
// eventOutcome is what the engine did with one event it offered to the active
// configuration.
//
// This used to be a bare bool meaning "the configuration changed", which
// answers false for two unrelated outcomes: an event no transition matched at
// all, and a targetless internal transition that ran its actions in place.
// Only the first is the discard §scxml-3.1.2 describes, and a
// count keyed off the old bool would have reported a handled event as one, so
// the two facts are spelled apart rather than inferred from each other.
// Mirrors the Rust runtime's EventOutcome.
type eventOutcome struct {
	// selected is whether any transition matched the event.
	selected bool
	// configurationChanged is false for a targetless internal transition,
	// which leaves the configuration alone.
	configurationChanged bool
}

// Engine is NOT safe for concurrent use. Callers needing multi-goroutine access
// must protect with sync.Mutex. This matches the C++ and Rust single-threaded
// microstep loop design.
type Engine[S comparable, E comparable] struct {
	// policy is the generated per-SM policy struct (datamodel, last-transition flags, etc.).
	policy StatePolicy[S, E]

	// currentState is the currently active state (or deepest active state for parallel machines).
	currentState S

	// internalQueue is the §scxml-C-1 internal event queue (high priority).
	internalQueue *EventQueueManager[EventWithMetadata[E]]

	// externalQueue is the §scxml-C-1 external event queue (low priority).
	externalQueue *EventQueueManager[EventWithMetadata[E]]

	// isRunning tracks whether the engine is currently running.
	isRunning bool

	// completionCallback is the §scxml-6.4 callback invoked when reaching a final state.
	completionCallback func()

	// onHTTPSend is the §scxml-C-2 HTTP send dispatch callback.
	// Returns *HttpSendResponse when real HTTP is used; nil for fire-and-forget.
	onHTTPSend func(HttpSendRequest) *HttpSendResponse

	// hostProcessors holds what serves each §scxml-6.2.5 Event I/O Processor
	// type the host declared to this build. Keyed by the `type` string, which
	// is what a `<send>` names; see host_processor.go.
	hostProcessors map[string]HostSendHandler

	// hostInvokers holds what RUNS each §scxml-6.4.1 `<invoke type>` the host
	// declared. A second map rather than a second use of the one above,
	// because delivering an event is not the same capability as running a
	// process with a lifecycle — see host_processor.go's invoker half.
	hostInvokers map[string]HostInvokeHandler

	// startedHostInvokes records every host-run invocation started and not yet
	// cancelled, so the exit chain can be an unconditional call: the engine
	// knows whether there is anything to cancel.
	startedHostInvokes map[hostInvokeKey]struct{}

	// scheduler is the §scxml-6.2 delayed event scheduler.
	scheduler *PullScheduler[E]

	// clock is where this engine reads "now" from — see SceClock. Nil until
	// first use, so an engine constructed before a clock is installed still
	// gets the MonotonicClock default.
	clock SceClock

	// turnNowMs is the reading clock gave when the current turn began, valid
	// only while inTurn — see beginTurn.
	turnNowMs int64
	inTurn    bool

	// discardedExternalEvents counts events taken off the external queue that
	// no transition matched (§scxml-3.1.2) — see DiscardedExternalEvents.
	discardedExternalEvents uint32

	// lastDiscardedEvent is the most recent event counted above; hasDiscarded
	// says whether there is one, because the zero value of E is a real event.
	lastDiscardedEvent E
	hasDiscarded       bool

	// unseenExternalEvents counts external events this machine never dequeued
	// because it had already stopped — see UnseenExternalEvents. hasUnseen
	// says whether lastUnseenEvent holds one, because the zero value of E is a
	// real event.
	unseenExternalEvents uint32
	lastUnseenEvent      E
	hasUnseen            bool

	// undecodablePayloads counts deliveries whose payload announced structure
	// and that the datamodel could not read as one (§scxml-B-2-8-1) — see
	// UndecodablePayloads. hasUndecodable says whether lastUndecodablePayload
	// holds one, because the zero value of E is a real event.
	undecodablePayloads    uint32
	lastUndecodablePayload E
	hasUndecodable         bool

	// unhandledErrorEvents counts error.* events this engine raised that no
	// transition matched (§scxml-3.12.2) — see UnhandledErrorEvents.
	unhandledErrorEvents uint32

	// lastUnhandledError is the most recent error event counted above;
	// hasUnhandledError says whether there is one, because the zero value of E
	// is a real event.
	lastUnhandledError E
	hasUnhandledError  bool

	// handlingErrorEvent says the drain is currently executing a transition
	// selected by an error.* event — the state in which a newly raised error
	// is a link in a chain rather than a first failure. It is the whole
	// discriminator behind ErrorCascadeEvents: a document answering five
	// hundred separate failures cleanly never sets it twice in a row.
	handlingErrorEvent bool

	// errorCascadeDepth is how many links the current chain has, reset the
	// moment the drain does anything else — see ErrorCascadeEvents.
	errorCascadeDepth uint32

	// errorCascadeEvents counts error.* events refused because the chain that
	// raised them had reached maxErrorCascadeDepth — see ErrorCascadeEvents.
	errorCascadeEvents uint32

	// lastErrorCascadeEvent is the most recent error event refused that way;
	// hasErrorCascadeEvent says whether there is one, because the zero value
	// of E is a real event.
	lastErrorCascadeEvent E
	hasErrorCascadeEvent  bool

	// truncatedMacrosteps counts macrosteps this engine stopped at
	// maxMacrostepMicrosteps with the chain still going — see
	// TruncatedMacrosteps.
	truncatedMacrosteps uint32

	// macrostepMicrostepsTaken counts microsteps taken by the macrostep now in
	// progress, against maxMacrostepMicrosteps. A field rather than a local,
	// for the reason Appendix D's loop is one loop: the eventless branch and
	// the internal-event branch take turns inside a single macrostep, so a
	// counter that lives in either one alone is reset by the other and bounds
	// nothing.
	macrostepMicrostepsTaken int

	// lastTruncatedMacrostepState is the state the drain was in when it last
	// stopped that way; hasTruncatedMacrostep says whether there is one,
	// because the zero value of S is a real state.
	lastTruncatedMacrostepState S
	hasTruncatedMacrostep       bool

	// macrostepTruncated says the macrostep now in progress has already been
	// stopped at the ceiling. The drain is reached twice per macrostep — once
	// from executeTransition and once from the main event loop's own loop —
	// so without this the ceiling is not a ceiling: each caller gets a fresh
	// budget and the machine takes twice the microsteps it was allowed,
	// counting each refusal separately. Cleared where the algorithm starts a
	// macrostep, which is the external dequeue.
	macrostepTruncated bool

	// donedataAtFinal is the §scxml-5.5 + 6.3.1 stashed donedata payload
	// for a top-level <final>. Entry actions stash it here; an invoking
	// parent reads it back via DonedataAtFinal() to lift onto
	// done.invoke.<id>._event.data. Mirrors the C++ AOT
	// stashDonedataAtFinal contract and the Rust Engine::donedata_at_final
	// field.
	donedataAtFinal string
}

// NewEngine constructs a new engine with the given policy instance.
//
// The initial state is set to policy.InitialState(). The engine is not yet
// running -- call Initialize() to enter the initial configuration and begin
// processing events.
//
// Matches Rust Engine::new.
func NewEngine[S comparable, E comparable](policy StatePolicy[S, E]) *Engine[S, E] {
	return &Engine[S, E]{
		policy:          policy,
		currentState:    policy.InitialState(),
		internalQueue:   NewEventQueueManager[EventWithMetadata[E]](),
		externalQueue:   NewEventQueueManager[EventWithMetadata[E]](),
		isRunning:       false,
		scheduler:       NewPullScheduler[E](),
		donedataAtFinal: "",
	}
}

// ================================================================
// Lifecycle (matches Rust/C++ public API)
// ================================================================

// Initialize enters the initial configuration and runs the macrostep loop until
// stable.
//
// Matches Rust Engine::initialize. §scxml-5.3 guarantees datamodel
// initialization happens before any state entry.
func (e *Engine[S, E]) Initialize() {
	// §scxml-3.13: entering the initial configuration is one turn, and the
	// <onentry> handlers it runs arm their <send delay>s against one instant —
	// see beginTurn for what reading the clock per <send> did to two of them.
	opened := e.beginTurn()
	defer e.endTurn(opened)

	e.isRunning = true

	// §scxml-5.3: Initialize datamodel before any state entry
	if e.policy.NeedsDataModelInit() {
		e.policy.InitializeDataModel(e)
	}

	// §scxml-3.3: Entry chain from root to initial leaf
	entryChain := BuildEntryChain[S, E](e.policy, e.currentState)
	e.executeEntryChain(entryChain)

	// §scxml-3.3: Resolve currentState to the deepest initial leaf
	e.resolveCurrentStateToLeaf()

	// W3C SCXML Appendix D: hand over to the outer loop. The macrostep
	// completes on eventless transitions and internal events, then the invokes
	// for the states just entered run, and only then is anything taken off the
	// external queue — so an autoforward child is live for every event
	// <onentry> queued on the way in.
	log.Printf("[sce] Engine::Initialize: entering main event loop")
	e.runMainEventLoop()
	log.Printf("[sce] Engine::Initialize: main event loop settled")

	// §scxml-6.4: Fire completion callback if we reached a final state during init
	if e.isInFinalState() && e.completionCallback != nil {
		log.Printf("[sce] Engine::Initialize: reached final state during init, invoking completion callback")
		active := e.GetActiveStates()
		finalState := e.currentState
		e.policy.ExecuteExitActions(finalState, e, active)
		e.completionCallback()
	}
}

// EnterAt enters a configuration this document already describes, without
// re-running the <onentry> actions an earlier run already ran (§scxml-3.2).
//
// # What it is for
//
// A host that persisted where a machine was and is bringing it back in a new
// process. Initialize replays the document from its initial state, which runs
// every <onentry> on the way in — a resumed session would send its greeting
// twice, re-arm timers, and re-issue acts whose recipients already received
// them.
//
// # What it takes, and why two arguments
//
// Exactly what the two readers on this engine publish: GetActiveStates and
// GetCurrentState. Handing back both is not redundancy — for a machine with
// <parallel> states the configuration does not determine the current state.
// currentState is the leaf the engine descended to, so which region it sits in
// is a fact about the transition history rather than about the configuration,
// and a set alone cannot recover it.
//
// # What it refuses
//
// Every set that is not a configuration of THIS document — see
// ValidateConfiguration for the rules and ConfigurationRejection for what each
// refusal names. Validation runs before any mutation, so a refused call leaves
// the engine exactly as it was; entering "near" the requested configuration is
// the one outcome this door must never produce, because a host has no way to
// detect it afterwards.
//
// # What it does not do
//
//   - No <onentry>, and no <onexit>: no state is entered or left.
//   - No macrostep. Initialize settles the machine before returning; this does
//     not, because the configuration handed in was already a settled one —
//     running the loop here could take an eventless transition the earlier run
//     had no reason to take, and fire the <send>s on the way. The host drives
//     the machine on with Step or Tick as it otherwise would.
//   - No datamodel restore. §scxml-5.3 declaration still runs, so the variables
//     exist with their document defaults and a host can then put its saved
//     values back through IScriptEngine — the engine does not persist datamodel
//     state and does not pretend to.
//
// The Go twin of the Rust runtime's Engine::enter_at and of the C++
// StaticExecutionEngine::enterAt.
//
// Returns ConfigurationAccepted on success; the rule that was broken otherwise,
// with the engine untouched.
func (e *Engine[S, E]) EnterAt(configuration []S, current S) ConfigurationRejection {
	// Before anything is touched: a rejection must not half-enter.
	verdict := ValidateConfiguration[S, E](e.policy, configuration, current)
	if verdict != ConfigurationAccepted {
		log.Printf("[sce] Engine::EnterAt: refused -- %s", verdict)
		return verdict
	}

	// §scxml-5.3: the datamodel is declared before anything can read it. This
	// is not a state entry action -- <datamodel> holds <data>, not executable
	// content -- so it runs here for the same reason it runs in Initialize: a
	// cond or an assign evaluated after this call would otherwise reference
	// variables that were never declared.
	if e.policy.NeedsDataModelInit() {
		e.policy.InitializeDataModel(e)
	}

	e.currentState = current

	// §scxml-3.4: a machine that keeps its own active set is handed it back.
	// The condition is the one the generator emits SetActiveStates under, so a
	// policy reached here has the override.
	if e.policy.HasActiveStates() {
		e.policy.SetActiveStates(configuration)
	}

	e.isRunning = true
	return ConfigurationAccepted
}

// Step processes one macrostep: drain queues and run eventless transitions.
//
// Matches Rust Engine::step. Used by parent SMs to explicitly drive children
// after sending them events (§scxml-6.4).
func (e *Engine[S, E]) Step() {
	// §scxml-3.13: one host call, one reading. The macrostep below can enter a
	// state whose <onentry> arms several <send delay>s, and they are one
	// instant's worth of executable content however long the host takes to run
	// them — see beginTurn.
	opened := e.beginTurn()
	defer e.endTurn(opened)

	e.runMainEventLoop()

	if e.isInFinalState() && e.completionCallback != nil {
		log.Printf("[sce] Engine::Step: invoking completion callback")
		e.completionCallback()
	}
}

// Tick polls the scheduler for ready delayed events, then runs a macrostep.
//
// Matches Rust Engine::tick. Called periodically by callers that have delayed
// <send> operations.
func (e *Engine[S, E]) Tick() {
	// §scxml-3.13: one turn, one reading. Everything below judges due against
	// the instant this tick began, and everything the macrosteps below arm is
	// measured from it — so a tick dispatches what was due when the host called
	// it, and cannot be extended by how long it takes to run (see beginTurn).
	opened := e.beginTurn()
	defer e.endTurn(opened)

	if !e.isRunning {
		return
	}
	if e.isInFinalState() {
		if e.completionCallback != nil {
			log.Printf("[sce] Engine::Tick: final state already reached, invoking completion callback")
			e.completionCallback()
		}
		return
	}

	// §scxml-6.2: dispatch the ready scheduled events, earliest deadline first
	// and one macrostep apart.
	//
	// One at a time, not all at once. <cancel> drops an event that has not been
	// dispatched yet, and a host that woke late holds several past their
	// deadlines: promoting them together makes every later one undroppable
	// before the earlier one's transitions have had a chance to run. That is
	// how a settle timer — arm a long <send delay>, cancel it when the short
	// signal arrives first — delivers the event it was told to cancel.
	// Measured 2026-08-19 on the Go, Rust and Python backends alike.
	//
	// "Due" is judged against the instant this tick began, not against a clock
	// re-read on every pass: a tick that chased its own slowness would dispatch
	// entries the host had not yet reached, in a loop the host cannot get
	// between (see beginTurn).
	for {
		event, data, hostSend, ok := e.scheduler.PopReadyActAt(e.turnNowMs)
		if !ok {
			break
		}
		if hostSend != nil {
			// §scxml-6.2.4: the wait is over, so now the act happens.
			e.performDeferredHostSend(*hostSend)
		} else {
			e.RaiseExternal(event, data, "")
		}
		// The macrostep this act drives may <cancel> a later one, so the
		// queue is re-consulted after it rather than before.
		e.runMainEventLoop()
		if !e.isRunning || e.isInFinalState() {
			break
		}
	}

	// §scxml-6.4: Tick child state machines
	if e.policy.HasChildTick() {
		e.policy.TickChildren(e)
	}

	// Delegate to Step() for the main event loop + completion callback.
	// §scxml-6.4's invokes are part of that loop and run there, ahead of
	// the external dequeue rather than after it.
	e.Step()
}

// Stop stops the engine. Subsequent Tick/ProcessEvent calls become no-ops.
func (e *Engine[S, E]) Stop() {
	e.isRunning = false
}

// IsRunning returns whether the engine is running (not stopped or awaiting completion).
func (e *Engine[S, E]) IsRunning() bool {
	return e.isRunning
}

// GetCurrentState returns the current active (leaf) state.
func (e *Engine[S, E]) GetCurrentState() S {
	return e.currentState
}

// GetActiveStates returns the full list of active states (§scxml-3.11).
//
// Non-parallel machines: returns the hierarchy [leaf, parent, grandparent, ..., root].
// Parallel machines: returns the union of all active regions via
// StatePolicy.GetActiveStates().
func (e *Engine[S, E]) GetActiveStates() []S {
	if e.policy.HasActiveStates() {
		return e.policy.GetActiveStates()
	}
	// Walk from current state up to root
	active := make([]S, 0, 8)
	current := e.currentState
	for {
		active = append(active, current)
		parent, ok := e.policy.GetParent(current)
		if !ok {
			break
		}
		current = parent
	}
	return active
}

// isInFinalState reports whether this session has ended — that is, whether the
// current state is a <final> whose parent is the <scxml> element
// (§scxml-3.7).
//
// Appendix D enterStates sets running = false for a <final> only when
// isSCXMLElement(s.parent); a nested one queues done.state.<parent> and the
// machine carries on. So the structural question — "is this state a <final>
// element" — is StatePolicy.IsFinalState, and it is not the completion
// criterion on its own. Everything that means "the machine is done" keys on
// this method: RunUntilCompletion, the completion callback, and the
// done.invoke.<id> a parent emits for an invoked child.
func (e *Engine[S, E]) isInFinalState() bool {
	if !e.policy.IsFinalState(e.currentState) {
		return false
	}
	_, hasParent := e.policy.GetParent(e.currentState)
	return !hasParent
}

// IsInFinalState is the exported version for external callers.
func (e *Engine[S, E]) IsInFinalState() bool {
	return e.isInFinalState()
}

// Policy returns the inner policy (read-only access).
func (e *Engine[S, E]) Policy() StatePolicy[S, E] {
	return e.policy
}

// ================================================================
// Event submission (matches Rust raise / raiseExternal overloads)
// ================================================================

// maxErrorCascadeDepth is how many links an error.* chain may have before the
// engine stops feeding it — see ErrorCascadeEvents.
//
// §scxml-3.12.2 says what to do with an error event nothing matches. It does
// not say what to do when something does match it and that handler fails too:
// the failure raises the same error, the same transition answers it, and the
// machine has no way out. Nothing in the specification bounds that, so the
// number is this engine's to choose, and it matches checkEventlessTransitions'
// ceiling — the sibling case of a document that cannot finish a macrostep,
// decided the same way for the same reason.
//
// A hundred links is far past any repair strategy a document plausibly spells
// (a handler that tries a fallback, then a second one, is three) and far short
// of a number a host would wait through: measured 2026-08-19, the Python
// engine ran 37,000 links a second on a two-line document, so an unattended
// supervisor did not hang — it burned a core until it was killed.
const maxErrorCascadeDepth uint32 = 100

// Raise enqueues an internal event with full metadata (high priority)
// (§scxml-C-1).
//
// Matches Rust Engine::raise.
//
// An error.* event raised while an error handler is running is refused once
// the chain reaches maxErrorCascadeDepth — see ErrorCascadeEvents for why the
// engine is the one that has to stop it. Only the engine's own error events
// are refused: an author's <raise> inside an error handler is the document
// doing its job and rides the queue like any other.
func (e *Engine[S, E]) Raise(event EventWithMetadata[E]) {
	// §scxml-3.12.2 names the error events this refuses; the clause itself is
	// silent on a handler that fails, which is why the ceiling is a choice
	// this engine documents rather than a rule it implements.
	if e.handlingErrorEvent && IsErrorEvent(e.policy.GetEventName(event.Event)) {
		e.errorCascadeDepth++
		if e.errorCascadeDepth >= maxErrorCascadeDepth {
			e.errorCascadeEvents++
			e.lastErrorCascadeEvent = event.Event
			e.hasErrorCascadeEvent = true
			if e.errorCascadeEvents == 1 {
				log.Printf("[sce] Engine::Raise: an error handler has raised an error %d times over; "+
					"refusing to feed the chain — the document's error handling is failing",
					maxErrorCascadeDepth)
			}
			return
		}
	}
	e.internalQueue.Raise(event)
}

// RaiseExternal enqueues an external event with optional data and origin
// (§scxml-C-1 / 6.2).
//
// Matches Rust Engine::raise_external.
func (e *Engine[S, E]) RaiseExternal(event E, eventData, origin string) {
	meta := NewEventWithFields(
		event,
		eventData,
		origin,
		"",                      // sendID
		EventTypeExternal,       // eventType
		SCXMLEventProcessorType, // originType
		"",                      // invokeID
		"",                      // target
	)
	e.externalQueue.Raise(meta)

	// §scxml-5.10.1: Mark next event as external for _event.type
	if e.policy.HasExternalEventFlag() {
		e.policy.SetNextEventIsExternal(true)
	}
}

// RaiseExternalByName raises an external event by name (§scxml-6.4.1, for
// child autoforward).
//
// If the name does not match any known event, the call is silently ignored.
// Matches Rust Engine::raise_external_by_name.
func (e *Engine[S, E]) RaiseExternalByName(eventName, eventData string) {
	event, ok := e.policy.GetEventFromName(eventName)
	if !ok {
		log.Printf("[sce] Engine::RaiseExternalByName: event '%s' not in enum, ignoring", eventName)
		return
	}
	e.RaiseExternal(event, eventData, "")
}

// RaiseExternalByNameWithMeta raises an autoforwarded external event that is
// name-addressed but carries the source event's _event fields (§scxml-6.4).
//
// Autoforward is the one path where an event leaves the machine owning its enum,
// so it crosses by name while the metadata travels with it. Unknown names
// degrade silently: a child need not declare every event its parent forwards.
func (e *Engine[S, E]) RaiseExternalByNameWithMeta(eventName string, metadata EventMetadata) {
	event, ok := e.policy.GetEventFromName(eventName)
	if !ok {
		log.Printf("[sce] Engine::RaiseExternalByNameWithMeta: event '%s' not in enum, ignoring", eventName)
		return
	}
	// Target stays empty: the copy is delivered to this machine, never
	// re-routed to the original event's target.
	e.RaiseExternalWithMeta(EventWithMetadata[E]{Event: event, Metadata: metadata})
}

// RaiseExternalWithMeta raises an external event with full metadata (W3C SCXML
// 6.4.1, for child-to-parent events).
//
// Preserves invokeid for parent finalize handlers.
// Matches Rust Engine::raise_external_with_meta.
func (e *Engine[S, E]) RaiseExternalWithMeta(event EventWithMetadata[E]) {
	log.Printf("[sce] Engine::RaiseExternalWithMeta: enqueuing external event with metadata")

	e.externalQueue.Raise(event)

	if e.policy.HasExternalEventFlag() {
		e.policy.SetNextEventIsExternal(true)
	}
}

// ProcessEvent processes an external event (convenience API, runs one macrostep)
// (§scxml-3.12).
//
// Matches Rust Engine::process_event.
func (e *Engine[S, E]) ProcessEvent(event E) {
	if !e.isRunning {
		// Refused rather than queued, so the drain in runMainEventLoop never
		// sees it — which is why the count is taken here as well as there.
		e.noteUnseenEvent(event)
		return
	}
	e.RaiseExternal(event, "", "")
	e.Step()
}

// ProcessEventWithMeta processes an external event with metadata (§scxml-5.10).
//
// Matches Rust Engine::process_event_with_meta.
func (e *Engine[S, E]) ProcessEventWithMeta(event E, metadata EventMetadata) {
	if !e.isRunning {
		// Same refusal as ProcessEvent, same reason it is counted here.
		e.noteUnseenEvent(event)
		return
	}
	meta := EventWithMetadata[E]{
		Event:    event,
		Metadata: metadata,
	}
	e.externalQueue.Raise(meta)
	e.Step()
}

// ================================================================
// Donedata stash (§scxml-5.5 + 6.3.1)
// ================================================================

// StashDonedataAtFinal records the donedata payload evaluated on a
// top-level <final> entry so an invoking parent can lift it onto
// done.invoke.<id>._event.data.
//
// Matches C++ StaticExecutionEngine::stashDonedataAtFinal and Rust
// Engine::stash_donedata_at_final.
func (e *Engine[S, E]) StashDonedataAtFinal(data string) {
	e.donedataAtFinal = data
}

// DonedataAtFinal returns the donedata payload stashed on top-level
// <final> entry, or "" if none was stashed.
//
// Matches C++ StaticExecutionEngine::donedataAtFinal and Rust
// Engine::donedata_at_final.
func (e *Engine[S, E]) DonedataAtFinal() string {
	return e.donedataAtFinal
}

// ================================================================
// Scheduler passthrough
// ================================================================

// ScheduleEvent schedules an event for delayed delivery. Returns the send ID.
//
// The deadline is this turn's instant plus the delay — see beginTurn for why
// the reading is the turn's rather than this statement's.
func (e *Engine[S, E]) ScheduleEvent(event E, delay time.Duration, sendID, eventData string) string {
	readyAtMs := e.schedNowMs() + int64(delay/time.Millisecond)
	return e.scheduler.ScheduleEventAt(event, readyAtMs, sendID, eventData)
}

// ScheduleHostSend arms a host-served `<send delay>`, to be performed when the
// delay elapses (§scxml-6.2.4 + §scxml-6.2.5). Returns the send ID.
//
// The delayed twin of PerformHostSend, called by the generated send site in its
// place when the document wrote a delay. §scxml-6.2.4 makes the wait a property
// of the send and not of the processor it named, so the two differ in WHEN the
// act happens and in nothing else — including the §scxml-6.2 report owed when
// nobody performs it, which Tick makes at the deadline.
//
// The act lands in the same queue as ScheduleEvent, so CancelEvent drops it
// (§scxml-6.3) and TimeUntilNextScheduled counts it.
func (e *Engine[S, E]) ScheduleHostSend(request HostSendRequest, delay time.Duration, sendID string) string {
	readyAtMs := e.schedNowMs() + int64(delay/time.Millisecond)
	return e.scheduler.ScheduleHostSendAt(request, readyAtMs, sendID)
}

// CancelEvent cancels a previously scheduled event by send ID.
func (e *Engine[S, E]) CancelEvent(sendID string) bool {
	return e.scheduler.CancelEvent(sendID)
}

// HasReadyEvents returns whether the scheduler has events ready to fire.
func (e *Engine[S, E]) HasReadyEvents() bool {
	return e.scheduler.HasReadyEventsAt(e.schedNowMs())
}

// TimeUntilNextScheduled reports how long until this machine next needs Tick.
// Zero means something is due now; the bool is false when the scheduler is
// empty and no clock-driven wake-up is owed.
//
// NeedsEventScheduler tells a host *which* entry point to drive the machine
// with. This tells it *when*, and a host that cannot ask has only one move
// left: pick a polling interval. That guess is not free in either direction —
// measured on a document whose <send delay="200ms"> is cancelled by a 100 ms
// signal, a 1 ms interval spends 176 wasted ticks to be on time, a 500 ms one
// fires 300 ms late, and a 250 ms one steps over both deadlines at once. An
// interval cannot straddle two deadlines it was never told about.
//
// The answer feeds a host loop directly — a time.After in a select, a
// context deadline, a ticker reset.
func (e *Engine[S, E]) TimeUntilNextScheduled() (time.Duration, bool) {
	nextMs, ok := e.scheduler.NextReadyAtMs()
	if !ok {
		return 0, false
	}
	remaining := nextMs - e.schedNowMs()
	if remaining < 0 {
		return 0, true
	}
	return time.Duration(remaining) * time.Millisecond, true
}

// ================================================================
// Clock (§scxml-6.2.2)
// ================================================================

// Clock reports where this engine reads "now" from — see SceClock.
//
// Never nil: an engine that was never given one reads a MonotonicClock, which
// is installed on first use.
func (e *Engine[S, E]) Clock() SceClock {
	if e.clock == nil {
		e.clock = NewMonotonicClock()
	}
	return e.clock
}

// SetClock installs the SceClock this engine measures its <send delay>
// deadlines against.
//
// Must be called before Initialize: the entry configuration's <onentry> can arm
// delayed sends, and swapping the clock under deadlines already computed from
// another one would leave the queue holding two incomparable time bases. That
// is a programming error rather than a recoverable condition, so it panics.
func (e *Engine[S, E]) SetClock(clock SceClock) {
	if clock == nil {
		panic("sce: Engine.SetClock requires a clock; pass NewMonotonicClock() for the default")
	}
	if e.isRunning {
		panic("sce: Engine.SetClock must be called before Initialize(): this engine has " +
			"already armed its entry configuration against the previous clock, and " +
			"deadlines from two clocks do not compare")
	}
	e.clock = clock
}

// AdvanceTimeMs moves this engine's clock forward by ms and runs whatever that
// made due (§scxml-6.2).
//
// The host-owned twin of Tick: Tick asks a clock that moves on its own what
// time it is, this one *sets* what time it is and then ticks. A machine driven
// exclusively through here has no dependency on the load of the machine it runs
// on — the same sequence of calls produces the same configuration every time.
//
// Panics unless Clock is a *ManualClock, because that is the only kind of clock
// a host can move. Calling it against the monotonic default is a programming
// error, not a no-op: it means the caller believes it owns time and it does
// not, and the events it is waiting for will arrive on a schedule it did not
// choose.
func (e *Engine[S, E]) AdvanceTimeMs(ms int64) {
	manual, ok := e.Clock().(*ManualClock)
	if !ok {
		panic("sce: Engine.AdvanceTimeMs needs a *ManualClock in Clock(); this engine's " +
			"time is not the host's to move. Call SetClock(NewManualClock(0)) before " +
			"Initialize(), or drive this machine with Tick() and TimeUntilNextScheduled()")
	}
	manual.Advance(ms)
	e.Tick()
}

// NowMs reports this engine's current reading of Clock, in milliseconds since
// that clock's origin.
//
// The absolute counterpart of TimeUntilNextScheduled's relative answer. A host
// owning time through a ManualClock uses it to say where in the run it is; a
// host on the wall clock uses it to correlate an engine's deadlines with its
// own log.
func (e *Engine[S, E]) NowMs() int64 {
	return e.schedNowMs()
}

// schedNowMs answers §scxml-3.13's question — what time it is, for everything
// this turn arms or judges.
//
// The clause executes a microstep's executable content as one unit and a
// macrostep as a chain of those, so "now" is a property of the turn the engine
// is in rather than of the statement asking for it. Between turns there is no
// turn for it to be a property of, and the host's queries
// (TimeUntilNextScheduled, NowMs) read the clock live.
func (e *Engine[S, E]) schedNowMs() int64 {
	if e.inTurn {
		return e.turnNowMs
	}
	return e.Clock().ElapsedMs()
}

// beginTurn opens a turn: it takes the single clock reading that everything
// inside it uses.
//
// Returns whether this call is the one that opened it, which endTurn needs so a
// nested entry point (ProcessEvent delegating to Step, Tick doing the same)
// does not close the outer turn early.
//
// §scxml-6.2.2 makes a delay the wait the DOCUMENT asks for — "how long the
// processor should wait before dispatching the message". Time the host spent
// descheduled between two statements of one microstep is not part of any delay
// the document wrote, so it must not reach the deadline. Reading the clock per
// statement instead was two defects at once, both measured on this backend:
//
//   - Two <send delay>s executed by one <onentry> took a reading each, so a
//     host descheduled between them by more than the gap between their delays
//     got the later send's deadline first — and the engine then dispatched them
//     in that order, so the document's <cancel> arrived after the event it
//     named. Which of two events the author ordered arrives first became a fact
//     about the host's scheduler.
//   - The dispatch loop in Tick re-read it on every pass, so a tick slow enough
//     to cross the next deadline dispatched that entry too, then the one after
//     it — the engine chasing deadlines its own slowness created, in a loop the
//     host cannot get between.
//
// Neither is reachable from a clock that is read once per turn.
func (e *Engine[S, E]) beginTurn() bool {
	if e.inTurn {
		return false
	}
	e.turnNowMs = e.Clock().ElapsedMs()
	e.inTurn = true
	return true
}

// endTurn closes a turn opened by beginTurn.
func (e *Engine[S, E]) endTurn(opened bool) {
	if opened {
		e.inTurn = false
	}
}

// DiscardedExternalEvents reports how many events this engine took off the
// external queue and discarded because no transition in any active state
// matched them (§scxml-3.1.2).
//
// Discarding is what the clause requires. This is the part the clause does not
// cover: the host that queued the event cannot otherwise tell that outcome
// from a handled one, because a self transition, a targetless internal
// transition and a discard all leave the configuration alone. Comparing the
// count across a drive is what turns "the machine ignored what I sent" into
// something the program can see.
//
// The C++ Interpreter has answered this all along (processEvent's
// TransitionResult.success, and getStatistics().failedTransitions); this is the
// generated engines' side of the same question.
//
// Counts external-queue events only. An internal <raise> that matches nothing
// is discarded too, but both ends of that are inside the document.
func (e *Engine[S, E]) DiscardedExternalEvents() uint32 {
	return e.discardedExternalEvents
}

// LastDiscardedEvent reports the most recent event DiscardedExternalEvents
// counted. The bool is false while that count is zero — the zero value of E is
// a real event, so it cannot stand in for "none".
//
// A count says something went nowhere; this says which thing did, which is the
// question a host debugging a stalled supervisor actually has.
func (e *Engine[S, E]) LastDiscardedEvent() (E, bool) {
	return e.lastDiscardedEvent, e.hasDiscarded
}

// NotePayloadReading records which §scxml-B-2-8-1 rung the payload just bound
// got.
//
// Called by generated code immediately after it binds _event, because that is
// the only moment the rung is known. Four of the five readings are the ladder
// working and are recorded by being ignored; the fifth is the one a host is
// wrong about.
func (e *Engine[S, E]) NotePayloadReading(event E, reading PayloadReading) {
	if reading == PayloadUndecodable {
		e.undecodablePayloads++
		e.lastUndecodablePayload = event
		e.hasUndecodable = true
	}
}

// UndecodablePayloads reports how many events arrived carrying a payload that
// announced itself as structure and that the datamodel could not read as one.
//
// §scxml-B-2-8-1 requires the fallback: content the processor cannot interpret
// becomes a space-normalized string. What it does not require — and what
// nothing here used to provide — is any way for the host that SENT that
// payload to learn its fields have stopped existing. The document reads
// _event.data.field, gets nothing, assigns nothing, and the run continues;
// measured 2026-08-22 on three independent Lua implementations, a payload in
// Lua's own table syntax silently emptied every variable the receiving
// transition assigned, including the one that primes the next session.
//
// Counts only the reading a host can act on. Prose delivered as text is the
// ladder working (W3C test 562) and is not counted, because a diagnostic that
// fires when nothing is wrong is one nobody reads.
func (e *Engine[S, E]) UndecodablePayloads() uint32 {
	return e.undecodablePayloads
}

// LastUndecodablePayload reports the most recent event UndecodablePayloads
// counted. The bool is false while that count is zero, for the same reason as
// LastDiscardedEvent: the zero value of E is a real event.
func (e *Engine[S, E]) LastUndecodablePayload() (E, bool) {
	return e.lastUndecodablePayload, e.hasUndecodable
}

// noteUnseenEvent records one external event this machine will never look at.
func (e *Engine[S, E]) noteUnseenEvent(event E) {
	// W3C SCXML Appendix D's main event loop: the loop that would have dequeued this has
	// ended, so the event is not "pending" — it is over.
	e.unseenExternalEvents++
	e.lastUnseenEvent = event
	e.hasUnseen = true
}

// recordUnseenExternalEvents empties the external queue into the count above,
// at the moment the main event loop ends.
//
// Drained rather than left in place so each event is counted exactly once: a
// host that keeps calling Step() on a halted machine would otherwise re-count
// the same queue on every call, and a count that grows while nothing arrives
// is a count nobody can use.
func (e *Engine[S, E]) recordUnseenExternalEvents() {
	for {
		meta, ok := e.externalQueue.Pop()
		if !ok {
			return
		}
		e.noteUnseenEvent(meta.Event)
	}
}

// UnseenExternalEvents reports how many external events the host handed this
// machine that it never looked at, because it had already stopped.
//
// W3C SCXML Appendix D's main event loop exits when the machine reaches a top-level final
// state, and §scxml-3.13 is explicit that the interpreter is then done.
// Refusing the event is therefore correct — and, exactly as with
// DiscardedExternalEvents and UndecodablePayloads, being unable to SAY it
// happened is not part of the clause.
//
// This is the count that separates the third explanation from the other two. A
// host that sent an event and saw nothing move has three candidates:
//
//	dequeued, no transition matched          -> DiscardedExternalEvents
//	dequeued, a transition matched but its
//	  guard was false                        -> neither
//	never dequeued — the machine had stopped -> this one
//
// Measured 2026-08-22: a consumer reported a guarded transition that "never
// fires", and four rewrites of the guard later the guard was still the
// suspect. Driving the same document here fired it on the first try, at that
// consumer's own pinned revision — so the difference was never the guard, and
// nothing in this engine could have said so.
func (e *Engine[S, E]) UnseenExternalEvents() uint32 {
	return e.unseenExternalEvents
}

// LastUnseenEvent reports the most recent event UnseenExternalEvents counted,
// and whether there is one.
//
// A count says an event went unlooked-at; this says which one, which is what a
// host debugging a supervisor that stopped answering actually needs.
func (e *Engine[S, E]) LastUnseenEvent() (E, bool) {
	return e.lastUnseenEvent, e.hasUnseen
}

// UnhandledErrorEvents reports how many error.* events this engine raised that
// no transition in any active state answered.
//
// §scxml-3.12.2 requires the processor to signal its own failures as error.*
// events on the internal queue, and says in the same breath that "they are
// ignored if no transition is found that matches them". Being ignored is the
// clause. Being unable to say it happened is not, and the difference matters to
// exactly one party: the host, which did not write the document, cannot see the
// failure anywhere in the configuration, and is the only one positioned to do
// something about it. A supervisor driving a machine whose <assign> silently
// fails every round reads IsRunning() == true and a plausible state forever.
//
// This is the sibling of DiscardedExternalEvents, and the two are deliberately
// separate counts rather than one. That one stops at the external queue because
// an author's unmatched <raise> has both ends inside the document; an error
// event's sender is the engine, so the same reasoning does not reach it. An
// author's <raise> that matches nothing is still not counted here.
//
// An error the document did answer is not counted either — the document dealt
// with it, and its handling is visible in the configuration the host can
// already read. What this counts is only the silent case.
//
// The C++ Interpreter has answered this all along, through
// getLastStateMachineError() and the message it raises error.execution with;
// this is the generated engines' side of it.
func (e *Engine[S, E]) UnhandledErrorEvents() uint32 {
	return e.unhandledErrorEvents
}

// LastUnhandledError reports the most recent error event UnhandledErrorEvents
// counted. The bool is false while that count is zero — the zero value of E is
// a real event, so it cannot stand in for "none".
//
// Which error it was narrows a silent failure from "something in this machine is
// broken" to a class: error.execution is the document's own executable content
// failing, error.communication is a <send> or <invoke> that could not reach its
// target — two different repairs, and a count alone separates neither.
func (e *Engine[S, E]) LastUnhandledError() (E, bool) {
	return e.lastUnhandledError, e.hasUnhandledError
}

// ErrorCascadeEvents reports how many error.* events this engine refused to
// queue because the error handler that raised them had been failing for
// maxErrorCascadeDepth links running.
//
// §scxml-3.12.2 says an unmatched error event is ignored, and
// UnhandledErrorEvents is that case. This is its opposite and its worse half:
// the document does match the error, and the handler fails the same way every
// time. The failure raises error.execution, the same transition answers it,
// and the drain never empties. Nothing in the clause covers it — it bounds
// what happens to an error nobody wants, not an error everybody wants and
// nobody can handle.
//
// Left to run, that is not a hang: it is a core at 100% forever. Measured
// 2026-08-19 on a two-line document, the Python engine turned 37,000 links a
// second while its configuration never moved and IsRunning() stayed true — the
// exact reading an unattended supervisor takes as healthy. So the engine stops
// feeding the chain and says how often it had to, which is the one fact that
// separates "this machine is idle" from "this machine's error handling is
// broken".
//
// A document that fails five hundred times cleanly counts zero here: the chain
// is measured from handler to handler, not from failure to failure, and any
// other internal event resets it. Nothing is discarded that a working document
// would have processed.
func (e *Engine[S, E]) ErrorCascadeEvents() uint32 {
	return e.errorCascadeEvents
}

// LastErrorCascadeEvent reports the most recent error event ErrorCascadeEvents
// refused. The bool is false while that count is zero — the zero value of E is
// a real event, so it cannot stand in for "none".
//
// Which error it was names the repair: error.execution is a handler whose own
// executable content fails, error.communication one that answers an
// unreachable target by talking to it again.
func (e *Engine[S, E]) LastErrorCascadeEvent() (E, bool) {
	return e.lastErrorCascadeEvent, e.hasErrorCascadeEvent
}

// TruncatedMacrosteps reports how many macrosteps this engine stopped short
// because their chain was still going after maxMacrostepMicrosteps microsteps.
//
// The specification says a macrostep ends in a configuration where nothing is
// enabled by NULL and no internal event is left, and its Principles and
// Constraints add that a macrostep may not terminate and that this "is
// currently allowed". A document with a cyclic eventless transition is
// therefore not malformed, and neither is one whose <raise> answers itself;
// both are documents whose macrostep is infinite, and an engine that runs
// either to the letter never returns.
//
// Both are counted here, because they are the same fact to a host: the
// macrostep it just drove did not reach a stable configuration. Which chain it
// was is what LastTruncatedMacrostepState points at.
//
// This engine does not run it to the letter. It stops, and this count is how a
// host learns that it did — because every other reading says the opposite:
// CurrentState answers, IsRunning is true, and the call returned in
// microseconds. The configuration behind those answers is not the stable one
// the clause promises; it is wherever the hundredth microstep happened to
// land, and the document has more to do that this engine will not do.
//
// A document whose chain is a hundred microsteps long and then settles counts
// zero: the ceiling is on microsteps taken, and the macrostep is only counted
// here when the loop still had work after them — a transition enabled by NULL,
// or an event left on the internal queue. Long chains are ordinary; endless
// ones are not.
func (e *Engine[S, E]) TruncatedMacrosteps() uint32 {
	return e.truncatedMacrosteps
}

// LastTruncatedMacrostepState reports the state this engine was in when it
// last stopped a macrostep that way. The bool is false while
// TruncatedMacrosteps is zero — the zero value of S is a real state, so it
// cannot stand in for "none".
//
// Which state it was is the whole repair: an endless chain is a closed walk
// through the state graph, and this names one state on it — the source of the
// transition that was refused, or the state the drain was standing in when it
// stopped taking internal events. The count alone says a document somewhere
// cannot settle; this says where to look.
func (e *Engine[S, E]) LastTruncatedMacrostepState() (S, bool) {
	return e.lastTruncatedMacrostepState, e.hasTruncatedMacrostep
}

// ================================================================
// Callbacks
// ================================================================

// SetCompletionCallback registers a callback invoked when the engine reaches a
// final state (§scxml-6.4).
func (e *Engine[S, E]) SetCompletionCallback(callback func()) {
	e.completionCallback = callback
}

// SetHTTPSendCallback registers an HTTP send dispatcher callback (§scxml-C-2).
//
// The callback returns *HttpSendResponse when real HTTP is used. The engine
// injects the response event into the external queue. Return nil for
// fire-and-forget sends.
func (e *Engine[S, E]) SetHTTPSendCallback(callback func(HttpSendRequest) *HttpSendResponse) {
	e.onHTTPSend = callback
}

// PerformHTTPSend dispatches a BasicHTTP send through the registered callback
// (§scxml-C-2).
//
// The callback is the sole dispatch mechanism. If it returns non-nil
// HttpSendResponse, the engine injects the response event into the external
// queue. The engine has no knowledge of HTTP transport — callers supply the
// implementation via SetHTTPSendCallback.
func (e *Engine[S, E]) PerformHTTPSend(target, eventName, content string, params map[string][]string, sendID string) {
	if e.onHTTPSend == nil {
		return
	}
	resp := e.onHTTPSend(HttpSendRequest{
		Target:    target,
		EventName: eventName,
		Content:   content,
		Params:    params,
		SendID:    sendID,
	})
	if resp != nil {
		if evt, ok := e.policy.GetEventFromName(resp.EventName); ok {
			meta := NewEventWithMetadata(evt)
			meta.Metadata = ExternalMetadata("", "")
			meta.Metadata.Data = resp.EventData
			e.externalQueue.Raise(meta)
		}
	}
}

// RunUntilCompletion runs the state machine to completion or timeout
// (§scxml-6.2).
//
// Calls Tick() in a loop until either the final state is reached or timeout
// elapses. Returns true on completion, false on timeout.
//
// pollInterval is a ceiling on the wait between ticks, not the interval
// actually slept: a nearer scheduler deadline (TimeUntilNextScheduled)
// shortens it, so a caller passing an interval coarser than the document's
// delays no longer steps over them.
//
// Matches Rust Engine::run_until_completion.
func (e *Engine[S, E]) RunUntilCompletion(timeout, pollInterval time.Duration) bool {
	// W3C SCXML: if already stopped but reached final state during initialize(), return true
	if !e.isRunning {
		return e.isInFinalState()
	}

	start := time.Now()
	for !e.isInFinalState() {
		if time.Since(start) > timeout {
			return false
		}
		// The scheduler's own answer wins whenever it is nearer: sleeping past
		// a deadline is what turns a coarse interval into a document that
		// behaves differently, and waking on it costs nothing extra.
		wait := pollInterval
		if next, ok := e.TimeUntilNextScheduled(); ok && next < wait {
			wait = next
		}
		time.Sleep(wait)
		e.Tick()
	}
	return true
}

// ================================================================
// Internal: microstep + macrostep implementation
// ================================================================

// runMainEventLoop is the W3C SCXML Appendix D outer loop, and the only place
// the three exported entry points express macrostep semantics.
//
// Appendix D names the external queue exactly once per iteration and it is
// after invoke(inv):
//
//	while running:
//	    while running and not macrostepDone:      # eventless + internal only
//	        ... selectEventlessTransitions() / internalQueue.dequeue() ...
//	    for state in statesToInvoke.sort(entryOrder):
//	        for inv in state.invoke.sort(documentOrder):
//	            invoke(inv)
//	    statesToInvoke.clear()
//	    if not internalQueue.isEmpty(): continue
//	    externalEvent = externalQueue.dequeue()
//
// Folding the external drain into the macrostep-completion loop instead is a
// different algorithm, not a shorter one. The invoked children do not exist
// yet while that drain runs, so everything <onentry> queued for this session
// on the way in is consumed with no autoforward child to receive it — and
// there is no later point at which it is delivered. One external event per
// iteration for the same reason: a state entered by event N's transition must
// have its invokes started before N+1 comes off the queue.
//
// Matches Rust Engine::run_main_event_loop.
func (e *Engine[S, E]) runMainEventLoop() {
	for {
		// W3C SCXML Appendix D: complete the macrostep on eventless
		// transitions and internal events alone.
		for {
			e.checkEventlessTransitions()
			if !e.internalQueue.HasEvents() {
				break
			}
			e.processInternalQueue()
			if e.macrostepTruncated {
				// Either branch may have spent the last of the budget. Without
				// this the loop turns forever on a chain that is no longer
				// being drained: the queue stays non-empty precisely because
				// the drain refused it.
				break
			}
		}

		if !e.isRunning || e.isInFinalState() {
			// W3C SCXML Appendix D's main event loop ends here, and whatever the host put on
			// the external queue ends with it. That is the clause: a machine
			// that reached a top-level final state has exited the interpreter.
			// Saying nothing about it is not the clause — see
			// UnseenExternalEvents.
			e.recordUnseenExternalEvents()
			break
		}

		// §scxml-6.4: invokes for states entered during this macrostep.
		if e.policy.HasInvokeSupport() {
			e.policy.ExecutePendingInvokes(e)
		}

		// W3C SCXML Appendix D: invoking may have raised internal error events
		// (and a child that completed synchronously may already have raised
		// done.invoke); handle them before touching the external queue.
		//
		// Not when this macrostep was already stopped at the ceiling: the queue
		// is non-empty because the drain refused it, so looping back is a spin
		// that takes no microstep, logs nothing, and never ends. Falling
		// through to the external dequeue instead is what keeps a machine
		// inside an endless chain reachable at all — the event that rescues it
		// is on that queue, and the clause's priority would otherwise hold it
		// behind a chain that never ends.
		if !e.macrostepTruncated && e.internalQueue.HasEvents() {
			continue
		}

		if !e.processNextExternalEvent() {
			break
		}
	}
}

// processInternalQueue drains the internal queue (§scxml-C-1, high priority).
//
// Bounded by the same macrostep budget the eventless branch spends, and for the
// same reason: a <raise> answered by a transition that raises again is a
// macrostep that never ends, exactly as a cyclic eventless transition is. Until
// 2026-08-20 this branch had no ceiling in any of the seven engines here, so
// that document did not return at all.
//
// Matches Rust Engine::process_internal_queue.
func (e *Engine[S, E]) processInternalQueue() {
	if e.macrostepTruncated {
		// The eventless branch of this same macrostep already ran out of
		// budget. Draining now would hand the chain a second one.
		return
	}
	log.Printf("[sce] Engine::processInternalQueue: starting internal queue drain")

	for e.internalQueue.HasEvents() {
		if e.macrostepMicrostepsTaken == maxMacrostepMicrosteps {
			// Work is still queued one microstep past the budget, so this is
			// the case the specification calls a macrostep that cannot end.
			// Refuse the microstep rather than take it: the event stays on the
			// queue, which is where the next macrostep will find it, and the
			// count says the configuration a host reads now is not a stable one.
			e.recordTruncatedMacrostep(e.currentState)
			log.Printf("[sce] Engine::processInternalQueue: macrostep still going after %d microsteps; stopped",
				maxMacrostepMicrosteps)
			return
		}
		eventWithMeta, ok := e.internalQueue.Pop()
		if !ok {
			break
		}
		// §scxml-5.4.1: Stop if top-level final state reached. Same
		// predicate as everything else that means "the machine is done" —
		// spelling the parent check out a second time here is what let the
		// exported one drift away from it.
		if e.isInFinalState() {
			log.Printf("[sce] Engine::processInternalQueue: top-level final state reached, stopping")
			return
		}
		// §scxml-5.10: Populate policy metadata from event
		e.policy.PopulateEventMetadata(&eventWithMeta.Metadata)
		// §scxml-3.12.2: the processor raises error.* into this queue and the
		// clause says they "are ignored if no transition is found that matches
		// them". Ignoring them is the clause; staying silent about it is not.
		// DiscardedExternalEvents deliberately stops at the external queue
		// because an unmatched <raise> has both ends inside the document — but
		// the sender of an error event is this engine, so that reasoning does
		// not reach it. The host never wrote the document, cannot see the
		// failure in the configuration, and is the only party able to act on it.
		//
		// The selection runs first and unconditionally: it is what processes
		// every internal event, and folding it into the condition below would
		// skip it for everything that is not an error.
		// An error raised from here on is raised by an error handler, which is
		// the one situation the engine cannot leave to the document: the
		// handler that failed is the same one that will answer the failure.
		// The flag is what Raise reads to tell that apart from a first
		// failure, and it is cleared before anything else can run so a chain
		// cannot be attributed to the wrong event.
		isError := IsErrorEvent(e.policy.GetEventName(eventWithMeta.Event))
		// The chain is not ended by the drain doing something else. An earlier
		// draft reset the depth on every non-error event, which reads as the
		// careful choice and is the opposite: a handler that raises its own
		// event before failing — a document that logs, then fails, which is
		// most of them — leaves the queue alternating tick, error, tick,
		// error…, and each tick put the ceiling back out of reach. The count
		// needs no such guard, because it only ever rises while an error
		// handler is running.
		e.handlingErrorEvent = isError
		outcome := e.executeTransition(eventWithMeta.Event)
		e.handlingErrorEvent = false
		if outcome.selected {
			// Appendix D: the loop turn that selects nothing takes no
			// microstep, so it spends no budget. Only a turn that answered the
			// event moved the machine, and only those are what a ceiling on
			// microsteps can be counted in.
			e.macrostepMicrostepsTaken++
		}
		if !outcome.selected && isError {
			e.unhandledErrorEvents++
			e.lastUnhandledError = eventWithMeta.Event
			e.hasUnhandledError = true
			log.Printf("[sce] Engine::processInternalQueue: error event matched no transition; unhandled")
		}
		e.policy.ClearEventMetadata()
	}
	// The queue emptied, so the chain — refused or merely finished — is over.
	// A machine whose next macrostep starts a new one starts it from zero, and
	// the count of what was refused stays where the host reads it.
	e.errorCascadeDepth = 0
}

// processNextExternalEvent takes exactly one event off the external queue, runs
// the preliminary <finalize> / autoforward step against it, then selects
// transitions. Reports whether an event was processed.
//
// One event, not a drain: Appendix D returns to the top of the outer loop after
// each external event, so a state entered by this event's transition gets its
// invokes started before the next one is dequeued.
//
// Matches Rust Engine::process_next_external_event.
func (e *Engine[S, E]) processNextExternalEvent() bool {
	eventWithMeta, ok := e.externalQueue.Pop()
	if !ok {
		return false
	}
	// Taking an event off the external queue is where
	// a macrostep begins, so it is where the previous one's ceiling stops
	// applying. A machine left inside an endless chain gets a full budget
	// for each event it is given, and each refusal is counted separately —
	// which is what tells a host that spins once from one that spins on
	// everything.
	//
	// Here and not at the entry to the loop above, which reads like the more
	// general boundary and is not one: a machine whose chain was refused would
	// spend a whole budget re-walking it before it ever looked at the event the
	// host sent to get it out. The refused events stay queued either way — this
	// is where the budget that drains them comes back.
	e.macrostepTruncated = false
	e.macrostepMicrostepsTaken = 0
	{
		// §scxml-6.5: Execute finalize before parent's own transition matching
		if e.policy.HasFinalize() {
			e.policy.ExecuteFinalizeForChildEvent(&eventWithMeta, e)
		}
		// W3C SCXML Appendix D mainEventLoop: autoforward belongs to the same
		// preliminary step as <finalize> above — both run against the event
		// this drain has just popped off the external queue, and both run
		// before transition selection. §scxml-6.4.2 fixes the position in
		// prose as well: the parent forwards "at the point at which it removes
		// it from the external event queue".
		//
		// Forwarding where the event is enqueued instead is a different
		// algorithm, not an earlier one. RaiseExternalWithMeta runs inside
		// whatever executable content produced the event, so a transition body
		// that queues two events hands the child both of them before the
		// parent has processed either — the child runs a whole event ahead and
		// the two sessions stop agreeing on what has happened.
		// Run-to-completion is a property of this loop's shape, so the forward
		// has to live in the loop.
		//
		// §scxml-6.4 mandates an exact copy, so the source event's metadata
		// rides along with the name. Target is not forwarded: it is a routing
		// decision owned by the originating <send>, and inheriting it would
		// re-route the child's copy.
		if e.policy.HasAutoforward() {
			name := e.policy.GetEventName(eventWithMeta.Event)
			e.policy.ForwardToAutoforwardChildren(name, eventWithMeta.Metadata, e)
		}
		// §scxml-5.10: Populate policy metadata from event
		e.policy.PopulateEventMetadata(&eventWithMeta.Metadata)
		// §scxml-3.1.2: "If no transition matches in any state, the event is
		// discarded." Discarding it is the rule; being unable to say so is not
		// part of the rule. The host that put this event on the queue is the
		// one party that cannot see the outcome -- a discard leaves the
		// configuration exactly as a self transition does -- and it is the
		// party that got the event wrong. Recorded for the external queue
		// only: an internal <raise> that matches nothing is the document's own
		// business, and both ends of it are in the document.
		if !e.executeTransition(eventWithMeta.Event).selected {
			e.discardedExternalEvents++
			e.lastDiscardedEvent = eventWithMeta.Event
			e.hasDiscarded = true
		}
		e.policy.ClearEventMetadata()
	}
	return true
}

// recordTruncatedMacrostep publishes a macrostep this engine stopped short,
// from whichever branch of Appendix D's inner loop ran out of budget.
//
// One function, two callers, for the reason the budget is one number: a host
// reads a macrostep that did not reach a stable configuration, and the branch
// it died in is a detail of the document, not of the contract. Two copies of
// this would be two chances for one of them to stop setting the flag that keeps
// the same chain from being handed a second budget.
func (e *Engine[S, E]) recordTruncatedMacrostep(state S) {
	e.truncatedMacrosteps++
	e.lastTruncatedMacrostepState = state
	e.hasTruncatedMacrostep = true
	e.macrostepTruncated = true
}

// maxMacrostepMicrosteps is how many microsteps one macrostep may take before
// this engine stops taking them — see TruncatedMacrosteps.
//
// The specification defines a macrostep as a chain of microsteps ending in a
// configuration where nothing is enabled by NULL and the internal queue is
// empty, and its Principles and Constraints say in as many words that such a
// chain need not exist: "A microstep always terminates. A macrostep may not. A
// macrostep that does not terminate may be said to consist of an infinitely
// long sequence of microsteps. This is currently allowed."
//
// So the ceiling is not conformance — it is this engine declining a document
// the specification permits, which is exactly why the decline has to be
// visible.
//
// One budget for the whole inner loop, not one per branch. Appendix D's loop
// takes a microstep on an eventless transition or on an internal event, and a
// document alternating the two is one chain, not two: budgeting the branches
// separately leaves that chain unbounded, which is what a per-call counter on
// the eventless branch alone did here until 2026-08-20.
//
// Ten times maxErrorCascadeDepth, and deliberately not equal to it. This is the
// backstop; the cascade ceiling is a diagnostic that names the error a handler
// keeps failing on, and a backstop that fires first makes that diagnostic
// unreachable. Measured 2026-08-20: with both at a hundred, a handler that
// raises one event of its own per link — two microsteps a link, which is what a
// document that logs before it fails looks like — was cut at fifty links by
// this ceiling and ErrorCascadeEvents never moved. The factor of ten is the
// headroom that keeps the specific report reachable for a handler raising up to
// eight events a link; a busier one is cut here instead, which is coarser but
// still reported.
const maxMacrostepMicrosteps = 1000

// checkEventlessTransitions checks and executes eventless transitions until
// stable (§scxml-3.13).
//
// Bounded at maxMacrostepMicrosteps microsteps and, when the chain is still
// going at that point, reported through TruncatedMacrosteps — the ceiling is a
// departure from a document the specification allows, so it is not a silent
// one. The budget is the macrostep's, not this call's: see
// maxMacrostepMicrosteps. Ported from Rust
// Engine::check_eventless_transitions.
func (e *Engine[S, E]) checkEventlessTransitions() {
	if e.macrostepTruncated {
		// This macrostep was already stopped at the ceiling. Re-entering the
		// drain would hand the same chain a second budget, which is the
		// runaway the ceiling exists to refuse.
		return
	}
	nullEvent := e.policy.NullEvent()
	// Microsteps taken, not loop turns: the turn that finds nothing enabled is
	// how a macrostep ends, and counting it would spend the budget on the
	// proof that no budget was needed. The count lives on the engine because
	// the macrostep does — see macrostepMicrostepsTaken.

	for {
		oldState := e.currentState
		preTransitionStates := e.GetActiveStates()
		newState := e.currentState

		tookTransition := e.policy.ProcessTransition(&newState, nullEvent, e)
		if !tookTransition {
			// §scxml-3.13: nothing is enabled by NULL — the macrostep reached
			// the stable configuration the clause describes, and nothing was
			// refused however long the chain was.
			break
		}

		if e.macrostepMicrostepsTaken == maxMacrostepMicrosteps {
			// The chain is still going one microstep past the budget, so this
			// is the case the specification calls a macrostep that cannot end.
			// Refuse the microstep rather than take it, and publish the
			// refusal: the configuration left behind is not a stable one and
			// only this counter says so.
			e.recordTruncatedMacrostep(oldState)
			log.Printf("[sce] Engine::checkEventlessTransitions: macrostep still going after %d microsteps; stopped",
				maxMacrostepMicrosteps)
			break
		}
		e.macrostepMicrostepsTaken++

		e.currentState = newState
		needsHierarchical := (oldState != newState) || !e.policy.LastTransitionIsTargetless()

		if !needsHierarchical {
			// Targetless transition -- execute actions only
			e.policy.ExecuteTransitionActions(e)
			continue
		}

		// Hierarchical exit/entry
		// For parallel state machines, process_transition already performed a full
		// microstep. Calling handleHierarchicalTransition again would double-run
		// onexit/onentry.
		if !e.policy.HasParallelStates() {
			e.handleHierarchicalTransition(oldState, newState, preTransitionStates)
		} else {
			e.resolveCurrentStateToLeaf()
		}

		// Check for final state
		if e.isInFinalState() {
			break
		}
	}
}

// executeTransition dispatches a single transition (§scxml-3.12 / §scxml-3.13).
//
// Calls ProcessTransition on the policy; if it returns true, performs the
// hierarchical exit/entry dance via handleHierarchicalTransition.
// Matches Rust Engine::execute_transition.
func (e *Engine[S, E]) executeTransition(event E) eventOutcome {
	oldState := e.currentState
	preTransitionStates := e.GetActiveStates()
	newState := e.currentState

	tookTransition := e.policy.ProcessTransition(&newState, event, e)
	if !tookTransition {
		return eventOutcome{}
	}

	e.currentState = newState
	isSelfTransition := oldState == newState
	needsHierarchical := (oldState != newState) ||
		(isSelfTransition && !e.policy.LastTransitionIsTargetless())

	if !needsHierarchical {
		// §scxml-3.4: targetless transition -- execute actions only
		e.policy.ExecuteTransitionActions(e)
		return eventOutcome{selected: true}
	}

	// §scxml-3.12: Hierarchical exit/entry
	//
	// For parallel state machines the generated process_transition already called
	// execute_microstep internally. Calling handleHierarchicalTransition again
	// would double-run onexit/onentry actions.
	if !e.policy.HasParallelStates() {
		e.handleHierarchicalTransition(oldState, newState, preTransitionStates)
	} else {
		// §scxml-3.3: Still resolve the currentState leaf
		e.resolveCurrentStateToLeaf()
	}
	e.checkEventlessTransitions()
	return eventOutcome{selected: true, configurationChanged: true}
}

// handleHierarchicalTransition executes hierarchical exit/entry between two
// states (§scxml-3.12 / §scxml-3.13).
//
// 1:1 port of Rust Engine::handle_hierarchical_transition. Handles:
//   - Internal vs external transition LCA calculation (W3C 5.9.2)
//   - Active descendant exit before source exit (W3C 3.13)
//   - Exit chain to LCA
//   - Ancestor/self transition target re-entry (W3C 3.10, test 579)
//   - Transition action execution between exit and entry
//   - Entry chain from LCA to new state
//   - No-LCA top-level case
func (e *Engine[S, E]) handleHierarchicalTransition(oldState, newState S, preTransitionStates []S) {
	log.Printf("[sce] Engine::handleHierarchicalTransition: %v -> %v", oldState, newState)

	// §scxml-5.9.2: Determine LCA based on transition type
	var lca S
	var hasLCA bool

	if e.policy.LastTransitionIsInternal() {
		isSelfTransition := oldState == newState
		isProperDescendant := !isSelfTransition && e.policy.IsDescendantOf(newState, oldState)
		isSourceCompound := e.policy.IsCompoundState(oldState)

		if isProperDescendant && isSourceCompound {
			// §scxml-3.13: Internal to proper descendant in compound -- source is LCA
			lca = oldState
			hasLCA = true
		} else {
			// W3C 3.13/5.9.2: Non-compound source or non-descendant -- behaves as external
			lca, hasLCA = FindLCA[S, E](e.policy, oldState, newState)
		}
	} else {
		lca, hasLCA = FindLCA[S, E](e.policy, oldState, newState)
	}

	if hasLCA {
		lcaState := lca

		// §scxml-3.13: Exit active descendants of oldState deepest first
		descendantsToExit := make([]S, 0, 4)
		for _, s := range preTransitionStates {
			if s != oldState && e.policy.IsDescendantOf(s, oldState) {
				descendantsToExit = append(descendantsToExit, s)
			}
		}
		// Sort by document order descending (deeper first)
		sort.Slice(descendantsToExit, func(i, j int) bool {
			return e.policy.GetDocumentOrder(descendantsToExit[i]) > e.policy.GetDocumentOrder(descendantsToExit[j])
		})

		for _, descendant := range descendantsToExit {
			log.Printf("[sce] handleHierarchicalTransition: exit descendant %v", descendant)
			e.policy.ExecuteExitActions(descendant, e, preTransitionStates)
		}

		// §scxml-3.13: Exit from oldState up to (not including) LCA
		exitChain := BuildExitChain[S, E](e.policy, oldState, lcaState)
		for _, state := range exitChain {
			log.Printf("[sce] handleHierarchicalTransition: exit %v", state)
			e.policy.ExecuteExitActions(state, e, preTransitionStates)
		}

		// §scxml-3.10 (test 579): Ancestor/self transition -- exit and re-enter target
		isTargetActive := false
		for _, s := range preTransitionStates {
			if s == newState {
				isTargetActive = true
				break
			}
		}
		if newState == lcaState && isTargetActive {
			log.Printf("[sce] handleHierarchicalTransition: ancestor/self transition -- exit target %v", newState)
			e.policy.ExecuteExitActions(newState, e, preTransitionStates)
		}

		// §scxml-3.13: Execute transition actions between exit and entry
		e.policy.ExecuteTransitionActions(e)

		// §scxml-3.13: Enter from LCA down to newState
		var entryChain []S
		if newState == lcaState {
			// Ancestor/self case -- enter full subtree from target
			full := BuildEntryChain[S, E](e.policy, newState)
			entryChain = make([]S, 0, len(full))
			for _, s := range full {
				if s == lcaState || e.policy.IsDescendantOf(s, lcaState) {
					entryChain = append(entryChain, s)
				}
			}
		} else {
			entryChain = BuildEntryChainFromAncestor[S, E](e.policy, newState, lcaState)
		}

		e.executeEntryChain(entryChain)

		if len(entryChain) > 0 {
			e.currentState = entryChain[len(entryChain)-1]
		}

		// §scxml-3.3: Resolve currentState to the deepest initial leaf.
		e.resolveCurrentStateToLeaf()
	} else {
		// No LCA -- top-level transition, exit all ancestors of oldState
		log.Printf("[sce] handleHierarchicalTransition: no LCA (top-level)")

		current := oldState
		hasMore := true
		for hasMore {
			log.Printf("[sce] handleHierarchicalTransition: exit to root: %v", current)
			e.policy.ExecuteExitActions(current, e, preTransitionStates)
			parent, ok := e.policy.GetParent(current)
			if !ok {
				hasMore = false
			} else {
				current = parent
			}
		}

		e.policy.ExecuteTransitionActions(e)

		entryChain := BuildEntryChain[S, E](e.policy, newState)
		e.executeEntryChain(entryChain)

		if len(entryChain) > 0 {
			e.currentState = entryChain[len(entryChain)-1]
		}

		e.resolveCurrentStateToLeaf()
	}
}

// executeEntryChain enters a whole root-to-target chain, giving every link but
// the last the next one as its pathChild (§scxml-D-addAncestorStatesToEnter).
//
// One place, because all three entry-chain walks in this engine owe the same
// rule and a chain walked with nil throughout puts two children of one compound
// state in the configuration.
func (e *Engine[S, E]) executeEntryChain(entryChain []S) {
	for i := range entryChain {
		var pathChild *S
		if i+1 < len(entryChain) {
			pathChild = &entryChain[i+1]
		}
		log.Printf("[sce] executeEntryChain: enter %v", entryChain[i])
		e.policy.ExecuteEntryActions(entryChain[i], e, pathChild)
	}
}

// resolveCurrentStateToLeaf walks currentState down through initial children
// to the leaf (§scxml-3.3).
//
// For non-parallel SMs: descends into the compound's initial child, calling
// ExecuteEntryActions for each level, until it reaches an atomic leaf.
//
// For parallel SMs: the generated ExecuteEntryActions already recurses, so this
// is just a pointer walk without entry.
//
// Matches Rust Engine::resolve_current_state_to_leaf.
func (e *Engine[S, E]) resolveCurrentStateToLeaf() {
	const maxDepth = 50
	for i := 0; i < maxDepth; i++ {
		if !e.policy.IsCompoundState(e.currentState) {
			break
		}
		child := e.policy.GetInitialOrHistoryChild(e.currentState)
		if child == e.currentState {
			break // No child to descend into
		}
		e.currentState = child
		if !e.policy.HasParallelStates() {
			// Non-parallel: template doesn't recurse, so we enter here. This
			// child IS the entry target of its own descent, so it takes its
			// defaults — nil, not a pathChild.
			e.policy.ExecuteEntryActions(child, e, nil)
		}
	}
}
