// SCE-GENERATED — DO NOT EDIT
// source-hash: 09d8fec62ddcc806d17c4e1da241b802dbe4f5317b3bd415a705b2ad99d82f3e

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_host_duty.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_host_duty.scxml:14 :: _machine

package com.sce.integration.static_host_duty

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticHostDutyState : State {
    data object Engaged : StaticHostDutyState
    data object Idle : StaticHostDutyState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticHostDutyEvent : Event {
    data object Engage : StaticHostDutyEvent
    data object Release : StaticHostDutyEvent
}
// --- State Machine (W3C SCXML) ---

// ── W3C SCXML G.7: `<sce:action>` host dispatch ───────────────────────
/**
 * W3C SCXML G.7: host operations dispatched by `<sce:action>`.
 * The host supplies the side effects while the statechart keeps each
 * operation symbolic. No runtime script engine is involved.
 */
interface StaticHostDutyActions {
    val realization: ActionRealization
    fun noteEngaged()
    fun startKeepalive()
    fun stopKeepalive()
}

/**
 * [StaticHostDutyActions] that performs nothing and records every call in order —
 * the host a test drives the machine with. Read [calls] after the machine
 * has run; each call is compared by value.
 */
class RecordingStaticHostDutyActions : StaticHostDutyActions {
    /** One recorded host call. */
    sealed interface Call {
        data object NoteEngaged : Call
        data object StartKeepalive : Call
        data object StopKeepalive : Call
    }

    private val recorded = mutableListOf<Call>()

    /** Every call so far, oldest first. */
    val calls: List<Call>
        get() = recorded.toList()

    /** Forget the calls recorded so far. */
    fun clear() {
        recorded.clear()
    }

    override val realization: ActionRealization = ActionRealization.SIMULATES

    override fun noteEngaged() {
        recorded += Call.NoteEngaged
    }
    override fun startKeepalive() {
        recorded += Call.StartKeepalive
    }
    override fun stopKeepalive() {
        recorded += Call.StopKeepalive
    }
}

class StaticHostDutyStateMachine(
    /**
     * W3C SCXML G.7: the host implementation every `<sce:action>` in this
     * document calls directly (`actions.<op>(…)`) instead of the script
     * engine.
     *
     * A constructor parameter rather than a setter, because the initial
     * state's `<onentry>` can already perform an act — a host installed
     * afterwards would arrive one act too late. It leads the parameter list
     * so it stays in the same position whether or not this machine also
     * takes a script engine.
     */
    private val actions: StaticHostDutyActions,
    /**
     * docs/adr/0038: `true` states that this machine is built for a host that
     * only simulates its duties (a recorder, a test double) on purpose. The
     * statement covers the children this machine builds with a host.
     */
    private val acceptSimulatedHost: Boolean = false,
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticHostDutyState, StaticHostDutyEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    init {
        require(actions.realization == ActionRealization.PERFORMS || acceptSimulatedHost) {
            "StaticHostDuty states a duty and its host declares that it only simulates the duties: " +
                "pass acceptSimulatedHost = true to run it on a recorder or a test double on purpose"
        }
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
        first = "5d52249374bda754d71b700a06673eba7505416f31282d135d0709089191effa",
        revised = "4c7995c5d033d96a315e1875fae1059c6dd6971fad03cd0a0a8e70b72b749d1a",
        format2First = "5d52249374bda754d71b700a06673eba7505416f31282d135d0709089191effa",
        format2Revised = "4c7995c5d033d96a315e1875fae1059c6dd6971fad03cd0a0a8e70b72b749d1a",
        records = listOf(
        ),
        enums = listOf(
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
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    override val initialState: StaticHostDutyState = StaticHostDutyState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticHostDutyState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticHostDutyState, HistoryId>> =
            listOf(StateTarget(StaticHostDutyState.Idle))

        // W3C SCXML 3.13: engaged's transition 0, as the microstep reads it.
        val transitionEngagedAt0 = EnabledTransition<StaticHostDutyState, HistoryId>(
            StaticHostDutyState.Engaged,
            listOf(StateTarget(StaticHostDutyState.Idle)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticHostDutyState, HistoryId>(
            StaticHostDutyState.Idle,
            listOf(StateTarget(StaticHostDutyState.Engaged)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticHostDutyState? = when (stateId) {
        "engaged" -> StaticHostDutyState.Engaged
        "idle" -> StaticHostDutyState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticHostDutyState): String = when (state) {
        is StaticHostDutyState.Engaged -> "engaged"
        is StaticHostDutyState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticHostDutyState): Int = when (state) {
        is StaticHostDutyState.Engaged -> 1
        is StaticHostDutyState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticHostDutyEvent? = when (name) {
        "engage" -> StaticHostDutyEvent.Engage
        "release" -> StaticHostDutyEvent.Release
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticHostDutyEvent): String? = when (event) {
        is StaticHostDutyEvent.Engage -> "engage"
        is StaticHostDutyEvent.Release -> "release"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticHostDutyState,
        event: StaticHostDutyEvent?
    ): EnabledTransition<StaticHostDutyState, HistoryId>? = when (state) {
        is StaticHostDutyState.Engaged -> when {
            event is StaticHostDutyEvent.Release -> transitionEngagedAt0
            else -> null
        }
        is StaticHostDutyState.Idle -> when {
            event is StaticHostDutyEvent.Engage -> transitionIdleAt0
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_host_duty.scxml:14 :: _machine
    override fun onEntry(state: StaticHostDutyState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticHostDutyState.Engaged -> {
                // SCE-MAP: static_host_duty.scxml:19 :: engaged :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            // W3C SCXML G.7: <sce:action name="start_keepalive">
            actions.startKeepalive()

            // W3C SCXML G.7: <sce:action name="note_engaged">
            actions.noteEngaged()
                }
            }
            is StaticHostDutyState.Idle -> {
                // SCE-MAP: static_host_duty.scxml:16 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_host_duty.scxml:14 :: _machine
    override fun onExit(state: StaticHostDutyState) {
        when (state) {
            is StaticHostDutyState.Engaged -> {
                // SCE-MAP: static_host_duty.scxml:19 :: engaged :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {

            // W3C SCXML G.7: <sce:action name="stop_keepalive">
            actions.stopKeepalive()
                }
            }
            is StaticHostDutyState.Idle -> {
                // SCE-MAP: static_host_duty.scxml:16 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_host_duty.scxml:14 :: _machine
    override fun executeTransitionContent(source: StaticHostDutyState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
