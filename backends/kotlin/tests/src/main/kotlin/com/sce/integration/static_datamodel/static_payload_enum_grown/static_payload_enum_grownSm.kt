// SCE-GENERATED — DO NOT EDIT
// source-hash: 14d897c9be408f8e2aa8b8289e8cf3808dd386abf8651c9342038e1ec7901597
// source: static_payload_enum_grown.scxml

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_payload_enum_grown.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_payload_enum_grown.scxml:18 :: _machine

package com.sce.integration.static_datamodel.static_payload_enum_grown

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticPayloadEnumGrownState : State {
    data object Browsing : StaticPayloadEnumGrownState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticPayloadEnumGrownEvent : Event {
    sealed interface Error : StaticPayloadEnumGrownEvent {
        data object Execution : Error
    }
    sealed interface View : StaticPayloadEnumGrownEvent {
        data object Grown : View
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticPayloadEnumGrownPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticPayloadEnumGrownViewGrownPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `view.grown`. Consumers inject it via the `raiseViewGrown` seam
// on the machine — they never name this class directly.
data class StaticPayloadEnumGrownViewGrownPayload(val layout: StaticPayloadEnumGrownViewModeEnum, val marks: List<StaticPayloadEnumGrownMarkRecord>) {
    init {
        com.sce.runtime.ValueBounds.list("marks", marks, 4) { _, _ -> }
    }
}


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:ViewMode` datamodel value. */
enum class StaticPayloadEnumGrownViewModeEnum(val declaredName: String) {
    MONTH("month"),
    WEEK("week"),
    DAY("day"),
    AGENDA_LIST("agenda_list");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        MONTH -> 0u.toUByte()
        WEEK -> 1u.toUByte()
        DAY -> 2u.toUByte()
        AGENDA_LIST -> 3u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticPayloadEnumGrownViewModeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of ViewMode")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticPayloadEnumGrownViewModeEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Mark` datamodel value. */
data class StaticPayloadEnumGrownMarkRecord(val day: UInt, val shift: Long?) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("day" to SavedValues.of(day), "shift" to SavedValues.optional(shift) { SavedValues.of(it) })

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticPayloadEnumGrownMarkRecord = StaticPayloadEnumGrownMarkRecord(day = SavedValues.uint32(SavedValues.field(value, what, "day"), "$what.day"), shift = SavedValues.optional(SavedValues.field(value, what, "shift"), "$what.shift") { e, w -> SavedValues.int64(e, w) })
    }
}
// --- State Machine (W3C SCXML) ---

class StaticPayloadEnumGrownStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticPayloadEnumGrownState, StaticPayloadEnumGrownEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `layout` datamodel variable, published (`sce:direction="out"`). */
    private var vLayout: StaticPayloadEnumGrownViewModeEnum = StaticPayloadEnumGrownViewModeEnum.MONTH

    /** What the published `layout` holds. */
    val layout: StaticPayloadEnumGrownViewModeEnum get() = vLayout
    /** W3C SCXML 5.2: the `agendas` datamodel variable, published (`sce:direction="out"`). */
    private var vAgendas: UInt = 0.toUInt()

    /** What the published `agendas` holds. */
    val agendas: UInt get() = vAgendas
    /** W3C SCXML 5.2: the `changes` datamodel variable, published (`sce:direction="out"`). */
    private var vChanges: UInt = 0.toUInt()

    /** What the published `changes` holds. */
    val changes: UInt get() = vChanges
    /** W3C SCXML 5.2: the `marks` datamodel variable, published (`sce:direction="out"`). */
    private var vMarks: UInt = 0.toUInt()

    /** What the published `marks` holds. */
    val marks: UInt get() = vMarks
    /** W3C SCXML 5.2: the `shown` datamodel variable, published (`sce:direction="out"`). */
    private var vShown: UInt = 0.toUInt()

    /** What the published `shown` holds. */
    val shown: UInt get() = vShown
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
        var agendas: UInt? = null
        var changes: UInt? = null
        var marks: UInt? = null
        var shown: UInt? = null
        var refusals: UInt? = null
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
        vLayout = StaticPayloadEnumGrownViewModeEnum.MONTH
        vAgendas = params.agendas ?: (0.toUInt())
        vChanges = params.changes ?: (0.toUInt())
        vMarks = params.marks ?: (0.toUInt())
        vShown = params.shown ?: (0.toUInt())
        vRefusals = params.refusals ?: (0.toUInt())
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val layout: StaticPayloadEnumGrownViewModeEnum,
        val agendas: UInt,
        val changes: UInt,
        val marks: UInt,
        val shown: UInt,
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
        val configuration: Set<StaticPayloadEnumGrownState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        layout = vLayout,
        agendas = vAgendas,
        changes = vChanges,
        marks = vMarks,
        shown = vShown,
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
        first = "f7b1a1d98195a92b5b9908f245f7dde9503a79d6d71696fb8864afec637086c0",
        revised = "d6092e519fca56df2410b098901b42c38c2d81625c30b610252231e3b5ff9dab",
        format2First = "bfcc2f0aedda1c720b3632a2ee00f2e6c810b4971b3866977bda94297f4ff50b",
        format2Revised = "d28fda39720cc46e964b0b19fd5ec30da75525276c20326b24038f8f990a7ddb",
        records = listOf(
        ),
        enums = listOf(
            EnumLineage(
                key = "enum:enum_view_mode.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("layout", null, false),
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
        variables += "layout" to vLayout.toSaved()
        variables += "agendas" to SavedValues.of(vAgendas)
        variables += "changes" to SavedValues.of(vChanges)
        variables += "marks" to SavedValues.of(vMarks)
        variables += "shown" to SavedValues.of(vShown)
        variables += "refusals" to SavedValues.of(vRefusals)
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
        vLayout = StaticPayloadEnumGrownViewModeEnum.fromSaved(savedGrown.variable("layout"), "layout")
        vAgendas = SavedValues.uint32(savedGrown.variable("agendas"), "agendas")
        vChanges = SavedValues.uint32(savedGrown.variable("changes"), "changes")
        vMarks = SavedValues.uint32(savedGrown.variable("marks"), "marks")
        vShown = SavedValues.uint32(savedGrown.variable("shown"), "shown")
        vRefusals = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
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
        StaticPayloadEnumGrownViewModeEnum.fromSaved(savedGrown.variable("layout"), "layout")
        SavedValues.uint32(savedGrown.variable("agendas"), "agendas")
        SavedValues.uint32(savedGrown.variable("changes"), "changes")
        SavedValues.uint32(savedGrown.variable("marks"), "marks")
        SavedValues.uint32(savedGrown.variable("shown"), "shown")
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingViewGrownPayload: StaticPayloadEnumGrownViewGrownPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, on which the engine refuses the delivery whole and
    // takes an error.execution in its place (docs/adr/0086, section 4.2). Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticPayloadEnumGrownEvent, metadata: EventMetadata) {
        pendingViewGrownPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticPayloadEnumGrownViewGrownPayload -> pendingViewGrownPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticPayloadEnumGrownEvent.View.Grown) {
                    val fields = EventPayload.decode(metadata.data, "{\"marks\":[]}")
                    pendingViewGrownPayload = StaticPayloadEnumGrownViewGrownPayload(fields.field("layout").let { when (val sceName = it.name()) { "month" -> StaticPayloadEnumGrownViewModeEnum.MONTH; "week" -> StaticPayloadEnumGrownViewModeEnum.WEEK; "day" -> StaticPayloadEnumGrownViewModeEnum.DAY; "agenda_list" -> StaticPayloadEnumGrownViewModeEnum.AGENDA_LIST; else -> throw it.unknownVariant(sceName, "ViewMode") } }, fields.field("marks").let { it.list(4) { it.record(null) { StaticPayloadEnumGrownMarkRecord(day = it.field("day").let { it.uint32() }, shift = it.field("shift").let { it.optional { it.int64() } }) } } })
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `view.grown` — binds the event name and the payload field values in one call.
    fun raiseViewGrown(layout: StaticPayloadEnumGrownViewModeEnum, marks: List<StaticPayloadEnumGrownMarkRecord>) {
        val scePayload = StaticPayloadEnumGrownViewGrownPayload(layout, marks)
        send(
            StaticPayloadEnumGrownEvent.View.Grown,
            EventMetadata(
                type = "external",
                typedPayload = scePayload,
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("layout" to com.sce.runtime.EventPayload.valueJson(scePayload.layout.declaredName), "marks" to com.sce.runtime.EventPayload.listJson(scePayload.marks) { com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("day" to com.sce.runtime.EventPayload.valueJson(it.day), "shift" to (it.shift?.let { com.sce.runtime.EventPayload.valueJson(it) } ?: com.sce.runtime.EventPayload.valueJson(null)))) })).json
            )
        )
    }


    override val initialState: StaticPayloadEnumGrownState = StaticPayloadEnumGrownState.Browsing

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
    override val documentInitialTargets: List<EntryTarget<StaticPayloadEnumGrownState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticPayloadEnumGrownState, HistoryId>> =
            listOf(StateTarget(StaticPayloadEnumGrownState.Browsing))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: browsing's transition 0, as the microstep reads it.
        val transitionBrowsingAt0 = EnabledTransition<StaticPayloadEnumGrownState, HistoryId>(
            StaticPayloadEnumGrownState.Browsing,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: browsing's transition 1, as the microstep reads it.
        val transitionBrowsingAt1 = EnabledTransition<StaticPayloadEnumGrownState, HistoryId>(
            StaticPayloadEnumGrownState.Browsing,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: browsing's transition 2, as the microstep reads it.
        val transitionBrowsingAt2 = EnabledTransition<StaticPayloadEnumGrownState, HistoryId>(
            StaticPayloadEnumGrownState.Browsing,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticPayloadEnumGrownState? = when (stateId) {
        "browsing" -> StaticPayloadEnumGrownState.Browsing
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticPayloadEnumGrownState): String = when (state) {
        is StaticPayloadEnumGrownState.Browsing -> "browsing"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticPayloadEnumGrownState): Int = when (state) {
        is StaticPayloadEnumGrownState.Browsing -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticPayloadEnumGrownEvent? = when (name) {
        "error.execution" -> StaticPayloadEnumGrownEvent.Error.Execution
        "view.grown" -> StaticPayloadEnumGrownEvent.View.Grown
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticPayloadEnumGrownEvent): String? = when (event) {
        is StaticPayloadEnumGrownEvent.Error.Execution -> "error.execution"
        is StaticPayloadEnumGrownEvent.View.Grown -> "view.grown"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticPayloadEnumGrownState, event: StaticPayloadEnumGrownEvent?): EnabledTransition<StaticPayloadEnumGrownState, HistoryId>? = when (state) {
        is StaticPayloadEnumGrownState.Browsing -> firstEnabledTransitionOfBrowsing(state, event)
    }


    // browsing's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfBrowsing(state: StaticPayloadEnumGrownState, event: StaticPayloadEnumGrownEvent?): EnabledTransition<StaticPayloadEnumGrownState, HistoryId>? = when {
        event is StaticPayloadEnumGrownEvent.View.Grown && pendingViewGrownPayload != null && (pendingViewGrownPayload!!.layout == StaticPayloadEnumGrownViewModeEnum.AGENDA_LIST) -> TransitionsPart0.transitionBrowsingAt0
        event is StaticPayloadEnumGrownEvent.View.Grown -> TransitionsPart0.transitionBrowsingAt1
        event is StaticPayloadEnumGrownEvent.Error.Execution -> TransitionsPart0.transitionBrowsingAt2
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_payload_enum_grown.scxml:18 :: _machine
    override fun onEntry(state: StaticPayloadEnumGrownState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticPayloadEnumGrownState.Browsing -> onEntryOfBrowsing()
        }
    }


    private fun onEntryOfBrowsing() {
                // SCE-MAP: static_payload_enum_grown.scxml:31 :: browsing :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_payload_enum_grown.scxml:18 :: _machine
    override fun onExit(state: StaticPayloadEnumGrownState) {
        when (state) {
            is StaticPayloadEnumGrownState.Browsing -> onExitOfBrowsing()
        }
    }


    private fun onExitOfBrowsing() {
                // SCE-MAP: static_payload_enum_grown.scxml:31 :: browsing :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_payload_enum_grown.scxml:18 :: _machine
    override fun executeTransitionContent(source: StaticPayloadEnumGrownState, transitionIndex: Int) {
        when (source) {
            is StaticPayloadEnumGrownState.Browsing -> transitionContentOfBrowsing(transitionIndex)
        }
    }


    // The content of `browsing`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfBrowsing(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfBrowsingAt0()
            1 -> transitionContentOfBrowsingAt1()
            2 -> transitionContentOfBrowsingAt2()
            else -> {}
        }
    }


    private fun transitionContentOfBrowsingAt0() {
                // SCE-MAP: static_payload_enum_grown.scxml:32 :: browsing :: _transition_0
                if (pendingViewGrownPayload == null) {
                    return
                }

            if (try { vAgendas = com.sce.forge.runtime.SceChecked.add(vAgendas, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadEnumGrownEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<assign location='agendas'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vLayout = pendingViewGrownPayload!!.layout

            vMarks = pendingViewGrownPayload!!.marks.size.toUInt()

            if (try { vShown = com.sce.forge.runtime.SceChecked.add(vShown, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadEnumGrownEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<assign location='shown'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfBrowsingAt1() {
                // SCE-MAP: static_payload_enum_grown.scxml:38 :: browsing :: _transition_1
                if (pendingViewGrownPayload == null) {
                    return
                }


            if (pendingViewGrownPayload!!.layout != vLayout) {

            if (try { vChanges = com.sce.forge.runtime.SceChecked.add(vChanges, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadEnumGrownEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<assign location='changes'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vLayout = pendingViewGrownPayload!!.layout
            }

            vMarks = pendingViewGrownPayload!!.marks.size.toUInt()

            if (try { vShown = com.sce.forge.runtime.SceChecked.add(vShown, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadEnumGrownEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<assign location='shown'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfBrowsingAt2() {
                // SCE-MAP: static_payload_enum_grown.scxml:46 :: browsing :: _transition_2

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadEnumGrownEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
