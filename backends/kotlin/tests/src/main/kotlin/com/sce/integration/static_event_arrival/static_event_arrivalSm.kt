// SCE-GENERATED — DO NOT EDIT
// source-hash: 48a87232ac281780a2bf087034fd503fdeb4a6a917bd2bcd8cecf2bac4f0fed7

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_event_arrival.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_event_arrival.scxml:22 :: _machine

package com.sce.integration.static_event_arrival

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticEventArrivalState : State {
    data object Listening : StaticEventArrivalState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticEventArrivalEvent : Event {
    sealed interface Request : StaticEventArrivalEvent {
        data object Self : Request
        data object Special : Request
    }
}
// --- State Machine (W3C SCXML) ---

class StaticEventArrivalStateMachine(
) : StateMachineEngine<StaticEventArrivalState, StaticEventArrivalEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `requests` datamodel variable, published (`sce:direction="out"`). */
    var requests: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `specials` datamodel variable, published (`sce:direction="out"`). */
    var specials: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var requests: UInt? = null
        var specials: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.requests?.let { requests = it }
        params.specials?.let { specials = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val requests: UInt,
        val specials: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticEventArrivalState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        requests = requests,
        specials = specials,
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
    val savedShape: String = "992cff77681778bf2a87af21f6ccd7e484e625e43a95c41373c35b22f2f582e1"

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
            "requests" to SavedValues.of(requests),
            "specials" to SavedValues.of(specials),
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
        val saved1 = SavedValues.uint32(saved.variable("requests"), "requests")
        val saved2 = SavedValues.uint32(saved.variable("specials"), "specials")
        requests = saved1
        specials = saved2
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticEventArrivalState = StaticEventArrivalState.Listening

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
    override val documentInitialTargets: List<EntryTarget<StaticEventArrivalState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticEventArrivalState, HistoryId>> =
            listOf(StateTarget(StaticEventArrivalState.Listening))

        // W3C SCXML 3.13: listening's transition 0, as the microstep reads it.
        val transitionListeningAt0 = EnabledTransition<StaticEventArrivalState, HistoryId>(
            StaticEventArrivalState.Listening,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: listening's transition 1, as the microstep reads it.
        val transitionListeningAt1 = EnabledTransition<StaticEventArrivalState, HistoryId>(
            StaticEventArrivalState.Listening,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticEventArrivalState? = when (stateId) {
        "listening" -> StaticEventArrivalState.Listening
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticEventArrivalState): String = when (state) {
        is StaticEventArrivalState.Listening -> "listening"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticEventArrivalState): Int = when (state) {
        is StaticEventArrivalState.Listening -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticEventArrivalEvent? = when (name) {
        "request" -> StaticEventArrivalEvent.Request.Self
        "request.special" -> StaticEventArrivalEvent.Request.Special
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticEventArrivalEvent): String? = when (event) {
        is StaticEventArrivalEvent.Request.Self -> "request"
        is StaticEventArrivalEvent.Request.Special -> "request.special"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticEventArrivalState,
        event: StaticEventArrivalEvent?
    ): EnabledTransition<StaticEventArrivalState, HistoryId>? = when (state) {
        is StaticEventArrivalState.Listening -> when {
            event is StaticEventArrivalEvent.Request.Special -> transitionListeningAt0
            (event is StaticEventArrivalEvent.Request || event is StaticEventArrivalEvent.Request.Special) -> transitionListeningAt1
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_event_arrival.scxml:22 :: _machine
    override fun onEntry(state: StaticEventArrivalState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticEventArrivalState.Listening -> {
                // SCE-MAP: static_event_arrival.scxml:28 :: listening :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_event_arrival.scxml:22 :: _machine
    override fun onExit(state: StaticEventArrivalState) {
        when (state) {
            is StaticEventArrivalState.Listening -> {
                // SCE-MAP: static_event_arrival.scxml:28 :: listening :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_event_arrival.scxml:22 :: _machine
    override fun executeTransitionContent(source: StaticEventArrivalState, transitionIndex: Int) {
        when (source) {
        is StaticEventArrivalState.Listening -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_event_arrival.scxml:29 :: listening :: _transition_0

            if (try { specials = com.sce.forge.runtime.SceChecked.add(specials, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_event_arrival.scxml:32 :: listening :: _transition_1

            if (try { requests = com.sce.forge.runtime.SceChecked.add(requests, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
