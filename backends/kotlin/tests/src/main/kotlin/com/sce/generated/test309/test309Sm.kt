// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 404543f718053b591c81820b426a89e378efb73d3257abfa500da63f6bd03099
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/309/test309.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test309.scxml:5

package com.sce.generated.test309

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test309State : State {
    data object Fail : Test309State
    data object Pass : Test309State
    data object S0 : Test309State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test309Event : Event {
    sealed interface Error : Test309Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test309StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test309State, Test309Event>(scriptEngine) {

    override val initialState: Test309State = Test309State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test309State? = when (stateId) {
        "fail" -> Test309State.Fail
        "pass" -> Test309State.Pass
        "s0" -> Test309State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test309State): String = when (state) {
        is Test309State.Fail -> "fail"
        is Test309State.Pass -> "pass"
        is Test309State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test309State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test309State): Int = when (state) {
        is Test309State.Fail -> 2
        is Test309State.Pass -> 1
        is Test309State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test309Event? = when (name) {
        "error.execution" -> Test309Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test309Event): String? = when (event) {
        is Test309Event.Error.Execution -> "error.execution"
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
            "test309",
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
            raiseInternal(Test309Event.Error.Execution)
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
            raiseInternal(Test309Event.Error.Execution)
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
            raiseInternal(Test309Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test309Event) {
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
        state: Test309State,
        event: Test309Event
    ): TransitionResult<Test309State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test309State
    ): TransitionResult<Test309State> = when (state) {
        is Test309State.S0 -> processNullS0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test309State> = when {
        safeEvaluateGuard("return") -> TransitionResult.External(Test309State.Fail, Test309State.S0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test309State.Pass, Test309State.S0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test309.scxml:5
    override fun onEntry(state: Test309State) {
        when (state) {
            is Test309State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test309State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test309State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test309.scxml:5
    override fun onExit(state: Test309State) {
        when (state) {
            is Test309State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test309State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test309State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test309.scxml:5
    override fun executeTransitionActions(
        source: Test309State,
        event: Test309Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
