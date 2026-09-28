// SCE-GENERATED — DO NOT EDIT
// source-hash: 7c5ed4658dd0e6b785296d3c9c5b5736e40be15bd60bff7f86e7f7d0542bd240

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_hybrid_invoke_carries_its_arguments/a_hybrid_invoke_carries_its_arguments.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:40 :: _machine

package com.sce.integration.a_hybrid_invoke_carries_its_arguments

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AHybridInvokeCarriesItsArgumentsState : State {
    data object Done : AHybridInvokeCarriesItsArgumentsState
    data object FailRefusedChildStarted : AHybridInvokeCarriesItsArgumentsState
    data object FailWrongChild : AHybridInvokeCarriesItsArgumentsState
    data object NamelistPhase : AHybridInvokeCarriesItsArgumentsState
    data object ParamsPhase : AHybridInvokeCarriesItsArgumentsState
    data object RefusedPhase : AHybridInvokeCarriesItsArgumentsState
    data object Run : AHybridInvokeCarriesItsArgumentsState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AHybridInvokeCarriesItsArgumentsEvent : Event {
    data object ChildUp : AHybridInvokeCarriesItsArgumentsEvent
    sealed interface Error : AHybridInvokeCarriesItsArgumentsEvent {
        data object Execution : Error
    }
    data object WrongChild : AHybridInvokeCarriesItsArgumentsEvent
}
// --- State Machine (W3C SCXML) ---

class AHybridInvokeCarriesItsArgumentsStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<AHybridInvokeCarriesItsArgumentsState, AHybridInvokeCarriesItsArgumentsEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `pick` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `pick` was assigned a value of another type, or the engine refused.
     */
    fun pick(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "pick")

    /**
     * §scxml-5.3: what the `v` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `v` was assigned a value of another type, or the engine refused.
     */
    fun v(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "v")

    /**
     * §scxml-5.3: what the `seed` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `seed` was assigned a value of another type, or the engine refused.
     */
    fun seed(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "seed")

    /**
     * §scxml-5.3: what the `errors` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `errors` was assigned a value of another type, or the engine refused.
     */
    fun errors(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "errors")

    /**
     * §scxml-5.3: what the `started` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `started` was assigned a value of another type, or the engine refused.
     */
    fun started(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "started")

    /**
     * §scxml-5.3: what the `paramsOk` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `paramsOk` was assigned a value of another type, or the engine refused.
     */
    fun paramsOk(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "paramsOk")

    /**
     * §scxml-5.3: what the `namelistOk` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `namelistOk` was assigned a value of another type, or the engine refused.
     */
    fun namelistOk(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "namelistOk")

    override val initialState: AHybridInvokeCarriesItsArgumentsState = AHybridInvokeCarriesItsArgumentsState.ParamsPhase

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

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
    override fun parentOf(state: AHybridInvokeCarriesItsArgumentsState): AHybridInvokeCarriesItsArgumentsState? = when (state) {
        is AHybridInvokeCarriesItsArgumentsState.NamelistPhase -> AHybridInvokeCarriesItsArgumentsState.Run
        is AHybridInvokeCarriesItsArgumentsState.ParamsPhase -> AHybridInvokeCarriesItsArgumentsState.Run
        is AHybridInvokeCarriesItsArgumentsState.RefusedPhase -> AHybridInvokeCarriesItsArgumentsState.Run
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: AHybridInvokeCarriesItsArgumentsState): Boolean = when (state) {
        is AHybridInvokeCarriesItsArgumentsState.Run -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: AHybridInvokeCarriesItsArgumentsState): Boolean = when (state) {
        is AHybridInvokeCarriesItsArgumentsState.Done, is AHybridInvokeCarriesItsArgumentsState.FailRefusedChildStarted, is AHybridInvokeCarriesItsArgumentsState.FailWrongChild -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: AHybridInvokeCarriesItsArgumentsState): List<AHybridInvokeCarriesItsArgumentsState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: AHybridInvokeCarriesItsArgumentsState): List<EntryTarget<AHybridInvokeCarriesItsArgumentsState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AHybridInvokeCarriesItsArgumentsState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<AHybridInvokeCarriesItsArgumentsState, List<AHybridInvokeCarriesItsArgumentsState>> = mapOf(
            AHybridInvokeCarriesItsArgumentsState.Run to listOf(AHybridInvokeCarriesItsArgumentsState.ParamsPhase, AHybridInvokeCarriesItsArgumentsState.NamelistPhase, AHybridInvokeCarriesItsArgumentsState.RefusedPhase),
        )

        val initialTargets: Map<AHybridInvokeCarriesItsArgumentsState, List<EntryTarget<AHybridInvokeCarriesItsArgumentsState, HistoryId>>> = mapOf(
            AHybridInvokeCarriesItsArgumentsState.Run to listOf(StateTarget(AHybridInvokeCarriesItsArgumentsState.ParamsPhase)),
        )

        val documentInitialTargetList: List<EntryTarget<AHybridInvokeCarriesItsArgumentsState, HistoryId>> =
            listOf(StateTarget(AHybridInvokeCarriesItsArgumentsState.Run))

        // W3C SCXML 3.13: namelistPhase's transition 0, as the microstep reads it.
        val transitionNamelistPhaseAt0 = EnabledTransition<AHybridInvokeCarriesItsArgumentsState, HistoryId>(
            AHybridInvokeCarriesItsArgumentsState.NamelistPhase,
            listOf(StateTarget(AHybridInvokeCarriesItsArgumentsState.RefusedPhase)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: paramsPhase's transition 0, as the microstep reads it.
        val transitionParamsPhaseAt0 = EnabledTransition<AHybridInvokeCarriesItsArgumentsState, HistoryId>(
            AHybridInvokeCarriesItsArgumentsState.ParamsPhase,
            listOf(StateTarget(AHybridInvokeCarriesItsArgumentsState.NamelistPhase)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: refusedPhase's transition 0, as the microstep reads it.
        val transitionRefusedPhaseAt0 = EnabledTransition<AHybridInvokeCarriesItsArgumentsState, HistoryId>(
            AHybridInvokeCarriesItsArgumentsState.RefusedPhase,
            listOf(StateTarget(AHybridInvokeCarriesItsArgumentsState.Done)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: refusedPhase's transition 1, as the microstep reads it.
        val transitionRefusedPhaseAt1 = EnabledTransition<AHybridInvokeCarriesItsArgumentsState, HistoryId>(
            AHybridInvokeCarriesItsArgumentsState.RefusedPhase,
            listOf(StateTarget(AHybridInvokeCarriesItsArgumentsState.FailRefusedChildStarted)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<AHybridInvokeCarriesItsArgumentsState, HistoryId>(
            AHybridInvokeCarriesItsArgumentsState.Run,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 1, as the microstep reads it.
        val transitionRunAt1 = EnabledTransition<AHybridInvokeCarriesItsArgumentsState, HistoryId>(
            AHybridInvokeCarriesItsArgumentsState.Run,
            listOf(StateTarget(AHybridInvokeCarriesItsArgumentsState.FailWrongChild)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AHybridInvokeCarriesItsArgumentsState? = when (stateId) {
        "done" -> AHybridInvokeCarriesItsArgumentsState.Done
        "failRefusedChildStarted" -> AHybridInvokeCarriesItsArgumentsState.FailRefusedChildStarted
        "failWrongChild" -> AHybridInvokeCarriesItsArgumentsState.FailWrongChild
        "namelistPhase" -> AHybridInvokeCarriesItsArgumentsState.NamelistPhase
        "paramsPhase" -> AHybridInvokeCarriesItsArgumentsState.ParamsPhase
        "refusedPhase" -> AHybridInvokeCarriesItsArgumentsState.RefusedPhase
        "run" -> AHybridInvokeCarriesItsArgumentsState.Run
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AHybridInvokeCarriesItsArgumentsState): String = when (state) {
        is AHybridInvokeCarriesItsArgumentsState.Done -> "done"
        is AHybridInvokeCarriesItsArgumentsState.FailRefusedChildStarted -> "failRefusedChildStarted"
        is AHybridInvokeCarriesItsArgumentsState.FailWrongChild -> "failWrongChild"
        is AHybridInvokeCarriesItsArgumentsState.NamelistPhase -> "namelistPhase"
        is AHybridInvokeCarriesItsArgumentsState.ParamsPhase -> "paramsPhase"
        is AHybridInvokeCarriesItsArgumentsState.RefusedPhase -> "refusedPhase"
        is AHybridInvokeCarriesItsArgumentsState.Run -> "run"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AHybridInvokeCarriesItsArgumentsState): Int = when (state) {
        is AHybridInvokeCarriesItsArgumentsState.Done -> 4
        is AHybridInvokeCarriesItsArgumentsState.FailRefusedChildStarted -> 6
        is AHybridInvokeCarriesItsArgumentsState.FailWrongChild -> 5
        is AHybridInvokeCarriesItsArgumentsState.NamelistPhase -> 2
        is AHybridInvokeCarriesItsArgumentsState.ParamsPhase -> 1
        is AHybridInvokeCarriesItsArgumentsState.RefusedPhase -> 3
        is AHybridInvokeCarriesItsArgumentsState.Run -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AHybridInvokeCarriesItsArgumentsEvent? = when (name) {
        "childUp" -> AHybridInvokeCarriesItsArgumentsEvent.ChildUp
        "error.execution" -> AHybridInvokeCarriesItsArgumentsEvent.Error.Execution
        "wrongChild" -> AHybridInvokeCarriesItsArgumentsEvent.WrongChild
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AHybridInvokeCarriesItsArgumentsEvent): String? = when (event) {
        is AHybridInvokeCarriesItsArgumentsEvent.ChildUp -> "childUp"
        is AHybridInvokeCarriesItsArgumentsEvent.Error.Execution -> "error.execution"
        is AHybridInvokeCarriesItsArgumentsEvent.WrongChild -> "wrongChild"
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
            "a_hybrid_invoke_carries_its_arguments",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'pick' with expr
        try {
            val initResult_pick = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"file:keeper.scxml\"", "'file:keeper.scxml'"))
            engine.setVariable(sid, "pick", initResult_pick)
        } catch (e: Exception) {
            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<data id='pick'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'v' with expr
        try {
            val initResult_v = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("7", "7"))
            engine.setVariable(sid, "v", initResult_v)
        } catch (e: Exception) {
            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<data id='v'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'seed' with expr
        try {
            val initResult_seed = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("41", "41"))
            engine.setVariable(sid, "seed", initResult_seed)
        } catch (e: Exception) {
            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<data id='seed'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'errors' with expr
        try {
            val initResult_errors = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "errors", initResult_errors)
        } catch (e: Exception) {
            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<data id='errors'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'started' with expr
        try {
            val initResult_started = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "started", initResult_started)
        } catch (e: Exception) {
            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<data id='started'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'paramsOk' with expr
        try {
            val initResult_paramsOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "paramsOk", initResult_paramsOk)
        } catch (e: Exception) {
            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<data id='paramsOk'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'namelistOk' with expr
        try {
            val initResult_namelistOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "namelistOk", initResult_namelistOk)
        } catch (e: Exception) {
            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<data id='namelistOk'> expr failed to evaluate")
        }




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
            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AHybridInvokeCarriesItsArgumentsEvent) {
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
    override fun bindCurrentEvent(event: AHybridInvokeCarriesItsArgumentsEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AHybridInvokeCarriesItsArgumentsState,
        event: AHybridInvokeCarriesItsArgumentsEvent?
    ): EnabledTransition<AHybridInvokeCarriesItsArgumentsState, HistoryId>? = when (state) {
        is AHybridInvokeCarriesItsArgumentsState.NamelistPhase -> when {
            event is AHybridInvokeCarriesItsArgumentsEvent.ChildUp -> transitionNamelistPhaseAt0
            else -> null
        }
        is AHybridInvokeCarriesItsArgumentsState.ParamsPhase -> when {
            event is AHybridInvokeCarriesItsArgumentsEvent.ChildUp -> transitionParamsPhaseAt0
            else -> null
        }
        is AHybridInvokeCarriesItsArgumentsState.RefusedPhase -> when {
            event is AHybridInvokeCarriesItsArgumentsEvent.Error.Execution -> transitionRefusedPhaseAt0
            event is AHybridInvokeCarriesItsArgumentsEvent.ChildUp -> transitionRefusedPhaseAt1
            else -> null
        }
        is AHybridInvokeCarriesItsArgumentsState.Run -> when {
            event is AHybridInvokeCarriesItsArgumentsEvent.Error.Execution -> transitionRunAt0
            event is AHybridInvokeCarriesItsArgumentsEvent.WrongChild -> transitionRunAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:40 :: _machine
    override fun onEntry(state: AHybridInvokeCarriesItsArgumentsState, isDefaultEntry: Boolean) {
        when (state) {
            is AHybridInvokeCarriesItsArgumentsState.Done -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:100 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AHybridInvokeCarriesItsArgumentsState.FailRefusedChildStarted -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:102 :: failRefusedChildStarted :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AHybridInvokeCarriesItsArgumentsState.FailWrongChild -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:101 :: failWrongChild :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AHybridInvokeCarriesItsArgumentsState.NamelistPhase -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:76 :: namelistPhase :: _state_body
                // W3C SCXML 6.4: Hybrid invoke — runtime expression evaluation + dynamic child
                // C++ parity: StateMachine::createFromSCXMLString() / FileLoadingHelper::loadScxmlFile()
                run {
                    val generatedInvokeId = "namelistPhase.${System.identityHashCode(this)}._invoke_1"
                    deferInvoke(state, generatedInvokeId) {
                        ensureScriptEngine()
                        val eng = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        try {
                            // W3C SCXML 6.4.3: Evaluate srcexpr → file path → load SCXML → create child.
                            // The evaluation is its own try: the clause it answers is
                            // "the expression could not be evaluated", and until this
                            // split that fact shared a catch — and a wording — with
                            // "the child could not be started". A `null` result took
                            // neither path and returned silently.
                            val filePath = try {
                                eng.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("pick", "pick"))?.toString()
                            } catch (_: Exception) {
                                null
                            }
                            if (filePath == null) {
                                raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke srcexpr='pick'> could not be evaluated")
                                return@deferInvoke
                            }
                            // §scxml-6.4 + SCE_ACCEPTED_SUBSET.md §2.13: the value
                            // chooses among the declared candidates, matched on the
                            // document stem so `file:x.scxml` and `./x.scxml` name
                            // one child. `startInvoke` takes a star-projected
                            // machine, so only the construction differs here.
                            val __sceSelected = filePath
                                .substringAfterLast('/')
                                .substringAfterLast('\\')
                                .removePrefix("file:")
                                .substringBeforeLast('.')
                            val childSM = when (__sceSelected) {
                                "keeper" -> KeeperStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                                "bare" -> BareStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                                else -> {
                                    // The failure the Interpreter reports when a
                                    // document will not load: nothing to create.
                                    raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke srcexpr='pick'> evaluated to a document it did not declare")
                                    return@deferInvoke
                                }
                            }
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        ensureScriptEngine()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        if (!(engineInv.hasVariable(sidInv, "seed"))) {
                            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke> namelist names a location that cannot be read")
                            return@deferInvoke
                        }
                        invokeParams["seed"] = engineInv.getVariable(sidInv, "seed")

                            setInvokeParams(childSM, invokeParams)
                            startInvoke("_invoke_1", childSM, false, null, "", generatedInvokeId)
                        } catch (_: Exception) {
                            // W3C SCXML 6.4: the child could not be started. Evaluation
                            // failure no longer reaches here — it has its own raise above,
                            // under the one wording every emitter uses for that fact.
                            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke> could not start a child")
                        }
                    }
                }
            }
            is AHybridInvokeCarriesItsArgumentsState.ParamsPhase -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:60 :: paramsPhase :: _state_body
                // W3C SCXML 6.4: Hybrid invoke — runtime expression evaluation + dynamic child
                // C++ parity: StateMachine::createFromSCXMLString() / FileLoadingHelper::loadScxmlFile()
                run {
                    val generatedInvokeId = "paramsPhase.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        ensureScriptEngine()
                        val eng = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        try {
                            // W3C SCXML 6.4.3: Evaluate srcexpr → file path → load SCXML → create child.
                            // The evaluation is its own try: the clause it answers is
                            // "the expression could not be evaluated", and until this
                            // split that fact shared a catch — and a wording — with
                            // "the child could not be started". A `null` result took
                            // neither path and returned silently.
                            val filePath = try {
                                eng.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("pick", "pick"))?.toString()
                            } catch (_: Exception) {
                                null
                            }
                            if (filePath == null) {
                                raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke srcexpr='pick'> could not be evaluated")
                                return@deferInvoke
                            }
                            // §scxml-6.4 + SCE_ACCEPTED_SUBSET.md §2.13: the value
                            // chooses among the declared candidates, matched on the
                            // document stem so `file:x.scxml` and `./x.scxml` name
                            // one child. `startInvoke` takes a star-projected
                            // machine, so only the construction differs here.
                            val __sceSelected = filePath
                                .substringAfterLast('/')
                                .substringAfterLast('\\')
                                .removePrefix("file:")
                                .substringBeforeLast('.')
                            val childSM = when (__sceSelected) {
                                "keeper" -> KeeperStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                                "bare" -> BareStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                                else -> {
                                    // The failure the Interpreter reports when a
                                    // document will not load: nothing to create.
                                    raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke srcexpr='pick'> evaluated to a document it did not declare")
                                    return@deferInvoke
                                }
                            }
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        ensureScriptEngine()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["seed"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("_scxml_add(seed, 1)", "seed + 1"))
                        } catch (_: Exception) {
                            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke> <param name='seed'> could not be read")
                        }
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["fromLoc"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("v", "v"))
                        } catch (_: Exception) {
                            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke> <param name='fromLoc'> could not be read")
                        }
                        raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke> <param name='emptyLoc'> names no location")
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["nowhere"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("\"leaked\"", "'leaked'"))
                        } catch (_: Exception) {
                            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke> <param name='nowhere'> could not be read")
                        }

                            setInvokeParams(childSM, invokeParams)
                            startInvoke("_invoke_0", childSM, false, null, "", generatedInvokeId)
                        } catch (_: Exception) {
                            // W3C SCXML 6.4: the child could not be started. Evaluation
                            // failure no longer reaches here — it has its own raise above,
                            // under the one wording every emitter uses for that fact.
                            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke> could not start a child")
                        }
                    }
                }
            }
            is AHybridInvokeCarriesItsArgumentsState.RefusedPhase -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:87 :: refusedPhase :: _state_body
                // W3C SCXML 6.4: Hybrid invoke — runtime expression evaluation + dynamic child
                // C++ parity: StateMachine::createFromSCXMLString() / FileLoadingHelper::loadScxmlFile()
                run {
                    val generatedInvokeId = "refusedPhase.${System.identityHashCode(this)}._invoke_2"
                    deferInvoke(state, generatedInvokeId) {
                        ensureScriptEngine()
                        val eng = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        try {
                            // W3C SCXML 6.4.3: Evaluate srcexpr → file path → load SCXML → create child.
                            // The evaluation is its own try: the clause it answers is
                            // "the expression could not be evaluated", and until this
                            // split that fact shared a catch — and a wording — with
                            // "the child could not be started". A `null` result took
                            // neither path and returned silently.
                            val filePath = try {
                                eng.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("pick", "pick"))?.toString()
                            } catch (_: Exception) {
                                null
                            }
                            if (filePath == null) {
                                raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke srcexpr='pick'> could not be evaluated")
                                return@deferInvoke
                            }
                            // §scxml-6.4 + SCE_ACCEPTED_SUBSET.md §2.13: the value
                            // chooses among the declared candidates, matched on the
                            // document stem so `file:x.scxml` and `./x.scxml` name
                            // one child. `startInvoke` takes a star-projected
                            // machine, so only the construction differs here.
                            val __sceSelected = filePath
                                .substringAfterLast('/')
                                .substringAfterLast('\\')
                                .removePrefix("file:")
                                .substringBeforeLast('.')
                            val childSM = when (__sceSelected) {
                                "keeper" -> KeeperStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                                "bare" -> BareStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                                else -> {
                                    // The failure the Interpreter reports when a
                                    // document will not load: nothing to create.
                                    raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke srcexpr='pick'> evaluated to a document it did not declare")
                                    return@deferInvoke
                                }
                            }
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        ensureScriptEngine()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        if (!(engineInv.hasVariable(sidInv, "undeclaredA"))) {
                            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke> namelist names a location that cannot be read")
                            return@deferInvoke
                        }
                        invokeParams["undeclaredA"] = engineInv.getVariable(sidInv, "undeclaredA")
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["seed"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("seed", "seed"))
                        } catch (_: Exception) {
                            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke> <param name='seed'> could not be read")
                        }

                            setInvokeParams(childSM, invokeParams)
                            startInvoke("_invoke_2", childSM, false, null, "", generatedInvokeId)
                        } catch (_: Exception) {
                            // W3C SCXML 6.4: the child could not be started. Evaluation
                            // failure no longer reaches here — it has its own raise above,
                            // under the one wording every emitter uses for that fact.
                            raisePlatformError(AHybridInvokeCarriesItsArgumentsEvent.Error.Execution, "<invoke> could not start a child")
                        }
                    }
                }
            }
            is AHybridInvokeCarriesItsArgumentsState.Run -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:54 :: run :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:40 :: _machine
    override fun onExit(state: AHybridInvokeCarriesItsArgumentsState) {
        when (state) {
            is AHybridInvokeCarriesItsArgumentsState.Done -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:100 :: done :: _state_body
            }
            is AHybridInvokeCarriesItsArgumentsState.FailRefusedChildStarted -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:102 :: failRefusedChildStarted :: _state_body
            }
            is AHybridInvokeCarriesItsArgumentsState.FailWrongChild -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:101 :: failWrongChild :: _state_body
            }
            is AHybridInvokeCarriesItsArgumentsState.NamelistPhase -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:76 :: namelistPhase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_1")
            }
            is AHybridInvokeCarriesItsArgumentsState.ParamsPhase -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:60 :: paramsPhase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            is AHybridInvokeCarriesItsArgumentsState.RefusedPhase -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:87 :: refusedPhase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_2")
            }
            is AHybridInvokeCarriesItsArgumentsState.Run -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:54 :: run :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:40 :: _machine
    override fun executeTransitionContent(source: AHybridInvokeCarriesItsArgumentsState, transitionIndex: Int) {
        when (source) {
        is AHybridInvokeCarriesItsArgumentsState.NamelistPhase -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:79 :: namelistPhase :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("started", "started"), com.sce.runtime.ScriptSource.lua("_scxml_add(started, 1)", "started + 1"))) {
                return
            }


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(((_event.data.seed == 41) and (_event.data.fromLoc == \"unset\")) and (_event.data.leak == \"undefined\"))", "_event.data.seed === 41 && _event.data.fromLoc === 'unset' && _event.data.leak === 'undefined'"))) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("namelistOk", "namelistOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            }
            else -> {}
        }
        is AHybridInvokeCarriesItsArgumentsState.ParamsPhase -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:68 :: paramsPhase :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("started", "started"), com.sce.runtime.ScriptSource.lua("_scxml_add(started, 1)", "started + 1"))) {
                return
            }


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("((((_event.data.seed == 42) and (_event.data.fromLoc == 7)) and (_event.data.emptyLoc == \"unset\")) and (_event.data.leak == \"undefined\"))", "_event.data.seed === 42 && _event.data.fromLoc === 7 && _event.data.emptyLoc === 'unset' && _event.data.leak === 'undefined'"))) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("paramsOk", "paramsOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            }
            else -> {}
        }
        is AHybridInvokeCarriesItsArgumentsState.RefusedPhase -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:93 :: refusedPhase :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("errors", "errors"), com.sce.runtime.ScriptSource.lua("_scxml_add(errors, 1)", "errors + 1"))) {
                return
            }
            }
            else -> {}
        }
        is AHybridInvokeCarriesItsArgumentsState.Run -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_hybrid_invoke_carries_its_arguments.scxml:55 :: run :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("errors", "errors"), com.sce.runtime.ScriptSource.lua("_scxml_add(errors, 1)", "errors + 1"))) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
