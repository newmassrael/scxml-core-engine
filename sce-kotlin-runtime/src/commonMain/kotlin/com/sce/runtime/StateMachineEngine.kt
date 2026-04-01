// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// SCE Kotlin Runtime — Abstract state machine engine

package com.sce.runtime

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

/**
 * Abstract base class for generated SCXML state machines.
 *
 * Provides the event processing loop, state observation via StateFlow,
 * and transition history via SharedFlow. Generated code overrides
 * [processEvent], [onEntry], [onExit], and [executeTransitionActions].
 *
 * Threading model:
 *   - Microstep loop runs on [Dispatchers.Default] (never blocks UI)
 *   - State observation via [currentState] (StateFlow, Compose-ready)
 *   - [send] is non-suspending (Channel.UNLIMITED, always succeeds)
 *
 * W3C SCXML Appendix D: Microstep algorithm.
 *
 * @param S State sealed interface type
 * @param E Event sealed interface type
 */
abstract class StateMachineEngine<S : State, E : Event> {

    // --- Active State Configuration (W3C SCXML 5.9.2) ---

    /**
     * W3C SCXML 5.9.2: Set of currently active state IDs for In() predicate.
     *
     * Tracks all active states including parallel region children.
     * Managed by generated [onEntry]/[onExit] code.
     *
     * Thread safety: Only accessed from the microstep coroutine
     * ([Dispatchers.Default] single-writer). Do not access from external threads.
     */
    protected val activeStateIds: MutableSet<String> = mutableSetOf()

    /**
     * W3C SCXML 5.9.2: Check if a state is in the active configuration.
     *
     * Used by generated code for In() predicate evaluation.
     * Must only be called from within the microstep loop (same coroutine as
     * [processEvent], [onEntry], [onExit]).
     *
     * @param stateId The SCXML state ID to check
     * @return true if the state is currently active
     */
    protected fun isStateActive(stateId: String): Boolean = stateId in activeStateIds

    // --- Observable State ---

    private val _currentState: MutableStateFlow<S> by lazy {
        MutableStateFlow(initialState)
    }

    /**
     * Latest state of the state machine (conflated).
     *
     * Compose integration: `val state by sm.currentState.collectAsState()`
     */
    val currentState: StateFlow<S> get() = _currentState.asStateFlow()

    private val _transitions = MutableSharedFlow<TransitionRecord<S, E>>(
        extraBufferCapacity = Channel.UNLIMITED
    )

    /**
     * All transition records (non-conflated) for debugging and logging.
     *
     * Every transition is emitted, even rapid sequential ones.
     */
    val transitions: SharedFlow<TransitionRecord<S, E>> get() = _transitions.asSharedFlow()

    // --- Event Queue ---

    /**
     * W3C SCXML 3.12.1: Event channel (FIFO, unbounded).
     *
     * Channel.UNLIMITED ensures [send] never blocks and never drops events.
     * Recreated on each [start] to support stop/start cycles.
     */
    private var eventChannel = Channel<E>(Channel.UNLIMITED)

    /**
     * W3C SCXML 3.12.1: Internal events from <raise> are processed
     * before external events. This queue is drained first each microstep.
     */
    private val internalEventQueue = ArrayDeque<E>()

    // --- Lifecycle ---

    private var job: Job? = null
    private var engineScope: CoroutineScope? = null

    /**
     * Whether the state machine has reached a final state.
     *
     * Guaranteed to be visible only after [currentState] reflects the final state.
     * This ordering is enforced by [markFinalStateReached] + deferred flush in
     * [processOneEvent], preventing observers from seeing isInFinalState=true
     * while currentState still points to the source state.
     */
    @Volatile
    var isInFinalState: Boolean = false
        private set

    /**
     * Pending final state flag, set by generated onEntry() code via
     * [markFinalStateReached]. Flushed to [isInFinalState] after
     * _currentState.value is updated in [processOneEvent].
     */
    private var pendingFinalState: Boolean = false

