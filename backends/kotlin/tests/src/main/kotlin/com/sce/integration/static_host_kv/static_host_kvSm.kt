// SCE-GENERATED — DO NOT EDIT
// source-hash: 23b50ff6771257bf06531b96d0a47eeaa5c39fb71939d9bd5e537100a9105206

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_host_kv.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_host_kv.scxml:31 :: _machine

package com.sce.integration.static_host_kv

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticHostKvState : State {
    data object Deleting : StaticHostKvState
    data object Getting : StaticHostKvState
    data object Idle : StaticHostKvState
    data object Putting : StaticHostKvState
    data object Walking : StaticHostKvState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticHostKvEvent : Event {
    sealed interface Done : StaticHostKvEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Del : Invoke
            data object Get : Invoke
            data object Put : Invoke
            data object Walk : Invoke
        }
    }
    sealed interface Error : StaticHostKvEvent {
        data object Execution : Error
        sealed interface Invoke : Error {
            data object Del : Invoke
            data object Get : Invoke
            data object Put : Invoke
            data object Walk : Invoke
        }
    }
    sealed interface Host : StaticHostKvEvent {
        sealed interface Kv : Host {
            sealed interface Delete : Kv {
                data object Request : Delete
            }
            sealed interface Get : Kv {
                data object Request : Get
            }
            sealed interface NextKey : Kv {
                data object Request : NextKey
            }
            sealed interface Put : Kv {
                data object Request : Put
            }
        }
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticHostKvPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticHostKvDoneInvokeDelPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.del`. Consumers inject it via the `raiseDoneInvokeDel` seam
// on the machine — they never name this class directly.
data class StaticHostKvDoneInvokeDelPayload(val error: String, val existed: Boolean)

// StaticHostKvDoneInvokeGetPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.get`. Consumers inject it via the `raiseDoneInvokeGet` seam
// on the machine — they never name this class directly.
data class StaticHostKvDoneInvokeGetPayload(val error: String, val found: Boolean, val value: String)

// StaticHostKvDoneInvokePutPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.put`. Consumers inject it via the `raiseDoneInvokePut` seam
// on the machine — they never name this class directly.
data class StaticHostKvDoneInvokePutPayload(val error: String, val replaced: Boolean)

// StaticHostKvDoneInvokeWalkPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.walk`. Consumers inject it via the `raiseDoneInvokeWalk` seam
// on the machine — they never name this class directly.
data class StaticHostKvDoneInvokeWalkPayload(val error: String, val found: Boolean, val key: String)

// StaticHostKvHostKvDeleteRequestPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `host.kv.delete.request`. Consumers inject it via the `raiseHostKvDeleteRequest` seam
// on the machine — they never name this class directly.
data class StaticHostKvHostKvDeleteRequestPayload(val key: String)

// StaticHostKvHostKvGetRequestPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `host.kv.get.request`. Consumers inject it via the `raiseHostKvGetRequest` seam
// on the machine — they never name this class directly.
data class StaticHostKvHostKvGetRequestPayload(val key: String)

// StaticHostKvHostKvNextKeyRequestPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `host.kv.next_key.request`. Consumers inject it via the `raiseHostKvNextKeyRequest` seam
// on the machine — they never name this class directly.
data class StaticHostKvHostKvNextKeyRequestPayload(val after: String, val prefix: String)

// StaticHostKvHostKvPutRequestPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `host.kv.put.request`. Consumers inject it via the `raiseHostKvPutRequest` seam
// on the machine — they never name this class directly.
data class StaticHostKvHostKvPutRequestPayload(val key: String, val value: String)


// --- State Machine (W3C SCXML) ---

// ── SCE Accepted Subset 2.12: typed host-run invoke interface ─────────
/** `sce:request`: the record the host is asked to start `<invoke id="get">` with (SCE Accepted Subset §2.12). */
data class StaticHostKvGetRequest(
    val key: String,
)

/** `sce:result`: the record the host completes `<invoke id="get">` with (SCE Accepted Subset §2.12). */
data class StaticHostKvGetResult(
    val error: String,
    val found: Boolean,
    val value: String,
) {
    /** The JSON `done.invoke.get` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "found" to found, "value" to value))
}

/** `sce:request`: the record the host is asked to start `<invoke id="put">` with (SCE Accepted Subset §2.12). */
data class StaticHostKvPutRequest(
    val key: String,
    val value: String,
)

/** `sce:result`: the record the host completes `<invoke id="put">` with (SCE Accepted Subset §2.12). */
data class StaticHostKvPutResult(
    val error: String,
    val replaced: Boolean,
) {
    /** The JSON `done.invoke.put` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "replaced" to replaced))
}

/** `sce:request`: the record the host is asked to start `<invoke id="del">` with (SCE Accepted Subset §2.12). */
data class StaticHostKvDelRequest(
    val key: String,
)

/** `sce:result`: the record the host completes `<invoke id="del">` with (SCE Accepted Subset §2.12). */
data class StaticHostKvDelResult(
    val error: String,
    val existed: Boolean,
) {
    /** The JSON `done.invoke.del` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "existed" to existed))
}

/** `sce:request`: the record the host is asked to start `<invoke id="walk">` with (SCE Accepted Subset §2.12). */
data class StaticHostKvWalkRequest(
    val after: String,
    val prefix: String,
)

/** `sce:result`: the record the host completes `<invoke id="walk">` with (SCE Accepted Subset §2.12). */
data class StaticHostKvWalkResult(
    val error: String,
    val found: Boolean,
    val key: String,
) {
    /** The JSON `done.invoke.walk` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "found" to found, "key" to key))
}

/**
 * The host side of this document's typed `<invoke type="sce-host-kv-delete">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostKvDeleteInvoker`.
 */
interface StaticHostKvSceHostKvDeleteInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="del">`. [token] names this start;
     * a host that finishes later hands it back to `completeDel` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startDel(request: StaticHostKvDelRequest, token: Long): StaticHostKvDelResult?

    /** §scxml-6.4: `<invoke id="del">`'s state exited while the start [token] names was still running. */
    fun cancelDel(token: Long)
}

/**
 * The host side of this document's typed `<invoke type="sce-host-kv-get">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostKvGetInvoker`.
 */
interface StaticHostKvSceHostKvGetInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="get">`. [token] names this start;
     * a host that finishes later hands it back to `completeGet` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startGet(request: StaticHostKvGetRequest, token: Long): StaticHostKvGetResult?

    /** §scxml-6.4: `<invoke id="get">`'s state exited while the start [token] names was still running. */
    fun cancelGet(token: Long)
}

/**
 * The host side of this document's typed `<invoke type="sce-host-kv-next-key">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostKvNextKeyInvoker`.
 */
interface StaticHostKvSceHostKvNextKeyInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="walk">`. [token] names this start;
     * a host that finishes later hands it back to `completeWalk` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startWalk(request: StaticHostKvWalkRequest, token: Long): StaticHostKvWalkResult?

    /** §scxml-6.4: `<invoke id="walk">`'s state exited while the start [token] names was still running. */
    fun cancelWalk(token: Long)
}

/**
 * The host side of this document's typed `<invoke type="sce-host-kv-put">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostKvPutInvoker`.
 */
interface StaticHostKvSceHostKvPutInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="put">`. [token] names this start;
     * a host that finishes later hands it back to `completePut` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startPut(request: StaticHostKvPutRequest, token: Long): StaticHostKvPutResult?

    /** §scxml-6.4: `<invoke id="put">`'s state exited while the start [token] names was still running. */
    fun cancelPut(token: Long)
}


class StaticHostKvStateMachine(
) : StateMachineEngine<StaticHostKvState, StaticHostKvEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `key` datamodel variable, the machine's own. */
    private var vKey: String = ""
    /** W3C SCXML 5.2: the `value` datamodel variable, the machine's own. */
    private var vValue: String = ""
    /** W3C SCXML 5.2: the `after` datamodel variable, the machine's own. */
    private var vAfter: String = ""
    /** W3C SCXML 5.2: the `prefix` datamodel variable, the machine's own. */
    private var vPrefix: String = ""
    /** W3C SCXML 5.2: the `outcome` datamodel variable, published (`sce:direction="out"`). */
    private var vOutcome: String = ""

    /** What the published `outcome` holds. */
    val outcome: String get() = vOutcome
    /** W3C SCXML 5.2: the `found` datamodel variable, published (`sce:direction="out"`). */
    private var vFound: Boolean = false

    /** What the published `found` holds. */
    val found: Boolean get() = vFound
    /** W3C SCXML 5.2: the `got` datamodel variable, published (`sce:direction="out"`). */
    private var vGot: String = ""

    /** What the published `got` holds. */
    val got: String get() = vGot
    /** W3C SCXML 5.2: the `replaced` datamodel variable, published (`sce:direction="out"`). */
    private var vReplaced: Boolean = false

    /** What the published `replaced` holds. */
    val replaced: Boolean get() = vReplaced
    /** W3C SCXML 5.2: the `existed` datamodel variable, published (`sce:direction="out"`). */
    private var vExisted: Boolean = false

    /** What the published `existed` holds. */
    val existed: Boolean get() = vExisted
    /** W3C SCXML 5.2: the `next_found` datamodel variable, published (`sce:direction="out"`). */
    private var vNextFound: Boolean = false

    /** What the published `next_found` holds. */
    val nextFound: Boolean get() = vNextFound
    /** W3C SCXML 5.2: the `next` datamodel variable, published (`sce:direction="out"`). */
    private var vNext: String = ""

    /** What the published `next` holds. */
    val next: String get() = vNext

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var key: String? = null
        var value: String? = null
        var after: String? = null
        var prefix: String? = null
        var outcome: String? = null
        var found: Boolean? = null
        var got: String? = null
        var replaced: Boolean? = null
        var existed: Boolean? = null
        var nextFound: Boolean? = null
        var next: String? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.key?.let { vKey = it }
        params.value?.let { vValue = it }
        params.after?.let { vAfter = it }
        params.prefix?.let { vPrefix = it }
        params.outcome?.let { vOutcome = it }
        params.found?.let { vFound = it }
        params.got?.let { vGot = it }
        params.replaced?.let { vReplaced = it }
        params.existed?.let { vExisted = it }
        params.nextFound?.let { vNextFound = it }
        params.next?.let { vNext = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val outcome: String,
        val found: Boolean,
        val got: String,
        val replaced: Boolean,
        val existed: Boolean,
        val nextFound: Boolean,
        val next: String,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticHostKvState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        outcome = vOutcome,
        found = vFound,
        got = vGot,
        replaced = vReplaced,
        existed = vExisted,
        nextFound = vNextFound,
        next = vNext,
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
    val savedShape: String = "e559d3befc5e45e1282094d521bf840ef102b9aac72ec8ceabae510e75799631"

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
            "key" to SavedValues.of(vKey),
            "value" to SavedValues.of(vValue),
            "after" to SavedValues.of(vAfter),
            "prefix" to SavedValues.of(vPrefix),
            "outcome" to SavedValues.of(vOutcome),
            "found" to SavedValues.of(vFound),
            "got" to SavedValues.of(vGot),
            "replaced" to SavedValues.of(vReplaced),
            "existed" to SavedValues.of(vExisted),
            "next_found" to SavedValues.of(vNextFound),
            "next" to SavedValues.of(vNext),
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
        val saved1 = SavedValues.string(saved.variable("key"), "key", 256)
        val saved2 = SavedValues.string(saved.variable("value"), "value", 4096)
        val saved3 = SavedValues.string(saved.variable("after"), "after", 256)
        val saved4 = SavedValues.string(saved.variable("prefix"), "prefix", 256)
        val saved5 = SavedValues.string(saved.variable("outcome"), "outcome", 24)
        val saved6 = SavedValues.bool(saved.variable("found"), "found")
        val saved7 = SavedValues.string(saved.variable("got"), "got", 4096)
        val saved8 = SavedValues.bool(saved.variable("replaced"), "replaced")
        val saved9 = SavedValues.bool(saved.variable("existed"), "existed")
        val saved10 = SavedValues.bool(saved.variable("next_found"), "next_found")
        val saved11 = SavedValues.string(saved.variable("next"), "next", 256)
        vKey = saved1
        vValue = saved2
        vAfter = saved3
        vPrefix = saved4
        vOutcome = saved5
        vFound = saved6
        vGot = saved7
        vReplaced = saved8
        vExisted = saved9
        vNextFound = saved10
        vNext = saved11
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, StaticHostKvState>> = listOf(
        Triple("sce-host-kv-get", "get", StaticHostKvState.Getting),
        Triple("sce-host-kv-put", "put", StaticHostKvState.Putting),
        Triple("sce-host-kv-delete", "del", StaticHostKvState.Deleting),
        Triple("sce-host-kv-next-key", "walk", StaticHostKvState.Walking),
    )

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDoneInvokeDelPayload: StaticHostKvDoneInvokeDelPayload? = null
    private var pendingDoneInvokeGetPayload: StaticHostKvDoneInvokeGetPayload? = null
    private var pendingDoneInvokePutPayload: StaticHostKvDoneInvokePutPayload? = null
    private var pendingDoneInvokeWalkPayload: StaticHostKvDoneInvokeWalkPayload? = null
    private var pendingHostKvDeleteRequestPayload: StaticHostKvHostKvDeleteRequestPayload? = null
    private var pendingHostKvGetRequestPayload: StaticHostKvHostKvGetRequestPayload? = null
    private var pendingHostKvNextKeyRequestPayload: StaticHostKvHostKvNextKeyRequestPayload? = null
    private var pendingHostKvPutRequestPayload: StaticHostKvHostKvPutRequestPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticHostKvEvent, metadata: EventMetadata) {
        pendingDoneInvokeDelPayload = null
        pendingDoneInvokeGetPayload = null
        pendingDoneInvokePutPayload = null
        pendingDoneInvokeWalkPayload = null
        pendingHostKvDeleteRequestPayload = null
        pendingHostKvGetRequestPayload = null
        pendingHostKvNextKeyRequestPayload = null
        pendingHostKvPutRequestPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticHostKvDoneInvokeDelPayload -> pendingDoneInvokeDelPayload = tp
            is StaticHostKvDoneInvokeGetPayload -> pendingDoneInvokeGetPayload = tp
            is StaticHostKvDoneInvokePutPayload -> pendingDoneInvokePutPayload = tp
            is StaticHostKvDoneInvokeWalkPayload -> pendingDoneInvokeWalkPayload = tp
            is StaticHostKvHostKvDeleteRequestPayload -> pendingHostKvDeleteRequestPayload = tp
            is StaticHostKvHostKvGetRequestPayload -> pendingHostKvGetRequestPayload = tp
            is StaticHostKvHostKvNextKeyRequestPayload -> pendingHostKvNextKeyRequestPayload = tp
            is StaticHostKvHostKvPutRequestPayload -> pendingHostKvPutRequestPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticHostKvEvent.Done.Invoke.Del) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokeDelPayload = StaticHostKvDoneInvokeDelPayload(fields.string("error"), fields.boolean("existed"))
                } else if (event == StaticHostKvEvent.Done.Invoke.Get) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokeGetPayload = StaticHostKvDoneInvokeGetPayload(fields.string("error"), fields.boolean("found"), fields.string("value"))
                } else if (event == StaticHostKvEvent.Done.Invoke.Put) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokePutPayload = StaticHostKvDoneInvokePutPayload(fields.string("error"), fields.boolean("replaced"))
                } else if (event == StaticHostKvEvent.Done.Invoke.Walk) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokeWalkPayload = StaticHostKvDoneInvokeWalkPayload(fields.string("error"), fields.boolean("found"), fields.string("key"))
                } else if (event == StaticHostKvEvent.Host.Kv.Delete.Request) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingHostKvDeleteRequestPayload = StaticHostKvHostKvDeleteRequestPayload(fields.string("key"))
                } else if (event == StaticHostKvEvent.Host.Kv.Get.Request) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingHostKvGetRequestPayload = StaticHostKvHostKvGetRequestPayload(fields.string("key"))
                } else if (event == StaticHostKvEvent.Host.Kv.NextKey.Request) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingHostKvNextKeyRequestPayload = StaticHostKvHostKvNextKeyRequestPayload(fields.string("after"), fields.string("prefix"))
                } else if (event == StaticHostKvEvent.Host.Kv.Put.Request) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingHostKvPutRequestPayload = StaticHostKvHostKvPutRequestPayload(fields.string("key"), fields.string("value"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `host.kv.delete.request` — binds the event name and the payload field values in one call.
    fun raiseHostKvDeleteRequest(key: String) {
        send(
            StaticHostKvEvent.Host.Kv.Delete.Request,
            EventMetadata(
                type = "external",
                typedPayload = StaticHostKvHostKvDeleteRequestPayload(key),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("key" to key))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `host.kv.get.request` — binds the event name and the payload field values in one call.
    fun raiseHostKvGetRequest(key: String) {
        send(
            StaticHostKvEvent.Host.Kv.Get.Request,
            EventMetadata(
                type = "external",
                typedPayload = StaticHostKvHostKvGetRequestPayload(key),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("key" to key))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `host.kv.next_key.request` — binds the event name and the payload field values in one call.
    fun raiseHostKvNextKeyRequest(after: String, prefix: String) {
        send(
            StaticHostKvEvent.Host.Kv.NextKey.Request,
            EventMetadata(
                type = "external",
                typedPayload = StaticHostKvHostKvNextKeyRequestPayload(after, prefix),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("after" to after, "prefix" to prefix))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `host.kv.put.request` — binds the event name and the payload field values in one call.
    fun raiseHostKvPutRequest(key: String, value: String) {
        send(
            StaticHostKvEvent.Host.Kv.Put.Request,
            EventMetadata(
                type = "external",
                typedPayload = StaticHostKvHostKvPutRequestPayload(key, value),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("key" to key, "value" to value))
            )
        )
    }


    // SCE Accepted Subset 2.12: registration and typed completion for this
    // document's typed host-run invokes.

    /**
     * Complete `<invoke id="del">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completeDel(token: Long, result: StaticHostKvDelResult): Boolean =
        completeHostInvoke("sce-host-kv-delete", "del", token, result.wire())

    /**
     * Register [invoker] as the handler for `type="sce-host-kv-delete"`.
     */
    fun registerSceHostKvDeleteInvoker(invoker: StaticHostKvSceHostKvDeleteInvoker) {
        registerInvoker("sce-host-kv-delete") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "del" -> {
                        val typed = StaticHostKvDelRequest(
                            key = TypedRequest.string(start.params, start.invokeId, "key"),
                        )
                        invoker.startDel(typed, start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "del" -> {
                        invoker.cancelDel(cancel.token)
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
     * Complete `<invoke id="get">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completeGet(token: Long, result: StaticHostKvGetResult): Boolean =
        completeHostInvoke("sce-host-kv-get", "get", token, result.wire())

    /**
     * Register [invoker] as the handler for `type="sce-host-kv-get"`.
     */
    fun registerSceHostKvGetInvoker(invoker: StaticHostKvSceHostKvGetInvoker) {
        registerInvoker("sce-host-kv-get") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "get" -> {
                        val typed = StaticHostKvGetRequest(
                            key = TypedRequest.string(start.params, start.invokeId, "key"),
                        )
                        invoker.startGet(typed, start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "get" -> {
                        invoker.cancelGet(cancel.token)
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
     * Complete `<invoke id="walk">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completeWalk(token: Long, result: StaticHostKvWalkResult): Boolean =
        completeHostInvoke("sce-host-kv-next-key", "walk", token, result.wire())

    /**
     * Register [invoker] as the handler for `type="sce-host-kv-next-key"`.
     */
    fun registerSceHostKvNextKeyInvoker(invoker: StaticHostKvSceHostKvNextKeyInvoker) {
        registerInvoker("sce-host-kv-next-key") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "walk" -> {
                        val typed = StaticHostKvWalkRequest(
                            after = TypedRequest.string(start.params, start.invokeId, "after"),
                            prefix = TypedRequest.string(start.params, start.invokeId, "prefix"),
                        )
                        invoker.startWalk(typed, start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "walk" -> {
                        invoker.cancelWalk(cancel.token)
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
     * Complete `<invoke id="put">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completePut(token: Long, result: StaticHostKvPutResult): Boolean =
        completeHostInvoke("sce-host-kv-put", "put", token, result.wire())

    /**
     * Register [invoker] as the handler for `type="sce-host-kv-put"`.
     */
    fun registerSceHostKvPutInvoker(invoker: StaticHostKvSceHostKvPutInvoker) {
        registerInvoker("sce-host-kv-put") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "put" -> {
                        val typed = StaticHostKvPutRequest(
                            key = TypedRequest.string(start.params, start.invokeId, "key"),
                            value = TypedRequest.string(start.params, start.invokeId, "value"),
                        )
                        invoker.startPut(typed, start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "put" -> {
                        invoker.cancelPut(cancel.token)
                        null
                    }
                    else -> null
                }
            } else {
                null
            }
        }
    }


    override val initialState: StaticHostKvState = StaticHostKvState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticHostKvState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticHostKvState, HistoryId>> =
            listOf(StateTarget(StaticHostKvState.Idle))

        // W3C SCXML 3.13: deleting's transition 0, as the microstep reads it.
        val transitionDeletingAt0 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Deleting,
            listOf(StateTarget(StaticHostKvState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: deleting's transition 1, as the microstep reads it.
        val transitionDeletingAt1 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Deleting,
            listOf(StateTarget(StaticHostKvState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: deleting's transition 2, as the microstep reads it.
        val transitionDeletingAt2 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Deleting,
            listOf(StateTarget(StaticHostKvState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: deleting's transition 3, as the microstep reads it.
        val transitionDeletingAt3 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Deleting,
            listOf(StateTarget(StaticHostKvState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: getting's transition 0, as the microstep reads it.
        val transitionGettingAt0 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Getting,
            listOf(StateTarget(StaticHostKvState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: getting's transition 1, as the microstep reads it.
        val transitionGettingAt1 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Getting,
            listOf(StateTarget(StaticHostKvState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: getting's transition 2, as the microstep reads it.
        val transitionGettingAt2 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Getting,
            listOf(StateTarget(StaticHostKvState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: getting's transition 3, as the microstep reads it.
        val transitionGettingAt3 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Getting,
            listOf(StateTarget(StaticHostKvState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Idle,
            listOf(StateTarget(StaticHostKvState.Getting)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Idle,
            listOf(StateTarget(StaticHostKvState.Putting)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Idle,
            listOf(StateTarget(StaticHostKvState.Deleting)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Idle,
            listOf(StateTarget(StaticHostKvState.Walking)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: putting's transition 0, as the microstep reads it.
        val transitionPuttingAt0 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Putting,
            listOf(StateTarget(StaticHostKvState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: putting's transition 1, as the microstep reads it.
        val transitionPuttingAt1 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Putting,
            listOf(StateTarget(StaticHostKvState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: putting's transition 2, as the microstep reads it.
        val transitionPuttingAt2 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Putting,
            listOf(StateTarget(StaticHostKvState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: putting's transition 3, as the microstep reads it.
        val transitionPuttingAt3 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Putting,
            listOf(StateTarget(StaticHostKvState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: walking's transition 0, as the microstep reads it.
        val transitionWalkingAt0 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Walking,
            listOf(StateTarget(StaticHostKvState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: walking's transition 1, as the microstep reads it.
        val transitionWalkingAt1 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Walking,
            listOf(StateTarget(StaticHostKvState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: walking's transition 2, as the microstep reads it.
        val transitionWalkingAt2 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Walking,
            listOf(StateTarget(StaticHostKvState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: walking's transition 3, as the microstep reads it.
        val transitionWalkingAt3 = EnabledTransition<StaticHostKvState, HistoryId>(
            StaticHostKvState.Walking,
            listOf(StateTarget(StaticHostKvState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticHostKvState? = when (stateId) {
        "deleting" -> StaticHostKvState.Deleting
        "getting" -> StaticHostKvState.Getting
        "idle" -> StaticHostKvState.Idle
        "putting" -> StaticHostKvState.Putting
        "walking" -> StaticHostKvState.Walking
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticHostKvState): String = when (state) {
        is StaticHostKvState.Deleting -> "deleting"
        is StaticHostKvState.Getting -> "getting"
        is StaticHostKvState.Idle -> "idle"
        is StaticHostKvState.Putting -> "putting"
        is StaticHostKvState.Walking -> "walking"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticHostKvState): Int = when (state) {
        is StaticHostKvState.Deleting -> 3
        is StaticHostKvState.Getting -> 1
        is StaticHostKvState.Idle -> 0
        is StaticHostKvState.Putting -> 2
        is StaticHostKvState.Walking -> 4
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticHostKvEvent? = when (name) {
        "done.invoke" -> StaticHostKvEvent.Done.Invoke.Self
        "done.invoke.del" -> StaticHostKvEvent.Done.Invoke.Del
        "done.invoke.get" -> StaticHostKvEvent.Done.Invoke.Get
        "done.invoke.put" -> StaticHostKvEvent.Done.Invoke.Put
        "done.invoke.walk" -> StaticHostKvEvent.Done.Invoke.Walk
        "error.execution" -> StaticHostKvEvent.Error.Execution
        "error.invoke.del" -> StaticHostKvEvent.Error.Invoke.Del
        "error.invoke.get" -> StaticHostKvEvent.Error.Invoke.Get
        "error.invoke.put" -> StaticHostKvEvent.Error.Invoke.Put
        "error.invoke.walk" -> StaticHostKvEvent.Error.Invoke.Walk
        "host.kv.delete.request" -> StaticHostKvEvent.Host.Kv.Delete.Request
        "host.kv.get.request" -> StaticHostKvEvent.Host.Kv.Get.Request
        "host.kv.next_key.request" -> StaticHostKvEvent.Host.Kv.NextKey.Request
        "host.kv.put.request" -> StaticHostKvEvent.Host.Kv.Put.Request
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticHostKvEvent): String? = when (event) {
        is StaticHostKvEvent.Done.Invoke.Self -> "done.invoke"
        is StaticHostKvEvent.Done.Invoke.Del -> "done.invoke.del"
        is StaticHostKvEvent.Done.Invoke.Get -> "done.invoke.get"
        is StaticHostKvEvent.Done.Invoke.Put -> "done.invoke.put"
        is StaticHostKvEvent.Done.Invoke.Walk -> "done.invoke.walk"
        is StaticHostKvEvent.Error.Execution -> "error.execution"
        is StaticHostKvEvent.Error.Invoke.Del -> "error.invoke.del"
        is StaticHostKvEvent.Error.Invoke.Get -> "error.invoke.get"
        is StaticHostKvEvent.Error.Invoke.Put -> "error.invoke.put"
        is StaticHostKvEvent.Error.Invoke.Walk -> "error.invoke.walk"
        is StaticHostKvEvent.Host.Kv.Delete.Request -> "host.kv.delete.request"
        is StaticHostKvEvent.Host.Kv.Get.Request -> "host.kv.get.request"
        is StaticHostKvEvent.Host.Kv.NextKey.Request -> "host.kv.next_key.request"
        is StaticHostKvEvent.Host.Kv.Put.Request -> "host.kv.put.request"
    }

    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "get",
        "put",
        "del",
        "walk",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticHostKvState,
        event: StaticHostKvEvent?
    ): EnabledTransition<StaticHostKvState, HistoryId>? = when (state) {
        is StaticHostKvState.Deleting -> when {
            event is StaticHostKvEvent.Done.Invoke.Del && pendingDoneInvokeDelPayload != null && (pendingDoneInvokeDelPayload!!.error == "") -> transitionDeletingAt0
            event is StaticHostKvEvent.Done.Invoke.Del -> transitionDeletingAt1
            event is StaticHostKvEvent.Error.Invoke.Del -> transitionDeletingAt2
            event is StaticHostKvEvent.Error.Execution -> transitionDeletingAt3
            else -> null
        }
        is StaticHostKvState.Getting -> when {
            event is StaticHostKvEvent.Done.Invoke.Get && pendingDoneInvokeGetPayload != null && (pendingDoneInvokeGetPayload!!.error == "") -> transitionGettingAt0
            event is StaticHostKvEvent.Done.Invoke.Get -> transitionGettingAt1
            event is StaticHostKvEvent.Error.Invoke.Get -> transitionGettingAt2
            event is StaticHostKvEvent.Error.Execution -> transitionGettingAt3
            else -> null
        }
        is StaticHostKvState.Idle -> when {
            event is StaticHostKvEvent.Host.Kv.Get.Request -> transitionIdleAt0
            event is StaticHostKvEvent.Host.Kv.Put.Request -> transitionIdleAt1
            event is StaticHostKvEvent.Host.Kv.Delete.Request -> transitionIdleAt2
            event is StaticHostKvEvent.Host.Kv.NextKey.Request -> transitionIdleAt3
            else -> null
        }
        is StaticHostKvState.Putting -> when {
            event is StaticHostKvEvent.Done.Invoke.Put && pendingDoneInvokePutPayload != null && (pendingDoneInvokePutPayload!!.error == "") -> transitionPuttingAt0
            event is StaticHostKvEvent.Done.Invoke.Put -> transitionPuttingAt1
            event is StaticHostKvEvent.Error.Invoke.Put -> transitionPuttingAt2
            event is StaticHostKvEvent.Error.Execution -> transitionPuttingAt3
            else -> null
        }
        is StaticHostKvState.Walking -> when {
            event is StaticHostKvEvent.Done.Invoke.Walk && pendingDoneInvokeWalkPayload != null && (pendingDoneInvokeWalkPayload!!.error == "") -> transitionWalkingAt0
            event is StaticHostKvEvent.Done.Invoke.Walk -> transitionWalkingAt1
            event is StaticHostKvEvent.Error.Invoke.Walk -> transitionWalkingAt2
            event is StaticHostKvEvent.Error.Execution -> transitionWalkingAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_host_kv.scxml:31 :: _machine
    override fun onEntry(state: StaticHostKvState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticHostKvState.Deleting -> {
                // SCE-MAP: static_host_kv.scxml:112 :: deleting :: _state_body
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
                    val generatedInvokeId = "deleting.${System.identityHashCode(this)}.del"
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
                            val v: Any? = vKey
                            hostInvokeParams["key"] =
                                (hostInvokeParams["key"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "key", v)
                        }
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-kv-delete",
                                invokeId = "del",
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
                            raisePlatformError(StaticHostKvEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StaticHostKvState.Getting -> {
                // SCE-MAP: static_host_kv.scxml:72 :: getting :: _state_body
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
                    val generatedInvokeId = "getting.${System.identityHashCode(this)}.get"
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
                            val v: Any? = vKey
                            hostInvokeParams["key"] =
                                (hostInvokeParams["key"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "key", v)
                        }
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-kv-get",
                                invokeId = "get",
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
                            raisePlatformError(StaticHostKvEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StaticHostKvState.Idle -> {
                // SCE-MAP: static_host_kv.scxml:55 :: idle :: _state_body
            }
            is StaticHostKvState.Putting -> {
                // SCE-MAP: static_host_kv.scxml:92 :: putting :: _state_body
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
                    val generatedInvokeId = "putting.${System.identityHashCode(this)}.put"
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
                            val v: Any? = vKey
                            hostInvokeParams["key"] =
                                (hostInvokeParams["key"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "key", v)
                        }
                        run {
                            val v: Any? = vValue
                            hostInvokeParams["value"] =
                                (hostInvokeParams["value"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "value", v)
                        }
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-kv-put",
                                invokeId = "put",
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
                            raisePlatformError(StaticHostKvEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StaticHostKvState.Walking -> {
                // SCE-MAP: static_host_kv.scxml:131 :: walking :: _state_body
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
                    val generatedInvokeId = "walking.${System.identityHashCode(this)}.walk"
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
                            val v: Any? = vAfter
                            hostInvokeParams["after"] =
                                (hostInvokeParams["after"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "after", v)
                        }
                        run {
                            val v: Any? = vPrefix
                            hostInvokeParams["prefix"] =
                                (hostInvokeParams["prefix"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "prefix", v)
                        }
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-kv-next-key",
                                invokeId = "walk",
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
                            raisePlatformError(StaticHostKvEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_host_kv.scxml:31 :: _machine
    override fun onExit(state: StaticHostKvState) {
        when (state) {
            is StaticHostKvState.Deleting -> {
                // SCE-MAP: static_host_kv.scxml:112 :: deleting :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-kv-delete", "del")
            }
            is StaticHostKvState.Getting -> {
                // SCE-MAP: static_host_kv.scxml:72 :: getting :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-kv-get", "get")
            }
            is StaticHostKvState.Idle -> {
                // SCE-MAP: static_host_kv.scxml:55 :: idle :: _state_body
            }
            is StaticHostKvState.Putting -> {
                // SCE-MAP: static_host_kv.scxml:92 :: putting :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-kv-put", "put")
            }
            is StaticHostKvState.Walking -> {
                // SCE-MAP: static_host_kv.scxml:131 :: walking :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-kv-next-key", "walk")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_host_kv.scxml:31 :: _machine
    override fun executeTransitionContent(source: StaticHostKvState, transitionIndex: Int) {
        when (source) {
        is StaticHostKvState.Deleting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_kv.scxml:116 :: deleting :: _transition_0
                if (pendingDoneInvokeDelPayload == null) {
                    return
                }

            vExisted = pendingDoneInvokeDelPayload!!.existed

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("done", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_kv.scxml:120 :: deleting :: _transition_1
                if (pendingDoneInvokeDelPayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeDelPayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_host_kv.scxml:123 :: deleting :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("deadline", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_host_kv.scxml:126 :: deleting :: _transition_3

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("refused", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticHostKvState.Getting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_kv.scxml:76 :: getting :: _transition_0
                if (pendingDoneInvokeGetPayload == null) {
                    return
                }

            vFound = pendingDoneInvokeGetPayload!!.found

            if (try { vGot = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeGetPayload!!.value, 4096); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='got'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("done", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_kv.scxml:81 :: getting :: _transition_1
                if (pendingDoneInvokeGetPayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeGetPayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_host_kv.scxml:84 :: getting :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("deadline", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_host_kv.scxml:87 :: getting :: _transition_3

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("refused", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticHostKvState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_kv.scxml:56 :: idle :: _transition_0
                if (pendingHostKvGetRequestPayload == null) {
                    return
                }

            if (try { vKey = com.sce.forge.runtime.SceChecked.bounded(pendingHostKvGetRequestPayload!!.key, 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='key'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_kv.scxml:59 :: idle :: _transition_1
                if (pendingHostKvPutRequestPayload == null) {
                    return
                }

            if (try { vKey = com.sce.forge.runtime.SceChecked.bounded(pendingHostKvPutRequestPayload!!.key, 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='key'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vValue = com.sce.forge.runtime.SceChecked.bounded(pendingHostKvPutRequestPayload!!.value, 4096); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='value'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_host_kv.scxml:63 :: idle :: _transition_2
                if (pendingHostKvDeleteRequestPayload == null) {
                    return
                }

            if (try { vKey = com.sce.forge.runtime.SceChecked.bounded(pendingHostKvDeleteRequestPayload!!.key, 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='key'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_host_kv.scxml:66 :: idle :: _transition_3
                if (pendingHostKvNextKeyRequestPayload == null) {
                    return
                }

            if (try { vAfter = com.sce.forge.runtime.SceChecked.bounded(pendingHostKvNextKeyRequestPayload!!.after, 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='after'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vPrefix = com.sce.forge.runtime.SceChecked.bounded(pendingHostKvNextKeyRequestPayload!!.prefix, 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='prefix'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticHostKvState.Putting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_kv.scxml:97 :: putting :: _transition_0
                if (pendingDoneInvokePutPayload == null) {
                    return
                }

            vReplaced = pendingDoneInvokePutPayload!!.replaced

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("done", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_kv.scxml:101 :: putting :: _transition_1
                if (pendingDoneInvokePutPayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokePutPayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_host_kv.scxml:104 :: putting :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("deadline", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_host_kv.scxml:107 :: putting :: _transition_3

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("refused", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticHostKvState.Walking -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_kv.scxml:136 :: walking :: _transition_0
                if (pendingDoneInvokeWalkPayload == null) {
                    return
                }

            vNextFound = pendingDoneInvokeWalkPayload!!.found

            if (try { vNext = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeWalkPayload!!.key, 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='next'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("done", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_kv.scxml:141 :: walking :: _transition_1
                if (pendingDoneInvokeWalkPayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeWalkPayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_host_kv.scxml:144 :: walking :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("deadline", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_host_kv.scxml:147 :: walking :: _transition_3

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("refused", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostKvEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
