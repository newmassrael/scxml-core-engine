// SCE-GENERATED — DO NOT EDIT
// source-hash: 8b4828d778fc1da05444ac6a052eb44c820b7a80e1f4075fc52bc6e97e8a070a

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_optional.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_optional.scxml:28 :: _machine

package com.sce.integration.static_optional

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticOptionalState : State {
    data object Done : StaticOptionalState
    data object Tracking : StaticOptionalState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticOptionalEvent : Event {
    sealed interface Due : StaticOptionalEvent {
        data object Clear : Due
        data object Fallback : Due
        data object Read : Due
        data object Sent : Due
        data object Set : Due
    }
    sealed interface Error : StaticOptionalEvent {
        data object Execution : Error
    }
    data object Finish : StaticOptionalEvent
    data object Relay : StaticOptionalEvent
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticOptionalPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticOptionalDueSentPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `due.sent`. Consumers inject it via the `raiseDueSent` seam
// on the machine — they never name this class directly.
data class StaticOptionalDueSentPayload(val due: Long?, val title: String?, val status: StaticOptionalStatusEnum?)

// StaticOptionalDueSetPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `due.set`. Consumers inject it via the `raiseDueSet` seam
// on the machine — they never name this class directly.
data class StaticOptionalDueSetPayload(val id: UInt, val due: Long?, val title: String?, val status: StaticOptionalStatusEnum?)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:Status` datamodel value. */
enum class StaticOptionalStatusEnum(val declaredName: String) {
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
        fun fromSaved(value: Any?, what: String): StaticOptionalStatusEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of Status")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticOptionalStatusEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Due` datamodel value. */
data class StaticOptionalDueRecord(val id: UInt, val due: Long?, val title: String?, val status: StaticOptionalStatusEnum?) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "due" to SavedValues.optional(due) { SavedValues.of(it) }, "title" to SavedValues.optional(title) { SavedValues.of(it) }, "status" to SavedValues.optional(status) { it.toSaved() })

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticOptionalDueRecord = StaticOptionalDueRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), due = SavedValues.optional(SavedValues.field(value, what, "due"), "$what.due") { e, w -> SavedValues.int64(e, w) }, title = SavedValues.optional(SavedValues.field(value, what, "title"), "$what.title") { e, w -> SavedValues.string(e, w, 16) }, status = SavedValues.optional(SavedValues.field(value, what, "status"), "$what.status") { e, w -> StaticOptionalStatusEnum.fromSaved(e, w) })
    }
}
// --- State Machine (W3C SCXML) ---

class StaticOptionalStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticOptionalState, StaticOptionalEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `due` datamodel variable, published (`sce:direction="out"`). */
    private var vDue: Long? = null

    /** What the published `due` holds. */
    val due: Long? get() = vDue
    /** W3C SCXML 5.2: the `other` datamodel variable, published (`sce:direction="out"`). */
    private var vOther: Long? = 7

    /** What the published `other` holds. */
    val other: Long? get() = vOther
    /** W3C SCXML 5.2: the `title` datamodel variable, published (`sce:direction="out"`). */
    private var vTitle: String? = null

    /** What the published `title` holds. */
    val title: String? get() = vTitle
    /** W3C SCXML 5.2: the `status` datamodel variable, published (`sce:direction="out"`). */
    private var vStatus: StaticOptionalStatusEnum? = StaticOptionalStatusEnum.OPEN

    /** What the published `status` holds. */
    val status: StaticOptionalStatusEnum? get() = vStatus
    /** W3C SCXML 5.2: the `item` datamodel variable, published (`sce:direction="out"`). */
    private var vItem: StaticOptionalDueRecord = StaticOptionalDueRecord(id = 0.toUInt(), due = null, title = "first", status = null)

    /** What the published `item` holds. */
    val item: StaticOptionalDueRecord get() = vItem
    /** W3C SCXML 5.2: the `items` datamodel variable, published (`sce:direction="out"`). */
    private var vItems: List<StaticOptionalDueRecord> = emptyList()

    /** What the published `items` holds. */
    val items: List<StaticOptionalDueRecord> get() = vItems
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
    private var vPlain: StaticOptionalStatusEnum = StaticOptionalStatusEnum.OPEN

    /** What the published `plain` holds. */
    val plain: StaticOptionalStatusEnum get() = vPlain
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
    private var vEchoTitle: String? = null

    /** What the published `echoTitle` holds. */
    val echoTitle: String? get() = vEchoTitle
    /** W3C SCXML 5.2: the `echoStatus` datamodel variable, published (`sce:direction="out"`). */
    private var vEchoStatus: StaticOptionalStatusEnum? = null

    /** What the published `echoStatus` holds. */
    val echoStatus: StaticOptionalStatusEnum? get() = vEchoStatus
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
        var fallbacks: UInt? = null
        var errors: UInt? = null
        var huge: Long? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
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
        params.fallbacks?.let { vFallbacks = it }
        params.errors?.let { vErrors = it }
        params.huge?.let { vHuge = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val due: Long?,
        val other: Long?,
        val title: String?,
        val status: StaticOptionalStatusEnum?,
        val item: StaticOptionalDueRecord,
        val items: List<StaticOptionalDueRecord>,
        val days: Long,
        val following: Long,
        val late: Boolean,
        val early: Boolean,
        val label: String,
        val plain: StaticOptionalStatusEnum,
        val kept: Long,
        val listed: Long,
        val missing: UInt,
        val same: Boolean,
        val absent: Boolean,
        val statusDone: Boolean,
        val echoed: Long,
        val echoTitle: String?,
        val echoStatus: StaticOptionalStatusEnum?,
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
        val configuration: Set<StaticOptionalState>,
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
                event = "due.set",
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
            "due" to SavedValues.optional(vDue) { SavedValues.of(it) },
            "other" to SavedValues.optional(vOther) { SavedValues.of(it) },
            "title" to SavedValues.optional(vTitle) { SavedValues.of(it) },
            "status" to SavedValues.optional(vStatus) { it.toSaved() },
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
            "echoTitle" to SavedValues.optional(vEchoTitle) { SavedValues.of(it) },
            "echoStatus" to SavedValues.optional(vEchoStatus) { it.toSaved() },
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
        val saved1 = SavedValues.optional(savedGrown.variable("due"), "due") { e, w -> SavedValues.int64(e, w) }
        val saved2 = SavedValues.optional(savedGrown.variable("other"), "other") { e, w -> SavedValues.int64(e, w) }
        val saved3 = SavedValues.optional(savedGrown.variable("title"), "title") { e, w -> SavedValues.string(e, w, 16) }
        val saved4 = SavedValues.optional(savedGrown.variable("status"), "status") { e, w -> StaticOptionalStatusEnum.fromSaved(e, w) }
        val saved5 = StaticOptionalDueRecord.fromSaved(savedGrown.variable("item"), "item")
        val saved6 = SavedValues.list(savedGrown.variable("items"), "items", 3) { e, w -> StaticOptionalDueRecord.fromSaved(e, w) }
        val saved7 = SavedValues.int64(savedGrown.variable("days"), "days")
        val saved8 = SavedValues.int64(savedGrown.variable("following"), "following")
        val saved9 = SavedValues.bool(savedGrown.variable("late"), "late")
        val saved10 = SavedValues.bool(savedGrown.variable("early"), "early")
        val saved11 = SavedValues.string(savedGrown.variable("label"), "label", 16)
        val saved12 = StaticOptionalStatusEnum.fromSaved(savedGrown.variable("plain"), "plain")
        val saved13 = SavedValues.int64(savedGrown.variable("kept"), "kept")
        val saved14 = SavedValues.int64(savedGrown.variable("listed"), "listed")
        val saved15 = SavedValues.uint32(savedGrown.variable("missing"), "missing")
        val saved16 = SavedValues.bool(savedGrown.variable("same"), "same")
        val saved17 = SavedValues.bool(savedGrown.variable("absent"), "absent")
        val saved18 = SavedValues.bool(savedGrown.variable("statusDone"), "statusDone")
        val saved19 = SavedValues.int64(savedGrown.variable("echoed"), "echoed")
        val saved20 = SavedValues.optional(savedGrown.variable("echoTitle"), "echoTitle") { e, w -> SavedValues.string(e, w, 16) }
        val saved21 = SavedValues.optional(savedGrown.variable("echoStatus"), "echoStatus") { e, w -> StaticOptionalStatusEnum.fromSaved(e, w) }
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
        SavedValues.optional(savedGrown.variable("due"), "due") { e, w -> SavedValues.int64(e, w) }
        SavedValues.optional(savedGrown.variable("other"), "other") { e, w -> SavedValues.int64(e, w) }
        SavedValues.optional(savedGrown.variable("title"), "title") { e, w -> SavedValues.string(e, w, 16) }
        SavedValues.optional(savedGrown.variable("status"), "status") { e, w -> StaticOptionalStatusEnum.fromSaved(e, w) }
        StaticOptionalDueRecord.fromSaved(savedGrown.variable("item"), "item")
        SavedValues.list(savedGrown.variable("items"), "items", 3) { e, w -> StaticOptionalDueRecord.fromSaved(e, w) }
        SavedValues.int64(savedGrown.variable("days"), "days")
        SavedValues.int64(savedGrown.variable("following"), "following")
        SavedValues.bool(savedGrown.variable("late"), "late")
        SavedValues.bool(savedGrown.variable("early"), "early")
        SavedValues.string(savedGrown.variable("label"), "label", 16)
        StaticOptionalStatusEnum.fromSaved(savedGrown.variable("plain"), "plain")
        SavedValues.int64(savedGrown.variable("kept"), "kept")
        SavedValues.int64(savedGrown.variable("listed"), "listed")
        SavedValues.uint32(savedGrown.variable("missing"), "missing")
        SavedValues.bool(savedGrown.variable("same"), "same")
        SavedValues.bool(savedGrown.variable("absent"), "absent")
        SavedValues.bool(savedGrown.variable("statusDone"), "statusDone")
        SavedValues.int64(savedGrown.variable("echoed"), "echoed")
        SavedValues.optional(savedGrown.variable("echoTitle"), "echoTitle") { e, w -> SavedValues.string(e, w, 16) }
        SavedValues.optional(savedGrown.variable("echoStatus"), "echoStatus") { e, w -> StaticOptionalStatusEnum.fromSaved(e, w) }
        SavedValues.uint32(savedGrown.variable("fallbacks"), "fallbacks")
        SavedValues.uint32(savedGrown.variable("errors"), "errors")
        SavedValues.int64(savedGrown.variable("huge"), "huge")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDueSentPayload: StaticOptionalDueSentPayload? = null
    private var pendingDueSetPayload: StaticOptionalDueSetPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticOptionalEvent, metadata: EventMetadata) {
        pendingDueSentPayload = null
        pendingDueSetPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticOptionalDueSentPayload -> pendingDueSentPayload = tp
            is StaticOptionalDueSetPayload -> pendingDueSetPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticOptionalEvent.Due.Sent) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDueSentPayload = StaticOptionalDueSentPayload(fields.optional("due") { _ -> fields.int64("due") }, fields.optional("title") { _ -> fields.string("title") }, fields.optional("status") { _ -> fields.string("status").let { name -> StaticOptionalStatusEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'status' ($name) is not a variant of Status") } })
                } else if (event == StaticOptionalEvent.Due.Set) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDueSetPayload = StaticOptionalDueSetPayload(fields.uint32("id"), fields.optional("due") { _ -> fields.int64("due") }, fields.optional("title") { _ -> fields.string("title") }, fields.optional("status") { _ -> fields.string("status").let { name -> StaticOptionalStatusEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'status' ($name) is not a variant of Status") } })
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `due.sent` — binds the event name and the payload field values in one call.
    fun raiseDueSent(due: Long?, title: String?, status: StaticOptionalStatusEnum?) {
        send(
            StaticOptionalEvent.Due.Sent,
            EventMetadata(
                type = "external",
                typedPayload = StaticOptionalDueSentPayload(due, title, status),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("due" to due, "title" to title, "status" to status?.declaredName))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `due.set` — binds the event name and the payload field values in one call.
    fun raiseDueSet(id: UInt, due: Long?, title: String?, status: StaticOptionalStatusEnum?) {
        send(
            StaticOptionalEvent.Due.Set,
            EventMetadata(
                type = "external",
                typedPayload = StaticOptionalDueSetPayload(id, due, title, status),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "due" to due, "title" to title, "status" to status?.declaredName))
            )
        )
    }


    override val initialState: StaticOptionalState = StaticOptionalState.Tracking

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
    override fun isFinalState(state: StaticOptionalState): Boolean = when (state) {
        is StaticOptionalState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticOptionalState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticOptionalState, HistoryId>> =
            listOf(StateTarget(StaticOptionalState.Tracking))

        // W3C SCXML 3.13: tracking's transition 0, as the microstep reads it.
        val transitionTrackingAt0 = EnabledTransition<StaticOptionalState, HistoryId>(
            StaticOptionalState.Tracking,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: tracking's transition 1, as the microstep reads it.
        val transitionTrackingAt1 = EnabledTransition<StaticOptionalState, HistoryId>(
            StaticOptionalState.Tracking,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: tracking's transition 2, as the microstep reads it.
        val transitionTrackingAt2 = EnabledTransition<StaticOptionalState, HistoryId>(
            StaticOptionalState.Tracking,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: tracking's transition 3, as the microstep reads it.
        val transitionTrackingAt3 = EnabledTransition<StaticOptionalState, HistoryId>(
            StaticOptionalState.Tracking,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: tracking's transition 4, as the microstep reads it.
        val transitionTrackingAt4 = EnabledTransition<StaticOptionalState, HistoryId>(
            StaticOptionalState.Tracking,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: tracking's transition 5, as the microstep reads it.
        val transitionTrackingAt5 = EnabledTransition<StaticOptionalState, HistoryId>(
            StaticOptionalState.Tracking,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: tracking's transition 6, as the microstep reads it.
        val transitionTrackingAt6 = EnabledTransition<StaticOptionalState, HistoryId>(
            StaticOptionalState.Tracking,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: tracking's transition 7, as the microstep reads it.
        val transitionTrackingAt7 = EnabledTransition<StaticOptionalState, HistoryId>(
            StaticOptionalState.Tracking,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: tracking's transition 8, as the microstep reads it.
        val transitionTrackingAt8 = EnabledTransition<StaticOptionalState, HistoryId>(
            StaticOptionalState.Tracking,
            listOf(StateTarget(StaticOptionalState.Done)),
            8,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticOptionalState? = when (stateId) {
        "done" -> StaticOptionalState.Done
        "tracking" -> StaticOptionalState.Tracking
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticOptionalState): String = when (state) {
        is StaticOptionalState.Done -> "done"
        is StaticOptionalState.Tracking -> "tracking"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticOptionalState): Int = when (state) {
        is StaticOptionalState.Done -> 1
        is StaticOptionalState.Tracking -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticOptionalEvent? = when (name) {
        "due.clear" -> StaticOptionalEvent.Due.Clear
        "due.fallback" -> StaticOptionalEvent.Due.Fallback
        "due.read" -> StaticOptionalEvent.Due.Read
        "due.sent" -> StaticOptionalEvent.Due.Sent
        "due.set" -> StaticOptionalEvent.Due.Set
        "error.execution" -> StaticOptionalEvent.Error.Execution
        "finish" -> StaticOptionalEvent.Finish
        "relay" -> StaticOptionalEvent.Relay
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticOptionalEvent): String? = when (event) {
        is StaticOptionalEvent.Due.Clear -> "due.clear"
        is StaticOptionalEvent.Due.Fallback -> "due.fallback"
        is StaticOptionalEvent.Due.Read -> "due.read"
        is StaticOptionalEvent.Due.Sent -> "due.sent"
        is StaticOptionalEvent.Due.Set -> "due.set"
        is StaticOptionalEvent.Error.Execution -> "error.execution"
        is StaticOptionalEvent.Finish -> "finish"
        is StaticOptionalEvent.Relay -> "relay"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticOptionalState,
        event: StaticOptionalEvent?
    ): EnabledTransition<StaticOptionalState, HistoryId>? = when (state) {
        is StaticOptionalState.Tracking -> when {
            event is StaticOptionalEvent.Due.Set -> transitionTrackingAt0
            event is StaticOptionalEvent.Due.Clear -> transitionTrackingAt1
            event is StaticOptionalEvent.Due.Read -> transitionTrackingAt2
            event is StaticOptionalEvent.Due.Fallback -> transitionTrackingAt3
            event is StaticOptionalEvent.Relay -> transitionTrackingAt4
            event is StaticOptionalEvent.Due.Sent && pendingDueSentPayload != null && (pendingDueSentPayload!!.due != null) -> transitionTrackingAt5
            event is StaticOptionalEvent.Due.Sent -> transitionTrackingAt6
            event is StaticOptionalEvent.Error.Execution -> transitionTrackingAt7
            event is StaticOptionalEvent.Finish -> transitionTrackingAt8
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_optional.scxml:28 :: _machine
    override fun onEntry(state: StaticOptionalState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticOptionalState.Done -> {
                // SCE-MAP: static_optional.scxml:148 :: done :: _state_body
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    var doneEventData = ""
                    // W3C SCXML 5.5: Evaluate <param> elements (C++ DoneDataHelper::evaluateParams pattern)
                    val doneParams = mutableMapOf<String, Any?>()
                    doneParams["due"] = (vDue)?.let { scePresent -> (scePresent).toString() }
                    doneParams["title"] = (vTitle)?.let { scePresent -> scePresent }
                    doneParams["status"] = (vStatus)?.let { scePresent -> (scePresent).declaredName }
                    doneParams["kept"] = (vItem.due)?.let { scePresent -> (scePresent).toString() }
                    doneParams["days"] = (vDue ?: 0L).toString()
                    // §scxml-5.5: the pairs that survived, `{}` when none did
                    // (C++ DoneDataHelper::evaluateParams). Not left to
                    // buildJsonFromParams, whose empty answer is a <send>'s.
                    doneEventData = if (doneParams.isEmpty()) "{}" else buildJsonFromParams(doneParams)
                    // W3C SCXML 5.5 + 6.3.1: stash onto the engine so the invoking parent's
                    // startInvoke completion callback can lift the payload onto
                    // done.invoke.<id>._event.data. Mirrors C++ AOT stashDonedataAtFinal.
                    stashDonedataAtFinal(doneEventData)
                }
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StaticOptionalState.Tracking -> {
                // SCE-MAP: static_optional.scxml:64 :: tracking :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_optional.scxml:28 :: _machine
    override fun onExit(state: StaticOptionalState) {
        when (state) {
            is StaticOptionalState.Done -> {
                // SCE-MAP: static_optional.scxml:148 :: done :: _state_body
            }
            is StaticOptionalState.Tracking -> {
                // SCE-MAP: static_optional.scxml:64 :: tracking :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_optional.scxml:28 :: _machine
    override fun executeTransitionContent(source: StaticOptionalState, transitionIndex: Int) {
        when (source) {
        is StaticOptionalState.Tracking -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_optional.scxml:67 :: tracking :: _transition_0
                if (pendingDueSetPayload == null) {
                    return
                }

            vDue = pendingDueSetPayload!!.due

            if (try { vTitle = (pendingDueSetPayload!!.title)?.let { scePresent -> com.sce.forge.runtime.SceChecked.bounded(scePresent, 16) }; false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='title'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vStatus = pendingDueSetPayload!!.status

            if (try { vItem = StaticOptionalDueRecord(id = pendingDueSetPayload!!.id, due = pendingDueSetPayload!!.due, title = (pendingDueSetPayload!!.title)?.let { scePresent -> com.sce.forge.runtime.SceChecked.bounded(scePresent, 16) }, status = pendingDueSetPayload!!.status); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='item'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (if (vItems.size < 3) { vItems = vItems + (vItem); false } else { raisePlatformError(StaticOptionalEvent.Error.Execution, "<sce:append target='items'>: the list already holds its capacity of 3"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_optional.scxml:75 :: tracking :: _transition_1

            vDue = null

            vOther = null

            vStatus = null

            vItem = vItem.copy(due = null)
            }
            2 -> {
                // SCE-MAP: static_optional.scxml:82 :: tracking :: _transition_2

            vDays = if (vDue != null) com.sce.forge.runtime.SceOptional.proven(vDue) else 0

            vLate = vDue != null && com.sce.forge.runtime.SceOptional.proven(vDue) > 10

            vEarly = vDue == null || com.sce.forge.runtime.SceOptional.proven(vDue) < 3

            if (try { vFollowing = if (!(vDue == null)) com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceOptional.proven(vDue), 1) else 0; false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='following'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (vTitle == null) {

            if (try { vLabel = com.sce.forge.runtime.SceChecked.bounded("untitled", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='label'>: an integer operation overflowed or failed"); true }) {
                return
            }
            } else if (com.sce.forge.runtime.SceOptional.proven(vTitle) == "soon") {

            if (try { vLabel = com.sce.forge.runtime.SceChecked.bounded("due soon", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='label'>: an integer operation overflowed or failed"); true }) {
                return
            }
            } else {

            if (try { vLabel = com.sce.forge.runtime.SceChecked.bounded(com.sce.forge.runtime.SceOptional.proven(vTitle), 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='label'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if (vStatus != null) {

            vPlain = com.sce.forge.runtime.SceOptional.proven(vStatus)
            }


            if (vItem.due != null) {

            vKept = com.sce.forge.runtime.SceOptional.proven(vItem.due)
            }

            vListed = 0

            vMissing = 0.toUInt()


            for (it in vItems) {


            if (it.due != null) {

            if (try { vListed = com.sce.forge.runtime.SceChecked.add(vListed, com.sce.forge.runtime.SceOptional.proven(it.due)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='listed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            } else {

            if (try { vMissing = com.sce.forge.runtime.SceChecked.add(vMissing, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='missing'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }

            vSame = vDue == vOther

            vAbsent = vDue == null

            vStatusDone = vStatus == StaticOptionalStatusEnum.DONE

            println("due: " + vDue)

            println("status: " + vStatus)
            }
            3 -> {
                // SCE-MAP: static_optional.scxml:117 :: tracking :: _transition_3

            if (try { vLabel = com.sce.forge.runtime.SceChecked.bounded(vTitle ?: "none", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='label'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vDays = vDue ?: com.sce.forge.runtime.SceChecked.mul(vHuge, vHuge); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='days'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vFallbacks = com.sce.forge.runtime.SceChecked.add(vFallbacks, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='fallbacks'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_optional.scxml:124 :: tracking :: _transition_4


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "due", (vDue)?.let { scePresent -> (scePresent).toString() })

            putParam(sendPayload, "title", (vTitle)?.let { scePresent -> scePresent })

            putParam(sendPayload, "status", (vStatus)?.let { scePresent -> (scePresent).declaredName })

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticOptionalEvent.Due.Sent, EventMetadata.external(sendId = "__send_0", origin = sessionId, data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            5 -> {
                // SCE-MAP: static_optional.scxml:133 :: tracking :: _transition_5
                if (pendingDueSentPayload == null) {
                    return
                }

            vEchoed = com.sce.forge.runtime.SceOptional.proven(pendingDueSentPayload!!.due)

            if (try { vEchoTitle = (pendingDueSentPayload!!.title)?.let { scePresent -> com.sce.forge.runtime.SceChecked.bounded(scePresent, 16) }; false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='echoTitle'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vEchoStatus = pendingDueSentPayload!!.status
            }
            6 -> {
                // SCE-MAP: static_optional.scxml:138 :: tracking :: _transition_6
                if (pendingDueSentPayload == null) {
                    return
                }

            vEchoed = -1

            if (try { vEchoTitle = (pendingDueSentPayload!!.title)?.let { scePresent -> com.sce.forge.runtime.SceChecked.bounded(scePresent, 16) }; false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='echoTitle'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vEchoStatus = pendingDueSentPayload!!.status
            }
            7 -> {
                // SCE-MAP: static_optional.scxml:143 :: tracking :: _transition_7

            if (try { vErrors = com.sce.forge.runtime.SceChecked.add(vErrors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
