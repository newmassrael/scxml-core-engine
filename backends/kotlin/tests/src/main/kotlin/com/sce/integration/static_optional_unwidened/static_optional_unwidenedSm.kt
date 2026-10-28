// SCE-GENERATED — DO NOT EDIT
// source-hash: 8b4828d778fc1da05444ac6a052eb44c820b7a80e1f4075fc52bc6e97e8a070a

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_optional_unwidened.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_optional_unwidened.scxml:16 :: _machine

package com.sce.integration.static_optional_unwidened

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticOptionalUnwidenedState : State {
    data object Done : StaticOptionalUnwidenedState
    data object Tracking : StaticOptionalUnwidenedState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticOptionalUnwidenedEvent : Event {
    data object Finish : StaticOptionalUnwidenedEvent
    data object Keep : StaticOptionalUnwidenedEvent
}
// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:Status` datamodel value. */
enum class StaticOptionalUnwidenedStatusEnum(val declaredName: String) {
    OPEN("open"),
    IN_PROGRESS("in_progress"),
    DONE("done");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        OPEN -> 2u.toUByte()
        IN_PROGRESS -> 5u.toUByte()
        DONE -> 7u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticOptionalUnwidenedStatusEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of Status")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticOptionalUnwidenedStatusEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Due` datamodel value. */
data class StaticOptionalUnwidenedDueRecord(val id: UInt, val due: Long, val title: String, val status: StaticOptionalUnwidenedStatusEnum) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "due" to SavedValues.of(due), "title" to SavedValues.of(title), "status" to status.toSaved())

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticOptionalUnwidenedDueRecord = StaticOptionalUnwidenedDueRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), due = SavedValues.int64(SavedValues.field(value, what, "due"), "$what.due"), title = SavedValues.string(SavedValues.field(value, what, "title"), "$what.title", 16), status = StaticOptionalUnwidenedStatusEnum.fromSaved(SavedValues.field(value, what, "status"), "$what.status"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticOptionalUnwidenedStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticOptionalUnwidenedState, StaticOptionalUnwidenedEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `due` datamodel variable, published (`sce:direction="out"`). */
    private var vDue: Long = 12

    /** What the published `due` holds. */
    val due: Long get() = vDue
    /** W3C SCXML 5.2: the `other` datamodel variable, published (`sce:direction="out"`). */
    private var vOther: Long = 7

    /** What the published `other` holds. */
    val other: Long get() = vOther
    /** W3C SCXML 5.2: the `title` datamodel variable, published (`sce:direction="out"`). */
    private var vTitle: String = "later"

    /** What the published `title` holds. */
    val title: String get() = vTitle
    /** W3C SCXML 5.2: the `status` datamodel variable, published (`sce:direction="out"`). */
    private var vStatus: StaticOptionalUnwidenedStatusEnum = StaticOptionalUnwidenedStatusEnum.IN_PROGRESS

    /** What the published `status` holds. */
    val status: StaticOptionalUnwidenedStatusEnum get() = vStatus
    /** W3C SCXML 5.2: the `item` datamodel variable, published (`sce:direction="out"`). */
    private var vItem: StaticOptionalUnwidenedDueRecord = StaticOptionalUnwidenedDueRecord(id = 2.toUInt(), due = 3, title = "first", status = StaticOptionalUnwidenedStatusEnum.OPEN)

    /** What the published `item` holds. */
    val item: StaticOptionalUnwidenedDueRecord get() = vItem
    /** W3C SCXML 5.2: the `items` datamodel variable, published (`sce:direction="out"`). */
    private var vItems: List<StaticOptionalUnwidenedDueRecord> = emptyList()

    /** What the published `items` holds. */
    val items: List<StaticOptionalUnwidenedDueRecord> get() = vItems
    /** W3C SCXML 5.2: the `days` datamodel variable, published (`sce:direction="out"`). */
    private var vDays: Long = 0

    /** What the published `days` holds. */
    val days: Long get() = vDays
    /** W3C SCXML 5.2: the `following` datamodel variable, published (`sce:direction="out"`). */
    private var vFollowing: Long = 0

    /** What the published `following` holds. */
    val following: Long get() = vFollowing
    /** W3C SCXML 5.2: the `late` datamodel variable, published (`sce:direction="out"`). */
    private var vLate: Boolean = false

    /** What the published `late` holds. */
    val late: Boolean get() = vLate
    /** W3C SCXML 5.2: the `early` datamodel variable, published (`sce:direction="out"`). */
    private var vEarly: Boolean = false

    /** What the published `early` holds. */
    val early: Boolean get() = vEarly
    /** W3C SCXML 5.2: the `label` datamodel variable, published (`sce:direction="out"`). */
    private var vLabel: String = ""

    /** What the published `label` holds. */
    val label: String get() = vLabel
    /** W3C SCXML 5.2: the `plain` datamodel variable, published (`sce:direction="out"`). */
    private var vPlain: StaticOptionalUnwidenedStatusEnum = StaticOptionalUnwidenedStatusEnum.OPEN

    /** What the published `plain` holds. */
    val plain: StaticOptionalUnwidenedStatusEnum get() = vPlain
    /** W3C SCXML 5.2: the `kept` datamodel variable, published (`sce:direction="out"`). */
    private var vKept: Long = 0

    /** What the published `kept` holds. */
    val kept: Long get() = vKept
    /** W3C SCXML 5.2: the `listed` datamodel variable, published (`sce:direction="out"`). */
    private var vListed: Long = 0

    /** What the published `listed` holds. */
    val listed: Long get() = vListed
    /** W3C SCXML 5.2: the `missing` datamodel variable, published (`sce:direction="out"`). */
    private var vMissing: UInt = 0.toUInt()

    /** What the published `missing` holds. */
    val missing: UInt get() = vMissing
    /** W3C SCXML 5.2: the `same` datamodel variable, published (`sce:direction="out"`). */
    private var vSame: Boolean = false

    /** What the published `same` holds. */
    val same: Boolean get() = vSame
    /** W3C SCXML 5.2: the `absent` datamodel variable, published (`sce:direction="out"`). */
    private var vAbsent: Boolean = false

    /** What the published `absent` holds. */
    val absent: Boolean get() = vAbsent
    /** W3C SCXML 5.2: the `statusDone` datamodel variable, published (`sce:direction="out"`). */
    private var vStatusDone: Boolean = false

    /** What the published `statusDone` holds. */
    val statusDone: Boolean get() = vStatusDone
    /** W3C SCXML 5.2: the `echoed` datamodel variable, published (`sce:direction="out"`). */
    private var vEchoed: Long = 0

    /** What the published `echoed` holds. */
    val echoed: Long get() = vEchoed
    /** W3C SCXML 5.2: the `echoTitle` datamodel variable, published (`sce:direction="out"`). */
    private var vEchoTitle: String = "echo"

    /** What the published `echoTitle` holds. */
    val echoTitle: String get() = vEchoTitle
    /** W3C SCXML 5.2: the `echoStatus` datamodel variable, published (`sce:direction="out"`). */
    private var vEchoStatus: StaticOptionalUnwidenedStatusEnum = StaticOptionalUnwidenedStatusEnum.DONE

    /** What the published `echoStatus` holds. */
    val echoStatus: StaticOptionalUnwidenedStatusEnum get() = vEchoStatus
    /** W3C SCXML 5.2: the `fallbacks` datamodel variable, published (`sce:direction="out"`). */
    private var vFallbacks: UInt = 0.toUInt()

    /** What the published `fallbacks` holds. */
    val fallbacks: UInt get() = vFallbacks
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    private var vErrors: UInt = 0.toUInt()

    /** What the published `errors` holds. */
    val errors: UInt get() = vErrors
    /** W3C SCXML 5.2: the `huge` datamodel variable, the machine's own. */
    private var vHuge: Long = 4294967296

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var due: Long? = null
        var other: Long? = null
        var title: String? = null
        var days: Long? = null
        var following: Long? = null
        var late: Boolean? = null
        var early: Boolean? = null
        var label: String? = null
        var kept: Long? = null
        var listed: Long? = null
        var missing: UInt? = null
        var same: Boolean? = null
        var absent: Boolean? = null
        var statusDone: Boolean? = null
        var echoed: Long? = null
        var echoTitle: String? = null
        var fallbacks: UInt? = null
        var errors: UInt? = null
        var huge: Long? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.due?.let { vDue = it }
        params.other?.let { vOther = it }
        params.title?.let { vTitle = it }
        params.days?.let { vDays = it }
        params.following?.let { vFollowing = it }
        params.late?.let { vLate = it }
        params.early?.let { vEarly = it }
        params.label?.let { vLabel = it }
        params.kept?.let { vKept = it }
        params.listed?.let { vListed = it }
        params.missing?.let { vMissing = it }
        params.same?.let { vSame = it }
        params.absent?.let { vAbsent = it }
        params.statusDone?.let { vStatusDone = it }
        params.echoed?.let { vEchoed = it }
        params.echoTitle?.let { vEchoTitle = it }
        params.fallbacks?.let { vFallbacks = it }
        params.errors?.let { vErrors = it }
        params.huge?.let { vHuge = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val due: Long,
        val other: Long,
        val title: String,
        val status: StaticOptionalUnwidenedStatusEnum,
        val item: StaticOptionalUnwidenedDueRecord,
        val items: List<StaticOptionalUnwidenedDueRecord>,
        val days: Long,
        val following: Long,
        val late: Boolean,
        val early: Boolean,
        val label: String,
        val plain: StaticOptionalUnwidenedStatusEnum,
        val kept: Long,
        val listed: Long,
        val missing: UInt,
        val same: Boolean,
        val absent: Boolean,
        val statusDone: Boolean,
        val echoed: Long,
        val echoTitle: String,
        val echoStatus: StaticOptionalUnwidenedStatusEnum,
        val fallbacks: UInt,
        val errors: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticOptionalUnwidenedState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        due = vDue,
        other = vOther,
        title = vTitle,
        status = vStatus,
        item = vItem,
        items = vItems,
        days = vDays,
        following = vFollowing,
        late = vLate,
        early = vEarly,
        label = vLabel,
        plain = vPlain,
        kept = vKept,
        listed = vListed,
        missing = vMissing,
        same = vSame,
        absent = vAbsent,
        statusDone = vStatusDone,
        echoed = vEchoed,
        echoTitle = vEchoTitle,
        echoStatus = vEchoStatus,
        fallbacks = vFallbacks,
        errors = vErrors,
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
        first = "107ff83fbd2c38a87a0f71adfa81fcc3c27bf980240687ee0f9999ecfbc460e2",
        revised = "446e9f68a2de213abce540de0df591de51badabf09d45a8b571910ae823c15fc",
        format2First = "97c67c3845141fbd65d751eef23b71ca25329452523e524d81ed9621ca5b8d74",
        format2Revised = "51b560ebab5c70ad2a607bce4b26e572a1110fc5f8cd00d8a79692205886638b",
        records = listOf(
            RecordLineage(
                event = "due.unwidened",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    RecordHolder("item", false),
                    RecordHolder("items", true),
                ),
            ),
        ),
        enums = listOf(
            EnumLineage(
                key = "enum:enum_item_status.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("status", null, false),
                    EnumHolder("item", "status", false),
                    EnumHolder("items", "status", true),
                    EnumHolder("plain", null, false),
                    EnumHolder("echoStatus", null, false),
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
    fun save(wallNowMs: Long): SavedState = savedState(
        savedShape,
        linkedMapOf(
            "due" to SavedValues.of(vDue),
            "other" to SavedValues.of(vOther),
            "title" to SavedValues.of(vTitle),
            "status" to vStatus.toSaved(),
            "item" to vItem.toSaved(),
            "items" to SavedValues.list(vItems) { it.toSaved() },
            "days" to SavedValues.of(vDays),
            "following" to SavedValues.of(vFollowing),
            "late" to SavedValues.of(vLate),
            "early" to SavedValues.of(vEarly),
            "label" to SavedValues.of(vLabel),
            "plain" to vPlain.toSaved(),
            "kept" to SavedValues.of(vKept),
            "listed" to SavedValues.of(vListed),
            "missing" to SavedValues.of(vMissing),
            "same" to SavedValues.of(vSame),
            "absent" to SavedValues.of(vAbsent),
            "statusDone" to SavedValues.of(vStatusDone),
            "echoed" to SavedValues.of(vEchoed),
            "echoTitle" to SavedValues.of(vEchoTitle),
            "echoStatus" to vEchoStatus.toSaved(),
            "fallbacks" to SavedValues.of(vFallbacks),
            "errors" to SavedValues.of(vErrors),
            "huge" to SavedValues.of(vHuge),
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
        val saved1 = SavedValues.int64(savedGrown.variable("due"), "due")
        val saved2 = SavedValues.int64(savedGrown.variable("other"), "other")
        val saved3 = SavedValues.string(savedGrown.variable("title"), "title", 16)
        val saved4 = StaticOptionalUnwidenedStatusEnum.fromSaved(savedGrown.variable("status"), "status")
        val saved5 = StaticOptionalUnwidenedDueRecord.fromSaved(savedGrown.variable("item"), "item")
        val saved6 = SavedValues.list(savedGrown.variable("items"), "items", 3) { e, w -> StaticOptionalUnwidenedDueRecord.fromSaved(e, w) }
        val saved7 = SavedValues.int64(savedGrown.variable("days"), "days")
        val saved8 = SavedValues.int64(savedGrown.variable("following"), "following")
        val saved9 = SavedValues.bool(savedGrown.variable("late"), "late")
        val saved10 = SavedValues.bool(savedGrown.variable("early"), "early")
        val saved11 = SavedValues.string(savedGrown.variable("label"), "label", 16)
        val saved12 = StaticOptionalUnwidenedStatusEnum.fromSaved(savedGrown.variable("plain"), "plain")
        val saved13 = SavedValues.int64(savedGrown.variable("kept"), "kept")
        val saved14 = SavedValues.int64(savedGrown.variable("listed"), "listed")
        val saved15 = SavedValues.uint32(savedGrown.variable("missing"), "missing")
        val saved16 = SavedValues.bool(savedGrown.variable("same"), "same")
        val saved17 = SavedValues.bool(savedGrown.variable("absent"), "absent")
        val saved18 = SavedValues.bool(savedGrown.variable("statusDone"), "statusDone")
        val saved19 = SavedValues.int64(savedGrown.variable("echoed"), "echoed")
        val saved20 = SavedValues.string(savedGrown.variable("echoTitle"), "echoTitle", 16)
        val saved21 = StaticOptionalUnwidenedStatusEnum.fromSaved(savedGrown.variable("echoStatus"), "echoStatus")
        val saved22 = SavedValues.uint32(savedGrown.variable("fallbacks"), "fallbacks")
        val saved23 = SavedValues.uint32(savedGrown.variable("errors"), "errors")
        val saved24 = SavedValues.int64(savedGrown.variable("huge"), "huge")
        vDue = saved1
        vOther = saved2
        vTitle = saved3
        vStatus = saved4
        vItem = saved5
        vItems = saved6
        vDays = saved7
        vFollowing = saved8
        vLate = saved9
        vEarly = saved10
        vLabel = saved11
        vPlain = saved12
        vKept = saved13
        vListed = saved14
        vMissing = saved15
        vSame = saved16
        vAbsent = saved17
        vStatusDone = saved18
        vEchoed = saved19
        vEchoTitle = saved20
        vEchoStatus = saved21
        vFallbacks = saved22
        vErrors = saved23
        vHuge = saved24
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
        SavedValues.int64(savedGrown.variable("due"), "due")
        SavedValues.int64(savedGrown.variable("other"), "other")
        SavedValues.string(savedGrown.variable("title"), "title", 16)
        StaticOptionalUnwidenedStatusEnum.fromSaved(savedGrown.variable("status"), "status")
        StaticOptionalUnwidenedDueRecord.fromSaved(savedGrown.variable("item"), "item")
        SavedValues.list(savedGrown.variable("items"), "items", 3) { e, w -> StaticOptionalUnwidenedDueRecord.fromSaved(e, w) }
        SavedValues.int64(savedGrown.variable("days"), "days")
        SavedValues.int64(savedGrown.variable("following"), "following")
        SavedValues.bool(savedGrown.variable("late"), "late")
        SavedValues.bool(savedGrown.variable("early"), "early")
        SavedValues.string(savedGrown.variable("label"), "label", 16)
        StaticOptionalUnwidenedStatusEnum.fromSaved(savedGrown.variable("plain"), "plain")
        SavedValues.int64(savedGrown.variable("kept"), "kept")
        SavedValues.int64(savedGrown.variable("listed"), "listed")
        SavedValues.uint32(savedGrown.variable("missing"), "missing")
        SavedValues.bool(savedGrown.variable("same"), "same")
        SavedValues.bool(savedGrown.variable("absent"), "absent")
        SavedValues.bool(savedGrown.variable("statusDone"), "statusDone")
        SavedValues.int64(savedGrown.variable("echoed"), "echoed")
        SavedValues.string(savedGrown.variable("echoTitle"), "echoTitle", 16)
        StaticOptionalUnwidenedStatusEnum.fromSaved(savedGrown.variable("echoStatus"), "echoStatus")
        SavedValues.uint32(savedGrown.variable("fallbacks"), "fallbacks")
        SavedValues.uint32(savedGrown.variable("errors"), "errors")
        SavedValues.int64(savedGrown.variable("huge"), "huge")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    override val initialState: StaticOptionalUnwidenedState = StaticOptionalUnwidenedState.Tracking

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

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: StaticOptionalUnwidenedState): Boolean = when (state) {
        is StaticOptionalUnwidenedState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticOptionalUnwidenedState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticOptionalUnwidenedState, HistoryId>> =
            listOf(StateTarget(StaticOptionalUnwidenedState.Tracking))

        // W3C SCXML 3.13: tracking's transition 0, as the microstep reads it.
        val transitionTrackingAt0 = EnabledTransition<StaticOptionalUnwidenedState, HistoryId>(
            StaticOptionalUnwidenedState.Tracking,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: tracking's transition 1, as the microstep reads it.
        val transitionTrackingAt1 = EnabledTransition<StaticOptionalUnwidenedState, HistoryId>(
            StaticOptionalUnwidenedState.Tracking,
            listOf(StateTarget(StaticOptionalUnwidenedState.Done)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticOptionalUnwidenedState? = when (stateId) {
        "done" -> StaticOptionalUnwidenedState.Done
        "tracking" -> StaticOptionalUnwidenedState.Tracking
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticOptionalUnwidenedState): String = when (state) {
        is StaticOptionalUnwidenedState.Done -> "done"
        is StaticOptionalUnwidenedState.Tracking -> "tracking"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticOptionalUnwidenedState): Int = when (state) {
        is StaticOptionalUnwidenedState.Done -> 1
        is StaticOptionalUnwidenedState.Tracking -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticOptionalUnwidenedEvent? = when (name) {
        "finish" -> StaticOptionalUnwidenedEvent.Finish
        "keep" -> StaticOptionalUnwidenedEvent.Keep
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticOptionalUnwidenedEvent): String? = when (event) {
        is StaticOptionalUnwidenedEvent.Finish -> "finish"
        is StaticOptionalUnwidenedEvent.Keep -> "keep"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticOptionalUnwidenedState,
        event: StaticOptionalUnwidenedEvent?
    ): EnabledTransition<StaticOptionalUnwidenedState, HistoryId>? = when (state) {
        is StaticOptionalUnwidenedState.Tracking -> when {
            event is StaticOptionalUnwidenedEvent.Keep -> transitionTrackingAt0
            event is StaticOptionalUnwidenedEvent.Finish -> transitionTrackingAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_optional_unwidened.scxml:16 :: _machine
    override fun onEntry(state: StaticOptionalUnwidenedState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticOptionalUnwidenedState.Done -> {
                // SCE-MAP: static_optional_unwidened.scxml:57 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StaticOptionalUnwidenedState.Tracking -> {
                // SCE-MAP: static_optional_unwidened.scxml:51 :: tracking :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_optional_unwidened.scxml:16 :: _machine
    override fun onExit(state: StaticOptionalUnwidenedState) {
        when (state) {
            is StaticOptionalUnwidenedState.Done -> {
                // SCE-MAP: static_optional_unwidened.scxml:57 :: done :: _state_body
            }
            is StaticOptionalUnwidenedState.Tracking -> {
                // SCE-MAP: static_optional_unwidened.scxml:51 :: tracking :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_optional_unwidened.scxml:16 :: _machine
    override fun executeTransitionContent(source: StaticOptionalUnwidenedState, transitionIndex: Int) {
        when (source) {
        is StaticOptionalUnwidenedState.Tracking -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_optional_unwidened.scxml:52 :: tracking :: _transition_0

            if (if (vItems.size < 3) { vItems = vItems + (vItem); false } else { true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
