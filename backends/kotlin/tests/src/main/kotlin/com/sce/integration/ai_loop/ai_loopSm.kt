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
        is AiLoopState.Budget, is AiLoopState.Drive, is AiLoopState.Running, is AiLoopState.Watch -> true
        else -> false
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: AiLoopState): Boolean = when (state) {
        is AiLoopState.Run -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: AiLoopState): Boolean = when (state) {
        is AiLoopState.Abandoned, is AiLoopState.Blocked, is AiLoopState.Cancelled, is AiLoopState.Converged, is AiLoopState.Exhausted, is AiLoopState.Failed, is AiLoopState.Reported, is AiLoopState.Stuck -> true
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
        val historyWhere = HistoryId(0)

        val childStates: Map<AiLoopState, List<AiLoopState>> = mapOf(
            AiLoopState.Budget to listOf(AiLoopState.Within, AiLoopState.Spent),
            AiLoopState.Drive to listOf(AiLoopState.Running, AiLoopState.Paused, AiLoopState.Abandoned),
            AiLoopState.Run to listOf(AiLoopState.Drive, AiLoopState.Watch, AiLoopState.Budget),
            AiLoopState.Running to listOf(AiLoopState.Priming, AiLoopState.Working, AiLoopState.Screening, AiLoopState.Judging, AiLoopState.Reflecting, AiLoopState.Restarting, AiLoopState.Closing, AiLoopState.Reported, AiLoopState.Stuck),
            AiLoopState.Watch to listOf(AiLoopState.Alive, AiLoopState.Rebuilding),
        )

        val initialTargets: Map<AiLoopState, List<EntryTarget<AiLoopState, HistoryId>>> = mapOf(
            AiLoopState.Budget to listOf(StateTarget(AiLoopState.Within)),
            AiLoopState.Drive to listOf(StateTarget(AiLoopState.Running)),
            AiLoopState.Running to listOf(StateTarget(AiLoopState.Priming)),
            AiLoopState.Watch to listOf(StateTarget(AiLoopState.Alive)),
        )

        val documentInitialTargetList: List<EntryTarget<AiLoopState, HistoryId>> =
            listOf(StateTarget(AiLoopState.Run))

        val historyParents: Map<HistoryId, AiLoopState> = mapOf(
            historyWhere to AiLoopState.Running,
        )

        val historyDefaultTargets: Map<HistoryId, List<EntryTarget<AiLoopState, HistoryId>>> = mapOf(
            historyWhere to listOf(StateTarget(AiLoopState.Working)),
        )

        val historiesByParent: Map<AiLoopState, List<Pair<HistoryId, Boolean>>> = mapOf(
            AiLoopState.Running to listOf(historyWhere to false),
        )

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

    // W3C SCXML 5.9: Guard evaluation with error.execution on failure
    //
    // The guard arrives as a `ScriptSource`, not a `String`: it carries the
    // language its text is in, so a machine generated for a Lua engine hands
    // over Lua the build-time frontend produced and one generated for an
    // ECMAScript engine hands over the author's own text — and the engine is
    // never left to guess which it got. The C++ sibling
    // (`process_transition.jinja2`) takes the same argument for the same
    // reason.
    private fun safeEvaluateGuard(guardExpr: com.sce.runtime.ScriptSource): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "a <transition> cond failed to evaluate")
            false
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
    private fun executeAssign(location: com.sce.runtime.ScriptSource, expr: com.sce.runtime.ScriptSource) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<assign> failed")
        }
    }

    // W3C SCXML 5.8: Script block execution
    private fun executeScriptBlock(script: com.sce.runtime.ScriptSource) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raisePlatformError(AiLoopEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AiLoopEvent) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        val eventName = eventNameOf(event) ?: return
        val meta = currentEventMetadata
        // W3C SCXML 5.10.1: C++ classifyEventType — platform events override type
        val effectiveType = when {
            eventName.startsWith("done.") || eventName.startsWith("error.") -> "platform"
            else -> meta.type
        }
        // W3C SCXML 5.10.1: C++ pattern — origin/origintype only for external events
        // Internal events (<raise>) have empty origin; external events (<send>) have session ID
        // W3C SCXML C.1: `_event.origin` is the sender's published
        // `_ioprocessors` location, not its bare session id — and this is the
        // one place that publishes `_event` to the document, so this is where
        // the id becomes a location. The engine keeps the bare id in
        // `EventMetadata.origin` because its session-keyed lookups (`<finalize>`
        // dispatch, cancelled-invoke filtering) match on it; converting at the
        // raise would make one value serve two consumers that need different
        // spellings. The conversion itself lives in
        // `com.sce.runtime.IoProcessors.publishedOrigin`, the port of the
        // `IOProcessorHelper::publishedOrigin` the C++ engines share: a second
        // spelling of the rule is how the backends would stop agreeing.
        val effectiveOrigin = com.sce.runtime.IoProcessors.publishedOrigin(
            if (meta.type == "external") meta.origin.ifEmpty { scriptSessionId ?: "" } else meta.origin
        )
        val effectiveOriginType = if (meta.type == "external") meta.originType.ifEmpty { "http://www.w3.org/TR/scxml/#SCXMLEventProcessor" } else meta.originType
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
                origin = effectiveOrigin,
                originType = effectiveOriginType,
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
    override fun firstEnabledTransition(
        state: AiLoopState,
        event: AiLoopEvent?
    ): EnabledTransition<AiLoopState, HistoryId>? = when (state) {
        is AiLoopState.Alive -> when {
            event is AiLoopEvent.Session.Lost -> transitionAliveAt0
            else -> null
        }
        is AiLoopState.Closing -> when {
            event is AiLoopEvent.Turn.Done -> transitionClosingAt0
            event is AiLoopEvent.Turn.Blocked -> transitionClosingAt1
            else -> null
        }
        is AiLoopState.Drive -> when {
            event is AiLoopEvent.Hold -> transitionDriveAt0
            event is AiLoopEvent.Turn.Interrupted -> transitionDriveAt1
            event is AiLoopEvent.Session.Lost -> transitionDriveAt2
            else -> null
        }
        is AiLoopState.Judging -> when {
            (event is AiLoopEvent.Judge || event is AiLoopEvent.Judge.Begin) && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_truthy(_event.data.done)", "_event.data.done")) -> transitionJudgingAt0
            (event is AiLoopEvent.Judge || event is AiLoopEvent.Judge.Begin) && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(turns_since_reflect >= reflect_every)", "turns_since_reflect >= reflect_every")) -> transitionJudgingAt1
            (event is AiLoopEvent.Judge || event is AiLoopEvent.Judge.Begin) -> transitionJudgingAt2
            else -> null
        }
        is AiLoopState.Paused -> when {
            event is AiLoopEvent.Turn.Done -> transitionPausedAt0
            event is AiLoopEvent.Turn.Interrupted -> transitionPausedAt1
            event is AiLoopEvent.Resume -> transitionPausedAt2
            event is AiLoopEvent.Unattended -> transitionPausedAt3
            else -> null
        }
        is AiLoopState.Priming -> when {
            event is AiLoopEvent.Prompt.Sent -> transitionPrimingAt0
            else -> null
        }
        is AiLoopState.Rebuilding -> when {
            event is AiLoopEvent.Session.Ready -> transitionRebuildingAt0
            else -> null
        }
        is AiLoopState.Reflecting -> when {
            event is AiLoopEvent.Reflect.Applied -> transitionReflectingAt0
            event is AiLoopEvent.Reflect.None -> transitionReflectingAt1
            else -> null
        }
        is AiLoopState.Restarting -> when {
            event is AiLoopEvent.Session.Ready && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(restarts > max_restarts)", "restarts > max_restarts")) -> transitionRestartingAt0
            event is AiLoopEvent.Session.Ready -> transitionRestartingAt1
            else -> null
        }
        is AiLoopState.Run -> when {
            event is AiLoopEvent.Run.Converged -> transitionRunAt0
            event is AiLoopEvent.Run.Exhausted -> transitionRunAt1
            event is AiLoopEvent.Run.Blocked -> transitionRunAt2
            event is AiLoopEvent.Fail -> transitionRunAt3
            event is AiLoopEvent.Cancel -> transitionRunAt4
            else -> null
        }
        is AiLoopState.Screening -> when {
            event is AiLoopEvent.Screen.Matched -> transitionScreeningAt0
            event is AiLoopEvent.Screen.None -> transitionScreeningAt1
            else -> null
        }
        is AiLoopState.Within -> when {
            event is AiLoopEvent.Turn.Done && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_scxml_add(turns, 1) >= max_turns)", "turns + 1 >= max_turns")) -> transitionWithinAt0
            event is AiLoopEvent.Turn.Done -> transitionWithinAt1
            else -> null
        }
        is AiLoopState.Working -> when {
            event is AiLoopEvent.Turn.Done -> transitionWorkingAt0
            event is AiLoopEvent.Turn.Blocked -> transitionWorkingAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: ai_loop.scxml:155 :: _machine
    override fun onEntry(state: AiLoopState, isDefaultEntry: Boolean) {
        when (state) {
            is AiLoopState.Abandoned -> {
                // SCE-MAP: ai_loop.scxml:493 :: abandoned :: _state_body

            raiseInternal(AiLoopEvent.Run.Blocked)
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
            is AiLoopState.Alive -> {
                // SCE-MAP: ai_loop.scxml:506 :: alive :: _state_body
            }
            is AiLoopState.Blocked -> {
                // SCE-MAP: ai_loop.scxml:548 :: blocked :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AiLoopState.Budget -> {
                // SCE-MAP: ai_loop.scxml:520 :: budget :: _state_body
            }
            is AiLoopState.Cancelled -> {
                // SCE-MAP: ai_loop.scxml:547 :: cancelled :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AiLoopState.Closing -> {
                // SCE-MAP: ai_loop.scxml:405 :: closing :: _state_body


            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            run {
                ensureScriptEngine()
                val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val hostParams = mutableMapOf<String, List<String>>()
                try {
                    val v = hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("end_prompt", "end_prompt"))
                    // The param crosses as text, and `toString()` is the
                    // platform's spelling of the value; this is the document's.
                    hostParams["text"] = listOf(valueToWireString(v))
                } catch (_: Exception) {
                    // W3C SCXML 5.7.1: report the failure and omit the name and
                    // the value — the act still happens, without a field the
                    // document could not produce.
                    raisePlatformError(AiLoopEvent.Error.Execution, "<send> <param name='text'> expr failed to evaluate")
                }
                val hostEventName = "prompt.end"
                val hostRequest = HostSendRequest(
                    processorType = "x-sce-host",
                    eventName = hostEventName,
                    target = "",
                    content = "",
                    params = hostParams,
                    sendId = "__send_7"
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
            }
            }
            is AiLoopState.Converged -> {
                // SCE-MAP: ai_loop.scxml:544 :: converged :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AiLoopState.Drive -> {
                // SCE-MAP: ai_loop.scxml:236 :: drive :: _state_body
            }
            is AiLoopState.Exhausted -> {
                // SCE-MAP: ai_loop.scxml:545 :: exhausted :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AiLoopState.Failed -> {
                // SCE-MAP: ai_loop.scxml:546 :: failed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AiLoopState.Judging -> {
                // SCE-MAP: ai_loop.scxml:344 :: judging :: _state_body


            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            run {
                ensureScriptEngine()
                val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val hostParams = mutableMapOf<String, List<String>>()
                try {
                    val v = hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("done_marker", "done_marker"))
                    // The param crosses as text, and `toString()` is the
                    // platform's spelling of the value; this is the document's.
                    hostParams["marker"] = listOf(valueToWireString(v))
                } catch (_: Exception) {
                    // W3C SCXML 5.7.1: report the failure and omit the name and
                    // the value — the act still happens, without a field the
                    // document could not produce.
                    raisePlatformError(AiLoopEvent.Error.Execution, "<send> <param name='marker'> expr failed to evaluate")
                }
                val hostEventName = "judge.begin"
                val hostRequest = HostSendRequest(
                    processorType = "x-sce-host",
                    eventName = hostEventName,
                    target = "",
                    content = "",
                    params = hostParams,
                    sendId = "__send_3"
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
            }
            }
            is AiLoopState.Paused -> {
                // SCE-MAP: ai_loop.scxml:451 :: paused :: _state_body


            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            run {
                val hostParams = mutableMapOf<String, List<String>>()
                val hostEventName = "notify.human"
                val hostRequest = HostSendRequest(
                    processorType = "x-sce-host",
                    eventName = hostEventName,
                    target = "",
                    content = "",
                    params = hostParams,
                    sendId = "__send_8"
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
            }
            }
            is AiLoopState.Priming -> {
                // SCE-MAP: ai_loop.scxml:291 :: priming :: _state_body


            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            run {
                ensureScriptEngine()
                val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val hostParams = mutableMapOf<String, List<String>>()
                try {
                    val v = hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("start_prompt", "start_prompt"))
                    // The param crosses as text, and `toString()` is the
                    // platform's spelling of the value; this is the document's.
                    hostParams["text"] = listOf(valueToWireString(v))
                } catch (_: Exception) {
                    // W3C SCXML 5.7.1: report the failure and omit the name and
                    // the value — the act still happens, without a field the
                    // document could not produce.
                    raisePlatformError(AiLoopEvent.Error.Execution, "<send> <param name='text'> expr failed to evaluate")
                }
                val hostEventName = "prompt.start"
                val hostRequest = HostSendRequest(
                    processorType = "x-sce-host",
                    eventName = hostEventName,
                    target = "",
                    content = "",
                    params = hostParams,
                    sendId = "__send_0"
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
            }
            }
            is AiLoopState.Rebuilding -> {
                // SCE-MAP: ai_loop.scxml:509 :: rebuilding :: _state_body
            }
            is AiLoopState.Reflecting -> {
                // SCE-MAP: ai_loop.scxml:374 :: reflecting :: _state_body


            executeAssign(com.sce.runtime.ScriptSource.lua("turns_since_reflect", "turns_since_reflect"), com.sce.runtime.ScriptSource.lua("0", "0"))


            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            run {
                val hostParams = mutableMapOf<String, List<String>>()
                val hostEventName = "reflect.begin"
                val hostRequest = HostSendRequest(
                    processorType = "x-sce-host",
                    eventName = hostEventName,
                    target = "",
                    content = "",
                    params = hostParams,
                    sendId = "__send_5"
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
            }
            }
            is AiLoopState.Reported -> {
                // SCE-MAP: ai_loop.scxml:426 :: reported :: _state_body

            raiseInternal(AiLoopEvent.Run.Converged)
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(AiLoopEvent.Done.State.Running, EventMetadata.platform())
            }
            is AiLoopState.Restarting -> {
                // SCE-MAP: ai_loop.scxml:394 :: restarting :: _state_body


            executeAssign(com.sce.runtime.ScriptSource.lua("restarts", "restarts"), com.sce.runtime.ScriptSource.lua("_scxml_add(restarts, 1)", "restarts + 1"))


            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            run {
                val hostParams = mutableMapOf<String, List<String>>()
                val hostEventName = "session.replace"
                val hostRequest = HostSendRequest(
                    processorType = "x-sce-host",
                    eventName = hostEventName,
                    target = "",
                    content = "",
                    params = hostParams,
                    sendId = "__send_6"
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
            }
            }
            is AiLoopState.Run -> {
                // SCE-MAP: ai_loop.scxml:233 :: run :: _state_body
            }
            is AiLoopState.Running -> {
                // SCE-MAP: ai_loop.scxml:274 :: running :: _state_body
            }
            is AiLoopState.Screening -> {
                // SCE-MAP: ai_loop.scxml:327 :: screening :: _state_body


            executeAssign(com.sce.runtime.ScriptSource.lua("screened", "screened"), com.sce.runtime.ScriptSource.lua("_scxml_add(screened, 1)", "screened + 1"))


            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            run {
                val hostParams = mutableMapOf<String, List<String>>()
                val hostEventName = "screen.begin"
                val hostRequest = HostSendRequest(
                    processorType = "x-sce-host",
                    eventName = hostEventName,
                    target = "",
                    content = "",
                    params = hostParams,
                    sendId = "__send_1"
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
            }
            }
            is AiLoopState.Spent -> {
                // SCE-MAP: ai_loop.scxml:529 :: spent :: _state_body

            raiseInternal(AiLoopEvent.Run.Exhausted)
            }
            is AiLoopState.Stuck -> {
                // SCE-MAP: ai_loop.scxml:434 :: stuck :: _state_body

            raiseInternal(AiLoopEvent.Run.Exhausted)
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(AiLoopEvent.Done.State.Running, EventMetadata.platform())
            }
            is AiLoopState.Watch -> {
                // SCE-MAP: ai_loop.scxml:505 :: watch :: _state_body
            }
            is AiLoopState.Within -> {
                // SCE-MAP: ai_loop.scxml:521 :: within :: _state_body
            }
            is AiLoopState.Working -> {
                // SCE-MAP: ai_loop.scxml:310 :: working :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: ai_loop.scxml:155 :: _machine
    override fun onExit(state: AiLoopState) {
        when (state) {
            is AiLoopState.Abandoned -> {
                // SCE-MAP: ai_loop.scxml:493 :: abandoned :: _state_body
            }
            is AiLoopState.Alive -> {
                // SCE-MAP: ai_loop.scxml:506 :: alive :: _state_body
            }
            is AiLoopState.Blocked -> {
                // SCE-MAP: ai_loop.scxml:548 :: blocked :: _state_body
            }
            is AiLoopState.Budget -> {
                // SCE-MAP: ai_loop.scxml:520 :: budget :: _state_body
            }
            is AiLoopState.Cancelled -> {
                // SCE-MAP: ai_loop.scxml:547 :: cancelled :: _state_body
            }
            is AiLoopState.Closing -> {
                // SCE-MAP: ai_loop.scxml:405 :: closing :: _state_body
            }
            is AiLoopState.Converged -> {
                // SCE-MAP: ai_loop.scxml:544 :: converged :: _state_body
            }
            is AiLoopState.Drive -> {
                // SCE-MAP: ai_loop.scxml:236 :: drive :: _state_body
            }
            is AiLoopState.Exhausted -> {
                // SCE-MAP: ai_loop.scxml:545 :: exhausted :: _state_body
            }
            is AiLoopState.Failed -> {
                // SCE-MAP: ai_loop.scxml:546 :: failed :: _state_body
            }
            is AiLoopState.Judging -> {
                // SCE-MAP: ai_loop.scxml:344 :: judging :: _state_body
            }
            is AiLoopState.Paused -> {
                // SCE-MAP: ai_loop.scxml:451 :: paused :: _state_body
            }
            is AiLoopState.Priming -> {
                // SCE-MAP: ai_loop.scxml:291 :: priming :: _state_body
            }
            is AiLoopState.Rebuilding -> {
                // SCE-MAP: ai_loop.scxml:509 :: rebuilding :: _state_body
            }
            is AiLoopState.Reflecting -> {
                // SCE-MAP: ai_loop.scxml:374 :: reflecting :: _state_body
            }
            is AiLoopState.Reported -> {
                // SCE-MAP: ai_loop.scxml:426 :: reported :: _state_body
            }
            is AiLoopState.Restarting -> {
                // SCE-MAP: ai_loop.scxml:394 :: restarting :: _state_body
            }
            is AiLoopState.Run -> {
                // SCE-MAP: ai_loop.scxml:233 :: run :: _state_body
            }
            is AiLoopState.Running -> {
                // SCE-MAP: ai_loop.scxml:274 :: running :: _state_body
            }
            is AiLoopState.Screening -> {
                // SCE-MAP: ai_loop.scxml:327 :: screening :: _state_body
            }
            is AiLoopState.Spent -> {
                // SCE-MAP: ai_loop.scxml:529 :: spent :: _state_body
            }
            is AiLoopState.Stuck -> {
                // SCE-MAP: ai_loop.scxml:434 :: stuck :: _state_body
            }
            is AiLoopState.Watch -> {
                // SCE-MAP: ai_loop.scxml:505 :: watch :: _state_body
            }
            is AiLoopState.Within -> {
                // SCE-MAP: ai_loop.scxml:521 :: within :: _state_body
            }
            is AiLoopState.Working -> {
                // SCE-MAP: ai_loop.scxml:310 :: working :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: ai_loop.scxml:155 :: _machine
    override fun executeTransitionContent(source: AiLoopState, transitionIndex: Int) {
        when (source) {
        is AiLoopState.Judging -> when (transitionIndex) {
            2 -> {
                // SCE-MAP: ai_loop.scxml:362 :: judging :: _transition_2


            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            run {
                ensureScriptEngine()
                val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val hostParams = mutableMapOf<String, List<String>>()
                try {
                    val v = hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("turn_prompt", "turn_prompt"))
                    // The param crosses as text, and `toString()` is the
                    // platform's spelling of the value; this is the document's.
                    hostParams["text"] = listOf(valueToWireString(v))
                } catch (_: Exception) {
                    // W3C SCXML 5.7.1: report the failure and omit the name and
                    // the value — the act still happens, without a field the
                    // document could not produce.
                    raisePlatformError(AiLoopEvent.Error.Execution, "<send> <param name='text'> expr failed to evaluate")
                }
                val hostEventName = "prompt.turn"
                val hostRequest = HostSendRequest(
                    processorType = "x-sce-host",
                    eventName = hostEventName,
                    target = "",
                    content = "",
                    params = hostParams,
                    sendId = "__send_2"
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
            }
            }
            else -> {}
        }
        is AiLoopState.Paused -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: ai_loop.scxml:468 :: paused :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("turns_since_reflect", "turns_since_reflect"), com.sce.runtime.ScriptSource.lua("_scxml_add(turns_since_reflect, 1)", "turns_since_reflect + 1"))
            }
            else -> {}
        }
        is AiLoopState.Reflecting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: ai_loop.scxml:379 :: reflecting :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("start_prompt", "start_prompt"), com.sce.runtime.ScriptSource.lua("_event.data.start_prompt", "_event.data.start_prompt"))


            executeAssign(com.sce.runtime.ScriptSource.lua("turn_prompt", "turn_prompt"), com.sce.runtime.ScriptSource.lua("_event.data.turn_prompt", "_event.data.turn_prompt"))


            executeAssign(com.sce.runtime.ScriptSource.lua("milestone", "milestone"), com.sce.runtime.ScriptSource.lua("_event.data.milestone", "_event.data.milestone"))
            }
            1 -> {
                // SCE-MAP: ai_loop.scxml:385 :: reflecting :: _transition_1


            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            run {
                ensureScriptEngine()
                val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val hostParams = mutableMapOf<String, List<String>>()
                try {
                    val v = hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("turn_prompt", "turn_prompt"))
                    // The param crosses as text, and `toString()` is the
                    // platform's spelling of the value; this is the document's.
                    hostParams["text"] = listOf(valueToWireString(v))
                } catch (_: Exception) {
                    // W3C SCXML 5.7.1: report the failure and omit the name and
                    // the value — the act still happens, without a field the
                    // document could not produce.
                    raisePlatformError(AiLoopEvent.Error.Execution, "<send> <param name='text'> expr failed to evaluate")
                }
                val hostEventName = "prompt.turn"
                val hostRequest = HostSendRequest(
                    processorType = "x-sce-host",
                    eventName = hostEventName,
                    target = "",
                    content = "",
                    params = hostParams,
                    sendId = "__send_4"
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
            }
            }
            else -> {}
        }
        is AiLoopState.Within -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: ai_loop.scxml:522 :: within :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("turns", "turns"), com.sce.runtime.ScriptSource.lua("_scxml_add(turns, 1)", "turns + 1"))
            }
            1 -> {
                // SCE-MAP: ai_loop.scxml:525 :: within :: _transition_1


            executeAssign(com.sce.runtime.ScriptSource.lua("turns", "turns"), com.sce.runtime.ScriptSource.lua("_scxml_add(turns, 1)", "turns + 1"))
            }
            else -> {}
        }
        is AiLoopState.Working -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: ai_loop.scxml:311 :: working :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("turns_since_reflect", "turns_since_reflect"), com.sce.runtime.ScriptSource.lua("_scxml_add(turns_since_reflect, 1)", "turns_since_reflect + 1"))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
