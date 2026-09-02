// SCE-GENERATED — DO NOT EDIT
// source-hash: 321b42acfe8cb266c51aff87d805eb471548c8d5250d5f0a5214385ef864d6e9
// template-hash: 85660c1341dd8abf7326f61f4efe828117f6cbaf56814ccb03d3fd81b42e6ed0
// generated-at: 0

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

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: AiLoopState): AiLoopState = when (state) {
        is AiLoopState.Budget -> AiLoopState.Within
        is AiLoopState.Drive -> AiLoopState.Priming
        is AiLoopState.Run -> AiLoopState.Priming
        is AiLoopState.Running -> AiLoopState.Priming
        is AiLoopState.Watch -> AiLoopState.Alive
        else -> state
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

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: AiLoopState): Boolean = when (state) {
        is AiLoopState.Budget -> false
        is AiLoopState.Drive -> false
        is AiLoopState.Run -> false
        is AiLoopState.Running -> false
        is AiLoopState.Watch -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: AiLoopState): Boolean = when (state) {
        is AiLoopState.Run -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: AiLoopState): List<AiLoopState> = when (state) {
        is AiLoopState.Run -> listOf(AiLoopState.Budget, AiLoopState.Drive, AiLoopState.Watch)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
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


    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: AiLoopState,
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        // W3C SCXML 3.13: Ancestor-only routing (abandoned has no own event transitions)
        is AiLoopState.Abandoned -> {
            val anc1 = processDrive(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processRun(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        is AiLoopState.Alive -> {
            val result = processAlive(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processRun(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (budget has no own event transitions)
        is AiLoopState.Budget -> {
            val anc1 = processRun(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is AiLoopState.Closing -> {
            val result = processClosing(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processDrive(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processRun(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
            }
        }
        is AiLoopState.Drive -> {
            val result = processDrive(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processRun(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is AiLoopState.Judging -> {
            val result = processJudging(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processDrive(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processRun(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
            }
        }
        is AiLoopState.Paused -> {
            val result = processPaused(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processDrive(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processRun(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
            }
        }
        is AiLoopState.Priming -> {
            val result = processPriming(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processDrive(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processRun(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
            }
        }
        is AiLoopState.Rebuilding -> {
            val result = processRebuilding(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processRun(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is AiLoopState.Reflecting -> {
            val result = processReflecting(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processDrive(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processRun(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (reported has no own event transitions)
        is AiLoopState.Reported -> {
            val anc1 = processDrive(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processRun(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        is AiLoopState.Restarting -> {
            val result = processRestarting(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processDrive(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processRun(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (running has no own event transitions)
        is AiLoopState.Running -> {
            val anc1 = processDrive(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processRun(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        is AiLoopState.Screening -> {
            val result = processScreening(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processDrive(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processRun(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (spent has no own event transitions)
        is AiLoopState.Spent -> {
            val anc1 = processRun(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (stuck has no own event transitions)
        is AiLoopState.Stuck -> {
            val anc1 = processDrive(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processRun(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (watch has no own event transitions)
        is AiLoopState.Watch -> {
            val anc1 = processRun(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is AiLoopState.Within -> {
            val result = processWithin(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processRun(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is AiLoopState.Working -> {
            val result = processWorking(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processDrive(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processRun(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
            }
        }
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processAlive(
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> = when {
        event is AiLoopEvent.Session.Lost -> TransitionResult.External(AiLoopState.Rebuilding, AiLoopState.Alive, 0)

        else -> TransitionResult.Ignored
    }

    private fun processClosing(
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> = when {
        event is AiLoopEvent.Turn.Done -> TransitionResult.External(AiLoopState.Reported, AiLoopState.Closing, 1)

        event is AiLoopEvent.Turn.Blocked -> TransitionResult.External(AiLoopState.Screening, AiLoopState.Closing, 2)

        else -> TransitionResult.Ignored
    }

    private fun processDrive(
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> = when {
        event is AiLoopEvent.Hold -> TransitionResult.InternalToTarget(AiLoopState.Paused, AiLoopState.Drive, 3)

        event is AiLoopEvent.Turn.Interrupted -> TransitionResult.InternalToTarget(AiLoopState.Paused, AiLoopState.Drive, 4)

        event is AiLoopEvent.Session.Lost -> TransitionResult.InternalToTarget(AiLoopState.Restarting, AiLoopState.Drive, 5)

        else -> TransitionResult.Ignored
    }

    private fun processJudging(
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> = when {
        // W3C SCXML 3.12.1: Prefix match for "judge"
        (event is AiLoopEvent.Judge || event is AiLoopEvent.Judge.Begin) && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_truthy(_event.data.done)", "_event.data.done")) -> TransitionResult.External(AiLoopState.Closing, AiLoopState.Judging, 6)

        // W3C SCXML 3.12.1: Prefix match for "judge"
        (event is AiLoopEvent.Judge || event is AiLoopEvent.Judge.Begin) && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(turns_since_reflect >= reflect_every)", "turns_since_reflect >= reflect_every")) -> TransitionResult.External(AiLoopState.Reflecting, AiLoopState.Judging, 7)

        // W3C SCXML 3.12.1: Prefix match for "judge"
        (event is AiLoopEvent.Judge || event is AiLoopEvent.Judge.Begin) -> TransitionResult.External(AiLoopState.Working, AiLoopState.Judging, 8)

        else -> TransitionResult.Ignored
    }

    private fun processPaused(
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> = when {
        event is AiLoopEvent.Turn.Done -> TransitionResult.External(AiLoopState.Judging, AiLoopState.Paused, 9)

        event is AiLoopEvent.Turn.Interrupted -> TransitionResult.External(AiLoopState.Paused, AiLoopState.Paused, 10)

        event is AiLoopEvent.Resume -> TransitionResult.External((historyStore["where"]?.takeIf { it.isNotEmpty() }?.let { resolveState(it[0]) } ?: AiLoopState.Working), AiLoopState.Paused, 11)

        event is AiLoopEvent.Unattended -> TransitionResult.External(AiLoopState.Abandoned, AiLoopState.Paused, 12)

        else -> TransitionResult.Ignored
    }

    private fun processPriming(
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> = when {
        event is AiLoopEvent.Prompt.Sent -> TransitionResult.External(AiLoopState.Working, AiLoopState.Priming, 13)

        else -> TransitionResult.Ignored
    }

    private fun processRebuilding(
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> = when {
        event is AiLoopEvent.Session.Ready -> TransitionResult.External(AiLoopState.Alive, AiLoopState.Rebuilding, 14)

        else -> TransitionResult.Ignored
    }

    private fun processReflecting(
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> = when {
        event is AiLoopEvent.Reflect.Applied -> TransitionResult.External(AiLoopState.Restarting, AiLoopState.Reflecting, 15)

        event is AiLoopEvent.Reflect.None -> TransitionResult.External(AiLoopState.Working, AiLoopState.Reflecting, 16)

        else -> TransitionResult.Ignored
    }

    private fun processRestarting(
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> = when {
        event is AiLoopEvent.Session.Ready && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(restarts > max_restarts)", "restarts > max_restarts")) -> TransitionResult.External(AiLoopState.Stuck, AiLoopState.Restarting, 17)

        event is AiLoopEvent.Session.Ready -> TransitionResult.External(AiLoopState.Priming, AiLoopState.Restarting, 18)

        else -> TransitionResult.Ignored
    }

    private fun processRun(
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> = when {
        event is AiLoopEvent.Run.Converged -> TransitionResult.External(AiLoopState.Converged, AiLoopState.Run, 19)

        event is AiLoopEvent.Run.Exhausted -> TransitionResult.External(AiLoopState.Exhausted, AiLoopState.Run, 20)

        event is AiLoopEvent.Run.Blocked -> TransitionResult.External(AiLoopState.Blocked, AiLoopState.Run, 21)

        event is AiLoopEvent.Fail -> TransitionResult.External(AiLoopState.Failed, AiLoopState.Run, 22)

        event is AiLoopEvent.Cancel -> TransitionResult.External(AiLoopState.Cancelled, AiLoopState.Run, 23)

        else -> TransitionResult.Ignored
    }

    private fun processScreening(
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> = when {
        event is AiLoopEvent.Screen.Matched -> TransitionResult.External(AiLoopState.Working, AiLoopState.Screening, 24)

        event is AiLoopEvent.Screen.None -> TransitionResult.External(AiLoopState.Paused, AiLoopState.Screening, 25)

        else -> TransitionResult.Ignored
    }

    private fun processWithin(
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> = when {
        event is AiLoopEvent.Turn.Done && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_scxml_add(turns, 1) >= max_turns)", "turns + 1 >= max_turns")) -> TransitionResult.External(AiLoopState.Spent, AiLoopState.Within, 26)

        event is AiLoopEvent.Turn.Done -> TransitionResult.External(AiLoopState.Within, AiLoopState.Within, 27)

        else -> TransitionResult.Ignored
    }

    private fun processWorking(
        event: AiLoopEvent
    ): TransitionResult<AiLoopState> = when {
        event is AiLoopEvent.Turn.Done -> TransitionResult.External(AiLoopState.Judging, AiLoopState.Working, 28)

        event is AiLoopEvent.Turn.Blocked -> TransitionResult.External(AiLoopState.Screening, AiLoopState.Working, 29)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: ai_loop.scxml:155 :: _machine
    override fun onEntry(state: AiLoopState, pathChild: AiLoopState?) {
        when (state) {
            is AiLoopState.Abandoned -> {
                // SCE-MAP: ai_loop.scxml:493 :: abandoned :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("abandoned")) return

            raiseInternal(AiLoopEvent.Run.Blocked)
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(AiLoopEvent.Done.State.Drive, EventMetadata.platform())
                // W3C SCXML 3.7.1: Check if all regions of parallel grandparent are complete
                if (false) {
                    raiseInternal(AiLoopEvent.Done.State.Run)
                }
            }
            is AiLoopState.Alive -> {
                // SCE-MAP: ai_loop.scxml:506 :: alive :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("alive")) return
            }
            is AiLoopState.Blocked -> {
                // SCE-MAP: ai_loop.scxml:548 :: blocked :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("blocked")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AiLoopState.Budget -> {
                // SCE-MAP: ai_loop.scxml:520 :: budget :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("budget")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(AiLoopState.Within)
                }
            }
            is AiLoopState.Cancelled -> {
                // SCE-MAP: ai_loop.scxml:547 :: cancelled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("cancelled")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AiLoopState.Closing -> {
                // SCE-MAP: ai_loop.scxml:405 :: closing :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("closing")) return


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
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("converged")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AiLoopState.Drive -> {
                // SCE-MAP: ai_loop.scxml:236 :: drive :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("drive")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(AiLoopState.Running)
                }
            }
            is AiLoopState.Exhausted -> {
                // SCE-MAP: ai_loop.scxml:545 :: exhausted :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("exhausted")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AiLoopState.Failed -> {
                // SCE-MAP: ai_loop.scxml:546 :: failed :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failed")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AiLoopState.Judging -> {
                // SCE-MAP: ai_loop.scxml:344 :: judging :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("judging")) return


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
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("paused")) return


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
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("priming")) return


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
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("rebuilding")) return
            }
            is AiLoopState.Reflecting -> {
                // SCE-MAP: ai_loop.scxml:374 :: reflecting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("reflecting")) return


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
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("reported")) return

            raiseInternal(AiLoopEvent.Run.Converged)
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(AiLoopEvent.Done.State.Running, EventMetadata.platform())
            }
            is AiLoopState.Restarting -> {
                // SCE-MAP: ai_loop.scxml:394 :: restarting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("restarting")) return


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
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("run")) return
                // W3C SCXML 3.4 + §scxml-D-addDescendantStatesToEnter: a
                // `<parallel>` hands out defaults even when it is only an
                // ancestor — Appendix D's one exception to the ancestor rule.
                // The exception has its own exception: not the region the entry
                // set is already descending into, which `pathChild` names and
                // which the caller enters with the target's own path.
                if (pathChild != AiLoopState.Budget) {
                    onEntry(AiLoopState.Budget)
                }
                if (pathChild != AiLoopState.Drive) {
                    onEntry(AiLoopState.Drive)
                }
                if (pathChild != AiLoopState.Watch) {
                    onEntry(AiLoopState.Watch)
                }
            }
            is AiLoopState.Running -> {
                // SCE-MAP: ai_loop.scxml:274 :: running :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("running")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(AiLoopState.Priming)
                }
            }
            is AiLoopState.Screening -> {
                // SCE-MAP: ai_loop.scxml:327 :: screening :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("screening")) return


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
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("spent")) return

            raiseInternal(AiLoopEvent.Run.Exhausted)
            }
            is AiLoopState.Stuck -> {
                // SCE-MAP: ai_loop.scxml:434 :: stuck :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("stuck")) return

            raiseInternal(AiLoopEvent.Run.Exhausted)
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(AiLoopEvent.Done.State.Running, EventMetadata.platform())
            }
            is AiLoopState.Watch -> {
                // SCE-MAP: ai_loop.scxml:505 :: watch :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("watch")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(AiLoopState.Alive)
                }
            }
            is AiLoopState.Within -> {
                // SCE-MAP: ai_loop.scxml:521 :: within :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("within")) return
            }
            is AiLoopState.Working -> {
                // SCE-MAP: ai_loop.scxml:310 :: working :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("working")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: ai_loop.scxml:155 :: _machine
    override fun onExit(state: AiLoopState) {
        when (state) {
            is AiLoopState.Abandoned -> {
                // SCE-MAP: ai_loop.scxml:493 :: abandoned :: _state_body
                activeStateIds.remove("abandoned")
            }
            is AiLoopState.Alive -> {
                // SCE-MAP: ai_loop.scxml:506 :: alive :: _state_body
                activeStateIds.remove("alive")
            }
            is AiLoopState.Blocked -> {
                // SCE-MAP: ai_loop.scxml:548 :: blocked :: _state_body
                activeStateIds.remove("blocked")
            }
            is AiLoopState.Budget -> {
                // SCE-MAP: ai_loop.scxml:520 :: budget :: _state_body
                activeStateIds.remove("budget")
            }
            is AiLoopState.Cancelled -> {
                // SCE-MAP: ai_loop.scxml:547 :: cancelled :: _state_body
                activeStateIds.remove("cancelled")
            }
            is AiLoopState.Closing -> {
                // SCE-MAP: ai_loop.scxml:405 :: closing :: _state_body
                activeStateIds.remove("closing")
            }
            is AiLoopState.Converged -> {
                // SCE-MAP: ai_loop.scxml:544 :: converged :: _state_body
                activeStateIds.remove("converged")
            }
            is AiLoopState.Drive -> {
                // SCE-MAP: ai_loop.scxml:236 :: drive :: _state_body
                activeStateIds.remove("drive")
            }
            is AiLoopState.Exhausted -> {
                // SCE-MAP: ai_loop.scxml:545 :: exhausted :: _state_body
                activeStateIds.remove("exhausted")
            }
            is AiLoopState.Failed -> {
                // SCE-MAP: ai_loop.scxml:546 :: failed :: _state_body
                activeStateIds.remove("failed")
            }
            is AiLoopState.Judging -> {
                // SCE-MAP: ai_loop.scxml:344 :: judging :: _state_body
                activeStateIds.remove("judging")
            }
            is AiLoopState.Paused -> {
                // SCE-MAP: ai_loop.scxml:451 :: paused :: _state_body
                activeStateIds.remove("paused")
            }
            is AiLoopState.Priming -> {
                // SCE-MAP: ai_loop.scxml:291 :: priming :: _state_body
                activeStateIds.remove("priming")
            }
            is AiLoopState.Rebuilding -> {
                // SCE-MAP: ai_loop.scxml:509 :: rebuilding :: _state_body
                activeStateIds.remove("rebuilding")
            }
            is AiLoopState.Reflecting -> {
                // SCE-MAP: ai_loop.scxml:374 :: reflecting :: _state_body
                activeStateIds.remove("reflecting")
            }
            is AiLoopState.Reported -> {
                // SCE-MAP: ai_loop.scxml:426 :: reported :: _state_body
                activeStateIds.remove("reported")
            }
            is AiLoopState.Restarting -> {
                // SCE-MAP: ai_loop.scxml:394 :: restarting :: _state_body
                activeStateIds.remove("restarting")
            }
            is AiLoopState.Run -> {
                // SCE-MAP: ai_loop.scxml:233 :: run :: _state_body
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<AiLoopState, Int>>()
                if (activeStateIds.contains("budget")) {
                    toExit.add(AiLoopState.Budget to 17)
                }
                if (activeStateIds.contains("spent")) {
                    toExit.add(AiLoopState.Spent to 19)
                }
                if (activeStateIds.contains("within")) {
                    toExit.add(AiLoopState.Within to 18)
                }
                if (activeStateIds.contains("drive")) {
                    toExit.add(AiLoopState.Drive to 1)
                }
                if (activeStateIds.contains("abandoned")) {
                    toExit.add(AiLoopState.Abandoned to 13)
                }
                if (activeStateIds.contains("paused")) {
                    toExit.add(AiLoopState.Paused to 12)
                }
                if (activeStateIds.contains("running")) {
                    toExit.add(AiLoopState.Running to 2)
                }
                if (activeStateIds.contains("closing")) {
                    toExit.add(AiLoopState.Closing to 9)
                }
                if (activeStateIds.contains("judging")) {
                    toExit.add(AiLoopState.Judging to 6)
                }
                if (activeStateIds.contains("priming")) {
                    toExit.add(AiLoopState.Priming to 3)
                }
                if (activeStateIds.contains("reflecting")) {
                    toExit.add(AiLoopState.Reflecting to 7)
                }
                if (activeStateIds.contains("reported")) {
                    toExit.add(AiLoopState.Reported to 10)
                }
                if (activeStateIds.contains("restarting")) {
                    toExit.add(AiLoopState.Restarting to 8)
                }
                if (activeStateIds.contains("screening")) {
                    toExit.add(AiLoopState.Screening to 5)
                }
                if (activeStateIds.contains("stuck")) {
                    toExit.add(AiLoopState.Stuck to 11)
                }
                if (activeStateIds.contains("working")) {
                    toExit.add(AiLoopState.Working to 4)
                }
                if (activeStateIds.contains("watch")) {
                    toExit.add(AiLoopState.Watch to 14)
                }
                if (activeStateIds.contains("alive")) {
                    toExit.add(AiLoopState.Alive to 15)
                }
                if (activeStateIds.contains("rebuilding")) {
                    toExit.add(AiLoopState.Rebuilding to 16)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("run")
            }
            is AiLoopState.Running -> {
                // SCE-MAP: ai_loop.scxml:274 :: running :: _state_body
                // W3C SCXML 3.11: Record shallow history for where
                // Uses preTransitionActiveStates (captured before exits, C++ pattern)
                historyStore["where"] = preTransitionActiveStates.filter { stateId ->
                    val st = resolveState(stateId) ?: return@filter false
                    parentOf(st)?.let { stateIdOf(it) } == "running"
                }.toList()
                activeStateIds.remove("running")
            }
            is AiLoopState.Screening -> {
                // SCE-MAP: ai_loop.scxml:327 :: screening :: _state_body
                activeStateIds.remove("screening")
            }
            is AiLoopState.Spent -> {
                // SCE-MAP: ai_loop.scxml:529 :: spent :: _state_body
                activeStateIds.remove("spent")
            }
            is AiLoopState.Stuck -> {
                // SCE-MAP: ai_loop.scxml:434 :: stuck :: _state_body
                activeStateIds.remove("stuck")
            }
            is AiLoopState.Watch -> {
                // SCE-MAP: ai_loop.scxml:505 :: watch :: _state_body
                activeStateIds.remove("watch")
            }
            is AiLoopState.Within -> {
                // SCE-MAP: ai_loop.scxml:521 :: within :: _state_body
                activeStateIds.remove("within")
            }
            is AiLoopState.Working -> {
                // SCE-MAP: ai_loop.scxml:310 :: working :: _state_body
                activeStateIds.remove("working")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: ai_loop.scxml:155 :: _machine
    override fun executeTransitionActions(
        source: AiLoopState,
        event: AiLoopEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        is AiLoopState.Judging -> when (transitionIndex) {
            8 -> {
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
            9 -> {
                // SCE-MAP: ai_loop.scxml:468 :: paused :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("turns_since_reflect", "turns_since_reflect"), com.sce.runtime.ScriptSource.lua("_scxml_add(turns_since_reflect, 1)", "turns_since_reflect + 1"))
            }
            else -> {}
        }
        is AiLoopState.Reflecting -> when (transitionIndex) {
            15 -> {
                // SCE-MAP: ai_loop.scxml:379 :: reflecting :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("start_prompt", "start_prompt"), com.sce.runtime.ScriptSource.lua("_event.data.start_prompt", "_event.data.start_prompt"))


            executeAssign(com.sce.runtime.ScriptSource.lua("turn_prompt", "turn_prompt"), com.sce.runtime.ScriptSource.lua("_event.data.turn_prompt", "_event.data.turn_prompt"))


            executeAssign(com.sce.runtime.ScriptSource.lua("milestone", "milestone"), com.sce.runtime.ScriptSource.lua("_event.data.milestone", "_event.data.milestone"))
            }
            16 -> {
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
            26 -> {
                // SCE-MAP: ai_loop.scxml:522 :: within :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("turns", "turns"), com.sce.runtime.ScriptSource.lua("_scxml_add(turns, 1)", "turns + 1"))
            }
            27 -> {
                // SCE-MAP: ai_loop.scxml:525 :: within :: _transition_1


            executeAssign(com.sce.runtime.ScriptSource.lua("turns", "turns"), com.sce.runtime.ScriptSource.lua("_scxml_add(turns, 1)", "turns + 1"))
            }
            else -> {}
        }
        is AiLoopState.Working -> when (transitionIndex) {
            28 -> {
                // SCE-MAP: ai_loop.scxml:311 :: working :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("turns_since_reflect", "turns_since_reflect"), com.sce.runtime.ScriptSource.lua("_scxml_add(turns_since_reflect, 1)", "turns_since_reflect + 1"))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
