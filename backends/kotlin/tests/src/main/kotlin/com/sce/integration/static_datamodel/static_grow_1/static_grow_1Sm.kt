// SCE-GENERATED — DO NOT EDIT
// source-hash: 2348cc6c8de653ee525307cba210b1222ddce00484a0ed7ca1a4a1a75b2abb4c
// source: static_grow_1.scxml

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_grow_1.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_grow_1.scxml:11 :: _machine

package com.sce.integration.static_datamodel.static_grow_1

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticGrow1State : State {
    data object Holding : StaticGrow1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticGrow1Event : Event {
    sealed interface Grow : StaticGrow1Event {
        data object Holding : Grow
        data object Inner : Grow
        data object Item : Grow
        data object Maybe : Grow
        data object Shaped : Grow
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticGrow1Payload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticGrow1GrowHoldingPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `grow.holding`. Consumers inject it via the `raiseGrowHolding` seam
// on the machine — they never name this class directly.
data class StaticGrow1GrowHoldingPayload(val outer: StaticGrow1OuterRecord?)

// StaticGrow1GrowInnerPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `grow.inner`. Consumers inject it via the `raiseGrowInner` seam
// on the machine — they never name this class directly.
data class StaticGrow1GrowInnerPayload(val n: UShort)

// StaticGrow1GrowItemPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `grow.item`. Consumers inject it via the `raiseGrowItem` seam
// on the machine — they never name this class directly.
data class StaticGrow1GrowItemPayload(val n: UShort)

// StaticGrow1GrowMaybePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `grow.maybe`. Consumers inject it via the `raiseGrowMaybe` seam
// on the machine — they never name this class directly.
data class StaticGrow1GrowMaybePayload(val inner: StaticGrow1InnerRecord?)

// StaticGrow1GrowShapedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `grow.shaped`. Consumers inject it via the `raiseGrowShaped` seam
// on the machine — they never name this class directly.
data class StaticGrow1GrowShapedPayload(val shape: StaticGrow1GrowShape1Union)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Inner` datamodel value. */
data class StaticGrow1InnerRecord(val n: UShort) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("n" to SavedValues.of(n))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticGrow1InnerRecord = StaticGrow1InnerRecord(n = SavedValues.uint16(SavedValues.field(value, what, "n"), "$what.n"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Outer` value that holds a composed value (docs/adr/0086). */
data class StaticGrow1OuterRecord(val id: UInt, val inner: StaticGrow1InnerRecord, val inners: List<StaticGrow1InnerRecord>) {
    init {
        com.sce.runtime.ValueBounds.list("inners", inners, 2) { _, _ -> }
    }

    /** What a value is read back from a saved state through (`fromSaved`). */
    companion object
}
/** This value as a saved state writes it. */
fun StaticGrow1OuterRecord.toSaved(): Any = linkedMapOf<String, Any?>("id" to SavedValues.of(this.id), "inner" to this.inner.toSaved(), "inners" to SavedValues.list(this.inners) { sceSaved0 -> sceSaved0.toSaved() })

/** The value a saved state holds, refused unless it is one. */
fun StaticGrow1OuterRecord.Companion.fromSaved(value: Any?, what: String): StaticGrow1OuterRecord = StaticGrow1OuterRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), inner = StaticGrow1InnerRecord.fromSaved(SavedValues.field(value, what, "inner"), "$what.inner"), inners = SavedValues.list(SavedValues.field(value, what, "inners"), "$what.inners", 2) { sceSaved0, sceWhat0 -> StaticGrow1InnerRecord.fromSaved(sceSaved0, sceWhat0) })
/** SCE Accepted Subset §2.15: a `record:Item` datamodel value. */
data class StaticGrow1ItemRecord(val n: UShort) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("n" to SavedValues.of(n))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticGrow1ItemRecord = StaticGrow1ItemRecord(n = SavedValues.uint16(SavedValues.field(value, what, "n"), "$what.n"))
    }
}
/** SCE Accepted Subset §2.15: an `enum:Kind` datamodel value. */
enum class StaticGrow1KindEnum(val declaredName: String) {
    SINGLE("single"),
    NONE("none");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        SINGLE -> 1u.toUByte()
        NONE -> 2u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticGrow1KindEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of Kind")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticGrow1KindEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `union:grow_shape_1` value (docs/adr/0092). */
sealed interface StaticGrow1GrowShape1Union {
    /** The case this value is in, as its tag names it. */
    val kind: StaticGrow1KindEnum

