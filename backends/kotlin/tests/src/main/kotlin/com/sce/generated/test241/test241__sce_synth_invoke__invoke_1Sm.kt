// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 7914245a302af6112e722e36ab0278821b7fbccd84f029fffa6bd9dd419eb713
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test241__sce_synth_invoke__invoke_1.scxml:3 :: _machine

package com.sce.generated.test241

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test241SceSynthInvokeInvoke1State : State {
    data object Sub02 : Test241SceSynthInvokeInvoke1State
    data object SubFinal2 : Test241SceSynthInvokeInvoke1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test241SceSynthInvokeInvoke1Event : Event {
    sealed interface Error : Test241SceSynthInvokeInvoke1Event {
        data object Execution : Error
    }
    data object Failure : Test241SceSynthInvokeInvoke1Event
    data object Success : Test241SceSynthInvokeInvoke1Event
}
// --- State Machine (W3C SCXML) ---

class Test241SceSynthInvokeInvoke1StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test241SceSynthInvokeInvoke1State, Test241SceSynthInvokeInvoke1Event>(scriptEngine) {

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
    fun Var1(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var1")

    override val initialState: Test241SceSynthInvokeInvoke1State = Test241SceSynthInvokeInvoke1State.Sub02

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test241SceSynthInvokeInvoke1State? = when (stateId) {
        "sub02" -> Test241SceSynthInvokeInvoke1State.Sub02
        "subFinal2" -> Test241SceSynthInvokeInvoke1State.SubFinal2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test241SceSynthInvokeInvoke1State): String = when (state) {
        is Test241SceSynthInvokeInvoke1State.Sub02 -> "sub02"
        is Test241SceSynthInvokeInvoke1State.SubFinal2 -> "subFinal2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test241SceSynthInvokeInvoke1State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test241SceSynthInvokeInvoke1State): Int = when (state) {
        is Test241SceSynthInvokeInvoke1State.Sub02 -> 0
        is Test241SceSynthInvokeInvoke1State.SubFinal2 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test241SceSynthInvokeInvoke1Event? = when (name) {
        "error.execution" -> Test241SceSynthInvokeInvoke1Event.Error.Execution
        "failure" -> Test241SceSynthInvokeInvoke1Event.Failure
        "success" -> Test241SceSynthInvokeInvoke1Event.Success
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test241SceSynthInvokeInvoke1Event): String? = when (event) {
        is Test241SceSynthInvokeInvoke1Event.Error.Execution -> "error.execution"
        is Test241SceSynthInvokeInvoke1Event.Failure -> "failure"
        is Test241SceSynthInvokeInvoke1Event.Success -> "success"
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
            "test241__sce_synth_invoke__invoke_1",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raisePlatformError(Test241SceSynthInvokeInvoke1Event.Error.Execution, "<data id='Var1'> expr failed to evaluate")
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
            raisePlatformError(Test241SceSynthInvokeInvoke1Event.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(Test241SceSynthInvokeInvoke1Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test241SceSynthInvokeInvoke1Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test241SceSynthInvokeInvoke1Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test241SceSynthInvokeInvoke1Event) {
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
        state: Test241SceSynthInvokeInvoke1State,
        event: Test241SceSynthInvokeInvoke1Event
    ): TransitionResult<Test241SceSynthInvokeInvoke1State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test241SceSynthInvokeInvoke1State
    ): TransitionResult<Test241SceSynthInvokeInvoke1State> = when (state) {
        is Test241SceSynthInvokeInvoke1State.Sub02 -> processNullSub02()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub02(
    ): TransitionResult<Test241SceSynthInvokeInvoke1State> = when {
        safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test241SceSynthInvokeInvoke1State.SubFinal2, Test241SceSynthInvokeInvoke1State.Sub02)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test241SceSynthInvokeInvoke1State.SubFinal2, Test241SceSynthInvokeInvoke1State.Sub02)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test241__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun onEntry(state: Test241SceSynthInvokeInvoke1State, pathChild: Test241SceSynthInvokeInvoke1State?) {
        when (state) {
            is Test241SceSynthInvokeInvoke1State.Sub02 -> {
                // SCE-MAP: test241__sce_synth_invoke__invoke_1.scxml:8 :: sub02 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub02")) return
            }
            is Test241SceSynthInvokeInvoke1State.SubFinal2 -> {
                // SCE-MAP: test241__sce_synth_invoke__invoke_1.scxml:17 :: subFinal2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal2")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test241__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun onExit(state: Test241SceSynthInvokeInvoke1State) {
        when (state) {
            is Test241SceSynthInvokeInvoke1State.Sub02 -> {
                // SCE-MAP: test241__sce_synth_invoke__invoke_1.scxml:8 :: sub02 :: _state_body
                activeStateIds.remove("sub02")
            }
            is Test241SceSynthInvokeInvoke1State.SubFinal2 -> {
                // SCE-MAP: test241__sce_synth_invoke__invoke_1.scxml:17 :: subFinal2 :: _state_body
                activeStateIds.remove("subFinal2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test241__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: Test241SceSynthInvokeInvoke1State,
        event: Test241SceSynthInvokeInvoke1Event?
    ) {
        when (source) {
        is Test241SceSynthInvokeInvoke1State.Sub02 -> when {
            event == null && safeEvaluateGuard("Var1 == 1") -> {
                // SCE-MAP: test241__sce_synth_invoke__invoke_1.scxml:9 :: sub02 :: _transition_0


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("success", "")
            }
            event == null -> {
                // SCE-MAP: test241__sce_synth_invoke__invoke_1.scxml:12 :: sub02 :: _transition_1


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("failure", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
