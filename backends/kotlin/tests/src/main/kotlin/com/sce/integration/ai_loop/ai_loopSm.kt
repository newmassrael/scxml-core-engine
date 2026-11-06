// SCE-GENERATED — DO NOT EDIT
// source-hash: 321b42acfe8cb266c51aff87d805eb471548c8d5250d5f0a5214385ef864d6e9

// GENERATED CODE — DO NOT EDIT
// Source: examples/ai_loop/ai_loop.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: ai_loop.scxml:155 :: _machine

package com.sce.integration.ai_loop

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AiLoopState : State {
    data object Abandoned : AiLoopState
    data object Alive : AiLoopState
    data object Blocked : AiLoopState
    data object Budget : AiLoopState
    data object Cancelled : AiLoopState
    data object Closing : AiLoopState
    data object Converged : AiLoopState
    data object Drive : AiLoopState
    data object Exhausted : AiLoopState
    data object Failed : AiLoopState
    data object Judging : AiLoopState
    data object Paused : AiLoopState
    data object Priming : AiLoopState
    data object Rebuilding : AiLoopState
    data object Reflecting : AiLoopState
    data object Reported : AiLoopState
    data object Restarting : AiLoopState
    data object Run : AiLoopState
    data object Running : AiLoopState
    data object Screening : AiLoopState
    data object Spent : AiLoopState
    data object Stuck : AiLoopState
    data object Watch : AiLoopState
    data object Within : AiLoopState
    data object Working : AiLoopState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AiLoopEvent : Event {
    data object Cancel : AiLoopEvent
    sealed interface Done : AiLoopEvent {
        sealed interface State : Done {
            data object Drive : State
            data object Run : State
            data object Running : State
        }
    }
    sealed interface Error : AiLoopEvent {
        data object Execution : Error
    }
    data object Fail : AiLoopEvent
    data object Hold : AiLoopEvent
    sealed interface Judge : AiLoopEvent {
        data object Self : Judge
        data object Begin : Judge
    }
    sealed interface Notify : AiLoopEvent {
        data object Human : Notify
    }
    sealed interface Prompt : AiLoopEvent {
        data object End : Prompt
        data object Sent : Prompt
        data object Start : Prompt
        data object Turn : Prompt
    }
    sealed interface Reflect : AiLoopEvent {
        data object Applied : Reflect
        data object Begin : Reflect
        data object None : Reflect
    }
    data object Resume : AiLoopEvent
    sealed interface Run : AiLoopEvent {
        data object Blocked : Run
        data object Converged : Run
        data object Exhausted : Run
    }
    sealed interface Screen : AiLoopEvent {
        data object Begin : Screen
        data object Matched : Screen
        data object None : Screen
    }
    sealed interface Session : AiLoopEvent {
        data object Lost : Session
        data object Ready : Session
        data object Replace : Session
    }
    sealed interface Turn : AiLoopEvent {
        data object Blocked : Turn
        data object Done : Turn
        data object Interrupted : Turn
    }
    data object Unattended : AiLoopEvent
}
// --- State Machine (W3C SCXML) ---

class AiLoopStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<AiLoopState, AiLoopEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `north_star` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `north_star` was assigned a value of another type, or the engine refused.
     */
    fun northStar(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "north_star")

    /**
     * §scxml-5.3: what the `milestone` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `milestone` was assigned a value of another type, or the engine refused.
     */
    fun milestone(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "milestone")

    /**
     * §scxml-5.3: what the `reference` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `reference` was assigned a value of another type, or the engine refused.
     */
    fun reference(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "reference")

    /**
     * §scxml-5.3: what the `start_prompt` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `start_prompt` was assigned a value of another type, or the engine refused.
     */
    fun startPrompt(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "start_prompt")

    /**
     * §scxml-5.3: what the `turn_prompt` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `turn_prompt` was assigned a value of another type, or the engine refused.
     */
    fun turnPrompt(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "turn_prompt")

    /**
     * §scxml-5.3: what the `end_prompt` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `end_prompt` was assigned a value of another type, or the engine refused.
     */
    fun endPrompt(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "end_prompt")

    /**
     * §scxml-5.3: what the `done_marker` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `done_marker` was assigned a value of another type, or the engine refused.
     */
    fun doneMarker(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "done_marker")

    /**
     * §scxml-5.3: what the `screen_rules` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `screen_rules` was assigned a value of another type, or the engine refused.
     *
     * The value as JSON text, serialised by the engine's own `JSON.stringify`
     * (§scxml-B-2) so the key order is the document's.
     */
    fun screenRules(): String? =
        com.sce.runtime.DatamodelRead.readJson(scriptEngine, scriptSessionId, "screen_rules")

    /**
     * §scxml-5.3: what the `screen_permissions` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `screen_permissions` was assigned a value of another type, or the engine refused.
     */
    fun screenPermissions(): Boolean? =
        com.sce.runtime.DatamodelRead.readBool(scriptEngine, scriptSessionId, "screen_permissions")

    /**
     * §scxml-5.3: what the `max_turns` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `max_turns` was assigned a value of another type, or the engine refused.
     */
    fun maxTurns(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "max_turns")

    /**
     * §scxml-5.3: what the `reflect_every` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `reflect_every` was assigned a value of another type, or the engine refused.
     */
    fun reflectEvery(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "reflect_every")

    /**
     * §scxml-5.3: what the `max_restarts` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `max_restarts` was assigned a value of another type, or the engine refused.
     */
    fun maxRestarts(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "max_restarts")

    /**
     * §scxml-5.3: what the `turns` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `turns` was assigned a value of another type, or the engine refused.
     */
    fun turns(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "turns")

    /**
     * §scxml-5.3: what the `turns_since_reflect` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `turns_since_reflect` was assigned a value of another type, or the engine refused.
     */
    fun turnsSinceReflect(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "turns_since_reflect")

    /**
     * §scxml-5.3: what the `screened` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `screened` was assigned a value of another type, or the engine refused.
     */
    fun screened(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "screened")

    /**
     * §scxml-5.3: what the `restarts` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `restarts` was assigned a value of another type, or the engine refused.
     */
    fun restarts(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "restarts")

    override val initialState: AiLoopState = AiLoopState.Priming

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: AiLoopState): AiLoopState? = when (state) {
        is AiLoopState.Abandoned -> AiLoopState.Drive
        is AiLoopState.Alive -> AiLoopState.Watch
        is AiLoopState.Budget -> AiLoopState.Run
        is AiLoopState.Closing -> AiLoopState.Running
        is AiLoopState.Drive -> AiLoopState.Run
        is AiLoopState.Judging -> AiLoopState.Running
        is AiLoopState.Paused -> AiLoopState.Drive
        is AiLoopState.Priming -> AiLoopState.Running
        is AiLoopState.Rebuilding -> AiLoopState.Watch
        is AiLoopState.Reflecting -> AiLoopState.Running
        is AiLoopState.Reported -> AiLoopState.Running
        is AiLoopState.Restarting -> AiLoopState.Running
        is AiLoopState.Running -> AiLoopState.Drive
        is AiLoopState.Screening -> AiLoopState.Running
        is AiLoopState.Spent -> AiLoopState.Budget
        is AiLoopState.Stuck -> AiLoopState.Running
        is AiLoopState.Watch -> AiLoopState.Run
        is AiLoopState.Within -> AiLoopState.Budget
        is AiLoopState.Working -> AiLoopState.Running
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: AiLoopState): Boolean = when (state) {
        is AiLoopState.Budget -> true
        is AiLoopState.Drive -> true
        is AiLoopState.Running -> true
        is AiLoopState.Watch -> true
        else -> false
    }


    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: AiLoopState): Boolean = when (state) {
        is AiLoopState.Run -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: AiLoopState): Boolean = when (state) {
        is AiLoopState.Abandoned -> true
        is AiLoopState.Blocked -> true
        is AiLoopState.Cancelled -> true
        is AiLoopState.Converged -> true
        is AiLoopState.Exhausted -> true
        is AiLoopState.Failed -> true
        is AiLoopState.Reported -> true
        is AiLoopState.Stuck -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: AiLoopState): List<AiLoopState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: AiLoopState): List<EntryTarget<AiLoopState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AiLoopState, HistoryId>>
        get() = documentInitialTargetList

    // W3C SCXML 3.10: the state a <history> is declared in.
    override fun historyParentOf(history: HistoryId): AiLoopState = historyParents.getValue(history)

    // W3C SCXML 3.10.2: a <history>'s default transition target, as written.
    override fun historyDefaultTargetsOf(history: HistoryId): List<EntryTarget<AiLoopState, HistoryId>> =
        historyDefaultTargets.getValue(history)

    // W3C SCXML 3.10: a state's <history> children, each with whether it is
    // deep — what the runtime records as the state is exited.
    override fun historiesOf(state: AiLoopState): List<Pair<HistoryId, Boolean>> =
        historiesByParent[state] ?: emptyList()

    private companion object {
        /** W3C SCXML 3.10: the `where` <history> (shallow). */
        val historyWhere: HistoryId get() = HistoryId(0)

        val childStates: Map<AiLoopState, List<AiLoopState>> = LinkedHashMap<AiLoopState, List<AiLoopState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<AiLoopState, List<AiLoopState>>) {
            table[AiLoopState.Budget] = listOf(AiLoopState.Within, AiLoopState.Spent)
            table[AiLoopState.Drive] = listOf(AiLoopState.Running, AiLoopState.Paused, AiLoopState.Abandoned)
            table[AiLoopState.Run] = listOf(AiLoopState.Drive, AiLoopState.Watch, AiLoopState.Budget)
            table[AiLoopState.Running] = listOf(AiLoopState.Priming, AiLoopState.Working, AiLoopState.Screening, AiLoopState.Judging, AiLoopState.Reflecting, AiLoopState.Restarting, AiLoopState.Closing, AiLoopState.Reported, AiLoopState.Stuck)
            table[AiLoopState.Watch] = listOf(AiLoopState.Alive, AiLoopState.Rebuilding)
        }


        val initialTargets: Map<AiLoopState, List<EntryTarget<AiLoopState, HistoryId>>> = LinkedHashMap<AiLoopState, List<EntryTarget<AiLoopState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<AiLoopState, List<EntryTarget<AiLoopState, HistoryId>>>) {
            table[AiLoopState.Budget] = listOf(StateTarget(AiLoopState.Within))
            table[AiLoopState.Drive] = listOf(StateTarget(AiLoopState.Running))
            table[AiLoopState.Running] = listOf(StateTarget(AiLoopState.Priming))
            table[AiLoopState.Watch] = listOf(StateTarget(AiLoopState.Alive))
        }


        val documentInitialTargetList: List<EntryTarget<AiLoopState, HistoryId>> =
            listOf(StateTarget(AiLoopState.Run))

        val historyParents: Map<HistoryId, AiLoopState> = LinkedHashMap<HistoryId, AiLoopState>().also {
            historyParentsPart0(it)
        }

        private fun historyParentsPart0(table: MutableMap<HistoryId, AiLoopState>) {
            table[historyWhere] = AiLoopState.Running
        }


        val historyDefaultTargets: Map<HistoryId, List<EntryTarget<AiLoopState, HistoryId>>> = LinkedHashMap<HistoryId, List<EntryTarget<AiLoopState, HistoryId>>>().also {
            historyDefaultTargetsPart0(it)
        }

        private fun historyDefaultTargetsPart0(table: MutableMap<HistoryId, List<EntryTarget<AiLoopState, HistoryId>>>) {
            table[historyWhere] = listOf(StateTarget(AiLoopState.Working))
        }


        val historiesByParent: Map<AiLoopState, List<Pair<HistoryId, Boolean>>> = LinkedHashMap<AiLoopState, List<Pair<HistoryId, Boolean>>>().also {
            historiesByParentPart0(it)
        }

        private fun historiesByParentPart0(table: MutableMap<AiLoopState, List<Pair<HistoryId, Boolean>>>) {
            table[AiLoopState.Running] = listOf(historyWhere to false)
        }

    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: alive's transition 0, as the microstep reads it.
        val transitionAliveAt0 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Alive,
            listOf(StateTarget(AiLoopState.Rebuilding)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: closing's transition 0, as the microstep reads it.
        val transitionClosingAt0 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Closing,
            listOf(StateTarget(AiLoopState.Reported)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: closing's transition 1, as the microstep reads it.
        val transitionClosingAt1 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Closing,
            listOf(StateTarget(AiLoopState.Screening)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: drive's transition 0, as the microstep reads it.
        val transitionDriveAt0 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Drive,
            listOf(StateTarget(AiLoopState.Paused)),
            0,
            hasActions = false,
            isInternal = true,
        )

        // W3C SCXML 3.13: drive's transition 1, as the microstep reads it.
        val transitionDriveAt1 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Drive,
            listOf(StateTarget(AiLoopState.Paused)),
            1,
            hasActions = false,
            isInternal = true,
        )

        // W3C SCXML 3.13: drive's transition 2, as the microstep reads it.
        val transitionDriveAt2 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Drive,
            listOf(StateTarget(AiLoopState.Restarting)),
            2,
            hasActions = false,
            isInternal = true,
        )

        // W3C SCXML 3.13: judging's transition 0, as the microstep reads it.
        val transitionJudgingAt0 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Judging,
            listOf(StateTarget(AiLoopState.Closing)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judging's transition 1, as the microstep reads it.
        val transitionJudgingAt1 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Judging,
            listOf(StateTarget(AiLoopState.Reflecting)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judging's transition 2, as the microstep reads it.
        val transitionJudgingAt2 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Judging,
            listOf(StateTarget(AiLoopState.Working)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: paused's transition 0, as the microstep reads it.
        val transitionPausedAt0 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Paused,
            listOf(StateTarget(AiLoopState.Judging)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: paused's transition 1, as the microstep reads it.
        val transitionPausedAt1 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Paused,
            listOf(StateTarget(AiLoopState.Paused)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: paused's transition 2, as the microstep reads it.
        val transitionPausedAt2 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Paused,
            listOf(HistoryTarget(historyWhere)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: paused's transition 3, as the microstep reads it.
        val transitionPausedAt3 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Paused,
            listOf(StateTarget(AiLoopState.Abandoned)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: priming's transition 0, as the microstep reads it.
        val transitionPrimingAt0 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Priming,
            listOf(StateTarget(AiLoopState.Working)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: rebuilding's transition 0, as the microstep reads it.
        val transitionRebuildingAt0 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Rebuilding,
            listOf(StateTarget(AiLoopState.Alive)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: reflecting's transition 0, as the microstep reads it.
        val transitionReflectingAt0 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Reflecting,
            listOf(StateTarget(AiLoopState.Restarting)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: reflecting's transition 1, as the microstep reads it.
        val transitionReflectingAt1 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Reflecting,
            listOf(StateTarget(AiLoopState.Working)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: restarting's transition 0, as the microstep reads it.
        val transitionRestartingAt0 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Restarting,
            listOf(StateTarget(AiLoopState.Stuck)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: restarting's transition 1, as the microstep reads it.
        val transitionRestartingAt1 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Restarting,
            listOf(StateTarget(AiLoopState.Priming)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Run,
            listOf(StateTarget(AiLoopState.Converged)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 1, as the microstep reads it.
        val transitionRunAt1 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Run,
            listOf(StateTarget(AiLoopState.Exhausted)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 2, as the microstep reads it.
        val transitionRunAt2 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Run,
            listOf(StateTarget(AiLoopState.Blocked)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 3, as the microstep reads it.
        val transitionRunAt3 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Run,
            listOf(StateTarget(AiLoopState.Failed)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 4, as the microstep reads it.
        val transitionRunAt4 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Run,
            listOf(StateTarget(AiLoopState.Cancelled)),
            4,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: screening's transition 0, as the microstep reads it.
        val transitionScreeningAt0 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Screening,
            listOf(StateTarget(AiLoopState.Working)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: screening's transition 1, as the microstep reads it.
        val transitionScreeningAt1 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Screening,
            listOf(StateTarget(AiLoopState.Paused)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: within's transition 0, as the microstep reads it.
        val transitionWithinAt0 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Within,
            listOf(StateTarget(AiLoopState.Spent)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: within's transition 1, as the microstep reads it.
        val transitionWithinAt1 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Within,
            listOf(StateTarget(AiLoopState.Within)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Working,
            listOf(StateTarget(AiLoopState.Judging)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 1, as the microstep reads it.
        val transitionWorkingAt1 = EnabledTransition<AiLoopState, HistoryId>(
            AiLoopState.Working,
            listOf(StateTarget(AiLoopState.Screening)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AiLoopState? = when (stateId) {
        "abandoned" -> AiLoopState.Abandoned
        "alive" -> AiLoopState.Alive
        "blocked" -> AiLoopState.Blocked
        "budget" -> AiLoopState.Budget
        "cancelled" -> AiLoopState.Cancelled
        "closing" -> AiLoopState.Closing
        "converged" -> AiLoopState.Converged
        "drive" -> AiLoopState.Drive
        "exhausted" -> AiLoopState.Exhausted
        "failed" -> AiLoopState.Failed
        "judging" -> AiLoopState.Judging
        "paused" -> AiLoopState.Paused
        "priming" -> AiLoopState.Priming
        "rebuilding" -> AiLoopState.Rebuilding
        "reflecting" -> AiLoopState.Reflecting
        "reported" -> AiLoopState.Reported
        "restarting" -> AiLoopState.Restarting
        "run" -> AiLoopState.Run
        "running" -> AiLoopState.Running
        "screening" -> AiLoopState.Screening
        "spent" -> AiLoopState.Spent
        "stuck" -> AiLoopState.Stuck
        "watch" -> AiLoopState.Watch
        "within" -> AiLoopState.Within
        "working" -> AiLoopState.Working
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AiLoopState): String = when (state) {
        is AiLoopState.Abandoned -> "abandoned"
        is AiLoopState.Alive -> "alive"
        is AiLoopState.Blocked -> "blocked"
        is AiLoopState.Budget -> "budget"
        is AiLoopState.Cancelled -> "cancelled"
        is AiLoopState.Closing -> "closing"
        is AiLoopState.Converged -> "converged"
        is AiLoopState.Drive -> "drive"
        is AiLoopState.Exhausted -> "exhausted"
        is AiLoopState.Failed -> "failed"
        is AiLoopState.Judging -> "judging"
        is AiLoopState.Paused -> "paused"
        is AiLoopState.Priming -> "priming"
        is AiLoopState.Rebuilding -> "rebuilding"
        is AiLoopState.Reflecting -> "reflecting"
        is AiLoopState.Reported -> "reported"
        is AiLoopState.Restarting -> "restarting"
        is AiLoopState.Run -> "run"
        is AiLoopState.Running -> "running"
        is AiLoopState.Screening -> "screening"
        is AiLoopState.Spent -> "spent"
        is AiLoopState.Stuck -> "stuck"
        is AiLoopState.Watch -> "watch"
        is AiLoopState.Within -> "within"
        is AiLoopState.Working -> "working"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AiLoopState): Int = when (state) {
        is AiLoopState.Abandoned -> 13
        is AiLoopState.Alive -> 15
        is AiLoopState.Blocked -> 24
        is AiLoopState.Budget -> 17
        is AiLoopState.Cancelled -> 23
        is AiLoopState.Closing -> 9
        is AiLoopState.Converged -> 20
        is AiLoopState.Drive -> 1
        is AiLoopState.Exhausted -> 21
        is AiLoopState.Failed -> 22
        is AiLoopState.Judging -> 6
        is AiLoopState.Paused -> 12
        is AiLoopState.Priming -> 3
        is AiLoopState.Rebuilding -> 16
        is AiLoopState.Reflecting -> 7
        is AiLoopState.Reported -> 10
        is AiLoopState.Restarting -> 8
        is AiLoopState.Run -> 0
        is AiLoopState.Running -> 2
        is AiLoopState.Screening -> 5
        is AiLoopState.Spent -> 19
        is AiLoopState.Stuck -> 11
        is AiLoopState.Watch -> 14
        is AiLoopState.Within -> 18
        is AiLoopState.Working -> 4
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AiLoopEvent? = when (name) {
        "cancel" -> AiLoopEvent.Cancel
        "done.state.drive" -> AiLoopEvent.Done.State.Drive
        "done.state.run" -> AiLoopEvent.Done.State.Run
        "done.state.running" -> AiLoopEvent.Done.State.Running
        "error.execution" -> AiLoopEvent.Error.Execution
        "fail" -> AiLoopEvent.Fail
        "hold" -> AiLoopEvent.Hold
        "judge" -> AiLoopEvent.Judge.Self
        "judge.begin" -> AiLoopEvent.Judge.Begin
        "notify.human" -> AiLoopEvent.Notify.Human
        "prompt.end" -> AiLoopEvent.Prompt.End
        "prompt.sent" -> AiLoopEvent.Prompt.Sent
        "prompt.start" -> AiLoopEvent.Prompt.Start
        "prompt.turn" -> AiLoopEvent.Prompt.Turn
        "reflect.applied" -> AiLoopEvent.Reflect.Applied
        "reflect.begin" -> AiLoopEvent.Reflect.Begin
        "reflect.none" -> AiLoopEvent.Reflect.None
        "resume" -> AiLoopEvent.Resume
        "run.blocked" -> AiLoopEvent.Run.Blocked
        "run.converged" -> AiLoopEvent.Run.Converged
        "run.exhausted" -> AiLoopEvent.Run.Exhausted
        "screen.begin" -> AiLoopEvent.Screen.Begin
        "screen.matched" -> AiLoopEvent.Screen.Matched
        "screen.none" -> AiLoopEvent.Screen.None
        "session.lost" -> AiLoopEvent.Session.Lost
        "session.ready" -> AiLoopEvent.Session.Ready
        "session.replace" -> AiLoopEvent.Session.Replace
        "turn.blocked" -> AiLoopEvent.Turn.Blocked
        "turn.done" -> AiLoopEvent.Turn.Done
        "turn.interrupted" -> AiLoopEvent.Turn.Interrupted
        "unattended" -> AiLoopEvent.Unattended
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AiLoopEvent): String? = when (event) {
        is AiLoopEvent.Cancel -> "cancel"
        is AiLoopEvent.Done.State.Drive -> "done.state.drive"
        is AiLoopEvent.Done.State.Run -> "done.state.run"
        is AiLoopEvent.Done.State.Running -> "done.state.running"
        is AiLoopEvent.Error.Execution -> "error.execution"
        is AiLoopEvent.Fail -> "fail"
        is AiLoopEvent.Hold -> "hold"
        is AiLoopEvent.Judge.Self -> "judge"
        is AiLoopEvent.Judge.Begin -> "judge.begin"
        is AiLoopEvent.Notify.Human -> "notify.human"
        is AiLoopEvent.Prompt.End -> "prompt.end"
        is AiLoopEvent.Prompt.Sent -> "prompt.sent"
        is AiLoopEvent.Prompt.Start -> "prompt.start"
        is AiLoopEvent.Prompt.Turn -> "prompt.turn"
        is AiLoopEvent.Reflect.Applied -> "reflect.applied"
        is AiLoopEvent.Reflect.Begin -> "reflect.begin"
        is AiLoopEvent.Reflect.None -> "reflect.none"
        is AiLoopEvent.Resume -> "resume"
        is AiLoopEvent.Run.Blocked -> "run.blocked"
        is AiLoopEvent.Run.Converged -> "run.converged"
        is AiLoopEvent.Run.Exhausted -> "run.exhausted"
        is AiLoopEvent.Screen.Begin -> "screen.begin"
        is AiLoopEvent.Screen.Matched -> "screen.matched"
        is AiLoopEvent.Screen.None -> "screen.none"
        is AiLoopEvent.Session.Lost -> "session.lost"
        is AiLoopEvent.Session.Ready -> "session.ready"
        is AiLoopEvent.Session.Replace -> "session.replace"
        is AiLoopEvent.Turn.Blocked -> "turn.blocked"
        is AiLoopEvent.Turn.Done -> "turn.done"
        is AiLoopEvent.Turn.Interrupted -> "turn.interrupted"
        is AiLoopEvent.Unattended -> "unattended"
    }




    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML 5.3: the declaration hook `enterAt` reaches. Every other caller
    // arrives through a guard, an assign or a script block, all of which run
    // `ensureScriptEngine()` on their own way in; a resume runs none of them,
    // and a host putting saved values back needs the variables to exist first.
    override fun declareDatamodel() {
        ensureScriptEngine()
    }

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // §scxml-C-1-1 / §scxml-C-2-3: the `_ioprocessors` entries come from the
        // same helper every other backend uses, so a machine reads the same
        // entry names and the same addresses whichever one runs it.
        engine.setupSystemVariables(
            sid,
            "ai_loop",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        declareVariablesPart0(engine, sid)



        // W3C SCXML 5.9.2: Register In() predicate callback
        engine.setStateQueryCallback(sid) { stateId -> isStateActive(stateId) }

        // W3C SCXML 6.4: Apply pending invoke params from parent
        // Only set params matching child's declared datamodel variables (C++ DatamodelValidationHelper)
        if (pendingInvokeParams.isNotEmpty()) {
            for ((pName, pValue) in pendingInvokeParams) {
                if (engine.hasVariable(sid, pName)) {
                    try { engine.setVariable(sid, pName, pValue) } catch (_: Exception) {}
                }
            }
            pendingInvokeParams = emptyMap()
        }

        scriptEngineInitialized = true
    }

    @Suppress("UNUSED_PARAMETER")
    private fun declareVariablesPart0(engine: ScxmlScriptEngine, sid: String) {
        // W3C SCXML 5.3: Initialize variable 'north_star' with expr
        try {
            val initResult_northStar = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"(edit me) the outcome this loop exists to reach\"", "'(edit me) the outcome this loop exists to reach'"))
            engine.setVariable(sid, "north_star", initResult_northStar)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='north_star'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'milestone' with expr
        try {
            val initResult_milestone = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"(edit me) the next checkpoint on the way there\"", "'(edit me) the next checkpoint on the way there'"))
            engine.setVariable(sid, "milestone", initResult_milestone)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='milestone'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'reference' with expr
        try {
            val initResult_reference = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"(edit me) paths, URLs or repos to consult\"", "'(edit me) paths, URLs or repos to consult'"))
            engine.setVariable(sid, "reference", initResult_reference)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='reference'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'start_prompt' with expr
        try {
            val initResult_startPrompt = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("(_scxml_tostring((_scxml_tostring(_scxml_add((_scxml_tostring((_scxml_tostring(_scxml_add((_scxml_tostring((_scxml_tostring((\"North star: \" .. _scxml_tostring(north_star))) .. \"\\n\")) .. \"Milestone: \"), milestone)) .. \"\\n\")) .. \"Reference: \"), reference)) .. \"\\n\")) .. \"Report what you did and what is left.\")", "'North star: ' + north_star + '\\n' +                 'Milestone: ' + milestone + '\\n' +                 'Reference: ' + reference + '\\n' +                 'Report what you did and what is left.'"))
            engine.setVariable(sid, "start_prompt", initResult_startPrompt)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='start_prompt'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'turn_prompt' with expr
        try {
            val initResult_turnPrompt = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("(_scxml_tostring((_scxml_tostring((\"Continue toward: \" .. _scxml_tostring(milestone))) .. \"\\n\")) .. \"Do the next smallest thing that is verifiable, then report.\")", "'Continue toward: ' + milestone + '\\n' +                 'Do the next smallest thing that is verifiable, then report.'"))
            engine.setVariable(sid, "turn_prompt", initResult_turnPrompt)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='turn_prompt'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'end_prompt' with expr
        try {
            val initResult_endPrompt = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"Summarise what changed, what was verified, and what is left open.\"", "'Summarise what changed, what was verified, and what is left open.'"))
            engine.setVariable(sid, "end_prompt", initResult_endPrompt)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='end_prompt'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'done_marker' with expr
        try {
            val initResult_doneMarker = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"MILESTONE REACHED\"", "'MILESTONE REACHED'"))
            engine.setVariable(sid, "done_marker", initResult_doneMarker)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='done_marker'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'screen_rules' with expr
        try {
            val initResult_screenRules = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{{[\"when\"] = \"design-decision\", [\"keys\"] = \"Escape\", [\"text\"] = \"Ignore cost. Rethink for the most durable answer, then proceed.\"}, {[\"when\"] = \"design-proposal\", [\"keys\"] = \"Escape\", [\"text\"] = \"Ignore cost. Rethink for the most durable answer, then proceed.\"}, {[\"when\"] = \"multiple-choice\", [\"keys\"] = \"Escape\", [\"text\"] = \"Ignore cost. Rethink for the most durable answer, then proceed.\"}}", "[             { when: 'design-decision', keys: 'Escape',               text: 'Ignore cost. Rethink for the most durable answer, then proceed.' },             { when: 'design-proposal', keys: 'Escape',               text: 'Ignore cost. Rethink for the most durable answer, then proceed.' },             { when: 'multiple-choice', keys: 'Escape',               text: 'Ignore cost. Rethink for the most durable answer, then proceed.' }           ]"))
            engine.setVariable(sid, "screen_rules", initResult_screenRules)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='screen_rules'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'screen_permissions' with expr
        try {
            val initResult_screenPermissions = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("false", "false"))
            engine.setVariable(sid, "screen_permissions", initResult_screenPermissions)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='screen_permissions'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'max_turns' with expr
        try {
            val initResult_maxTurns = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("40", "40"))
            engine.setVariable(sid, "max_turns", initResult_maxTurns)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='max_turns'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'reflect_every' with expr
        try {
            val initResult_reflectEvery = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("8", "8"))
            engine.setVariable(sid, "reflect_every", initResult_reflectEvery)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='reflect_every'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'max_restarts' with expr
        try {
            val initResult_maxRestarts = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("6", "6"))
            engine.setVariable(sid, "max_restarts", initResult_maxRestarts)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='max_restarts'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'turns' with expr
        try {
            val initResult_turns = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "turns", initResult_turns)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='turns'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'turns_since_reflect' with expr
        try {
            val initResult_turnsSinceReflect = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "turns_since_reflect", initResult_turnsSinceReflect)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='turns_since_reflect'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'screened' with expr
        try {
            val initResult_screened = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "screened", initResult_screened)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='screened'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'restarts' with expr
        try {
            val initResult_restarts = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "restarts", initResult_restarts)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<data id='restarts'> expr failed to evaluate")
        }
    }

    // W3C SCXML 5.9: Guard evaluation with error.execution on failure
    //
    // The guard arrives as a `ScriptSource`, not a `String`: it carries the
    // language its text is in, so a machine generated for a Lua engine hands
    // over Lua the build-time frontend produced and one generated for an
    // ECMAScript engine hands over the author's own text — and the engine is
    // never left to guess which it got. The C++ sibling
    // (`process_transition.jinja2`) takes the same argument for the same
    // reason.
    private fun safeEvaluateGuard(guardExpr: com.sce.runtime.ScriptSource): Boolean =
        evaluateGuardRaising(guardExpr, "a <transition> cond failed to evaluate") ?: false

    // W3C SCXML 5.9.1 + 4.9: an <if> or <elseif> cond, evaluated and reported
    // as a transition guard is. A failure also runs [onFailure]: the <if>
    // still selects on `false`, and is then the element whose processing
    // raised, so its block ends after it.
    @Suppress("unused")
    private inline fun evaluateIfCond(guardExpr: com.sce.runtime.ScriptSource, onFailure: () -> Unit): Boolean {
        val result = evaluateGuardRaising(guardExpr, "an <if> cond failed to evaluate")
        if (result == null) onFailure()
        return result ?: false
    }

    // W3C SCXML 5.9.1: a cond that cannot be evaluated raises error.execution;
    // `null` says so, where a bare `false` could not.
    private fun evaluateGuardRaising(guardExpr: com.sce.runtime.ScriptSource, reason: String): Boolean? {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, reason)
            null
        }
    }

    // W3C SCXML B.2: the value of an inline `<content>` body, serialized
    // for transport.
    //
    // The reading is decided at build time — `source` is already the
    // expression or string literal the clause's ordered readings give —
    // and this evaluates it *here*, at send time, rather than handing the
    // expression to whatever reads `_event.data` later. That distinction
    // is not academic: the two engines this backend runs on disagree
    // about what a data string is. QuickJS tries a JS evaluation before
    // falling back; Rhino goes straight from JSON to the normalized
    // string, so an expression handed to it arrives as its own source
    // text. `JSON.stringify` is what both of them can read back, and it
    // is the same shape the C++ backend transports.
    //
    // The serialization wraps BOTH halves, in each half's own language. A
    // wrapper composed around one of them only would build a `ScriptSource`
    // whose two strings no longer say the same thing, and the diagnostic that
    // reads `source` would name an expression the engine never ran. `JSON` is
    // a §scxml-B-2-9 name both engines carry, so the wrapper is the same eight
    // characters on either arm — what differs is what it wraps.
    private fun evaluateSendContent(source: com.sce.runtime.ScriptSource): String {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        val serialized = when (source.language) {
            com.sce.runtime.ScriptLanguage.ECMAScript ->
                com.sce.runtime.ScriptSource.ecmascript("JSON.stringify((" + source.source + "))")
            com.sce.runtime.ScriptLanguage.Lua ->
                com.sce.runtime.ScriptSource.lua(
                    "JSON.stringify((" + source.text + "))",
                    "JSON.stringify((" + source.source + "))",
                )
        }
        return try {
            engine.evaluateExpr(sid, serialized)?.toString() ?: ""
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "an expression could not be serialised to JSON")
            ""
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    //
    // Both halves carry a language: this engine's Lua arm splices the location
    // in front of `=` and runs the result, so a write target written in
    // ECMAScript has to have been lowered too. Same split as
    // `ScxmlScriptEngine.assign`.
    // Returns whether the assignment took place; on failure error.execution is
    // already raised, and the caller ends its block (W3C SCXML 4.9).
    private fun executeAssign(location: com.sce.runtime.ScriptSource, expr: com.sce.runtime.ScriptSource): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.assign(sid, location, expr)
            true
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<assign> failed")
            false
        }
    }

    // W3C SCXML 5.8: Script block execution
    // Returns whether the script ran; on failure error.execution is already
    // raised, and the caller ends its block (W3C SCXML 4.9).
    private fun executeScriptBlock(script: com.sce.runtime.ScriptSource): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.executeScript(sid, script)
            true
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AiLoopEvent) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        val meta = currentEventMetadata
        // §scxml-5.10: the name the event ARRIVED under, which is longer than its
        // member's when a name the document does not write was matched through a
        // prefix (§scxml-3.12.1).
        val eventName = meta.name.ifEmpty { eventNameOf(event) ?: return }
        // W3C SCXML 5.10.1: C++ classifyEventType — platform events override type
        val effectiveType = when {
            eventName.startsWith("done.") || eventName.startsWith("error.") -> "platform"
            else -> meta.type
        }
        // W3C SCXML 5.10.1 + C.1: `_event.origin` and `_event.origintype` are the
        // reply address the processor that delivered the event wrote where it
        // built it, as one value (docs/adr/0090): the SCXML processor's type and
        // the sender's published `_ioprocessors` location for an event a session
        // sent, a host's own pair for what a host named, the Mesh transport's
        // for a peer. Read here as written, with no conversion. An event no
        // processor stamped — an internal or platform event, a host-posted event
        // or reply that names no party, a BasicHTTP request — has none and is
        // handed over as the empty string, which the binding surfaces as blank
        // (undefined, §scxml-B-2-8): nothing is filled in for it, neither this
        // session's own address nor the SCXML processor's type.
        val eventOrigin = meta.origin
        // §scxml-B-2-8-1: the binding answers which rung the payload got, and
        // that answer used to end here. The ladder decided between a DOM, a
        // value and a space-normalized string, and the decision was dropped —
        // so a payload that announced structure and would not parse reached
        // the document as raw characters, every `_event.data.<field>` read
        // empty, and nothing anywhere could say so.
        //
        // Recorded on the spot rather than returned up: this class extends
        // `StateMachineEngine`, so the frame that binds already holds both the
        // reading and the event it belongs to — which is the pairing the count
        // needs.
        val payloadReading = engine.setCurrentEvent(
            sid,
            com.sce.runtime.SetCurrentEventArgs(
                name = eventName,
                data = meta.data,
                type = effectiveType,
                sendId = meta.sendId,
                origin = eventOrigin?.address ?: "",
                originType = eventOrigin?.processor ?: "",
                invokeId = meta.invokeId
            )
        )
        notePayloadReading(event, payloadReading)
    }



    // W3C SCXML 5.10: bind the event as the `_event` its transitions' guards
    // read — once, before the first guard runs, and not for an eventless
    // selection, which has no event of its own.
    override fun bindCurrentEvent(event: AiLoopEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when (state) {
        is AiLoopState.Alive -> firstEnabledTransitionOfAlive(state, event)
        is AiLoopState.Closing -> firstEnabledTransitionOfClosing(state, event)
        is AiLoopState.Drive -> firstEnabledTransitionOfDrive(state, event)
        is AiLoopState.Judging -> firstEnabledTransitionOfJudging(state, event)
        is AiLoopState.Paused -> firstEnabledTransitionOfPaused(state, event)
        is AiLoopState.Priming -> firstEnabledTransitionOfPriming(state, event)
        is AiLoopState.Rebuilding -> firstEnabledTransitionOfRebuilding(state, event)
        is AiLoopState.Reflecting -> firstEnabledTransitionOfReflecting(state, event)
        is AiLoopState.Restarting -> firstEnabledTransitionOfRestarting(state, event)
        is AiLoopState.Run -> firstEnabledTransitionOfRun(state, event)
        is AiLoopState.Screening -> firstEnabledTransitionOfScreening(state, event)
        is AiLoopState.Within -> firstEnabledTransitionOfWithin(state, event)
        is AiLoopState.Working -> firstEnabledTransitionOfWorking(state, event)
        else -> null
    }


    // alive's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAlive(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when {
        event is AiLoopEvent.Session.Lost -> TransitionsPart0.transitionAliveAt0
        else -> null
    }


    // closing's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfClosing(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when {
        event is AiLoopEvent.Turn.Done -> TransitionsPart0.transitionClosingAt0
        event is AiLoopEvent.Turn.Blocked -> TransitionsPart0.transitionClosingAt1
        else -> null
    }


    // drive's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfDrive(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when {
        event is AiLoopEvent.Hold -> TransitionsPart0.transitionDriveAt0
        event is AiLoopEvent.Turn.Interrupted -> TransitionsPart0.transitionDriveAt1
        event is AiLoopEvent.Session.Lost -> TransitionsPart0.transitionDriveAt2
        else -> null
    }


    // judging's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfJudging(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when {
        (event is AiLoopEvent.Judge || event is AiLoopEvent.Judge.Begin) && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_truthy(_event.data.done)", "_event.data.done")) -> TransitionsPart0.transitionJudgingAt0
        (event is AiLoopEvent.Judge || event is AiLoopEvent.Judge.Begin) && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(turns_since_reflect >= reflect_every)", "turns_since_reflect >= reflect_every")) -> TransitionsPart0.transitionJudgingAt1
        (event is AiLoopEvent.Judge || event is AiLoopEvent.Judge.Begin) -> TransitionsPart0.transitionJudgingAt2
        else -> null
    }


    // paused's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfPaused(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when {
        event is AiLoopEvent.Turn.Done -> TransitionsPart0.transitionPausedAt0
        event is AiLoopEvent.Turn.Interrupted -> TransitionsPart0.transitionPausedAt1
        event is AiLoopEvent.Resume -> TransitionsPart0.transitionPausedAt2
        event is AiLoopEvent.Unattended -> TransitionsPart0.transitionPausedAt3
        else -> null
    }


    // priming's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfPriming(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when {
        event is AiLoopEvent.Prompt.Sent -> TransitionsPart0.transitionPrimingAt0
        else -> null
    }


    // rebuilding's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRebuilding(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when {
        event is AiLoopEvent.Session.Ready -> TransitionsPart0.transitionRebuildingAt0
        else -> null
    }


    // reflecting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfReflecting(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when {
        event is AiLoopEvent.Reflect.Applied -> TransitionsPart0.transitionReflectingAt0
        event is AiLoopEvent.Reflect.None -> TransitionsPart0.transitionReflectingAt1
        else -> null
    }


    // restarting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRestarting(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when {
        event is AiLoopEvent.Session.Ready && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(restarts > max_restarts)", "restarts > max_restarts")) -> TransitionsPart0.transitionRestartingAt0
        event is AiLoopEvent.Session.Ready -> TransitionsPart0.transitionRestartingAt1
        else -> null
    }


    // run's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRun(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when {
        event is AiLoopEvent.Run.Converged -> TransitionsPart0.transitionRunAt0
        event is AiLoopEvent.Run.Exhausted -> TransitionsPart0.transitionRunAt1
        event is AiLoopEvent.Run.Blocked -> TransitionsPart0.transitionRunAt2
        event is AiLoopEvent.Fail -> TransitionsPart0.transitionRunAt3
        event is AiLoopEvent.Cancel -> TransitionsPart0.transitionRunAt4
        else -> null
    }


    // screening's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfScreening(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when {
        event is AiLoopEvent.Screen.Matched -> TransitionsPart0.transitionScreeningAt0
        event is AiLoopEvent.Screen.None -> TransitionsPart0.transitionScreeningAt1
        else -> null
    }


    // within's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWithin(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when {
        event is AiLoopEvent.Turn.Done && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_scxml_add(turns, 1) >= max_turns)", "turns + 1 >= max_turns")) -> TransitionsPart0.transitionWithinAt0
        event is AiLoopEvent.Turn.Done -> TransitionsPart0.transitionWithinAt1
        else -> null
    }


    // working's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWorking(state: AiLoopState, event: AiLoopEvent?): EnabledTransition<AiLoopState, HistoryId>? = when {
        event is AiLoopEvent.Turn.Done -> TransitionsPart0.transitionWorkingAt0
        event is AiLoopEvent.Turn.Blocked -> TransitionsPart0.transitionWorkingAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: ai_loop.scxml:155 :: _machine
    override fun onEntry(state: AiLoopState, isDefaultEntry: Boolean) {
        when (state) {
            is AiLoopState.Abandoned -> onEntryOfAbandoned()
            is AiLoopState.Alive -> onEntryOfAlive()
            is AiLoopState.Blocked -> onEntryOfBlocked()
            is AiLoopState.Budget -> onEntryOfBudget()
            is AiLoopState.Cancelled -> onEntryOfCancelled()
            is AiLoopState.Closing -> onEntryOfClosing()
            is AiLoopState.Converged -> onEntryOfConverged()
            is AiLoopState.Drive -> onEntryOfDrive()
            is AiLoopState.Exhausted -> onEntryOfExhausted()
            is AiLoopState.Failed -> onEntryOfFailed()
            is AiLoopState.Judging -> onEntryOfJudging()
            is AiLoopState.Paused -> onEntryOfPaused()
            is AiLoopState.Priming -> onEntryOfPriming()
            is AiLoopState.Rebuilding -> onEntryOfRebuilding()
            is AiLoopState.Reflecting -> onEntryOfReflecting()
            is AiLoopState.Reported -> onEntryOfReported()
            is AiLoopState.Restarting -> onEntryOfRestarting()
            is AiLoopState.Run -> onEntryOfRun()
            is AiLoopState.Running -> onEntryOfRunning()
            is AiLoopState.Screening -> onEntryOfScreening()
            is AiLoopState.Spent -> onEntryOfSpent()
            is AiLoopState.Stuck -> onEntryOfStuck()
            is AiLoopState.Watch -> onEntryOfWatch()
            is AiLoopState.Within -> onEntryOfWithin()
            is AiLoopState.Working -> onEntryOfWorking()
        }
    }


    private fun onEntryOfAbandoned() {
                // SCE-MAP: ai_loop.scxml:493 :: abandoned :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            raiseInternal(AiLoopEvent.Run.Blocked)
                }
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(AiLoopEvent.Done.State.Drive, EventMetadata.platform())
                // W3C SCXML 3.7.1: this <final> may have completed the
                // <parallel> grandparent — Appendix D's isInFinalState, which
                // counts a region that is itself a <parallel> only once all
                // of ITS regions are final.
                if (isStateInFinalState(AiLoopState.Run)) {
                    raiseInternal(AiLoopEvent.Done.State.Run)
                }
    }

    private fun onEntryOfAlive() {
                // SCE-MAP: ai_loop.scxml:506 :: alive :: _state_body
    }

    private fun onEntryOfBlocked() {
                // SCE-MAP: ai_loop.scxml:548 :: blocked :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfBudget() {
                // SCE-MAP: ai_loop.scxml:520 :: budget :: _state_body
    }

    private fun onEntryOfCancelled() {
                // SCE-MAP: ai_loop.scxml:547 :: cancelled :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfClosing() {
                // SCE-MAP: ai_loop.scxml:405 :: closing :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            var paramFailed = false
            ensureScriptEngine()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "text", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("end_prompt", "end_prompt")))
            } catch (_: Exception) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send> <param name='text'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "prompt.end",
                target = "",
                content = "",
                params = sendWireParams,
                sendId = "__send_7",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_7")
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfConverged() {
                // SCE-MAP: ai_loop.scxml:544 :: converged :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfDrive() {
                // SCE-MAP: ai_loop.scxml:236 :: drive :: _state_body
    }

    private fun onEntryOfExhausted() {
                // SCE-MAP: ai_loop.scxml:545 :: exhausted :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailed() {
                // SCE-MAP: ai_loop.scxml:546 :: failed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfJudging() {
                // SCE-MAP: ai_loop.scxml:344 :: judging :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            var paramFailed = false
            ensureScriptEngine()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "marker", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("done_marker", "done_marker")))
            } catch (_: Exception) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send> <param name='marker'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "judge.begin",
                target = "",
                content = "",
                params = sendWireParams,
                sendId = "__send_3",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_3")
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfPaused() {
                // SCE-MAP: ai_loop.scxml:451 :: paused :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "notify.human",
                target = "",
                content = "",
                params = sendWireParams,
                sendId = "__send_8",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_8")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfPriming() {
                // SCE-MAP: ai_loop.scxml:291 :: priming :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            var paramFailed = false
            ensureScriptEngine()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "text", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("start_prompt", "start_prompt")))
            } catch (_: Exception) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send> <param name='text'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "prompt.start",
                target = "",
                content = "",
                params = sendWireParams,
                sendId = "__send_0",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_0")
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfRebuilding() {
                // SCE-MAP: ai_loop.scxml:509 :: rebuilding :: _state_body
    }

    private fun onEntryOfReflecting() {
                // SCE-MAP: ai_loop.scxml:374 :: reflecting :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("turns_since_reflect", "turns_since_reflect"), com.sce.runtime.ScriptSource.lua("0", "0"))) {
                return@run
            }


            if (run send@{
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "reflect.begin",
                target = "",
                content = "",
                params = sendWireParams,
                sendId = "__send_5",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_5")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfReported() {
                // SCE-MAP: ai_loop.scxml:426 :: reported :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            raiseInternal(AiLoopEvent.Run.Converged)
                }
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(AiLoopEvent.Done.State.Running, EventMetadata.platform())
    }

    private fun onEntryOfRestarting() {
                // SCE-MAP: ai_loop.scxml:394 :: restarting :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("restarts", "restarts"), com.sce.runtime.ScriptSource.lua("_scxml_add(restarts, 1)", "restarts + 1"))) {
                return@run
            }


            if (run send@{
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "session.replace",
                target = "",
                content = "",
                params = sendWireParams,
                sendId = "__send_6",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_6")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfRun() {
                // SCE-MAP: ai_loop.scxml:233 :: run :: _state_body
    }

    private fun onEntryOfRunning() {
                // SCE-MAP: ai_loop.scxml:274 :: running :: _state_body
    }

    private fun onEntryOfScreening() {
                // SCE-MAP: ai_loop.scxml:327 :: screening :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("screened", "screened"), com.sce.runtime.ScriptSource.lua("_scxml_add(screened, 1)", "screened + 1"))) {
                return@run
            }


            if (run send@{
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "screen.begin",
                target = "",
                content = "",
                params = sendWireParams,
                sendId = "__send_1",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_1")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfSpent() {
                // SCE-MAP: ai_loop.scxml:529 :: spent :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            raiseInternal(AiLoopEvent.Run.Exhausted)
                }
    }

    private fun onEntryOfStuck() {
                // SCE-MAP: ai_loop.scxml:434 :: stuck :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            raiseInternal(AiLoopEvent.Run.Exhausted)
                }
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(AiLoopEvent.Done.State.Running, EventMetadata.platform())
    }

    private fun onEntryOfWatch() {
                // SCE-MAP: ai_loop.scxml:505 :: watch :: _state_body
    }

    private fun onEntryOfWithin() {
                // SCE-MAP: ai_loop.scxml:521 :: within :: _state_body
    }

    private fun onEntryOfWorking() {
                // SCE-MAP: ai_loop.scxml:310 :: working :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: ai_loop.scxml:155 :: _machine
    override fun onExit(state: AiLoopState) {
        when (state) {
            is AiLoopState.Abandoned -> onExitOfAbandoned()
            is AiLoopState.Alive -> onExitOfAlive()
            is AiLoopState.Blocked -> onExitOfBlocked()
            is AiLoopState.Budget -> onExitOfBudget()
            is AiLoopState.Cancelled -> onExitOfCancelled()
            is AiLoopState.Closing -> onExitOfClosing()
            is AiLoopState.Converged -> onExitOfConverged()
            is AiLoopState.Drive -> onExitOfDrive()
            is AiLoopState.Exhausted -> onExitOfExhausted()
            is AiLoopState.Failed -> onExitOfFailed()
            is AiLoopState.Judging -> onExitOfJudging()
            is AiLoopState.Paused -> onExitOfPaused()
            is AiLoopState.Priming -> onExitOfPriming()
            is AiLoopState.Rebuilding -> onExitOfRebuilding()
            is AiLoopState.Reflecting -> onExitOfReflecting()
            is AiLoopState.Reported -> onExitOfReported()
            is AiLoopState.Restarting -> onExitOfRestarting()
            is AiLoopState.Run -> onExitOfRun()
            is AiLoopState.Running -> onExitOfRunning()
            is AiLoopState.Screening -> onExitOfScreening()
            is AiLoopState.Spent -> onExitOfSpent()
            is AiLoopState.Stuck -> onExitOfStuck()
            is AiLoopState.Watch -> onExitOfWatch()
            is AiLoopState.Within -> onExitOfWithin()
            is AiLoopState.Working -> onExitOfWorking()
        }
    }


    private fun onExitOfAbandoned() {
                // SCE-MAP: ai_loop.scxml:493 :: abandoned :: _state_body
    }

    private fun onExitOfAlive() {
                // SCE-MAP: ai_loop.scxml:506 :: alive :: _state_body
    }

    private fun onExitOfBlocked() {
                // SCE-MAP: ai_loop.scxml:548 :: blocked :: _state_body
    }

    private fun onExitOfBudget() {
                // SCE-MAP: ai_loop.scxml:520 :: budget :: _state_body
    }

    private fun onExitOfCancelled() {
                // SCE-MAP: ai_loop.scxml:547 :: cancelled :: _state_body
    }

    private fun onExitOfClosing() {
                // SCE-MAP: ai_loop.scxml:405 :: closing :: _state_body
    }

    private fun onExitOfConverged() {
                // SCE-MAP: ai_loop.scxml:544 :: converged :: _state_body
    }

    private fun onExitOfDrive() {
                // SCE-MAP: ai_loop.scxml:236 :: drive :: _state_body
    }

    private fun onExitOfExhausted() {
                // SCE-MAP: ai_loop.scxml:545 :: exhausted :: _state_body
    }

    private fun onExitOfFailed() {
                // SCE-MAP: ai_loop.scxml:546 :: failed :: _state_body
    }

    private fun onExitOfJudging() {
                // SCE-MAP: ai_loop.scxml:344 :: judging :: _state_body
    }

    private fun onExitOfPaused() {
                // SCE-MAP: ai_loop.scxml:451 :: paused :: _state_body
    }

    private fun onExitOfPriming() {
                // SCE-MAP: ai_loop.scxml:291 :: priming :: _state_body
    }

    private fun onExitOfRebuilding() {
                // SCE-MAP: ai_loop.scxml:509 :: rebuilding :: _state_body
    }

    private fun onExitOfReflecting() {
                // SCE-MAP: ai_loop.scxml:374 :: reflecting :: _state_body
    }

    private fun onExitOfReported() {
                // SCE-MAP: ai_loop.scxml:426 :: reported :: _state_body
    }

    private fun onExitOfRestarting() {
                // SCE-MAP: ai_loop.scxml:394 :: restarting :: _state_body
    }

    private fun onExitOfRun() {
                // SCE-MAP: ai_loop.scxml:233 :: run :: _state_body
    }

    private fun onExitOfRunning() {
                // SCE-MAP: ai_loop.scxml:274 :: running :: _state_body
    }

    private fun onExitOfScreening() {
                // SCE-MAP: ai_loop.scxml:327 :: screening :: _state_body
    }

    private fun onExitOfSpent() {
                // SCE-MAP: ai_loop.scxml:529 :: spent :: _state_body
    }

    private fun onExitOfStuck() {
                // SCE-MAP: ai_loop.scxml:434 :: stuck :: _state_body
    }

    private fun onExitOfWatch() {
                // SCE-MAP: ai_loop.scxml:505 :: watch :: _state_body
    }

    private fun onExitOfWithin() {
                // SCE-MAP: ai_loop.scxml:521 :: within :: _state_body
    }

    private fun onExitOfWorking() {
                // SCE-MAP: ai_loop.scxml:310 :: working :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: ai_loop.scxml:155 :: _machine
    override fun executeTransitionContent(source: AiLoopState, transitionIndex: Int) {
        when (source) {
            is AiLoopState.Judging -> transitionContentOfJudging(transitionIndex)
            is AiLoopState.Paused -> transitionContentOfPaused(transitionIndex)
            is AiLoopState.Reflecting -> transitionContentOfReflecting(transitionIndex)
            is AiLoopState.Within -> transitionContentOfWithin(transitionIndex)
            is AiLoopState.Working -> transitionContentOfWorking(transitionIndex)
            else -> {}
        }
    }


    // The content of `judging`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfJudging(transitionIndex: Int) {
        when (transitionIndex) {
            2 -> transitionContentOfJudgingAt2()
            else -> {}
        }
    }


    private fun transitionContentOfJudgingAt2() {
                // SCE-MAP: ai_loop.scxml:362 :: judging :: _transition_2


            if (run send@{
            var paramFailed = false
            ensureScriptEngine()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "text", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("turn_prompt", "turn_prompt")))
            } catch (_: Exception) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send> <param name='text'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "prompt.turn",
                target = "",
                content = "",
                params = sendWireParams,
                sendId = "__send_2",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_2")
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    // The content of `paused`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfPaused(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfPausedAt0()
            else -> {}
        }
    }


    private fun transitionContentOfPausedAt0() {
                // SCE-MAP: ai_loop.scxml:468 :: paused :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("turns_since_reflect", "turns_since_reflect"), com.sce.runtime.ScriptSource.lua("_scxml_add(turns_since_reflect, 1)", "turns_since_reflect + 1"))) {
                return
            }
    }

    // The content of `reflecting`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfReflecting(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfReflectingAt0()
            1 -> transitionContentOfReflectingAt1()
            else -> {}
        }
    }


    private fun transitionContentOfReflectingAt0() {
                // SCE-MAP: ai_loop.scxml:379 :: reflecting :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("start_prompt", "start_prompt"), com.sce.runtime.ScriptSource.lua("_event.data.start_prompt", "_event.data.start_prompt"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("turn_prompt", "turn_prompt"), com.sce.runtime.ScriptSource.lua("_event.data.turn_prompt", "_event.data.turn_prompt"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("milestone", "milestone"), com.sce.runtime.ScriptSource.lua("_event.data.milestone", "_event.data.milestone"))) {
                return
            }
    }

    private fun transitionContentOfReflectingAt1() {
                // SCE-MAP: ai_loop.scxml:385 :: reflecting :: _transition_1


            if (run send@{
            var paramFailed = false
            ensureScriptEngine()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "text", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("turn_prompt", "turn_prompt")))
            } catch (_: Exception) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send> <param name='text'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "prompt.turn",
                target = "",
                content = "",
                params = sendWireParams,
                sendId = "__send_4",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(AiLoopEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_4")
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    // The content of `within`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWithin(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWithinAt0()
            1 -> transitionContentOfWithinAt1()
            else -> {}
        }
    }


    private fun transitionContentOfWithinAt0() {
                // SCE-MAP: ai_loop.scxml:522 :: within :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("turns", "turns"), com.sce.runtime.ScriptSource.lua("_scxml_add(turns, 1)", "turns + 1"))) {
                return
            }
    }

    private fun transitionContentOfWithinAt1() {
                // SCE-MAP: ai_loop.scxml:525 :: within :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("turns", "turns"), com.sce.runtime.ScriptSource.lua("_scxml_add(turns, 1)", "turns + 1"))) {
                return
            }
    }

    // The content of `working`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWorking(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWorkingAt0()
            else -> {}
        }
    }


    private fun transitionContentOfWorkingAt0() {
                // SCE-MAP: ai_loop.scxml:311 :: working :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("turns_since_reflect", "turns_since_reflect"), com.sce.runtime.ScriptSource.lua("_scxml_add(turns_since_reflect, 1)", "turns_since_reflect + 1"))) {
                return
            }
    }
}