    /**
     * W3C SCXML 3.7: Mark that a top-level final state has been entered.
     *
     * Called from generated [onEntry] code. The actual [isInFinalState] flag
     * is deferred until [_currentState] is updated, so that observers never
     * see isInFinalState=true with a stale currentState value.
     */
    protected fun markFinalStateReached() {
        pendingFinalState = true
    }

    // --- Generated Code Overrides ---

    /**
     * Initial state of the state machine.
     *
     * W3C SCXML 3.2: Resolved from the `initial` attribute.
     * Must be an atomic (leaf) state for processEvent to work correctly.
     */
    abstract val initialState: S

    /**
     * Pure function: determine transition result for (state, event) pair.
     *
     * Generated as exhaustive `when` expressions over state and event types.
     * No side effects — the engine handles exit/entry/action ordering.
     *
     * W3C SCXML 3.12: Event processing algorithm.
     */
    abstract fun processEvent(state: S, event: E): TransitionResult<S>

    /**
     * W3C SCXML Appendix D: Check for eventless (null) transitions.
     *
     * Eventless transitions fire automatically after state entry,
     * before waiting for external events. Override in generated code
     * for state machines that have eventless transitions.
     *
     * @return TransitionResult for any enabled eventless transition, or Ignored
     */
    protected open fun processNullEvent(state: S): TransitionResult<S> = TransitionResult.Ignored

    /**
     * Execute entry actions for a state.
     *
     * W3C SCXML 3.8: `<onentry>` executable content.
     */
    abstract fun onEntry(state: S)

    /**
     * Execute exit actions for a state.
     *
     * W3C SCXML 3.9: `<onexit>` executable content.
     */
    abstract fun onExit(state: S)

    /**
     * Execute transition actions for a (source, event) pair.
     *
     * W3C SCXML 3.13: Executable content within `<transition>`.
     * Called between onExit(source) and onEntry(target).
     *
     * @param event null for eventless transitions
     */
    abstract fun executeTransitionActions(source: S, event: E?)

    // --- State Hierarchy (W3C SCXML 3.3/3.4) ---

    /**
     * W3C SCXML 3.3: Get the parent of a state in the hierarchy.
     *
     * Override in generated code with the actual state hierarchy.
     * Returns null for root states.
     */
    protected open fun parentOf(state: S): S? = null

    /**
     * W3C SCXML 3.4: Check if [descendant] is a descendant of [ancestor].
     *
     * Uses [parentOf] to walk up the hierarchy.
     */
    protected fun isDescendantOf(descendant: S, ancestor: S): Boolean {
        var current: S? = parentOf(descendant)
        while (current != null) {
            if (current == ancestor) return true
            current = parentOf(current)
        }
        return false
    }

    /**
     * Resolve a compound/parallel state to its initial leaf state.
     *
     * Override in generated code for state machines with compound/parallel states.
     * Default returns state unchanged (already a leaf).
     */
    protected open fun resolveLeafState(state: S): S = state

    /**
     * Resolve a state ID string back to its State object.
     *
     * Override in generated code to map state IDs to sealed interface objects.
     * Used by the runtime to iterate over active states for parallel processing.
     */
    protected open fun resolveState(stateId: String): S? = null

    /**
     * Check if a state is an atomic (leaf) state — no children.
     *
     * Override in generated code. Default returns true (flat state machines).
     */
    protected open fun isAtomicState(state: S): Boolean = true

    /**
     * W3C SCXML 3.13: Get document order index for exit order sorting.
     *
     * Override in generated code. Higher values = later in document.
     */
    protected open fun documentOrderOf(state: S): Int = 0

    /**
     * Get the string state ID for a state object (reverse of resolveState).
     *
     * Override in generated code.
     */
    protected open fun stateIdOf(state: S): String = ""

    // --- Event Submission ---

