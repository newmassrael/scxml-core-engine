// SCE-GENERATED — DO NOT EDIT
// source-hash: 596ac4b1afa5b66720218d6a44ed9e9093342daa23116ea6b40585ea85ed56ab

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_record_enum.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_record_enum.scxml:13 :: _machine

package com.sce.integration.static_record_enum

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticRecordEnumState : State {
    data object Viewing : StaticRecordEnumState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticRecordEnumEvent : Event {
    data object Agenda : StaticRecordEnumEvent
    data object Count : StaticRecordEnumEvent
    data object Forget : StaticRecordEnumEvent
    data object Recall : StaticRecordEnumEvent
    data object Remember : StaticRecordEnumEvent
    data object Toggle : StaticRecordEnumEvent
    data object Zoom : StaticRecordEnumEvent
}
// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:ViewMode` datamodel value. */
enum class StaticRecordEnumViewModeEnum(val declaredName: String) {
    MONTH("month"),
    WEEK("week"),
    DAY("day"),
    AGENDA_LIST("agenda_list");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticRecordEnumViewModeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of ViewMode")
        }
    }
}
/** SCE Accepted Subset §2.15: a `record:View` datamodel value. */
data class StaticRecordEnumViewRecord(val layout: StaticRecordEnumViewModeEnum, val zoom: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("layout" to layout.toSaved(), "zoom" to SavedValues.of(zoom))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticRecordEnumViewRecord = StaticRecordEnumViewRecord(layout = StaticRecordEnumViewModeEnum.fromSaved(SavedValues.field(value, what, "layout"), "$what.layout"), zoom = SavedValues.uint8(SavedValues.field(value, what, "zoom"), "$what.zoom"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticRecordEnumStateMachine(
) : StateMachineEngine<StaticRecordEnumState, StaticRecordEnumEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `shown` datamodel variable, published (`sce:direction="out"`). */
    var shown: StaticRecordEnumViewRecord = StaticRecordEnumViewRecord(layout = StaticRecordEnumViewModeEnum.MONTH, zoom = 1.toUByte())
        private set
    /** W3C SCXML 5.2: the `seen` datamodel variable, published (`sce:direction="out"`). */
    var seen: List<StaticRecordEnumViewRecord> = emptyList()
        private set
    /** W3C SCXML 5.2: the `weeks` datamodel variable, published (`sce:direction="out"`). */
    var weeks: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `flips` datamodel variable, published (`sce:direction="out"`). */
    var flips: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var weeks: UInt? = null
        var flips: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.weeks?.let { weeks = it }
        params.flips?.let { flips = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val shown: StaticRecordEnumViewRecord,
        val seen: List<StaticRecordEnumViewRecord>,
        val weeks: UInt,
        val flips: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticRecordEnumState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        shown = shown,
        seen = seen,
        weeks = weeks,
        flips = flips,
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
    val savedShape: String = "941b4cd3ef3c635175f4eebd0cda9103f5ee12a9d569303a82d4aa1c62bc7475"

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
            "seen" to SavedValues.list(seen) { it.toSaved() },
            "weeks" to SavedValues.of(weeks),
            "flips" to SavedValues.of(flips),
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
        val saved1 = StaticRecordEnumViewRecord.fromSaved(saved.variable("shown"), "shown")
        val saved2 = SavedValues.list(saved.variable("seen"), "seen", 4) { e, w -> StaticRecordEnumViewRecord.fromSaved(e, w) }
        val saved3 = SavedValues.uint32(saved.variable("weeks"), "weeks")
        val saved4 = SavedValues.uint32(saved.variable("flips"), "flips")
        shown = saved1
        seen = saved2
        weeks = saved3
        flips = saved4
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticRecordEnumState = StaticRecordEnumState.Viewing

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
    override val documentInitialTargets: List<EntryTarget<StaticRecordEnumState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticRecordEnumState, HistoryId>> =
            listOf(StateTarget(StaticRecordEnumState.Viewing))

        // W3C SCXML 3.13: viewing's transition 0, as the microstep reads it.
        val transitionViewingAt0 = EnabledTransition<StaticRecordEnumState, HistoryId>(
            StaticRecordEnumState.Viewing,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 1, as the microstep reads it.
        val transitionViewingAt1 = EnabledTransition<StaticRecordEnumState, HistoryId>(
            StaticRecordEnumState.Viewing,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 2, as the microstep reads it.
        val transitionViewingAt2 = EnabledTransition<StaticRecordEnumState, HistoryId>(
            StaticRecordEnumState.Viewing,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 3, as the microstep reads it.
        val transitionViewingAt3 = EnabledTransition<StaticRecordEnumState, HistoryId>(
            StaticRecordEnumState.Viewing,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 4, as the microstep reads it.
        val transitionViewingAt4 = EnabledTransition<StaticRecordEnumState, HistoryId>(
            StaticRecordEnumState.Viewing,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 5, as the microstep reads it.
        val transitionViewingAt5 = EnabledTransition<StaticRecordEnumState, HistoryId>(
            StaticRecordEnumState.Viewing,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 6, as the microstep reads it.
        val transitionViewingAt6 = EnabledTransition<StaticRecordEnumState, HistoryId>(
            StaticRecordEnumState.Viewing,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 7, as the microstep reads it.
        val transitionViewingAt7 = EnabledTransition<StaticRecordEnumState, HistoryId>(
            StaticRecordEnumState.Viewing,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticRecordEnumState? = when (stateId) {
        "viewing" -> StaticRecordEnumState.Viewing
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticRecordEnumState): String = when (state) {
        is StaticRecordEnumState.Viewing -> "viewing"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticRecordEnumState): Int = when (state) {
        is StaticRecordEnumState.Viewing -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticRecordEnumEvent? = when (name) {
        "agenda" -> StaticRecordEnumEvent.Agenda
        "count" -> StaticRecordEnumEvent.Count
        "forget" -> StaticRecordEnumEvent.Forget
        "recall" -> StaticRecordEnumEvent.Recall
        "remember" -> StaticRecordEnumEvent.Remember
        "toggle" -> StaticRecordEnumEvent.Toggle
        "zoom" -> StaticRecordEnumEvent.Zoom
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticRecordEnumEvent): String? = when (event) {
        is StaticRecordEnumEvent.Agenda -> "agenda"
        is StaticRecordEnumEvent.Count -> "count"
        is StaticRecordEnumEvent.Forget -> "forget"
        is StaticRecordEnumEvent.Recall -> "recall"
        is StaticRecordEnumEvent.Remember -> "remember"
        is StaticRecordEnumEvent.Toggle -> "toggle"
        is StaticRecordEnumEvent.Zoom -> "zoom"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticRecordEnumState,
        event: StaticRecordEnumEvent?
    ): EnabledTransition<StaticRecordEnumState, HistoryId>? = when (state) {
        is StaticRecordEnumState.Viewing -> when {
            event is StaticRecordEnumEvent.Toggle && shown.layout == StaticRecordEnumViewModeEnum.MONTH -> transitionViewingAt0
            event is StaticRecordEnumEvent.Toggle && shown.layout == StaticRecordEnumViewModeEnum.WEEK -> transitionViewingAt1
            event is StaticRecordEnumEvent.Agenda -> transitionViewingAt2
            event is StaticRecordEnumEvent.Zoom -> transitionViewingAt3
            event is StaticRecordEnumEvent.Remember -> transitionViewingAt4
            event is StaticRecordEnumEvent.Count -> transitionViewingAt5
            event is StaticRecordEnumEvent.Recall -> transitionViewingAt6
            event is StaticRecordEnumEvent.Forget -> transitionViewingAt7
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_record_enum.scxml:13 :: _machine
    override fun onEntry(state: StaticRecordEnumState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticRecordEnumState.Viewing -> {
                // SCE-MAP: static_record_enum.scxml:26 :: viewing :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_record_enum.scxml:13 :: _machine
    override fun onExit(state: StaticRecordEnumState) {
        when (state) {
            is StaticRecordEnumState.Viewing -> {
                // SCE-MAP: static_record_enum.scxml:26 :: viewing :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_record_enum.scxml:13 :: _machine
    override fun executeTransitionContent(source: StaticRecordEnumState, transitionIndex: Int) {
        when (source) {
        is StaticRecordEnumState.Viewing -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_record_enum.scxml:29 :: viewing :: _transition_0

            shown = shown.copy(layout = StaticRecordEnumViewModeEnum.WEEK)

            if (try { flips = com.sce.forge.runtime.SceChecked.add(flips, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_record_enum.scxml:33 :: viewing :: _transition_1

            shown = shown.copy(layout = StaticRecordEnumViewModeEnum.MONTH)

            if (try { flips = com.sce.forge.runtime.SceChecked.add(flips, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_record_enum.scxml:37 :: viewing :: _transition_2

            shown = shown.copy(layout = StaticRecordEnumViewModeEnum.AGENDA_LIST)
            }
            3 -> {
                // SCE-MAP: static_record_enum.scxml:40 :: viewing :: _transition_3

            if (try { shown = shown.copy(zoom = com.sce.forge.runtime.SceChecked.add(shown.zoom, 1.toUByte())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_record_enum.scxml:44 :: viewing :: _transition_4

            if (if (seen.size < 4) { seen = seen + (shown); false } else { true }) {
                return
            }
            }
            5 -> {
                // SCE-MAP: static_record_enum.scxml:48 :: viewing :: _transition_5

            weeks = 0.toUInt()


            for (v in seen) {


            if (v.layout == StaticRecordEnumViewModeEnum.WEEK) {

            if (try { weeks = com.sce.forge.runtime.SceChecked.add(weeks, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            }
            }
            6 -> {
                // SCE-MAP: static_record_enum.scxml:57 :: viewing :: _transition_6


            for (v in seen) {

            shown = v
            }
            }
            7 -> {
                // SCE-MAP: static_record_enum.scxml:62 :: viewing :: _transition_7

            seen = emptyList()
            }
            else -> {}
        }
        }
    }
}
