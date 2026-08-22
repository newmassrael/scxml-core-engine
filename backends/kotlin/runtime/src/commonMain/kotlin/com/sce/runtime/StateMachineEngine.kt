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

// Process-wide counter for scriptSessionId allocation. Using hashCode() here
// would collide across instances (32-bit identity hash has no uniqueness
// guarantee), causing sibling StateMachineEngine instances to share the same
// script-engine session and clobber each other's datamodel variables.
@OptIn(ExperimentalAtomicApi::class)
private val scriptSessionIdCounter = AtomicLong(0)

/**
 * §scxml-5.10: Event metadata for _event system variable.
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
    val invokeId: String = "",
    // NL→IR Item C1 Path A (EventSchema MCU native lowering): the type-erased
    // typed `_event.data` payload carrier. For an event whose imported
    // EventSchema lowered a transition guard to a native comparison, the
    // generated per-event inject seam packs the typed payload data class here;
    // the generated `populateTypedPayload` override lifts it back into a typed
    // policy field the native guard reads. `null` for every untyped event, so
    // the script-engine baseline is byte-unchanged. The Kotlin twin of the Go
    // `EventMetadata.TypedPayload any` / C++ `EventWithMetadata.typedPayload`.
    val typedPayload: Any? = null
) {
    companion object {
        val EMPTY = EventMetadata()
        fun internal() = EventMetadata(type = "internal")

        /**
         * §scxml-5.10: an internal event carrying `_event.data`.
         *
         * `<send target="#_internal">` with `<param>` children queues a
         * payload exactly as an external send does; without this the
         * generated code had no way to attach one and dropped the params
         * silently. Mirrors the [platform] pair above.
         */
        fun internal(data: String) = EventMetadata(type = "internal", data = data)
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
/**
 * How many links an `error.*` chain may have before the engine stops feeding
 * it — see [StateMachineEngine.errorCascadeEvents].
 *
 * §scxml-3.12.2 says what to do with an error event nothing matches. It does
 * not say what to do when something *does* match it and that handler fails
 * too: the failure raises the same error, the same transition answers it, and
 * the machine has no way out. Nothing in the specification bounds that, so the
 * number is this engine's to choose, and it is the same hundred
 * [StateMachineEngine.drainEventlessAndInternal] already uses for the sibling
 * case of a macrostep that cannot finish.
 *
 * A hundred links is far past any repair strategy a document plausibly spells
 * (a handler that tries a fallback, then a second one, is three) and far short
 * of a number a host would wait through: measured 2026-08-19, the Python
 * engine ran 37,000 links a second on a two-line document, so an unattended
 * supervisor did not hang — it burned a core until it was killed.
 */
private const val MAX_ERROR_CASCADE_DEPTH = 100

/**
 * How many microsteps one macrostep may take before this engine stops taking
 * them — see [StateMachineEngine.truncatedMacrosteps].
 *
 * The specification defines a macrostep as a chain of microsteps ending in a
 * configuration where nothing is enabled by NULL and the internal queue is
 * empty, and its Principles and Constraints say in as many words that such a
 * chain need not exist: "A microstep always terminates. A macrostep may not. A
 * macrostep that does not terminate may be said to consist of an infinitely
 * long sequence of microsteps. This is currently allowed."
 *
 * So the ceiling is not conformance — it is this engine declining a document
 * the specification permits, which is exactly why the decline has to be
 * visible.
 *
 * One budget for the whole inner loop, not one per branch. An earlier draft
 * here budgeted the two branches separately, on the argument that an internal
 * `<raise>` chain is a different document from an eventless cycle. It is — but
 * a document that alternates the two is a third, and separate budgets are what
 * let that one past both: each branch keeps finding its own counter unspent.
 * Appendix D takes a microstep on an eventless transition *or* on an internal
 * event inside one loop, and the ceiling belongs where the loop is.
 *
 * Ten times [MAX_ERROR_CASCADE_DEPTH], and deliberately not equal to it. This
 * is the backstop; the cascade ceiling is a diagnostic that names the error a
 * handler keeps failing on, and a backstop that fires first makes that
 * diagnostic unreachable. Measured 2026-08-20: with both at a hundred, a
 * handler that raises one event of its own per link — two microsteps a link,
 * which is what a document that logs before it fails looks like — was cut at
 * fifty links by this ceiling and [errorCascadeEvents] never moved. The factor
 * of ten is the headroom that keeps the specific report reachable for a handler
 * raising up to eight events a link; a busier one is cut here instead, which is
 * coarser but still reported.
 */
private const val MAX_MACROSTEP_MICROSTEPS = 1000