    /** The case `single`, holding its record. */
    data class SINGLE(val value: StaticGrow1InnerRecord) : StaticGrow1GrowShape1Union {
        override val kind: StaticGrow1KindEnum
            get() = StaticGrow1KindEnum.SINGLE
    }

    /** The case `none`, its tag alone. */
    data object NONE : StaticGrow1GrowShape1Union {
        override val kind: StaticGrow1KindEnum
            get() = StaticGrow1KindEnum.NONE
    }

    /** The record of the case `single`, read where a test of the tag proved the value is in it. */
    fun provenSingle(): StaticGrow1InnerRecord =
        (this as? SINGLE)?.value ?: throw IllegalStateException("a value of grow_shape_1 read as single is in another case")

    /** What a value is read back from a saved state through (`fromSaved`). */
    companion object
}
/** This value as a saved state writes it: its tag beside the fields of its case. */
fun StaticGrow1GrowShape1Union.toSaved(): Any = when (this) {
    is StaticGrow1GrowShape1Union.SINGLE -> SavedValues.union("kind", "single", this.value.toSaved())
    StaticGrow1GrowShape1Union.NONE -> SavedValues.union("kind", "none", null)
}

/** The value a saved state holds, read by its tag first, refused unless it is one of the cases of `grow_shape_1`. */
fun StaticGrow1GrowShape1Union.Companion.fromSaved(value: Any?, what: String): StaticGrow1GrowShape1Union =
    when (val sceTag = SavedValues.unionTag(value, what, "kind")) {
        "single" -> StaticGrow1GrowShape1Union.SINGLE(StaticGrow1InnerRecord.fromSaved(value, what))
        "none" -> StaticGrow1GrowShape1Union.NONE
        else -> throw StateRefusal("'$what' ($sceTag) is not a case of grow_shape_1")
    }
// --- State Machine (W3C SCXML) ---

class StaticGrow1StateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticGrow1State, StaticGrow1Event>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `outer` datamodel variable, published (`sce:direction="out"`). */
    private var vOuter: StaticGrow1OuterRecord? = null

    /** What the published `outer` holds. */
    val outer: StaticGrow1OuterRecord? get() = vOuter
    /** W3C SCXML 5.2: the `kept` datamodel variable, published (`sce:direction="out"`). */
    private var vKept: StaticGrow1InnerRecord = StaticGrow1InnerRecord(n = 0.toUShort())

    /** What the published `kept` holds. */
    val kept: StaticGrow1InnerRecord get() = vKept
    /** W3C SCXML 5.2: the `many` datamodel variable, published (`sce:direction="out"`). */
    private var vMany: List<StaticGrow1ItemRecord> = emptyList()

    /** What the published `many` holds. */
    val many: List<StaticGrow1ItemRecord> get() = vMany
    /** W3C SCXML 5.2: the `maybe` datamodel variable, published (`sce:direction="out"`). */
    private var vMaybe: StaticGrow1InnerRecord? = null

    /** What the published `maybe` holds. */
    val maybe: StaticGrow1InnerRecord? get() = vMaybe
    /** W3C SCXML 5.2: the `shape` datamodel variable, published (`sce:direction="out"`). */
    private var vShape: StaticGrow1GrowShape1Union = StaticGrow1GrowShape1Union.NONE

