// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 84a841eae761d6fbf94d15cd646ae14f47646822f90559441b47e8f14bddfb19
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/175/test175.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test175.scxml:6 :: _machine

package com.sce.generated.test175

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test175State : State {
    data object Fail : Test175State
    data object Pass : Test175State
    data object S0 : Test175State
    data object S1 : Test175State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test175Event : Event {
    sealed interface Error : Test175Event {
        data object Execution : Error
    }
    data object Event1 : Test175Event
    data object Event2 : Test175Event
}
// --- State Machine (W3C SCXML) ---

class Test175StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test175State, Test175Event>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `Var1` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var1` was assigned a value of another type, or the engine refused.
     */
    fun Var1(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "Var1")

    override val initialState: Test175State = Test175State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test175State? = when (stateId) {
        "fail" -> Test175State.Fail
        "pass" -> Test175State.Pass
        "s0" -> Test175State.S0
        "s1" -> Test175State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test175State): String = when (state) {
        is Test175State.Fail -> "fail"
        is Test175State.Pass -> "pass"
        is Test175State.S0 -> "s0"
        is Test175State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test175State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test175State): Int = when (state) {
        is Test175State.Fail -> 3
        is Test175State.Pass -> 2
        is Test175State.S0 -> 0
        is Test175State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test175Event? = when (name) {
        "error.execution" -> Test175Event.Error.Execution
        "event1" -> Test175Event.Event1
        "event2" -> Test175Event.Event2
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test175Event): String? = when (event) {
        is Test175Event.Error.Execution -> "error.execution"
        is Test175Event.Event1 -> "event1"
        is Test175Event.Event2 -> "event2"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

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
            "test175",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "'0s'")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raisePlatformError(Test175Event.Error.Execution, "<data id='Var1'> expr failed to evaluate")
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
    private fun safeEvaluateGuard(guardExpr: String): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(Test175Event.Error.Execution, "a <transition> cond failed to evaluate")
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
    private fun evaluateSendContent(source: String): String {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateExpr(sid, "JSON.stringify((" + source + "))")?.toString() ?: ""
        } catch (e: Exception) {
            raisePlatformError(Test175Event.Error.Execution, "an expression could not be serialised to JSON")
            ""
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    private fun executeAssign(location: String, expr: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raisePlatformError(Test175Event.Error.Execution, "<assign> failed")
        }
    }

    // W3C SCXML 5.8: Script block execution
    private fun executeScriptBlock(script: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raisePlatformError(Test175Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test175Event) {
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
        engine.setCurrentEvent(
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
    }


    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: Test175State,
        event: Test175Event
    ): TransitionResult<Test175State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test175State.S0 -> processS0(event)
        is Test175State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test175Event
    ): TransitionResult<Test175State> = when {
        event is Test175Event.Event1 -> TransitionResult.External(Test175State.S1, Test175State.S0)

        event is Test175Event.Event2 -> TransitionResult.External(Test175State.Fail, Test175State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test175Event
    ): TransitionResult<Test175State> = when {
        event is Test175Event.Event2 -> TransitionResult.External(Test175State.Pass, Test175State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test175State.Fail, Test175State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test175.scxml:6 :: _machine
    override fun onEntry(state: Test175State, pathChild: Test175State?) {
        when (state) {
            is Test175State.Fail -> {
                // SCE-MAP: test175.scxml:28 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test175State.Pass -> {
                // SCE-MAP: test175.scxml:27 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test175State.S0 -> {
                // SCE-MAP: test175.scxml:11 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            executeAssign("Var1", "'1s'")


            // W3C SCXML 6.2: Dynamic delay evaluation
            run {
                ensureScriptEngine()
                val engineDly = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidDly = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val delayStrE: String
                try {
                    val v = engineDly.evaluateExpr(sidDly, "Var1")
                    delayStrE = v?.toString() ?: "0s"
                } catch (_: Exception) {
                    raisePlatformError(Test175Event.Error.Execution, "<send> delayexpr failed to evaluate")
                    return@run
                }
                val delayMsE = parseDelay(delayStrE)
                scheduleSend("__send_0", delayMsE, Test175Event.Event2)
            }


            scheduleSend("__send_1", 500L, Test175Event.Event1)
            }
            is Test175State.S1 -> {
                // SCE-MAP: test175.scxml:22 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test175.scxml:6 :: _machine
    override fun onExit(state: Test175State) {
        when (state) {
            is Test175State.Fail -> {
                // SCE-MAP: test175.scxml:28 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test175State.Pass -> {
                // SCE-MAP: test175.scxml:27 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test175State.S0 -> {
                // SCE-MAP: test175.scxml:11 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test175State.S1 -> {
                // SCE-MAP: test175.scxml:22 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test175.scxml:6 :: _machine
    override fun executeTransitionActions(
        source: Test175State,
        event: Test175Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
