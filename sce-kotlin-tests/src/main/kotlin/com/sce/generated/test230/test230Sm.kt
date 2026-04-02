// GENERATED CODE — DO NOT EDIT
// Source: resources/230/test230.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test230

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test230State : State {
    data object Fail : Test230State
    data object Final : Test230State
    data object S0 : Test230State
    data object S01 : Test230State
    data object S02 : Test230State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test230Event : Event {
    sealed interface Cancel : Test230Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test230Event
    sealed interface Done : Test230Event {
        data object Invoke : Done
    }
    sealed interface Error : Test230Event {
        data object Execution : Error
    }
    data object Timeout : Test230Event
}
// --- State Machine (W3C SCXML) ---

class Test230StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test230State, Test230Event>(scriptEngine) {

    override val initialState: Test230State = Test230State.S01

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(Test230State.S0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test230State): Test230State? = when (state) {
        is Test230State.S01 -> Test230State.S0
        is Test230State.S02 -> Test230State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test230State): Test230State = when (state) {
        is Test230State.S0 -> Test230State.S01
        else -> state
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test230Event? = when (name) {
        "cancel.invoke" -> Test230Event.Cancel.Invoke
        "childToParent" -> Test230Event.ChildToParent
        "done.invoke" -> Test230Event.Done.Invoke
        "error.execution" -> Test230Event.Error.Execution
        "timeout" -> Test230Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test230Event): String? = when (event) {
        is Test230Event.Cancel.Invoke -> "cancel.invoke"
        is Test230Event.ChildToParent -> "childToParent"
        is Test230Event.Done.Invoke -> "done.invoke"
        is Test230Event.Error.Execution -> "error.execution"
        is Test230Event.Timeout -> "timeout"
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
        engine.setupSystemVariables(sid, "test230")




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
            raiseInternal(Test230Event.Error.Execution)
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
            raiseInternal(Test230Event.Error.Execution)
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
            raiseInternal(Test230Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test230Event) {
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
        state: Test230State,
        event: Test230Event
    ): TransitionResult<Test230State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test230State.S0 -> processS0(event)
        is Test230State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test230State.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test230Event
    ): TransitionResult<Test230State> = when {
        event is Test230Event.Timeout -> TransitionResult.External(Test230State.Final, Test230State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test230Event
    ): TransitionResult<Test230State> = when {
        event is Test230Event.ChildToParent -> TransitionResult.External(Test230State.S02, Test230State.S01)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test230State.Fail)
    }

    private fun processS02(
        event: Test230Event
    ): TransitionResult<Test230State> = when {
        event is Test230Event.Done.Invoke -> TransitionResult.External(Test230State.Final, Test230State.S02)

        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test230State) {
        when (state) {
            is Test230State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test230State.Final -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test230State.S0 -> {
            scheduleSend("__send_0", 3000L, Test230Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test230Child0StateMachine(scriptEngine)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, true, Test230Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test230State.S01)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test230State) {
        when (state) {
            is Test230State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test230State,
        event: Test230Event?
    ) {
        when (source) {
        is Test230State.S01 -> when {
            event is Test230Event.ChildToParent -> {
            println("name is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.name")?.toString() ?: ""))
            println("type is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.type")?.toString() ?: ""))
            println("sendid is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.sendid")?.toString() ?: ""))
            println("origin is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.origin")?.toString() ?: ""))
            println("origintype is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.origintype")?.toString() ?: ""))
            println("invokeid is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.invokeid")?.toString() ?: ""))
            println("data is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.data")?.toString() ?: ""))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