    /**
     * Submit an event to the state machine (non-suspending, fire-and-forget).
     *
     * Always succeeds because the channel is UNLIMITED.
     * UI event handlers are not suspend functions, so this is non-suspending.
     */
    fun send(event: E) {
        eventChannel.trySend(event)
    }

    /**
     * Submit an event and suspend until the resulting state is available.
     *
     * Useful for testing: `val newState = sm.sendAndAwait(event)`.
     */
    suspend fun sendAndAwait(event: E): S {
        val before = _currentState.value
        send(event)
        // R1 fix: first{} is a terminal operator that completes after matching
        return _currentState.first { it != before }
    }

    // --- Lifecycle ---

    /**
     * W3C SCXML 3.2/3.4: Enter initial state configuration.
     *
     * Default: enter [initialState] only (flat state machines).
     * Override in generated code to handle compound/parallel state hierarchies
     * where ancestors must be entered recursively from root to leaf.
     *
     * For parallel states (W3C SCXML 3.4), the override enters from the
     * top-level initial state, recursing through [onEntry] to activate
     * all child regions and register them in [activeStateIds].
     */
    protected open fun enterInitialConfiguration() {
        onEntry(initialState)
    }

    /**
     * Start the event processing loop.
     *
     * W3C SCXML Appendix D: Enter initial state, then process events.
     *
     * @param scope CoroutineScope that controls the lifecycle.
     *              Use `viewModelScope` for Android, `TestScope` for testing.
     */
    fun start(scope: CoroutineScope) {
        if (job != null) return  // Already started

        // R3 fix: Recreate channel for stop/start reuse
        eventChannel = Channel(Channel.UNLIMITED)

        // R2 fix: Store scope for delayed send support
        engineScope = scope

        job = scope.launch(Dispatchers.Default) {
            // R4 fix: Execute initial entry on Dispatchers.Default, not caller thread
            enterInitialConfiguration()

            // Resolve to leaf state after initial configuration
            _currentState.value = resolveLeafState(_currentState.value)

            // Flush pending final state from initial entry (e.g., test415:
            // initial state IS a final state)
            flushPendingFinalState()

            // W3C SCXML Appendix D: Process eventless transitions and internal
            // events raised during initial entry (e.g., done.state from <final>)
            drainEventlessAndInternal()

            for (event in eventChannel) {
                if (isInFinalState) break
                processMicrostep(event)
            }
        }
    }

    /**
     * Stop the event processing loop.
     *
     * Cancels the coroutine and closes the event channel.
     * The engine can be restarted with [start].
     */
    fun stop() {
        job?.cancel()
        job = null
        engineScope = null
        eventChannel.close()
        delayedSendJobs.values.forEach { it.cancel() }
        delayedSendJobs.clear()
        // Reset state for stop/start reuse
        activeStateIds.clear()
        isInFinalState = false
        pendingFinalState = false
        internalEventQueue.clear()
    }

    // --- Internal Event Queue (for <raise>) ---

    /**
     * W3C SCXML 3.12.1: Raise an internal event (processed before external events).
     *
     * Called from generated onEntry/onExit/executeTransitionActions code.
     * Always called from the microstep coroutine (single-threaded access).
     */
    protected fun raiseInternal(event: E) {
        internalEventQueue.addLast(event)
    }

    // --- Delayed Send Support ---

    /** Active delayed send jobs, keyed by sendid for cancellation. */
    private val delayedSendJobs = mutableMapOf<String, Job>()

    /**
     * W3C SCXML 6.2: Schedule a delayed event send.
     *
     * @param sendId Identifier for cancellation via `<cancel>`
     * @param delayMs Delay in milliseconds
     * @param event Event to send after delay
     */
    protected fun scheduleSend(sendId: String, delayMs: Long, event: E) {
        val scope = engineScope ?: return
        delayedSendJobs[sendId]?.cancel()
        delayedSendJobs[sendId] = scope.launch {
            kotlinx.coroutines.delay(delayMs)
            send(event)
            delayedSendJobs.remove(sendId)
        }
    }

