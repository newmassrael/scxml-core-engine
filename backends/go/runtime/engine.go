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
// Engine is NOT safe for concurrent use. Callers needing multi-goroutine access
// must protect with sync.Mutex. This matches the C++ and Rust single-threaded
// microstep loop design.
type Engine[S comparable, E comparable] struct {
	// policy is the generated per-SM policy struct (datamodel, last-transition flags, etc.).
	policy StatePolicy[S, E]

	// currentState is the currently active state (or deepest active state for parallel machines).
	currentState S

	// internalQueue is the W3C SCXML C.1 internal event queue (high priority).
	internalQueue *EventQueueManager[EventWithMetadata[E]]

	// externalQueue is the W3C SCXML C.1 external event queue (low priority).
	externalQueue *EventQueueManager[EventWithMetadata[E]]

	// isRunning tracks whether the engine is currently running.
	isRunning bool

	// completionCallback is the W3C SCXML 6.4 callback invoked when reaching a final state.
	completionCallback func()

	// onHTTPSend is the W3C SCXML C.2 HTTP send dispatch callback.
	// Returns *HttpSendResponse when real HTTP is used; nil for fire-and-forget.
	onHTTPSend func(HttpSendRequest) *HttpSendResponse

	// scheduler is the W3C SCXML 6.2 delayed event scheduler.
	scheduler *PullScheduler[E]

	// donedataAtFinal is the W3C SCXML 5.5 + 6.3.1 stashed donedata payload
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
// Matches Rust Engine::initialize. W3C SCXML 5.3 guarantees datamodel
// initialization happens before any state entry.
func (e *Engine[S, E]) Initialize() {
	e.isRunning = true

	// W3C SCXML 5.3: Initialize datamodel before any state entry
	if e.policy.NeedsDataModelInit() {
		e.policy.InitializeDataModel(e)
	}

	// W3C SCXML 3.3: Entry chain from root to initial leaf
	entryChain := BuildEntryChain[S, E](e.policy, e.currentState)
	for _, state := range entryChain {
		e.policy.ExecuteEntryActions(state, e)
	}

	// W3C SCXML 3.3: Resolve currentState to the deepest initial leaf
	e.resolveCurrentStateToLeaf()

	// W3C SCXML C.1: Macrostep completion loop -- drain internal + eventless
	// until a stable configuration is reached.
	log.Printf("[sce] Engine::Initialize: entering macrostep completion loop")
	for {
		e.checkEventlessTransitions()
		if !e.internalQueue.HasEvents() && !e.externalQueue.HasEvents() {
			break
		}
		e.processEventQueues()
	}
	log.Printf("[sce] Engine::Initialize: macrostep completion loop finished")

	// W3C SCXML 6.4: Execute pending invokes once stable
	if e.policy.HasInvokeSupport() {
		e.policy.ExecutePendingInvokes(e)
		// Process done.invoke events raised by immediately-completed children
		log.Printf("[sce] Engine::Initialize: processing events raised by completed invokes")
		e.processEventQueues()
		e.checkEventlessTransitions()
	}

	// W3C SCXML 6.4: Fire completion callback if we reached a final state during init
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
// after sending them events (W3C SCXML 6.4).
func (e *Engine[S, E]) Step() {
	e.processEventQueues()
	e.checkEventlessTransitions()

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

	// W3C SCXML 6.2: Pop all ready scheduled events into the external queue
	for {
		event, data, ok := e.scheduler.PopReadyEvent()
		if !ok {
			break
		}
		e.RaiseExternal(event, data, "")
	}

	// W3C SCXML 6.4: Tick child state machines
	if e.policy.HasChildTick() {
		e.policy.TickChildren(e)
	}

	// Delegate to Step() for queue drain + completion callback
	e.Step()

	// W3C SCXML 6.4: Execute pending invokes after macrostep
	if e.policy.HasInvokeSupport() {
		e.policy.ExecutePendingInvokes(e)
	}
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

// GetActiveStates returns the full list of active states (W3C SCXML 3.11).
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

// isInFinalState returns whether the current state is a top-level final state
// (W3C SCXML 3.3).
func (e *Engine[S, E]) isInFinalState() bool {
	return e.policy.IsFinalState(e.currentState)
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

// Raise enqueues an internal event with full metadata (high priority)
// (W3C SCXML C.1).
//
// Matches Rust Engine::raise.
func (e *Engine[S, E]) Raise(event EventWithMetadata[E]) {
	e.internalQueue.Raise(event)
}

// RaiseExternal enqueues an external event with optional data and origin
// (W3C SCXML C.1 / 6.2).
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

	// W3C SCXML 5.10.1: Mark next event as external for _event.type
	if e.policy.HasExternalEventFlag() {
		e.policy.SetNextEventIsExternal(true)
	}
}

// RaiseExternalByName raises an external event by name (W3C SCXML 6.4.1, for
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

// RaiseExternalWithMeta raises an external event with full metadata (W3C SCXML
// 6.4.1, for child-to-parent events).
//
// Preserves invokeid for parent finalize handlers.
// Matches Rust Engine::raise_external_with_meta.
func (e *Engine[S, E]) RaiseExternalWithMeta(event EventWithMetadata[E]) {
	log.Printf("[sce] Engine::RaiseExternalWithMeta: enqueuing external event with metadata")

	// W3C SCXML 6.4.1: Autoforward
	if e.policy.HasAutoforward() {
		name := e.policy.GetEventName(event.Event)
		e.policy.ForwardToAutoforwardChildren(name, e)
	}

	e.externalQueue.Raise(event)

	if e.policy.HasExternalEventFlag() {
		e.policy.SetNextEventIsExternal(true)
	}
}

// ProcessEvent processes an external event (convenience API, runs one macrostep)
// (W3C SCXML 3.12).
//
// Matches Rust Engine::process_event.
func (e *Engine[S, E]) ProcessEvent(event E) {
	if !e.isRunning {
		return
	}
	e.RaiseExternal(event, "", "")
	e.Step()
}

// ProcessEventWithMeta processes an external event with metadata (W3C SCXML 5.10).
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
// Donedata stash (W3C SCXML 5.5 + 6.3.1)
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

// ================================================================
// Callbacks
// ================================================================

// SetCompletionCallback registers a callback invoked when the engine reaches a
// final state (W3C SCXML 6.4).
func (e *Engine[S, E]) SetCompletionCallback(callback func()) {
	e.completionCallback = callback
}

// SetHTTPSendCallback registers an HTTP send dispatcher callback (W3C SCXML C.2).
//
// The callback returns *HttpSendResponse when real HTTP is used. The engine
// injects the response event into the external queue. Return nil for
// fire-and-forget sends.
func (e *Engine[S, E]) SetHTTPSendCallback(callback func(HttpSendRequest) *HttpSendResponse) {
	e.onHTTPSend = callback
}

// PerformHTTPSend dispatches a BasicHTTP send through the registered callback
// (W3C SCXML C.2).
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
// (W3C SCXML 6.2).
//
// Polls the scheduler and calls Tick() in a loop until either the final state
// is reached or timeout elapses. Returns true on completion, false on timeout.
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
		time.Sleep(pollInterval)
		e.Tick()
	}
	return true
}

// ================================================================
// Internal: microstep + macrostep implementation
// ================================================================

// processEventQueues processes both internal and external queues (W3C SCXML D.1).
//
// Matches Rust Engine::process_event_queues.
func (e *Engine[S, E]) processEventQueues() {
	log.Printf("[sce] Engine::processEventQueues: starting internal queue drain")

	// W3C SCXML C.1: Internal queue first
	for {
		eventWithMeta, ok := e.internalQueue.Pop()
		if !ok {
			break
		}
		// W3C SCXML 5.4.1: Stop if top-level final state reached
		if e.policy.IsFinalState(e.currentState) {
			if _, hasParent := e.policy.GetParent(e.currentState); !hasParent {
				log.Printf("[sce] Engine::processEventQueues: top-level final state reached, stopping")
				return
			}
		}
		// W3C SCXML 5.10: Populate policy metadata from event
		e.policy.PopulateEventMetadata(&eventWithMeta.Metadata)
		e.executeTransition(eventWithMeta.Event)
		e.policy.ClearEventMetadata()
	}

	// W3C SCXML C.1: External queue second
	for {
		eventWithMeta, ok := e.externalQueue.Pop()
		if !ok {
			break
		}
		// W3C SCXML 6.5: Execute finalize before parent's own transition matching
		if e.policy.HasFinalize() {
			e.policy.ExecuteFinalizeForChildEvent(&eventWithMeta, e)
		}
		// W3C SCXML 5.10: Populate policy metadata from event
		e.policy.PopulateEventMetadata(&eventWithMeta.Metadata)
		e.executeTransition(eventWithMeta.Event)
		e.policy.ClearEventMetadata()
	}
}

// checkEventlessTransitions checks and executes eventless transitions until
// stable (W3C SCXML 3.13).
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

// executeTransition dispatches a single transition (W3C SCXML 3.12/3.13).
//
// Calls ProcessTransition on the policy; if it returns true, performs the
// hierarchical exit/entry dance via handleHierarchicalTransition.
// Matches Rust Engine::execute_transition.
func (e *Engine[S, E]) executeTransition(event E) bool {
	oldState := e.currentState
	preTransitionStates := e.GetActiveStates()
	newState := e.currentState

	tookTransition := e.policy.ProcessTransition(&newState, event, e)
	if !tookTransition {
		return false
	}

	e.currentState = newState
	isSelfTransition := oldState == newState
	needsHierarchical := (oldState != newState) ||
		(isSelfTransition && !e.policy.LastTransitionIsTargetless())

	if !needsHierarchical {
		// W3C SCXML 3.4: targetless transition -- execute actions only
		e.policy.ExecuteTransitionActions(e)
		return false
	}

	// W3C SCXML 3.12: Hierarchical exit/entry
	//
	// For parallel state machines the generated process_transition already called
	// execute_microstep internally. Calling handleHierarchicalTransition again
	// would double-run onexit/onentry actions.
	if !e.policy.HasParallelStates() {
		e.handleHierarchicalTransition(oldState, newState, preTransitionStates)
	} else {
		// W3C SCXML 3.3: Still resolve the currentState leaf
		e.resolveCurrentStateToLeaf()
	}
	e.checkEventlessTransitions()
	return true
}

// handleHierarchicalTransition executes hierarchical exit/entry between two
// states (W3C SCXML 3.12/3.13).
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

	// W3C SCXML 5.9.2: Determine LCA based on transition type
	var lca S
	var hasLCA bool

	if e.policy.LastTransitionIsInternal() {
		isSelfTransition := oldState == newState
		isProperDescendant := !isSelfTransition && e.policy.IsDescendantOf(newState, oldState)
		isSourceCompound := e.policy.IsCompoundState(oldState)

		if isProperDescendant && isSourceCompound {
			// W3C SCXML 3.13: Internal to proper descendant in compound -- source is LCA
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

		// W3C SCXML 3.13: Exit active descendants of oldState deepest first
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

		// W3C SCXML 3.13: Exit from oldState up to (not including) LCA
		exitChain := BuildExitChain[S, E](e.policy, oldState, lcaState)
		for _, state := range exitChain {
			log.Printf("[sce] handleHierarchicalTransition: exit %v", state)
			e.policy.ExecuteExitActions(state, e, preTransitionStates)
		}

		// W3C SCXML 3.10 (test 579): Ancestor/self transition -- exit and re-enter target
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

		// W3C SCXML 3.13: Execute transition actions between exit and entry
		e.policy.ExecuteTransitionActions(e)

		// W3C SCXML 3.13: Enter from LCA down to newState
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

		for _, state := range entryChain {
			log.Printf("[sce] handleHierarchicalTransition: enter %v", state)
			e.policy.ExecuteEntryActions(state, e)
		}

		if len(entryChain) > 0 {
			e.currentState = entryChain[len(entryChain)-1]
		}

		// W3C SCXML 3.3: Resolve currentState to the deepest initial leaf.
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
		for _, state := range entryChain {
			log.Printf("[sce] handleHierarchicalTransition: enter from root: %v", state)
			e.policy.ExecuteEntryActions(state, e)
		}

		if len(entryChain) > 0 {
			e.currentState = entryChain[len(entryChain)-1]
		}

		e.resolveCurrentStateToLeaf()
	}
}

// resolveCurrentStateToLeaf walks currentState down through initial children
// to the leaf (W3C SCXML 3.3).
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
			// Non-parallel: template doesn't recurse, so we enter here.
			e.policy.ExecuteEntryActions(child, e)
		}
	}
}
