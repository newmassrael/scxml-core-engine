// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 025e57d78939dcd3c3bbc54b606a62c00b45f367a9a3d9faa2cdd4bf5896d8fc
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/307/test307.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test307.scxml:1

package com.sce.generated.test307

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test307State : State {
    data object Final : Test307State
    data object S0 : Test307State
    data object S1 : Test307State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test307Event : Event {
    data object Bar : Test307Event
    sealed interface Error : Test307Event {
        data object Self : Error
        data object Execution : Error
    }
    data object Foo : Test307Event
}
// --- State Machine (W3C SCXML) ---

class Test307StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test307State, Test307Event>(scriptEngine) {

    override val initialState: Test307State = Test307State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test307State? = when (stateId) {
        "final" -> Test307State.Final
        "s0" -> Test307State.S0
        "s1" -> Test307State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test307State): String = when (state) {
        is Test307State.Final -> "final"
        is Test307State.S0 -> "s0"
        is Test307State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test307State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test307State): Int = when (state) {
        is Test307State.Final -> 2
        is Test307State.S0 -> 0
        is Test307State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test307Event? = when (name) {
        "bar" -> Test307Event.Bar
        "error" -> Test307Event.Error.Self
        "error.execution" -> Test307Event.Error.Execution
        "foo" -> Test307Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test307Event): String? = when (event) {
        is Test307Event.Bar -> "bar"
        is Test307Event.Error.Self -> "error"
        is Test307Event.Error.Execution -> "error.execution"
        is Test307Event.Foo -> "foo"
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
            "test307",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )





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
            raiseInternal(Test307Event.Error.Execution)
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
            raiseInternal(Test307Event.Error.Execution)
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
            raiseInternal(Test307Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test307Event) {
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
        state: Test307State,
        event: Test307Event
    ): TransitionResult<Test307State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test307State.S0 -> processS0(event)
        is Test307State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test307Event
    ): TransitionResult<Test307State> = when {
        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test307Event.Error || event is Test307Event.Error.Execution) -> TransitionResult.External(Test307State.S1, Test307State.S0)

        event is Test307Event.Foo -> TransitionResult.External(Test307State.S1, Test307State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test307Event
    ): TransitionResult<Test307State> = when {
        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test307Event.Error || event is Test307Event.Error.Execution) -> TransitionResult.External(Test307State.Final, Test307State.S1)

        event is Test307Event.Bar -> TransitionResult.External(Test307State.Final, Test307State.S1)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test307.scxml:1
    override fun onEntry(state: Test307State) {
        when (state) {
            is Test307State.Final -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("final")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test307State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("entering s0 value of Var 1 is: : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "Var1")?.toString() ?: ""))
            } catch (_: Exception) {}

            raiseInternal(Test307Event.Foo)
            }
            is Test307State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
                // W3C SCXML 5.3: Late binding — initialize state-level datamodel on entry
                run {
                    ensureScriptEngine()
                    val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    try {
                        val v = engine.evaluateExpr(sid, "1")
                        engine.setVariable(sid, "Var1", v)
                    } catch (e: Exception) {
                        raiseInternal(Test307Event.Error.Execution)
                    }
                }

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("entering s1, value of non-existent substructure of Var 1 is: : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "Var1.foo")?.toString() ?: ""))
            } catch (_: Exception) {}

            raiseInternal(Test307Event.Bar)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test307.scxml:1
    override fun onExit(state: Test307State) {
        when (state) {
            is Test307State.Final -> {
                activeStateIds.remove("final")
            }
            is Test307State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test307State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test307.scxml:1
    override fun executeTransitionActions(
        source: Test307State,
        event: Test307Event?
    ) {
        when (source) {
        is Test307State.S0 -> when {
            (event is Test307Event.Error || event is Test307Event.Error.Execution) -> {

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("error in state s0: " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event")?.toString() ?: ""))
            } catch (_: Exception) {}
            }
            event is Test307Event.Foo -> {

            println("no error in s0")
            }
            else -> {}
        }
        is Test307State.S1 -> when {
            (event is Test307Event.Error || event is Test307Event.Error.Execution) -> {

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("error in state s1: " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event")?.toString() ?: ""))
            } catch (_: Exception) {}
            }
            event is Test307Event.Bar -> {

            println("No error in s1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