    /** What the published `shape` holds. */
    val shape: StaticGrow1GrowShape1Union get() = vShape

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val outer: StaticGrow1OuterRecord?,
        val kept: StaticGrow1InnerRecord,
        val many: List<StaticGrow1ItemRecord>,
        val maybe: StaticGrow1InnerRecord?,
        val shape: StaticGrow1GrowShape1Union,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticGrow1State>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        outer = vOuter,
        kept = vKept,
        many = vMany,
        maybe = vMaybe,
        shape = vShape,
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
        first = "77ba2dd8755f9bfffa136b3efbf1b24e441ed2a69a6d46e22712dd3e938eeb54",
        revised = "4948eadff2652e6d9009f7ce977ce26e3606fcf222041cd76637ad95931a2b5c",
        format2First = "77ba2dd8755f9bfffa136b3efbf1b24e441ed2a69a6d46e22712dd3e938eeb54",
        format2Revised = "4948eadff2652e6d9009f7ce977ce26e3606fcf222041cd76637ad95931a2b5c",
        records = listOf(
            RecordLineage(
                event = "grow.inner",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("kept", false),
                    RecordHolder("maybe", false),
                ),
            ),
            RecordLineage(
                event = "grow.item",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("many", true),
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
    fun save(wallNowMs: Long): SavedState {
        val variables = LinkedHashMap<String, Any?>()
        saveVariablesPart0(variables)
        return savedState(savedShape, variables, wallNowMs)
    }

    private fun saveVariablesPart0(variables: MutableMap<String, Any?>) {
        variables += "outer" to SavedValues.optional(vOuter) { it.toSaved() }
        variables += "kept" to vKept.toSaved()
        variables += "many" to SavedValues.list(vMany) { it.toSaved() }
        variables += "maybe" to SavedValues.optional(vMaybe) { it.toSaved() }
        variables += "shape" to vShape.toSaved()
    }

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
        // Every part is judged before any is written, so a refused value leaves
        // the machine as it was; then each part reads its values again and
        // writes them. A part answers no closure over the values it read: a
        // closure's captures are the arguments of a JVM method, at most 255
        // slots with a `Long` or a `Double` taking two, and a part of
        // `kotlin_method_items` variables passed that, so the class failed to
        // load (`ClassFormatError: Too many arguments in method signature`).
        judgeVariablesPart0(savedGrown)
        restoreVariablesPart0(savedGrown)
        enterSaved(savedGrown, wallNowMs)
    }

