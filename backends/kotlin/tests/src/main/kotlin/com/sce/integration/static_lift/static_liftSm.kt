// SCE-GENERATED — DO NOT EDIT
// source-hash: 50ba4afa70e08ed0ad002509e6baae9623e17ddc8afb219bbba6da905f187359

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_lift.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_lift.scxml:19 :: _machine

package com.sce.integration.static_lift

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticLiftState : State {
    data object Lifting : StaticLiftState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticLiftEvent : Event {
    sealed interface Error : StaticLiftEvent {
        data object Execution : Error
    }
    sealed interface Text : StaticLiftEvent {
        data object Given : Text
        data object Kept : Text
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticLiftPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticLiftTextGivenPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `text.given`. Consumers inject it via the `raiseTextGiven` seam
// on the machine — they never name this class directly.
data class StaticLiftTextGivenPayload(val text: String)

// StaticLiftTextKeptPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `text.kept`. Consumers inject it via the `raiseTextKept` seam
// on the machine — they never name this class directly.
data class StaticLiftTextKeptPayload(val text: String)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:enum_lift_kind` datamodel value. */
enum class StaticLiftEnumLiftKindEnum(val declaredName: String) {
    MEETING("meeting"),
    TASK("task");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        MEETING -> 0u.toUByte()
        TASK -> 1u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticLiftEnumLiftKindEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of enum_lift_kind")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticLiftEnumLiftKindEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Answer` datamodel value. */
data class StaticLiftAnswerRecord(val id: UInt, val amount: Long, val title: String, val kind: StaticLiftEnumLiftKindEnum, val note: String?, val tag: ByteArray, val priority: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "amount" to SavedValues.of(amount), "title" to SavedValues.of(title), "kind" to kind.toSaved(), "note" to SavedValues.optional(note) { SavedValues.of(it) }, "tag" to SavedValues.of(tag), "priority" to SavedValues.of(priority))

    override fun equals(other: Any?): Boolean =
        other is StaticLiftAnswerRecord && id == other.id && amount == other.amount && title == other.title && kind == other.kind && note == other.note && tag.contentEquals(other.tag) && priority == other.priority

    override fun hashCode(): Int = 31 * (31 * (31 * (31 * (31 * (31 * (31 * (0) + id.hashCode()) + amount.hashCode()) + title.hashCode()) + kind.hashCode()) + note.hashCode()) + tag.contentHashCode()) + priority.hashCode()

    /** This value with a copy of each byte string, which a host may write into. */
    fun detached(): StaticLiftAnswerRecord = copy(tag = tag.copyOf())

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticLiftAnswerRecord = StaticLiftAnswerRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), amount = SavedValues.int64(SavedValues.field(value, what, "amount"), "$what.amount"), title = SavedValues.string(SavedValues.field(value, what, "title"), "$what.title", 16), kind = StaticLiftEnumLiftKindEnum.fromSaved(SavedValues.field(value, what, "kind"), "$what.kind"), note = SavedValues.optional(SavedValues.field(value, what, "note"), "$what.note") { e, w -> SavedValues.string(e, w, 8) }, tag = SavedValues.bytes(SavedValues.field(value, what, "tag"), "$what.tag", 4), priority = SavedValues.uint8(SavedValues.field(value, what, "priority"), "$what.priority"))
    }
}
/** SCE Accepted Subset §2.15: an `enum:refusal_code` datamodel value. */
enum class StaticLiftRefusalCodeEnum(val declaredName: String) {
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
        fun fromSaved(value: Any?, what: String): StaticLiftRefusalCodeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of refusal_code")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticLiftRefusalCodeEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:LiftRefusal` datamodel value. */
data class StaticLiftLiftRefusalRecord(val code: StaticLiftRefusalCodeEnum, val path: String, val bound: UInt?) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("code" to code.toSaved(), "path" to SavedValues.of(path), "bound" to SavedValues.optional(bound) { SavedValues.of(it) })

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticLiftLiftRefusalRecord = StaticLiftLiftRefusalRecord(code = StaticLiftRefusalCodeEnum.fromSaved(SavedValues.field(value, what, "code"), "$what.code"), path = SavedValues.string(SavedValues.field(value, what, "path"), "$what.path", 256), bound = SavedValues.optional(SavedValues.field(value, what, "bound"), "$what.bound") { e, w -> SavedValues.uint32(e, w) })
    }
}
/** The bounds this machine is built with, each by the name `sce:limit` gives it (docs/adr/0050). */
object StaticLiftLimits {
    /** The bound `sce:limit="lift_path"` names, as this build resolved it. */
    const val LIFT_PATH: UInt = 256u
}
// --- State Machine (W3C SCXML) ---

class StaticLiftStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticLiftState, StaticLiftEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `answer` datamodel variable, published (`sce:direction="out"`). */
    private var vAnswer: StaticLiftAnswerRecord? = null

    /** What the published `answer` holds. */
    val answer: StaticLiftAnswerRecord? get() = vAnswer?.detached()
    /** W3C SCXML 5.2: the `why` datamodel variable, published (`sce:direction="out"`). */
    private var vWhy: StaticLiftLiftRefusalRecord? = null

    /** What the published `why` holds. */
    val why: StaticLiftLiftRefusalRecord? get() = vWhy
    /** W3C SCXML 5.2: the `said` datamodel variable, published (`sce:direction="out"`). */
    private var vSaid: String = ""

    /** What the published `said` holds. */
    val said: String get() = vSaid
    /** W3C SCXML 5.2: the `lifts` datamodel variable, published (`sce:direction="out"`). */
    private var vLifts: UInt = 0.toUInt()

    /** What the published `lifts` holds. */
    val lifts: UInt get() = vLifts
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    private var vErrors: UInt = 0.toUInt()

    /** What the published `errors` holds. */
    val errors: UInt get() = vErrors

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var said: String? = null
        var lifts: UInt? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.said?.let { vSaid = it }
        params.lifts?.let { vLifts = it }
        params.errors?.let { vErrors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val answer: StaticLiftAnswerRecord?,
        val why: StaticLiftLiftRefusalRecord?,
        val said: String,
        val lifts: UInt,
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
        val configuration: Set<StaticLiftState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        answer = vAnswer?.detached(),
        why = vWhy,
        said = vSaid,
        lifts = vLifts,
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
        first = "bcc64bc7d8cbabe025c700ffc5d239d7f9ac55e720b5338913046399534bc5f0",
        revised = "f86ff2576aa4492e7db15825af60d0f20acaa2442b8344bc223c75c7b4ba4192",
        format2First = "a0338bb4be14b3f0513f97a845c32c3635f6e8bc270638ca868ed0e4f7646e57",
        format2Revised = "aaa57de3b4aec9e8b2b203ac096e30cd24ec843f1def6d5398e47e99737c7c95",
        records = listOf(
            RecordLineage(
                event = "lift.answer",
                revision = 2,
                revisions = listOf("346ba2c78ac1d5e5d878a2f364be668e2f2503a0f555b1d323584dc30cd26483"),
                added = listOf(
                    AddedField("priority", 2, "3"),
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("answer", false),
                ),
            ),
            RecordLineage(
                event = "lift.refusal",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("why", false),
                ),
            ),
        ),
        enums = listOf(
            EnumLineage(
                key = "enum:enum_lift_kind.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("answer", "kind", false),
                ),
            ),
            EnumLineage(
                key = "enum:sce:std/lift/refusal_code.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("why", "code", false),
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
        variables += "answer" to SavedValues.optional(vAnswer) { it.toSaved() }
        variables += "why" to SavedValues.optional(vWhy) { it.toSaved() }
        variables += "said" to SavedValues.of(vSaid)
        variables += "lifts" to SavedValues.of(vLifts)
        variables += "errors" to SavedValues.of(vErrors)
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
        val saved1 = SavedValues.optional(savedGrown.variable("answer"), "answer") { e, w -> StaticLiftAnswerRecord.fromSaved(e, w) }
        val saved2 = SavedValues.optional(savedGrown.variable("why"), "why") { e, w -> StaticLiftLiftRefusalRecord.fromSaved(e, w) }
        val saved3 = SavedValues.string(savedGrown.variable("said"), "said", 512)
        val saved4 = SavedValues.uint32(savedGrown.variable("lifts"), "lifts")
        val saved5 = SavedValues.uint32(savedGrown.variable("errors"), "errors")
        return {
            vAnswer = saved1
            vWhy = saved2
            vSaid = saved3
            vLifts = saved4
            vErrors = saved5
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
        SavedValues.optional(savedGrown.variable("answer"), "answer") { e, w -> StaticLiftAnswerRecord.fromSaved(e, w) }
        SavedValues.optional(savedGrown.variable("why"), "why") { e, w -> StaticLiftLiftRefusalRecord.fromSaved(e, w) }
        SavedValues.string(savedGrown.variable("said"), "said", 512)
        SavedValues.uint32(savedGrown.variable("lifts"), "lifts")
        SavedValues.uint32(savedGrown.variable("errors"), "errors")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingTextGivenPayload: StaticLiftTextGivenPayload? = null
    private var pendingTextKeptPayload: StaticLiftTextKeptPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, on which the engine refuses the delivery whole and
    // takes an error.execution in its place (docs/adr/0086, section 4.2). Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticLiftEvent, metadata: EventMetadata) {
        pendingTextGivenPayload = null
        pendingTextKeptPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticLiftTextGivenPayload -> pendingTextGivenPayload = tp
            is StaticLiftTextKeptPayload -> pendingTextKeptPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticLiftEvent.Text.Given) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTextGivenPayload = StaticLiftTextGivenPayload(fields.string("text", 512))
                } else if (event == StaticLiftEvent.Text.Kept) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTextKeptPayload = StaticLiftTextKeptPayload(fields.string("text", 512))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `text.given` — binds the event name and the payload field values in one call.
    fun raiseTextGiven(text: String) {
        send(
            StaticLiftEvent.Text.Given,
            EventMetadata(
                type = "external",
                typedPayload = StaticLiftTextGivenPayload(text),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("text" to text))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `text.kept` — binds the event name and the payload field values in one call.
    fun raiseTextKept(text: String) {
        send(
            StaticLiftEvent.Text.Kept,
            EventMetadata(
                type = "external",
                typedPayload = StaticLiftTextKeptPayload(text),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("text" to text))
            )
        )
    }


    override val initialState: StaticLiftState = StaticLiftState.Lifting

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
    override val documentInitialTargets: List<EntryTarget<StaticLiftState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticLiftState, HistoryId>> =
            listOf(StateTarget(StaticLiftState.Lifting))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: lifting's transition 0, as the microstep reads it.
        val transitionLiftingAt0 = EnabledTransition<StaticLiftState, HistoryId>(
            StaticLiftState.Lifting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: lifting's transition 1, as the microstep reads it.
        val transitionLiftingAt1 = EnabledTransition<StaticLiftState, HistoryId>(
            StaticLiftState.Lifting,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: lifting's transition 2, as the microstep reads it.
        val transitionLiftingAt2 = EnabledTransition<StaticLiftState, HistoryId>(
            StaticLiftState.Lifting,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticLiftState? = when (stateId) {
        "lifting" -> StaticLiftState.Lifting
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticLiftState): String = when (state) {
        is StaticLiftState.Lifting -> "lifting"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticLiftState): Int = when (state) {
        is StaticLiftState.Lifting -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticLiftEvent? = when (name) {
        "error.execution" -> StaticLiftEvent.Error.Execution
        "text.given" -> StaticLiftEvent.Text.Given
        "text.kept" -> StaticLiftEvent.Text.Kept
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticLiftEvent): String? = when (event) {
        is StaticLiftEvent.Error.Execution -> "error.execution"
        is StaticLiftEvent.Text.Given -> "text.given"
        is StaticLiftEvent.Text.Kept -> "text.kept"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticLiftState, event: StaticLiftEvent?): EnabledTransition<StaticLiftState, HistoryId>? = when (state) {
        is StaticLiftState.Lifting -> firstEnabledTransitionOfLifting(state, event)
    }


    // lifting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfLifting(state: StaticLiftState, event: StaticLiftEvent?): EnabledTransition<StaticLiftState, HistoryId>? = when {
        event is StaticLiftEvent.Text.Given -> TransitionsPart0.transitionLiftingAt0
        event is StaticLiftEvent.Text.Kept -> TransitionsPart0.transitionLiftingAt1
        event is StaticLiftEvent.Error.Execution -> TransitionsPart0.transitionLiftingAt2
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_lift.scxml:19 :: _machine
    override fun onEntry(state: StaticLiftState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticLiftState.Lifting -> onEntryOfLifting()
        }
    }


    private fun onEntryOfLifting() {
                // SCE-MAP: static_lift.scxml:32 :: lifting :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_lift.scxml:19 :: _machine
    override fun onExit(state: StaticLiftState) {
        when (state) {
            is StaticLiftState.Lifting -> onExitOfLifting()
        }
    }


    private fun onExitOfLifting() {
                // SCE-MAP: static_lift.scxml:32 :: lifting :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_lift.scxml:19 :: _machine
    override fun executeTransitionContent(source: StaticLiftState, transitionIndex: Int) {
        when (source) {
            is StaticLiftState.Lifting -> transitionContentOfLifting(transitionIndex)
        }
    }


    // The content of `lifting`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfLifting(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfLiftingAt0()
            1 -> transitionContentOfLiftingAt1()
            2 -> transitionContentOfLiftingAt2()
            else -> {}
        }
    }


    private fun transitionContentOfLiftingAt0() {
                // SCE-MAP: static_lift.scxml:33 :: lifting :: _transition_0
                if (pendingTextGivenPayload == null) {
                    return
                }

            when (val sceLift = com.sce.runtime.EventPayload.liftRecord(pendingTextGivenPayload!!.text, 1, 256, "{\"priority\":3}") { r -> StaticLiftAnswerRecord(id = r.field("id").let { v -> v.uint32() }, amount = r.field("amount").let { v -> v.int64() }, title = r.field("title").let { v -> v.string(16) }, kind = r.field("kind").let { v -> v.name().let { name -> when (name) { "meeting" -> StaticLiftEnumLiftKindEnum.MEETING; "task" -> StaticLiftEnumLiftKindEnum.TASK; else -> throw v.unknownVariant(name, "enum_lift_kind") } } }, note = r.field("note").let { v -> v.optional { present -> present.string(8) } }, tag = r.field("tag").let { v -> v.bytes(4) }, priority = r.field("priority").let { v -> v.uint8() }) }) { is com.sce.runtime.EventPayload.LiftOutcome.Lifted -> { vAnswer = sceLift.value; vWhy = null }; is com.sce.runtime.EventPayload.LiftOutcome.Refused -> { vWhy = StaticLiftLiftRefusalRecord(code = when (sceLift.refusal.code) { com.sce.runtime.EventPayload.RefusalCode.NOT_JSON -> StaticLiftRefusalCodeEnum.NOT_JSON; com.sce.runtime.EventPayload.RefusalCode.NOT_OBJECT -> StaticLiftRefusalCodeEnum.NOT_OBJECT; com.sce.runtime.EventPayload.RefusalCode.TOO_DEEP -> StaticLiftRefusalCodeEnum.TOO_DEEP; com.sce.runtime.EventPayload.RefusalCode.MISSING -> StaticLiftRefusalCodeEnum.MISSING; com.sce.runtime.EventPayload.RefusalCode.NULL_NOT_OPTIONAL -> StaticLiftRefusalCodeEnum.NULL_NOT_OPTIONAL; com.sce.runtime.EventPayload.RefusalCode.WRONG_KIND -> StaticLiftRefusalCodeEnum.WRONG_KIND; com.sce.runtime.EventPayload.RefusalCode.BAD_NUMBER -> StaticLiftRefusalCodeEnum.BAD_NUMBER; com.sce.runtime.EventPayload.RefusalCode.OUT_OF_RANGE -> StaticLiftRefusalCodeEnum.OUT_OF_RANGE; com.sce.runtime.EventPayload.RefusalCode.TOO_LONG -> StaticLiftRefusalCodeEnum.TOO_LONG; com.sce.runtime.EventPayload.RefusalCode.TOO_MANY -> StaticLiftRefusalCodeEnum.TOO_MANY; com.sce.runtime.EventPayload.RefusalCode.UNKNOWN_VARIANT -> StaticLiftRefusalCodeEnum.UNKNOWN_VARIANT; com.sce.runtime.EventPayload.RefusalCode.NOT_BYTE_TEXT -> StaticLiftRefusalCodeEnum.NOT_BYTE_TEXT; com.sce.runtime.EventPayload.RefusalCode.NUL_IN_TEXT -> StaticLiftRefusalCodeEnum.NUL_IN_TEXT; com.sce.runtime.EventPayload.RefusalCode.TOO_FEW -> StaticLiftRefusalCodeEnum.TOO_FEW; }, path = sceLift.refusal.pointer, bound = sceLift.refusal.bound?.toUInt()) } }

            if (try { vLifts = com.sce.forge.runtime.SceChecked.add(vLifts, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticLiftEvent.Error.Execution, "<assign location='lifts'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfLiftingAt1() {
                // SCE-MAP: static_lift.scxml:37 :: lifting :: _transition_1
                if (pendingTextKeptPayload == null) {
                    return
                }

            if (try { vSaid = com.sce.forge.runtime.SceChecked.bounded(pendingTextKeptPayload!!.text, 512); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticLiftEvent.Error.Execution, "<assign location='said'>: an integer operation overflowed or failed"); true }) {
                return
            }

            when (val sceLift = com.sce.runtime.EventPayload.liftRecord(vSaid, 1, 256, "{\"priority\":3}") { r -> StaticLiftAnswerRecord(id = r.field("id").let { v -> v.uint32() }, amount = r.field("amount").let { v -> v.int64() }, title = r.field("title").let { v -> v.string(16) }, kind = r.field("kind").let { v -> v.name().let { name -> when (name) { "meeting" -> StaticLiftEnumLiftKindEnum.MEETING; "task" -> StaticLiftEnumLiftKindEnum.TASK; else -> throw v.unknownVariant(name, "enum_lift_kind") } } }, note = r.field("note").let { v -> v.optional { present -> present.string(8) } }, tag = r.field("tag").let { v -> v.bytes(4) }, priority = r.field("priority").let { v -> v.uint8() }) }) { is com.sce.runtime.EventPayload.LiftOutcome.Lifted -> { vAnswer = sceLift.value; vWhy = null }; is com.sce.runtime.EventPayload.LiftOutcome.Refused -> { vWhy = StaticLiftLiftRefusalRecord(code = when (sceLift.refusal.code) { com.sce.runtime.EventPayload.RefusalCode.NOT_JSON -> StaticLiftRefusalCodeEnum.NOT_JSON; com.sce.runtime.EventPayload.RefusalCode.NOT_OBJECT -> StaticLiftRefusalCodeEnum.NOT_OBJECT; com.sce.runtime.EventPayload.RefusalCode.TOO_DEEP -> StaticLiftRefusalCodeEnum.TOO_DEEP; com.sce.runtime.EventPayload.RefusalCode.MISSING -> StaticLiftRefusalCodeEnum.MISSING; com.sce.runtime.EventPayload.RefusalCode.NULL_NOT_OPTIONAL -> StaticLiftRefusalCodeEnum.NULL_NOT_OPTIONAL; com.sce.runtime.EventPayload.RefusalCode.WRONG_KIND -> StaticLiftRefusalCodeEnum.WRONG_KIND; com.sce.runtime.EventPayload.RefusalCode.BAD_NUMBER -> StaticLiftRefusalCodeEnum.BAD_NUMBER; com.sce.runtime.EventPayload.RefusalCode.OUT_OF_RANGE -> StaticLiftRefusalCodeEnum.OUT_OF_RANGE; com.sce.runtime.EventPayload.RefusalCode.TOO_LONG -> StaticLiftRefusalCodeEnum.TOO_LONG; com.sce.runtime.EventPayload.RefusalCode.TOO_MANY -> StaticLiftRefusalCodeEnum.TOO_MANY; com.sce.runtime.EventPayload.RefusalCode.UNKNOWN_VARIANT -> StaticLiftRefusalCodeEnum.UNKNOWN_VARIANT; com.sce.runtime.EventPayload.RefusalCode.NOT_BYTE_TEXT -> StaticLiftRefusalCodeEnum.NOT_BYTE_TEXT; com.sce.runtime.EventPayload.RefusalCode.NUL_IN_TEXT -> StaticLiftRefusalCodeEnum.NUL_IN_TEXT; com.sce.runtime.EventPayload.RefusalCode.TOO_FEW -> StaticLiftRefusalCodeEnum.TOO_FEW; }, path = sceLift.refusal.pointer, bound = sceLift.refusal.bound?.toUInt()) } }

            if (try { vLifts = com.sce.forge.runtime.SceChecked.add(vLifts, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticLiftEvent.Error.Execution, "<assign location='lifts'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfLiftingAt2() {
                // SCE-MAP: static_lift.scxml:42 :: lifting :: _transition_2

            if (try { vErrors = com.sce.forge.runtime.SceChecked.add(vErrors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticLiftEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