    /**
     * W3C SCXML 6.3: Cancel a delayed event send.
     *
     * @param sendId Identifier of the send to cancel
     */
    protected fun cancelSend(sendId: String) {
        delayedSendJobs.remove(sendId)?.cancel()
    }

    // --- Microstep Processing ---

    /**
     * W3C SCXML Appendix D: Process a macrostep triggered by an external event.
     *
     * Algorithm:
     * 1. Process the external event
     * 2. Drain eventless transitions and internal events until stable
     */
    private fun processMicrostep(event: E) {
        processOneEvent(event)
        drainEventlessAndInternal()
    }

    /**
     * Flush pending final state flag to the observable [isInFinalState].
     *
     * Called after initial configuration and at stable points to ensure
     * final state is visible even when no transitions fired.
     */
    private fun flushPendingFinalState() {
        if (pendingFinalState) {
            pendingFinalState = false
            isInFinalState = true
        }
    }

    /**
     * Collect active atomic (leaf) states sorted by document order.
     *
     * W3C SCXML 3.13: Document order determines transition priority
     * when multiple parallel children could handle the same event.
     * Returns empty list for simple machines (no activeStateIds).
     */
    private fun activeLeafStatesInDocumentOrder(): List<S> {
        if (activeStateIds.isEmpty()) return emptyList()
        val leaves = mutableListOf<Pair<S, Int>>()
        for (stateId in activeStateIds) {
            val state = resolveState(stateId) ?: continue
            if (!isAtomicState(state)) continue
            leaves.add(state to documentOrderOf(state))
        }
        leaves.sortBy { it.second }
        return leaves.map { it.first }
    }

    /**
     * W3C SCXML Appendix D: Drain eventless transitions and internal events.
     *
     * Repeats until no more eventless transitions are enabled and the
     * internal event queue is empty. This implements the inner loop of
     * the W3C macrostep algorithm.
     *
     * W3C SCXML 3.4: For parallel states, ALL non-conflicting eventless
     * transitions are selected and fired in a single microstep. This matches
     * the W3C selectEventlessTransitions() algorithm where transitions in
     * different parallel regions execute simultaneously.
     */
    private fun drainEventlessAndInternal() {
        while (!isInFinalState) {
            // W3C SCXML Appendix D: Eventless transitions take priority
            var foundEventless = false

            val leaves = activeLeafStatesInDocumentOrder()
            if (leaves.isNotEmpty()) {
                // W3C SCXML Appendix D: Select ALL enabled eventless transitions
                val enabledTransitions = mutableListOf<Pair<S, TransitionResult<S>>>()
                for (state in leaves) {
                    val nullResult = processNullEvent(state)
                    if (nullResult !is TransitionResult.Ignored) {
                        enabledTransitions.add(state to nullResult)
                    }
                }

                if (enabledTransitions.size > 1) {
                    // W3C SCXML Appendix D.2: Multiple simultaneous transitions
                    // in different parallel regions — batch microstep
                    applySimultaneousTransitions(enabledTransitions)
                    foundEventless = true
                } else if (enabledTransitions.size == 1) {
                    val (source, result) = enabledTransitions[0]
                    applyTransitionFrom(source, result, null)
                    foundEventless = true
                }
            } else {
                // Simple machines without activeStateIds tracking — use _currentState
                val nullResult = processNullEvent(_currentState.value)
                if (nullResult !is TransitionResult.Ignored) {
                    applyTransition(nullResult, null)
                    foundEventless = true
                }
            }
            if (foundEventless) {
                flushPendingFinalState()
                continue
            }

            // W3C SCXML 3.12.1: Internal events next
            if (internalEventQueue.isNotEmpty()) {
                val internalEvent = internalEventQueue.removeFirst()
                processOneEvent(internalEvent)
                flushPendingFinalState()
                continue
            }

            // Stable: no eventless transitions, no internal events
            break
        }
    }

