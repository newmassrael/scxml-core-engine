// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d9c7eeffd42250afac7bb84392f7db6b4e0a95d9e7e2e16957a4ecc188fd0aa8
// generated-at: 1779980218

// GENERATED CODE — DO NOT EDIT
// Source: resources/226/test226sub1.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test226sub1.scxml:5

package com.sce.generated.test226

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test226sub1State : State {
    data object Final : Test226sub1State
    data object S0 : Test226sub1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test226sub1Event : Event {
    sealed interface Error : Test226sub1Event {
        data object Execution : Error
    }
    data object VarBound : Test226sub1Event
}
// --- State Machine (W3C SCXML) ---

class Test226sub1StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test226sub1State, Test226sub1Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test226sub1State = Test226sub1State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test226sub1State? = when (stateId) {
        "final" -> Test226sub1State.Final
        "s0" -> Test226sub1State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test226sub1State): String = when (state) {
        is Test226sub1State.Final -> "final"
        is Test226sub1State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test226sub1State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test226sub1State): Int = when (state) {
        is Test226sub1State.Final -> 1
        is Test226sub1State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test226sub1Event? = when (name) {
        "error.execution" -> Test226sub1Event.Error.Execution
        "varBound" -> Test226sub1Event.VarBound
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test226sub1Event): String? = when (event) {
        is Test226sub1Event.Error.Execution -> "error.execution"
        is Test226sub1Event.VarBound -> "varBound"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test226sub1")

        // W3C SCXML 5.2: Runtime variable 'Var1' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var1", null)
        } catch (_: Exception) {}




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
            raiseInternal(Test226sub1Event.Error.Execution)
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
            raiseInternal(Test226sub1Event.Error.Execution)
        }
    }

    // W3C SCXML 3.8.6: Script block execution
    private fun executeScriptBlock(script: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raiseInternal(Test226sub1Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test226sub1Event) {
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
        state: Test226sub1State,
        event: Test226sub1Event
    ): TransitionResult<Test226sub1State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test226sub1State
    ): TransitionResult<Test226sub1State> = when (state) {
        is Test226sub1State.S0 -> processNullS0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test226sub1State> = when {
        safeEvaluateGuard("typeof Var1 !== 'undefined'") -> TransitionResult.External(Test226sub1State.Final, Test226sub1State.S0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test226sub1State.Final, Test226sub1State.S0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test226sub1.scxml:5
    override fun onEntry(state: Test226sub1State) {
        when (state) {
            is Test226sub1State.Final -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("final")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test226sub1State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test226sub1.scxml:5
    override fun onExit(state: Test226sub1State) {
        when (state) {
            is Test226sub1State.Final -> {
                activeStateIds.remove("final")
            }
            is Test226sub1State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test226sub1.scxml:5
    override fun executeTransitionActions(
        source: Test226sub1State,
        event: Test226sub1Event?
    ) {
        when (source) {
        is Test226sub1State.S0 -> when {
            event == null && safeEvaluateGuard("typeof Var1 !== 'undefined'") -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("varBound", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
