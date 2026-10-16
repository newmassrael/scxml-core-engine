// SCE-GENERATED — DO NOT EDIT
// source-hash: 486a7ede17aec50321702f8e51d91329868b60553618b2452672e3f16a3437c4

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_host_notify.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_host_notify.scxml:23 :: _machine

package com.sce.integration.static_host_notify

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticHostNotifyState : State {
    data object Asking : StaticHostNotifyState
    data object Cancelling : StaticHostNotifyState
    data object Idle : StaticHostNotifyState
    data object Posting : StaticHostNotifyState
    data object PostingFar : StaticHostNotifyState
    data object Requesting : StaticHostNotifyState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticHostNotifyEvent : Event {
    sealed interface Done : StaticHostNotifyEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Ask : Invoke
            data object Cancel : Invoke
            data object Far : Invoke
            data object Post : Invoke
            data object Prompt : Invoke
        }
    }
    sealed interface Error : StaticHostNotifyEvent {
        data object Execution : Error
        sealed interface Invoke : Error {
            data object Ask : Invoke
            data object Cancel : Invoke
            data object Post : Invoke
            data object Prompt : Invoke
        }
    }
    sealed interface Host : StaticHostNotifyEvent {
        sealed interface Notify : Host {
            sealed interface Cancel : Notify {
                data object Request : Cancel
            }
            sealed interface Far : Notify {
                data object Request : Far
            }
            sealed interface Permission : Notify {
                data object Request : Permission
            }
            sealed interface Post : Notify {
                data object Request : Post
            }
            sealed interface RequestPermission : Notify {
                data object Request : RequestPermission
            }
        }
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticHostNotifyPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticHostNotifyDoneInvokeAskPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.ask`. Consumers inject it via the `raiseDoneInvokeAsk` seam
// on the machine — they never name this class directly.
data class StaticHostNotifyDoneInvokeAskPayload(val error: String, val state: String)

// StaticHostNotifyDoneInvokeCancelPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.cancel`. Consumers inject it via the `raiseDoneInvokeCancel` seam
// on the machine — they never name this class directly.
data class StaticHostNotifyDoneInvokeCancelPayload(val error: String, val existed: Boolean)

// StaticHostNotifyDoneInvokeFarPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.far`. Consumers inject it via the `raiseDoneInvokeFar` seam
// on the machine — they never name this class directly.
data class StaticHostNotifyDoneInvokeFarPayload(val error: String, val status: String)

// StaticHostNotifyDoneInvokePostPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.post`. Consumers inject it via the `raiseDoneInvokePost` seam
// on the machine — they never name this class directly.
data class StaticHostNotifyDoneInvokePostPayload(val error: String, val status: String)

// StaticHostNotifyDoneInvokePromptPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.prompt`. Consumers inject it via the `raiseDoneInvokePrompt` seam
// on the machine — they never name this class directly.
data class StaticHostNotifyDoneInvokePromptPayload(val error: String, val state: String)

// StaticHostNotifyHostNotifyCancelRequestPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `host.notify.cancel.request`. Consumers inject it via the `raiseHostNotifyCancelRequest` seam
// on the machine — they never name this class directly.
data class StaticHostNotifyHostNotifyCancelRequestPayload(val id: String)

// StaticHostNotifyHostNotifyPostRequestPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `host.notify.post.request`. Consumers inject it via the `raiseHostNotifyPostRequest` seam
// on the machine — they never name this class directly.
data class StaticHostNotifyHostNotifyPostRequestPayload(val id: String, val title: String, val body: String, val at_ms: Long)


// --- State Machine (W3C SCXML) ---

// ── SCE Accepted Subset 2.12: typed host-run invoke interface ─────────
/** `sce:request`: the record the host is asked to start `<invoke id="far">` with (SCE Accepted Subset §2.12). */
data class StaticHostNotifyFarRequest(
    val id: String,
    val title: String,
    val body: String,
    val at_ms: Long,
)

/** `sce:result`: the record the host completes `<invoke id="far">` with (SCE Accepted Subset §2.12). */
data class StaticHostNotifyFarResult(
    val error: String,
    val status: String,
) {
    /** The JSON `done.invoke.far` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "status" to status))
}

/** `sce:result`: the record the host completes `<invoke id="ask">` with (SCE Accepted Subset §2.12). */
data class StaticHostNotifyAskResult(
    val error: String,
    val state: String,
) {
    /** The JSON `done.invoke.ask` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "state" to state))
}

/** `sce:result`: the record the host completes `<invoke id="prompt">` with (SCE Accepted Subset §2.12). */
data class StaticHostNotifyPromptResult(
    val error: String,
    val state: String,
) {
    /** The JSON `done.invoke.prompt` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "state" to state))
}

/** `sce:request`: the record the host is asked to start `<invoke id="post">` with (SCE Accepted Subset §2.12). */
data class StaticHostNotifyPostRequest(
    val id: String,
    val title: String,
    val body: String,
    val at_ms: Long,
)

/** `sce:result`: the record the host completes `<invoke id="post">` with (SCE Accepted Subset §2.12). */
data class StaticHostNotifyPostResult(
    val error: String,
    val status: String,
) {
    /** The JSON `done.invoke.post` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "status" to status))
}

/** `sce:request`: the record the host is asked to start `<invoke id="cancel">` with (SCE Accepted Subset §2.12). */
data class StaticHostNotifyCancelRequest(
    val id: String,
)

/** `sce:result`: the record the host completes `<invoke id="cancel">` with (SCE Accepted Subset §2.12). */
data class StaticHostNotifyCancelResult(
    val error: String,
    val existed: Boolean,
) {
    /** The JSON `done.invoke.cancel` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "existed" to existed))
}

/**
 * The host side of this document's typed `<invoke type="sce-host-notify-cancel">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostNotifyCancelInvoker`.
 */
interface StaticHostNotifySceHostNotifyCancelInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="cancel">`. [token] names this start;
     * a host that finishes later hands it back to `completeCancel` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startCancel(request: StaticHostNotifyCancelRequest, token: Long): StaticHostNotifyCancelResult?

    /** §scxml-6.4: `<invoke id="cancel">`'s state exited while the start [token] names was still running. */
    fun cancelCancel(token: Long)
}

/**
 * The host side of this document's typed `<invoke type="sce-host-notify-permission">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostNotifyPermissionInvoker`.
 */
interface StaticHostNotifySceHostNotifyPermissionInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="ask">`. [token] names this start;
     * a host that finishes later hands it back to `completeAsk` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startAsk(token: Long): StaticHostNotifyAskResult?

    /** §scxml-6.4: `<invoke id="ask">`'s state exited while the start [token] names was still running. */
    fun cancelAsk(token: Long)
}

/**
 * The host side of this document's typed `<invoke type="sce-host-notify-post">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostNotifyPostInvoker`.
 */
interface StaticHostNotifySceHostNotifyPostInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="far">`. [token] names this start;
     * a host that finishes later hands it back to `completeFar` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startFar(request: StaticHostNotifyFarRequest, token: Long): StaticHostNotifyFarResult?

    /** §scxml-6.4: `<invoke id="far">`'s state exited while the start [token] names was still running. */
    fun cancelFar(token: Long)
    /**
     * §scxml-6.4: begin `<invoke id="post">`. [token] names this start;
     * a host that finishes later hands it back to `completePost` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startPost(request: StaticHostNotifyPostRequest, token: Long): StaticHostNotifyPostResult?

    /** §scxml-6.4: `<invoke id="post">`'s state exited while the start [token] names was still running. */
    fun cancelPost(token: Long)
}

/**
 * The host side of this document's typed `<invoke type="sce-host-notify-request-permission">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostNotifyRequestPermissionInvoker`.
 */
interface StaticHostNotifySceHostNotifyRequestPermissionInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="prompt">`. [token] names this start;
     * a host that finishes later hands it back to `completePrompt` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startPrompt(token: Long): StaticHostNotifyPromptResult?

    /** §scxml-6.4: `<invoke id="prompt">`'s state exited while the start [token] names was still running. */
    fun cancelPrompt(token: Long)
}


class StaticHostNotifyStateMachine(
) : StateMachineEngine<StaticHostNotifyState, StaticHostNotifyEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `id` datamodel variable, the machine's own. */
    private var vId: String = ""
    /** W3C SCXML 5.2: the `title` datamodel variable, the machine's own. */
    private var vTitle: String = ""
    /** W3C SCXML 5.2: the `body` datamodel variable, the machine's own. */
    private var vBody: String = ""
    /** W3C SCXML 5.2: the `at_ms` datamodel variable, the machine's own. */
    private var vAtMs: Long = 0
    /** W3C SCXML 5.2: the `outcome` datamodel variable, published (`sce:direction="out"`). */
    private var vOutcome: String = ""

    /** What the published `outcome` holds. */
    val outcome: String get() = vOutcome
    /** W3C SCXML 5.2: the `permission` datamodel variable, published (`sce:direction="out"`). */
    private var vPermission: String = ""

    /** What the published `permission` holds. */
    val permission: String get() = vPermission
    /** W3C SCXML 5.2: the `status` datamodel variable, published (`sce:direction="out"`). */
    private var vStatus: String = ""

    /** What the published `status` holds. */
    val status: String get() = vStatus
    /** W3C SCXML 5.2: the `existed` datamodel variable, published (`sce:direction="out"`). */
    private var vExisted: Boolean = false

    /** What the published `existed` holds. */
    val existed: Boolean get() = vExisted

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var id: String? = null
        var title: String? = null
        var body: String? = null
        var atMs: Long? = null
        var outcome: String? = null
        var permission: String? = null
        var status: String? = null
        var existed: Boolean? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.id?.let { vId = it }
        params.title?.let { vTitle = it }
        params.body?.let { vBody = it }
        params.atMs?.let { vAtMs = it }
        params.outcome?.let { vOutcome = it }
        params.permission?.let { vPermission = it }
        params.status?.let { vStatus = it }
        params.existed?.let { vExisted = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val outcome: String,
        val permission: String,
        val status: String,
        val existed: Boolean,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticHostNotifyState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        outcome = vOutcome,
        permission = vPermission,
        status = vStatus,
        existed = vExisted,
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
     * The shape a saved state of this document is bound to: a state saved
     * from a document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not.
     */
    val savedShape: String = "aacb68b4caadf48158b4802424684c04db0f2ce532b000706210688c7dbdc377"

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
            "id" to SavedValues.of(vId),
            "title" to SavedValues.of(vTitle),
            "body" to SavedValues.of(vBody),
            "at_ms" to SavedValues.of(vAtMs),
            "outcome" to SavedValues.of(vOutcome),
            "permission" to SavedValues.of(vPermission),
            "status" to SavedValues.of(vStatus),
            "existed" to SavedValues.of(vExisted),
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
        beginRestore(saved, savedShape)
        val saved1 = SavedValues.string(saved.variable("id"), "id", 64)
        val saved2 = SavedValues.string(saved.variable("title"), "title", 128)
        val saved3 = SavedValues.string(saved.variable("body"), "body", 512)
        val saved4 = SavedValues.int64(saved.variable("at_ms"), "at_ms")
        val saved5 = SavedValues.string(saved.variable("outcome"), "outcome", 24)
        val saved6 = SavedValues.string(saved.variable("permission"), "permission", 16)
        val saved7 = SavedValues.string(saved.variable("status"), "status", 16)
        val saved8 = SavedValues.bool(saved.variable("existed"), "existed")
        vId = saved1
        vTitle = saved2
        vBody = saved3
        vAtMs = saved4
        vOutcome = saved5
        vPermission = saved6
        vStatus = saved7
        vExisted = saved8
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, StaticHostNotifyState>> = listOf(
        Triple("sce-host-notify-post", "far", StaticHostNotifyState.PostingFar),
        Triple("sce-host-notify-permission", "ask", StaticHostNotifyState.Asking),
        Triple("sce-host-notify-request-permission", "prompt", StaticHostNotifyState.Requesting),
        Triple("sce-host-notify-post", "post", StaticHostNotifyState.Posting),
        Triple("sce-host-notify-cancel", "cancel", StaticHostNotifyState.Cancelling),
    )

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDoneInvokeAskPayload: StaticHostNotifyDoneInvokeAskPayload? = null
    private var pendingDoneInvokeCancelPayload: StaticHostNotifyDoneInvokeCancelPayload? = null
    private var pendingDoneInvokeFarPayload: StaticHostNotifyDoneInvokeFarPayload? = null
    private var pendingDoneInvokePostPayload: StaticHostNotifyDoneInvokePostPayload? = null
    private var pendingDoneInvokePromptPayload: StaticHostNotifyDoneInvokePromptPayload? = null
    private var pendingHostNotifyCancelRequestPayload: StaticHostNotifyHostNotifyCancelRequestPayload? = null
    private var pendingHostNotifyPostRequestPayload: StaticHostNotifyHostNotifyPostRequestPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticHostNotifyEvent, metadata: EventMetadata) {
        pendingDoneInvokeAskPayload = null
        pendingDoneInvokeCancelPayload = null
        pendingDoneInvokeFarPayload = null
        pendingDoneInvokePostPayload = null
        pendingDoneInvokePromptPayload = null
        pendingHostNotifyCancelRequestPayload = null
        pendingHostNotifyPostRequestPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticHostNotifyDoneInvokeAskPayload -> pendingDoneInvokeAskPayload = tp
            is StaticHostNotifyDoneInvokeCancelPayload -> pendingDoneInvokeCancelPayload = tp
            is StaticHostNotifyDoneInvokeFarPayload -> pendingDoneInvokeFarPayload = tp
            is StaticHostNotifyDoneInvokePostPayload -> pendingDoneInvokePostPayload = tp
            is StaticHostNotifyDoneInvokePromptPayload -> pendingDoneInvokePromptPayload = tp
            is StaticHostNotifyHostNotifyCancelRequestPayload -> pendingHostNotifyCancelRequestPayload = tp
            is StaticHostNotifyHostNotifyPostRequestPayload -> pendingHostNotifyPostRequestPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticHostNotifyEvent.Done.Invoke.Ask) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokeAskPayload = StaticHostNotifyDoneInvokeAskPayload(fields.string("error"), fields.string("state"))
                } else if (event == StaticHostNotifyEvent.Done.Invoke.Cancel) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokeCancelPayload = StaticHostNotifyDoneInvokeCancelPayload(fields.string("error"), fields.boolean("existed"))
                } else if (event == StaticHostNotifyEvent.Done.Invoke.Far) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokeFarPayload = StaticHostNotifyDoneInvokeFarPayload(fields.string("error"), fields.string("status"))
                } else if (event == StaticHostNotifyEvent.Done.Invoke.Post) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokePostPayload = StaticHostNotifyDoneInvokePostPayload(fields.string("error"), fields.string("status"))
                } else if (event == StaticHostNotifyEvent.Done.Invoke.Prompt) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokePromptPayload = StaticHostNotifyDoneInvokePromptPayload(fields.string("error"), fields.string("state"))
                } else if (event == StaticHostNotifyEvent.Host.Notify.Cancel.Request) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingHostNotifyCancelRequestPayload = StaticHostNotifyHostNotifyCancelRequestPayload(fields.string("id"))
                } else if (event == StaticHostNotifyEvent.Host.Notify.Post.Request) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingHostNotifyPostRequestPayload = StaticHostNotifyHostNotifyPostRequestPayload(fields.string("id"), fields.string("title"), fields.string("body"), fields.int64("at_ms"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `host.notify.cancel.request` — binds the event name and the payload field values in one call.
    fun raiseHostNotifyCancelRequest(id: String) {
        send(
            StaticHostNotifyEvent.Host.Notify.Cancel.Request,
            EventMetadata(
                type = "external",
                typedPayload = StaticHostNotifyHostNotifyCancelRequestPayload(id),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `host.notify.post.request` — binds the event name and the payload field values in one call.
    fun raiseHostNotifyPostRequest(id: String, title: String, body: String, at_ms: Long) {
        send(
            StaticHostNotifyEvent.Host.Notify.Post.Request,
            EventMetadata(
                type = "external",
                typedPayload = StaticHostNotifyHostNotifyPostRequestPayload(id, title, body, at_ms),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "title" to title, "body" to body, "at_ms" to at_ms))
            )
        )
    }


    // SCE Accepted Subset 2.12: registration and typed completion for this
    // document's typed host-run invokes.

    /**
     * Complete `<invoke id="cancel">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completeCancel(token: Long, result: StaticHostNotifyCancelResult): Boolean =
        completeHostInvoke("sce-host-notify-cancel", "cancel", token, result.wire())

    /**
     * Register [invoker] as the handler for `type="sce-host-notify-cancel"`.
     */
    fun registerSceHostNotifyCancelInvoker(invoker: StaticHostNotifySceHostNotifyCancelInvoker) {
        registerInvoker("sce-host-notify-cancel") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "cancel" -> {
                        val typed = StaticHostNotifyCancelRequest(
                            id = TypedRequest.string(start.params, start.invokeId, "id"),
                        )
                        invoker.startCancel(typed, start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "cancel" -> {
                        invoker.cancelCancel(cancel.token)
                        null
                    }
                    else -> null
                }
            } else {
                null
            }
        }
    }

    /**
     * Complete `<invoke id="ask">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completeAsk(token: Long, result: StaticHostNotifyAskResult): Boolean =
        completeHostInvoke("sce-host-notify-permission", "ask", token, result.wire())

    /**
     * Register [invoker] as the handler for `type="sce-host-notify-permission"`.
     */
    fun registerSceHostNotifyPermissionInvoker(invoker: StaticHostNotifySceHostNotifyPermissionInvoker) {
        registerInvoker("sce-host-notify-permission") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "ask" -> {
                        invoker.startAsk(start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "ask" -> {
                        invoker.cancelAsk(cancel.token)
                        null
                    }
                    else -> null
                }
            } else {
                null
            }
        }
    }

    /**
     * Complete `<invoke id="far">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completeFar(token: Long, result: StaticHostNotifyFarResult): Boolean =
        completeHostInvoke("sce-host-notify-post", "far", token, result.wire())

    /**
     * Complete `<invoke id="post">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completePost(token: Long, result: StaticHostNotifyPostResult): Boolean =
        completeHostInvoke("sce-host-notify-post", "post", token, result.wire())

    /**
     * Register [invoker] as the handler for `type="sce-host-notify-post"`.
     */
    fun registerSceHostNotifyPostInvoker(invoker: StaticHostNotifySceHostNotifyPostInvoker) {
        registerInvoker("sce-host-notify-post") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "far" -> {
                        val typed = StaticHostNotifyFarRequest(
                            id = TypedRequest.string(start.params, start.invokeId, "id"),
                            title = TypedRequest.string(start.params, start.invokeId, "title"),
                            body = TypedRequest.string(start.params, start.invokeId, "body"),
                            at_ms = TypedRequest.int64(start.params, start.invokeId, "at_ms"),
                        )
                        invoker.startFar(typed, start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    "post" -> {
                        val typed = StaticHostNotifyPostRequest(
                            id = TypedRequest.string(start.params, start.invokeId, "id"),
                            title = TypedRequest.string(start.params, start.invokeId, "title"),
                            body = TypedRequest.string(start.params, start.invokeId, "body"),
                            at_ms = TypedRequest.int64(start.params, start.invokeId, "at_ms"),
                        )
                        invoker.startPost(typed, start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "far" -> {
                        invoker.cancelFar(cancel.token)
                        null
                    }
                    "post" -> {
                        invoker.cancelPost(cancel.token)
                        null
                    }
                    else -> null
                }
            } else {
                null
            }
        }
    }

    /**
     * Complete `<invoke id="prompt">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completePrompt(token: Long, result: StaticHostNotifyPromptResult): Boolean =
        completeHostInvoke("sce-host-notify-request-permission", "prompt", token, result.wire())

    /**
     * Register [invoker] as the handler for `type="sce-host-notify-request-permission"`.
     */
    fun registerSceHostNotifyRequestPermissionInvoker(invoker: StaticHostNotifySceHostNotifyRequestPermissionInvoker) {
        registerInvoker("sce-host-notify-request-permission") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "prompt" -> {
                        invoker.startPrompt(start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "prompt" -> {
                        invoker.cancelPrompt(cancel.token)
                        null
                    }
                    else -> null
                }
            } else {
                null
            }
        }
    }


    override val initialState: StaticHostNotifyState = StaticHostNotifyState.Idle

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

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
    override val documentInitialTargets: List<EntryTarget<StaticHostNotifyState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticHostNotifyState, HistoryId>> =
            listOf(StateTarget(StaticHostNotifyState.Idle))

        // W3C SCXML 3.13: asking's transition 0, as the microstep reads it.
        val transitionAskingAt0 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Asking,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: asking's transition 1, as the microstep reads it.
        val transitionAskingAt1 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Asking,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: asking's transition 2, as the microstep reads it.
        val transitionAskingAt2 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Asking,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: asking's transition 3, as the microstep reads it.
        val transitionAskingAt3 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Asking,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: cancelling's transition 0, as the microstep reads it.
        val transitionCancellingAt0 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Cancelling,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: cancelling's transition 1, as the microstep reads it.
        val transitionCancellingAt1 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Cancelling,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: cancelling's transition 2, as the microstep reads it.
        val transitionCancellingAt2 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Cancelling,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: cancelling's transition 3, as the microstep reads it.
        val transitionCancellingAt3 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Cancelling,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Idle,
            listOf(StateTarget(StaticHostNotifyState.Asking)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Idle,
            listOf(StateTarget(StaticHostNotifyState.Requesting)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Idle,
            listOf(StateTarget(StaticHostNotifyState.Posting)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Idle,
            listOf(StateTarget(StaticHostNotifyState.Cancelling)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Idle,
            listOf(StateTarget(StaticHostNotifyState.PostingFar)),
            4,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: posting's transition 0, as the microstep reads it.
        val transitionPostingAt0 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Posting,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: posting's transition 1, as the microstep reads it.
        val transitionPostingAt1 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Posting,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: posting's transition 2, as the microstep reads it.
        val transitionPostingAt2 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Posting,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: posting's transition 3, as the microstep reads it.
        val transitionPostingAt3 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Posting,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: posting_far's transition 0, as the microstep reads it.
        val transitionPostingFarAt0 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.PostingFar,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: posting_far's transition 1, as the microstep reads it.
        val transitionPostingFarAt1 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.PostingFar,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: posting_far's transition 2, as the microstep reads it.
        val transitionPostingFarAt2 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.PostingFar,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: requesting's transition 0, as the microstep reads it.
        val transitionRequestingAt0 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Requesting,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: requesting's transition 1, as the microstep reads it.
        val transitionRequestingAt1 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Requesting,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: requesting's transition 2, as the microstep reads it.
        val transitionRequestingAt2 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Requesting,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: requesting's transition 3, as the microstep reads it.
        val transitionRequestingAt3 = EnabledTransition<StaticHostNotifyState, HistoryId>(
            StaticHostNotifyState.Requesting,
            listOf(StateTarget(StaticHostNotifyState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticHostNotifyState? = when (stateId) {
        "asking" -> StaticHostNotifyState.Asking
        "cancelling" -> StaticHostNotifyState.Cancelling
        "idle" -> StaticHostNotifyState.Idle
        "posting" -> StaticHostNotifyState.Posting
        "posting_far" -> StaticHostNotifyState.PostingFar
        "requesting" -> StaticHostNotifyState.Requesting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticHostNotifyState): String = when (state) {
        is StaticHostNotifyState.Asking -> "asking"
        is StaticHostNotifyState.Cancelling -> "cancelling"
        is StaticHostNotifyState.Idle -> "idle"
        is StaticHostNotifyState.Posting -> "posting"
        is StaticHostNotifyState.PostingFar -> "posting_far"
        is StaticHostNotifyState.Requesting -> "requesting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticHostNotifyState): Int = when (state) {
        is StaticHostNotifyState.Asking -> 2
        is StaticHostNotifyState.Cancelling -> 5
        is StaticHostNotifyState.Idle -> 0
        is StaticHostNotifyState.Posting -> 4
        is StaticHostNotifyState.PostingFar -> 1
        is StaticHostNotifyState.Requesting -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticHostNotifyEvent? = when (name) {
        "done.invoke" -> StaticHostNotifyEvent.Done.Invoke.Self
        "done.invoke.ask" -> StaticHostNotifyEvent.Done.Invoke.Ask
        "done.invoke.cancel" -> StaticHostNotifyEvent.Done.Invoke.Cancel
        "done.invoke.far" -> StaticHostNotifyEvent.Done.Invoke.Far
        "done.invoke.post" -> StaticHostNotifyEvent.Done.Invoke.Post
        "done.invoke.prompt" -> StaticHostNotifyEvent.Done.Invoke.Prompt
        "error.execution" -> StaticHostNotifyEvent.Error.Execution
        "error.invoke.ask" -> StaticHostNotifyEvent.Error.Invoke.Ask
        "error.invoke.cancel" -> StaticHostNotifyEvent.Error.Invoke.Cancel
        "error.invoke.post" -> StaticHostNotifyEvent.Error.Invoke.Post
        "error.invoke.prompt" -> StaticHostNotifyEvent.Error.Invoke.Prompt
        "host.notify.cancel.request" -> StaticHostNotifyEvent.Host.Notify.Cancel.Request
        "host.notify.far.request" -> StaticHostNotifyEvent.Host.Notify.Far.Request
        "host.notify.permission.request" -> StaticHostNotifyEvent.Host.Notify.Permission.Request
        "host.notify.post.request" -> StaticHostNotifyEvent.Host.Notify.Post.Request
        "host.notify.request_permission.request" -> StaticHostNotifyEvent.Host.Notify.RequestPermission.Request
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticHostNotifyEvent): String? = when (event) {
        is StaticHostNotifyEvent.Done.Invoke.Self -> "done.invoke"
        is StaticHostNotifyEvent.Done.Invoke.Ask -> "done.invoke.ask"
        is StaticHostNotifyEvent.Done.Invoke.Cancel -> "done.invoke.cancel"
        is StaticHostNotifyEvent.Done.Invoke.Far -> "done.invoke.far"
        is StaticHostNotifyEvent.Done.Invoke.Post -> "done.invoke.post"
        is StaticHostNotifyEvent.Done.Invoke.Prompt -> "done.invoke.prompt"
        is StaticHostNotifyEvent.Error.Execution -> "error.execution"
        is StaticHostNotifyEvent.Error.Invoke.Ask -> "error.invoke.ask"
        is StaticHostNotifyEvent.Error.Invoke.Cancel -> "error.invoke.cancel"
        is StaticHostNotifyEvent.Error.Invoke.Post -> "error.invoke.post"
        is StaticHostNotifyEvent.Error.Invoke.Prompt -> "error.invoke.prompt"
        is StaticHostNotifyEvent.Host.Notify.Cancel.Request -> "host.notify.cancel.request"
        is StaticHostNotifyEvent.Host.Notify.Far.Request -> "host.notify.far.request"
        is StaticHostNotifyEvent.Host.Notify.Permission.Request -> "host.notify.permission.request"
        is StaticHostNotifyEvent.Host.Notify.Post.Request -> "host.notify.post.request"
        is StaticHostNotifyEvent.Host.Notify.RequestPermission.Request -> "host.notify.request_permission.request"
    }

    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "far",
        "ask",
        "prompt",
        "post",
        "cancel",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticHostNotifyState,
        event: StaticHostNotifyEvent?
    ): EnabledTransition<StaticHostNotifyState, HistoryId>? = when (state) {
        is StaticHostNotifyState.Asking -> when {
            event is StaticHostNotifyEvent.Done.Invoke.Ask && pendingDoneInvokeAskPayload != null && (pendingDoneInvokeAskPayload!!.error == "") -> transitionAskingAt0
            event is StaticHostNotifyEvent.Done.Invoke.Ask -> transitionAskingAt1
            event is StaticHostNotifyEvent.Error.Invoke.Ask -> transitionAskingAt2
            event is StaticHostNotifyEvent.Error.Execution -> transitionAskingAt3
            else -> null
        }
        is StaticHostNotifyState.Cancelling -> when {
            event is StaticHostNotifyEvent.Done.Invoke.Cancel && pendingDoneInvokeCancelPayload != null && (pendingDoneInvokeCancelPayload!!.error == "") -> transitionCancellingAt0
            event is StaticHostNotifyEvent.Done.Invoke.Cancel -> transitionCancellingAt1
            event is StaticHostNotifyEvent.Error.Invoke.Cancel -> transitionCancellingAt2
            event is StaticHostNotifyEvent.Error.Execution -> transitionCancellingAt3
            else -> null
        }
        is StaticHostNotifyState.Idle -> when {
            event is StaticHostNotifyEvent.Host.Notify.Permission.Request -> transitionIdleAt0
            event is StaticHostNotifyEvent.Host.Notify.RequestPermission.Request -> transitionIdleAt1
            event is StaticHostNotifyEvent.Host.Notify.Post.Request -> transitionIdleAt2
            event is StaticHostNotifyEvent.Host.Notify.Cancel.Request -> transitionIdleAt3
            event is StaticHostNotifyEvent.Host.Notify.Far.Request -> transitionIdleAt4
            else -> null
        }
        is StaticHostNotifyState.Posting -> when {
            event is StaticHostNotifyEvent.Done.Invoke.Post && pendingDoneInvokePostPayload != null && (pendingDoneInvokePostPayload!!.error == "") -> transitionPostingAt0
            event is StaticHostNotifyEvent.Done.Invoke.Post -> transitionPostingAt1
            event is StaticHostNotifyEvent.Error.Invoke.Post -> transitionPostingAt2
            event is StaticHostNotifyEvent.Error.Execution -> transitionPostingAt3
            else -> null
        }
        is StaticHostNotifyState.PostingFar -> when {
            event is StaticHostNotifyEvent.Done.Invoke.Far && pendingDoneInvokeFarPayload != null && (pendingDoneInvokeFarPayload!!.error == "") -> transitionPostingFarAt0
            event is StaticHostNotifyEvent.Done.Invoke.Far -> transitionPostingFarAt1
            event is StaticHostNotifyEvent.Error.Execution -> transitionPostingFarAt2
            else -> null
        }
        is StaticHostNotifyState.Requesting -> when {
            event is StaticHostNotifyEvent.Done.Invoke.Prompt && pendingDoneInvokePromptPayload != null && (pendingDoneInvokePromptPayload!!.error == "") -> transitionRequestingAt0
            event is StaticHostNotifyEvent.Done.Invoke.Prompt -> transitionRequestingAt1
            event is StaticHostNotifyEvent.Error.Invoke.Prompt -> transitionRequestingAt2
            event is StaticHostNotifyEvent.Error.Execution -> transitionRequestingAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_host_notify.scxml:23 :: _machine
    override fun onEntry(state: StaticHostNotifyState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticHostNotifyState.Asking -> {
                // SCE-MAP: static_host_notify.scxml:81 :: asking :: _state_body
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
                    val generatedInvokeId = "asking.${System.identityHashCode(this)}.ask"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-notify-permission",
                                invokeId = "ask",
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
                            raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StaticHostNotifyState.Cancelling -> {
                // SCE-MAP: static_host_notify.scxml:137 :: cancelling :: _state_body
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
                    val generatedInvokeId = "cancelling.${System.identityHashCode(this)}.cancel"
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
                            val v: Any? = vId
                            hostInvokeParams["id"] =
                                (hostInvokeParams["id"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "id", v)
                        }
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-notify-cancel",
                                invokeId = "cancel",
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
                            raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StaticHostNotifyState.Idle -> {
                // SCE-MAP: static_host_notify.scxml:41 :: idle :: _state_body
            }
            is StaticHostNotifyState.Posting -> {
                // SCE-MAP: static_host_notify.scxml:115 :: posting :: _state_body
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
                    val generatedInvokeId = "posting.${System.identityHashCode(this)}.post"
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
                            val v: Any? = vId
                            hostInvokeParams["id"] =
                                (hostInvokeParams["id"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "id", v)
                        }
                        run {
                            val v: Any? = vTitle
                            hostInvokeParams["title"] =
                                (hostInvokeParams["title"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "title", v)
                        }
                        run {
                            val v: Any? = vBody
                            hostInvokeParams["body"] =
                                (hostInvokeParams["body"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "body", v)
                        }
                        run {
                            val v: Any? = (vAtMs).toString()
                            hostInvokeParams["at_ms"] =
                                (hostInvokeParams["at_ms"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "at_ms", v)
                        }
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-notify-post",
                                invokeId = "post",
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
                            raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StaticHostNotifyState.PostingFar -> {
                // SCE-MAP: static_host_notify.scxml:62 :: posting_far :: _state_body
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
                    val generatedInvokeId = "posting_far.${System.identityHashCode(this)}.far"
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
                        hostInvokeParams["id"] =
                            (hostInvokeParams["id"] ?: emptyList()) + "far"
                        putParam(hostInvokePayload, "id", "far")
                        run {
                            val v: Any? = vTitle
                            hostInvokeParams["title"] =
                                (hostInvokeParams["title"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "title", v)
                        }
                        run {
                            val v: Any? = vBody
                            hostInvokeParams["body"] =
                                (hostInvokeParams["body"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "body", v)
                        }
                        try {
                            val v: Any? = (com.sce.forge.runtime.SceChecked.add(vAtMs, 1)).toString()
                            hostInvokeParams["at_ms"] =
                                (hostInvokeParams["at_ms"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "at_ms", v)
                        } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                            raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<invoke> <param name='at_ms'> cannot be its request field: it failed to evaluate")
                            return@deferInvoke
                        }
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-notify-post",
                                invokeId = "far",
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
                            raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StaticHostNotifyState.Requesting -> {
                // SCE-MAP: static_host_notify.scxml:98 :: requesting :: _state_body
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
                    val generatedInvokeId = "requesting.${System.identityHashCode(this)}.prompt"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-notify-request-permission",
                                invokeId = "prompt",
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
                            raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_host_notify.scxml:23 :: _machine
    override fun onExit(state: StaticHostNotifyState) {
        when (state) {
            is StaticHostNotifyState.Asking -> {
                // SCE-MAP: static_host_notify.scxml:81 :: asking :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-notify-permission", "ask")
            }
            is StaticHostNotifyState.Cancelling -> {
                // SCE-MAP: static_host_notify.scxml:137 :: cancelling :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-notify-cancel", "cancel")
            }
            is StaticHostNotifyState.Idle -> {
                // SCE-MAP: static_host_notify.scxml:41 :: idle :: _state_body
            }
            is StaticHostNotifyState.Posting -> {
                // SCE-MAP: static_host_notify.scxml:115 :: posting :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-notify-post", "post")
            }
            is StaticHostNotifyState.PostingFar -> {
                // SCE-MAP: static_host_notify.scxml:62 :: posting_far :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-notify-post", "far")
            }
            is StaticHostNotifyState.Requesting -> {
                // SCE-MAP: static_host_notify.scxml:98 :: requesting :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-notify-request-permission", "prompt")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_host_notify.scxml:23 :: _machine
    override fun executeTransitionContent(source: StaticHostNotifyState, transitionIndex: Int) {
        when (source) {
        is StaticHostNotifyState.Asking -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_notify.scxml:83 :: asking :: _transition_0
                if (pendingDoneInvokeAskPayload == null) {
                    return
                }

            if (try { vPermission = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeAskPayload!!.state, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='permission'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("done", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_notify.scxml:87 :: asking :: _transition_1
                if (pendingDoneInvokeAskPayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeAskPayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_host_notify.scxml:90 :: asking :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("deadline", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_host_notify.scxml:93 :: asking :: _transition_3

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("refused", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticHostNotifyState.Cancelling -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_notify.scxml:141 :: cancelling :: _transition_0
                if (pendingDoneInvokeCancelPayload == null) {
                    return
                }

            vExisted = pendingDoneInvokeCancelPayload!!.existed

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("done", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_notify.scxml:145 :: cancelling :: _transition_1
                if (pendingDoneInvokeCancelPayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeCancelPayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_host_notify.scxml:148 :: cancelling :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("deadline", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_host_notify.scxml:151 :: cancelling :: _transition_3

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("refused", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticHostNotifyState.Idle -> when (transitionIndex) {
            2 -> {
                // SCE-MAP: static_host_notify.scxml:44 :: idle :: _transition_2
                if (pendingHostNotifyPostRequestPayload == null) {
                    return
                }

            if (try { vId = com.sce.forge.runtime.SceChecked.bounded(pendingHostNotifyPostRequestPayload!!.id, 64); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='id'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vTitle = com.sce.forge.runtime.SceChecked.bounded(pendingHostNotifyPostRequestPayload!!.title, 128); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='title'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vBody = com.sce.forge.runtime.SceChecked.bounded(pendingHostNotifyPostRequestPayload!!.body, 512); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='body'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vAtMs = pendingHostNotifyPostRequestPayload!!.at_ms
            }
            3 -> {
                // SCE-MAP: static_host_notify.scxml:50 :: idle :: _transition_3
                if (pendingHostNotifyCancelRequestPayload == null) {
                    return
                }

            if (try { vId = com.sce.forge.runtime.SceChecked.bounded(pendingHostNotifyCancelRequestPayload!!.id, 64); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='id'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_host_notify.scxml:57 :: idle :: _transition_4

            vAtMs = 9223372036854775807
            }
            else -> {}
        }
        is StaticHostNotifyState.Posting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_notify.scxml:122 :: posting :: _transition_0
                if (pendingDoneInvokePostPayload == null) {
                    return
                }

            if (try { vStatus = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokePostPayload!!.status, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='status'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("done", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_notify.scxml:126 :: posting :: _transition_1
                if (pendingDoneInvokePostPayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokePostPayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_host_notify.scxml:129 :: posting :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("deadline", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_host_notify.scxml:132 :: posting :: _transition_3

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("refused", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticHostNotifyState.PostingFar -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_notify.scxml:69 :: posting_far :: _transition_0
                if (pendingDoneInvokeFarPayload == null) {
                    return
                }

            if (try { vStatus = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeFarPayload!!.status, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='status'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("done", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_notify.scxml:73 :: posting_far :: _transition_1
                if (pendingDoneInvokeFarPayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeFarPayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_host_notify.scxml:76 :: posting_far :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("refused", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticHostNotifyState.Requesting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_notify.scxml:100 :: requesting :: _transition_0
                if (pendingDoneInvokePromptPayload == null) {
                    return
                }

            if (try { vPermission = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokePromptPayload!!.state, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='permission'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("done", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_notify.scxml:104 :: requesting :: _transition_1
                if (pendingDoneInvokePromptPayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokePromptPayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_host_notify.scxml:107 :: requesting :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("deadline", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_host_notify.scxml:110 :: requesting :: _transition_3

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("refused", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostNotifyEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
