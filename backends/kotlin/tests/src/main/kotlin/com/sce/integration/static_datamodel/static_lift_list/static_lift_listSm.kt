// SCE-GENERATED — DO NOT EDIT
// source-hash: 65fb42bdd588f958b0fbb29ac261cf039d02606d7afaae0720c045624bd679ba

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_lift_list.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_lift_list.scxml:21 :: _machine

package com.sce.integration.static_datamodel.static_lift_list

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticLiftListState : State {
    data object Lifting : StaticLiftListState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticLiftListEvent : Event {
    sealed interface Error : StaticLiftListEvent {
        data object Execution : Error
    }
    sealed interface Text : StaticLiftListEvent {
        data object Given : Text
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticLiftListPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticLiftListTextGivenPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `text.given`. Consumers inject it via the `raiseTextGiven` seam
// on the machine — they never name this class directly.
data class StaticLiftListTextGivenPayload(val text: String)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Item` datamodel value. */
data class StaticLiftListItemRecord(val title: String, val at: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("title" to SavedValues.of(title), "at" to SavedValues.of(at))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticLiftListItemRecord = StaticLiftListItemRecord(title = SavedValues.string(SavedValues.field(value, what, "title"), "$what.title", 16), at = SavedValues.uint8(SavedValues.field(value, what, "at"), "$what.at"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Plan` value that holds a composed value (docs/adr/0086). */
data class StaticLiftListPlanRecord(val title: String, val items: List<StaticLiftListItemRecord>, val priority: UByte) {
    init {
        com.sce.runtime.ValueBounds.text("title", title, 16)
        com.sce.runtime.ValueBounds.listWithin("items", items, 1, 3) { _, _ -> }
    }

    /** What a value is read back from a saved state through (`fromSaved`). */
    companion object
}
/** This value as a saved state writes it. */
fun StaticLiftListPlanRecord.toSaved(): Any = linkedMapOf<String, Any?>("title" to SavedValues.of(this.title), "items" to SavedValues.list(this.items) { sceSaved0 -> sceSaved0.toSaved() }, "priority" to SavedValues.of(this.priority))

/** The value a saved state holds, refused unless it is one. */
fun StaticLiftListPlanRecord.Companion.fromSaved(value: Any?, what: String): StaticLiftListPlanRecord = StaticLiftListPlanRecord(title = SavedValues.string(SavedValues.field(value, what, "title"), "$what.title", 16), items = SavedValues.listWithin(SavedValues.field(value, what, "items"), "$what.items", 1, 3) { sceSaved0, sceWhat0 -> StaticLiftListItemRecord.fromSaved(sceSaved0, sceWhat0) }, priority = SavedValues.uint8(SavedValues.field(value, what, "priority"), "$what.priority"))
/** SCE Accepted Subset §2.15: an `enum:refusal_code` datamodel value. */
enum class StaticLiftListRefusalCodeEnum(val declaredName: String) {
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
        fun fromSaved(value: Any?, what: String): StaticLiftListRefusalCodeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of refusal_code")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticLiftListRefusalCodeEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:LiftRefusal` datamodel value. */
data class StaticLiftListLiftRefusalRecord(val code: StaticLiftListRefusalCodeEnum, val path: String, val bound: UInt?) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("code" to code.toSaved(), "path" to SavedValues.of(path), "bound" to SavedValues.optional(bound) { SavedValues.of(it) })

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticLiftListLiftRefusalRecord = StaticLiftListLiftRefusalRecord(code = StaticLiftListRefusalCodeEnum.fromSaved(SavedValues.field(value, what, "code"), "$what.code"), path = SavedValues.string(SavedValues.field(value, what, "path"), "$what.path", 256), bound = SavedValues.optional(SavedValues.field(value, what, "bound"), "$what.bound") { e, w -> SavedValues.uint32(e, w) })
    }
}
/** The bounds this machine is built with, each by the name `sce:limit` gives it (docs/adr/0050). */
object StaticLiftListLimits {
    /** The bound `sce:limit="lift_path"` names, as this build resolved it. */
    const val LIFT_PATH: UInt = 256u
}
// --- State Machine (W3C SCXML) ---

class StaticLiftListStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticLiftListState, StaticLiftListEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `answer` datamodel variable, published (`sce:direction="out"`). */
    private var vAnswer: StaticLiftListPlanRecord? = null

    /** What the published `answer` holds. */
    val answer: StaticLiftListPlanRecord? get() = vAnswer
    /** W3C SCXML 5.2: the `why` datamodel variable, published (`sce:direction="out"`). */
    private var vWhy: StaticLiftListLiftRefusalRecord? = null

    /** What the published `why` holds. */
    val why: StaticLiftListLiftRefusalRecord? get() = vWhy
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
        var lifts: UInt? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.lifts?.let { vLifts = it }
        params.errors?.let { vErrors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val answer: StaticLiftListPlanRecord?,
        val why: StaticLiftListLiftRefusalRecord?,
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
        val configuration: Set<StaticLiftListState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        answer = vAnswer,
        why = vWhy,
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
        first = "278e253fe85156670b7b4f447d0185c5e8c8d979b1ec705f860e0ee004f52a5a",
        revised = "b1902fc18d4de773fb70cb2f5b49e3f401597bfe11a18a3007e4491081f79c57",
        format2First = "f3fa049a25f94721181b3af6944e982e3f185467a30439495bcd514aa60ff730",
        format2Revised = "7fc625a9db3df62e3a393adfec6e2ae2caefffd13e14b692078c07971b1ddc82",
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
                    RecordHolder("why", false),
                ),
            ),
        ),
        enums = listOf(
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
        val saved1 = SavedValues.optional(savedGrown.variable("answer"), "answer") { e, w -> StaticLiftListPlanRecord.fromSaved(e, w) }
        val saved2 = SavedValues.optional(savedGrown.variable("why"), "why") { e, w -> StaticLiftListLiftRefusalRecord.fromSaved(e, w) }
        val saved3 = SavedValues.uint32(savedGrown.variable("lifts"), "lifts")
        val saved4 = SavedValues.uint32(savedGrown.variable("errors"), "errors")
        return {
            vAnswer = saved1
            vWhy = saved2
            vLifts = saved3
            vErrors = saved4
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
        SavedValues.optional(savedGrown.variable("answer"), "answer") { e, w -> StaticLiftListPlanRecord.fromSaved(e, w) }
        SavedValues.optional(savedGrown.variable("why"), "why") { e, w -> StaticLiftListLiftRefusalRecord.fromSaved(e, w) }
        SavedValues.uint32(savedGrown.variable("lifts"), "lifts")
        SavedValues.uint32(savedGrown.variable("errors"), "errors")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingTextGivenPayload: StaticLiftListTextGivenPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, on which the engine refuses the delivery whole and
    // takes an error.execution in its place (docs/adr/0086, section 4.2). Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticLiftListEvent, metadata: EventMetadata) {
        pendingTextGivenPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticLiftListTextGivenPayload -> pendingTextGivenPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticLiftListEvent.Text.Given) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTextGivenPayload = StaticLiftListTextGivenPayload(fields.string("text", 512))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `text.given` — binds the event name and the payload field values in one call.
    fun raiseTextGiven(text: String) {
        send(
            StaticLiftListEvent.Text.Given,
            EventMetadata(
                type = "external",
                typedPayload = StaticLiftListTextGivenPayload(text),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("text" to text))
            )
        )
    }


    override val initialState: StaticLiftListState = StaticLiftListState.Lifting

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
    override val documentInitialTargets: List<EntryTarget<StaticLiftListState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticLiftListState, HistoryId>> =
            listOf(StateTarget(StaticLiftListState.Lifting))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: lifting's transition 0, as the microstep reads it.
        val transitionLiftingAt0 = EnabledTransition<StaticLiftListState, HistoryId>(
            StaticLiftListState.Lifting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: lifting's transition 1, as the microstep reads it.
        val transitionLiftingAt1 = EnabledTransition<StaticLiftListState, HistoryId>(
            StaticLiftListState.Lifting,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticLiftListState? = when (stateId) {
        "lifting" -> StaticLiftListState.Lifting
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticLiftListState): String = when (state) {
        is StaticLiftListState.Lifting -> "lifting"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticLiftListState): Int = when (state) {
        is StaticLiftListState.Lifting -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticLiftListEvent? = when (name) {
        "error.execution" -> StaticLiftListEvent.Error.Execution
        "text.given" -> StaticLiftListEvent.Text.Given
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticLiftListEvent): String? = when (event) {
        is StaticLiftListEvent.Error.Execution -> "error.execution"
        is StaticLiftListEvent.Text.Given -> "text.given"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticLiftListState, event: StaticLiftListEvent?): EnabledTransition<StaticLiftListState, HistoryId>? = when (state) {
        is StaticLiftListState.Lifting -> firstEnabledTransitionOfLifting(state, event)
    }


    // lifting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfLifting(state: StaticLiftListState, event: StaticLiftListEvent?): EnabledTransition<StaticLiftListState, HistoryId>? = when {
        event is StaticLiftListEvent.Text.Given -> TransitionsPart0.transitionLiftingAt0
        event is StaticLiftListEvent.Error.Execution -> TransitionsPart0.transitionLiftingAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_lift_list.scxml:21 :: _machine
    override fun onEntry(state: StaticLiftListState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticLiftListState.Lifting -> onEntryOfLifting()
        }
    }


    private fun onEntryOfLifting() {
                // SCE-MAP: static_lift_list.scxml:33 :: lifting :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_lift_list.scxml:21 :: _machine
    override fun onExit(state: StaticLiftListState) {
        when (state) {
            is StaticLiftListState.Lifting -> onExitOfLifting()
        }
    }


    private fun onExitOfLifting() {
                // SCE-MAP: static_lift_list.scxml:33 :: lifting :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_lift_list.scxml:21 :: _machine
    override fun executeTransitionContent(source: StaticLiftListState, transitionIndex: Int) {
        when (source) {
            is StaticLiftListState.Lifting -> transitionContentOfLifting(transitionIndex)
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
            else -> {}
        }
    }


    private fun transitionContentOfLiftingAt0() {
                // SCE-MAP: static_lift_list.scxml:34 :: lifting :: _transition_0
                if (pendingTextGivenPayload == null) {
                    return
                }

            when (val sceLift = com.sce.runtime.EventPayload.liftValue(pendingTextGivenPayload!!.text, 3, 256) { it.record("{\"priority\":3}") { StaticLiftListPlanRecord(title = it.field("title").let { it.string(16) }, items = it.field("items").let { it.listWithin(1, 3) { it.record(null) { StaticLiftListItemRecord(title = it.field("title").let { it.string(16) }, at = it.field("at").let { it.uint8() }) } } }, priority = it.field("priority").let { it.uint8() }) } }) { is com.sce.runtime.EventPayload.LiftOutcome.Lifted -> { vAnswer = sceLift.value; vWhy = null }; is com.sce.runtime.EventPayload.LiftOutcome.Refused -> { vWhy = StaticLiftListLiftRefusalRecord(code = when (sceLift.refusal.code) { com.sce.runtime.EventPayload.RefusalCode.NOT_JSON -> StaticLiftListRefusalCodeEnum.NOT_JSON; com.sce.runtime.EventPayload.RefusalCode.NOT_OBJECT -> StaticLiftListRefusalCodeEnum.NOT_OBJECT; com.sce.runtime.EventPayload.RefusalCode.TOO_DEEP -> StaticLiftListRefusalCodeEnum.TOO_DEEP; com.sce.runtime.EventPayload.RefusalCode.MISSING -> StaticLiftListRefusalCodeEnum.MISSING; com.sce.runtime.EventPayload.RefusalCode.NULL_NOT_OPTIONAL -> StaticLiftListRefusalCodeEnum.NULL_NOT_OPTIONAL; com.sce.runtime.EventPayload.RefusalCode.WRONG_KIND -> StaticLiftListRefusalCodeEnum.WRONG_KIND; com.sce.runtime.EventPayload.RefusalCode.BAD_NUMBER -> StaticLiftListRefusalCodeEnum.BAD_NUMBER; com.sce.runtime.EventPayload.RefusalCode.OUT_OF_RANGE -> StaticLiftListRefusalCodeEnum.OUT_OF_RANGE; com.sce.runtime.EventPayload.RefusalCode.TOO_LONG -> StaticLiftListRefusalCodeEnum.TOO_LONG; com.sce.runtime.EventPayload.RefusalCode.TOO_MANY -> StaticLiftListRefusalCodeEnum.TOO_MANY; com.sce.runtime.EventPayload.RefusalCode.UNKNOWN_VARIANT -> StaticLiftListRefusalCodeEnum.UNKNOWN_VARIANT; com.sce.runtime.EventPayload.RefusalCode.NOT_BYTE_TEXT -> StaticLiftListRefusalCodeEnum.NOT_BYTE_TEXT; com.sce.runtime.EventPayload.RefusalCode.NUL_IN_TEXT -> StaticLiftListRefusalCodeEnum.NUL_IN_TEXT; com.sce.runtime.EventPayload.RefusalCode.TOO_FEW -> StaticLiftListRefusalCodeEnum.TOO_FEW; }, path = sceLift.refusal.pointer, bound = sceLift.refusal.bound?.toUInt()) } }

            if (try { vLifts = com.sce.forge.runtime.SceChecked.add(vLifts, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticLiftListEvent.Error.Execution, "<assign location='lifts'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfLiftingAt1() {
                // SCE-MAP: static_lift_list.scxml:38 :: lifting :: _transition_1

            if (try { vErrors = com.sce.forge.runtime.SceChecked.add(vErrors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticLiftListEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
