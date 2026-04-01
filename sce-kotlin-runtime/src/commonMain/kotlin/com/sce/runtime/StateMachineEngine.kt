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
     */
    abstract fun executeTransitionActions(source: S, event: E)

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

            // Flush pending final state from initial entry (e.g., test415:
            // initial state IS a final state, so markFinalStateReached()
            // was called during enterInitialConfiguration)
            if (pendingFinalState) {
                pendingFinalState = false
                isInFinalState = true
            }

            // W3C SCXML 3.12.1: Drain internal events raised during initial entry
            // (e.g., done.state events from <final> states entered at startup)
            while (internalEventQueue.isNotEmpty()) {
                val internalEvent = internalEventQueue.removeFirst()
                processOneEvent(internalEvent)
            }

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
     * W3C SCXML Appendix D: Process a single microstep.
     *
     * 1. Process the external event
     * 2. Drain internal event queue (from <raise>)
     */
    private fun processMicrostep(event: E) {
        processOneEvent(event)

        // W3C SCXML 3.12.1: Drain internal event queue
        while (internalEventQueue.isNotEmpty()) {
            val internalEvent = internalEventQueue.removeFirst()
            processOneEvent(internalEvent)
        }
    }

    private fun processOneEvent(event: E) {
        val currentStateValue = _currentState.value

        when (val result = processEvent(currentStateValue, event)) {
            is TransitionResult.External -> {
                val source = currentStateValue
                val target = result.target

                // W3C SCXML 3.13: Exit -> Transition Actions -> Entry
                onExit(source)
                executeTransitionActions(source, event)
                onEntry(target)

                // Update observable state BEFORE flushing isInFinalState.
                // This guarantees observers never see isInFinalState=true
                // while currentState still reflects the source state.
                _currentState.value = target
                if (pendingFinalState) {
                    pendingFinalState = false
                    isInFinalState = true
                }

                // Emit transition record
                _transitions.tryEmit(
                    TransitionRecord(
                        source = source,
                        event = event,
                        target = target,
                        timestamp = nextTimestamp()
                    )
                )
            }
            is TransitionResult.Internal -> {
                // W3C SCXML 3.13: type="internal" — actions only
                executeTransitionActions(currentStateValue, event)
            }
            is TransitionResult.Ignored -> {
                // W3C SCXML 3.12: No matching transition, discard event
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
