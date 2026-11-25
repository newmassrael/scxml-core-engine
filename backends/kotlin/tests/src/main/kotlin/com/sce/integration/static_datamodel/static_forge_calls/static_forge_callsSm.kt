// SCE-GENERATED — DO NOT EDIT
// source-hash: 59eb4e5f00c36054dea015ed0e049cb2693622ae18f04d44b1ed8996241fc85b
// source: static_forge_calls.scxml

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_forge_calls.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_forge_calls.scxml:23 :: _machine

package com.sce.integration.static_datamodel.static_forge_calls

import com.sce.runtime.*
import com.sce.generated.forge_call_code.*
import com.sce.generated.forge_call_high.*
import com.sce.generated.forge_call_level.*
import com.sce.generated.forge_call_limit.*
import com.sce.generated.forge_call_named.*
import com.sce.generated.forge_call_next.*
import com.sce.generated.forge_call_offset.*
import com.sce.generated.forge_call_range.*
import com.sce.generated.forge_call_ratio.*
import com.sce.generated.forge_call_status.*
import com.sce.generated.forge_call_weight.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticForgeCallsState : State {
    data object Run : StaticForgeCallsState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticForgeCallsEvent : Event {
    data object Advance : StaticForgeCallsEvent
    data object CheckCustomer : StaticForgeCallsEvent
    data object ClearCustomer : StaticForgeCallsEvent
    sealed interface Error : StaticForgeCallsEvent {
        data object Execution : Error
    }
    data object Probe : StaticForgeCallsEvent
    data object Rise : StaticForgeCallsEvent
    data object Shift : StaticForgeCallsEvent
    data object Spin : StaticForgeCallsEvent
}
// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:Status` datamodel value. */
enum class StaticForgeCallsStatusEnum(val declaredName: String) {
    OPEN("open"),
    HELD("held"),
    DONE("done");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        OPEN -> 1u.toUByte()
        HELD -> 2u.toUByte()
        DONE -> 3u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticForgeCallsStatusEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of Status")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticForgeCallsStatusEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: an `enum:Level` datamodel value. */
enum class StaticForgeCallsLevelEnum(val declaredName: String) {
    LIGHT("light"),
    HEAVY("heavy");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        LIGHT -> 10u.toUByte()
        HEAVY -> 20u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticForgeCallsLevelEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of Level")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticForgeCallsLevelEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
// ── SCE Accepted Subset §2.15: calls of an algorithm that takes or returns state ──
/** A value of [StaticForgeCallsLevelEnum] as the variant of the same declared name of [ForgeCallLevel]. */
fun StaticForgeCallsLevelEnumToDocument(v: StaticForgeCallsLevelEnum): ForgeCallLevel = when (v) { StaticForgeCallsLevelEnum.LIGHT -> ForgeCallLevel.LIGHT; StaticForgeCallsLevelEnum.HEAVY -> ForgeCallLevel.HEAVY }

/** A value of [ForgeCallLevel] as the variant of the same declared name of [StaticForgeCallsLevelEnum]. */
fun StaticForgeCallsLevelEnumFromDocument(v: ForgeCallLevel): StaticForgeCallsLevelEnum = when (v) { ForgeCallLevel.LIGHT -> StaticForgeCallsLevelEnum.LIGHT; ForgeCallLevel.HEAVY -> StaticForgeCallsLevelEnum.HEAVY }
/** A value of [StaticForgeCallsStatusEnum] as the variant of the same declared name of [ForgeCallStatus]. */
fun StaticForgeCallsStatusEnumToDocument(v: StaticForgeCallsStatusEnum): ForgeCallStatus = when (v) { StaticForgeCallsStatusEnum.OPEN -> ForgeCallStatus.OPEN; StaticForgeCallsStatusEnum.HELD -> ForgeCallStatus.HELD; StaticForgeCallsStatusEnum.DONE -> ForgeCallStatus.DONE }

/** A value of [ForgeCallStatus] as the variant of the same declared name of [StaticForgeCallsStatusEnum]. */
fun StaticForgeCallsStatusEnumFromDocument(v: ForgeCallStatus): StaticForgeCallsStatusEnum = when (v) { ForgeCallStatus.OPEN -> StaticForgeCallsStatusEnum.OPEN; ForgeCallStatus.HELD -> StaticForgeCallsStatusEnum.HELD; ForgeCallStatus.DONE -> StaticForgeCallsStatusEnum.DONE }

// --- State Machine (W3C SCXML) ---

class StaticForgeCallsStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticForgeCallsState, StaticForgeCallsEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `gear` datamodel variable, published (`sce:direction="out"`). */
    private var vGear: UByte = 0.toUByte()

    /** What the published `gear` holds. */
    val gear: UByte get() = vGear
    /** W3C SCXML 5.2: the `ratio` datamodel variable, published (`sce:direction="out"`). */
    private var vRatio: Int = 0

    /** What the published `ratio` holds. */
    val ratio: Int get() = vRatio
    /** W3C SCXML 5.2: the `code` datamodel variable, published (`sce:direction="out"`). */
    private var vCode: Int = 0

    /** What the published `code` holds. */
    val code: Int get() = vCode
    /** W3C SCXML 5.2: the `misses` datamodel variable, published (`sce:direction="out"`). */
    private var vMisses: UInt = 0.toUInt()

    /** What the published `misses` holds. */
    val misses: UInt get() = vMisses
    /** W3C SCXML 5.2: the `level` datamodel variable, published (`sce:direction="out"`). */
    private var vLevel: Int = 0

    /** What the published `level` holds. */
    val level: Int get() = vLevel
    /** W3C SCXML 5.2: the `offset` datamodel variable, published (`sce:direction="out"`). */
    private var vOffset: Int = 0

    /** What the published `offset` holds. */
    val offset: Int get() = vOffset
    /** W3C SCXML 5.2: the `accepted` datamodel variable, published (`sce:direction="out"`). */
    private var vAccepted: Boolean = false

    /** What the published `accepted` holds. */
    val accepted: Boolean get() = vAccepted
    /** W3C SCXML 5.2: the `rpm` datamodel variable, published (`sce:direction="out"`). */
    private var vRpm: UShort = 0.toUShort()

    /** What the published `rpm` holds. */
    val rpm: UShort get() = vRpm
    /** W3C SCXML 5.2: the `limit` datamodel variable, published (`sce:direction="out"`). */
    private var vLimit: Double = 0.0

    /** What the published `limit` holds. */
    val limit: Double get() = vLimit
    /** W3C SCXML 5.2: the `highs` datamodel variable, published (`sce:direction="out"`). */
    private var vHighs: UInt = 0.toUInt()

    /** What the published `highs` holds. */
    val highs: UInt get() = vHighs
    /** W3C SCXML 5.2: the `phase` datamodel variable, published (`sce:direction="out"`). */
    private var vPhase: StaticForgeCallsStatusEnum = StaticForgeCallsStatusEnum.OPEN

    /** What the published `phase` holds. */
    val phase: StaticForgeCallsStatusEnum get() = vPhase
    /** W3C SCXML 5.2: the `weight` datamodel variable, published (`sce:direction="out"`). */
    private var vWeight: StaticForgeCallsLevelEnum = StaticForgeCallsLevelEnum.LIGHT

    /** What the published `weight` holds. */
    val weight: StaticForgeCallsLevelEnum get() = vWeight
    /** W3C SCXML 5.2: the `customer` datamodel variable, published (`sce:direction="out"`). */
    private var vCustomer: String = "anna"

    /** What the published `customer` holds. */
    val customer: String get() = vCustomer
    /** W3C SCXML 5.2: the `known` datamodel variable, published (`sce:direction="out"`). */
    private var vKnown: Boolean = false

    /** What the published `known` holds. */
    val known: Boolean get() = vKnown

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var gear: UByte? = null
        var ratio: Int? = null
        var code: Int? = null
        var misses: UInt? = null
        var level: Int? = null
        var offset: Int? = null
        var accepted: Boolean? = null
        var rpm: UShort? = null
        var limit: Double? = null
        var highs: UInt? = null
        var customer: String? = null
        var known: Boolean? = null
    }

    /**
     * §scxml-6.4: give this machine the values [params] carries, each as the
     * initial value of its variable in place of the one its `<data>` gives, so
     * that a variable declared after it whose initial value reads it reads the
     * handed value (docs/adr/0098). Called before [initialize].
     */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    @Suppress("UNUSED_PARAMETER")
    private fun acceptParamsPart0(params: InvokeParams) {
        vGear = params.gear ?: (0.toUByte())
        vRatio = params.ratio ?: (0)
        vCode = params.code ?: (0)
        vMisses = params.misses ?: (0.toUInt())
        vLevel = params.level ?: (0)
        vOffset = params.offset ?: (0)
        vAccepted = params.accepted ?: (false)
        vRpm = params.rpm ?: (0.toUShort())
        vLimit = params.limit ?: (0.0)
        vHighs = params.highs ?: (0.toUInt())
        vPhase = StaticForgeCallsStatusEnum.OPEN
        vWeight = StaticForgeCallsLevelEnum.LIGHT
        vCustomer = params.customer ?: ("anna")
        vKnown = params.known ?: (false)
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val gear: UByte,
        val ratio: Int,
        val code: Int,
        val misses: UInt,
        val level: Int,
        val offset: Int,
        val accepted: Boolean,
        val rpm: UShort,
        val limit: Double,
        val highs: UInt,
        val phase: StaticForgeCallsStatusEnum,
        val weight: StaticForgeCallsLevelEnum,
        val customer: String,
        val known: Boolean,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticForgeCallsState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        gear = vGear,
        ratio = vRatio,
        code = vCode,
        misses = vMisses,
        level = vLevel,
        offset = vOffset,
        accepted = vAccepted,
        rpm = vRpm,
        limit = vLimit,
        highs = vHighs,
        phase = vPhase,
        weight = vWeight,
        customer = vCustomer,
        known = vKnown,
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
        first = "f8c208dcc14ca056d0b9299f9ccd618ec2750f05632205a4c651d46096c9344a",
        revised = "25698583336d197df47e95c878010ab439aed9ae9aaf753df2da2a80ee38651f",
        format2First = "e1ceb88b42e3ad186cb83379ddd3fac6c041b4cd242dee005c43a6acf78cbcba",
        format2Revised = "47d468eaf1347e8d4209e924eca025b0aee6cf836f6689fc2013b701e7531b53",
        records = listOf(
        ),
        enums = listOf(
            EnumLineage(
                key = "enum:forge_call_level.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("weight", null, false),
                ),
            ),
            EnumLineage(
                key = "enum:forge_call_status.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("phase", null, false),
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
        variables += "gear" to SavedValues.of(vGear)
        variables += "ratio" to SavedValues.of(vRatio)
        variables += "code" to SavedValues.of(vCode)
        variables += "misses" to SavedValues.of(vMisses)
        variables += "level" to SavedValues.of(vLevel)
        variables += "offset" to SavedValues.of(vOffset)
        variables += "accepted" to SavedValues.of(vAccepted)
        variables += "rpm" to SavedValues.of(vRpm)
        variables += "limit" to SavedValues.of(vLimit)
        variables += "highs" to SavedValues.of(vHighs)
        variables += "phase" to vPhase.toSaved()
        variables += "weight" to vWeight.toSaved()
        variables += "customer" to SavedValues.of(vCustomer)
        variables += "known" to SavedValues.of(vKnown)
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
        vGear = SavedValues.uint8(savedGrown.variable("gear"), "gear")
        vRatio = SavedValues.int32(savedGrown.variable("ratio"), "ratio")
        vCode = SavedValues.int32(savedGrown.variable("code"), "code")
        vMisses = SavedValues.uint32(savedGrown.variable("misses"), "misses")
        vLevel = SavedValues.int32(savedGrown.variable("level"), "level")
        vOffset = SavedValues.int32(savedGrown.variable("offset"), "offset")
        vAccepted = SavedValues.bool(savedGrown.variable("accepted"), "accepted")
        vRpm = SavedValues.uint16(savedGrown.variable("rpm"), "rpm")
        vLimit = SavedValues.float64(savedGrown.variable("limit"), "limit")
        vHighs = SavedValues.uint32(savedGrown.variable("highs"), "highs")
        vPhase = StaticForgeCallsStatusEnum.fromSaved(savedGrown.variable("phase"), "phase")
        vWeight = StaticForgeCallsLevelEnum.fromSaved(savedGrown.variable("weight"), "weight")
        vCustomer = SavedValues.string(savedGrown.variable("customer"), "customer", 16)
        vKnown = SavedValues.bool(savedGrown.variable("known"), "known")
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
        SavedValues.uint8(savedGrown.variable("gear"), "gear")
        SavedValues.int32(savedGrown.variable("ratio"), "ratio")
        SavedValues.int32(savedGrown.variable("code"), "code")
        SavedValues.uint32(savedGrown.variable("misses"), "misses")
        SavedValues.int32(savedGrown.variable("level"), "level")
        SavedValues.int32(savedGrown.variable("offset"), "offset")
        SavedValues.bool(savedGrown.variable("accepted"), "accepted")
        SavedValues.uint16(savedGrown.variable("rpm"), "rpm")
        SavedValues.float64(savedGrown.variable("limit"), "limit")
        SavedValues.uint32(savedGrown.variable("highs"), "highs")
        StaticForgeCallsStatusEnum.fromSaved(savedGrown.variable("phase"), "phase")
        StaticForgeCallsLevelEnum.fromSaved(savedGrown.variable("weight"), "weight")
        SavedValues.string(savedGrown.variable("customer"), "customer", 16)
        SavedValues.bool(savedGrown.variable("known"), "known")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    override val initialState: StaticForgeCallsState = StaticForgeCallsState.Run

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
    override val documentInitialTargets: List<EntryTarget<StaticForgeCallsState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticForgeCallsState, HistoryId>> =
            listOf(StateTarget(StaticForgeCallsState.Run))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<StaticForgeCallsState, HistoryId>(
            StaticForgeCallsState.Run,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: run's transition 1, as the microstep reads it.
        val transitionRunAt1 = EnabledTransition<StaticForgeCallsState, HistoryId>(
            StaticForgeCallsState.Run,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: run's transition 2, as the microstep reads it.
        val transitionRunAt2 = EnabledTransition<StaticForgeCallsState, HistoryId>(
            StaticForgeCallsState.Run,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: run's transition 3, as the microstep reads it.
        val transitionRunAt3 = EnabledTransition<StaticForgeCallsState, HistoryId>(
            StaticForgeCallsState.Run,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: run's transition 4, as the microstep reads it.
        val transitionRunAt4 = EnabledTransition<StaticForgeCallsState, HistoryId>(
            StaticForgeCallsState.Run,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: run's transition 5, as the microstep reads it.
        val transitionRunAt5 = EnabledTransition<StaticForgeCallsState, HistoryId>(
            StaticForgeCallsState.Run,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: run's transition 6, as the microstep reads it.
        val transitionRunAt6 = EnabledTransition<StaticForgeCallsState, HistoryId>(
            StaticForgeCallsState.Run,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: run's transition 7, as the microstep reads it.
        val transitionRunAt7 = EnabledTransition<StaticForgeCallsState, HistoryId>(
            StaticForgeCallsState.Run,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticForgeCallsState? = when (stateId) {
        "run" -> StaticForgeCallsState.Run
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticForgeCallsState): String = when (state) {
        is StaticForgeCallsState.Run -> "run"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticForgeCallsState): Int = when (state) {
        is StaticForgeCallsState.Run -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticForgeCallsEvent? = when (name) {
        "advance" -> StaticForgeCallsEvent.Advance
        "check_customer" -> StaticForgeCallsEvent.CheckCustomer
        "clear_customer" -> StaticForgeCallsEvent.ClearCustomer
        "error.execution" -> StaticForgeCallsEvent.Error.Execution
        "probe" -> StaticForgeCallsEvent.Probe
        "rise" -> StaticForgeCallsEvent.Rise
        "shift" -> StaticForgeCallsEvent.Shift
        "spin" -> StaticForgeCallsEvent.Spin
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticForgeCallsEvent): String? = when (event) {
        is StaticForgeCallsEvent.Advance -> "advance"
        is StaticForgeCallsEvent.CheckCustomer -> "check_customer"
        is StaticForgeCallsEvent.ClearCustomer -> "clear_customer"
        is StaticForgeCallsEvent.Error.Execution -> "error.execution"
        is StaticForgeCallsEvent.Probe -> "probe"
        is StaticForgeCallsEvent.Rise -> "rise"
        is StaticForgeCallsEvent.Shift -> "shift"
        is StaticForgeCallsEvent.Spin -> "spin"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticForgeCallsState, event: StaticForgeCallsEvent?): EnabledTransition<StaticForgeCallsState, HistoryId>? = when (state) {
        is StaticForgeCallsState.Run -> firstEnabledTransitionOfRun(state, event)
    }


    // run's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRun(state: StaticForgeCallsState, event: StaticForgeCallsEvent?): EnabledTransition<StaticForgeCallsState, HistoryId>? = when {
        event is StaticForgeCallsEvent.Shift -> TransitionsPart0.transitionRunAt0
        event is StaticForgeCallsEvent.Rise -> TransitionsPart0.transitionRunAt1
        event is StaticForgeCallsEvent.Spin -> TransitionsPart0.transitionRunAt2
        event is StaticForgeCallsEvent.Probe && forgeCallHigh(vLevel) -> TransitionsPart0.transitionRunAt3
        event is StaticForgeCallsEvent.Advance -> TransitionsPart0.transitionRunAt4
        event is StaticForgeCallsEvent.CheckCustomer -> TransitionsPart0.transitionRunAt5
        event is StaticForgeCallsEvent.ClearCustomer -> TransitionsPart0.transitionRunAt6
        event is StaticForgeCallsEvent.Error.Execution -> TransitionsPart0.transitionRunAt7
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_forge_calls.scxml:23 :: _machine
    override fun onEntry(state: StaticForgeCallsState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticForgeCallsState.Run -> onEntryOfRun()
        }
    }


    private fun onEntryOfRun() {
                // SCE-MAP: static_forge_calls.scxml:52 :: run :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_forge_calls.scxml:23 :: _machine
    override fun onExit(state: StaticForgeCallsState) {
        when (state) {
            is StaticForgeCallsState.Run -> onExitOfRun()
        }
    }


    private fun onExitOfRun() {
                // SCE-MAP: static_forge_calls.scxml:52 :: run :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_forge_calls.scxml:23 :: _machine
    override fun executeTransitionContent(source: StaticForgeCallsState, transitionIndex: Int) {
        when (source) {
            is StaticForgeCallsState.Run -> transitionContentOfRun(transitionIndex)
        }
    }


    // The content of `run`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfRun(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfRunAt0()
            1 -> transitionContentOfRunAt1()
            2 -> transitionContentOfRunAt2()
            3 -> transitionContentOfRunAt3()
            4 -> transitionContentOfRunAt4()
            5 -> transitionContentOfRunAt5()
            6 -> transitionContentOfRunAt6()
            7 -> transitionContentOfRunAt7()
            else -> {}
        }
    }


    private fun transitionContentOfRunAt0() {
                // SCE-MAP: static_forge_calls.scxml:53 :: run :: _transition_0

            if (try { vGear = com.sce.forge.runtime.SceChecked.add(vGear, 1.toUByte()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForgeCallsEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<assign location='gear'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vRatio = lookupRatio(vGear)

            if (try { vCode = com.sce.forge.runtime.SceChecked.take(com.sce.forge.runtime.SceChecked.found(lookupCode(vGear))); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForgeCallsEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<assign location='code'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfRunAt1() {
                // SCE-MAP: static_forge_calls.scxml:58 :: run :: _transition_1

            if (try { vLevel = com.sce.forge.runtime.SceChecked.add(vLevel, 7); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForgeCallsEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<assign location='level'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vOffset = computeOffset(vLevel)

            vAccepted = ForgeCallRange().validate(vLevel, vOffset).valid
    }

    private fun transitionContentOfRunAt2() {
                // SCE-MAP: static_forge_calls.scxml:63 :: run :: _transition_2

            if (try { vRpm = com.sce.forge.runtime.SceChecked.add(vRpm, 500.toUShort()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForgeCallsEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<assign location='rpm'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vLimit = ForgeCallLimit.lookup(vRpm)
    }

    private fun transitionContentOfRunAt3() {
                // SCE-MAP: static_forge_calls.scxml:67 :: run :: _transition_3

            if (try { vHighs = com.sce.forge.runtime.SceChecked.add(vHighs, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForgeCallsEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<assign location='highs'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfRunAt4() {
                // SCE-MAP: static_forge_calls.scxml:70 :: run :: _transition_4

            vPhase = StaticForgeCallsStatusEnumFromDocument(computeAfter(StaticForgeCallsStatusEnumToDocument(vPhase)))

            if (try { vWeight = StaticForgeCallsLevelEnumFromDocument(com.sce.forge.runtime.SceChecked.take(com.sce.forge.runtime.SceChecked.found(lookupWeight(StaticForgeCallsStatusEnumToDocument(vPhase))))); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForgeCallsEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<assign location='weight'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfRunAt5() {
                // SCE-MAP: static_forge_calls.scxml:74 :: run :: _transition_5

            vKnown = ForgeCallNamed().validate(vCustomer).valid
    }

    private fun transitionContentOfRunAt6() {
                // SCE-MAP: static_forge_calls.scxml:77 :: run :: _transition_6

            vCustomer = ""
    }

    private fun transitionContentOfRunAt7() {
                // SCE-MAP: static_forge_calls.scxml:80 :: run :: _transition_7

            if (try { vMisses = com.sce.forge.runtime.SceChecked.add(vMisses, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForgeCallsEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<assign location='misses'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
