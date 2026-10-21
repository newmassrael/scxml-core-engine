// SCE-GENERATED — DO NOT EDIT
// source-hash: 048fe3b13b4ba344e3f60c0d3a93bf8f8568ab7bd254b4d4763d7e631895e2f6

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_host_http.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_host_http.scxml:22 :: _machine

package com.sce.integration.static_host_http

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticHostHttpState : State {
    data object Fetching : StaticHostHttpState
    data object Idle : StaticHostHttpState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticHostHttpEvent : Event {
    sealed interface Done : StaticHostHttpEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Fetch : Invoke
        }
    }
    sealed interface Error : StaticHostHttpEvent {
        data object Execution : Error
        sealed interface Invoke : Error {
            data object Self : Invoke
            data object Fetch : Invoke
        }
    }
    sealed interface Host : StaticHostHttpEvent {
        sealed interface Http : Host {
            sealed interface Fetch : Http {
                data object Request : Fetch
            }
        }
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticHostHttpPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticHostHttpDoneInvokeFetchPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.fetch`. Consumers inject it via the `raiseDoneInvokeFetch` seam
// on the machine — they never name this class directly.
data class StaticHostHttpDoneInvokeFetchPayload(val error: String, val status: UShort, val headers: String, val body: String)

// StaticHostHttpHostHttpFetchRequestPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `host.http.fetch.request`. Consumers inject it via the `raiseHostHttpFetchRequest` seam
// on the machine — they never name this class directly.
data class StaticHostHttpHostHttpFetchRequestPayload(val method: String, val url: String, val headers: String, val body: String)


// --- State Machine (W3C SCXML) ---

// ── SCE Accepted Subset 2.12: typed host-run invoke interface ─────────
/** `sce:request`: the record the host is asked to start `<invoke id="fetch">` with (SCE Accepted Subset §2.12). */
data class StaticHostHttpFetchRequest(
    val method: String,
    val url: String,
    val headers: String,
    val body: String,
)

/** `sce:result`: the record the host completes `<invoke id="fetch">` with (SCE Accepted Subset §2.12). */
data class StaticHostHttpFetchResult(
    val error: String,
    val status: UShort,
    val headers: String,
    val body: String,
) {
    /** The JSON `done.invoke.fetch` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "status" to status, "headers" to headers, "body" to body))
}

/**
 * The host side of this document's typed `<invoke type="sce-host-http-fetch">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostHttpFetchInvoker`.
 */
interface StaticHostHttpSceHostHttpFetchInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="fetch">`. [token] names this start;
     * a host that finishes later hands it back to `completeFetch` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startFetch(request: StaticHostHttpFetchRequest, token: Long): StaticHostHttpFetchResult?

    /** §scxml-6.4: `<invoke id="fetch">`'s state exited while the start [token] names was still running. */
    fun cancelFetch(token: Long)
}


