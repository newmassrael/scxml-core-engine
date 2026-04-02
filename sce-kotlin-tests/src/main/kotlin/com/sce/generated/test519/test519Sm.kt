// GENERATED CODE — DO NOT EDIT
// Source: resources/519/test519.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test519

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test519State : State {
    data object Fail : Test519State
    data object Pass : Test519State
    data object S0 : Test519State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test519Event : Event {
    sealed interface Error : Test519Event {
        data object Execution : Error
    }
    data object Test : Test519Event
    data object Timeout : Test519Event
}
// --- State Machine (W3C SCXML) ---

class Test519StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test519State, Test519Event>(scriptEngine) {

    override val initialState: Test519State = Test519State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test519Event? = when (name) {
        "error.execution" -> Test519Event.Error.Execution
        "test" -> Test519Event.Test
        "timeout" -> Test519Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test519Event): String? = when (event) {
        is Test519Event.Error.Execution -> "error.execution"
        is Test519Event.Test -> "test"
        is Test519Event.Timeout -> "timeout"
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
        engine.setupSystemVariables(sid, "test519")




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
            raiseInternal(Test519Event.Error.Execution)
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
            raiseInternal(Test519Event.Error.Execution)
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
            raiseInternal(Test519Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test519Event) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
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
            sid, eventName,
            data = meta.data,
            type = effectiveType,
            sendId = meta.sendId,
            origin = effectiveOrigin,
            originType = effectiveOriginType,
            invokeId = meta.invokeId
        )
    }

    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: Test519State,
        event: Test519Event
    ): TransitionResult<Test519State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test519State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test519Event
    ): TransitionResult<Test519State> = when {
        event is Test519Event.Test && safeEvaluateGuard("_event.data[\"param1\"] == 1") -> TransitionResult.External(Test519State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test519State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test519State) {
        when (state) {
            is Test519State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test519State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test519State.S0 -> {
            scheduleSend("__send_0", 30000L, Test519Event.Timeout)
            // W3C SCXML 5.10: Evaluate params/namelist for event data
            run {
                ensureScriptEngine()
                val engineE = scriptEngine ?: return@run
                val sidE = scriptSessionId ?: return@run
                val paramsE = mutableMapOf<String, Any?>()
                try { paramsE["param1"] = engineE.evaluateExpr(sidE, "1") } catch (_: Exception) {}
                val eventDataE = buildJsonFromParams(paramsE)
                send(Test519Event.Test, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = eventDataE))
            }
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test519State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test519State,
        event: Test519Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