    /**
     * W3C SCXML Appendix D.2: Apply multiple non-conflicting transitions
     * as a single microstep.
     *
     * For parallel states, the W3C algorithm requires that all enabled
     * non-conflicting eventless transitions fire simultaneously:
     * 1. Exit all source states in reverse document order
     * 2. Execute all transition actions in document order
     * 3. Enter all target states in document order
     *
     * This ensures correct event ordering when parallel regions have
     * eventless transitions with executable content in exits/actions/entries.
     *
     * NOTE: This assumes all transitions are non-conflicting (different parallel
     * regions). AOT-generated processNullEvent() only returns leaf-state transitions
     * within their own region — ancestor eventless transitions are not included.
     * This makes W3C removeConflictingTransitions() unnecessary for AOT machines.
     */
    private fun applySimultaneousTransitions(
        transitions: List<Pair<S, TransitionResult<S>>>
    ) {
        // Separate External and Internal transitions
        val externals = mutableListOf<Pair<S, TransitionResult.External<S>>>()
        val internals = mutableListOf<Pair<S, TransitionResult.Internal>>()
        for ((source, result) in transitions) {
            when (result) {
                is TransitionResult.External -> externals.add(source to result)
                is TransitionResult.Internal -> internals.add(source to result)
                is TransitionResult.Ignored -> {}
            }
        }

        if (externals.isNotEmpty()) {
            // Sort by source document order
            val sorted = externals.sortedBy { documentOrderOf(it.first) }

            // W3C SCXML Appendix D.2, Step 1: Exit all in reverse document order
            for ((source, result) in sorted.reversed()) {
                exitHierarchy(source, result.target)
            }

            // W3C SCXML Appendix D.2, Step 2: Transition actions in document order
            for ((source, _) in sorted) {
                executeTransitionActions(source, null)
            }

            // W3C SCXML Appendix D.2, Step 3: Enter all targets in document order
            for ((_, result) in sorted) {
                onEntry(result.target)
            }

            // Update _currentState to last entered leaf
            _currentState.value = resolveLeafState(sorted.last().second.target)
        }

        // Internal transitions: execute actions only (no state change)
        for ((source, _) in internals) {
            executeTransitionActions(source, null)
        }
    }

    /**
     * Process a single event (internal or external).
     *
     * For parallel state machines, tries all active leaf states in
     * document order to find a matching transition (first match wins).
     * W3C SCXML 3.13: Document order priority for conflict resolution.
     */
    private fun processOneEvent(event: E) {
        val leaves = activeLeafStatesInDocumentOrder()
        if (leaves.isNotEmpty()) {
            for (state in leaves) {
                val result = processEvent(state, event)
                if (result !is TransitionResult.Ignored) {
                    applyTransitionFrom(state, result, event)
                    return
                }
            }
            // No active state handled the event — ignored
        } else {
            // Simple machine: use _currentState
            val result = processEvent(_currentState.value, event)
            applyTransition(result, event)
        }
    }

    /**
     * Apply a transition result using _currentState as source.
     *
     * @param event null for eventless transitions
     */
    private fun applyTransition(result: TransitionResult<S>, event: E?) {
        applyTransitionFrom(_currentState.value, result, event)
    }

