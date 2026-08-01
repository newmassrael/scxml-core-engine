// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: ab200b8eb821f02e246ff33a9f9da5a6f5493996f3df460e1a87cc5891e5b49d
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/558/test558.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test558.scxml:5

package com.sce.generated.test558

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test558State : State {
    data object Fail : Test558State
    data object Pass : Test558State
    data object S0 : Test558State
    data object S1 : Test558State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test558Event : Event {
    sealed interface Error : Test558Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test558StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test558State, Test558Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test558State = Test558State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test558State? = when (stateId) {
        "fail" -> Test558State.Fail
        "pass" -> Test558State.Pass
        "s0" -> Test558State.S0
        "s1" -> Test558State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test558State): String = when (state) {
        is Test558State.Fail -> "fail"
        is Test558State.Pass -> "pass"
        is Test558State.S0 -> "s0"
        is Test558State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test558State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test558State): Int = when (state) {
        is Test558State.Fail -> 3
        is Test558State.Pass -> 2
        is Test558State.S0 -> 0
        is Test558State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test558Event? = when (name) {
        "error.execution" -> Test558Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test558Event): String? = when (event) {
        is Test558Event.Error.Execution -> "error.execution"
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
            "test558",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML B.2: Initialize variable 'var1' with inline content (C++ parseEventData pattern)
        try {
            val initResult_var1 = engine.parseDataValue(sid, "this  is \na string")
            engine.setVariable(sid, "var1", initResult_var1)
        } catch (e: Exception) {
            raiseInternal(Test558Event.Error.Execution)
        }
        // W3C SCXML 5.2.2: Load variable 'var2' from external source (C++ DataModelInitHelper pattern)
        try {
            val srcContent_var2 = engine.loadDataFromSrc("file:test558.txt", "resources/558")
            if (srcContent_var2 != null) {
                val srcValue_var2 = engine.parseDataValue(sid, srcContent_var2)
                engine.setVariable(sid, "var2", srcValue_var2)
            }
        } catch (e: Exception) {
            raiseInternal(Test558Event.Error.Execution)
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
            raiseInternal(Test558Event.Error.Execution)
            false
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
            raiseInternal(Test558Event.Error.Execution)
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
            raiseInternal(Test558Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test558Event) {
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
        val effectiveOrigin = if (meta.type == "external") meta.origin.ifEmpty { scriptSessionId ?: "" } else meta.origin
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
        state: Test558State,
        event: Test558Event
    ): TransitionResult<Test558State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test558State
    ): TransitionResult<Test558State> = when (state) {
        is Test558State.S0 -> processNullS0()
        is Test558State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test558State> = when {
        safeEvaluateGuard("var1 == 'this is a string'") -> TransitionResult.External(Test558State.S1, Test558State.S0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test558State.Fail, Test558State.S0)
    }

    private fun processNullS1(
    ): TransitionResult<Test558State> = when {
        safeEvaluateGuard("var2 == 'this is a string'") -> TransitionResult.External(Test558State.Pass, Test558State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test558State.Fail, Test558State.S1)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test558.scxml:5
    override fun onEntry(state: Test558State) {
        when (state) {
            is Test558State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test558State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test558State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test558State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test558.scxml:5
    override fun onExit(state: Test558State) {
        when (state) {
            is Test558State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test558State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test558State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test558State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test558.scxml:5
    override fun executeTransitionActions(
        source: Test558State,
        event: Test558Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
