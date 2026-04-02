// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// SCE Kotlin Runtime — Abstract state machine engine

package com.sce.runtime

import kotlinx.coroutines.CompletableDeferred
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
 * W3C SCXML 5.10: Event metadata for _event system variable.
 *
 * Carries type, data, sendid, origin, origintype, and invokeid
 * alongside events through the processing pipeline.
 */
data class EventMetadata(
    val data: String = "",
    val type: String = "external",
    val sendId: String = "",
    val origin: String = "",
    val originType: String = "",
    val invokeId: String = ""
) {
    companion object {
        val EMPTY = EventMetadata()
        fun internal() = EventMetadata(type = "internal")
        fun platform() = EventMetadata(type = "platform")
        fun platform(data: String) = EventMetadata(type = "platform", data = data)
        fun external(
            sendId: String = "",
            origin: String = "",
            originType: String = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor",
            data: String = ""
        ) = EventMetadata(
            type = "external",
            sendId = sendId,
            origin = origin,
            originType = originType,
            data = data
        )
    }
}

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
abstract class StateMachineEngine<S : State, E : Event>(
    protected val scriptEngine: ScxmlScriptEngine? = null
) {

    // --- Script Engine Session (W3C SCXML B.1) ---

    /** Session ID for script engine scope isolation. */
    protected var scriptSessionId: String? = null
        private set

    /** Lazy initialization flag — generated ensureScriptEngine() sets this. */
    protected var scriptEngineInitialized: Boolean = false

    /**
     * Allocate a script engine session ID.
     * Called by generated ensureScriptEngine() during lazy initialization.
     */
    protected fun allocateScriptSession(): String {
        val sid = "session_${hashCode()}"
        scriptSessionId = sid
        return sid
    }

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

    // --- Event Metadata (W3C SCXML 5.10) ---

    /** Internal wrapper pairing an event with its W3C SCXML 5.10 metadata. */
    private data class QueuedEvent<E>(val event: E, val metadata: EventMetadata = EventMetadata.EMPTY)

    /**
     * W3C SCXML 5.10: Metadata for the event currently being processed.
     * Set before processEvent/processNullEvent so that generated
     * setCurrentEventInScriptEngine() can read it.
     */
    protected var currentEventMetadata: EventMetadata = EventMetadata.EMPTY

    /**
     * W3C SCXML 3.12.1: Event channel (FIFO, unbounded).
     *
     * Channel.UNLIMITED ensures [send] never blocks and never drops events.
     * Recreated on each [start] to support stop/start cycles.
     */
    private var eventChannel = Channel<QueuedEvent<E>>(Channel.UNLIMITED)

    /**
     * W3C SCXML 3.12.1: Internal events from <raise> are processed
     * before external events. This queue is drained first each microstep.
     */
    private val internalEventQueue = ArrayDeque<QueuedEvent<E>>()

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
     * Succeeds while the SM is running (Channel.UNLIMITED, never drops).
     * After the SM reaches a final state, the channel is closed and events
     * are silently discarded (SM no longer processes events per W3C SCXML 3.7).
     */
    fun send(event: E) {
        eventChannel.trySend(QueuedEvent(event))
    }

    /**
     * W3C SCXML 5.10: Submit an event with metadata (type, data, sendid, etc.).
     *
     * Used by generated send actions that need to attach event metadata.
     */
    fun send(event: E, metadata: EventMetadata) {
        eventChannel.trySend(QueuedEvent(event, metadata))
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
        eventChannel = Channel<QueuedEvent<E>>(Channel.UNLIMITED)

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

            // W3C SCXML 6.4: Execute deferred invokes after initial configuration
            executePendingInvokes()

            // W3C SCXML 3.7: Only enter event loop if not already in final state
            // (child SMs may reach final state during drainEventlessAndInternal)
            if (!isInFinalState) {
                for (queued in eventChannel) {
                    if (isInFinalState) break
                    currentEventMetadata = queued.metadata
                    processMicrostep(queued.event)
                }
            }

            // W3C SCXML 6.2: Cancel pending delayed sends on session termination
            // Per spec, terminated sessions must not deliver delayed events (test187)
            delayedSendJobs.values.forEach { it.cancel() }
            delayedSendJobs.clear()
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
        // W3C SCXML 6.4: Cancel all active invokes
        for ((_, entry) in activeInvokes) {
            entry.child.stop()
            entry.monitorJob.cancel()
        }
        activeInvokes.clear()
        pendingInvokes.clear()
        // W3C SCXML B.1: Destroy script engine session
        if (scriptEngineInitialized) {
            scriptSessionId?.let { scriptEngine?.destroySession(it) }
            scriptEngineInitialized = false
            scriptSessionId = null
        }
        // Reset state for stop/start reuse
        activeStateIds.clear()
        isInFinalState = false
        pendingFinalState = false
        internalEventQueue.clear()
        completion = CompletableDeferred()
    }

    // --- Internal Event Queue (for <raise>) ---

    /**
     * W3C SCXML 3.12.1: Raise an internal event (processed before external events).
     *
     * Called from generated onEntry/onExit/executeTransitionActions code.
     * Always called from the microstep coroutine (single-threaded access).
     * Default metadata type = "internal" per W3C SCXML 5.10.
     */
    protected fun raiseInternal(event: E) {
        internalEventQueue.addLast(QueuedEvent(event, EventMetadata.internal()))
    }

    /**
     * W3C SCXML 5.10: Raise an internal event with explicit metadata.
     *
     * Used for platform events (done.state, error.*) and events carrying data.
     */
    protected fun raiseInternal(event: E, metadata: EventMetadata) {
        internalEventQueue.addLast(QueuedEvent(event, metadata))
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
        scheduleSend(sendId, delayMs, event, EventMetadata.EMPTY)
    }

    /**
     * W3C SCXML 6.2: Schedule a delayed event send with metadata.
     */
    protected fun scheduleSend(sendId: String, delayMs: Long, event: E, metadata: EventMetadata) {
        val scope = engineScope ?: return
        delayedSendJobs[sendId]?.cancel()
        delayedSendJobs[sendId] = scope.launch(Dispatchers.Default) {
            kotlinx.coroutines.delay(delayMs)
            send(event, metadata)
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

    /**
     * W3C SCXML 6.4 (test187): Schedule a delayed send to parent.
     * Cancelled when child session stops (all delayedSendJobs are cancelled in stop()).
     */
    protected fun scheduleParentSend(sendId: String, delayMs: Long, eventName: String) {
        scheduleParentSend(sendId, delayMs, eventName, "")
    }

    /**
     * W3C SCXML 6.4: Schedule a delayed send to parent with event data.
     */
    protected fun scheduleParentSend(sendId: String, delayMs: Long, eventName: String, eventData: String) {
        val scope = engineScope ?: return
        delayedSendJobs[sendId]?.cancel()
        delayedSendJobs[sendId] = scope.launch(Dispatchers.Default) {
            kotlinx.coroutines.delay(delayMs)
            onSendToParent?.invoke(eventName, eventData)
            delayedSendJobs.remove(sendId)
        }
    }

    // --- Invoke Support (W3C SCXML 6.4) ---

    /**
     * W3C SCXML 6.4: Completion signal for invoke monitoring.
     * Completes when this state machine reaches a top-level final state.
     * Reset on [stop] for stop/start reuse.
     */
    var completion: CompletableDeferred<Unit> = CompletableDeferred()
        private set

    /**
     * W3C SCXML 6.4: Callback for child SMs to send events to parent.
     * Set by parent when starting an invoked child SM.
     * Called from generated code when child executes send target="#_parent".
     * Parameters: (eventName: String, eventData: String)
     */
    @Volatile
    var onSendToParent: ((String, String) -> Unit)? = null
        internal set

    /** Active invoked child sessions, keyed by invoke ID. */
    private data class InvokeEntry(
        val child: StateMachineEngine<*, *>,
        val monitorJob: Job,
        val autoforward: Boolean,
        val finalizeScript: String = ""
    )
    private val activeInvokes = mutableMapOf<String, InvokeEntry>()

    // --- Deferred Invoke Support (W3C SCXML 6.4) ---

    /**
     * W3C SCXML 6.4: Pending invoke to be executed at macrostep end.
     *
     * Invokes are deferred during state entry and only executed at macrostep end.
     * This ensures that invokes in states entered-then-exited during a macrostep
     * are cancelled and never executed (e.g., test 422).
     */
    private data class PendingInvoke<S>(
        val invokeId: String,
        val state: S,
        val executor: () -> Unit
    )
    private val pendingInvokes = mutableListOf<PendingInvoke<S>>()

    /**
     * W3C SCXML 6.4: Defer an invoke for execution at macrostep end.
     *
     * Called from generated onEntry code instead of startInvoke directly.
     * The executor lambda captures the full invoke setup (child creation,
     * param passing, startInvoke call).
     */
    protected fun deferInvoke(state: S, invokeId: String, executor: () -> Unit) {
        pendingInvokes.add(PendingInvoke(invokeId, state, executor))
    }

    /**
     * W3C SCXML 6.4: Cancel pending invokes for a state being exited.
     *
     * Called from generated onExit code. Removes any deferred invokes
     * for states that were entered but exited before macrostep end.
     */
    protected fun cancelPendingInvokesForState(state: S) {
        pendingInvokes.removeAll { it.state == state }
    }

    /**
     * W3C SCXML 6.4: Execute all pending invokes at macrostep end.
     *
     * Only invokes in states that are still active (entered and not exited
     * during the macrostep) are executed. Called after drainEventlessAndInternal().
     */
    private fun executePendingInvokes() {
        if (pendingInvokes.isEmpty()) return
        val toExecute = pendingInvokes.toList()
        pendingInvokes.clear()
        for (pending in toExecute) {
            pending.executor()
        }
    }

    /**
     * W3C SCXML 6.4: Start an invoked child state machine.
     *
     * @param invokeId Invoke session identifier
     * @param child Child state machine instance
     * @param autoforward Forward parent events to child
     * @param doneEvent Event to send when child completes (done.invoke)
     */
    /**
     * W3C SCXML 6.4: Set invoke parameters on a child SM before starting it.
     *
     * Stores param values that will be applied when the child's script engine
     * is initialized. Called by generated code between child construction
     * and startInvoke.
     *
     * @param child Child state machine instance
     * @param params Map of variable name to value pairs
     */
    protected fun setInvokeParams(child: StateMachineEngine<*, *>, params: Map<String, Any?>) {
        child.pendingInvokeParams = params
    }

    /**
     * W3C SCXML 6.4: Pending invoke parameters to be applied during script engine init.
     * Set by parent's setInvokeParams, consumed by child's ensureScriptEngine.
     */
    protected var pendingInvokeParams: Map<String, Any?> = emptyMap()

    /**
     * W3C SCXML 6.4: Start an invoked child state machine.
     *
     * @param invokeId Static invoke element ID — used for activeInvokes key, done.invoke metadata, cancelInvoke
     * @param child Child state machine instance
     * @param autoforward Forward parent events to child
     * @param doneEvent Event to send when child completes (done.invoke)
     * @param finalizeScript W3C SCXML 6.5: Script to execute before child events are processed
     * @param generatedInvokeId Runtime-generated ID (stateid.platformid.index) — used for child-to-parent event metadata
     */
    protected fun startInvoke(
        invokeId: String,
        child: StateMachineEngine<*, *>,
        autoforward: Boolean,
        doneEvent: E?,
        finalizeScript: String = "",
        generatedInvokeId: String = invokeId
    ) {
        val scope = engineScope ?: return

        // W3C SCXML 6.4: Set up child->parent event routing with metadata
        // Child-to-parent events carry the generated invoke ID (matches idlocation value)
        child.onSendToParent = { eventName, eventData ->
            resolveEventByName(eventName)?.let {
                send(it, EventMetadata(
                    type = "external",
                    invokeId = generatedInvokeId,
                    origin = child.scriptSessionId ?: "",
                    originType = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor",
                    data = eventData
                ))
            }
        }

        // Start child on the same scope
        child.start(scope)

        // Monitor child completion for done.invoke
        // Run on Dispatchers.Default to avoid BlockingEventLoop deadlock in tests
        val monitorJob = scope.launch(Dispatchers.Default) {
            child.completion.await()
            if (doneEvent != null) {
                // W3C SCXML 5.10: done.invoke uses static invoke ID
                send(doneEvent, EventMetadata(
                    type = "platform",
                    invokeId = invokeId
                ))
            }
            // W3C SCXML 6.5: Do NOT remove from activeInvokes here.
            // The removal must happen on the parent's microstep thread after
            // finalize has had a chance to execute. The parent's event loop
            // cleans up completed invokes via cleanupCompletedInvokes().
        }

        activeInvokes[invokeId] = InvokeEntry(child, monitorJob, autoforward, finalizeScript)
    }

    /**
     * W3C SCXML 6.4: Cancel an invoked child state machine on state exit.
     */
    protected fun cancelInvoke(invokeId: String) {
        activeInvokes.remove(invokeId)?.let {
            it.child.stop()
            it.monitorJob.cancel()
        }
    }

    /**
     * W3C SCXML 6.4: Send event to invoked child by invoke ID.
     * Uses string-based routing for type-erased cross-SM communication.
     */
    protected fun sendToChild(invokeId: String, eventName: String) {
        activeInvokes[invokeId]?.child?.sendByName(eventName)
    }

    /**
     * W3C SCXML 6.4: Send event by name (string-based, for cross-SM routing).
     * Internal: only used by parent SM's [sendToChild] for type-erased communication.
     */
    internal fun sendByName(name: String) {
        resolveEventByName(name)?.let { send(it) }
    }

    // --- Event Data Helpers ---

    /**
     * W3C SCXML 5.10: Build JSON object from evaluated param name/value pairs.
     *
     * Matches C++ EventDataHelper::buildJsonFromParams behavior.
     * Used by generated send/donedata code to construct _event.data payload.
     */
    protected fun buildJsonFromParams(params: Map<String, Any?>): String {
        if (params.isEmpty()) return ""
        val sb = StringBuilder("{")
        var first = true
        for ((key, value) in params) {
            if (!first) sb.append(",")
            first = false
            sb.append("\"").append(key).append("\":")
            sb.append(valueToJson(value))
        }
        sb.append("}")
        return sb.toString()
    }

    protected fun valueToJson(value: Any?): String = when (value) {
        null -> "null"
        is Boolean -> value.toString()
        is Number -> {
            val d = value.toDouble()
            if (d == d.toLong().toDouble() && !d.isInfinite()) d.toLong().toString()
            else d.toString()
        }
        is String -> "\"${value.replace("\\", "\\\\").replace("\"", "\\\"")}\""
        is Map<*, *> -> {
            val entries = value.entries.joinToString(",") { (k, v) ->
                "\"${k}\":${valueToJson(v)}"
            }
            "{$entries}"
        }
        is List<*> -> {
            val items = value.joinToString(",") { valueToJson(it) }
            "[$items]"
        }
        is Array<*> -> {
            val items = value.joinToString(",") { valueToJson(it) }
            "[$items]"
        }
        else -> "\"${value.toString().replace("\\", "\\\\").replace("\"", "\\\"")}\""
    }

    /**
     * W3C SCXML 6.4: Resolve event name string to Event object.
     * Override in generated code for cross-SM event routing.
     */
    protected open fun resolveEventByName(name: String): E? = null

    /**
     * W3C SCXML 6.4: Resolve Event object to event name string.
     * Reverse of [resolveEventByName]. Override in generated code.
     * Used by autoforward to convert typed parent events to string names
     * for type-erased child routing.
     */
    protected open fun eventNameOf(event: E): String? = null

    // --- Finalize Support (W3C SCXML 6.5) ---

    /**
     * W3C SCXML 6.5: Execute finalize for events from invoked children.
     *
     * Finalize runs BEFORE the event is processed, with _event set to the child's event.
     * This allows finalize to update parent datamodel variables based on event data
     * before transition guards are evaluated.
     *
     * Matches C++ executeFinalizeForChildEvent() behavior.
     */
    private fun executeFinalizeForChildEvent(event: E) {
        val metadata = currentEventMetadata
        if (metadata.origin.isEmpty()) return

        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return

        for ((_, entry) in activeInvokes) {
            if (entry.finalizeScript.isNotEmpty() &&
                entry.child.scriptSessionId == metadata.origin) {
                // W3C SCXML 6.5: Set _event before finalize execution
                val eventName = eventNameOf(event) ?: ""
                engine.setCurrentEvent(
                    sid, eventName,
                    data = metadata.data,
                    type = metadata.type,
                    sendId = metadata.sendId,
                    origin = metadata.origin,
                    originType = metadata.originType,
                    invokeId = metadata.invokeId
                )
                // W3C SCXML 6.5: Execute finalize script
                try {
                    engine.executeScript(sid, entry.finalizeScript)
                } catch (_: Exception) {
                    // Finalize errors are silently ignored per spec
                }
                return
            }
        }
    }

    // --- Microstep Processing ---

    /**
     * W3C SCXML Appendix D: Process a macrostep triggered by an external event.
     *
     * Algorithm:
     * 1. Execute finalize for events from invoked children (W3C SCXML 6.5)
     * 2. Auto-forward event to child sessions (W3C SCXML 6.4.6)
     * 3. Process the external event
     * 4. Drain eventless transitions and internal events until stable
     * 5. Execute pending invokes at macrostep end (W3C SCXML 6.4)
     * 6. Clean up completed invoke sessions
     */
    private fun processMicrostep(event: E) {
        // W3C SCXML 6.5: Execute finalize before event processing
        executeFinalizeForChildEvent(event)
        // W3C SCXML 6.4: Auto-forward external events to child invoke sessions.
        // Only external events (from the event channel) reach processMicrostep;
        // internal events from <raise> are processed in drainEventlessAndInternal
        // and must NOT be forwarded per spec.
        autoForwardEvent(event)
        processOneEvent(event)
        drainEventlessAndInternal()
        // W3C SCXML 6.4: Execute deferred invokes at macrostep end
        executePendingInvokes()
        // W3C SCXML 6.5: Clean up completed invokes (deferred from monitor coroutine)
        cleanupCompletedInvokes()
    }

    /**
     * W3C SCXML 6.5: Remove completed invoke entries from activeInvokes.
     *
     * Cleanup is deferred from the monitor coroutine to the parent's microstep
     * thread so that finalize can access the InvokeEntry (including its
     * finalizeScript) before it is removed.
     */
    private fun cleanupCompletedInvokes() {
        if (activeInvokes.isEmpty()) return
        activeInvokes.entries.removeAll { it.value.child.isInFinalState }
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

            // W3C SCXML 3.8: Execute onexit actions for the final state before
            // notifying parent. Matches C++ AOT StaticExecutionEngine::initialize()
            // which calls executeOnExit(currentState_) for the final state only.
            // Ancestors are NOT exited here — transition exitHierarchy already
            // handled ancestor exits. This ensures child-to-parent events
            // (e.g., test236 SubFinal onexit) arrive before done.invoke.
            onExit(_currentState.value)

            // W3C SCXML 6.4: Notify invoke monitors that this SM completed
            if (!completion.isCompleted) completion.complete(Unit)
            // W3C SCXML 3.7: Close event channel to terminate event loop coroutine.
            // After reaching final state, no further events are processed.
            eventChannel.close()
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
                val queued = internalEventQueue.removeFirst()
                currentEventMetadata = queued.metadata
                processOneEvent(queued.event)
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
                is TransitionResult.InternalToTarget -> {
                    // Treat internal-with-target like external for parallel batch processing
                    externals.add(source to TransitionResult.External(result.target, result.transitionSource))
                }
                is TransitionResult.Internal -> internals.add(source to result)
                is TransitionResult.Ignored -> {}
            }
        }

        if (externals.isNotEmpty()) {
            // Sort by source document order
            val sorted = externals.sortedBy { documentOrderOf(it.first) }

            // W3C SCXML Appendix D.2, Step 1: Exit all in reverse document order
            for ((source, result) in sorted.reversed()) {
                exitHierarchy(source, result.target, result.transitionSource)
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
     * W3C SCXML 6.4.6: Forward external event to all autoforward child sessions.
     *
     * Matches C++ AOT StaticExecutionEngine::raiseExternal() which forwards
     * all events without platform event filtering. Child's resolveEventByName
     * silently ignores unrecognized events (e.g., done.invoke).
     */
    private fun autoForwardEvent(event: E) {
        if (activeInvokes.isEmpty()) return

        val eventName = eventNameOf(event) ?: return

        for ((_, entry) in activeInvokes) {
            if (entry.autoforward) {
                entry.child.sendByName(eventName)
            }
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
                // When transitionSource is set, use it for LCCA in the parallel path
                exitHierarchy(source, target, result.transitionSource)
                executeTransitionActions(source, event)
                onEntry(target)

                // Resolve to leaf state for compound/parallel targets
                val leafTarget = resolveLeafState(target)

                // Update observable state BEFORE flushing isInFinalState.
                _currentState.value = leafTarget
                // W3C SCXML 3.7 + 6.4: Single path for final state + invoke completion
                flushPendingFinalState()

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
            is TransitionResult.InternalToTarget -> {
                // W3C SCXML 3.13: Internal transition with target.
                // Exit descendants of transitionSource (but NOT the source itself),
                // execute transition actions, enter target.
                val target = result.target
                val txSource = result.transitionSource

                // Exit active descendants of transitionSource that are not target or its descendants
                if (activeStateIds.isNotEmpty()) {
                    val statesToExit = mutableListOf<Pair<S, Int>>()
                    for (stateId in activeStateIds.toList()) {
                        val state = resolveState(stateId) ?: continue
                        if (state == txSource) continue  // Don't exit the source
                        if (state == target || isDescendantOf(state, target)) continue
                        if (!isDescendantOf(state, txSource)) continue
                        statesToExit.add(state to documentOrderOf(state))
                    }
                    statesToExit.sortByDescending { it.second }
                    for ((state, _) in statesToExit) {
                        val sid = stateIdOf(state)
                        if (sid.isNotEmpty() && activeStateIds.contains(sid)) {
                            onExit(state)
                        }
                    }
                } else {
                    // Simple machine — just exit the current leaf state
                    onExit(source)
                }

                executeTransitionActions(source, event)
                onEntry(target)

                val leafTarget = resolveLeafState(target)
                _currentState.value = leafTarget
                flushPendingFinalState()

                if (event != null) {
                    _transitions.tryEmit(
                        TransitionRecord(source = source, event = event, target = leafTarget, timestamp = nextTimestamp())
                    )
                }
            }
            is TransitionResult.Internal -> {
                // W3C SCXML 3.13: type="internal" — actions only (targetless)
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
    private fun exitHierarchy(source: S, target: S, transitionSource: S? = null) {
        if (activeStateIds.isEmpty()) {
            // Non-parallel machine — exit source and ancestors up to LCCA
            onExit(source)
            // W3C SCXML 3.13: Exit ancestor states when transitioning out.
            // Walk up from source, exiting each ancestor until we reach the LCCA
            // (the first ancestor that contains the target as a descendant).
            // For flat machines parentOf returns null immediately, so this is a no-op.
            var ancestor = parentOf(source)
            while (ancestor != null) {
                if (isDescendantOf(target, ancestor)) {
                    // LCCA found — ancestor contains both source and target.
                    // Don't exit it (both sides live under it).
                    break
                }
                // Exit this ancestor (target is outside it).
                // W3C SCXML 3.13: type="external" transitions to an ancestor
                // exit-and-reenter the ancestor itself (ancestor == target case).
                onExit(ancestor)
                if (ancestor == target) break
                ancestor = parentOf(ancestor)
            }
            return
        }

        // Step 1: Find LCCA (Least Common Compound Ancestor)
        // W3C SCXML 3.13: Use transition source (where transition is defined)
        // for LCCA computation when available, instead of the leaf state.
        // This ensures correct exit sets for transitions defined on ancestor states.
        val lccaStart = transitionSource ?: source
        var lcca: S? = parentOf(lccaStart)
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

    // --- Delay Parsing Helper ---

    /**
     * W3C SCXML 6.2: Parse delay string (e.g., "500ms", "1s", "2.5s") to milliseconds.
     * Matches C++ SendSchedulingHelper::parseDelayString behavior.
     */
    protected fun parseDelay(delay: String): Long {
        val trimmed = delay.trim()
        if (trimmed.isEmpty()) return 0L
        return when {
            trimmed.endsWith("ms") -> {
                trimmed.dropLast(2).trim().toDoubleOrNull()?.toLong() ?: 0L
            }
            trimmed.endsWith("s") -> {
                val seconds = trimmed.dropLast(1).trim().toDoubleOrNull() ?: 0.0
                (seconds * 1000).toLong()
            }
            else -> trimmed.toDoubleOrNull()?.toLong() ?: 0L
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
