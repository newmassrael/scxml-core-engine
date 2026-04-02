// GENERATED CODE — DO NOT EDIT
// Source: resources/561/test561.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test561

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test561State : State {
    data object Fail : Test561State
    data object Pass : Test561State
    data object S0 : Test561State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test561Event : Event {
    sealed interface Error : Test561Event {
        data object Execution : Error
    }
    data object Foo : Test561Event
}
// --- State Machine (W3C SCXML) ---

class Test561StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test561State, Test561Event>(scriptEngine) {

    override val initialState: Test561State = Test561State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test561State? = when (stateId) {
        "fail" -> Test561State.Fail
        "pass" -> Test561State.Pass
        "s0" -> Test561State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test561State): String = when (state) {
        is Test561State.Fail -> "fail"
        is Test561State.Pass -> "pass"
        is Test561State.S0 -> "s0"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test561State): Boolean = when (state) {
        else -> true
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test561State): Int = when (state) {
        is Test561State.Fail -> 2
        is Test561State.Pass -> 1
        is Test561State.S0 -> 0
        else -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test561Event? = when (name) {
        "error.execution" -> Test561Event.Error.Execution
        "foo" -> Test561Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test561Event): String? = when (event) {
        is Test561Event.Error.Execution -> "error.execution"
        is Test561Event.Foo -> "foo"
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
        engine.setupSystemVariables(sid, "test561")





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
            raiseInternal(Test561Event.Error.Execution)
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
            raiseInternal(Test561Event.Error.Execution)
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
            raiseInternal(Test561Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test561Event) {
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
        state: Test561State,
        event: Test561Event
    ): TransitionResult<Test561State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test561State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test561Event
    ): TransitionResult<Test561State> = when {
        event is Test561Event.Foo && safeEvaluateGuard("_event.data.getElementsByTagName('book')[1].getAttribute('title') == 'title2'") -> TransitionResult.External(Test561State.Pass, Test561State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test561State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test561State) {
        when (state) {
            is Test561State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test561State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test561State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            send(Test561Event.Foo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test561State) {
        when (state) {
            is Test561State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test561State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test561State.S0 -> {
                activeStateIds.remove("s0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test561State,
        event: Test561Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
