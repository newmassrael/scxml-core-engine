// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 7d180dffdd955c10062343fb76305c7a80a95112d21da2591e0f0959805b08ad
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/178/test178.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test178.scxml:10

package com.sce.generated.test178

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test178State : State {
    data object Fail : Test178State
    data object Final : Test178State
    data object S0 : Test178State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test178Event : Event {
    sealed interface Error : Test178Event {
        data object Execution : Error
    }
    data object Event1 : Test178Event
}
// --- State Machine (W3C SCXML) ---

class Test178StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test178State, Test178Event>(scriptEngine) {

    override val initialState: Test178State = Test178State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test178State? = when (stateId) {
        "fail" -> Test178State.Fail
        "final" -> Test178State.Final
        "s0" -> Test178State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test178State): String = when (state) {
        is Test178State.Fail -> "fail"
        is Test178State.Final -> "final"
        is Test178State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test178State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test178State): Int = when (state) {
        is Test178State.Fail -> 2
        is Test178State.Final -> 1
        is Test178State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test178Event? = when (name) {
        "error.execution" -> Test178Event.Error.Execution
        "event1" -> Test178Event.Event1
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test178Event): String? = when (event) {
        is Test178Event.Error.Execution -> "error.execution"
        is Test178Event.Event1 -> "event1"
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
            "test178",
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
            raiseInternal(Test178Event.Error.Execution)
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
            raiseInternal(Test178Event.Error.Execution)
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
            raiseInternal(Test178Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test178Event) {
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
        state: Test178State,
        event: Test178Event
    ): TransitionResult<Test178State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test178State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test178Event
    ): TransitionResult<Test178State> = when {
        event is Test178Event.Event1 -> TransitionResult.External(Test178State.Final, Test178State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test178State.Fail, Test178State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test178.scxml:10
    override fun onEntry(state: Test178State) {
        when (state) {
            is Test178State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test178State.Final -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("final")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test178State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            // W3C SCXML 5.10: Evaluate params/namelist for event data
            run {
                ensureScriptEngine()
                val engineE = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidE = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsE = mutableMapOf<String, Any?>()
                try { paramsE["Var1"] = engineE.evaluateExpr(sidE, "2") } catch (_: Exception) { paramsE["Var1"] = "" }
                try { paramsE["Var1"] = engineE.evaluateExpr(sidE, "3") } catch (_: Exception) { paramsE["Var1"] = "" }
                val eventDataE = buildJsonFromParams(paramsE)
                send(Test178Event.Event1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = eventDataE))
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test178.scxml:10
    override fun onExit(state: Test178State) {
        when (state) {
            is Test178State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test178State.Final -> {
                activeStateIds.remove("final")
            }
            is Test178State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test178.scxml:10
    override fun executeTransitionActions(
        source: Test178State,
        event: Test178Event?
    ) {
        when (source) {
        is Test178State.S0 -> when {
            event is Test178Event.Event1 -> {

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("_event : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.raw")?.toString() ?: ""))
            } catch (_: Exception) {}
            }
            else -> {}
        }
        else -> {}
        }
    }
}
