// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_union.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_union.scxml:21 :: _machine

package com.sce.integration.static_union

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticUnionState : State {
    data object Idle : StaticUnionState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticUnionEvent : Event {
    sealed interface Error : StaticUnionEvent {
        data object Execution : Error
    }
    sealed interface Shape : StaticUnionEvent {
        data object Said : Shape
    }
    sealed interface Shapes : StaticUnionEvent {
        data object Sent : Shapes
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticUnionPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticUnionShapeSaidPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `shape.said`. Consumers inject it via the `raiseShapeSaid` seam
// on the machine — they never name this class directly.
data class StaticUnionShapeSaidPayload(val text: String)

// StaticUnionShapesSentPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `shapes.sent`. Consumers inject it via the `raiseShapesSent` seam
// on the machine — they never name this class directly.
data class StaticUnionShapesSentPayload(val one: StaticUnionUnionShapeUnion, val many: List<StaticUnionUnionShapeUnion>, val maybe: StaticUnionUnionShapeUnion?) {
    init {
        com.sce.runtime.ValueBounds.listWithin("many", many, 1, 3) { _, _ -> }
    }
}


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:Kind` datamodel value. */
enum class StaticUnionKindEnum(val declaredName: String) {
    DOT("dot"),
    CIRCLE("circle"),
    DISC("disc"),
    RECT("rect"),
    LABEL("label"),
    PATH("path");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        DOT -> 0u.toUByte()
        CIRCLE -> 1u.toUByte()
        DISC -> 2u.toUByte()
        RECT -> 3u.toUByte()
        LABEL -> 4u.toUByte()
        PATH -> 5u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticUnionKindEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of Kind")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticUnionKindEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Radius` datamodel value. */
data class StaticUnionRadiusRecord(val radius: UShort) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("radius" to SavedValues.of(radius))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticUnionRadiusRecord = StaticUnionRadiusRecord(radius = SavedValues.uint16(SavedValues.field(value, what, "radius"), "$what.radius"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Rect` datamodel value. */
data class StaticUnionRectRecord(val width: UShort, val height: UShort) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("width" to SavedValues.of(width), "height" to SavedValues.of(height))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticUnionRectRecord = StaticUnionRectRecord(width = SavedValues.uint16(SavedValues.field(value, what, "width"), "$what.width"), height = SavedValues.uint16(SavedValues.field(value, what, "height"), "$what.height"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Label` datamodel value. */
data class StaticUnionLabelRecord(val text: String, val note: String?) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("text" to SavedValues.of(text), "note" to SavedValues.optional(note) { SavedValues.of(it) })

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticUnionLabelRecord = StaticUnionLabelRecord(text = SavedValues.string(SavedValues.field(value, what, "text"), "$what.text", 8), note = SavedValues.optional(SavedValues.field(value, what, "note"), "$what.note") { e, w -> SavedValues.string(e, w, 4) })
    }
}
/** SCE Accepted Subset §2.15: a `record:Path` value that holds a composed value (docs/adr/0086). */
data class StaticUnionPathRecord(val points: List<UByte>) {
    init {
        com.sce.runtime.ValueBounds.list("points", points, 3) { _, _ -> }
    }

    /** What a value is read back from a saved state through (`fromSaved`). */
    companion object
}
/** This value as a saved state writes it. */
fun StaticUnionPathRecord.toSaved(): Any = linkedMapOf<String, Any?>("points" to SavedValues.list(this.points) { sceSaved0 -> SavedValues.of(sceSaved0) })

/** The value a saved state holds, refused unless it is one. */
fun StaticUnionPathRecord.Companion.fromSaved(value: Any?, what: String): StaticUnionPathRecord = StaticUnionPathRecord(points = SavedValues.list(SavedValues.field(value, what, "points"), "$what.points", 3) { sceSaved0, sceWhat0 -> SavedValues.uint8(sceSaved0, sceWhat0) })
/** SCE Accepted Subset §2.15: a `union:union_shape` value (docs/adr/0092). */
sealed interface StaticUnionUnionShapeUnion {
    /** The case this value is in, as its tag names it. */
    val kind: StaticUnionKindEnum

    /** The case `dot`, its tag alone. */
    data object DOT : StaticUnionUnionShapeUnion {
        override val kind: StaticUnionKindEnum
            get() = StaticUnionKindEnum.DOT
    }

    /** The case `circle`, holding its record. */
    data class CIRCLE(val value: StaticUnionRadiusRecord) : StaticUnionUnionShapeUnion {
        override val kind: StaticUnionKindEnum
            get() = StaticUnionKindEnum.CIRCLE
    }

    /** The case `disc`, holding its record. */
    data class DISC(val value: StaticUnionRadiusRecord) : StaticUnionUnionShapeUnion {
        override val kind: StaticUnionKindEnum
            get() = StaticUnionKindEnum.DISC
    }

    /** The case `rect`, holding its record. */
    data class RECT(val value: StaticUnionRectRecord) : StaticUnionUnionShapeUnion {
        override val kind: StaticUnionKindEnum
            get() = StaticUnionKindEnum.RECT
    }

    /** The case `label`, holding its record. */
    data class LABEL(val value: StaticUnionLabelRecord) : StaticUnionUnionShapeUnion {
        override val kind: StaticUnionKindEnum
            get() = StaticUnionKindEnum.LABEL
    }

    /** The case `path`, holding its record. */
    data class PATH(val value: StaticUnionPathRecord) : StaticUnionUnionShapeUnion {
        override val kind: StaticUnionKindEnum
            get() = StaticUnionKindEnum.PATH
    }

    /** The record of the case `circle`, read where a test of the tag proved the value is in it. */
    fun provenCircle(): StaticUnionRadiusRecord =
        (this as? CIRCLE)?.value ?: throw IllegalStateException("a value of union_shape read as circle is in another case")

    /** The record of the case `disc`, read where a test of the tag proved the value is in it. */
    fun provenDisc(): StaticUnionRadiusRecord =
        (this as? DISC)?.value ?: throw IllegalStateException("a value of union_shape read as disc is in another case")

    /** The record of the case `rect`, read where a test of the tag proved the value is in it. */
    fun provenRect(): StaticUnionRectRecord =
        (this as? RECT)?.value ?: throw IllegalStateException("a value of union_shape read as rect is in another case")

    /** The record of the case `label`, read where a test of the tag proved the value is in it. */
    fun provenLabel(): StaticUnionLabelRecord =
        (this as? LABEL)?.value ?: throw IllegalStateException("a value of union_shape read as label is in another case")

    /** The record of the case `path`, read where a test of the tag proved the value is in it. */
    fun provenPath(): StaticUnionPathRecord =
        (this as? PATH)?.value ?: throw IllegalStateException("a value of union_shape read as path is in another case")

    /** The field `radius` the cases that declare it share, read where a test of the tag proved the value is in one of them. */
    fun provenFieldRadius(): UShort = when (this) {
        is CIRCLE -> this.value.radius
        is DISC -> this.value.radius
        else -> throw IllegalStateException("a value of union_shape read as radius is in another case")
    }

    /** This value with the field `radius` of its case replaced by [sceValue], where a test of the tag proved the value is in a case that declares it. */
    fun provenFieldRadiusWith(sceValue: UShort): StaticUnionUnionShapeUnion = when (this) {
        is CIRCLE -> CIRCLE(this.value.copy(radius = sceValue))
        is DISC -> DISC(this.value.copy(radius = sceValue))
        else -> throw IllegalStateException("a value of union_shape read as radius is in another case")
    }

    /** What a value is read back from a saved state through (`fromSaved`). */
    companion object
}
/** This value as a saved state writes it: its tag beside the fields of its case. */
fun StaticUnionUnionShapeUnion.toSaved(): Any = when (this) {
    StaticUnionUnionShapeUnion.DOT -> SavedValues.union("kind", "dot", null)
    is StaticUnionUnionShapeUnion.CIRCLE -> SavedValues.union("kind", "circle", this.value.toSaved())
    is StaticUnionUnionShapeUnion.DISC -> SavedValues.union("kind", "disc", this.value.toSaved())
    is StaticUnionUnionShapeUnion.RECT -> SavedValues.union("kind", "rect", this.value.toSaved())
    is StaticUnionUnionShapeUnion.LABEL -> SavedValues.union("kind", "label", this.value.toSaved())
    is StaticUnionUnionShapeUnion.PATH -> SavedValues.union("kind", "path", this.value.toSaved())
}

/** The value a saved state holds, read by its tag first, refused unless it is one of the cases of `union_shape`. */
fun StaticUnionUnionShapeUnion.Companion.fromSaved(value: Any?, what: String): StaticUnionUnionShapeUnion =
    when (val sceTag = SavedValues.unionTag(value, what, "kind")) {
        "dot" -> StaticUnionUnionShapeUnion.DOT
        "circle" -> StaticUnionUnionShapeUnion.CIRCLE(StaticUnionRadiusRecord.fromSaved(value, what))
        "disc" -> StaticUnionUnionShapeUnion.DISC(StaticUnionRadiusRecord.fromSaved(value, what))
        "rect" -> StaticUnionUnionShapeUnion.RECT(StaticUnionRectRecord.fromSaved(value, what))
        "label" -> StaticUnionUnionShapeUnion.LABEL(StaticUnionLabelRecord.fromSaved(value, what))
        "path" -> StaticUnionUnionShapeUnion.PATH(StaticUnionPathRecord.fromSaved(value, what))
        else -> throw StateRefusal("'$what' ($sceTag) is not a case of union_shape")
    }
/** SCE Accepted Subset §2.15: an `enum:refusal_code` datamodel value. */
enum class StaticUnionRefusalCodeEnum(val declaredName: String) {
    NOT_JSON("notJson"),
    NOT_OBJECT("notObject"),
    TOO_DEEP("tooDeep"),
    MISSING("missing"),
    NULL_NOT_OPTIONAL("nullNotOptional"),
    WRONG_KIND("wrongKind"),
    BAD_NUMBER("badNumber"),
    OUT_OF_RANGE("outOfRange"),
    TOO_LONG("tooLong"),
    TOO_MANY("tooMany"),
    UNKNOWN_VARIANT("unknownVariant"),
    NOT_BYTE_TEXT("notByteText"),
    NUL_IN_TEXT("nulInText"),
    TOO_FEW("tooFew");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        NOT_JSON -> 1u.toUByte()
        NOT_OBJECT -> 2u.toUByte()
        TOO_DEEP -> 3u.toUByte()
        MISSING -> 4u.toUByte()
        NULL_NOT_OPTIONAL -> 5u.toUByte()
        WRONG_KIND -> 6u.toUByte()
        BAD_NUMBER -> 7u.toUByte()
        OUT_OF_RANGE -> 8u.toUByte()
        TOO_LONG -> 9u.toUByte()
        TOO_MANY -> 10u.toUByte()
        UNKNOWN_VARIANT -> 11u.toUByte()
        NOT_BYTE_TEXT -> 12u.toUByte()
        NUL_IN_TEXT -> 13u.toUByte()
        TOO_FEW -> 14u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticUnionRefusalCodeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of refusal_code")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticUnionRefusalCodeEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:LiftRefusal` datamodel value. */
data class StaticUnionLiftRefusalRecord(val code: StaticUnionRefusalCodeEnum, val path: String, val bound: UInt?) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("code" to code.toSaved(), "path" to SavedValues.of(path), "bound" to SavedValues.optional(bound) { SavedValues.of(it) })

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticUnionLiftRefusalRecord = StaticUnionLiftRefusalRecord(code = StaticUnionRefusalCodeEnum.fromSaved(SavedValues.field(value, what, "code"), "$what.code"), path = SavedValues.string(SavedValues.field(value, what, "path"), "$what.path", 256), bound = SavedValues.optional(SavedValues.field(value, what, "bound"), "$what.bound") { e, w -> SavedValues.uint32(e, w) })
    }
}
/** The bounds this machine is built with, each by the name `sce:limit` gives it (docs/adr/0050). */
object StaticUnionLimits {
    /** The bound `sce:limit="lift_path"` names, as this build resolved it. */
    const val LIFT_PATH: UInt = 256u
}
// --- State Machine (W3C SCXML) ---

class StaticUnionStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticUnionState, StaticUnionEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `built` datamodel variable, published (`sce:direction="out"`). */
    private var vBuilt: StaticUnionUnionShapeUnion = StaticUnionUnionShapeUnion.RECT(StaticUnionRectRecord(width = 1.toUShort(), height = 2.toUShort()))

    /** What the published `built` holds. */
    val built: StaticUnionUnionShapeUnion get() = vBuilt
    /** W3C SCXML 5.2: the `kept` datamodel variable, published (`sce:direction="out"`). */
    private var vKept: StaticUnionUnionShapeUnion? = null

    /** What the published `kept` holds. */
    val kept: StaticUnionUnionShapeUnion? get() = vKept
    /** W3C SCXML 5.2: the `last_kind` datamodel variable, published (`sce:direction="out"`). */
    private var vLastKind: StaticUnionKindEnum = StaticUnionKindEnum.DOT

    /** What the published `last_kind` holds. */
    val lastKind: StaticUnionKindEnum get() = vLastKind
    /** W3C SCXML 5.2: the `radius` datamodel variable, published (`sce:direction="out"`). */
    private var vRadius: UShort = 0.toUShort()

    /** What the published `radius` holds. */
    val radius: UShort get() = vRadius
    /** W3C SCXML 5.2: the `width` datamodel variable, published (`sce:direction="out"`). */
    private var vWidth: UShort = 0.toUShort()

    /** What the published `width` holds. */
    val width: UShort get() = vWidth
    /** W3C SCXML 5.2: the `height` datamodel variable, published (`sce:direction="out"`). */
    private var vHeight: UShort = 0.toUShort()

    /** What the published `height` holds. */
    val height: UShort get() = vHeight
    /** W3C SCXML 5.2: the `labels` datamodel variable, published (`sce:direction="out"`). */
    private var vLabels: UInt = 0.toUInt()

    /** What the published `labels` holds. */
    val labels: UInt get() = vLabels
    /** W3C SCXML 5.2: the `points` datamodel variable, published (`sce:direction="out"`). */
    private var vPoints: UInt = 0.toUInt()

    /** What the published `points` holds. */
    val points: UInt get() = vPoints
    /** W3C SCXML 5.2: the `dots` datamodel variable, published (`sce:direction="out"`). */
    private var vDots: UInt = 0.toUInt()

    /** What the published `dots` holds. */
    val dots: UInt get() = vDots
    /** W3C SCXML 5.2: the `many_radius` datamodel variable, published (`sce:direction="out"`). */
    private var vManyRadius: UShort = 0.toUShort()

    /** What the published `many_radius` holds. */
    val manyRadius: UShort get() = vManyRadius
    /** W3C SCXML 5.2: the `many_width` datamodel variable, published (`sce:direction="out"`). */
    private var vManyWidth: UShort = 0.toUShort()

    /** What the published `many_width` holds. */
    val manyWidth: UShort get() = vManyWidth
    /** W3C SCXML 5.2: the `many_shapes` datamodel variable, published (`sce:direction="out"`). */
    private var vManyShapes: UInt = 0.toUInt()

    /** What the published `many_shapes` holds. */
    val manyShapes: UInt get() = vManyShapes
    /** W3C SCXML 5.2: the `maybe_width` datamodel variable, published (`sce:direction="out"`). */
    private var vMaybeWidth: UShort = 0.toUShort()

    /** What the published `maybe_width` holds. */
    val maybeWidth: UShort get() = vMaybeWidth
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    private var vRefusals: UInt = 0.toUInt()

    /** What the published `refusals` holds. */
    val refusals: UInt get() = vRefusals
    /** W3C SCXML 5.2: the `lifted` datamodel variable, published (`sce:direction="out"`). */
    private var vLifted: StaticUnionUnionShapeUnion? = null

    /** What the published `lifted` holds. */
    val lifted: StaticUnionUnionShapeUnion? get() = vLifted
    /** W3C SCXML 5.2: the `lift_why` datamodel variable, published (`sce:direction="out"`). */
    private var vLiftWhy: StaticUnionLiftRefusalRecord? = null

    /** What the published `lift_why` holds. */
    val liftWhy: StaticUnionLiftRefusalRecord? get() = vLiftWhy

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var radius: UShort? = null
        var width: UShort? = null
        var height: UShort? = null
        var labels: UInt? = null
        var points: UInt? = null
        var dots: UInt? = null
        var manyRadius: UShort? = null
        var manyWidth: UShort? = null
        var manyShapes: UInt? = null
        var maybeWidth: UShort? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.radius?.let { vRadius = it }
        params.width?.let { vWidth = it }
        params.height?.let { vHeight = it }
        params.labels?.let { vLabels = it }
        params.points?.let { vPoints = it }
        params.dots?.let { vDots = it }
        params.manyRadius?.let { vManyRadius = it }
        params.manyWidth?.let { vManyWidth = it }
        params.manyShapes?.let { vManyShapes = it }
        params.maybeWidth?.let { vMaybeWidth = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val built: StaticUnionUnionShapeUnion,
        val kept: StaticUnionUnionShapeUnion?,
        val lastKind: StaticUnionKindEnum,
        val radius: UShort,
        val width: UShort,
        val height: UShort,
        val labels: UInt,
        val points: UInt,
        val dots: UInt,
        val manyRadius: UShort,
        val manyWidth: UShort,
        val manyShapes: UInt,
        val maybeWidth: UShort,
        val refusals: UInt,
        val lifted: StaticUnionUnionShapeUnion?,
        val liftWhy: StaticUnionLiftRefusalRecord?,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticUnionState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        built = vBuilt,
        kept = vKept,
        lastKind = vLastKind,
        radius = vRadius,
        width = vWidth,
        height = vHeight,
        labels = vLabels,
        points = vPoints,
        dots = vDots,
        manyRadius = vManyRadius,
        manyWidth = vManyWidth,
        manyShapes = vManyShapes,
        maybeWidth = vMaybeWidth,
        refusals = vRefusals,
        lifted = vLifted,
        liftWhy = vLiftWhy,
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
        first = "c8aae7160c566c907be2b8a15519d2845a3fcb1f4beee776eb7dc057f573b9d0",
        revised = "50df1ca63466918d7d1c99c7d3975685a6cc8ffbf52ad6d968bf883193868735",
        format2First = "393e1fdb3bf8c00b6afe66f251168a999d043b2f4599b3e4b80a02f0ec36959c",
        format2Revised = "53f8ecdef3f99a3dcfa54ea7ae234a87745f1c979b3cc297ad413c2d9a953b2d",
        records = listOf(
            RecordLineage(
                event = "lift.refusal",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("lift_why", false),
                ),
            ),
        ),
        enums = listOf(
            EnumLineage(
                key = "enum:enum_shape_kind.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("last_kind", null, false),
                ),
            ),
            EnumLineage(
                key = "enum:sce:std/lift/refusal_code.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("lift_why", "code", false),
                ),
            ),
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
        variables += "built" to vBuilt.toSaved()
        variables += "kept" to SavedValues.optional(vKept) { it.toSaved() }
        variables += "last_kind" to vLastKind.toSaved()
        variables += "radius" to SavedValues.of(vRadius)
        variables += "width" to SavedValues.of(vWidth)
        variables += "height" to SavedValues.of(vHeight)
        variables += "labels" to SavedValues.of(vLabels)
        variables += "points" to SavedValues.of(vPoints)
        variables += "dots" to SavedValues.of(vDots)
        variables += "many_radius" to SavedValues.of(vManyRadius)
        variables += "many_width" to SavedValues.of(vManyWidth)
        variables += "many_shapes" to SavedValues.of(vManyShapes)
        variables += "maybe_width" to SavedValues.of(vMaybeWidth)
        variables += "refusals" to SavedValues.of(vRefusals)
        variables += "lifted" to SavedValues.optional(vLifted) { it.toSaved() }
        variables += "lift_why" to SavedValues.optional(vLiftWhy) { it.toSaved() }
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
        val writeVariablesPart0 = restoreVariablesPart0(savedGrown)
        writeVariablesPart0()
        enterSaved(savedGrown, wallNowMs)
    }

    // Reads this part's variables from [savedGrown], refusing a value its
    // variable's type cannot hold, and answers what writes them: nothing is
    // written until every part has read.
    private fun restoreVariablesPart0(savedGrown: SavedState): () -> Unit {
        val saved1 = StaticUnionUnionShapeUnion.fromSaved(savedGrown.variable("built"), "built")
        val saved2 = SavedValues.optional(savedGrown.variable("kept"), "kept") { e, w -> StaticUnionUnionShapeUnion.fromSaved(e, w) }
        val saved3 = StaticUnionKindEnum.fromSaved(savedGrown.variable("last_kind"), "last_kind")
        val saved4 = SavedValues.uint16(savedGrown.variable("radius"), "radius")
        val saved5 = SavedValues.uint16(savedGrown.variable("width"), "width")
        val saved6 = SavedValues.uint16(savedGrown.variable("height"), "height")
        val saved7 = SavedValues.uint32(savedGrown.variable("labels"), "labels")
        val saved8 = SavedValues.uint32(savedGrown.variable("points"), "points")
        val saved9 = SavedValues.uint32(savedGrown.variable("dots"), "dots")
        val saved10 = SavedValues.uint16(savedGrown.variable("many_radius"), "many_radius")
        val saved11 = SavedValues.uint16(savedGrown.variable("many_width"), "many_width")
        val saved12 = SavedValues.uint32(savedGrown.variable("many_shapes"), "many_shapes")
        val saved13 = SavedValues.uint16(savedGrown.variable("maybe_width"), "maybe_width")
        val saved14 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        val saved15 = SavedValues.optional(savedGrown.variable("lifted"), "lifted") { e, w -> StaticUnionUnionShapeUnion.fromSaved(e, w) }
        val saved16 = SavedValues.optional(savedGrown.variable("lift_why"), "lift_why") { e, w -> StaticUnionLiftRefusalRecord.fromSaved(e, w) }
        return {
            vBuilt = saved1
            vKept = saved2
            vLastKind = saved3
            vRadius = saved4
            vWidth = saved5
            vHeight = saved6
            vLabels = saved7
            vPoints = saved8
            vDots = saved9
            vManyRadius = saved10
            vManyWidth = saved11
            vManyShapes = saved12
            vMaybeWidth = saved13
            vRefusals = saved14
            vLifted = saved15
            vLiftWhy = saved16
        }
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
        StaticUnionUnionShapeUnion.fromSaved(savedGrown.variable("built"), "built")
        SavedValues.optional(savedGrown.variable("kept"), "kept") { e, w -> StaticUnionUnionShapeUnion.fromSaved(e, w) }
        StaticUnionKindEnum.fromSaved(savedGrown.variable("last_kind"), "last_kind")
        SavedValues.uint16(savedGrown.variable("radius"), "radius")
        SavedValues.uint16(savedGrown.variable("width"), "width")
        SavedValues.uint16(savedGrown.variable("height"), "height")
        SavedValues.uint32(savedGrown.variable("labels"), "labels")
        SavedValues.uint32(savedGrown.variable("points"), "points")
        SavedValues.uint32(savedGrown.variable("dots"), "dots")
        SavedValues.uint16(savedGrown.variable("many_radius"), "many_radius")
        SavedValues.uint16(savedGrown.variable("many_width"), "many_width")
        SavedValues.uint32(savedGrown.variable("many_shapes"), "many_shapes")
        SavedValues.uint16(savedGrown.variable("maybe_width"), "maybe_width")
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        SavedValues.optional(savedGrown.variable("lifted"), "lifted") { e, w -> StaticUnionUnionShapeUnion.fromSaved(e, w) }
        SavedValues.optional(savedGrown.variable("lift_why"), "lift_why") { e, w -> StaticUnionLiftRefusalRecord.fromSaved(e, w) }
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingShapeSaidPayload: StaticUnionShapeSaidPayload? = null
    private var pendingShapesSentPayload: StaticUnionShapesSentPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, on which the engine refuses the delivery whole and
    // takes an error.execution in its place (docs/adr/0086, section 4.2). Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticUnionEvent, metadata: EventMetadata) {
        pendingShapeSaidPayload = null
        pendingShapesSentPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticUnionShapeSaidPayload -> pendingShapeSaidPayload = tp
            is StaticUnionShapesSentPayload -> pendingShapesSentPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticUnionEvent.Shape.Said) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingShapeSaidPayload = StaticUnionShapeSaidPayload(fields.string("text", 96))
                } else if (event == StaticUnionEvent.Shapes.Sent) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingShapesSentPayload = StaticUnionShapesSentPayload(fields.field("one").let { when (it.union("kind", { when (val sceName = it.name()) { "dot" -> StaticUnionKindEnum.DOT; "circle" -> StaticUnionKindEnum.CIRCLE; "disc" -> StaticUnionKindEnum.DISC; "rect" -> StaticUnionKindEnum.RECT; "label" -> StaticUnionKindEnum.LABEL; "path" -> StaticUnionKindEnum.PATH; else -> throw it.unknownVariant(sceName, "Kind") } }) { sceCase, _ -> sceCase }) { StaticUnionKindEnum.DOT -> StaticUnionUnionShapeUnion.DOT; StaticUnionKindEnum.CIRCLE -> StaticUnionUnionShapeUnion.CIRCLE(it.let { it.record(null) { StaticUnionRadiusRecord(radius = it.field("radius").let { it.uint16() }) } }); StaticUnionKindEnum.DISC -> StaticUnionUnionShapeUnion.DISC(it.let { it.record(null) { StaticUnionRadiusRecord(radius = it.field("radius").let { it.uint16() }) } }); StaticUnionKindEnum.RECT -> StaticUnionUnionShapeUnion.RECT(it.let { it.record(null) { StaticUnionRectRecord(width = it.field("width").let { it.uint16() }, height = it.field("height").let { it.uint16() }) } }); StaticUnionKindEnum.LABEL -> StaticUnionUnionShapeUnion.LABEL(it.let { it.record(null) { StaticUnionLabelRecord(text = it.field("text").let { it.string(8) }, note = it.field("note").let { it.optional { it.string(4) } }) } }); StaticUnionKindEnum.PATH -> StaticUnionUnionShapeUnion.PATH(it.let { it.record(null) { StaticUnionPathRecord(points = it.field("points").let { it.list(3) { it.uint8() } }) } }) } }, fields.field("many").let { it.listWithin(1, 3) { when (it.union("kind", { when (val sceName = it.name()) { "dot" -> StaticUnionKindEnum.DOT; "circle" -> StaticUnionKindEnum.CIRCLE; "disc" -> StaticUnionKindEnum.DISC; "rect" -> StaticUnionKindEnum.RECT; "label" -> StaticUnionKindEnum.LABEL; "path" -> StaticUnionKindEnum.PATH; else -> throw it.unknownVariant(sceName, "Kind") } }) { sceCase, _ -> sceCase }) { StaticUnionKindEnum.DOT -> StaticUnionUnionShapeUnion.DOT; StaticUnionKindEnum.CIRCLE -> StaticUnionUnionShapeUnion.CIRCLE(it.let { it.record(null) { StaticUnionRadiusRecord(radius = it.field("radius").let { it.uint16() }) } }); StaticUnionKindEnum.DISC -> StaticUnionUnionShapeUnion.DISC(it.let { it.record(null) { StaticUnionRadiusRecord(radius = it.field("radius").let { it.uint16() }) } }); StaticUnionKindEnum.RECT -> StaticUnionUnionShapeUnion.RECT(it.let { it.record(null) { StaticUnionRectRecord(width = it.field("width").let { it.uint16() }, height = it.field("height").let { it.uint16() }) } }); StaticUnionKindEnum.LABEL -> StaticUnionUnionShapeUnion.LABEL(it.let { it.record(null) { StaticUnionLabelRecord(text = it.field("text").let { it.string(8) }, note = it.field("note").let { it.optional { it.string(4) } }) } }); StaticUnionKindEnum.PATH -> StaticUnionUnionShapeUnion.PATH(it.let { it.record(null) { StaticUnionPathRecord(points = it.field("points").let { it.list(3) { it.uint8() } }) } }) } } }, fields.field("maybe").let { it.optional { when (it.union("kind", { when (val sceName = it.name()) { "dot" -> StaticUnionKindEnum.DOT; "circle" -> StaticUnionKindEnum.CIRCLE; "disc" -> StaticUnionKindEnum.DISC; "rect" -> StaticUnionKindEnum.RECT; "label" -> StaticUnionKindEnum.LABEL; "path" -> StaticUnionKindEnum.PATH; else -> throw it.unknownVariant(sceName, "Kind") } }) { sceCase, _ -> sceCase }) { StaticUnionKindEnum.DOT -> StaticUnionUnionShapeUnion.DOT; StaticUnionKindEnum.CIRCLE -> StaticUnionUnionShapeUnion.CIRCLE(it.let { it.record(null) { StaticUnionRadiusRecord(radius = it.field("radius").let { it.uint16() }) } }); StaticUnionKindEnum.DISC -> StaticUnionUnionShapeUnion.DISC(it.let { it.record(null) { StaticUnionRadiusRecord(radius = it.field("radius").let { it.uint16() }) } }); StaticUnionKindEnum.RECT -> StaticUnionUnionShapeUnion.RECT(it.let { it.record(null) { StaticUnionRectRecord(width = it.field("width").let { it.uint16() }, height = it.field("height").let { it.uint16() }) } }); StaticUnionKindEnum.LABEL -> StaticUnionUnionShapeUnion.LABEL(it.let { it.record(null) { StaticUnionLabelRecord(text = it.field("text").let { it.string(8) }, note = it.field("note").let { it.optional { it.string(4) } }) } }); StaticUnionKindEnum.PATH -> StaticUnionUnionShapeUnion.PATH(it.let { it.record(null) { StaticUnionPathRecord(points = it.field("points").let { it.list(3) { it.uint8() } }) } }) } } })
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `shape.said` — binds the event name and the payload field values in one call.
    fun raiseShapeSaid(text: String) {
        send(
            StaticUnionEvent.Shape.Said,
            EventMetadata(
                type = "external",
                typedPayload = StaticUnionShapeSaidPayload(text),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("text" to text))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `shapes.sent` — binds the event name and the payload field values in one call.
    fun raiseShapesSent(one: StaticUnionUnionShapeUnion, many: List<StaticUnionUnionShapeUnion>, maybe: StaticUnionUnionShapeUnion?) {
        val scePayload = StaticUnionShapesSentPayload(one, many, maybe)
        send(
            StaticUnionEvent.Shapes.Sent,
            EventMetadata(
                type = "external",
                typedPayload = scePayload,
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("one" to (scePayload.one).let { when (it) { StaticUnionUnionShapeUnion.DOT -> com.sce.runtime.EventPayload.unionJson("kind", "dot", emptyMap()); is StaticUnionUnionShapeUnion.CIRCLE -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "circle", mapOf<String, Any?>("radius" to com.sce.runtime.EventPayload.valueJson(it.radius))) }; is StaticUnionUnionShapeUnion.DISC -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "disc", mapOf<String, Any?>("radius" to com.sce.runtime.EventPayload.valueJson(it.radius))) }; is StaticUnionUnionShapeUnion.RECT -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "rect", mapOf<String, Any?>("width" to com.sce.runtime.EventPayload.valueJson(it.width), "height" to com.sce.runtime.EventPayload.valueJson(it.height))) }; is StaticUnionUnionShapeUnion.LABEL -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "label", mapOf<String, Any?>("text" to com.sce.runtime.EventPayload.valueJson(it.text), "note" to ((it.note)?.let { com.sce.runtime.EventPayload.valueJson(it) } ?: com.sce.runtime.EventPayload.valueJson(null)))) }; is StaticUnionUnionShapeUnion.PATH -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "path", mapOf<String, Any?>("points" to com.sce.runtime.EventPayload.listJson(it.points) { com.sce.runtime.EventPayload.valueJson(it) })) } } }, "many" to com.sce.runtime.EventPayload.listJson(scePayload.many) { (it).let { when (it) { StaticUnionUnionShapeUnion.DOT -> com.sce.runtime.EventPayload.unionJson("kind", "dot", emptyMap()); is StaticUnionUnionShapeUnion.CIRCLE -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "circle", mapOf<String, Any?>("radius" to com.sce.runtime.EventPayload.valueJson(it.radius))) }; is StaticUnionUnionShapeUnion.DISC -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "disc", mapOf<String, Any?>("radius" to com.sce.runtime.EventPayload.valueJson(it.radius))) }; is StaticUnionUnionShapeUnion.RECT -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "rect", mapOf<String, Any?>("width" to com.sce.runtime.EventPayload.valueJson(it.width), "height" to com.sce.runtime.EventPayload.valueJson(it.height))) }; is StaticUnionUnionShapeUnion.LABEL -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "label", mapOf<String, Any?>("text" to com.sce.runtime.EventPayload.valueJson(it.text), "note" to ((it.note)?.let { com.sce.runtime.EventPayload.valueJson(it) } ?: com.sce.runtime.EventPayload.valueJson(null)))) }; is StaticUnionUnionShapeUnion.PATH -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "path", mapOf<String, Any?>("points" to com.sce.runtime.EventPayload.listJson(it.points) { com.sce.runtime.EventPayload.valueJson(it) })) } } } }, "maybe" to ((scePayload.maybe)?.let { (it).let { when (it) { StaticUnionUnionShapeUnion.DOT -> com.sce.runtime.EventPayload.unionJson("kind", "dot", emptyMap()); is StaticUnionUnionShapeUnion.CIRCLE -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "circle", mapOf<String, Any?>("radius" to com.sce.runtime.EventPayload.valueJson(it.radius))) }; is StaticUnionUnionShapeUnion.DISC -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "disc", mapOf<String, Any?>("radius" to com.sce.runtime.EventPayload.valueJson(it.radius))) }; is StaticUnionUnionShapeUnion.RECT -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "rect", mapOf<String, Any?>("width" to com.sce.runtime.EventPayload.valueJson(it.width), "height" to com.sce.runtime.EventPayload.valueJson(it.height))) }; is StaticUnionUnionShapeUnion.LABEL -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "label", mapOf<String, Any?>("text" to com.sce.runtime.EventPayload.valueJson(it.text), "note" to ((it.note)?.let { com.sce.runtime.EventPayload.valueJson(it) } ?: com.sce.runtime.EventPayload.valueJson(null)))) }; is StaticUnionUnionShapeUnion.PATH -> it.value.let { com.sce.runtime.EventPayload.unionJson("kind", "path", mapOf<String, Any?>("points" to com.sce.runtime.EventPayload.listJson(it.points) { com.sce.runtime.EventPayload.valueJson(it) })) } } } } ?: com.sce.runtime.EventPayload.valueJson(null)))).json
            )
        )
    }


    override val initialState: StaticUnionState = StaticUnionState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticUnionState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticUnionState, HistoryId>> =
            listOf(StateTarget(StaticUnionState.Idle))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticUnionState, HistoryId>(
            StaticUnionState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticUnionState, HistoryId>(
            StaticUnionState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticUnionState, HistoryId>(
            StaticUnionState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticUnionState? = when (stateId) {
        "idle" -> StaticUnionState.Idle
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticUnionState): String = when (state) {
        is StaticUnionState.Idle -> "idle"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticUnionState): Int = when (state) {
        is StaticUnionState.Idle -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticUnionEvent? = when (name) {
        "error.execution" -> StaticUnionEvent.Error.Execution
        "shape.said" -> StaticUnionEvent.Shape.Said
        "shapes.sent" -> StaticUnionEvent.Shapes.Sent
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticUnionEvent): String? = when (event) {
        is StaticUnionEvent.Error.Execution -> "error.execution"
        is StaticUnionEvent.Shape.Said -> "shape.said"
        is StaticUnionEvent.Shapes.Sent -> "shapes.sent"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticUnionState, event: StaticUnionEvent?): EnabledTransition<StaticUnionState, HistoryId>? = when (state) {
        is StaticUnionState.Idle -> firstEnabledTransitionOfIdle(state, event)
    }


    // idle's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdle(state: StaticUnionState, event: StaticUnionEvent?): EnabledTransition<StaticUnionState, HistoryId>? = when {
        event is StaticUnionEvent.Shapes.Sent -> TransitionsPart0.transitionIdleAt0
        event is StaticUnionEvent.Shape.Said -> TransitionsPart0.transitionIdleAt1
        event is StaticUnionEvent.Error.Execution -> TransitionsPart0.transitionIdleAt2
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_union.scxml:21 :: _machine
    override fun onEntry(state: StaticUnionState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticUnionState.Idle -> onEntryOfIdle()
        }
    }


    private fun onEntryOfIdle() {
                // SCE-MAP: static_union.scxml:63 :: idle :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_union.scxml:21 :: _machine
    override fun onExit(state: StaticUnionState) {
        when (state) {
            is StaticUnionState.Idle -> onExitOfIdle()
        }
    }


    private fun onExitOfIdle() {
                // SCE-MAP: static_union.scxml:63 :: idle :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_union.scxml:21 :: _machine
    override fun executeTransitionContent(source: StaticUnionState, transitionIndex: Int) {
        when (source) {
            is StaticUnionState.Idle -> transitionContentOfIdle(transitionIndex)
        }
    }


    // The content of `idle`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfIdle(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfIdleAt0()
            1 -> transitionContentOfIdleAt1()
            2 -> transitionContentOfIdleAt2()
            else -> {}
        }
    }


    private fun transitionContentOfIdleAt0() {
                // SCE-MAP: static_union.scxml:64 :: idle :: _transition_0
                if (pendingShapesSentPayload == null) {
                    return
                }

            vLastKind = pendingShapesSentPayload!!.one.kind


            if (pendingShapesSentPayload!!.one.kind == StaticUnionKindEnum.DOT) {

            if (try { vDots = com.sce.forge.runtime.SceChecked.add(vDots, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticUnionEvent.Error.Execution, "<assign location='dots'>: an integer operation overflowed or failed"); true }) {
                return
            }
            } else if (pendingShapesSentPayload!!.one.kind == StaticUnionKindEnum.CIRCLE || pendingShapesSentPayload!!.one.kind == StaticUnionKindEnum.DISC) {

            vRadius = pendingShapesSentPayload!!.one.provenFieldRadius()
            } else if (pendingShapesSentPayload!!.one.kind == StaticUnionKindEnum.RECT) {

            vWidth = pendingShapesSentPayload!!.one.provenRect().width

            vHeight = pendingShapesSentPayload!!.one.provenRect().height
            } else if (pendingShapesSentPayload!!.one.kind == StaticUnionKindEnum.PATH) {


            for (point in pendingShapesSentPayload!!.one.provenPath().points) {

            if (try { vPoints = com.sce.forge.runtime.SceChecked.add(vPoints, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticUnionEvent.Error.Execution, "<assign location='points'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            } else {

            if (try { vLabels = com.sce.forge.runtime.SceChecked.add(vLabels, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticUnionEvent.Error.Execution, "<assign location='labels'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            for (shape in pendingShapesSentPayload!!.many) {

            if (try { vManyShapes = com.sce.forge.runtime.SceChecked.add(vManyShapes, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticUnionEvent.Error.Execution, "<assign location='many_shapes'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (shape.kind == StaticUnionKindEnum.RECT) {

            vManyWidth = shape.provenRect().width
            } else if (shape.kind == StaticUnionKindEnum.CIRCLE || shape.kind == StaticUnionKindEnum.DISC) {

            vManyRadius = shape.provenFieldRadius()
            }
            }


            if (pendingShapesSentPayload!!.maybe != null && com.sce.forge.runtime.SceOptional.proven(pendingShapesSentPayload!!.maybe).kind == StaticUnionKindEnum.RECT) {

            vMaybeWidth = com.sce.forge.runtime.SceOptional.proven(pendingShapesSentPayload!!.maybe).provenRect().width
            }


            if (vBuilt.kind == StaticUnionKindEnum.RECT) {

            if (try { vBuilt = StaticUnionUnionShapeUnion.RECT(vBuilt.provenRect().copy(width = com.sce.forge.runtime.SceChecked.add(vBuilt.provenRect().width, 1.toUShort()))); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticUnionEvent.Error.Execution, "<assign location='built.width'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            vKept = pendingShapesSentPayload!!.one
    }

    private fun transitionContentOfIdleAt1() {
                // SCE-MAP: static_union.scxml:98 :: idle :: _transition_1
                if (pendingShapeSaidPayload == null) {
                    return
                }

            when (val sceLift = com.sce.runtime.EventPayload.liftValue(pendingShapeSaidPayload!!.text, 2, 256) { when (it.union("kind", { when (val sceName = it.name()) { "dot" -> StaticUnionKindEnum.DOT; "circle" -> StaticUnionKindEnum.CIRCLE; "disc" -> StaticUnionKindEnum.DISC; "rect" -> StaticUnionKindEnum.RECT; "label" -> StaticUnionKindEnum.LABEL; "path" -> StaticUnionKindEnum.PATH; else -> throw it.unknownVariant(sceName, "Kind") } }) { sceCase, _ -> sceCase }) { StaticUnionKindEnum.DOT -> StaticUnionUnionShapeUnion.DOT; StaticUnionKindEnum.CIRCLE -> StaticUnionUnionShapeUnion.CIRCLE(it.let { it.record(null) { StaticUnionRadiusRecord(radius = it.field("radius").let { it.uint16() }) } }); StaticUnionKindEnum.DISC -> StaticUnionUnionShapeUnion.DISC(it.let { it.record(null) { StaticUnionRadiusRecord(radius = it.field("radius").let { it.uint16() }) } }); StaticUnionKindEnum.RECT -> StaticUnionUnionShapeUnion.RECT(it.let { it.record(null) { StaticUnionRectRecord(width = it.field("width").let { it.uint16() }, height = it.field("height").let { it.uint16() }) } }); StaticUnionKindEnum.LABEL -> StaticUnionUnionShapeUnion.LABEL(it.let { it.record(null) { StaticUnionLabelRecord(text = it.field("text").let { it.string(8) }, note = it.field("note").let { it.optional { it.string(4) } }) } }); StaticUnionKindEnum.PATH -> StaticUnionUnionShapeUnion.PATH(it.let { it.record(null) { StaticUnionPathRecord(points = it.field("points").let { it.list(3) { it.uint8() } }) } }) } }) { is com.sce.runtime.EventPayload.LiftOutcome.Lifted -> { vLifted = sceLift.value; vLiftWhy = null }; is com.sce.runtime.EventPayload.LiftOutcome.Refused -> { vLiftWhy = StaticUnionLiftRefusalRecord(code = when (sceLift.refusal.code) { com.sce.runtime.EventPayload.RefusalCode.NOT_JSON -> StaticUnionRefusalCodeEnum.NOT_JSON; com.sce.runtime.EventPayload.RefusalCode.NOT_OBJECT -> StaticUnionRefusalCodeEnum.NOT_OBJECT; com.sce.runtime.EventPayload.RefusalCode.TOO_DEEP -> StaticUnionRefusalCodeEnum.TOO_DEEP; com.sce.runtime.EventPayload.RefusalCode.MISSING -> StaticUnionRefusalCodeEnum.MISSING; com.sce.runtime.EventPayload.RefusalCode.NULL_NOT_OPTIONAL -> StaticUnionRefusalCodeEnum.NULL_NOT_OPTIONAL; com.sce.runtime.EventPayload.RefusalCode.WRONG_KIND -> StaticUnionRefusalCodeEnum.WRONG_KIND; com.sce.runtime.EventPayload.RefusalCode.BAD_NUMBER -> StaticUnionRefusalCodeEnum.BAD_NUMBER; com.sce.runtime.EventPayload.RefusalCode.OUT_OF_RANGE -> StaticUnionRefusalCodeEnum.OUT_OF_RANGE; com.sce.runtime.EventPayload.RefusalCode.TOO_LONG -> StaticUnionRefusalCodeEnum.TOO_LONG; com.sce.runtime.EventPayload.RefusalCode.TOO_MANY -> StaticUnionRefusalCodeEnum.TOO_MANY; com.sce.runtime.EventPayload.RefusalCode.UNKNOWN_VARIANT -> StaticUnionRefusalCodeEnum.UNKNOWN_VARIANT; com.sce.runtime.EventPayload.RefusalCode.NOT_BYTE_TEXT -> StaticUnionRefusalCodeEnum.NOT_BYTE_TEXT; com.sce.runtime.EventPayload.RefusalCode.NUL_IN_TEXT -> StaticUnionRefusalCodeEnum.NUL_IN_TEXT; com.sce.runtime.EventPayload.RefusalCode.TOO_FEW -> StaticUnionRefusalCodeEnum.TOO_FEW; }, path = sceLift.refusal.pointer, bound = sceLift.refusal.bound?.toUInt()) } }
    }

    private fun transitionContentOfIdleAt2() {
                // SCE-MAP: static_union.scxml:103 :: idle :: _transition_2

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticUnionEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
