// GENERATED CODE — DO NOT EDIT
// Source: resources/557/test557.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test557

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test557State : State {
    data object Fail : Test557State
    data object Pass : Test557State
    data object S0 : Test557State
    data object S1 : Test557State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test557Event : Event {
    sealed interface Error : Test557Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test557StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test557State, Test557Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test557State = Test557State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test557Event? = when (name) {
        "error.execution" -> Test557Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test557Event): String? = when (event) {
        is Test557Event.Error.Execution -> "error.execution"
        else -> null
    }


    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: return
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test557")

        // W3C SCXML B.2: Initialize variable 'var1' with inline content
        try {
            val initResult_var1 = engine.evaluateExpr(sid, "<books xmlns=\"\">\n     <book title=\"title1\"/>\n     <book title=\"title2\"/>\n   </books>")
            engine.setVariable(sid, "var1", initResult_var1)
        } catch (e: Exception) {
            raiseInternal(Test557Event.Error.Execution)
        }
        // W3C SCXML 5.2: Runtime variable 'var2' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "var2", null)
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
        val engine = scriptEngine ?: return false
        val sid = scriptSessionId ?: return false
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raiseInternal(Test557Event.Error.Execution)
            false
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    private fun executeAssign(location: String, expr: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raiseInternal(Test557Event.Error.Execution)
        }
    }

    // W3C SCXML 3.8.6: Script block execution
    private fun executeScriptBlock(script: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raiseInternal(Test557Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test557Event) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        val eventName = eventNameOf(event) ?: return
        val meta = currentEventMetadata
        engine.setCurrentEvent(
            sid, eventName,
            data = meta.data,
            type = meta.type,
            sendId = meta.sendId,
            origin = meta.origin.ifEmpty { scriptSessionId ?: "" },
            originType = meta.originType.ifEmpty { "http://www.w3.org/TR/scxml/#SCXMLEventProcessor" },
            invokeId = meta.invokeId
        )
    }

    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: Test557State,
        event: Test557Event
    ): TransitionResult<Test557State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test557State
    ): TransitionResult<Test557State> = when (state) {
        is Test557State.S0 -> processNullS0()
        is Test557State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test557State> = when {
        safeEvaluateGuard("var1.getElementsByTagName('book')[0].getAttribute('title') == 'title1'") -> TransitionResult.External(Test557State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test557State.Fail)
    }

    private fun processNullS1(
    ): TransitionResult<Test557State> = when {
        safeEvaluateGuard("var2.getElementsByTagName('book')[1].getAttribute('title') == 'title2'") -> TransitionResult.External(Test557State.Pass)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test557State.Fail)
    }

    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test557State) {
        when (state) {
            is Test557State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test557State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test557State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test557State,
        event: Test557Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