class StaticHostHttpStateMachine(
) : StateMachineEngine<StaticHostHttpState, StaticHostHttpEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `method` datamodel variable, the machine's own. */
    private var vMethod: String = ""
    /** W3C SCXML 5.2: the `url` datamodel variable, the machine's own. */
    private var vUrl: String = ""
    /** W3C SCXML 5.2: the `request_headers` datamodel variable, the machine's own. */
    private var vRequestHeaders: String = ""
    /** W3C SCXML 5.2: the `request_body` datamodel variable, the machine's own. */
    private var vRequestBody: String = ""
    /** W3C SCXML 5.2: the `outcome` datamodel variable, published (`sce:direction="out"`). */
    private var vOutcome: String = ""

    /** What the published `outcome` holds. */
    val outcome: String get() = vOutcome
    /** W3C SCXML 5.2: the `status` datamodel variable, published (`sce:direction="out"`). */
    private var vStatus: UShort = 0.toUShort()

    /** What the published `status` holds. */
    val status: UShort get() = vStatus
    /** W3C SCXML 5.2: the `response_headers` datamodel variable, published (`sce:direction="out"`). */
    private var vResponseHeaders: String = ""

    /** What the published `response_headers` holds. */
    val responseHeaders: String get() = vResponseHeaders
    /** W3C SCXML 5.2: the `response_body` datamodel variable, published (`sce:direction="out"`). */
    private var vResponseBody: String = ""

    /** What the published `response_body` holds. */
    val responseBody: String get() = vResponseBody

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var method: String? = null
        var url: String? = null
        var requestHeaders: String? = null
        var requestBody: String? = null
        var outcome: String? = null
        var status: UShort? = null
        var responseHeaders: String? = null
        var responseBody: String? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.method?.let { vMethod = it }
        params.url?.let { vUrl = it }
        params.requestHeaders?.let { vRequestHeaders = it }
        params.requestBody?.let { vRequestBody = it }
        params.outcome?.let { vOutcome = it }
        params.status?.let { vStatus = it }
        params.responseHeaders?.let { vResponseHeaders = it }
        params.responseBody?.let { vResponseBody = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val outcome: String,
        val status: UShort,
        val responseHeaders: String,
        val responseBody: String,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticHostHttpState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        outcome = vOutcome,
        status = vStatus,
        responseHeaders = vResponseHeaders,
        responseBody = vResponseBody,
    )

    private val _snapshot = kotlinx.coroutines.flow.MutableStateFlow(
        Snapshot(emptySet(), currentData(), false)
    )

    /**
     * The machine as the host sees it: one [Snapshot] per completed macrostep
     * (W3C SCXML Appendix D), never a state between two microsteps.
     *
     * Compose integration: `val s by sm.snapshot.collectAsState()`
     */
    val snapshot: kotlinx.coroutines.flow.StateFlow<Snapshot>
        get() = _snapshot

    override fun onMacrostepComplete(truncated: Boolean) {
        _snapshot.value = Snapshot(activeConfiguration, currentData(), truncated)
    }

    // ── SCE Accepted Subset §2.15: saving this machine, restoring it ─────────

    /**
     * What a saved state of this document is bound to: a state saved from a
     * document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not, and one
     * saved at an earlier revision of a record schema is read with each field a
     * later revision added at its default.
     */
    val savedShape: SavedShape = SavedShape(
        first = "448e3c50f8d63de0f3a6240edee966889d1d15938aedd4a15fc2e2e733f46d5e",
        revised = "4ea95f1cb7f5532c4fb84d343bb273dae3034a36d54fa2c946a9b541f7508478",
        format2First = "448e3c50f8d63de0f3a6240edee966889d1d15938aedd4a15fc2e2e733f46d5e",
        format2Revised = "4ea95f1cb7f5532c4fb84d343bb273dae3034a36d54fa2c946a9b541f7508478",
        records = listOf(
        ),
        enums = listOf(
        ),
    )

    /**
     * This machine's whole state at the macrostep boundary it stands at —
     * every variable, the machine's own included, and where it stands — as
     * the `sce-saved-state` document every backend reads ([SavedState.toJson]).
     * Each delayed `<send>` still waiting is written as the moment it comes due
     * on the wall clock whose reading now is [wallNowMs], in milliseconds since
     * the Unix epoch.
     *
     * @throws StateRefusal for a machine that is not running, or whose last
     *   macrostep stopped at the microstep ceiling.
     */
    fun save(wallNowMs: Long): SavedState = savedState(
        savedShape,
        linkedMapOf(
            "method" to SavedValues.of(vMethod),
            "url" to SavedValues.of(vUrl),
            "request_headers" to SavedValues.of(vRequestHeaders),
            "request_body" to SavedValues.of(vRequestBody),
            "outcome" to SavedValues.of(vOutcome),
            "status" to SavedValues.of(vStatus),
            "response_headers" to SavedValues.of(vResponseHeaders),
            "response_body" to SavedValues.of(vResponseBody),
        ),
        wallNowMs,
    )

    /** [save] at the host's wall clock now. */
    fun save(): SavedState = save(SavedState.wallClockMs())

    /**
     * Stand this machine where [saved] left one, in place of [initialize]: no
     * `<onentry>` runs and no `<data>` is evaluated, since the saved run
     * already did both. Every value is read before any is written, so a
     * refused restore leaves the machine as it was.
     *
     * The delayed sends [saved] holds are armed against this machine's `clock`,
     * which is installed before a restore as before [initialize]; [wallNowMs]
     * is what time it is on the wall clock the saved `due`s were written
     * against. A send comes due when its saved moment does, and one already due
     * comes due now.
     *
     * @throws StateRefusal for a machine that has already started, a state
     *   saved from a document of another shape, a configuration that is not
     *   one of this document, or a value its variable's type cannot hold.
     */
    fun restore(saved: SavedState, wallNowMs: Long) {
        // The state as one of this machine: each record saved at an earlier
        // revision of its schema holds the defaults of the fields added since.
        val savedGrown = beginRestore(saved, savedShape)
        val saved1 = SavedValues.string(savedGrown.variable("method"), "method", 8)
        val saved2 = SavedValues.string(savedGrown.variable("url"), "url", 2048)
        val saved3 = SavedValues.string(savedGrown.variable("request_headers"), "request_headers", 2048)
        val saved4 = SavedValues.string(savedGrown.variable("request_body"), "request_body", 16384)
        val saved5 = SavedValues.string(savedGrown.variable("outcome"), "outcome", 24)
        val saved6 = SavedValues.uint16(savedGrown.variable("status"), "status")
        val saved7 = SavedValues.string(savedGrown.variable("response_headers"), "response_headers", 4096)
        val saved8 = SavedValues.string(savedGrown.variable("response_body"), "response_body", 16384)
        vMethod = saved1
        vUrl = saved2
        vRequestHeaders = saved3
        vRequestBody = saved4
        vOutcome = saved5
        vStatus = saved6
        vResponseHeaders = saved7
        vResponseBody = saved8
        enterSaved(savedGrown, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, StaticHostHttpState>> = listOf(
        Triple("sce-host-http-fetch", "fetch", StaticHostHttpState.Fetching),
    )

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDoneInvokeFetchPayload: StaticHostHttpDoneInvokeFetchPayload? = null
    private var pendingHostHttpFetchRequestPayload: StaticHostHttpHostHttpFetchRequestPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticHostHttpEvent, metadata: EventMetadata) {
        pendingDoneInvokeFetchPayload = null
        pendingHostHttpFetchRequestPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticHostHttpDoneInvokeFetchPayload -> pendingDoneInvokeFetchPayload = tp
            is StaticHostHttpHostHttpFetchRequestPayload -> pendingHostHttpFetchRequestPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticHostHttpEvent.Done.Invoke.Fetch) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokeFetchPayload = StaticHostHttpDoneInvokeFetchPayload(fields.string("error"), fields.uint16("status"), fields.string("headers"), fields.string("body"))
                } else if (event == StaticHostHttpEvent.Host.Http.Fetch.Request) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingHostHttpFetchRequestPayload = StaticHostHttpHostHttpFetchRequestPayload(fields.string("method"), fields.string("url"), fields.string("headers"), fields.string("body"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `host.http.fetch.request` — binds the event name and the payload field values in one call.
    fun raiseHostHttpFetchRequest(method: String, url: String, headers: String, body: String) {
        send(
            StaticHostHttpEvent.Host.Http.Fetch.Request,
            EventMetadata(
                type = "external",
                typedPayload = StaticHostHttpHostHttpFetchRequestPayload(method, url, headers, body),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("method" to method, "url" to url, "headers" to headers, "body" to body))
            )
        )
    }


    // SCE Accepted Subset 2.12: registration and typed completion for this
    // document's typed host-run invokes.

    /**
     * Complete `<invoke id="fetch">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completeFetch(token: Long, result: StaticHostHttpFetchResult): Boolean =
        completeHostInvoke("sce-host-http-fetch", "fetch", token, result.wire())

    /**
     * Register [invoker] as the handler for `type="sce-host-http-fetch"`.
     */
    fun registerSceHostHttpFetchInvoker(invoker: StaticHostHttpSceHostHttpFetchInvoker) {
        registerInvoker("sce-host-http-fetch") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "fetch" -> {
                        val typed = StaticHostHttpFetchRequest(
                            method = TypedRequest.string(start.params, start.invokeId, "method"),
                            url = TypedRequest.string(start.params, start.invokeId, "url"),
                            headers = TypedRequest.string(start.params, start.invokeId, "headers"),
                            body = TypedRequest.string(start.params, start.invokeId, "body"),
                        )
                        invoker.startFetch(typed, start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "fetch" -> {
                        invoker.cancelFetch(cancel.token)
                        null
                    }
                    else -> null
                }
            } else {
                null
            }
        }
    }


    override val initialState: StaticHostHttpState = StaticHostHttpState.Idle

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticHostHttpState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticHostHttpState, HistoryId>> =
            listOf(StateTarget(StaticHostHttpState.Idle))

        // W3C SCXML 3.13: fetching's transition 0, as the microstep reads it.
        val transitionFetchingAt0 = EnabledTransition<StaticHostHttpState, HistoryId>(
            StaticHostHttpState.Fetching,
            listOf(StateTarget(StaticHostHttpState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: fetching's transition 1, as the microstep reads it.
        val transitionFetchingAt1 = EnabledTransition<StaticHostHttpState, HistoryId>(
            StaticHostHttpState.Fetching,
            listOf(StateTarget(StaticHostHttpState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: fetching's transition 2, as the microstep reads it.
        val transitionFetchingAt2 = EnabledTransition<StaticHostHttpState, HistoryId>(
            StaticHostHttpState.Fetching,
            listOf(StateTarget(StaticHostHttpState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: fetching's transition 3, as the microstep reads it.
        val transitionFetchingAt3 = EnabledTransition<StaticHostHttpState, HistoryId>(
            StaticHostHttpState.Fetching,
            listOf(StateTarget(StaticHostHttpState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticHostHttpState, HistoryId>(
            StaticHostHttpState.Idle,
            listOf(StateTarget(StaticHostHttpState.Fetching)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticHostHttpState? = when (stateId) {
        "fetching" -> StaticHostHttpState.Fetching
        "idle" -> StaticHostHttpState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticHostHttpState): String = when (state) {
        is StaticHostHttpState.Fetching -> "fetching"
        is StaticHostHttpState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticHostHttpState): Int = when (state) {
        is StaticHostHttpState.Fetching -> 1
        is StaticHostHttpState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticHostHttpEvent? = when (name) {
        "done.invoke" -> StaticHostHttpEvent.Done.Invoke.Self
        "done.invoke.fetch" -> StaticHostHttpEvent.Done.Invoke.Fetch
        "error.execution" -> StaticHostHttpEvent.Error.Execution
        "error.invoke" -> StaticHostHttpEvent.Error.Invoke.Self
        "error.invoke.fetch" -> StaticHostHttpEvent.Error.Invoke.Fetch
        "host.http.fetch.request" -> StaticHostHttpEvent.Host.Http.Fetch.Request
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticHostHttpEvent): String? = when (event) {
        is StaticHostHttpEvent.Done.Invoke.Self -> "done.invoke"
        is StaticHostHttpEvent.Done.Invoke.Fetch -> "done.invoke.fetch"
        is StaticHostHttpEvent.Error.Execution -> "error.execution"
        is StaticHostHttpEvent.Error.Invoke.Self -> "error.invoke"
        is StaticHostHttpEvent.Error.Invoke.Fetch -> "error.invoke.fetch"
        is StaticHostHttpEvent.Host.Http.Fetch.Request -> "host.http.fetch.request"
    }

    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "fetch",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticHostHttpState,
        event: StaticHostHttpEvent?
    ): EnabledTransition<StaticHostHttpState, HistoryId>? = when (state) {
        is StaticHostHttpState.Fetching -> when {
            event is StaticHostHttpEvent.Done.Invoke.Fetch && pendingDoneInvokeFetchPayload != null && (pendingDoneInvokeFetchPayload!!.error == "") -> transitionFetchingAt0
            event is StaticHostHttpEvent.Done.Invoke.Fetch -> transitionFetchingAt1
            event is StaticHostHttpEvent.Error.Invoke.Fetch -> transitionFetchingAt2
            event is StaticHostHttpEvent.Error.Execution -> transitionFetchingAt3
            else -> null
        }
        is StaticHostHttpState.Idle -> when {
            event is StaticHostHttpEvent.Host.Http.Fetch.Request -> transitionIdleAt0
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_host_http.scxml:22 :: _machine
    override fun onEntry(state: StaticHostHttpState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticHostHttpState.Fetching -> {
                // SCE-MAP: static_host_http.scxml:46 :: fetching :: _state_body
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "fetching.${System.identityHashCode(this)}.fetch"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        // SCE Accepted Subset 2.12: `sce:request` makes the
                        // params a record, so each value is held to its field
                        // here, where the invocation starts; one that cannot
                        // be evaluated or does not fit starts nothing.
                        run {
                            val v: Any? = vMethod
                            hostInvokeParams["method"] =
                                (hostInvokeParams["method"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "method", v)
                        }
                        run {
                            val v: Any? = vUrl
                            hostInvokeParams["url"] =
                                (hostInvokeParams["url"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "url", v)
                        }
                        run {
                            val v: Any? = vRequestHeaders
                            hostInvokeParams["headers"] =
                                (hostInvokeParams["headers"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "headers", v)
                        }
                        run {
                            val v: Any? = vRequestBody
                            hostInvokeParams["body"] =
                                (hostInvokeParams["body"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "body", v)
                        }
                        run {
                            val v: Any? = (5000).toLong()
                            hostInvokeParams["_sce_deadline_ms"] =
                                (hostInvokeParams["_sce_deadline_ms"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "_sce_deadline_ms", v)
                        }
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-http-fetch",
                                invokeId = "fetch",
                                src = "",
                                params = hostInvokeParams,
                                eventData = if (hostInvokePayload.isEmpty()) "" else buildJsonFromParams(hostInvokePayload),
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StaticHostHttpEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StaticHostHttpState.Idle -> {
                // SCE-MAP: static_host_http.scxml:37 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_host_http.scxml:22 :: _machine
    override fun onExit(state: StaticHostHttpState) {
        when (state) {
            is StaticHostHttpState.Fetching -> {
                // SCE-MAP: static_host_http.scxml:46 :: fetching :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-http-fetch", "fetch")
            }
            is StaticHostHttpState.Idle -> {
                // SCE-MAP: static_host_http.scxml:37 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_host_http.scxml:22 :: _machine
    override fun executeTransitionContent(source: StaticHostHttpState, transitionIndex: Int) {
        when (source) {
        is StaticHostHttpState.Fetching -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_http.scxml:54 :: fetching :: _transition_0
                if (pendingDoneInvokeFetchPayload == null) {
                    return
                }

            vStatus = pendingDoneInvokeFetchPayload!!.status

            if (try { vResponseHeaders = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeFetchPayload!!.headers, 4096); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostHttpEvent.Error.Execution, "<assign location='response_headers'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vResponseBody = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeFetchPayload!!.body, 16384); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostHttpEvent.Error.Execution, "<assign location='response_body'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("done", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostHttpEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_http.scxml:60 :: fetching :: _transition_1
                if (pendingDoneInvokeFetchPayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeFetchPayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostHttpEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_host_http.scxml:63 :: fetching :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("deadline", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostHttpEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_host_http.scxml:66 :: fetching :: _transition_3

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("refused", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostHttpEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticHostHttpState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_http.scxml:38 :: idle :: _transition_0
                if (pendingHostHttpFetchRequestPayload == null) {
                    return
                }

            if (try { vMethod = com.sce.forge.runtime.SceChecked.bounded(pendingHostHttpFetchRequestPayload!!.method, 8); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostHttpEvent.Error.Execution, "<assign location='method'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vUrl = com.sce.forge.runtime.SceChecked.bounded(pendingHostHttpFetchRequestPayload!!.url, 2048); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostHttpEvent.Error.Execution, "<assign location='url'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vRequestHeaders = com.sce.forge.runtime.SceChecked.bounded(pendingHostHttpFetchRequestPayload!!.headers, 2048); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostHttpEvent.Error.Execution, "<assign location='request_headers'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vRequestBody = com.sce.forge.runtime.SceChecked.bounded(pendingHostHttpFetchRequestPayload!!.body, 16384); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostHttpEvent.Error.Execution, "<assign location='request_body'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