    /**
     * Apply a transition result with an explicit source state.
     *
     * Used by parallel eventless processing where the source may differ
     * from _currentState (multiple active leaf states).
     *
     * @param source the state that originated the transition
     * @param event null for eventless transitions
     */
    private fun applyTransitionFrom(source: S, result: TransitionResult<S>, event: E?) {
        when (result) {
            is TransitionResult.External -> {
                val target = result.target

                // W3C SCXML 3.13: Exit -> Transition Actions -> Entry
                exitHierarchy(source, target)
                executeTransitionActions(source, event)
                onEntry(target)

                // Resolve to leaf state for compound/parallel targets
                val leafTarget = resolveLeafState(target)

                // Update observable state BEFORE flushing isInFinalState.
                _currentState.value = leafTarget
                if (pendingFinalState) {
                    pendingFinalState = false
                    isInFinalState = true
                }

                // Emit transition record (only for event-based transitions)
                if (event != null) {
                    _transitions.tryEmit(
                        TransitionRecord(
                            source = source,
                            event = event,
                            target = leafTarget,
                            timestamp = nextTimestamp()
                        )
                    )
                }
            }
            is TransitionResult.Internal -> {
                // W3C SCXML 3.13: type="internal" — actions only
                executeTransitionActions(source, event)
            }
            is TransitionResult.Ignored -> {
                // W3C SCXML 3.12: No matching transition, discard event
            }
        }
    }

    // --- Hierarchical Exit (W3C SCXML 3.4/3.13) ---

    /**
     * W3C SCXML 3.13: Exit states from source up to the LCCA with target.
     *
     * For flat machines (no activeStateIds), exits source only.
     * For hierarchical machines, computes the proper exit set:
     * 1. Find LCCA (Least Common Compound Ancestor)
     * 2. Collect all active states that are descendants of LCCA
     *    but not the target or its descendants
     * 3. Sort by reverse document order
     * 4. Exit each in order
     *
     * This matches the W3C SCXML algorithm and correctly handles
     * parallel state exit ordering.
     *
     * Note: Generated onExit() for parallel states also contains descendant
     * exit logic as a defensive fallback (e.g., when onExit is called directly
     * outside of exitHierarchy). When called from here, the activeStateIds
     * check in that generated code ensures no double-exit occurs — descendants
     * are already removed by the time the parallel state's onExit runs.
     */
    private fun exitHierarchy(source: S, target: S) {
        if (activeStateIds.isEmpty()) {
            // Simple machine — just exit source
            onExit(source)
            return
        }

        // Step 1: Find LCCA (Least Common Compound Ancestor)
        // Walk up from source to find first ancestor that contains target.
        // If no common ancestor found (lcca == null), all active states
        // from source's subtree upward must be exited.
        var lcca: S? = parentOf(source)
        while (lcca != null) {
            if (lcca == target || isDescendantOf(target, lcca)) break
            lcca = parentOf(lcca)
        }

        // Step 2: Collect active states to exit
        val statesToExit = mutableListOf<Pair<S, Int>>()
        for (stateId in activeStateIds.toList()) {
            val state = resolveState(stateId) ?: continue
            // Don't exit target or its descendants
            if (state == target || isDescendantOf(state, target)) continue
            if (lcca != null) {
                // Normal case: exit descendants of LCCA (but not LCCA itself)
                if (state == lcca) continue
                if (!isDescendantOf(state, lcca)) continue
            } else {
                // No common ancestor: exit source and all its ancestors/descendants
                // This handles root→root transitions (e.g., parallel root to final)
                if (state != source && !isDescendantOf(state, source) && !isDescendantOf(source, state)) continue
            }
            statesToExit.add(state to documentOrderOf(state))
        }

        // Step 3: Sort by reverse document order (deepest states first)
        statesToExit.sortByDescending { it.second }

        // Step 4: Exit each
        for ((state, _) in statesToExit) {
            // Check still active (may have been removed by a parallel's onExit)
            val sid = stateIdOf(state)
            if (sid.isNotEmpty() && activeStateIds.contains(sid)) {
                onExit(state)
            }
        }
    }

    /**
     * Monotonic sequence counter for transition ordering.
     *
     * KMP commonMain does not have System.nanoTime(). For Phase 1,
     * a monotonic counter is sufficient for transition ordering.
     * Platform-specific implementations can override with real timestamps.
     */
    private var sequenceCounter = 0L
    private fun nextTimestamp(): Long = sequenceCounter++
}
