// SCE-GENERATED — DO NOT EDIT
// source-hash: dfdabf7a65fef0f1867c84d20330eb5ef5358227fc76a2ebaf8ec09099db52e5

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_enum.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_enum.scxml:12 :: _machine

package com.sce.integration.static_enum

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticEnumState : State {
    data object Browsing : StaticEnumState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticEnumEvent : Event {
    data object Agenda : StaticEnumEvent
    data object Back : StaticEnumEvent
    data object Swap : StaticEnumEvent
    data object Zoom : StaticEnumEvent
}
// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:ViewMode` datamodel value. */
enum class StaticEnumViewModeEnum(val declaredName: String) {
    MONTH("month"),
    WEEK("week"),
    DAY("day"),
    AGENDA_LIST("agenda_list");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticEnumViewModeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of ViewMode")
        }
    }
}
// --- State Machine (W3C SCXML) ---

class StaticEnumStateMachine(
) : StateMachineEngine<StaticEnumState, StaticEnumEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `layout` datamodel variable, published (`sce:direction="out"`). */
    var layout: StaticEnumViewModeEnum = StaticEnumViewModeEnum.MONTH
        private set
    /** W3C SCXML 5.2: the `previous` datamodel variable, the machine's own. */
    private var previous: StaticEnumViewModeEnum = StaticEnumViewModeEnum.MONTH
    /** W3C SCXML 5.2: the `changes` datamodel variable, published (`sce:direction="out"`). */
    var changes: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var changes: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.changes?.let { changes = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val layout: StaticEnumViewModeEnum,
        val changes: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticEnumState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        layout = layout,
        changes = changes,
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
    val savedShape: String = "520180e6f3f0c0a8ba6940f472c5d101a70edffe748e29d09589295b8c6ea8eb"

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
            "layout" to layout.toSaved(),
            "previous" to previous.toSaved(),
            "changes" to SavedValues.of(changes),
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
        val saved1 = StaticEnumViewModeEnum.fromSaved(saved.variable("layout"), "layout")
        val saved2 = StaticEnumViewModeEnum.fromSaved(saved.variable("previous"), "previous")
        val saved3 = SavedValues.uint32(saved.variable("changes"), "changes")
        layout = saved1
        previous = saved2
        changes = saved3
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticEnumState = StaticEnumState.Browsing

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
    override val documentInitialTargets: List<EntryTarget<StaticEnumState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticEnumState, HistoryId>> =
            listOf(StateTarget(StaticEnumState.Browsing))

        // W3C SCXML 3.13: browsing's transition 0, as the microstep reads it.
        val transitionBrowsingAt0 = EnabledTransition<StaticEnumState, HistoryId>(
            StaticEnumState.Browsing,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: browsing's transition 1, as the microstep reads it.
        val transitionBrowsingAt1 = EnabledTransition<StaticEnumState, HistoryId>(
            StaticEnumState.Browsing,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: browsing's transition 2, as the microstep reads it.
        val transitionBrowsingAt2 = EnabledTransition<StaticEnumState, HistoryId>(
            StaticEnumState.Browsing,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: browsing's transition 3, as the microstep reads it.
        val transitionBrowsingAt3 = EnabledTransition<StaticEnumState, HistoryId>(
            StaticEnumState.Browsing,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: browsing's transition 4, as the microstep reads it.
        val transitionBrowsingAt4 = EnabledTransition<StaticEnumState, HistoryId>(
            StaticEnumState.Browsing,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticEnumState? = when (stateId) {
        "browsing" -> StaticEnumState.Browsing
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticEnumState): String = when (state) {
        is StaticEnumState.Browsing -> "browsing"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticEnumState): Int = when (state) {
        is StaticEnumState.Browsing -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticEnumEvent? = when (name) {
        "agenda" -> StaticEnumEvent.Agenda
        "back" -> StaticEnumEvent.Back
        "swap" -> StaticEnumEvent.Swap
        "zoom" -> StaticEnumEvent.Zoom
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticEnumEvent): String? = when (event) {
        is StaticEnumEvent.Agenda -> "agenda"
        is StaticEnumEvent.Back -> "back"
        is StaticEnumEvent.Swap -> "swap"
        is StaticEnumEvent.Zoom -> "zoom"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticEnumState,
        event: StaticEnumEvent?
    ): EnabledTransition<StaticEnumState, HistoryId>? = when (state) {
        is StaticEnumState.Browsing -> when {
            event is StaticEnumEvent.Zoom && layout == StaticEnumViewModeEnum.MONTH -> transitionBrowsingAt0
            event is StaticEnumEvent.Zoom && layout == StaticEnumViewModeEnum.WEEK -> transitionBrowsingAt1
            event is StaticEnumEvent.Back && layout != previous -> transitionBrowsingAt2
            event is StaticEnumEvent.Swap -> transitionBrowsingAt3
            event is StaticEnumEvent.Agenda -> transitionBrowsingAt4
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_enum.scxml:12 :: _machine
    override fun onEntry(state: StaticEnumState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticEnumState.Browsing -> {
                // SCE-MAP: static_enum.scxml:20 :: browsing :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_enum.scxml:12 :: _machine
    override fun onExit(state: StaticEnumState) {
        when (state) {
            is StaticEnumState.Browsing -> {
                // SCE-MAP: static_enum.scxml:20 :: browsing :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_enum.scxml:12 :: _machine
    override fun executeTransitionContent(source: StaticEnumState, transitionIndex: Int) {
        when (source) {
        is StaticEnumState.Browsing -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_enum.scxml:22 :: browsing :: _transition_0

            previous = layout

            layout = StaticEnumViewModeEnum.DAY

            if (try { changes = com.sce.forge.runtime.SceChecked.add(changes, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_enum.scxml:27 :: browsing :: _transition_1

            previous = layout

            layout = StaticEnumViewModeEnum.DAY

            if (try { changes = com.sce.forge.runtime.SceChecked.add(changes, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_enum.scxml:33 :: browsing :: _transition_2

            layout = previous

            if (try { changes = com.sce.forge.runtime.SceChecked.add(changes, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_enum.scxml:38 :: browsing :: _transition_3

            previous = layout

            layout = if (layout == StaticEnumViewModeEnum.WEEK) StaticEnumViewModeEnum.MONTH else StaticEnumViewModeEnum.WEEK

            if (try { changes = com.sce.forge.runtime.SceChecked.add(changes, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_enum.scxml:43 :: browsing :: _transition_4

            previous = layout

            layout = StaticEnumViewModeEnum.AGENDA_LIST

            if (try { changes = com.sce.forge.runtime.SceChecked.add(changes, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }

            println("layout: " + layout)
            }
            else -> {}
        }
        }
    }
}