abstract class StateMachineEngine<S : State, E : Event>(
    protected val scriptEngine: ScxmlScriptEngine? = null
) {

    // --- Script Engine Session (§scxml-B-1) ---

    /** Session ID for script engine scope isolation. */
    protected var scriptSessionId: String? = null
        private set

    /** Lazy initialization flag — generated ensureScriptEngine() sets this. */
    protected var scriptEngineInitialized: Boolean = false

    /**
     * §scxml-C-2-3: inbound BasicHTTP endpoint serving this machine.
     *
     * The address belongs to the deployment — whoever runs the HTTP listener
     * chooses where it listens — so the engine takes it from here rather than
     * guessing one, and publishes it as the processor's 'location' in
     * `_ioprocessors`. Leaving it empty means no BasicHTTP endpoint serves the
     * session, and no entry is published.
     *
     * Must be set before the first event is processed, since `_ioprocessors`
     * is populated once during lazy session setup.
     */
    var basicHttpAccessUri: String = ""

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

    // --- Active State Configuration (§scxml-5.9.2) ---

    /**
     * §scxml-5.9.2: Set of currently active state IDs for In() predicate.
     *
     * Tracks all active states including parallel region children.
     * Managed by generated [onEntry]/[onExit] code.
     *
     * Thread safety: Only accessed from the microstep coroutine
     * ([Dispatchers.Default] single-writer). Do not access from external threads.
     */
    protected val activeStateIds: MutableSet<String> = mutableSetOf()

    /**
     * §scxml-3.11: History state storage.
     * Maps history state ID to recorded active state IDs at time of parent exit.
     * Shallow history stores direct children; deep history stores leaf descendants.
     */
    protected val historyStore: MutableMap<String, List<String>> = mutableMapOf()

    /**
     * §scxml-3.11: Pre-transition active states snapshot.
     * Captured before exit phase so history recording sees the full configuration.
     * Matches C++ activeStatesBeforeTransition pattern.
     */
    protected var preTransitionActiveStates: Set<String> = emptySet()

    /**
     * §scxml-5.9.2: Check if a state is in the active configuration.
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

    // --- Event Metadata (§scxml-5.10) ---

    /** Internal wrapper pairing an event with its §scxml-5.10 metadata. */
    private data class QueuedEvent<E>(val event: E, val metadata: EventMetadata = EventMetadata.EMPTY)

    /**
     * §scxml-5.10: Metadata for the event currently being processed.
     * Set before processEvent/processNullEvent so that generated
     * setCurrentEventInScriptEngine() can read it.
     */
    protected var currentEventMetadata: EventMetadata = EventMetadata.EMPTY

    /**
     * NL→IR Item C1 Path A (EventSchema MCU native lowering): lift the dequeued
     * event's type-erased typed `_event.data` payload into the typed policy
     * field(s) the generated native guards read. Called at every point that
     * assigns [currentEventMetadata], so a stale payload from a prior event is
     * always cleared before the next microstep — a non-typed event (carrier
     * `null`) resets the generated fields to `null`, making every typed guard
     * fail. The base implementation is a no-op; only a generated machine that
     * lowered at least one typed guard overrides it. The Kotlin twin of the Go
     * policy's `PopulateEventMetadata` type-switch / the C11 pop loop's
     * `sm->pending_payload = evt.payload`.
     */
    protected open fun populateTypedPayload(metadata: EventMetadata) {}

    /**
     * §scxml-3.12.1: Event channel (FIFO, unbounded).
     *
     * Channel.UNLIMITED ensures [send] never blocks and never drops events.
     * Recreated on each [start] to support stop/start cycles.
     */
    private var eventChannel = Channel<QueuedEvent<E>>(Channel.UNLIMITED)

    /**
     * §scxml-3.12.1: Internal events from <raise> are processed
     * before external events. This queue is drained first each microstep.
     */
    private val internalEventQueue = ArrayDeque<QueuedEvent<E>>()

    /** External event queue for sync mode (C++ externalQueue_ pattern). */
    private val externalEventQueue = ArrayDeque<QueuedEvent<E>>()

    // --- HTTP Send (§scxml-C-2: BasicHTTP Event I/O Processor) ---

    /**
     * §scxml-C-2: HTTP send request descriptor.
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
     * §scxml-C-2: Callback for HTTP send dispatch.
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
     * §scxml-C-2: Dispatch an HTTP POST send action.
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
     * §scxml-C-2: Schedule a delayed HTTP POST send action.
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

    /**
     * The [SceClock] this engine judges `<send delay>` deadlines against —
     * §scxml-6.2.2's "how long the processor should wait before dispatching".
     *
     * Defaults to [MonotonicClock], which is the wall clock and what a
     * production host wants. Install a [ManualClock] instead to own time
     * outright — a simulation, a replay, a discrete-event scheduler and a
     * deterministic test all need the engine to advance only when they say so
     * — and then drive the machine with [advanceTimeMs].
     *
     * Must be installed before [initialize]: the entry configuration's
     * `<onentry>` can arm delayed sends, and swapping the clock under deadlines
     * already computed from another one would leave the queue holding two
     * incomparable time bases.
     */
    var clock: SceClock = MonotonicClock()
        set(value) {
            check(!syncMode) {
                "the clock must be installed before initialize(): this engine has " +
                    "already armed its entry configuration against the previous one, " +
                    "and deadlines from two clocks do not compare"
            }
            field = value
        }

    /**
     * The reading [clock] gave when the current turn began, or `null` between
     * turns. See [beginTurn].
     */
    private var turnNowMs: Long? = null

    /**
     * §scxml-3.13: what time it is, for everything this turn arms or judges.
     *
     * The clause executes a microstep's executable content as one unit, and a
     * macrostep as a chain of those, so "now" is a property of the turn the
     * engine is in rather than of the statement asking. Between turns there is
     * no turn to be a property of, and the host's queries
     * ([timeUntilNextScheduledMs], [nowMs]) read the clock live.
     */
    private fun engineElapsedMs(): Long = turnNowMs ?: clock.elapsedMs()

    /**
     * Open a turn: take the single [clock] reading everything inside it uses.
     *
     * Returns whether this call is the one that opened it, which [endTurn]
     * needs so a nested entry point does not close the outer turn early.
     *
     * §scxml-6.2.2 makes a delay the wait the DOCUMENT asks for — "how long
     * the processor should wait before dispatching the message". Time the host
     * spent descheduled between two statements of one microstep is not part of
     * any delay the document wrote, so it must not reach the deadline.
     *
     * Reading the clock per statement instead was two defects at once, both
     * measured on this backend 2026-08-20:
     *
     * - Two `<send delay>`s executed by one `<onentry>` took a reading each,
     *   so a host descheduled between them by more than the gap between their
     *   delays got the later send's deadline first — and the engine then
     *   dispatched, and the document's `<cancel>` arrived after the event it
     *   named. Which of two events the author ordered arrives first became a
     *   fact about the host's scheduler.
     * - The dispatch loop in [tick] re-read it on every pass, so a tick slow
     *   enough to cross the next deadline dispatched that entry too, then the
     *   one after it — the engine chasing deadlines its own slowness created,
     *   in a loop the host cannot get between.
     *
     * Neither is reachable from a clock that is read once per turn. The
     * virtual-clock backends never had either, which is why no fixture found
     * them: a clock that only moves when the host moves it cannot move between
     * two readings taken inside one turn.
     */
    private fun beginTurn(): Boolean {
        if (turnNowMs != null) return false
        turnNowMs = clock.elapsedMs()
        return true
    }

    /** Close a turn opened by [beginTurn]. */
    private fun endTurn(opened: Boolean) {
        if (opened) turnNowMs = null
    }

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
     * §scxml-3.1.2: external events no transition matched, and the most recent
     * of them. See [discardedExternalEvents] and [lastDiscardedEvent].
     */
    private var discardedExternalEventCount: Int = 0
    private var lastDiscarded: E? = null

    /**
     * W3C SCXML B.2.8.1: deliveries whose payload announced structure and that
     * the datamodel could not read as one, and the most recent of them. See
     * [undecodablePayloads] and [lastUndecodablePayload].
     */
    private var undecodablePayloadCount: Int = 0
    private var lastUndecodable: E? = null

    /**
     * §scxml-3.12.2: `error.*` events this engine raised that no transition
     * matched, and the most recent of them. See [unhandledErrorEvents] and
     * [lastUnhandledError].
     */
    private var unhandledErrorEventCount: Int = 0
    private var lastUnhandledErrorEvent: E? = null

    /**
     * §scxml-3.12.2: the drain is executing a transition an `error.*` event
     * selected, which is the state in which a newly raised error is a link in
     * a chain rather than a first failure, plus how long that chain is and
     * what the engine refused because of it. See [errorCascadeEvents].
     */
    private var handlingErrorEvent: Boolean = false
    private var errorCascadeDepth: Int = 0
    private var errorCascadeEventCount: Int = 0
    private var lastErrorCascade: E? = null

    /**
     * Macrosteps stopped at [MAX_MACROSTEP_MICROSTEPS] with the chain still
     * going, the state the drain was in when that last happened, and whether
     * the macrostep now in progress is already one of them. See
     * [truncatedMacrosteps].
     *
     * The flag exists because the drain is reached more than once per
     * macrostep; without it each caller would get a fresh budget and each
     * refusal would be counted separately. It is cleared where the algorithm
     * starts a macrostep, which is the external dequeue.
     */
    private var truncatedMacrostepCount: Int = 0
    private var lastTruncatedMacrostep: S? = null
    private var macrostepTruncated: Boolean = false

    /**
     * Microsteps taken by the macrostep now in progress, against
     * [MAX_MACROSTEP_MICROSTEPS].
     *
     * A field rather than a local, for the reason Appendix D's loop is one
     * loop: the eventless branch and the internal-event branch take turns
     * inside a single macrostep, so a counter that lives in either one alone
     * bounds only half of it.
     */
    private var macrostepMicrostepsTaken: Int = 0

    /**
     * §scxml-3.7: Mark that a top-level final state has been entered.
     *
     * Called from generated [onEntry] code. The actual [isInFinalState] flag
     * is deferred until [_currentState] is updated, so that observers never
     * see isInFinalState=true with a stale currentState value.
     */
    protected fun markFinalStateReached() {
        pendingFinalState = true
    }

    /**
     * §scxml-5.5 + 6.3.1: Stashed `<donedata>` payload for the top-level
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
     * §scxml-5.5 + 6.3.1: Readback for the parent's invoke completion
     * callback. Empty string when the terminal `<final>` had no `<donedata>`.
     */
    fun donedataAtFinal(): String = donedataAtFinal

    // --- Generated Code Overrides ---

    /**
     * Initial state of the state machine.
     *
     * §scxml-3.2: Resolved from the `initial` attribute.
     * Must be an atomic (leaf) state for processEvent to work correctly.
     */
    abstract val initialState: S

    /**
     * Whether driving this machine needs the scheduler to be polled.
     *
     * `true` when the document carries a delayed `<send>` — which the spec's
     * send section routes through the event scheduler — or an `<invoke>`d child
     * that does. It matters only in the synchronous mode
     * ([initialize] + [tick]): there `scheduleSend` records into a time-ordered
     * queue that nothing drains unless the host calls [tick], so a host that
     * never does waits forever with no error and no warning. Under
     * [start] each delayed send is its own coroutine and fires on its own, so
     * the host has nothing to arrange.
     *
     * `open` with a `false` default rather than `abstract` so a hand-written
     * machine keeps compiling; every generated machine declares it.
     */
    open val needsEventScheduler: Boolean = false

    /**
     * Pure function: determine transition result for (state, event) pair.
     *
     * Generated as exhaustive `when` expressions over state and event types.
     * No side effects — the engine handles exit/entry/action ordering.
     *
     * §scxml-3.12: Event processing algorithm.
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
     * Execute entry actions for a state, and give it the descendants Appendix D
     * says it is owed.
     *
     * §scxml-3.8: `<onentry>` executable content + initial child entry.
     *
     * [pathChild] is what tells Appendix D's two entry functions apart, and it
     * is the whole of the difference between them:
     *
     * - `null` — [state] is the entry TARGET, so `addDescendantStatesToEnter`
     *   applies: a compound state takes its default initial child and a
     *   `<parallel>` takes every region.
     * - non-null — [state] is merely an ANCESTOR on the way to a deeper target,
     *   and [pathChild] is the one of its children the entry set already holds.
     *   `addAncestorStatesToEnter` adds it WITHOUT its default; the single
     *   exception is a `<parallel>`, whose OTHER regions still take theirs
     *   because nothing is entering inside them.
     *
     * This replaced a `suppressChildEntry` flag that could only say "no
     * defaults at all". Measured 2026-08-15: with the flag set for a parallel
     * ancestor, its sibling regions were entered without descending, so a
     * region nothing was targeting inside never reached its initial child —
     * pinned by `integration_resources/ancestor_entry_is_not_default_entry/`.
     */
    abstract fun onEntry(state: S, pathChild: S? = null)


    /**
     * Execute exit actions for a state.
     *
     * §scxml-3.9: `<onexit>` executable content.
     */
    abstract fun onExit(state: S)

    /**
     * Execute transition actions for a (source, event) pair.
     *
     * §scxml-3.13: Executable content within `<transition>`.
     * Called between onExit(source) and onEntry(target).
     *
     * @param event null for eventless transitions
     */
    abstract fun executeTransitionActions(source: S, event: E?)

    // --- State Hierarchy (§scxml-3.3 / §scxml-3.4) ---

    /**
     * §scxml-3.3: Get the parent of a state in the hierarchy.
     *
     * Override in generated code with the actual state hierarchy.
     * Returns null for root states.
     */
    protected open fun parentOf(state: S): S? = null

    /**
     * §scxml-3.4: Check if [descendant] is a descendant of [ancestor].
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
     * §scxml-3.4: Check if a state is a parallel state.
     *
     * Override in generated code for state machines with parallel states.
     * Used to determine if sibling regions need re-entry after transitions.
     */
    protected open fun isParallelState(state: S): Boolean = false

    /**
     * §scxml-3.4: Get child regions of a parallel state.
     *
     * C++ getParallelRegions() pattern: returns direct child states of a parallel state.
     * Override in generated code for state machines with parallel states.
     */
    protected open fun getParallelRegions(state: S): List<S> = emptyList()

    /**
     * §scxml-3.13: Get document order index for exit order sorting.
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
     * are silently discarded (SM no longer processes events per §scxml-3.7).
     */
    fun send(event: E) {
        if (syncMode) {
            externalEventQueue.addLast(QueuedEvent(event))
        } else {
            eventChannel.trySend(QueuedEvent(event))
        }
    }

    /**
     * §scxml-5.10: Submit an event with metadata (type, data, sendid, etc.).
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
     * §scxml-3.2 / §scxml-3.4: Enter initial state configuration.
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
        // Every link here takes its defaults — `pathChild` stays null on
        // purpose. §scxml-D-addAncestorStatesToEnter is about a state on the way
        // to a target somebody NAMED; this chain is the opposite, a default
        // descent that codegen has already resolved (`initialState` is the leaf,
        // not the document's `initial`). Measured 2026-08-15: passing the next
        // link here suppressed `s0`'s `<initial>` transition content and W3C
        // test579 reached `fail` — the `<initial>`/history branch is exactly
        // what a default entry owes. The duplicate guard in `onEntry` makes the
        // later links no-ops.
        for (state in chain) {
            onEntry(state)
        }
    }

    /**
     * §scxml-3.4: Re-enter exited regions of an active parallel state.
     *
     * C++ executeMicrostep pattern: when a parallel state is still active but some of its
     * child regions were exited during the microstep, re-enter those regions with their
     * initial states.
     *
     * The region the entry set is descending into needs no exclusion here: the
     * ancestor walk that precedes every call enters the inactive ancestors
     * first, so by the time this runs that region is active and the loop below
     * skips it. §scxml-D-addDescendantStatesToEnter is then exactly what is
     * left — the regions with nothing entering inside them.
     *
     * @param parallelState the parallel state to check
     */
    private fun reenterParallelRegions(parallelState: S) {
        // C++ executeMicrostep pattern (lines 456-482):
        // Use getParallelRegions() to find child regions, re-enter only inactive ones.
        val regions = getParallelRegions(parallelState)
        for (region in regions) {
            val regionId = stateIdOf(region)
            if (regionId.isNotEmpty() && activeStateIds.contains(regionId)) continue

            // Region was exited — re-enter with entry actions
            onEntry(region)

            // §scxml-3.3: If region is compound, enter initial child
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
        // Default descent, so every link takes its defaults — see
        // `enterInitialConfiguration` for why `pathChild` stays null on a chain
        // nobody targeted.
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

            // §scxml-6.4: Execute deferred invokes after initial configuration
            executePendingInvokes()

            // §scxml-3.7: Only enter event loop if not already in final state
            // (child SMs may reach final state during drainEventlessAndInternal)
            if (!isInFinalState) {
                for (queued in eventChannel) {
                    if (isInFinalState) break
                    currentEventMetadata = queued.metadata
                    populateTypedPayload(queued.metadata)
                    processMicrostep(queued.event, queued.metadata)
                }
            }

            // §scxml-6.2: Cancel pending delayed sends on session termination
            // Per spec, terminated sessions must not deliver delayed events (test187)
            delayedSendJobs.values.forEach { it.cancel() }
            delayedSendJobs.clear()
        }
    }

    // --- Sync Execution API (C++ AOT StaticExecutionEngine pattern) ---

    /**
     * §scxml-3.2 / §scxml-3.3: Synchronous initialization (C++ initialize() pattern).
     *
     * Enters initial configuration, runs macrostep completion loop until stable.
     * No coroutines — completes immediately for simple state machines.
     * For delayed sends/invokes, follow with [tick] polling loop.
     */
    fun initialize() {
        syncMode = true
        // §scxml-3.13: entering the initial configuration is one turn, and the
        // `<onentry>` handlers it runs arm their `<send delay>`s against one
        // instant — see [beginTurn] for what reading the clock per `<send>`
        // did to two of them.
        val opened = beginTurn()
        try {
            enterInitialConfiguration()
            _currentState.value = activeLeafStatesInDocumentOrder().lastOrNull()
                ?: resolveLeafState(_currentState.value)
            flushPendingFinalState()

            // W3C SCXML Appendix D: hand over to the outer loop. The macrostep
            // completes on eventless transitions and internal events, then the
            // invokes for the states just entered run, and only then is anything
            // taken off the external queue — so an autoforward child is live for
            // every event onentry queued on the way in.
            runMainEventLoop()
        } finally {
            endTurn(opened)
        }
    }

    /**
     * §scxml-6.2: Single tick — poll scheduler, tick children, process events.
     * Call repeatedly for tests with delayed sends (C++ tick() pattern).
     */
    fun tick() {
        if (isInFinalState) return
        // §scxml-3.13: one turn, one reading. Everything below judges due
        // against the instant this tick began, and everything the macrosteps
        // below arm is measured from it — so a tick dispatches what was due
        // when the host called it, and cannot be extended by how long it
        // takes to run (see [beginTurn]).
        val opened = beginTurn()
        try {
            // §scxml-6.2: dispatch the due sends one macrostep apart rather than
            // queueing them together. `<cancel>` drops a send that has not been
            // delivered yet, and a host that ticked late holds several past their
            // fire times: queueing them all first makes every later one
            // undroppable before the earlier one's transitions have run. That is
            // how a settle timer — arm a long `<send delay>`, cancel it when the
            // short signal arrives first — delivers the event it was told to
            // cancel (measured 2026-08-19 on the Rust, Go and Python backends,
            // whose scheduler this one mirrors).
            while (promoteNextDueSend()) {
                runMainEventLoop()
                if (isInFinalState) {
                    cleanupCompletedInvokes()
                    return
                }
            }
            pollScheduledHttpSends()
            tickChildren()
            // §scxml-6.4's invokes are part of the main event loop and run
            // there, ahead of the external dequeue rather than after it.
            runMainEventLoop()
            cleanupCompletedInvokes()
        } finally {
            endTurn(opened)
        }
    }

    /**
     * How long until this machine next needs [tick], in milliseconds. `0` means
     * a send is due now; `null` means nothing is owed.
     *
     * `needsEventScheduler` tells a host *which* entry point to drive the
     * machine with. This tells it *when*, and a host that cannot ask has only
     * one move left: pick a polling interval — which cannot straddle two fire
     * times it was never told about.
     *
     * Always `null` outside sync mode: there [scheduleSend] launches a coroutine
     * that fires on its own, so the host is owed no wake-up at all. The two
     * modes disagree about who owns the clock, and this answers for whichever
     * one this engine is in.
     */
    fun timeUntilNextScheduledMs(): Long? {
        if (!syncMode) return null
        val next = (scheduledSends.firstOrNull()?.fireTimeMs)
            .let { sends ->
                val https = scheduledHttpSends.firstOrNull()?.fireTimeMs
                when {
                    sends == null -> https
                    https == null -> sends
                    else -> minOf(sends, https)
                }
            } ?: return null
        return maxOf(0L, next - engineElapsedMs())
    }

    /**
     * Move this engine's clock forward by [ms] and run whatever that made due
     * (§scxml-6.2).
     *
     * The host-owned twin of [tick]: `tick` asks a clock that moves on its own
     * what time it is, this one *sets* what time it is and then ticks. A
     * machine driven exclusively through here has no dependency on the load of
     * the machine it runs on — the same sequence of calls produces the same
     * configuration every time.
     *
     * Requires a [ManualClock] in [clock], because that is the only kind of
     * clock a host can move. Calling it against the default [MonotonicClock]
     * is a programming error, not a no-op: it means the caller believes it owns
     * time and it does not, and the events it is waiting for will arrive on a
     * schedule it did not choose.
     *
     * An invoked child shares its parent's clock instance, so one call moves
     * parent and child together and both read the same absolute time
     * (§scxml-6.4).
     */
    fun advanceTimeMs(ms: Long) {
        require(ms >= 0L) { "advanceTimeMs requires a non-negative delta: $ms" }
        val manual = clock as? ManualClock
            ?: error(
                "advanceTimeMs needs a ManualClock in `clock`; this engine has a " +
                    "${clock::class.simpleName}, whose time the host does not own. " +
                    "Install a ManualClock before initialize(), or drive this machine " +
                    "with tick() and timeUntilNextScheduledMs()"
            )
        manual.advance(ms)
        tick()
    }

    /**
     * This engine's current reading of [clock], in milliseconds since the
     * clock's origin.
     *
     * The absolute counterpart of [timeUntilNextScheduledMs]'s relative answer.
     * A host owning time through [ManualClock] uses it to say where in the run
     * it is; a host on the wall clock uses it to correlate an engine's
     * deadlines with its own log.
     */
    fun nowMs(): Long = engineElapsedMs()

    /**
     * §scxml-3.1.2: how many events this engine took off the external queue
     * and discarded because no transition in any active state matched them.
     *
     * Discarding is what the clause requires. This is the part the clause does
     * not cover: the host that queued the event cannot otherwise tell that
     * outcome from a handled one, because a self transition, a targetless
     * internal transition and a discard all leave the configuration alone.
     * Comparing the count across a drive turns "the machine ignored what I
     * sent" into something the program can see.
     *
     * The C++ Interpreter has answered this all along (`processEvent`'s
     * `TransitionResult.success` and `getStatistics().failedTransitions`); this
     * is the generated engines' side of the same question. Unlike
     * [timeUntilNextScheduledMs] it answers in both modes — the coroutine mode
     * owns the clock, but neither mode owns the host's choice of event.
     *
     * Counts external-queue events only: an internal `<raise>` that matches
     * nothing has both its ends inside the document.
     */
    fun discardedExternalEvents(): Int = discardedExternalEventCount

    /**
     * The most recent event [discardedExternalEvents] counted, or `null` while
     * that count is zero.
     *
     * A count says something went nowhere; this says which thing did, which is
     * the question a host debugging a stalled supervisor actually has.
     */
    fun lastDiscardedEvent(): E? = lastDiscarded

    /**
     * Record which W3C SCXML B.2.8.1 rung the payload just bound got.
     *
     * Called by generated code immediately after it binds `_event`, because
     * that is the only moment the rung is known. Four of the five readings are
     * the ladder working and are recorded by being ignored; the fifth is the
     * one a host is wrong about.
     */
    fun notePayloadReading(event: E, reading: PayloadReading) {
        if (reading == PayloadReading.Undecodable) {
            undecodablePayloadCount++
            lastUndecodable = event
        }
    }

    /**
     * W3C SCXML B.2.8.1: how many events arrived carrying a payload that
     * announced itself as structure and that the datamodel could not read as
     * one.
     *
     * The clause requires the fallback: content the processor cannot interpret
     * becomes a space-normalized string. What it does not require — and what
     * nothing here used to provide — is any way for the host that SENT that
     * payload to learn its fields have stopped existing. The document reads
     * `_event.data.field`, gets nothing, assigns nothing, and the run
     * continues; measured 2026-08-22 on three independent Lua implementations,
     * a payload in Lua's own table syntax silently emptied every variable the
     * receiving transition assigned, including the one that primes the next
     * session.
     *
     * This backend has four script engines behind one interface, and which one
     * a document gets is the embedder's constructor argument. That choice used
     * to decide whether a lost payload was knowable; now all four report it.
     *
     * Counts only the reading a host can act on. Prose delivered as text is
     * the ladder working (W3C test 562) and is not counted, because a
     * diagnostic that fires when nothing is wrong is one nobody reads.
     */
    fun undecodablePayloads(): Int = undecodablePayloadCount

    /**
     * The most recent event [undecodablePayloads] counted, or `null` while
     * that count is zero. A count says something was lost; this says which
     * delivery lost it.
     */
    fun lastUndecodablePayload(): E? = lastUndecodable

    /**
     * §scxml-3.12.2: how many `error.*` events this engine raised that no
     * transition in any active state answered.
     *
     * The clause requires the processor to signal its own failures as `error.*`
     * events on the internal queue, and says in the same breath that "they are
     * ignored if no transition is found that matches them". Being ignored is
     * the clause. Being unable to say it happened is not, and the difference
     * matters to exactly one party: the host, which did not write the document,
     * cannot see the failure anywhere in the configuration, and is the only one
     * positioned to do something about it. A supervisor driving a machine whose
     * `<assign>` silently fails every round reads a plausible state forever.
     *
     * This is the sibling of [discardedExternalEvents], and the two are
     * deliberately separate counts rather than one. That one stops at the
     * external queue because an author's unmatched `<raise>` has both ends
     * inside the document; an error event's sender is the engine, so the same
     * reasoning does not reach it. An author's `<raise>` that matches nothing
     * is still not counted here.
     *
     * An error the document *did* answer is not counted either — the document
     * dealt with it, and its handling is visible in the configuration the host
     * can already read. What this counts is only the silent case.
     *
     * Answers in both drive modes: unlike the external queue, which
     * [processMicrostep] and [processNextExternalEvent] each feed, the internal
     * queue has a single drain that both modes run.
     *
     * The C++ Interpreter has answered this all along, through
     * `getLastStateMachineError()` and the message it raises `error.execution`
     * with; this is the generated engines' side of it.
     */
    fun unhandledErrorEvents(): Int = unhandledErrorEventCount

    /**
     * The most recent `error.*` event [unhandledErrorEvents] counted, or `null`
     * while that count is zero.
     *
     * Which error it was narrows a silent failure from "something in this
     * machine is broken" to a class: `error.execution` is the document's own
     * executable content failing, `error.communication` is a `<send>` or
     * `<invoke>` that could not reach its target — two different repairs, and a
     * count alone separates neither.
     */
    fun lastUnhandledError(): E? = lastUnhandledErrorEvent

    /**
     * §scxml-3.12.2: how many `error.*` events this engine refused to queue
     * because the error handler that raised them had been failing for
     * [MAX_ERROR_CASCADE_DEPTH] links running.
     *
     * The clause says an unmatched error event is ignored, and
     * [unhandledErrorEvents] is that case. This is its opposite and its worse
     * half: the document *does* match the error, and the handler fails the
     * same way every time. The failure raises `error.execution`, the same
     * transition answers it, and the drain never empties. Nothing in the
     * clause covers it — it bounds what happens to an error nobody wants, not
     * an error everybody wants and nobody can handle.
     *
     * Left to run, that is not a hang: it is a core at 100% forever. Measured
     * 2026-08-19 on a two-line document, the Python engine turned 37,000 links
     * a second while its configuration never moved — the exact reading an
     * unattended supervisor takes as healthy. This engine stopped at
     * [drainEventlessAndInternal]'s iteration ceiling instead of spinning, and
     * said nothing about it: bounded and silent is the same signal as
     * unbounded to the host reading it.
     *
     * A document that fails five hundred times cleanly counts zero here: the
     * chain is measured from *handler to handler*, not from failure to
     * failure, and any other internal event resets it.
     */
    fun errorCascadeEvents(): Int = errorCascadeEventCount

    /**
     * The most recent `error.*` event [errorCascadeEvents] refused, or `null`
     * while that count is zero.
     *
     * Which error it was names the repair: `error.execution` is a handler
     * whose own executable content fails, `error.communication` one that
     * answers an unreachable target by talking to it again.
     */
    fun lastErrorCascadeEvent(): E? = lastErrorCascade

    /**
     * How many macrosteps this engine stopped short because their chain was
     * still going after [MAX_MACROSTEP_MICROSTEPS] microsteps.
     *
     * The clause says a macrostep ends in a configuration where nothing is
     * enabled by NULL and no internal event is left, and the specification's
     * Principles and Constraints add that a macrostep *may not terminate* and
     * that this "is currently allowed". A document with a cyclic eventless
     * transition is therefore not malformed, and neither is one whose
     * `<raise>` answers itself; both are documents whose macrostep is
     * infinite, and an engine that runs either to the letter never returns.
     *
     * Both are counted here, because they are the same fact to a host: the
     * macrostep it just drove did not reach a stable configuration. Which
     * chain it was is what [lastTruncatedMacrostepState] points at.
     *
     * This engine does not run either to the letter, and it was the quietest
     * of the seven about it: the eventless ceiling in
     * [drainEventlessAndInternal] was silent until 2026-08-20 and the internal
     * one until the round after. Bounded and silent is the same signal as
     * unbounded to the host reading it — [currentState] answers with a state
     * the document names and the call returned. The configuration behind that
     * answer is not the stable one the clause promises.
     *
     * A document whose chain is a hundred microsteps long and then settles
     * counts zero: the ceiling is on microsteps *taken*, and the macrostep is
     * only counted here when the loop still had work after them — a transition
     * enabled by NULL, or an event left on the internal queue. Long chains are
     * ordinary; endless ones are not.
     */
    fun truncatedMacrosteps(): Int = truncatedMacrostepCount

    /**
     * The state this engine was in when it last stopped a macrostep that way,
     * or `null` while [truncatedMacrosteps] is zero.
     *
     * Which state it was is the whole repair: an endless chain is a closed
     * walk through the state graph, and this names one state on it — the
     * source of the transition that was refused, or the state the drain was
     * standing in when it stopped taking internal events. The count alone says
     * a document somewhere cannot settle; this says where to look.
     */
    fun lastTruncatedMacrostepState(): S? = lastTruncatedMacrostep

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

    /**
     * C++ PullScheduler::popReadyEvent pattern — take the single earliest due
     * entry off the time-ordered queue, returning whether one was taken.
     *
     * One per call because [tick] runs a macrostep between them: a `<cancel>`
     * performed by an earlier send's transitions must still reach a later one
     * that has not been queued yet.
     */
    private fun promoteNextDueSend(): Boolean {
        val now = engineElapsedMs()
        if (scheduledSends.isEmpty() || scheduledSends.first().fireTimeMs > now) {
            return false
        }
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
        return true
    }

    /** Fire ready delayed HTTP sends (the spec's BasicHTTP event processor). */
    private fun pollScheduledHttpSends() {
        val now = engineElapsedMs()
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
     * Matches: checkEventlessTransitions() → executePendingInvokes() → external dequeue → loop
     */
    /**
     * W3C SCXML Appendix D `mainEventLoop` — the outer loop, and the only place
     * the sync entry points ([initialize], [tick]) express macrostep semantics.
     *
     * Appendix D names the external queue exactly once per iteration and it is
     * *after* `invoke(inv)`:
     *
     * ```
     * while running:
     *     while running and not macrostepDone:      # eventless + internal only
     *         ... selectEventlessTransitions() / internalQueue.dequeue() ...
     *     for state in statesToInvoke.sort(entryOrder):
     *         for inv in state.invoke.sort(documentOrder):
     *             invoke(inv)
     *     statesToInvoke.clear()
     *     if not internalQueue.isEmpty(): continue
     *     externalEvent = externalQueue.dequeue()
     * ```
     *
     * Folding the external drain into the macrostep-completion loop instead is
     * a different algorithm, not a shorter one. The invoked children do not
     * exist yet while that drain runs, so everything `<onentry>` queued for
     * this session on the way in is consumed with no `autoforward` child to
     * receive it — and there is no later point at which it is delivered. One
     * external event per iteration for the same reason: a state entered by
     * event N's transition must have its invokes started before N+1 comes off
     * the queue.
     */
    private fun runMainEventLoop() {
        while (true) {
            // W3C SCXML Appendix D: complete the macrostep on eventless
            // transitions and internal events alone.
            drainEventlessAndInternal()
            if (isInFinalState) break
            // §scxml-6.4: invokes for states entered during this macrostep.
            executePendingInvokes()
            // W3C SCXML Appendix D: invoking may have raised internal error
            // events (and a child that completed synchronously may already have
            // raised done.invoke); handle them before touching the external
            // queue.
            //
            // Not when this macrostep was already stopped at the ceiling: the
            // queue is non-empty because the drain refused it, so looping back
            // is a spin that takes no microstep, says nothing, and never ends.
            // Falling through to the external dequeue instead is what keeps a
            // machine inside an endless chain reachable at all — the event that
            // rescues it is on that queue, and the clause's priority would
            // otherwise hold it behind a chain that never ends.
            if (internalEventQueue.isNotEmpty() && !macrostepTruncated) continue
            if (externalEventQueue.isEmpty()) break
            processNextExternalEvent()
        }
    }

    /**
     * W3C SCXML Appendix D — take exactly one event off the external queue, run
     * the preliminary `<finalize>` / autoforward step against it, then select
     * transitions.
     */
    private fun processNextExternalEvent() {
        val queued = externalEventQueue.removeFirst()
        // Taking an event off the external queue is
        // where a macrostep begins, so it is where the previous one's ceiling
        // stops applying. A machine left inside an endless chain gets a full
        // budget for each event it is given, and each refusal is counted
        // separately — which is what tells a host that spins once from one
        // that spins on everything.
        //
        // Here and not at the entry to [runMainEventLoop], which reads like
        // the more general boundary and is not one: a machine whose chain was
        // refused would spend a whole budget re-walking it before it ever
        // looked at the event the host sent to get it out. The refused events
        // stay queued either way — this is where the budget that drains them
        // comes back.
        macrostepTruncated = false
        macrostepMicrostepsTaken = 0
        currentEventMetadata = queued.metadata
        populateTypedPayload(queued.metadata)
        executeFinalizeForChildEvent(queued.event)
        autoForwardEvent(queued.event, queued.metadata)
        // §scxml-3.1.2: discarding an event no transition matched is the rule;
        // being unable to say so is not part of the rule. The host that queued
        // this event is the one party that cannot see the outcome — a discard
        // leaves the configuration exactly as a self transition does — and the
        // party that got the event wrong. Counted for the external queue only:
        // an internal `<raise>` that matches nothing has both its ends inside
        // the document.
        if (!processOneEvent(queued.event)) {
            discardedExternalEventCount++
            lastDiscarded = queued.event
        }
        flushPendingFinalState()
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
        // §scxml-6.4: Cancel all active invokes
        for ((_, entry) in activeInvokes) {
            entry.child.stop()
            entry.monitorJob.cancel()
        }
        activeInvokes.clear()
        pendingInvokes.clear()
        // §scxml-B-1: Destroy script engine session
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
     * §scxml-3.12.1: Raise an internal event (processed before external events).
     *
     * Called from generated onEntry/onExit/executeTransitionActions code.
     * Always called from the microstep coroutine (single-threaded access).
     * Default metadata type = "internal" per §scxml-5.10.
     */
    protected fun raiseInternal(event: E) {
        enqueueInternal(event, EventMetadata.internal())
    }

    /**
     * The single point every internal event passes through, so the one
     * decision the engine makes about them is made once.
     *
     * An `error.*` event raised while an error handler is running is refused
     * once the chain reaches [MAX_ERROR_CASCADE_DEPTH] — see
     * [errorCascadeEvents] for why the engine is the one that has to stop it.
     * Only the engine's own error events are refused: an author's `<raise>`
     * inside an error handler is the document doing its job and rides the
     * queue like any other.
     */
    private fun enqueueInternal(event: E, metadata: EventMetadata) {
        // §scxml-3.12.2 names the error events this refuses; the clause itself
        // is silent on a handler that fails, which is why the ceiling is a
        // choice this engine documents rather than a rule it implements.
        val name = eventNameOf(event)
        if (handlingErrorEvent && name != null && isErrorEvent(name)) {
            errorCascadeDepth++
            if (errorCascadeDepth >= MAX_ERROR_CASCADE_DEPTH) {
                errorCascadeEventCount++
                lastErrorCascade = event
                return
            }
        }
        internalEventQueue.addLast(QueuedEvent(event, metadata))
    }

    /**
     * §scxml-5.10: Raise an internal event with explicit metadata.
     *
     * Used for platform events (done.state, error.*) and events carrying data.
     */
    protected fun raiseInternal(event: E, metadata: EventMetadata) {
        enqueueInternal(event, metadata)
    }

    /**
     * Raise an `error.*` event this engine itself produced, carrying what failed.
     *
     * The single way generated code raises a platform error, so the two things
     * such an event owes its reader are filled in one place rather than at each
     * site that happens to remember:
     *
     *  * `_event.type` is `"platform"`. The one-argument [raiseInternal] above
     *    defaults to `EventMetadata.internal()`, and most generated error sites
     *    called it — so an event the PROCESSOR raised was labelled as one the
     *    document asked for.
     *  * `_event.data` carries [message]. A document that answers
     *    `error.execution` was handed an empty string, so it could see THAT
     *    something failed and never WHAT.
     *
     * [sendId] is for the `<send>` failures the spec requires it on (test 332);
     * it stays blank everywhere else.
     *
     * [message] names the failing construct and is a literal formed at
     * generation time, never the script engine's own error text — the dynamic
     * detail belongs on the log line beside the call.
     */
    protected fun raisePlatformError(event: E, message: String, sendId: String = "") {
        // §scxml-3.12.2: the processor signals its own failures as `error.*`,
        // and "platforms MAY include additional information about the nature of
        // the error in the 'data' field". Cited in the body so the ledger binds
        // it to this symbol.
        enqueueInternal(
            event,
            EventMetadata(data = message, type = "platform", sendId = sendId)
        )
    }

    // --- Delayed Send Support ---

    /** Active delayed send jobs, keyed by sendid for cancellation. */
    private val delayedSendJobs = mutableMapOf<String, Job>()

    /**
     * §scxml-6.2: Schedule a delayed event send.
     *
     * @param sendId Identifier for cancellation via `<cancel>`
     * @param delayMs Delay in milliseconds
     * @param event Event to send after delay
     */
    protected fun scheduleSend(sendId: String, delayMs: Long, event: E) {
        scheduleSend(sendId, delayMs, event, EventMetadata.EMPTY)
    }

    /**
     * §scxml-6.2: Schedule a delayed event send with metadata.
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
     * §scxml-6.3: Cancel a delayed event send.
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
     * §scxml-6.4 (test187): Schedule a delayed send to parent.
     * Cancelled when child session stops (all delayedSendJobs are cancelled in stop()).
     */
    protected fun scheduleParentSend(sendId: String, delayMs: Long, eventName: String) {
        scheduleParentSend(sendId, delayMs, eventName, "")
    }

    /**
     * §scxml-6.4: Schedule a delayed send to parent with event data.
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

    // --- Invoke Support (§scxml-6.4) ---

    /**
     * §scxml-6.4: Completion signal for invoke monitoring.
     * Completes when this state machine reaches a top-level final state.
     * Reset on [stop] for stop/start reuse.
     */
    var completion: CompletableDeferred<Unit> = CompletableDeferred()
        private set

    /**
     * §scxml-6.4: Callback for child SMs to send events to parent.
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

    // --- Deferred Invoke Support (§scxml-6.4) ---

    /**
     * §scxml-6.4: Pending invoke to be executed at macrostep end.
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
     * §scxml-6.4: Defer an invoke for execution at macrostep end.
     *
     * Called from generated onEntry code instead of startInvoke directly.
     * The executor lambda captures the full invoke setup (child creation,
     * param passing, startInvoke call).
     */
    protected fun deferInvoke(state: S, invokeId: String, executor: () -> Unit) {
        pendingInvokes.add(PendingInvoke(invokeId, state, executor))
    }

    /**
     * §scxml-6.4: Cancel pending invokes for a state being exited.
     *
     * Called from generated onExit code. Removes any deferred invokes
     * for states that were entered but exited before macrostep end.
     */
    protected fun cancelPendingInvokesForState(state: S) {
        pendingInvokes.removeAll { it.state == state }
    }

    /**
     * §scxml-6.4: Execute all pending invokes at macrostep end.
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
     * §scxml-6.4: Start an invoked child state machine.
     *
     * @param invokeId Invoke session identifier
     * @param child Child state machine instance
     * @param autoforward Forward parent events to child
     * @param doneEvent Event to send when child completes (done.invoke)
     */
    /**
     * §scxml-6.4: Set invoke parameters on a child SM before starting it.
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
     * §scxml-6.4: Pending invoke parameters to be applied during script engine init.
     * Set by parent's setInvokeParams, consumed by child's ensureScriptEngine.
     */
    protected var pendingInvokeParams: Map<String, Any?> = emptyMap()

    /**
     * §scxml-6.4: Start an invoked child state machine.
     *
     * @param invokeId Static invoke element ID — used for activeInvokes key, done.invoke metadata, cancelInvoke
     * @param child Child state machine instance
     * @param autoforward Forward parent events to child
     * @param doneEvent Event to send when child completes (done.invoke)
     * @param finalizeScript §scxml-6.5: Script to execute before child events are processed
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
        // §scxml-6.4: Set up child->parent event routing with metadata
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
            // §scxml-5.5 + 6.3.1: lift the child's stashed <donedata> onto
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

            // §scxml-6.4: the child's delayed sends are measured against the
            // same clock as ours. A child that read its own would start its
            // origin at construction time, so `<send delay="100ms">` on either
            // side of the boundary would mean two different absolute instants —
            // and on a host-owned clock the child would not move at all,
            // because the host advances the engine it holds, not the ones that
            // engine invoked.
            child.clock = clock
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
     * §scxml-6.4: Cancel an invoked child state machine on state exit.
     */
    protected fun cancelInvoke(invokeId: String) {
        activeInvokes.remove(invokeId)?.let {
            it.child.stop()
            it.monitorJob.cancel()
        }
    }

    /**
     * §scxml-6.4: Send event to invoked child by invoke ID.
     * Uses string-based routing for type-erased cross-SM communication.
     */
    protected fun sendToChild(invokeId: String, eventName: String) {
        activeInvokes[invokeId]?.child?.sendByName(eventName)
    }

    /**
     * §scxml-6.4: Send event by name (string-based, for cross-SM routing).
     * Internal: only used by parent SM's [sendToChild] for type-erased communication.
     */
    internal fun sendByName(name: String) {
        resolveEventByName(name)?.let { send(it) }
    }

    /**
     * §scxml-C-1: deliver an event addressed to a child's published location.
     *
     * Each invoked child owns a script session id, and that id is what the
     * child's `_ioprocessors` entry names, so a `<send>` whose target decodes to
     * one of them is addressed to that child rather than to this machine. A
     * `false` return means the address names no live child of ours and the event
     * takes the normal external path — the routing half of C.1 is what makes the
     * published location a usable target rather than a string that merely
     * compares equal.
     */
    protected fun deliverToChildSession(
        childSessionId: String,
        eventName: String,
        eventData: String = ""
    ): Boolean {
        if (childSessionId.isEmpty()) return false
        for ((_, entry) in activeInvokes) {
            if (entry.child.scriptSessionId == childSessionId) {
                entry.child.sendByNameWithData(eventName, eventData)
                return true
            }
        }
        return false
    }

    /**
     * §scxml-6.4 / C.1: type-erased injection carrying the send's payload.
     * [sendByName] drops event data because its callers have none; an addressed
     * `<send>` does, and losing it would make the delivery arrive stripped.
     */
    internal fun sendByNameWithData(name: String, data: String) {
        resolveEventByName(name)?.let {
            send(it, EventMetadata(type = "external", data = data))
        }
    }

    /**
     * §scxml-C-2: Send event by name with metadata (public, for HTTP callbacks).
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
     * §scxml-5.10: Build JSON object from evaluated param name/value pairs.
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

    /**
     * Record one `<send>` `<param>`, where a name may repeat.
     *
     * W3C's own test 178 sends two params of one name with different values and
     * requires BOTH pairs to be delivered; a map cannot hold one key twice, so
     * the second occurrence turns the entry into a list in document order.
     * Writing straight into the map — which every generated send used to do —
     * kept only the last value, silently. The wrapper class is what keeps that
     * distinct from a param whose OWN value is a list: appending to the value
     * would otherwise make `<param name="a" expr="[1,2]"/>` and two `a` params
     * indistinguishable.
     */
    protected class RepeatedParam(val values: MutableList<Any?>)

    protected fun putParam(params: MutableMap<String, Any?>, name: String, value: Any?) {
        val held = params[name]
        when {
            !params.containsKey(name) -> params[name] = value
            held is RepeatedParam -> held.values.add(value)
            else -> params[name] = RepeatedParam(mutableListOf(held, value))
        }
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
        is RepeatedParam -> {
            val items = value.values.joinToString(",") { valueToJson(it) }
            "[$items]"
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
     * §scxml-C-2: a value as the text a form-encoded param carries.
     *
     * The BasicHTTP Event I/O Processor sends each `<param>` as one
     * `name=value` pair, so the value crosses as text and the receiving end
     * hands that text to `_event.data`; no script engine reads it at either
     * end. [valueToJson] is the neighbour for a structured payload — it would
     * wrap a string in quotes that are not part of it — and an engine literal
     * is the other, which would put the sender's language on the wire.
     *
     * What this replaced was a bare `toString()`, which is the *platform's*
     * spelling: Rhino hands back a Double for every number, so a document
     * that sent `5` put `5.0` on the wire while the C++ channel sent `5` for
     * the same document. The rendering here is ECMAScript's `String(value)`,
     * with absence empty (§scxml-C-1) and a structured value as JSON —
     * matching C++ `ScriptResultUtils::resultToString` arm for arm.
     */
    protected fun valueToWireString(value: Any?): String = when (value) {
        null -> ""
        is Boolean -> value.toString()
        is Number -> {
            val d = value.toDouble()
            when {
                d.isNaN() -> "NaN"
                d == Double.POSITIVE_INFINITY -> "Infinity"
                d == Double.NEGATIVE_INFINITY -> "-Infinity"
                d == d.toLong().toDouble() -> d.toLong().toString()
                else -> d.toString()
            }
        }
        // Already text: quoting it would deliver characters the document
        // never wrote.
        is String -> value
        is Map<*, *>, is List<*>, is Array<*>, is RepeatedParam -> valueToJson(value)
        else -> value.toString()
    }

    /**
     * §scxml-6.4: Resolve event name string to Event object.
     * Override in generated code for cross-SM event routing.
     */
    protected open fun resolveEventByName(name: String): E? = null

    /**
     * §scxml-6.4: Resolve Event object to event name string.
     * Reverse of [resolveEventByName]. Override in generated code.
     * Used by autoforward to convert typed parent events to string names
     * for type-erased child routing.
     */
    protected open fun eventNameOf(event: E): String? = null

    // --- Finalize Support (§scxml-6.5) ---

    /**
     * §scxml-6.5: Execute finalize for events from invoked children.
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

        val engine = scriptEngine ?: error("scriptEngine is required for executeFinalizeForChildEvent (codegen invariant: state machine has active invokes ⇒ needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")

        for ((_, entry) in activeInvokes) {
            if (entry.finalizeScript.isNotEmpty() &&
                entry.child.scriptSessionId == metadata.origin) {
                // §scxml-6.5: Set _event before finalize execution
                val eventName = eventNameOf(event) ?: ""
                engine.setCurrentEvent(
                    sid,
                    SetCurrentEventArgs(
                        name = eventName,
                        data = metadata.data,
                        type = metadata.type,
                        sendId = metadata.sendId,
                        origin = metadata.origin,
                        originType = metadata.originType,
                        invokeId = metadata.invokeId
                    )
                )
                // §scxml-6.5: Execute finalize script
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
     * 1. Execute finalize for events from invoked children (§scxml-6.5)
     * 2. Auto-forward event to child sessions (§scxml-6.4.1)
     * 3. Process the external event
     * 4. Drain eventless transitions and internal events until stable
     * 5. Execute pending invokes at macrostep end (§scxml-6.4)
     * 6. Clean up completed invoke sessions
     */
    private fun processMicrostep(event: E, metadata: EventMetadata) {
        // This engine has two driving modes and this is the other one's
        // external dequeue: the coroutine mode reads its events from a channel
        // and never passes through [processNextExternalEvent], so the ceiling
        // has to be released here too. Without it a machine that truncated one
        // macrostep in coroutine mode would carry the spent budget into every
        // event after it and do nothing, forever, while reporting itself as
        // running.
        macrostepTruncated = false
        macrostepMicrostepsTaken = 0
        // §scxml-6.5: Execute finalize before event processing
        executeFinalizeForChildEvent(event)
        // §scxml-6.4: Auto-forward external events to child invoke sessions.
        // Only external events (from the event channel) reach processMicrostep;
        // internal events from <raise> are processed in drainEventlessAndInternal
        // and must NOT be forwarded per spec.
        autoForwardEvent(event, metadata)
        // §scxml-3.1.2: the coroutine mode's external events arrive here rather
        // than through [processNextExternalEvent], and a host driving that mode
        // is owed the same answer — this engine has two entry points for one
        // queue, so a count recorded at only one of them would be right for
        // half its callers.
        if (!processOneEvent(event)) {
            discardedExternalEventCount++
            lastDiscarded = event
        }
        drainEventlessAndInternal()
        // §scxml-6.4: Execute deferred invokes at macrostep end
        executePendingInvokes()
        // §scxml-6.5: Clean up completed invokes (deferred from monitor coroutine)
        cleanupCompletedInvokes()
    }

    /**
     * §scxml-6.5: Remove completed invoke entries from activeInvokes.
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

            // §scxml-3.8: Execute onexit actions for the final state before
            // notifying parent. Matches C++ AOT StaticExecutionEngine::initialize()
            // which calls executeOnExit(currentState_) for the final state only.
            // Ancestors are NOT exited here — transition exitHierarchy already
            // handled ancestor exits. This ensures child-to-parent events
            // (e.g., test236 SubFinal onexit) arrive before done.invoke.
            onExit(_currentState.value)

            // §scxml-6.4: Notify invoke monitors that this SM completed
            if (!completion.isCompleted) completion.complete(Unit)
            // §scxml-3.7: Close event channel to terminate event loop coroutine.
            // After reaching final state, no further events are processed.
            eventChannel.close()
        }
    }

    /**
     * Publish a macrostep this engine stopped short, from whichever branch of
     * Appendix D's inner loop ran out of budget.
     *
     * One function, two callers, for the reason the budget is one number: a
     * host reads a macrostep that did not reach a stable configuration, and
     * the branch it died in is a detail of the document, not of the contract.
     * Two copies of this would be two chances for one of them to stop setting
     * the flag that keeps the same chain from being handed a second budget —
     * which is exactly what the internal branch did here until 2026-08-20,
     * where it neither counted nor flagged and simply broke.
     */
    private fun recordTruncatedMacrostep(state: S) {
        truncatedMacrostepCount++
        lastTruncatedMacrostep = state
        macrostepTruncated = true
    }

    /**
     * Collect active atomic (leaf) states sorted by document order.
     *
     * §scxml-3.13: Document order determines transition priority
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
     * §scxml-3.4: For parallel states, ALL non-conflicting eventless
     * transitions are selected and fired in a single microstep. This matches
     * the W3C selectEventlessTransitions() algorithm where transitions in
     * different parallel regions execute simultaneously.
     */
    private fun drainEventlessAndInternal() {
        if (macrostepTruncated) {
            // This macrostep was already stopped at the ceiling. Re-entering
            // the drain would hand the same chain a second budget, which is
            // the runaway the ceiling exists to refuse.
            return
        }
        // Microsteps taken, not loop turns: the turn that finds nothing
        // enabled is how a macrostep ends, and counting it would spend the
        // budget on the proof that no budget was needed. One count for both
        // branches, on [macrostepMicrostepsTaken], because the macrostep is
        // one — see [MAX_MACROSTEP_MICROSTEPS] for what separate budgets let
        // through.
        while (!isInFinalState) {
            // W3C SCXML Appendix D: Eventless transitions take priority

            // W3C SCXML Appendix D: Unified eventless processing (C++ pattern)
            val leaves = activeLeafStatesInDocumentOrder()
            val enabledTransitions = mutableListOf<Pair<S, TransitionResult<S>>>()
            for (state in leaves) {
                val nullResult = processNullEvent(state)
                if (nullResult !is TransitionResult.Ignored) {
                    enabledTransitions.add(state to nullResult)
                }
            }
            if (enabledTransitions.isNotEmpty()) {
                if (macrostepMicrostepsTaken == MAX_MACROSTEP_MICROSTEPS) {
                    // The chain is still going one microstep past the budget,
                    // so this is the case the specification calls a macrostep
                    // that cannot end. Refuse the microstep rather than take it,
                    // and publish the refusal: the configuration left behind
                    // is not a stable one and only this counter says so.
                    recordTruncatedMacrostep(enabledTransitions[0].first)
                    break
                }
                if (enabledTransitions.size > 1) {
                    applySimultaneousTransitions(enabledTransitions)
                } else {
                    val (source, result) = enabledTransitions[0]
                    applyTransitionFrom(source, result, null)
                }
                macrostepMicrostepsTaken++
                flushPendingFinalState()
                continue
            }

            // §scxml-3.12.1: Internal events next
            if (internalEventQueue.isNotEmpty()) {
                if (macrostepMicrostepsTaken == MAX_MACROSTEP_MICROSTEPS) {
                    // Work is still queued one microstep past the budget, so
                    // this is the case the specification calls a macrostep
                    // that cannot end. Refuse the microstep rather than take
                    // it: the event stays on the queue, which is where the
                    // next macrostep will find it, and the count says the
                    // configuration a host reads now is not a stable one.
                    //
                    // Until 2026-08-20 this branch broke here and said
                    // nothing, which is the same signal as never returning to
                    // the host that has to read it.
                    recordTruncatedMacrostep(_currentState.value)
                    break
                }
                val queued = internalEventQueue.removeFirst()
                currentEventMetadata = queued.metadata
                populateTypedPayload(queued.metadata)
                // §scxml-3.12.2: the processor raises `error.*` into this
                // queue and the clause says they "are ignored if no transition
                // is found that matches them". Ignoring them is the clause;
                // staying silent about it is not.
                // [discardedExternalEvents] deliberately stops at the external
                // queue because an unmatched `<raise>` has both ends inside
                // the document — but the sender of an error event is this
                // engine, so that reasoning does not reach it. The host never
                // wrote the document, cannot see the failure in the
                // configuration, and is the only party able to act on it.
                //
                // The dispatch runs first and unconditionally: it is what
                // processes every internal event, and folding it into the
                // condition below would skip it for everything that is not an
                // error. This drain is the machine's only internal-queue path,
                // so unlike the external count there is one site, not two.
                // An error raised from here on is raised *by an error
                // handler*, which is the one situation the engine cannot leave
                // to the document: the handler that failed is the same one
                // that will answer the failure. The flag is what
                // [enqueueInternal] reads to tell that apart from a first
                // failure, and it is cleared before anything else can run so a
                // chain cannot be attributed to the wrong event.
                val name = eventNameOf(queued.event)
                val isError = name != null && isErrorEvent(name)
                // The chain is not ended by the drain doing something else. An
                // earlier draft reset the depth on every non-error event,
                // which reads as the careful choice and is the opposite: a
                // handler that raises its own event before failing — a
                // document that logs, then fails, which is most of them —
                // leaves the queue alternating `tick, error, tick, error…`,
                // and each `tick` put the ceiling back out of reach. The count
                // needs no such guard, because it only ever rises while an
                // error handler is running.
                handlingErrorEvent = isError
                val selected = processOneEvent(queued.event)
                handlingErrorEvent = false
                if (selected) {
                    // Appendix D: the loop turn that selects nothing takes no
                    // microstep, so it spends no budget. Only a turn that
                    // answered the event moved the machine, and only those are
                    // what a ceiling on microsteps can be counted in.
                    macrostepMicrostepsTaken++
                }
                if (!selected && isError) {
                    unhandledErrorEventCount++
                    lastUnhandledErrorEvent = queued.event
                }
                flushPendingFinalState()
                continue
            }

            // Stable: no eventless transitions, no internal events. The chain
            // — refused or merely finished — is over with the queue: a machine
            // whose next macrostep starts a new one starts it from zero, and
            // the count of what was refused stays where the host reads it.
            errorCascadeDepth = 0
            break
        }
    }

    /**
     * §scxml-D-microstepProcedure: Apply multiple non-conflicting transitions
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

            // §scxml-D-microstepProcedure, Step 1: Exit all in reverse document order
            for ((source, result) in sorted.reversed()) {
                exitHierarchy(source, result.target, result.transitionSource)
            }

            // §scxml-D-microstepProcedure, Step 2: Transition actions in document order
            for ((source, _) in sorted) {
                executeTransitionActions(source, null)
            }

            // §scxml-D-microstepProcedure, Step 3: Enter all targets in document order
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
     * §scxml-D-removeConflictingTransitions: For parallel state machines, collect ALL enabled
     * transitions from all active leaf states, remove conflicting transitions,
     * then execute as an atomic microstep.
     *
     * For non-parallel (single active leaf), first match wins.
     */
    private fun processOneEvent(event: E): Boolean {
        val leaves = activeLeafStatesInDocumentOrder()

        if (leaves.size <= 1) {
            // Non-parallel: simple first-match-wins (original behavior)
            for (state in leaves) {
                val result = processEvent(state, event)
                if (result !is TransitionResult.Ignored) {
                    applyTransitionFrom(state, result, event)
                    return true
                }
            }
            // §scxml-3.1.2: no transition matched, so the event is discarded.
            // Reported rather than merely done, so the external dequeue can
            // count it — see [discardedExternalEvents].
            return false
        }

        // §scxml-D-selectTransitions: Collect transitions from all active leaf states.
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

        // §scxml-3.1.2: nothing in any region answered, so the event is discarded.
        if (enabledTransitions.isEmpty() && internalTransitions.isEmpty()) return false

        // §scxml-3.13: Internal (targetless) transitions execute actions only.
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
            return true
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
        return true
    }

    /**
     * §scxml-D-removeConflictingTransitions: Remove conflicting transitions from the enabled set.
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
            val isTargetless: Boolean  // §scxml-5.9.2: targetless transitions never conflict
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
                // §scxml-5.9.2: Targetless transitions have no exit set.
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

                // §scxml-3.13: Check if exit sets overlap
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
                    // §scxml-3.13: Use transition source (where transition is defined)
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
     * §scxml-3.13: Compute LCCA (Least Common Compound Ancestor) of two states.
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
     * §scxml-D-microstepProcedure: Compute exit set -> Exit all -> Actions all -> Enter all.
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

            // §scxml-3.11: Capture active states before exit for history recording
            preTransitionActiveStates = activeStateIds.toSet()

            // §scxml-D-computeExitSet: Compute union exit set (C++ ParallelTransitionHelper pattern)
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
                        // §scxml-3.4: If active ancestor is parallel, re-enter exited regions
                        if (isParallelState(anc)) {
                            parallelAncToReenter = anc
                        }
                        break
                    }
                    anc = parentOf(anc)
                }
                ancestorsToEnter.reverse()

                // §scxml-D-addAncestorStatesToEnter: an ancestor is entered
                // WITHOUT its default initial child — the entry set already
                // holds the next link, which is the following ancestor or, for
                // the last one, the target itself.
                for ((i, ancestor) in ancestorsToEnter.withIndex()) {
                    onEntry(ancestor, ancestorsToEnter.getOrNull(i + 1) ?: target)
                }

                // C++ executeMicrostep: re-enter parallel sibling regions
                if (parallelAncToReenter != null) {
                    reenterParallelRegions(parallelAncToReenter!!)
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
     * §scxml-6.4.1: Forward external event to all autoforward child sessions.
     *
     * Matches C++ AOT StaticExecutionEngine::raiseExternal() which forwards
     * all events without platform event filtering. Child's resolveEventByName
     * silently ignores unrecognized events (e.g., done.invoke).
     */
    private fun autoForwardEvent(event: E, metadata: EventMetadata) {
        if (activeInvokes.isEmpty()) return

        val eventName = eventNameOf(event) ?: return

        for ((_, entry) in activeInvokes) {
            if (entry.autoforward) {
                // §scxml-6.4 requires an exact copy of the event to reach
                // the child, so the parent's `_event` fields travel with the
                // name: the name is the only identity the two machines share
                // (the child's Event type is unrelated). `typedPayload` is
                // dropped deliberately — it is bound to the parent's own event
                // type and the child re-hydrates from `data`.
                entry.child.sendEventByName(eventName, metadata.copy(typedPayload = null))
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

                // §scxml-3.11: Capture active states before exit for history recording
                preTransitionActiveStates = activeStateIds.toSet()

                // §scxml-3.13: Exit -> Transition Actions -> Entry
                // When transitionSource is set, use it for LCCA in the parallel path
                exitHierarchy(source, target, result.transitionSource)
                executeTransitionActions(source, event)

                // §scxml-3.13: Enter ancestors on path from LCCA to target
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
                        // §scxml-3.4: If active ancestor is parallel, re-enter exited regions
                        if (isParallelState(anc)) {
                            parallelAncestorToReenter = anc
                        }
                        break
                    }
                    anc = parentOf(anc)
                }
                ancestorsToEnter.reverse()

                // §scxml-D-addAncestorStatesToEnter: an ancestor is entered
                // WITHOUT its default initial child — the entry set already
                // holds the next link, which is the following ancestor or, for
                // the last one, the target itself.
                for ((i, ancestor) in ancestorsToEnter.withIndex()) {
                    onEntry(ancestor, ancestorsToEnter.getOrNull(i + 1) ?: target)
                }

                // C++ executeMicrostep: re-enter parallel sibling regions (full entry)
                if (parallelAncestorToReenter != null) {
                    reenterParallelRegions(parallelAncestorToReenter)
                }

                // Enter target with full entry (C++ executeEntryActions + initial child)
                onEntry(target)
                enterInitialChildrenIfNeeded(target)

                // §scxml-3.13: Resolve to actual active leaf (C++ pattern)
                val leafTarget = activeLeafStatesInDocumentOrder().lastOrNull()
                    ?: resolveLeafState(target)

                // Update observable state BEFORE flushing isInFinalState.
                _currentState.value = leafTarget
                // §scxml-3.7 + 6.4: Single path for final state + invoke completion
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
                // §scxml-3.13: Internal transition with target.
                // Exit descendants of transitionSource (but NOT the source itself),
                // execute transition actions, enter target.
                val target = result.target
                val txSource = result.transitionSource

                // §scxml-3.13: Exit active descendants of transitionSource (unified C++ pattern)
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

                // §scxml-3.3: Enter initial children (same as External case)
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
                // §scxml-3.13: type="internal" — actions only (targetless)
                executeTransitionActions(source, event)
            }
            is TransitionResult.Ignored -> {
                // §scxml-3.12: No matching transition, discard event
            }
        }
    }

    // --- Hierarchical Exit (§scxml-3.4 / §scxml-3.13) ---

    /**
     * §scxml-3.13: Exit states from source up to the LCCA with target.
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
        // §scxml-3.13: Unified exit (C++ StaticExecutionEngine pattern)
        // Step 1: Find LCCA (Least Common Compound Ancestor)
        // §scxml-3.13: Use transition source (where transition is defined)
        // for LCCA computation when available, instead of the leaf state.
        // This ensures correct exit sets for transitions defined on ancestor states.
        val lccaStart = transitionSource ?: source
        var lcca: S? = parentOf(lccaStart)
        while (lcca != null) {
            // §scxml-3.13: LCCA must be a PROPER ancestor of both source and target.
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
     * §scxml-6.2: Parse delay string (e.g., "500ms", "1s", "2.5s") to milliseconds.
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
     * KMP commonMain does not have System.nanoTime();
     * a monotonic counter is sufficient for transition ordering.
     * Platform-specific implementations can override with real timestamps.
     */
    private var sequenceCounter = 0L
    private fun nextTimestamp(): Long = sequenceCounter++
}
