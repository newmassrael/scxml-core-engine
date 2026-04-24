// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin Runtime — Abstract state machine engine

package com.sce.runtime

import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.concurrent.atomics.incrementAndFetch
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
import kotlin.time.TimeSource

// Process-wide counter for scriptSessionId allocation. Using hashCode() here
// would collide across instances (32-bit identity hash has no uniqueness
// guarantee), causing sibling StateMachineEngine instances to share the same
// script-engine session and clobber each other's datamodel variables.
@OptIn(ExperimentalAtomicApi::class)
private val scriptSessionIdCounter = AtomicLong(0)

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
    @OptIn(ExperimentalAtomicApi::class)
    protected fun allocateScriptSession(): String {
        val sid = "session_${scriptSessionIdCounter.incrementAndFetch()}"
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
     * W3C SCXML 3.11: History state storage.
     * Maps history state ID to recorded active state IDs at time of parent exit.
     * Shallow history stores direct children; deep history stores leaf descendants.
     */
    protected val historyStore: MutableMap<String, List<String>> = mutableMapOf()

    /**
     * C++ executeEntryActions vs onEntry separation flag.
     *
     * When true, [onEntry] executes entry actions only (adds to activeStateIds,
     * runs onentry content) WITHOUT entering initial/history children.
     * Matches C++ buildEntryChain pattern where ancestors are entered with
     * executeEntryActions() and only the target gets full child entry.
     *
     * Set by [applyTransitionFrom] during ancestor entry phase.
     */
    protected var suppressChildEntry: Boolean = false

    /**
     * W3C SCXML 3.11: Pre-transition active states snapshot.
     * Captured before exit phase so history recording sees the full configuration.
     * Matches C++ activeStatesBeforeTransition pattern.
     */
    protected var preTransitionActiveStates: Set<String> = emptySet()

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

    /** External event queue for sync mode (C++ externalQueue_ pattern). */
    private val externalEventQueue = ArrayDeque<QueuedEvent<E>>()

    // --- HTTP Send (W3C SCXML C.2: BasicHTTP Event I/O Processor) ---

    /**
     * W3C SCXML C.2: HTTP send request descriptor.
     *
     * Carries all data needed to build and dispatch an HTTP POST request
     * for BasicHTTPEventProcessor sends. Platform-agnostic — actual HTTP
     * dispatch is handled by the [onHttpSend] callback set by the test harness.
     */
    data class HttpSendRequest(
        val target: String,
        val eventName: String = "",
        val content: String = "",
        val params: Map<String, List<String>> = emptyMap(),
        val sendId: String = ""
    )

    /**
     * W3C SCXML C.2: Callback for HTTP send dispatch.
     *
     * Set by the test harness (W3CHttpTestBase) to route BasicHTTPEventProcessor
     * sends to an actual HTTP POST client. Matches C++ HttpSendHelper pattern
     * where the engine delegates HTTP dispatch to a platform-specific client.
     *
     * Volatile for visibility across threads (async mode safety).
     */
    @Volatile
    var onHttpSend: ((HttpSendRequest) -> Unit)? = null

    /**
     * W3C SCXML C.2: Dispatch an HTTP POST send action.
     *
     * Called by generated code when send type is BasicHTTPEventProcessor
     * and target is an HTTP URL. Delegates to [onHttpSend] callback.
     *
     * @param target HTTP target URL
     * @param eventName SCXML event name (becomes _scxmleventname param)
     * @param content Content body for content-only sends
     * @param params Form parameters map (multi-value)
     * @param sendId W3C SCXML sendid for correlation
     */
    protected fun performHttpSend(
        target: String,
        eventName: String,
        content: String,
        params: Map<String, List<String>>,
        sendId: String
    ) {
        onHttpSend?.invoke(HttpSendRequest(target, eventName, content, params, sendId))
    }

    /**
     * W3C SCXML C.2: Schedule a delayed HTTP POST send action.
     *
     * Stores the HTTP send request in the scheduled sends queue (sync mode)
     * and dispatches via [performHttpSend] when the delay expires during [tick].
     *
     * @param delayMs Delay in milliseconds before dispatching the HTTP POST
     */
    protected fun scheduleHttpSend(
        delayMs: Long,
        target: String,
        eventName: String,
        content: String,
        params: Map<String, List<String>>,
        sendId: String
    ) {
        val request = HttpSendRequest(target, eventName, content, params, sendId)
        val fireTime = engineElapsedMs() + delayMs
        val seqNum = schedulerSequence++
        scheduledHttpSends.add(ScheduledHttpSendEntry(fireTime, seqNum, sendId, request))
        scheduledHttpSends.sortWith(compareBy({ it.fireTimeMs }, { it.sequenceNum }))
    }

    private data class ScheduledHttpSendEntry(
        val fireTimeMs: Long,
        val sequenceNum: Long,
        val sendId: String,
        val request: HttpSendRequest
    )
    private val scheduledHttpSends = mutableListOf<ScheduledHttpSendEntry>()

    // --- Sync Execution (C++ AOT StaticExecutionEngine pattern) ---

    private var syncMode = false
    private val engineCreationMark = TimeSource.Monotonic.markNow()
    private fun engineElapsedMs(): Long = engineCreationMark.elapsedNow().inWholeMilliseconds

    private data class ScheduledSendEntry(
        val fireTimeMs: Long,
        val sequenceNum: Long,
        val sendId: String,
        val event: Any?,
        val metadata: EventMetadata,
        val isParentSend: Boolean = false,
        val parentEventName: String = "",
        val parentEventData: String = ""
    )
    private val scheduledSends = mutableListOf<ScheduledSendEntry>()
    private var schedulerSequence = 0L

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

    /**
     * W3C SCXML 5.5 + 6.3.1: Stashed `<donedata>` payload for the top-level
     * `<final>` that ended this machine. The parent invoker reads it back via
     * [donedataAtFinal] in [startInvoke]'s completion callback and threads it
     * onto the emitted `done.invoke.<id>._event.data`.
     *
     * C++ AOT parity: mirrors `StaticExecutionEngine::stashDonedataAtFinal` /
     * `donedataAtFinal()`. Compound-state finals emit `done.state.<parent>`
     * with the payload carried directly on the event metadata and so do not
     * route through this field.
     */
    private var donedataAtFinal: String = ""

    /**
     * Called from generated [onEntry] code when a top-level `<final>` with
     * `<donedata>` is entered. The evaluated payload is held on the engine
     * until the invoking parent emits `done.invoke.<id>`.
     */
    protected fun stashDonedataAtFinal(data: String) {
        donedataAtFinal = data
    }

    /**
     * W3C SCXML 5.5 + 6.3.1: Readback for the parent's invoke completion
     * callback. Empty string when the terminal `<final>` had no `<donedata>`.
     */
    fun donedataAtFinal(): String = donedataAtFinal

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
     * W3C SCXML 3.8: `<onentry>` executable content + initial child entry.
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
     * W3C SCXML 3.4: Check if a state is a parallel state.
     *
     * Override in generated code for state machines with parallel states.
     * Used to determine if sibling regions need re-entry after transitions.
     */
    protected open fun isParallelState(state: S): Boolean = false

    /**
     * W3C SCXML 3.4: Get child regions of a parallel state.
     *
     * C++ getParallelRegions() pattern: returns direct child states of a parallel state.
     * Override in generated code for state machines with parallel states.
     */
    protected open fun getParallelRegions(state: S): List<S> = emptyList()

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
        if (syncMode) {
            externalEventQueue.addLast(QueuedEvent(event))
        } else {
            eventChannel.trySend(QueuedEvent(event))
        }
    }

    /**
     * W3C SCXML 5.10: Submit an event with metadata (type, data, sendid, etc.).
     *
     * Used by generated send actions that need to attach event metadata.
     */
    fun send(event: E, metadata: EventMetadata) {
        if (syncMode) {
            externalEventQueue.addLast(QueuedEvent(event, metadata))
        } else {
            eventChannel.trySend(QueuedEvent(event, metadata))
        }
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
     * C++ buildEntryChain pattern: build ancestor chain from root to initialState,
     * then enter each state. onEntry for compound/parallel states handles recursive
     * descent into initial children and parallel regions.
     *
     * Override in generated code only for script engine initialization.
     */
    protected open fun enterInitialConfiguration() {
        // C++ HierarchicalStateHelper::buildEntryChain pattern:
        // Walk from initialState to root, reverse, enter each
        val chain = mutableListOf<S>()
        var cur: S? = initialState
        while (cur != null) {
            chain.add(cur)
            cur = parentOf(cur)
        }
        chain.reverse()
        for (state in chain) {
            onEntry(state)
        }
    }

    /**
     * W3C SCXML 3.4: Re-enter exited regions of an active parallel state.
     *
     * C++ executeMicrostep pattern: when a parallel state is still active but some of its
     * child regions were exited during the microstep, re-enter those regions with their
     * initial states.
     *
     * @param parallelState the parallel state to check
     * @param allTargets set of all transition targets (skip regions that are explicit targets)
     */
    private fun reenterParallelRegions(parallelState: S, allTargets: Set<S>) {
        // C++ executeMicrostep pattern (lines 456-482):
        // Use getParallelRegions() to find child regions, re-enter only inactive ones.
        val regions = getParallelRegions(parallelState)
        for (region in regions) {
            val regionId = stateIdOf(region)
            if (regionId.isNotEmpty() && activeStateIds.contains(regionId)) continue

            // Region was exited — re-enter with entry actions
            onEntry(region)

            // W3C SCXML 3.3: If region is compound, enter initial child
            enterInitialChildrenIfNeeded(region)
        }
    }

    private fun enterInitialChildrenIfNeeded(target: S) {
        val leaf = resolveLeafState(target)
        if (leaf == target) return
        // C++ pattern: check if onEntry already entered a child (history or parallel)
        val targetId = stateIdOf(target)
        val hasActiveChild = activeStateIds.any { stateId ->
            val st = resolveState(stateId) ?: return@any false
            val p = parentOf(st)
            p != null && stateIdOf(p) == targetId
        }
        if (hasActiveChild) return
        // C++ buildEntryChain: walk from leaf to target, reverse, enter each
        val intermediates = mutableListOf<S>()
        var cur: S? = leaf
        while (cur != null && cur != target) {
            intermediates.add(cur)
            cur = parentOf(cur)
        }
        intermediates.reverse()
        for (state in intermediates) {
            onEntry(state)
        }
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

            // Resolve to actual leaf state after initial configuration (C++ pattern)
            _currentState.value = activeLeafStatesInDocumentOrder().lastOrNull()
                ?: resolveLeafState(_currentState.value)

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

    // --- Sync Execution API (C++ AOT StaticExecutionEngine pattern) ---

    /**
     * W3C SCXML 3.2/3.3: Synchronous initialization (C++ initialize() pattern).
     *
     * Enters initial configuration, runs macrostep completion loop until stable.
     * No coroutines — completes immediately for simple state machines.
     * For delayed sends/invokes, follow with [tick] polling loop.
     */
    fun initialize() {
        syncMode = true
        enterInitialConfiguration()
        _currentState.value = activeLeafStatesInDocumentOrder().lastOrNull()
            ?: resolveLeafState(_currentState.value)
        flushPendingFinalState()

        // W3C SCXML C.1: Macrostep completion loop
        macrostepLoop()

        // W3C SCXML 6.4: Execute pending invokes after macrostep completes
        executePendingInvokes()

        // C++ pattern: process events raised by completed invokes
        macrostepLoop()
    }

    /**
     * W3C SCXML 6.2: Single tick — poll scheduler, tick children, process events.
     * Call repeatedly for tests with delayed sends (C++ tick() pattern).
     */
    fun tick() {
        if (isInFinalState) return
        pollScheduler()
        tickChildren()
        macrostepLoop()
        executePendingInvokes()
        cleanupCompletedInvokes()
    }

    /**
     * Destroy script engine session and release resources (sync mode cleanup).
     * Call after test assertions. Does not reset state — [currentState] remains readable.
     */
    fun cleanup() {
        scheduledSends.clear()
        scheduledHttpSends.clear()
        externalEventQueue.clear()
        for ((_, entry) in activeInvokes) {
            entry.child.cleanup()
        }
        activeInvokes.clear()
        pendingInvokes.clear()
        if (scriptEngineInitialized) {
            scriptSessionId?.let { scriptEngine?.destroySession(it) }
            scriptEngineInitialized = false
            scriptSessionId = null
        }
    }

    /** C++ PullScheduler::popReadyEvent pattern — pop ready events from time-ordered queue. */
    private fun pollScheduler() {
        val now = engineElapsedMs()
        while (scheduledSends.isNotEmpty() && scheduledSends.first().fireTimeMs <= now) {
            val entry = scheduledSends.removeAt(0)
            if (entry.isParentSend) {
                onSendToParent?.invoke(entry.parentEventName, entry.parentEventData)
            } else {
                // Justification (UNCHECKED_CAST): scheduledSends erases the
                // event type to Any to share the queue across parent-send and
                // self-send entries; the producer-side scheduleSend() only
                // accepts E, so the cast back to E is type-safe by
                // construction.
                @Suppress("UNCHECKED_CAST")
                externalEventQueue.addLast(QueuedEvent(entry.event as E, entry.metadata))
            }
        }
        // W3C SCXML C.2: Fire ready delayed HTTP sends
        while (scheduledHttpSends.isNotEmpty() && scheduledHttpSends.first().fireTimeMs <= now) {
            val entry = scheduledHttpSends.removeAt(0)
            performHttpSend(
                entry.request.target, entry.request.eventName,
                entry.request.content, entry.request.params, entry.request.sendId
            )
        }
    }

    /** C++ tickChildren pattern — tick all active child SMs. */
    private fun tickChildren() {
        for ((_, entry) in activeInvokes.toList()) {
            if (entry.child.isInFinalState) continue
            entry.child.tick()
            if (entry.child.isInFinalState) {
                entry.onComplete?.invoke()
            }
        }
    }

    /**
     * C++ initialize() macrostep loop: eventless + internal + external until stable.
     * Matches: checkEventlessTransitions() → processEventQueues() → loop
     */
    private fun macrostepLoop() {
        while (!isInFinalState) {
            drainEventlessAndInternal()
            if (externalEventQueue.isEmpty()) break
            // C++ processEventQueues: process external queue
            while (externalEventQueue.isNotEmpty() && !isInFinalState) {
                val queued = externalEventQueue.removeFirst()
                currentEventMetadata = queued.metadata
                executeFinalizeForChildEvent(queued.event)
                autoForwardEvent(queued.event)
                processOneEvent(queued.event)
                flushPendingFinalState()
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
        externalEventQueue.clear()
        scheduledSends.clear()
        syncMode = false
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
        if (syncMode) {
            // C++ PullScheduler pattern: record in time-ordered queue
            cancelSend(sendId)
            scheduledSends.add(ScheduledSendEntry(
                fireTimeMs = engineElapsedMs() + delayMs,
                sequenceNum = schedulerSequence++,
                sendId = sendId,
                event = event,
                metadata = metadata
            ))
            scheduledSends.sortWith(compareBy<ScheduledSendEntry> { it.fireTimeMs }.thenBy { it.sequenceNum })
        } else {
            val scope = engineScope ?: return
            delayedSendJobs[sendId]?.cancel()
            delayedSendJobs[sendId] = scope.launch(Dispatchers.Default) {
                kotlinx.coroutines.delay(delayMs)
                send(event, metadata)
                delayedSendJobs.remove(sendId)
            }
        }
    }

    /**
     * W3C SCXML 6.3: Cancel a delayed event send.
     *
     * @param sendId Identifier of the send to cancel
     */
    protected fun cancelSend(sendId: String) {
        if (syncMode) {
            scheduledSends.removeAll { it.sendId == sendId }
            scheduledHttpSends.removeAll { it.sendId == sendId }
        } else {
            delayedSendJobs.remove(sendId)?.cancel()
        }
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
        if (syncMode) {
            cancelSend(sendId)
            if (delayMs <= 0) {
                onSendToParent?.invoke(eventName, eventData)
            } else {
                scheduledSends.add(ScheduledSendEntry(
                    fireTimeMs = engineElapsedMs() + delayMs,
                    sequenceNum = schedulerSequence++,
                    sendId = sendId,
                    event = null,
                    metadata = EventMetadata.EMPTY,
                    isParentSend = true,
                    parentEventName = eventName,
                    parentEventData = eventData
                ))
                scheduledSends.sortWith(compareBy<ScheduledSendEntry> { it.fireTimeMs }.thenBy { it.sequenceNum })
            }
        } else {
            val scope = engineScope ?: return
            delayedSendJobs[sendId]?.cancel()
            delayedSendJobs[sendId] = scope.launch(Dispatchers.Default) {
                kotlinx.coroutines.delay(delayMs)
                onSendToParent?.invoke(eventName, eventData)
                delayedSendJobs.remove(sendId)
            }
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
        val finalizeScript: String = "",
        val onComplete: (() -> Unit)? = null
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
        // W3C SCXML 6.4: Set up child->parent event routing with metadata
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

        if (syncMode) {
            // C++ AOT pattern: synchronous child initialization.
            // W3C SCXML 5.5 + 6.3.1: lift the child's stashed <donedata> onto
            // done.invoke.<id>._event.data — mirrors the C++ AOT contract in
            // tools/codegen/templates/invoke_methods.jinja2 (child->donedataAtFinal()
            // + EventMetadataHelper::createDoneInvokeEvent). Reading the field
            // inside the closure (not at construction time) lets the generated
            // onEntry code stash after `activeInvokes[...]` is wired in.
            val onComplete = if (doneEvent != null) {
                {
                    externalEventQueue.addLast(QueuedEvent(doneEvent, EventMetadata(
                        type = "platform",
                        invokeId = invokeId,
                        data = child.donedataAtFinal()
                    )))
                }
            } else null

            child.initialize()
            activeInvokes[invokeId] = InvokeEntry(child, Job(), autoforward, finalizeScript, onComplete)

            // C++ pattern: if child completed immediately, raise done.invoke
            if (child.isInFinalState) {
                onComplete?.invoke()
            }
            return
        }

        val scope = engineScope ?: return
        child.start(scope)

        val monitorJob = scope.launch(Dispatchers.Default) {
            child.completion.await()
            if (doneEvent != null) {
                send(doneEvent, EventMetadata(
                    type = "platform",
                    invokeId = invokeId,
                    data = child.donedataAtFinal()
                ))
            }
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

    /**
     * W3C SCXML C.2: Send event by name with metadata (public, for HTTP callbacks).
     *
     * Used by test harness (W3CHttpTestBase) to inject HTTP response events
     * back into the SM by string name. Resolves name to typed Event via
     * [resolveEventByName] and dispatches with metadata.
     */
    fun sendEventByName(name: String, metadata: EventMetadata = EventMetadata.EMPTY) {
        resolveEventByName(name)?.let { send(it, metadata) }
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
     * Returns empty list only when no states are active (before start or after final).
     */
    private fun activeLeafStatesInDocumentOrder(): List<S> {
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
        var iterations = 0
        val maxIterations = 100  // C++ checkEventlessTransitions safety limit
        while (!isInFinalState && iterations++ < maxIterations) {
            // W3C SCXML Appendix D: Eventless transitions take priority
            var foundEventless = false

            // W3C SCXML Appendix D: Unified eventless processing (C++ pattern)
            val leaves = activeLeafStatesInDocumentOrder()
            val enabledTransitions = mutableListOf<Pair<S, TransitionResult<S>>>()
            for (state in leaves) {
                val nullResult = processNullEvent(state)
                if (nullResult !is TransitionResult.Ignored) {
                    enabledTransitions.add(state to nullResult)
                }
            }
            if (enabledTransitions.size > 1) {
                applySimultaneousTransitions(enabledTransitions)
                foundEventless = true
            } else if (enabledTransitions.size == 1) {
                val (source, result) = enabledTransitions[0]
                applyTransitionFrom(source, result, null)
                foundEventless = true
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
            // C++ pattern: onEntry (executeEntryActions) handles parallel region descent
            for ((_, result) in sorted) {
                onEntry(result.target)
            }

            // Update _currentState to last entered leaf
            _currentState.value = activeLeafStatesInDocumentOrder().lastOrNull()
                ?: resolveLeafState(sorted.last().second.target)
        }

        // Internal transitions: execute actions only (no state change)
        for ((source, _) in internals) {
            executeTransitionActions(source, null)
        }
    }

    /**
     * Process a single event (internal or external).
     *
     * W3C SCXML Appendix D.2: For parallel state machines, collect ALL enabled
     * transitions from all active leaf states, remove conflicting transitions,
     * then execute as an atomic microstep.
     *
     * For non-parallel (single active leaf), first match wins.
     */
    private fun processOneEvent(event: E) {
        val leaves = activeLeafStatesInDocumentOrder()

        if (leaves.size <= 1) {
            // Non-parallel: simple first-match-wins (original behavior)
            for (state in leaves) {
                val result = processEvent(state, event)
                if (result !is TransitionResult.Ignored) {
                    applyTransitionFrom(state, result, event)
                    return
                }
            }
            return
        }

        // W3C SCXML Appendix D.2: Collect transitions from all active leaf states.
        //
        // External/InternalToTarget: collect ALL and apply conflict resolution.
        // Internal (targetless): collect per leaf state for action execution.
        val enabledTransitions = mutableListOf<Pair<S, TransitionResult<S>>>()
        val internalTransitions = mutableListOf<Pair<S, TransitionResult<S>>>()
        for (state in leaves) {
            val result = processEvent(state, event)
            if (result !is TransitionResult.Ignored) {
                if (result is TransitionResult.Internal) {
                    internalTransitions.add(state to result)
                } else {
                    enabledTransitions.add(state to result)
                }
            }
        }

        if (enabledTransitions.isEmpty() && internalTransitions.isEmpty()) return

        // W3C SCXML D.2: Internal (targetless) transitions execute actions only.
        // For parallel states, execute each unique Internal transition's actions.
        // Dedup: if the generated executeTransitionActions for two different source states
        // both dispatch to the same ancestor's branch, the actions fire twice. To prevent
        // this, we use first-match-wins for Internal transitions — only the first leaf
        // in document order executes actions (it includes ancestor actions via effective_transitions).
        if (enabledTransitions.isEmpty()) {
            // Only Internal transitions — first-match-wins
            if (internalTransitions.isNotEmpty()) {
                val (source, result) = internalTransitions[0]
                applyTransitionFrom(source, result, event)
            }
            return
        }

        // Mix of External and Internal transitions
        // Add Internal transitions to the enabled set for simultaneous execution
        val allTransitions = enabledTransitions + internalTransitions
        val filtered = removeConflictingTransitions(allTransitions)
        if (filtered.size == 1) {
            val (source, result) = filtered[0]
            applyTransitionFrom(source, result, event)
        } else if (filtered.size > 1) {
            applySimultaneousTransitions(filtered, event)
        }
    }

    /**
     * W3C SCXML Appendix D.2: Remove conflicting transitions from the enabled set.
     *
     * Two transitions conflict if their exit sets overlap. The exit set of a
     * transition is the set of all states that would be exited by it:
     * - For external: all descendants of the LCCA (domain) of source and target
     * - For internal/targetless: empty
     *
     * When conflicts exist, the transition from the descendant source preempts
     * the ancestor. For same-depth siblings, document order wins.
     *
     * C++ ConflictResolutionHelper pattern.
     */
    private fun removeConflictingTransitions(
        transitions: List<Pair<S, TransitionResult<S>>>
    ): List<Pair<S, TransitionResult<S>>> {
        if (transitions.size <= 1) return transitions

        // Compute exit set (as domain state) for each transition
        data class TransitionWithDomain(
            val pair: Pair<S, TransitionResult<S>>,
            val domain: S?,  // LCCA of source and target (null = root domain OR targetless)
            val isTargetless: Boolean  // W3C SCXML 5.9.2: targetless transitions never conflict
        )

        val withDomains = transitions.map { pair ->
            val (source, result) = pair
            val domain: S? = when (result) {
                is TransitionResult.External -> {
                    // Domain = LCCA of transitionSource (or source) and target
                    val txSource = result.transitionSource ?: source
                    computeLCCA(txSource, result.target)
                }
                is TransitionResult.InternalToTarget -> {
                    result.transitionSource  // Domain is the transition source for internal-with-target
                }
                else -> null  // Targetless: no exit set, never conflicts
            }
            val isTargetless = result is TransitionResult.Internal
            TransitionWithDomain(pair, domain, isTargetless)
        }

        val result = mutableListOf<TransitionWithDomain>()
        for (candidate in withDomains) {
            if (candidate.isTargetless) {
                // W3C SCXML 5.9.2: Targetless transitions have no exit set.
                // They don't conflict with External transitions, pass through.
                // Dedup of same-ancestor targetless transitions is handled in processOneEvent
                // (Internal transitions use first-match-wins when no External transitions exist).
                result.add(candidate)
                continue
            }

            var dominated = false
            val toRemove = mutableListOf<TransitionWithDomain>()

            for (existing in result) {
                if (existing.isTargetless) continue

                // W3C SCXML D.2: Check if exit sets overlap
                // Exit sets overlap if one domain is ancestor-or-equal of the other's source,
                // or the domains overlap. Domain=null means root (exits everything).
                val candidateSource = candidate.pair.first
                val existingSource = existing.pair.first

                val conflict = if (candidate.domain == null || existing.domain == null) {
                    // Domain=null (root): conflicts with everything that has an exit set
                    true
                } else {
                    isDescendantOrSelf(existingSource, candidate.domain!!) ||
                    isDescendantOrSelf(candidateSource, existing.domain!!)
                }

                if (conflict) {
                    // W3C SCXML D.2: Use transition source (where transition is defined)
                    // for preemption, not the leaf state that was checked.
                    val candidateTxSource = when (val r = candidate.pair.second) {
                        is TransitionResult.External -> r.transitionSource ?: candidateSource
                        else -> candidateSource
                    }
                    val existingTxSource = when (val r = existing.pair.second) {
                        is TransitionResult.External -> r.transitionSource ?: existingSource
                        else -> existingSource
                    }

                    // Resolve: descendant transition source wins
                    if (isDescendantOf(candidateTxSource, existingTxSource)) {
                        // Candidate is more specific -> it preempts existing
                        toRemove.add(existing)
                    } else if (isDescendantOf(existingTxSource, candidateTxSource)) {
                        // Existing is more specific -> it preempts candidate
                        dominated = true
                        break
                    } else {
                        // Same level or siblings: document order of transition source — lower wins
                        if (documentOrderOf(existingTxSource) < documentOrderOf(candidateTxSource)) {
                            dominated = true
                            break
                        } else {
                            toRemove.add(existing)
                        }
                    }
                }
            }

            if (!dominated) {
                result.removeAll(toRemove)
                result.add(candidate)
            }
        }
        return result.map { it.pair }
    }

    /**
     * W3C SCXML 3.13: Compute LCCA (Least Common Compound Ancestor) of two states.
     */
    private fun computeLCCA(source: S, target: S): S? {
        // Collect ancestors of source
        val sourceAncestors = mutableListOf<S>()
        var anc: S? = parentOf(source)
        while (anc != null) {
            sourceAncestors.add(anc)
            anc = parentOf(anc)
        }

        // Walk up from target's parent, find first shared ancestor
        var tAnc: S? = parentOf(target)
        while (tAnc != null) {
            if (sourceAncestors.contains(tAnc)) return tAnc
            tAnc = parentOf(tAnc)
        }

        // Also check if source itself is ancestor of target
        anc = parentOf(target)
        while (anc != null) {
            if (anc == source) return parentOf(source)
            anc = parentOf(anc)
        }

        return null  // Root
    }

    private fun isDescendantOrSelf(state: S, possibleAncestor: S): Boolean {
        if (stateIdOf(state) == stateIdOf(possibleAncestor)) return true
        return isDescendantOf(state, possibleAncestor)
    }

    /**
     * Apply multiple non-conflicting event-based transitions as a single microstep.
     * W3C SCXML Appendix D.2: Compute exit set -> Exit all -> Actions all -> Enter all.
     *
     * C++ ParallelTransitionHelper::computeStatesToExit pattern:
     * The exit set is the union of all individual transitions' exit sets,
     * but only states that are NOT targets of any transition.
     */
    private fun applySimultaneousTransitions(
        transitions: List<Pair<S, TransitionResult<S>>>,
        event: E?
    ) {
        val externals = mutableListOf<Pair<S, TransitionResult.External<S>>>()
        val internals = mutableListOf<Pair<S, TransitionResult<S>>>()
        for ((source, result) in transitions) {
            when (result) {
                is TransitionResult.External -> externals.add(source to result)
                is TransitionResult.InternalToTarget -> {
                    externals.add(source to TransitionResult.External(result.target, result.transitionSource))
                }
                is TransitionResult.Internal -> internals.add(source to result)
                is TransitionResult.Ignored -> {}
            }
        }

        if (externals.isNotEmpty()) {
            val sorted = externals.sortedBy { documentOrderOf(it.first) }

            // W3C SCXML 3.11: Capture active states before exit for history recording
            preTransitionActiveStates = activeStateIds.toSet()

            // W3C SCXML Appendix D.2: Compute union exit set (C++ ParallelTransitionHelper pattern)
            // For each external transition, compute its individual exit set,
            // then union them. A state is in the exit set if it is a descendant
            // of the transition's domain AND it's currently active.
            val exitSet = mutableSetOf<String>()
            for ((source, result) in sorted) {
                val txSource = result.transitionSource ?: source
                // Compute domain (LCCA)
                var lcca: S? = parentOf(txSource)
                while (lcca != null) {
                    if (isDescendantOf(result.target, lcca)) break
                    lcca = parentOf(lcca)
                }
                // Add all active descendants of LCCA to exit set
                for (stateId in activeStateIds) {
                    if (lcca != null) {
                        val state = resolveState(stateId) ?: continue
                        if (state == lcca) continue
                        if (!isDescendantOf(state, lcca)) continue
                    }
                    exitSet.add(stateId)
                }
            }

            // Sort exit set by reverse document order
            val statesToExit = exitSet.mapNotNull { id ->
                resolveState(id)?.let { it to documentOrderOf(it) }
            }.sortedByDescending { it.second }

            // Step 1: Exit all in reverse document order
            for ((state, _) in statesToExit) {
                val sid = stateIdOf(state)
                if (sid.isNotEmpty() && activeStateIds.contains(sid)) {
                    onExit(state)
                }
            }

            // Step 2: Transition actions in document order
            for ((source, _) in sorted) {
                executeTransitionActions(source, event)
            }

            // Step 3: Enter all targets in document order
            // C++ executeMicrostep pattern: buildEntryChain + parallel region re-entry
            val allTargets = sorted.map { it.second.target }.toSet()
            for ((_, result) in sorted) {
                val target = result.target
                val ancestorsToEnter = mutableListOf<S>()
                var parallelAncToReenter: S? = null
                var anc = parentOf(target)
                while (anc != null) {
                    val ancId = stateIdOf(anc)
                    if (ancId.isNotEmpty() && !activeStateIds.contains(ancId)) {
                        ancestorsToEnter.add(anc)
                    } else {
                        // W3C SCXML 3.4: If active ancestor is parallel, re-enter exited regions
                        if (isParallelState(anc)) {
                            parallelAncToReenter = anc
                        }
                        break
                    }
                    anc = parentOf(anc)
                }
                ancestorsToEnter.reverse()

                // C++ buildEntryChain + executeEntryActions: ancestors with actions only
                suppressChildEntry = true
                for (ancestor in ancestorsToEnter) {
                    onEntry(ancestor)
                }
                suppressChildEntry = false

                // C++ executeMicrostep: re-enter parallel sibling regions
                if (parallelAncToReenter != null) {
                    reenterParallelRegions(parallelAncToReenter!!, allTargets)
                    parallelAncToReenter = null
                }

                // Enter target with full entry
                onEntry(target)
                enterInitialChildrenIfNeeded(target)
            }

            _currentState.value = activeLeafStatesInDocumentOrder().lastOrNull()
                ?: resolveLeafState(sorted.last().second.target)
            flushPendingFinalState()
        }

        // Internal transitions: execute actions only
        for ((source, _) in internals) {
            executeTransitionActions(source, event)
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

                // W3C SCXML 3.11: Capture active states before exit for history recording
                preTransitionActiveStates = activeStateIds.toSet()

                // W3C SCXML 3.13: Exit -> Transition Actions -> Entry
                // When transitionSource is set, use it for LCCA in the parallel path
                exitHierarchy(source, target, result.transitionSource)
                executeTransitionActions(source, event)

                // W3C SCXML 3.13: Enter ancestors on path from LCCA to target
                // C++ buildEntryChainFromParent: [ancestor1, ancestor2, ..., target]
                // All use same onEntry — duplicate guard prevents double initial child entry
                val ancestorsToEnter = mutableListOf<S>()
                var parallelAncestorToReenter: S? = null
                var anc = parentOf(target)
                while (anc != null) {
                    val ancId = stateIdOf(anc)
                    if (ancId.isNotEmpty() && !activeStateIds.contains(ancId)) {
                        ancestorsToEnter.add(anc)
                    } else {
                        // W3C SCXML 3.4: If active ancestor is parallel, re-enter exited regions
                        if (isParallelState(anc)) {
                            parallelAncestorToReenter = anc
                        }
                        break
                    }
                    anc = parentOf(anc)
                }
                ancestorsToEnter.reverse()

                // C++ buildEntryChain + executeEntryActions pattern:
                // Enter ancestors with actions only (no child entry)
                suppressChildEntry = true
                for (ancestor in ancestorsToEnter) {
                    onEntry(ancestor)
                }
                suppressChildEntry = false

                // C++ executeMicrostep: re-enter parallel sibling regions (full entry)
                if (parallelAncestorToReenter != null) {
                    reenterParallelRegions(parallelAncestorToReenter, setOf(target))
                }

                // Enter target with full entry (C++ executeEntryActions + initial child)
                onEntry(target)
                enterInitialChildrenIfNeeded(target)

                // W3C SCXML 3.13: Resolve to actual active leaf (C++ pattern)
                val leafTarget = activeLeafStatesInDocumentOrder().lastOrNull()
                    ?: resolveLeafState(target)

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

                // W3C SCXML 3.13: Exit active descendants of transitionSource (unified C++ pattern)
                // Target is included in exit set (will be re-entered)
                val statesToExit = mutableListOf<Pair<S, Int>>()
                for (stateId in activeStateIds.toList()) {
                    val state = resolveState(stateId) ?: continue
                    if (state == txSource) continue  // Don't exit the source itself
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

                executeTransitionActions(source, event)
                onEntry(target)

                // W3C SCXML 3.3: Enter initial children (same as External case)
                enterInitialChildrenIfNeeded(target)

                val leafTarget = activeLeafStatesInDocumentOrder().lastOrNull()
                    ?: resolveLeafState(target)
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
        // W3C SCXML 3.13: Unified exit (C++ StaticExecutionEngine pattern)
        // Step 1: Find LCCA (Least Common Compound Ancestor)
        // W3C SCXML 3.13: Use transition source (where transition is defined)
        // for LCCA computation when available, instead of the leaf state.
        // This ensures correct exit sets for transitions defined on ancestor states.
        val lccaStart = transitionSource ?: source
        var lcca: S? = parentOf(lccaStart)
        while (lcca != null) {
            // W3C SCXML 3.13: LCCA must be a PROPER ancestor of both source and target.
            // For external transitions to an ancestor, the ancestor itself is NOT the LCCA
            // (it must be exited and re-entered).
            if (isDescendantOf(target, lcca)) break
            lcca = parentOf(lcca)
        }

        // Step 2: Collect active states to exit
        // W3C SCXML: Exit set = all active states that are proper descendants of domain(t)
        // For external transitions, this includes the target if it's active (it will be re-entered)
        val statesToExit = mutableListOf<Pair<S, Int>>()
        for (stateId in activeStateIds.toList()) {
            val state = resolveState(stateId) ?: continue
            if (lcca != null) {
                // Normal case: exit all descendants of LCCA (but not LCCA itself)
                if (state == lcca) continue
                if (!isDescendantOf(state, lcca)) continue
            }
            // lcca == null: domain is implicit root — exit ALL active states
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
