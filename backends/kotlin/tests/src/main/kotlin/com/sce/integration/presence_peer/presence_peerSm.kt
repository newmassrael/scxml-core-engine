// SCE-GENERATED — DO NOT EDIT
// source-hash: 8b4828d778fc1da05444ac6a052eb44c820b7a80e1f4075fc52bc6e97e8a070a

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/presence_peer.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: presence_peer.scxml:52 :: _machine

package com.sce.integration.presence_peer

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface PresencePeerState : State {
    data object Open : PresencePeerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface PresencePeerEvent : Event {
    sealed interface Error : PresencePeerEvent {
        data object Communication : Error
        data object Execution : Error
    }
    sealed interface Presence : PresencePeerEvent {
        data object Alive : Presence
        data object Beat : Presence
        data object Expire : Presence
        data object Join : Presence
        data object Left : Presence
        data object Seen : Presence
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `PresencePeerPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// PresencePeerPresenceAlivePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `presence.alive`. Consumers inject it via the `raisePresenceAlive` seam
// on the machine — they never name this class directly.
data class PresencePeerPresenceAlivePayload(val client: UInt, val at: Long)

// PresencePeerPresenceBeatPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `presence.beat`. Consumers inject it via the `raisePresenceBeat` seam
// on the machine — they never name this class directly.
data class PresencePeerPresenceBeatPayload(val now: Long)

// PresencePeerPresenceExpirePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `presence.expire`. Consumers inject it via the `raisePresenceExpire` seam
// on the machine — they never name this class directly.
data class PresencePeerPresenceExpirePayload(val gen: UInt)

// PresencePeerPresenceJoinPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `presence.join`. Consumers inject it via the `raisePresenceJoin` seam
// on the machine — they never name this class directly.
data class PresencePeerPresenceJoinPayload(val node: UInt, val hub: Boolean, val ttl: UInt, val slack: UInt)

// PresencePeerPresenceLeftPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `presence.left`. Consumers inject it via the `raisePresenceLeft` seam
// on the machine — they never name this class directly.
data class PresencePeerPresenceLeftPayload(val client: UInt, val last: Long)

// PresencePeerPresenceSeenPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `presence.seen`. Consumers inject it via the `raisePresenceSeen` seam
// on the machine — they never name this class directly.
data class PresencePeerPresenceSeenPayload(val client: UInt, val last: Long, val gen: UInt)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Seen` datamodel value. */
data class PresencePeerSeenRecord(val client: UInt, val last: Long, val gen: UInt) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("client" to SavedValues.of(client), "last" to SavedValues.of(last), "gen" to SavedValues.of(gen))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): PresencePeerSeenRecord = PresencePeerSeenRecord(client = SavedValues.uint32(SavedValues.field(value, what, "client"), "$what.client"), last = SavedValues.int64(SavedValues.field(value, what, "last"), "$what.last"), gen = SavedValues.uint32(SavedValues.field(value, what, "gen"), "$what.gen"))
    }
}
// --- State Machine (W3C SCXML) ---

class PresencePeerStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<PresencePeerState, PresencePeerEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `node` datamodel variable, published (`sce:direction="out"`). */
    private var vNode: UInt = 0.toUInt()

    /** What the published `node` holds. */
    val node: UInt get() = vNode
    /** W3C SCXML 5.2: the `hub` datamodel variable, published (`sce:direction="out"`). */
    private var vHub: Boolean = false

    /** What the published `hub` holds. */
    val hub: Boolean get() = vHub
    /** W3C SCXML 5.2: the `ttl` datamodel variable, published (`sce:direction="out"`). */
    private var vTtl: UInt = 0.toUInt()

    /** What the published `ttl` holds. */
    val ttl: UInt get() = vTtl
    /** W3C SCXML 5.2: the `lease` datamodel variable, published (`sce:direction="out"`). */
    private var vLease: UInt = 0.toUInt()

    /** What the published `lease` holds. */
    val lease: UInt get() = vLease
    /** W3C SCXML 5.2: the `online` datamodel variable, published (`sce:direction="out"`). */
    private var vOnline: List<PresencePeerSeenRecord> = emptyList()

    /** What the published `online` holds. */
    val online: List<PresencePeerSeenRecord> get() = vOnline
    /** W3C SCXML 5.2: the `rebuilt` datamodel variable, the machine's own. */
    private var vRebuilt: List<PresencePeerSeenRecord> = emptyList()
    /** W3C SCXML 5.2: the `entry` datamodel variable, the machine's own. */
    private var vEntry: PresencePeerSeenRecord = PresencePeerSeenRecord(client = 0.toUInt(), last = 0, gen = 0.toUInt())
    /** W3C SCXML 5.2: the `serial` datamodel variable, published (`sce:direction="out"`). */
    private var vSerial: UInt = 0.toUInt()

    /** What the published `serial` holds. */
    val serial: UInt get() = vSerial
    /** W3C SCXML 5.2: the `known` datamodel variable, the machine's own. */
    private var vKnown: Boolean = false
    /** W3C SCXML 5.2: the `fresh` datamodel variable, the machine's own. */
    private var vFresh: Boolean = false
    /** W3C SCXML 5.2: the `lost` datamodel variable, published (`sce:direction="out"`). */
    private var vLost: UInt = 0.toUInt()

    /** What the published `lost` holds. */
    val lost: UInt get() = vLost
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    private var vRefusals: UInt = 0.toUInt()

    /** What the published `refusals` holds. */
    val refusals: UInt get() = vRefusals

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var node: UInt? = null
        var hub: Boolean? = null
        var ttl: UInt? = null
        var lease: UInt? = null
        var serial: UInt? = null
        var known: Boolean? = null
        var fresh: Boolean? = null
        var lost: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.node?.let { vNode = it }
        params.hub?.let { vHub = it }
        params.ttl?.let { vTtl = it }
        params.lease?.let { vLease = it }
        params.serial?.let { vSerial = it }
        params.known?.let { vKnown = it }
        params.fresh?.let { vFresh = it }
        params.lost?.let { vLost = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val node: UInt,
        val hub: Boolean,
        val ttl: UInt,
        val lease: UInt,
        val online: List<PresencePeerSeenRecord>,
        val serial: UInt,
        val lost: UInt,
        val refusals: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<PresencePeerState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        node = vNode,
        hub = vHub,
        ttl = vTtl,
        lease = vLease,
        online = vOnline,
        serial = vSerial,
        lost = vLost,
        refusals = vRefusals,
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
        first = "61319fa1be44c43d13305ba3a241a6aa058882da0d4c2736fb3c896d16fad0e3",
        revised = "b0d96ebe733fde22663f4d265dac2287795efe126ca751ed6753da1ff2c53a4a",
        format2First = "61319fa1be44c43d13305ba3a241a6aa058882da0d4c2736fb3c896d16fad0e3",
        format2Revised = "b0d96ebe733fde22663f4d265dac2287795efe126ca751ed6753da1ff2c53a4a",
        records = listOf(
            RecordLineage(
                event = "presence.seen",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    RecordHolder("online", true),
                    RecordHolder("rebuilt", true),
                    RecordHolder("entry", false),
                ),
            ),
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
            "node" to SavedValues.of(vNode),
            "hub" to SavedValues.of(vHub),
            "ttl" to SavedValues.of(vTtl),
            "lease" to SavedValues.of(vLease),
            "online" to SavedValues.list(vOnline) { it.toSaved() },
            "rebuilt" to SavedValues.list(vRebuilt) { it.toSaved() },
            "entry" to vEntry.toSaved(),
            "serial" to SavedValues.of(vSerial),
            "known" to SavedValues.of(vKnown),
            "fresh" to SavedValues.of(vFresh),
            "lost" to SavedValues.of(vLost),
            "refusals" to SavedValues.of(vRefusals),
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
        val saved1 = SavedValues.uint32(savedGrown.variable("node"), "node")
        val saved2 = SavedValues.bool(savedGrown.variable("hub"), "hub")
        val saved3 = SavedValues.uint32(savedGrown.variable("ttl"), "ttl")
        val saved4 = SavedValues.uint32(savedGrown.variable("lease"), "lease")
        val saved5 = SavedValues.list(savedGrown.variable("online"), "online", 128) { e, w -> PresencePeerSeenRecord.fromSaved(e, w) }
        val saved6 = SavedValues.list(savedGrown.variable("rebuilt"), "rebuilt", 128) { e, w -> PresencePeerSeenRecord.fromSaved(e, w) }
        val saved7 = PresencePeerSeenRecord.fromSaved(savedGrown.variable("entry"), "entry")
        val saved8 = SavedValues.uint32(savedGrown.variable("serial"), "serial")
        val saved9 = SavedValues.bool(savedGrown.variable("known"), "known")
        val saved10 = SavedValues.bool(savedGrown.variable("fresh"), "fresh")
        val saved11 = SavedValues.uint32(savedGrown.variable("lost"), "lost")
        val saved12 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        vNode = saved1
        vHub = saved2
        vTtl = saved3
        vLease = saved4
        vOnline = saved5
        vRebuilt = saved6
        vEntry = saved7
        vSerial = saved8
        vKnown = saved9
        vFresh = saved10
        vLost = saved11
        vRefusals = saved12
        enterSaved(savedGrown, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // docs/adr/0045: this machine as a child session of another's saved state —
    // saved with it, judged with it before anything is built, and restored in
    // place of initialized.
    override fun saveAsSession(wallNowMs: Long): SavedState = save(wallNowMs)

    override fun judgeAsSession(saved: SavedState) {
        val savedGrown = beginRestore(saved, savedShape)
        SavedValues.uint32(savedGrown.variable("node"), "node")
        SavedValues.bool(savedGrown.variable("hub"), "hub")
        SavedValues.uint32(savedGrown.variable("ttl"), "ttl")
        SavedValues.uint32(savedGrown.variable("lease"), "lease")
        SavedValues.list(savedGrown.variable("online"), "online", 128) { e, w -> PresencePeerSeenRecord.fromSaved(e, w) }
        SavedValues.list(savedGrown.variable("rebuilt"), "rebuilt", 128) { e, w -> PresencePeerSeenRecord.fromSaved(e, w) }
        PresencePeerSeenRecord.fromSaved(savedGrown.variable("entry"), "entry")
        SavedValues.uint32(savedGrown.variable("serial"), "serial")
        SavedValues.bool(savedGrown.variable("known"), "known")
        SavedValues.bool(savedGrown.variable("fresh"), "fresh")
        SavedValues.uint32(savedGrown.variable("lost"), "lost")
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingPresenceAlivePayload: PresencePeerPresenceAlivePayload? = null
    private var pendingPresenceBeatPayload: PresencePeerPresenceBeatPayload? = null
    private var pendingPresenceExpirePayload: PresencePeerPresenceExpirePayload? = null
    private var pendingPresenceJoinPayload: PresencePeerPresenceJoinPayload? = null
    private var pendingPresenceLeftPayload: PresencePeerPresenceLeftPayload? = null
    private var pendingPresenceSeenPayload: PresencePeerPresenceSeenPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: PresencePeerEvent, metadata: EventMetadata) {
        pendingPresenceAlivePayload = null
        pendingPresenceBeatPayload = null
        pendingPresenceExpirePayload = null
        pendingPresenceJoinPayload = null
        pendingPresenceLeftPayload = null
        pendingPresenceSeenPayload = null
        when (val tp = metadata.typedPayload) {
            is PresencePeerPresenceAlivePayload -> pendingPresenceAlivePayload = tp
            is PresencePeerPresenceBeatPayload -> pendingPresenceBeatPayload = tp
            is PresencePeerPresenceExpirePayload -> pendingPresenceExpirePayload = tp
            is PresencePeerPresenceJoinPayload -> pendingPresenceJoinPayload = tp
            is PresencePeerPresenceLeftPayload -> pendingPresenceLeftPayload = tp
            is PresencePeerPresenceSeenPayload -> pendingPresenceSeenPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == PresencePeerEvent.Presence.Alive) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingPresenceAlivePayload = PresencePeerPresenceAlivePayload(fields.uint32("client"), fields.int64("at"))
                } else if (event == PresencePeerEvent.Presence.Beat) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingPresenceBeatPayload = PresencePeerPresenceBeatPayload(fields.int64("now"))
                } else if (event == PresencePeerEvent.Presence.Expire) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingPresenceExpirePayload = PresencePeerPresenceExpirePayload(fields.uint32("gen"))
                } else if (event == PresencePeerEvent.Presence.Join) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingPresenceJoinPayload = PresencePeerPresenceJoinPayload(fields.uint32("node"), fields.boolean("hub"), fields.uint32("ttl"), fields.uint32("slack"))
                } else if (event == PresencePeerEvent.Presence.Left) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingPresenceLeftPayload = PresencePeerPresenceLeftPayload(fields.uint32("client"), fields.int64("last"))
                } else if (event == PresencePeerEvent.Presence.Seen) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingPresenceSeenPayload = PresencePeerPresenceSeenPayload(fields.uint32("client"), fields.int64("last"), fields.uint32("gen"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `presence.alive` — binds the event name and the payload field values in one call.
    fun raisePresenceAlive(client: UInt, at: Long) {
        send(
            PresencePeerEvent.Presence.Alive,
            EventMetadata(
                type = "external",
                typedPayload = PresencePeerPresenceAlivePayload(client, at),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("client" to client, "at" to at))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `presence.beat` — binds the event name and the payload field values in one call.
    fun raisePresenceBeat(now: Long) {
        send(
            PresencePeerEvent.Presence.Beat,
            EventMetadata(
                type = "external",
                typedPayload = PresencePeerPresenceBeatPayload(now),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("now" to now))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `presence.expire` — binds the event name and the payload field values in one call.
    fun raisePresenceExpire(gen: UInt) {
        send(
            PresencePeerEvent.Presence.Expire,
            EventMetadata(
                type = "external",
                typedPayload = PresencePeerPresenceExpirePayload(gen),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("gen" to gen))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `presence.join` — binds the event name and the payload field values in one call.
    fun raisePresenceJoin(node: UInt, hub: Boolean, ttl: UInt, slack: UInt) {
        send(
            PresencePeerEvent.Presence.Join,
            EventMetadata(
                type = "external",
                typedPayload = PresencePeerPresenceJoinPayload(node, hub, ttl, slack),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("node" to node, "hub" to hub, "ttl" to ttl, "slack" to slack))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `presence.left` — binds the event name and the payload field values in one call.
    fun raisePresenceLeft(client: UInt, last: Long) {
        send(
            PresencePeerEvent.Presence.Left,
            EventMetadata(
                type = "external",
                typedPayload = PresencePeerPresenceLeftPayload(client, last),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("client" to client, "last" to last))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `presence.seen` — binds the event name and the payload field values in one call.
    fun raisePresenceSeen(client: UInt, last: Long, gen: UInt) {
        send(
            PresencePeerEvent.Presence.Seen,
            EventMetadata(
                type = "external",
                typedPayload = PresencePeerPresenceSeenPayload(client, last, gen),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("client" to client, "last" to last, "gen" to gen))
            )
        )
    }


    override val initialState: PresencePeerState = PresencePeerState.Open

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
    override val documentInitialTargets: List<EntryTarget<PresencePeerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<PresencePeerState, HistoryId>> =
            listOf(StateTarget(PresencePeerState.Open))

        // W3C SCXML 3.13: open's transition 0, as the microstep reads it.
        val transitionOpenAt0 = EnabledTransition<PresencePeerState, HistoryId>(
            PresencePeerState.Open,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 1, as the microstep reads it.
        val transitionOpenAt1 = EnabledTransition<PresencePeerState, HistoryId>(
            PresencePeerState.Open,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 2, as the microstep reads it.
        val transitionOpenAt2 = EnabledTransition<PresencePeerState, HistoryId>(
            PresencePeerState.Open,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 3, as the microstep reads it.
        val transitionOpenAt3 = EnabledTransition<PresencePeerState, HistoryId>(
            PresencePeerState.Open,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 4, as the microstep reads it.
        val transitionOpenAt4 = EnabledTransition<PresencePeerState, HistoryId>(
            PresencePeerState.Open,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 5, as the microstep reads it.
        val transitionOpenAt5 = EnabledTransition<PresencePeerState, HistoryId>(
            PresencePeerState.Open,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 6, as the microstep reads it.
        val transitionOpenAt6 = EnabledTransition<PresencePeerState, HistoryId>(
            PresencePeerState.Open,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 7, as the microstep reads it.
        val transitionOpenAt7 = EnabledTransition<PresencePeerState, HistoryId>(
            PresencePeerState.Open,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): PresencePeerState? = when (stateId) {
        "open" -> PresencePeerState.Open
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: PresencePeerState): String = when (state) {
        is PresencePeerState.Open -> "open"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: PresencePeerState): Int = when (state) {
        is PresencePeerState.Open -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): PresencePeerEvent? = when (name) {
        "error.communication" -> PresencePeerEvent.Error.Communication
        "error.execution" -> PresencePeerEvent.Error.Execution
        "presence.alive" -> PresencePeerEvent.Presence.Alive
        "presence.beat" -> PresencePeerEvent.Presence.Beat
        "presence.expire" -> PresencePeerEvent.Presence.Expire
        "presence.join" -> PresencePeerEvent.Presence.Join
        "presence.left" -> PresencePeerEvent.Presence.Left
        "presence.seen" -> PresencePeerEvent.Presence.Seen
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: PresencePeerEvent): String? = when (event) {
        is PresencePeerEvent.Error.Communication -> "error.communication"
        is PresencePeerEvent.Error.Execution -> "error.execution"
        is PresencePeerEvent.Presence.Alive -> "presence.alive"
        is PresencePeerEvent.Presence.Beat -> "presence.beat"
        is PresencePeerEvent.Presence.Expire -> "presence.expire"
        is PresencePeerEvent.Presence.Join -> "presence.join"
        is PresencePeerEvent.Presence.Left -> "presence.left"
        is PresencePeerEvent.Presence.Seen -> "presence.seen"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: PresencePeerState,
        event: PresencePeerEvent?
    ): EnabledTransition<PresencePeerState, HistoryId>? = when (state) {
        is PresencePeerState.Open -> when {
            event is PresencePeerEvent.Presence.Join -> transitionOpenAt0
            event is PresencePeerEvent.Presence.Beat -> transitionOpenAt1
            event is PresencePeerEvent.Presence.Alive && vHub -> transitionOpenAt2
            event is PresencePeerEvent.Presence.Seen -> transitionOpenAt3
            event is PresencePeerEvent.Presence.Expire -> transitionOpenAt4
            event is PresencePeerEvent.Presence.Left -> transitionOpenAt5
            event is PresencePeerEvent.Error.Communication -> transitionOpenAt6
            event is PresencePeerEvent.Error.Execution -> transitionOpenAt7
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: presence_peer.scxml:52 :: _machine
    override fun onEntry(state: PresencePeerState, isDefaultEntry: Boolean) {
        when (state) {
            is PresencePeerState.Open -> {
                // SCE-MAP: presence_peer.scxml:78 :: open :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: presence_peer.scxml:52 :: _machine
    override fun onExit(state: PresencePeerState) {
        when (state) {
            is PresencePeerState.Open -> {
                // SCE-MAP: presence_peer.scxml:78 :: open :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: presence_peer.scxml:52 :: _machine
    override fun executeTransitionContent(source: PresencePeerState, transitionIndex: Int) {
        when (source) {
        is PresencePeerState.Open -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: presence_peer.scxml:79 :: open :: _transition_0
                if (pendingPresenceJoinPayload == null) {
                    return
                }

            vNode = pendingPresenceJoinPayload!!.node

            vHub = pendingPresenceJoinPayload!!.hub

            vTtl = pendingPresenceJoinPayload!!.ttl

            vLease = pendingPresenceJoinPayload!!.ttl


            if (!pendingPresenceJoinPayload!!.hub) {

            if (try { vLease = com.sce.forge.runtime.SceChecked.add(pendingPresenceJoinPayload!!.ttl, pendingPresenceJoinPayload!!.slack); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(PresencePeerEvent.Error.Execution, "<assign location='lease'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }
            1 -> {
                // SCE-MAP: presence_peer.scxml:93 :: open :: _transition_1
                if (pendingPresenceBeatPayload == null) {
                    return
                }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "client", (vNode).toLong())

            putParam(sendPayload, "at", (pendingPresenceBeatPayload!!.now).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "presence.alive",
                target = "#server",
                content = "",
                params = sendWireParams,
                sendId = "__send_0",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(PresencePeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_0")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            2 -> {
                // SCE-MAP: presence_peer.scxml:103 :: open :: _transition_2
                if (pendingPresenceAlivePayload == null) {
                    return
                }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "client", (pendingPresenceAlivePayload!!.client).toLong())

            putParam(sendPayload, "last", (pendingPresenceAlivePayload!!.at).toString())

            putParam(sendPayload, "gen", (0).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(PresencePeerEvent.Presence.Seen, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "client", (pendingPresenceAlivePayload!!.client).toLong())

            putParam(sendPayload, "last", (pendingPresenceAlivePayload!!.at).toString())

            putParam(sendPayload, "gen", (0).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "presence.seen",
                target = "#room",
                content = "",
                params = sendWireParams,
                sendId = "__send_2",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(PresencePeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_2")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            3 -> {
                // SCE-MAP: presence_peer.scxml:119 :: open :: _transition_3
                if (pendingPresenceSeenPayload == null) {
                    return
                }

            vFresh = true

            vRebuilt = emptyList()


            for (m in vOnline) {


            if (m.client == pendingPresenceSeenPayload!!.client && pendingPresenceSeenPayload!!.last <= m.last) {

            vFresh = false
            }


            if (m.client != pendingPresenceSeenPayload!!.client) {

            if (if (vRebuilt.size < 128) { vRebuilt = vRebuilt + (m); false } else { raisePlatformError(PresencePeerEvent.Error.Execution, "<sce:append target='rebuilt'>: the list already holds its capacity of 128"); true }) {
                return
            }
            }
            }


            if (vFresh) {

            vEntry = vEntry.copy(client = pendingPresenceSeenPayload!!.client)

            vEntry = vEntry.copy(last = pendingPresenceSeenPayload!!.last)

            if (try { vEntry = vEntry.copy(gen = com.sce.forge.runtime.SceChecked.add(vSerial, 1.toUInt())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(PresencePeerEvent.Error.Execution, "<assign location='entry.gen'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (if (vRebuilt.size < 128) { vRebuilt = vRebuilt + (vEntry); false } else { raisePlatformError(PresencePeerEvent.Error.Execution, "<sce:append target='rebuilt'>: the list already holds its capacity of 128"); true }) {
                return
            }

            vOnline = emptyList()


            for (r in vRebuilt) {

            if (if (vOnline.size < 128) { vOnline = vOnline + (r); false } else { raisePlatformError(PresencePeerEvent.Error.Execution, "<sce:append target='online'>: the list already holds its capacity of 128"); true }) {
                return
            }
            }

            if (try { vSerial = com.sce.forge.runtime.SceChecked.add(vSerial, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(PresencePeerEvent.Error.Execution, "<assign location='serial'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (run send@{
            // The delay is a string computed from the machine's fields now, and
            // read as the CSS2 time it must be (ARCHITECTURE.md, "Durations"):
            // an operation that fails, or a value that is no time, is the
            // argument error, and the message is not scheduled under some
            // default wait.
            val sendDelayText: String = vLease.toString() + "ms"
            val sendDelayMs = com.sce.runtime.SendHelper.parseDelayMs(sendDelayText) ?: run {
                raisePlatformError(PresencePeerEvent.Error.Execution, "<send> delayexpr is not a CSS2 time", "__send_3")
                return@send true
            }
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "gen", (vSerial).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_3", sendDelayMs, PresencePeerEvent.Presence.Expire, EventMetadata.external(sendId = "__send_3", origin = sessionId, data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            4 -> {
                // SCE-MAP: presence_peer.scxml:151 :: open :: _transition_4
                if (pendingPresenceExpirePayload == null) {
                    return
                }

            vKnown = false

            vRebuilt = emptyList()


            for (m in vOnline) {


            if (m.gen == pendingPresenceExpirePayload!!.gen) {

            vKnown = true

            vEntry = m
            }


            if (m.gen != pendingPresenceExpirePayload!!.gen) {

            if (if (vRebuilt.size < 128) { vRebuilt = vRebuilt + (m); false } else { raisePlatformError(PresencePeerEvent.Error.Execution, "<sce:append target='rebuilt'>: the list already holds its capacity of 128"); true }) {
                return
            }
            }
            }


            if (vKnown) {

            vOnline = emptyList()


            for (r in vRebuilt) {

            if (if (vOnline.size < 128) { vOnline = vOnline + (r); false } else { raisePlatformError(PresencePeerEvent.Error.Execution, "<sce:append target='online'>: the list already holds its capacity of 128"); true }) {
                return
            }
            }


            if (vHub) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "client", (vEntry.client).toLong())

            putParam(sendPayload, "last", (vEntry.last).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "presence.left",
                target = "#room",
                content = "",
                params = sendWireParams,
                sendId = "__send_4",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(PresencePeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_4")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            }
            5 -> {
                // SCE-MAP: presence_peer.scxml:179 :: open :: _transition_5
                if (pendingPresenceLeftPayload == null) {
                    return
                }

            vRebuilt = emptyList()


            for (m in vOnline) {


            if (m.client != pendingPresenceLeftPayload!!.client || m.last > pendingPresenceLeftPayload!!.last) {

            if (if (vRebuilt.size < 128) { vRebuilt = vRebuilt + (m); false } else { raisePlatformError(PresencePeerEvent.Error.Execution, "<sce:append target='rebuilt'>: the list already holds its capacity of 128"); true }) {
                return
            }
            }
            }

            vOnline = emptyList()


            for (r in vRebuilt) {

            if (if (vOnline.size < 128) { vOnline = vOnline + (r); false } else { raisePlatformError(PresencePeerEvent.Error.Execution, "<sce:append target='online'>: the list already holds its capacity of 128"); true }) {
                return
            }
            }
            }
            6 -> {
                // SCE-MAP: presence_peer.scxml:193 :: open :: _transition_6

            if (try { vLost = com.sce.forge.runtime.SceChecked.add(vLost, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(PresencePeerEvent.Error.Execution, "<assign location='lost'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            7 -> {
                // SCE-MAP: presence_peer.scxml:197 :: open :: _transition_7

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(PresencePeerEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
