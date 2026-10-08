// SCE-GENERATED — DO NOT EDIT
// source-hash: e3981fcd9cff1f8f15c89f3192467f522d2adc9208df092cb08ac7ed74e348c9

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_send_target.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_send_target.scxml:18 :: _machine

package com.sce.integration.static_send_target

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticSendTargetState : State {
    data object Idle : StaticSendTargetState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticSendTargetEvent : Event {
    sealed interface Aim : StaticSendTargetEvent {
        data object Internal : Aim
        data object Parent : Aim
        data object Stranger : Aim
    }
    sealed interface Error : StaticSendTargetEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Fire : StaticSendTargetEvent
    data object Landed : StaticSendTargetEvent
    data object Narrow : StaticSendTargetEvent
}
// --- State Machine (W3C SCXML) ---

class StaticSendTargetStateMachine(
) : StateMachineEngine<StaticSendTargetState, StaticSendTargetEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `route` datamodel variable, the machine's own. */
    private var route: String = "#_internal"
    /** W3C SCXML 5.2: the `landed` datamodel variable, published (`sce:direction="out"`). */
    var landed: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `refused` datamodel variable, published (`sce:direction="out"`). */
    var refused: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var route: String? = null
        var landed: UInt? = null
        var refused: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.route?.let { route = it }
        params.landed?.let { landed = it }
        params.refused?.let { refused = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val landed: UInt,
        val refused: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticSendTargetState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        landed = landed,
        refused = refused,
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
    val savedShape: String = "bc863388d4492ecbc92204f377e9e4bb03c09312cb692c262c1b41146eab182d"

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
            "route" to SavedValues.of(route),
            "landed" to SavedValues.of(landed),
            "refused" to SavedValues.of(refused),
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
        val saved1 = SavedValues.string(saved.variable("route"), "route", 24)
        val saved2 = SavedValues.uint32(saved.variable("landed"), "landed")
        val saved3 = SavedValues.uint32(saved.variable("refused"), "refused")
        route = saved1
        landed = saved2
        refused = saved3
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticSendTargetState = StaticSendTargetState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticSendTargetState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticSendTargetState, HistoryId>> =
            listOf(StateTarget(StaticSendTargetState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticSendTargetState, HistoryId>(
            StaticSendTargetState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticSendTargetState, HistoryId>(
            StaticSendTargetState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticSendTargetState, HistoryId>(
            StaticSendTargetState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticSendTargetState, HistoryId>(
            StaticSendTargetState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticSendTargetState, HistoryId>(
            StaticSendTargetState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<StaticSendTargetState, HistoryId>(
            StaticSendTargetState.Idle,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 6, as the microstep reads it.
        val transitionIdleAt6 = EnabledTransition<StaticSendTargetState, HistoryId>(
            StaticSendTargetState.Idle,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticSendTargetState? = when (stateId) {
        "idle" -> StaticSendTargetState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticSendTargetState): String = when (state) {
        is StaticSendTargetState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticSendTargetState): Int = when (state) {
        is StaticSendTargetState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticSendTargetEvent? = when (name) {
        "aim.internal" -> StaticSendTargetEvent.Aim.Internal
        "aim.parent" -> StaticSendTargetEvent.Aim.Parent
        "aim.stranger" -> StaticSendTargetEvent.Aim.Stranger
        "error.communication" -> StaticSendTargetEvent.Error.Communication
        "error.execution" -> StaticSendTargetEvent.Error.Execution
        "fire" -> StaticSendTargetEvent.Fire
        "landed" -> StaticSendTargetEvent.Landed
        "narrow" -> StaticSendTargetEvent.Narrow
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticSendTargetEvent): String? = when (event) {
        is StaticSendTargetEvent.Aim.Internal -> "aim.internal"
        is StaticSendTargetEvent.Aim.Parent -> "aim.parent"
        is StaticSendTargetEvent.Aim.Stranger -> "aim.stranger"
        is StaticSendTargetEvent.Error.Communication -> "error.communication"
        is StaticSendTargetEvent.Error.Execution -> "error.execution"
        is StaticSendTargetEvent.Fire -> "fire"
        is StaticSendTargetEvent.Landed -> "landed"
        is StaticSendTargetEvent.Narrow -> "narrow"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticSendTargetState,
        event: StaticSendTargetEvent?
    ): EnabledTransition<StaticSendTargetState, HistoryId>? = when (state) {
        is StaticSendTargetState.Idle -> when {
            event is StaticSendTargetEvent.Fire -> transitionIdleAt0
            event is StaticSendTargetEvent.Narrow -> transitionIdleAt1
            event is StaticSendTargetEvent.Aim.Internal -> transitionIdleAt2
            event is StaticSendTargetEvent.Aim.Parent -> transitionIdleAt3
            event is StaticSendTargetEvent.Aim.Stranger -> transitionIdleAt4
            event is StaticSendTargetEvent.Landed -> transitionIdleAt5
            event is StaticSendTargetEvent.Error.Communication -> transitionIdleAt6
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_send_target.scxml:18 :: _machine
    override fun onEntry(state: StaticSendTargetState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticSendTargetState.Idle -> {
                // SCE-MAP: static_send_target.scxml:25 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_send_target.scxml:18 :: _machine
    override fun onExit(state: StaticSendTargetState) {
        when (state) {
            is StaticSendTargetState.Idle -> {
                // SCE-MAP: static_send_target.scxml:25 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_send_target.scxml:18 :: _machine
    override fun executeTransitionContent(source: StaticSendTargetState, transitionIndex: Int) {
        when (source) {
        is StaticSendTargetState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_send_target.scxml:26 :: idle :: _transition_0


            if (run send@{
            // docs/adr/0005, decision 3: the target is a string computed from the
            // machine's fields now, and the send goes only by a route the document
            // declared as `sce:targets`. A value in none of them names no route this
            // send may take — an address nobody answers at, which is reported below as
            // error.communication (W3C SCXML C.1) — and an operation that fails cannot
            // be evaluated, the argument error.
            val _computed: String = route
            val _rt: String = if (_computed == "#_internal" || _computed == "#_parent") _computed else ""
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(StaticSendTargetEvent.Error.Communication, "<send> targetexpr evaluated to nothing, or to a route the send does not declare, so there is no target to reach", "__send_0")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
StaticSendTargetEvent.Landed,
"landed",
                sendData,
0L,
                "__send_0",
                StaticSendTargetEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(StaticSendTargetEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_0")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(StaticSendTargetEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_0")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: static_send_target.scxml:29 :: idle :: _transition_1


            if (run send@{
            // docs/adr/0005, decision 3: the target is a string computed from the
            // machine's fields now, and the send goes only by a route the document
            // declared as `sce:targets`. A value in none of them names no route this
            // send may take — an address nobody answers at, which is reported below as
            // error.communication (W3C SCXML C.1) — and an operation that fails cannot
            // be evaluated, the argument error.
            val _computed: String = route
            val _rt: String = if (_computed == "#_parent") _computed else ""
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(StaticSendTargetEvent.Error.Communication, "<send> targetexpr evaluated to nothing, or to a route the send does not declare, so there is no target to reach", "__send_1")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
StaticSendTargetEvent.Landed,
"landed",
                sendData,
0L,
                "__send_1",
                StaticSendTargetEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(StaticSendTargetEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_1")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(StaticSendTargetEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_1")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            2 -> {
                // SCE-MAP: static_send_target.scxml:32 :: idle :: _transition_2

            if (try { route = com.sce.forge.runtime.SceChecked.bounded("#_internal", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendTargetEvent.Error.Execution, "<assign location='route'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_send_target.scxml:35 :: idle :: _transition_3

            if (try { route = com.sce.forge.runtime.SceChecked.bounded("#_parent", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendTargetEvent.Error.Execution, "<assign location='route'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_send_target.scxml:38 :: idle :: _transition_4

            if (try { route = com.sce.forge.runtime.SceChecked.bounded("#_scxml_stranger", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendTargetEvent.Error.Execution, "<assign location='route'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            5 -> {
                // SCE-MAP: static_send_target.scxml:41 :: idle :: _transition_5

            if (try { landed = com.sce.forge.runtime.SceChecked.add(landed, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendTargetEvent.Error.Execution, "<assign location='landed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            6 -> {
                // SCE-MAP: static_send_target.scxml:44 :: idle :: _transition_6

            if (try { refused = com.sce.forge.runtime.SceChecked.add(refused, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendTargetEvent.Error.Execution, "<assign location='refused'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
