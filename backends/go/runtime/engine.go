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

	// scheduler is the §scxml-6.2 delayed event scheduler.
	scheduler *PullScheduler[E]

	// discardedExternalEvents counts events taken off the external queue that
	// no transition matched (§scxml-3.1.2) — see DiscardedExternalEvents.
	discardedExternalEvents uint32

	// lastDiscardedEvent is the most recent event counted above; hasDiscarded
	// says whether there is one, because the zero value of E is a real event.
	lastDiscardedEvent E
	hasDiscarded       bool

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

// Step processes one macrostep: drain queues and run eventless transitions.
//
// Matches Rust Engine::step. Used by parent SMs to explicitly drive children
// after sending them events (§scxml-6.4).
func (e *Engine[S, E]) Step() {
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
	for {
		event, data, ok := e.scheduler.PopReadyEvent()
		if !ok {
			break
		}
		e.RaiseExternal(event, data, "")
		// The macrostep this event drives may <cancel> a later one, so the
		// next deadline is re-read after it rather than before.
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
func (e *Engine[S, E]) ScheduleEvent(event E, delay time.Duration, sendID, eventData string) string {
	return e.scheduler.ScheduleEvent(event, delay, sendID, eventData)
}

// CancelEvent cancels a previously scheduled event by send ID.
func (e *Engine[S, E]) CancelEvent(sendID string) bool {
	return e.scheduler.CancelEvent(sendID)
}

// HasReadyEvents returns whether the scheduler has events ready to fire.
func (e *Engine[S, E]) HasReadyEvents() bool {
	return e.scheduler.HasReadyEvents()
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
	next, ok := e.scheduler.NextReadyAt()
	if !ok {
		return 0, false
	}
	remaining := time.Until(next)
	if remaining < 0 {
		return 0, true
	}
	return remaining, true
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
		}

		if !e.isRunning || e.isInFinalState() {
			break
		}

		// §scxml-6.4: invokes for states entered during this macrostep.
		if e.policy.HasInvokeSupport() {
			e.policy.ExecutePendingInvokes(e)
		}

		// W3C SCXML Appendix D: invoking may have raised internal error events
		// (and a child that completed synchronously may already have raised
		// done.invoke); handle them before touching the external queue.
		if e.internalQueue.HasEvents() {
			continue
		}

		if !e.processNextExternalEvent() {
			break
		}
	}
}

// processInternalQueue drains the internal queue (§scxml-C-1, high priority).
//
// Matches Rust Engine::process_internal_queue.
func (e *Engine[S, E]) processInternalQueue() {
	log.Printf("[sce] Engine::processInternalQueue: starting internal queue drain")

	for {
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
		if !isError {
			// The drain did something else, so whatever chain was building is
			// over. Counting links across an unrelated internal event would
			// report a document that merely fails often as one that cannot
			// stop failing.
			e.errorCascadeDepth = 0
		}
		e.handlingErrorEvent = isError
		outcome := e.executeTransition(eventWithMeta.Event)
		e.handlingErrorEvent = false
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

// checkEventlessTransitions checks and executes eventless transitions until
// stable (§scxml-3.13).
//
// Uses bounded iteration to prevent infinite loops from cyclic eventless chains.
// Ported from Rust Engine::check_eventless_transitions.
func (e *Engine[S, E]) checkEventlessTransitions() {
	const maxIterations = 100
	nullEvent := e.policy.NullEvent()

	for iteration := 0; iteration < maxIterations; iteration++ {
		oldState := e.currentState
		preTransitionStates := e.GetActiveStates()
		newState := e.currentState

		tookTransition := e.policy.ProcessTransition(&newState, nullEvent, e)
		if !tookTransition {
			break
		}

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

		if iteration == maxIterations-1 {
			log.Printf("[sce] Engine::checkEventlessTransitions: max iterations reached (%d)", maxIterations)
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
