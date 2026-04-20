
// GENERATED CODE — DO NOT EDIT
// Source: resources/234/test234_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test234

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test234Child0State : State {
    data object SubFinal1 : Test234Child0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test234Child0Event : Event {
    data object ChildToParent : Test234Child0Event
    sealed interface Error : Test234Child0Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test234Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test234Child0State, Test234Child0Event>(scriptEngine) {

    override val initialState: Test234Child0State = Test234Child0State.SubFinal1

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test234Child0State? = when (stateId) {
        "subFinal1" -> Test234Child0State.SubFinal1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test234Child0State): String = when (state) {
        is Test234Child0State.SubFinal1 -> "subFinal1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test234Child0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test234Child0State): Int = when (state) {
        is Test234Child0State.SubFinal1 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test234Child0Event? = when (name) {
        "childToParent" -> Test234Child0Event.ChildToParent
        "error.execution" -> Test234Child0Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test234Child0Event): String? = when (event) {
        is Test234Child0Event.ChildToParent -> "childToParent"
        is Test234Child0Event.Error.Execution -> "error.execution"
        // Kotlin `when` expression exhaustiveness: a child machine that
        // inherits the override (has_parent_communication path) but
        // declares no events of its own produces an empty sealed
        // hierarchy, and `when (event)` without `else` fails to compile.
        // The branch is redundant on non-empty hierarchies but harmless.
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
        engine.setupSystemVariables(sid, "test234_child0")





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
            raiseInternal(Test234Child0Event.Error.Execution)
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
            raiseInternal(Test234Child0Event.Error.Execution)
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
            raiseInternal(Test234Child0Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test234Child0Event) {
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
        state: Test234Child0State,
        event: Test234Child0Event
    ): TransitionResult<Test234Child0State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---


    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test234Child0State) {
        when (state) {
            is Test234Child0State.SubFinal1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal1")) return


            // W3C SCXML 5.10: Evaluate params for parent send (test233)
            run {
                ensureScriptEngine()
                val engineP = scriptEngine ?: return@run
                val sidP = scriptSessionId ?: return@run
                val paramsP = mutableMapOf<String, Any?>()
                try { paramsP["aParam"] = engineP.evaluateExpr(sidP, "2") } catch (_: Exception) { paramsP["aParam"] = "" }
                val eventDataP = buildJsonFromParams(paramsP)
                onSendToParent?.invoke("childToParent", eventDataP)
            }
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test234Child0State) {
        when (state) {
            is Test234Child0State.SubFinal1 -> {
                activeStateIds.remove("subFinal1")
            }
        }
    }

    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test234Child0State,
        event: Test234Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