    // Writes this part's variables from [savedGrown], which every judge part
    // has already held to its variables' types.
    private fun restoreVariablesPart0(savedGrown: SavedState) {
        vOuter = SavedValues.optional(savedGrown.variable("outer"), "outer") { e, w -> StaticGrow1OuterRecord.fromSaved(e, w) }
        vKept = StaticGrow1InnerRecord.fromSaved(savedGrown.variable("kept"), "kept")
        vMany = SavedValues.list(savedGrown.variable("many"), "many", 2) { e, w -> StaticGrow1ItemRecord.fromSaved(e, w) }
        vMaybe = SavedValues.optional(savedGrown.variable("maybe"), "maybe") { e, w -> StaticGrow1InnerRecord.fromSaved(e, w) }
        vShape = StaticGrow1GrowShape1Union.fromSaved(savedGrown.variable("shape"), "shape")
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // docs/adr/0045: this machine as a child session of another's saved state —
    // saved with it, judged with it before anything is built, and restored in
    // place of initialized.
    override fun saveAsSession(wallNowMs: Long): SavedState = save(wallNowMs)

    override fun judgeAsSession(saved: SavedState) {
        val savedGrown = beginRestore(saved, savedShape)
        judgeVariablesPart0(savedGrown)
    }

    private fun judgeVariablesPart0(savedGrown: SavedState) {
        SavedValues.optional(savedGrown.variable("outer"), "outer") { e, w -> StaticGrow1OuterRecord.fromSaved(e, w) }
        StaticGrow1InnerRecord.fromSaved(savedGrown.variable("kept"), "kept")
        SavedValues.list(savedGrown.variable("many"), "many", 2) { e, w -> StaticGrow1ItemRecord.fromSaved(e, w) }
        SavedValues.optional(savedGrown.variable("maybe"), "maybe") { e, w -> StaticGrow1InnerRecord.fromSaved(e, w) }
        StaticGrow1GrowShape1Union.fromSaved(savedGrown.variable("shape"), "shape")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingGrowHoldingPayload: StaticGrow1GrowHoldingPayload? = null
    private var pendingGrowInnerPayload: StaticGrow1GrowInnerPayload? = null
    private var pendingGrowItemPayload: StaticGrow1GrowItemPayload? = null
    private var pendingGrowMaybePayload: StaticGrow1GrowMaybePayload? = null
    private var pendingGrowShapedPayload: StaticGrow1GrowShapedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, on which the engine refuses the delivery whole and
    // takes an error.execution in its place (docs/adr/0086, section 4.2). Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticGrow1Event, metadata: EventMetadata) {
        pendingGrowHoldingPayload = null
        pendingGrowInnerPayload = null
        pendingGrowItemPayload = null
        pendingGrowMaybePayload = null
        pendingGrowShapedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticGrow1GrowHoldingPayload -> pendingGrowHoldingPayload = tp
            is StaticGrow1GrowInnerPayload -> pendingGrowInnerPayload = tp
            is StaticGrow1GrowItemPayload -> pendingGrowItemPayload = tp
            is StaticGrow1GrowMaybePayload -> pendingGrowMaybePayload = tp
            is StaticGrow1GrowShapedPayload -> pendingGrowShapedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticGrow1Event.Grow.Holding) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingGrowHoldingPayload = StaticGrow1GrowHoldingPayload(fields.field("outer").let { it.optional { it.record(null) { StaticGrow1OuterRecord(id = it.field("id").let { it.uint32() }, inner = it.field("inner").let { it.record(null) { StaticGrow1InnerRecord(n = it.field("n").let { it.uint16() }) } }, inners = it.field("inners").let { it.list(2) { it.record(null) { StaticGrow1InnerRecord(n = it.field("n").let { it.uint16() }) } } }) } } })
                } else if (event == StaticGrow1Event.Grow.Inner) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingGrowInnerPayload = StaticGrow1GrowInnerPayload(fields.uint16("n"))
                } else if (event == StaticGrow1Event.Grow.Item) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingGrowItemPayload = StaticGrow1GrowItemPayload(fields.uint16("n"))
                } else if (event == StaticGrow1Event.Grow.Maybe) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingGrowMaybePayload = StaticGrow1GrowMaybePayload(fields.field("inner").let { it.optional { it.record(null) { StaticGrow1InnerRecord(n = it.field("n").let { it.uint16() }) } } })
                } else if (event == StaticGrow1Event.Grow.Shaped) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingGrowShapedPayload = StaticGrow1GrowShapedPayload(fields.field("shape").let { when (it.union("kind", { when (val sceName = it.name()) { "single" -> StaticGrow1KindEnum.SINGLE; "none" -> StaticGrow1KindEnum.NONE; else -> throw it.unknownVariant(sceName, "Kind") } }) { sceCase, _ -> sceCase }) { StaticGrow1KindEnum.SINGLE -> StaticGrow1GrowShape1Union.SINGLE(it.let { it.record(null) { StaticGrow1InnerRecord(n = it.field("n").let { it.uint16() }) } }); StaticGrow1KindEnum.NONE -> StaticGrow1GrowShape1Union.NONE } })
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `grow.holding` — binds the event name and the payload field values in one call.
    fun raiseGrowHolding(outer: StaticGrow1OuterRecord?) {
        val scePayload = StaticGrow1GrowHoldingPayload(outer)
        send(
            StaticGrow1Event.Grow.Holding,
            EventMetadata(
                type = "external",
                typedPayload = scePayload,
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("outer" to (scePayload.outer?.let { com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("id" to com.sce.runtime.EventPayload.valueJson(it.id), "inner" to com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("n" to com.sce.runtime.EventPayload.valueJson(it.inner.n))), "inners" to com.sce.runtime.EventPayload.listJson(it.inners) { com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("n" to com.sce.runtime.EventPayload.valueJson(it.n))) })) } ?: com.sce.runtime.EventPayload.valueJson(null)))).json
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `grow.inner` — binds the event name and the payload field values in one call.
    fun raiseGrowInner(n: UShort) {
        send(
            StaticGrow1Event.Grow.Inner,
            EventMetadata(
                type = "external",
                typedPayload = StaticGrow1GrowInnerPayload(n),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("n" to n))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `grow.item` — binds the event name and the payload field values in one call.
    fun raiseGrowItem(n: UShort) {
        send(
            StaticGrow1Event.Grow.Item,
            EventMetadata(
                type = "external",
                typedPayload = StaticGrow1GrowItemPayload(n),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("n" to n))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `grow.maybe` — binds the event name and the payload field values in one call.
    fun raiseGrowMaybe(inner: StaticGrow1InnerRecord?) {
        val scePayload = StaticGrow1GrowMaybePayload(inner)
        send(
            StaticGrow1Event.Grow.Maybe,
            EventMetadata(
                type = "external",
                typedPayload = scePayload,
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("inner" to (scePayload.inner?.let { com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("n" to com.sce.runtime.EventPayload.valueJson(it.n))) } ?: com.sce.runtime.EventPayload.valueJson(null)))).json
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `grow.shaped` — binds the event name and the payload field values in one call.
    fun raiseGrowShaped(shape: StaticGrow1GrowShape1Union) {
        val scePayload = StaticGrow1GrowShapedPayload(shape)
        send(
            StaticGrow1Event.Grow.Shaped,
            EventMetadata(
                type = "external",
                typedPayload = scePayload,
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("shape" to scePayload.shape.let { when (it) { is StaticGrow1GrowShape1Union.SINGLE -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "single", mapOf<String, Any?>("n" to com.sce.runtime.EventPayload.valueJson(it.n))) }; StaticGrow1GrowShape1Union.NONE -> com.sce.runtime.EventPayload.unionJson("kind", "none", emptyMap()) } })).json
            )
        )
    }


    override val initialState: StaticGrow1State = StaticGrow1State.Holding

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
    override val documentInitialTargets: List<EntryTarget<StaticGrow1State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticGrow1State, HistoryId>> =
            listOf(StateTarget(StaticGrow1State.Holding))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: holding's transition 0, as the microstep reads it.
        val transitionHoldingAt0 = EnabledTransition<StaticGrow1State, HistoryId>(
            StaticGrow1State.Holding,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 1, as the microstep reads it.
        val transitionHoldingAt1 = EnabledTransition<StaticGrow1State, HistoryId>(
            StaticGrow1State.Holding,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 2, as the microstep reads it.
        val transitionHoldingAt2 = EnabledTransition<StaticGrow1State, HistoryId>(
            StaticGrow1State.Holding,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 3, as the microstep reads it.
        val transitionHoldingAt3 = EnabledTransition<StaticGrow1State, HistoryId>(
            StaticGrow1State.Holding,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 4, as the microstep reads it.
        val transitionHoldingAt4 = EnabledTransition<StaticGrow1State, HistoryId>(
            StaticGrow1State.Holding,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticGrow1State? = when (stateId) {
        "holding" -> StaticGrow1State.Holding
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticGrow1State): String = when (state) {
        is StaticGrow1State.Holding -> "holding"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticGrow1State): Int = when (state) {
        is StaticGrow1State.Holding -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticGrow1Event? = when (name) {
        "grow.holding" -> StaticGrow1Event.Grow.Holding
        "grow.inner" -> StaticGrow1Event.Grow.Inner
        "grow.item" -> StaticGrow1Event.Grow.Item
        "grow.maybe" -> StaticGrow1Event.Grow.Maybe
        "grow.shaped" -> StaticGrow1Event.Grow.Shaped
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticGrow1Event): String? = when (event) {
        is StaticGrow1Event.Grow.Holding -> "grow.holding"
        is StaticGrow1Event.Grow.Inner -> "grow.inner"
        is StaticGrow1Event.Grow.Item -> "grow.item"
        is StaticGrow1Event.Grow.Maybe -> "grow.maybe"
        is StaticGrow1Event.Grow.Shaped -> "grow.shaped"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticGrow1State, event: StaticGrow1Event?): EnabledTransition<StaticGrow1State, HistoryId>? = when (state) {
        is StaticGrow1State.Holding -> firstEnabledTransitionOfHolding(state, event)
    }


    // holding's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfHolding(state: StaticGrow1State, event: StaticGrow1Event?): EnabledTransition<StaticGrow1State, HistoryId>? = when {
        event is StaticGrow1Event.Grow.Holding -> TransitionsPart0.transitionHoldingAt0
        event is StaticGrow1Event.Grow.Inner -> TransitionsPart0.transitionHoldingAt1
        event is StaticGrow1Event.Grow.Item -> TransitionsPart0.transitionHoldingAt2
        event is StaticGrow1Event.Grow.Maybe -> TransitionsPart0.transitionHoldingAt3
        event is StaticGrow1Event.Grow.Shaped -> TransitionsPart0.transitionHoldingAt4
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_grow_1.scxml:11 :: _machine
    override fun onEntry(state: StaticGrow1State, isDefaultEntry: Boolean) {
        when (state) {
            is StaticGrow1State.Holding -> onEntryOfHolding()
        }
    }


    private fun onEntryOfHolding() {
                // SCE-MAP: static_grow_1.scxml:32 :: holding :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_grow_1.scxml:11 :: _machine
    override fun onExit(state: StaticGrow1State) {
        when (state) {
            is StaticGrow1State.Holding -> onExitOfHolding()
        }
    }


    private fun onExitOfHolding() {
                // SCE-MAP: static_grow_1.scxml:32 :: holding :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_grow_1.scxml:11 :: _machine
    override fun executeTransitionContent(source: StaticGrow1State, transitionIndex: Int) {
        when (source) {
            is StaticGrow1State.Holding -> transitionContentOfHolding(transitionIndex)
        }
    }


    // The content of `holding`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfHolding(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfHoldingAt0()
            1 -> transitionContentOfHoldingAt1()
            2 -> transitionContentOfHoldingAt2()
            3 -> transitionContentOfHoldingAt3()
            4 -> transitionContentOfHoldingAt4()
            else -> {}
        }
    }


    private fun transitionContentOfHoldingAt0() {
                // SCE-MAP: static_grow_1.scxml:33 :: holding :: _transition_0
                if (pendingGrowHoldingPayload == null) {
                    return
                }

            vOuter = pendingGrowHoldingPayload!!.outer
    }

    private fun transitionContentOfHoldingAt1() {
                // SCE-MAP: static_grow_1.scxml:36 :: holding :: _transition_1
                if (pendingGrowInnerPayload == null) {
                    return
                }

            vKept = StaticGrow1InnerRecord(n = pendingGrowInnerPayload!!.n)
    }

    private fun transitionContentOfHoldingAt2() {
                // SCE-MAP: static_grow_1.scxml:39 :: holding :: _transition_2
                if (pendingGrowItemPayload == null) {
                    return
                }

            if (if (vMany.size < 2) { vMany = vMany + (StaticGrow1ItemRecord(n = pendingGrowItemPayload!!.n)); false } else { true }) {
                return
            }
    }

    private fun transitionContentOfHoldingAt3() {
                // SCE-MAP: static_grow_1.scxml:42 :: holding :: _transition_3
                if (pendingGrowMaybePayload == null) {
                    return
                }

            vMaybe = pendingGrowMaybePayload!!.inner
    }

    private fun transitionContentOfHoldingAt4() {
                // SCE-MAP: static_grow_1.scxml:45 :: holding :: _transition_4
                if (pendingGrowShapedPayload == null) {
                    return
                }

            vShape = pendingGrowShapedPayload!!.shape
    }
}
