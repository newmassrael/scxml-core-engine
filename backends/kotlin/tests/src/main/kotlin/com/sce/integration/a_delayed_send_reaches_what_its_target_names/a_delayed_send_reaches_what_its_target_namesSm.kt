// SCE-GENERATED — DO NOT EDIT
// source-hash: f55f0d50f2ff05a8b6dba1e27e40d55e81b13b70ab29ebc5ee03738d4ee0ecd4

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_delayed_send_reaches_what_its_target_names/a_delayed_send_reaches_what_its_target_names.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:51 :: _machine

package com.sce.integration.a_delayed_send_reaches_what_its_target_names

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ADelayedSendReachesWhatItsTargetNamesState : State {
    data object Done : ADelayedSendReachesWhatItsTargetNamesState
    data object Run : ADelayedSendReachesWhatItsTargetNamesState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ADelayedSendReachesWhatItsTargetNamesEvent : Event {
    sealed interface Done : ADelayedSendReachesWhatItsTargetNamesEvent {
        data object Invoke : Done
    }
    data object Early : ADelayedSendReachesWhatItsTargetNamesEvent
    sealed interface Error : ADelayedSendReachesWhatItsTargetNamesEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Hello : ADelayedSendReachesWhatItsTargetNamesEvent
    data object Inner : ADelayedSendReachesWhatItsTargetNamesEvent
    data object Late : ADelayedSendReachesWhatItsTargetNamesEvent
    data object Lost : ADelayedSendReachesWhatItsTargetNamesEvent
    data object LostReached : ADelayedSendReachesWhatItsTargetNamesEvent
    data object Ping : ADelayedSendReachesWhatItsTargetNamesEvent
    data object Pong : ADelayedSendReachesWhatItsTargetNamesEvent
    data object Settle : ADelayedSendReachesWhatItsTargetNamesEvent
    data object Stop : ADelayedSendReachesWhatItsTargetNamesEvent
}
// --- State Machine (W3C SCXML) ---

class ADelayedSendReachesWhatItsTargetNamesStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ADelayedSendReachesWhatItsTargetNamesState, ADelayedSendReachesWhatItsTargetNamesEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `order` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `order` was assigned a value of another type, or the engine refused.
     */
    fun order(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "order")

    /**
     * §scxml-5.3: what the `innerInternal` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `innerInternal` was assigned a value of another type, or the engine refused.
     */
    fun innerInternal(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "innerInternal")

    /**
     * §scxml-5.3: what the `lateOk` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `lateOk` was assigned a value of another type, or the engine refused.
     */
    fun lateOk(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "lateOk")

    /**
     * §scxml-5.3: what the `lateCount` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `lateCount` was assigned a value of another type, or the engine refused.
     */
    fun lateCount(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "lateCount")

    /**
     * §scxml-5.3: what the `pongOk` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `pongOk` was assigned a value of another type, or the engine refused.
     */
    fun pongOk(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "pongOk")

    /**
     * §scxml-5.3: what the `commErrors` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `commErrors` was assigned a value of another type, or the engine refused.
     */
    fun commErrors(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "commErrors")

    /**
     * §scxml-5.3: what the `lostArrived` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `lostArrived` was assigned a value of another type, or the engine refused.
     */
    fun lostArrived(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "lostArrived")

    /**
     * §scxml-5.3: what the `afterStranger` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterStranger` was assigned a value of another type, or the engine refused.
     */
    fun afterStranger(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterStranger")

    /**
     * §scxml-5.3: what the `hellos` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `hellos` was assigned a value of another type, or the engine refused.
     */
    fun hellos(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "hellos")

    override val initialState: ADelayedSendReachesWhatItsTargetNamesState = ADelayedSendReachesWhatItsTargetNamesState.Run

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

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: ADelayedSendReachesWhatItsTargetNamesState): Boolean = when (state) {
        is ADelayedSendReachesWhatItsTargetNamesState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ADelayedSendReachesWhatItsTargetNamesState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<ADelayedSendReachesWhatItsTargetNamesState, HistoryId>> =
            listOf(StateTarget(ADelayedSendReachesWhatItsTargetNamesState.Run))

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<ADelayedSendReachesWhatItsTargetNamesState, HistoryId>(
            ADelayedSendReachesWhatItsTargetNamesState.Run,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 1, as the microstep reads it.
        val transitionRunAt1 = EnabledTransition<ADelayedSendReachesWhatItsTargetNamesState, HistoryId>(
            ADelayedSendReachesWhatItsTargetNamesState.Run,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 2, as the microstep reads it.
        val transitionRunAt2 = EnabledTransition<ADelayedSendReachesWhatItsTargetNamesState, HistoryId>(
            ADelayedSendReachesWhatItsTargetNamesState.Run,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 3, as the microstep reads it.
        val transitionRunAt3 = EnabledTransition<ADelayedSendReachesWhatItsTargetNamesState, HistoryId>(
            ADelayedSendReachesWhatItsTargetNamesState.Run,
            emptyList(),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 4, as the microstep reads it.
        val transitionRunAt4 = EnabledTransition<ADelayedSendReachesWhatItsTargetNamesState, HistoryId>(
            ADelayedSendReachesWhatItsTargetNamesState.Run,
            emptyList(),
            4,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 5, as the microstep reads it.
        val transitionRunAt5 = EnabledTransition<ADelayedSendReachesWhatItsTargetNamesState, HistoryId>(
            ADelayedSendReachesWhatItsTargetNamesState.Run,
            emptyList(),
            5,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 6, as the microstep reads it.
        val transitionRunAt6 = EnabledTransition<ADelayedSendReachesWhatItsTargetNamesState, HistoryId>(
            ADelayedSendReachesWhatItsTargetNamesState.Run,
            emptyList(),
            6,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 7, as the microstep reads it.
        val transitionRunAt7 = EnabledTransition<ADelayedSendReachesWhatItsTargetNamesState, HistoryId>(
            ADelayedSendReachesWhatItsTargetNamesState.Run,
            emptyList(),
            7,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 8, as the microstep reads it.
        val transitionRunAt8 = EnabledTransition<ADelayedSendReachesWhatItsTargetNamesState, HistoryId>(
            ADelayedSendReachesWhatItsTargetNamesState.Run,
            listOf(StateTarget(ADelayedSendReachesWhatItsTargetNamesState.Done)),
            8,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ADelayedSendReachesWhatItsTargetNamesState? = when (stateId) {
        "done" -> ADelayedSendReachesWhatItsTargetNamesState.Done
        "run" -> ADelayedSendReachesWhatItsTargetNamesState.Run
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ADelayedSendReachesWhatItsTargetNamesState): String = when (state) {
        is ADelayedSendReachesWhatItsTargetNamesState.Done -> "done"
        is ADelayedSendReachesWhatItsTargetNamesState.Run -> "run"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ADelayedSendReachesWhatItsTargetNamesState): Int = when (state) {
        is ADelayedSendReachesWhatItsTargetNamesState.Done -> 1
        is ADelayedSendReachesWhatItsTargetNamesState.Run -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ADelayedSendReachesWhatItsTargetNamesEvent? = when (name) {
        "done.invoke" -> ADelayedSendReachesWhatItsTargetNamesEvent.Done.Invoke
        "early" -> ADelayedSendReachesWhatItsTargetNamesEvent.Early
        "error.communication" -> ADelayedSendReachesWhatItsTargetNamesEvent.Error.Communication
        "error.execution" -> ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution
        "hello" -> ADelayedSendReachesWhatItsTargetNamesEvent.Hello
        "inner" -> ADelayedSendReachesWhatItsTargetNamesEvent.Inner
        "late" -> ADelayedSendReachesWhatItsTargetNamesEvent.Late
        "lost" -> ADelayedSendReachesWhatItsTargetNamesEvent.Lost
        "lostReached" -> ADelayedSendReachesWhatItsTargetNamesEvent.LostReached
        "ping" -> ADelayedSendReachesWhatItsTargetNamesEvent.Ping
        "pong" -> ADelayedSendReachesWhatItsTargetNamesEvent.Pong
        "settle" -> ADelayedSendReachesWhatItsTargetNamesEvent.Settle
        "stop" -> ADelayedSendReachesWhatItsTargetNamesEvent.Stop
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ADelayedSendReachesWhatItsTargetNamesEvent): String? = when (event) {
        is ADelayedSendReachesWhatItsTargetNamesEvent.Done.Invoke -> "done.invoke"
        is ADelayedSendReachesWhatItsTargetNamesEvent.Early -> "early"
        is ADelayedSendReachesWhatItsTargetNamesEvent.Error.Communication -> "error.communication"
        is ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution -> "error.execution"
        is ADelayedSendReachesWhatItsTargetNamesEvent.Hello -> "hello"
        is ADelayedSendReachesWhatItsTargetNamesEvent.Inner -> "inner"
        is ADelayedSendReachesWhatItsTargetNamesEvent.Late -> "late"
        is ADelayedSendReachesWhatItsTargetNamesEvent.Lost -> "lost"
        is ADelayedSendReachesWhatItsTargetNamesEvent.LostReached -> "lostReached"
        is ADelayedSendReachesWhatItsTargetNamesEvent.Ping -> "ping"
        is ADelayedSendReachesWhatItsTargetNamesEvent.Pong -> "pong"
        is ADelayedSendReachesWhatItsTargetNamesEvent.Settle -> "settle"
        is ADelayedSendReachesWhatItsTargetNamesEvent.Stop -> "stop"
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
            "a_delayed_send_reaches_what_its_target_names",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'order' with expr
        try {
            val initResult_order = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "order", initResult_order)
        } catch (e: Exception) {
            raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "<data id='order'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'innerInternal' with expr
        try {
            val initResult_innerInternal = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "innerInternal", initResult_innerInternal)
        } catch (e: Exception) {
            raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "<data id='innerInternal'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'lateOk' with expr
        try {
            val initResult_lateOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "lateOk", initResult_lateOk)
        } catch (e: Exception) {
            raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "<data id='lateOk'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'lateCount' with expr
        try {
            val initResult_lateCount = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "lateCount", initResult_lateCount)
        } catch (e: Exception) {
            raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "<data id='lateCount'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'pongOk' with expr
        try {
            val initResult_pongOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "pongOk", initResult_pongOk)
        } catch (e: Exception) {
            raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "<data id='pongOk'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'commErrors' with expr
        try {
            val initResult_commErrors = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "commErrors", initResult_commErrors)
        } catch (e: Exception) {
            raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "<data id='commErrors'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'lostArrived' with expr
        try {
            val initResult_lostArrived = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "lostArrived", initResult_lostArrived)
        } catch (e: Exception) {
            raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "<data id='lostArrived'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterStranger' with expr
        try {
            val initResult_afterStranger = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterStranger", initResult_afterStranger)
        } catch (e: Exception) {
            raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "<data id='afterStranger'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'hellos' with expr
        try {
            val initResult_hellos = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "hellos", initResult_hellos)
        } catch (e: Exception) {
            raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "<data id='hellos'> expr failed to evaluate")
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
            raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ADelayedSendReachesWhatItsTargetNamesEvent) {
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
    override fun bindCurrentEvent(event: ADelayedSendReachesWhatItsTargetNamesEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ADelayedSendReachesWhatItsTargetNamesState,
        event: ADelayedSendReachesWhatItsTargetNamesEvent?
    ): EnabledTransition<ADelayedSendReachesWhatItsTargetNamesState, HistoryId>? = when (state) {
        is ADelayedSendReachesWhatItsTargetNamesState.Run -> when {
            event is ADelayedSendReachesWhatItsTargetNamesEvent.Error.Communication -> transitionRunAt0
            event is ADelayedSendReachesWhatItsTargetNamesEvent.Inner -> transitionRunAt1
            event is ADelayedSendReachesWhatItsTargetNamesEvent.Early -> transitionRunAt2
            event is ADelayedSendReachesWhatItsTargetNamesEvent.Late -> transitionRunAt3
            event is ADelayedSendReachesWhatItsTargetNamesEvent.Hello && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(hellos == 1)", "hellos === 1")) -> transitionRunAt4
            event is ADelayedSendReachesWhatItsTargetNamesEvent.Hello -> transitionRunAt5
            event is ADelayedSendReachesWhatItsTargetNamesEvent.LostReached -> transitionRunAt6
            event is ADelayedSendReachesWhatItsTargetNamesEvent.Pong -> transitionRunAt7
            event is ADelayedSendReachesWhatItsTargetNamesEvent.Settle -> transitionRunAt8
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:51 :: _machine
    override fun onEntry(state: ADelayedSendReachesWhatItsTargetNamesState, isDefaultEntry: Boolean) {
        when (state) {
            is ADelayedSendReachesWhatItsTargetNamesState.Done -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:155 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is ADelayedSendReachesWhatItsTargetNamesState.Run -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:66 :: run :: _state_body
                // W3C SCXML 3.8: Onentry block 1/2
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_4", 10L, ADelayedSendReachesWhatItsTargetNamesEvent.Early, EventMetadata.external(sendId = "__send_4", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            // W3C SCXML 6.2.4: a delay postpones the send, it does not change
            // where it goes — the event joins the internal queue when due.
            scheduleInternalSend("__send_5", 20L, ADelayedSendReachesWhatItsTargetNamesEvent.Inner, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 3.8: Onentry block 2/2
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML C.1: `#_scxml_<sessionid>` names a session, and the only
            // one this processor reaches by that name is its own — the bare
            // prefix names it too (test 190). Any other is not reachable:
            // error.communication, nothing delivered, the block ended (4.9).
            if (scriptSessionId != "stranger") {
                raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Communication, "<send target='#_scxml_stranger'> names a session this processor cannot reach", "__send_6")
                return@send true
            }
            // W3C SCXML 6.2.4: this session's own external queue, when due.
            scheduleSend("__send_6", 20L, ADelayedSendReachesWhatItsTargetNamesEvent.Lost, EventMetadata.external(sendId = "__send_6", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterStranger", "afterStranger"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "run.${System.identityHashCode(this)}.kid"
                    deferInvoke(state, generatedInvokeId) {

                        val childSM = ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeKidStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("kid", childSM, false, ADelayedSendReachesWhatItsTargetNamesEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "run.${System.identityHashCode(this)}.gone"
                    deferInvoke(state, generatedInvokeId) {

                        val childSM = ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("gone", childSM, false, ADelayedSendReachesWhatItsTargetNamesEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:51 :: _machine
    override fun onExit(state: ADelayedSendReachesWhatItsTargetNamesState) {
        when (state) {
            is ADelayedSendReachesWhatItsTargetNamesState.Done -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:155 :: done :: _state_body
            }
            is ADelayedSendReachesWhatItsTargetNamesState.Run -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:66 :: run :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("kid")
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("gone")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:51 :: _machine
    override fun executeTransitionContent(source: ADelayedSendReachesWhatItsTargetNamesState, transitionIndex: Int) {
        when (source) {
        is ADelayedSendReachesWhatItsTargetNamesState.Run -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:114 :: run :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("commErrors", "commErrors"), com.sce.runtime.ScriptSource.lua("_scxml_add(commErrors, 1)", "commErrors + 1"))) {
                return
            }
            }
            1 -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:117 :: run :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("order", "order"), com.sce.runtime.ScriptSource.lua("_scxml_add((order * 10), 1)", "order * 10 + 1"))) {
                return
            }


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.type == \"internal\")", "_event.type === 'internal'"))) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("innerInternal", "innerInternal"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            }
            2 -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:123 :: run :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("order", "order"), com.sce.runtime.ScriptSource.lua("_scxml_add((order * 10), 3)", "order * 10 + 3"))) {
                return
            }
            }
            3 -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:126 :: run :: _transition_3


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("lateCount", "lateCount"), com.sce.runtime.ScriptSource.lua("_scxml_add(lateCount, 1)", "lateCount + 1"))) {
                return
            }


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data == 5)", "_event.data === 5"))) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("lateOk", "lateOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            }
            4 -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:132 :: run :: _transition_4


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("hellos", "hellos"), com.sce.runtime.ScriptSource.lua("2", "2"))) {
                return
            }


            if (run send@{
            var paramFailed = false
            // W3C SCXML 5.6.2: the value of <content expr> is the event's data.
            // "If the evaluation of 'expr' produces an error, the Processor MUST
            // place error.execution in the internal event queue and use the
            // empty string as the value of the <content> element": the message
            // still goes, carrying "", and — like a failing <param> — the error
            // ends the block once it has (§scxml-4.9).
            ensureScriptEngine()
            val sendData = try {
                val sendContentValue = (scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)"))
                    .evaluateExpr(scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)"), com.sce.runtime.ScriptSource.lua("7", "7"))
                if (sendContentValue != null) valueToJson(sendContentValue) else ""
            } catch (_: Exception) {
                raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Execution, "<send> contentexpr failed to evaluate", "__send_0")
                paramFailed = true
                valueToJson("")
            }
            // W3C SCXML 6.2.4 + 6.4: a delayed send to an invoked child. Whether
            // it is still running is judged when the send comes due; if not,
            // the sender hears error.communication then (W3C SCXML C.1).
            scheduleChildSend("__send_0", 20L, "kid", "ping", sendData, ADelayedSendReachesWhatItsTargetNamesEvent.Error.Communication)
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2.4 + 6.4: a delayed send to an invoked child. Whether
            // it is still running is judged when the send comes due; if not,
            // the sender hears error.communication then (W3C SCXML C.1).
            scheduleChildSend("__send_1", 100L, "gone", "lost", sendData, ADelayedSendReachesWhatItsTargetNamesEvent.Error.Communication)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test192): Send event to invoked child. The build
            // refused a target naming no invocation above; one that is not
            // running is not there to reach (W3C SCXML C.1).
            if (!sendToChild("gone", "stop", sendData)) {
                raisePlatformError(ADelayedSendReachesWhatItsTargetNamesEvent.Error.Communication, "<send target='#_gone'> names an invocation that is not running", "__send_2")
                return@send true
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            5 -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:140 :: run :: _transition_5


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("hellos", "hellos"), com.sce.runtime.ScriptSource.lua("_scxml_add(hellos, 1)", "hellos + 1"))) {
                return
            }
            }
            6 -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:143 :: run :: _transition_6


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("lostArrived", "lostArrived"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            7 -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names.scxml:146 :: run :: _transition_7


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data == 8)", "_event.data === 8"))) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pongOk", "pongOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_3", 300L, ADelayedSendReachesWhatItsTargetNamesEvent.Settle, EventMetadata.external(sendId = "__send_3", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
