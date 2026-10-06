// SCE-GENERATED — DO NOT EDIT
// source-hash: 64fb223807f4c558ee24bbcd3b67b8ca6498504cd273de737c75cdc2b6d599ee

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_donedata_record.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_donedata_record.scxml:13 :: _machine

package com.sce.integration.static_donedata_record

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticDonedataRecordState : State {
    data object Counting : StaticDonedataRecordState
    data object Done : StaticDonedataRecordState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticDonedataRecordEvent : Event {
    data object Bump : StaticDonedataRecordEvent
    sealed interface Error : StaticDonedataRecordEvent {
        data object Execution : Error
    }
    data object Finish : StaticDonedataRecordEvent
}
// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:ViewMode` datamodel value. */
enum class StaticDonedataRecordViewModeEnum(val declaredName: String) {
    MONTH("month"),
    WEEK("week"),
    DAY("day"),
    AGENDA_LIST("agenda_list");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticDonedataRecordViewModeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of ViewMode")
        }
    }
}
/** SCE Accepted Subset §2.15: a `record:View` datamodel value. */
data class StaticDonedataRecordViewRecord(val layout: StaticDonedataRecordViewModeEnum, val zoom: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("layout" to layout.toSaved(), "zoom" to SavedValues.of(zoom))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticDonedataRecordViewRecord = StaticDonedataRecordViewRecord(layout = StaticDonedataRecordViewModeEnum.fromSaved(SavedValues.field(value, what, "layout"), "$what.layout"), zoom = SavedValues.uint8(SavedValues.field(value, what, "zoom"), "$what.zoom"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticDonedataRecordStateMachine(
) : StateMachineEngine<StaticDonedataRecordState, StaticDonedataRecordEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `shown` datamodel variable, published (`sce:direction="out"`). */
    var shown: StaticDonedataRecordViewRecord = StaticDonedataRecordViewRecord(layout = StaticDonedataRecordViewModeEnum.DAY, zoom = 2.toUByte())
        private set

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val shown: StaticDonedataRecordViewRecord,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticDonedataRecordState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        shown = shown,
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
    val savedShape: String = "fd144745a4a7d0c2313c4f28ec7184ffb7a698b5ed3361febe933a4c30ecea2b"

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
            "shown" to shown.toSaved(),
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
        val saved1 = StaticDonedataRecordViewRecord.fromSaved(saved.variable("shown"), "shown")
        shown = saved1
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticDonedataRecordState = StaticDonedataRecordState.Counting

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
    override fun isFinalState(state: StaticDonedataRecordState): Boolean = when (state) {
        is StaticDonedataRecordState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticDonedataRecordState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticDonedataRecordState, HistoryId>> =
            listOf(StateTarget(StaticDonedataRecordState.Counting))

        // W3C SCXML 3.13: counting's transition 0, as the microstep reads it.
        val transitionCountingAt0 = EnabledTransition<StaticDonedataRecordState, HistoryId>(
            StaticDonedataRecordState.Counting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: counting's transition 1, as the microstep reads it.
        val transitionCountingAt1 = EnabledTransition<StaticDonedataRecordState, HistoryId>(
            StaticDonedataRecordState.Counting,
            listOf(StateTarget(StaticDonedataRecordState.Done)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticDonedataRecordState? = when (stateId) {
        "counting" -> StaticDonedataRecordState.Counting
        "done" -> StaticDonedataRecordState.Done
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticDonedataRecordState): String = when (state) {
        is StaticDonedataRecordState.Counting -> "counting"
        is StaticDonedataRecordState.Done -> "done"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticDonedataRecordState): Int = when (state) {
        is StaticDonedataRecordState.Counting -> 0
        is StaticDonedataRecordState.Done -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticDonedataRecordEvent? = when (name) {
        "bump" -> StaticDonedataRecordEvent.Bump
        "error.execution" -> StaticDonedataRecordEvent.Error.Execution
        "finish" -> StaticDonedataRecordEvent.Finish
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticDonedataRecordEvent): String? = when (event) {
        is StaticDonedataRecordEvent.Bump -> "bump"
        is StaticDonedataRecordEvent.Error.Execution -> "error.execution"
        is StaticDonedataRecordEvent.Finish -> "finish"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticDonedataRecordState,
        event: StaticDonedataRecordEvent?
    ): EnabledTransition<StaticDonedataRecordState, HistoryId>? = when (state) {
        is StaticDonedataRecordState.Counting -> when {
            event is StaticDonedataRecordEvent.Bump -> transitionCountingAt0
            event is StaticDonedataRecordEvent.Finish -> transitionCountingAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_donedata_record.scxml:13 :: _machine
    override fun onEntry(state: StaticDonedataRecordState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticDonedataRecordState.Counting -> {
                // SCE-MAP: static_donedata_record.scxml:23 :: counting :: _state_body
            }
            is StaticDonedataRecordState.Done -> {
                // SCE-MAP: static_donedata_record.scxml:30 :: done :: _state_body
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    var doneEventData = ""
                    // W3C SCXML 5.5: Evaluate <param> elements (C++ DoneDataHelper::evaluateParams pattern)
                    val doneParams = mutableMapOf<String, Any?>()
                    doneParams["layout"] = (shown.layout).declaredName
                    doneParams["zoom"] = (shown.zoom).toLong()
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
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_donedata_record.scxml:13 :: _machine
    override fun onExit(state: StaticDonedataRecordState) {
        when (state) {
            is StaticDonedataRecordState.Counting -> {
                // SCE-MAP: static_donedata_record.scxml:23 :: counting :: _state_body
            }
            is StaticDonedataRecordState.Done -> {
                // SCE-MAP: static_donedata_record.scxml:30 :: done :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_donedata_record.scxml:13 :: _machine
    override fun executeTransitionContent(source: StaticDonedataRecordState, transitionIndex: Int) {
        when (source) {
        is StaticDonedataRecordState.Counting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_donedata_record.scxml:24 :: counting :: _transition_0

            if (try { shown = shown.copy(zoom = com.sce.forge.runtime.SceChecked.add(shown.zoom, 1.toUByte())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticDonedataRecordEvent.Error.Execution, "<assign location='shown.zoom'>: an integer operation overflowed or failed"); true }) {
                return
            }

            shown = shown.copy(layout = StaticDonedataRecordViewModeEnum.AGENDA_LIST)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
