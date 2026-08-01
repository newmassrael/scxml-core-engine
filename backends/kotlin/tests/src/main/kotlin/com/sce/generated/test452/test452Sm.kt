// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: daa56c2f4afb81deb723d1d6725c872edb8b62d3d9c4a93c07c834af3417504f
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/452/test452.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test452.scxml:5

package com.sce.generated.test452

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test452State : State {
    data object Fail : Test452State
    data object Pass : Test452State
    data object S0 : Test452State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test452Event : Event {
    sealed interface Error : Test452Event {
        data object Execution : Error
    }
    data object Event1 : Test452Event
}
// --- State Machine (W3C SCXML) ---

class Test452StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test452State, Test452Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test452State = Test452State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test452State? = when (stateId) {
        "fail" -> Test452State.Fail
        "pass" -> Test452State.Pass
        "s0" -> Test452State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test452State): String = when (state) {
        is Test452State.Fail -> "fail"
        is Test452State.Pass -> "pass"
        is Test452State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test452State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test452State): Int = when (state) {
        is Test452State.Fail -> 2
        is Test452State.Pass -> 1
        is Test452State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test452Event? = when (name) {
        "error.execution" -> Test452Event.Error.Execution
        "event1" -> Test452Event.Event1
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test452Event): String? = when (event) {
        is Test452Event.Error.Execution -> "error.execution"
        is Test452Event.Event1 -> "event1"
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
            "test452",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'foo' with expr
        try {
            val initResult_foo = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "foo", initResult_foo)
        } catch (e: Exception) {
            raiseInternal(Test452Event.Error.Execution)
        }


        // W3C SCXML 5.8: Execute global scripts at document load time
        try {
            engine.executeScript(sid, "function testobject() {\n    this.bar = 0;}")
        } catch (e: Exception) {
            raiseInternal(Test452Event.Error.Execution)
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
            raiseInternal(Test452Event.Error.Execution)
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
            raiseInternal(Test452Event.Error.Execution)
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
            raiseInternal(Test452Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test452Event) {
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
        state: Test452State,
        event: Test452Event
    ): TransitionResult<Test452State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test452State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test452Event
    ): TransitionResult<Test452State> = when {
        event is Test452Event.Event1 && safeEvaluateGuard("foo.bar == 1") -> TransitionResult.External(Test452State.Pass, Test452State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test452State.Fail, Test452State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test452.scxml:5
    override fun onEntry(state: Test452State) {
        when (state) {
            is Test452State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test452State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test452State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            executeAssign("foo", "new testobject();")


            executeAssign("foo.bar", "1")

            raiseInternal(Test452Event.Event1)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test452.scxml:5
    override fun onExit(state: Test452State) {
        when (state) {
            is Test452State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test452State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test452State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test452.scxml:5
    override fun executeTransitionActions(
        source: Test452State,
        event: Test452Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
