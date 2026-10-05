// SCE-GENERATED — DO NOT EDIT
// source-hash: 1fe728b521f52adf866bb0e0fb56e6c9d8272ffd7fa321459c53679f859508c9

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_donedata_content.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_donedata_content.scxml:12 :: _machine

package com.sce.integration.static_donedata_content

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticDonedataContentState : State {
    data object Counting : StaticDonedataContentState
    data object Done : StaticDonedataContentState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticDonedataContentEvent : Event {
    sealed interface Error : StaticDonedataContentEvent {
        data object Execution : Error
    }
    data object Finish : StaticDonedataContentEvent
    data object Tick : StaticDonedataContentEvent
}
// --- State Machine (W3C SCXML) ---

class StaticDonedataContentStateMachine(
) : StateMachineEngine<StaticDonedataContentState, StaticDonedataContentEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `count` datamodel variable, published (`sce:direction="out"`). */
    var count: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var count: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.count?.let { count = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val count: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticDonedataContentState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        count = count,
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
    val savedShape: String = "1338ef746484e3cad4377294a89e6faf1dd6df3a9e93d520359ef96699a73532"

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
            "count" to SavedValues.of(count),
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
        val saved1 = SavedValues.uint32(saved.variable("count"), "count")
        count = saved1
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticDonedataContentState = StaticDonedataContentState.Counting

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
    override fun isFinalState(state: StaticDonedataContentState): Boolean = when (state) {
        is StaticDonedataContentState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticDonedataContentState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticDonedataContentState, HistoryId>> =
            listOf(StateTarget(StaticDonedataContentState.Counting))

        // W3C SCXML 3.13: counting's transition 0, as the microstep reads it.
        val transitionCountingAt0 = EnabledTransition<StaticDonedataContentState, HistoryId>(
            StaticDonedataContentState.Counting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: counting's transition 1, as the microstep reads it.
        val transitionCountingAt1 = EnabledTransition<StaticDonedataContentState, HistoryId>(
            StaticDonedataContentState.Counting,
            listOf(StateTarget(StaticDonedataContentState.Done)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticDonedataContentState? = when (stateId) {
        "counting" -> StaticDonedataContentState.Counting
        "done" -> StaticDonedataContentState.Done
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticDonedataContentState): String = when (state) {
        is StaticDonedataContentState.Counting -> "counting"
        is StaticDonedataContentState.Done -> "done"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticDonedataContentState): Int = when (state) {
        is StaticDonedataContentState.Counting -> 0
        is StaticDonedataContentState.Done -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticDonedataContentEvent? = when (name) {
        "error.execution" -> StaticDonedataContentEvent.Error.Execution
        "finish" -> StaticDonedataContentEvent.Finish
        "tick" -> StaticDonedataContentEvent.Tick
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticDonedataContentEvent): String? = when (event) {
        is StaticDonedataContentEvent.Error.Execution -> "error.execution"
        is StaticDonedataContentEvent.Finish -> "finish"
        is StaticDonedataContentEvent.Tick -> "tick"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticDonedataContentState,
        event: StaticDonedataContentEvent?
    ): EnabledTransition<StaticDonedataContentState, HistoryId>? = when (state) {
        is StaticDonedataContentState.Counting -> when {
            event is StaticDonedataContentEvent.Tick -> transitionCountingAt0
            event is StaticDonedataContentEvent.Finish -> transitionCountingAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_donedata_content.scxml:12 :: _machine
    override fun onEntry(state: StaticDonedataContentState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticDonedataContentState.Counting -> {
                // SCE-MAP: static_donedata_content.scxml:17 :: counting :: _state_body
            }
            is StaticDonedataContentState.Done -> {
                // SCE-MAP: static_donedata_content.scxml:23 :: done :: _state_body
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    var doneEventData = ""
                    // W3C SCXML 5.5: inline text is the content value, finished at build time.
                    doneEventData = "\"42\""
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
    // SCE-MAP: static_donedata_content.scxml:12 :: _machine
    override fun onExit(state: StaticDonedataContentState) {
        when (state) {
            is StaticDonedataContentState.Counting -> {
                // SCE-MAP: static_donedata_content.scxml:17 :: counting :: _state_body
            }
            is StaticDonedataContentState.Done -> {
                // SCE-MAP: static_donedata_content.scxml:23 :: done :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_donedata_content.scxml:12 :: _machine
    override fun executeTransitionContent(source: StaticDonedataContentState, transitionIndex: Int) {
        when (source) {
        is StaticDonedataContentState.Counting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_donedata_content.scxml:18 :: counting :: _transition_0

            if (try { count = com.sce.forge.runtime.SceChecked.add(count, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticDonedataContentEvent.Error.Execution, "<assign location='count'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
