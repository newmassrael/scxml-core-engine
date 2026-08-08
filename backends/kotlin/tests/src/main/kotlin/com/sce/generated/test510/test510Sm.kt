// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e9541de728219e5b918752124cad2b5ba2950a5da7bb328f3588c49d2bba35c4
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/510/test510.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test510.scxml:5

package com.sce.generated.test510

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test510State : State {
    data object Fail : Test510State
    data object Pass : Test510State
    data object S0 : Test510State
    data object S1 : Test510State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test510Event : Event {
    sealed interface Error : Test510Event {
        data object Communication : Error
        data object Execution : Error
    }
    data object Internal : Test510Event
    data object Test : Test510Event
    data object Timeout : Test510Event
}
// --- State Machine (W3C SCXML) ---

class Test510StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test510State, Test510Event>(scriptEngine) {

    override val initialState: Test510State = Test510State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test510State? = when (stateId) {
        "fail" -> Test510State.Fail
        "pass" -> Test510State.Pass
        "s0" -> Test510State.S0
        "s1" -> Test510State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test510State): String = when (state) {
        is Test510State.Fail -> "fail"
        is Test510State.Pass -> "pass"
        is Test510State.S0 -> "s0"
        is Test510State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test510State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test510State): Int = when (state) {
        is Test510State.Fail -> 3
        is Test510State.Pass -> 2
        is Test510State.S0 -> 0
        is Test510State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test510Event? = when (name) {
        "error.communication" -> Test510Event.Error.Communication
        "error.execution" -> Test510Event.Error.Execution
        "internal" -> Test510Event.Internal
        "test" -> Test510Event.Test
        "timeout" -> Test510Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test510Event): String? = when (event) {
        is Test510Event.Error.Communication -> "error.communication"
        is Test510Event.Error.Execution -> "error.execution"
        is Test510Event.Internal -> "internal"
        is Test510Event.Test -> "test"
        is Test510Event.Timeout -> "timeout"
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
            "test510",
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
            raiseInternal(Test510Event.Error.Execution)
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
            raiseInternal(Test510Event.Error.Execution)
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
            raiseInternal(Test510Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test510Event) {
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
        state: Test510State,
        event: Test510Event
    ): TransitionResult<Test510State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test510State.S0 -> processS0(event)
        is Test510State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test510Event
    ): TransitionResult<Test510State> = when {
        event is Test510Event.Internal -> TransitionResult.External(Test510State.S1, Test510State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test510State.Fail, Test510State.S0)
    }

    private fun processS1(
        event: Test510Event
    ): TransitionResult<Test510State> = when {
        event is Test510Event.Test -> TransitionResult.External(Test510State.Pass, Test510State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test510State.Fail, Test510State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test510.scxml:5
    override fun onEntry(state: Test510State) {
        when (state) {
            is Test510State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test510State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test510State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 30000L, Test510Event.Timeout)


            // W3C SCXML 6.2: Resolve dynamic target (targetexpr="_ioprocessors['basichttp'].location")
            var _resolvedTarget: String? = null
            run resolveTarget@{
                ensureScriptEngine()
                val eng = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    val v = eng.evaluateExpr(sid, "_ioprocessors['basichttp'].location")
                    val target = v?.toString() ?: ""
                    // W3C SCXML 6.2 (test194): Invalid target (C++ SendHelper::isInvalidTarget)
                    if (target.startsWith("!")) {
                        raiseInternal(Test510Event.Error.Execution, EventMetadata(type = "platform", sendId = "__send_1"))
                        return@resolveTarget
                    }
                    // W3C SCXML C.1 (test496): Unreachable target (C++ SendHelper::isUnreachableTarget)
                    if (target.isEmpty() || target == "undefined") {
                        raiseInternal(Test510Event.Error.Communication, EventMetadata.platform())
                        return@resolveTarget
                    }
                    _resolvedTarget = target
                } catch (_: Exception) {
                    raiseInternal(Test510Event.Error.Execution, EventMetadata.platform())
                }
            }
            _resolvedTarget?.let { _rt ->
            // W3C SCXML C.2: Validate dynamic target is HTTP URL
            if (!_rt.startsWith("http://") && !_rt.startsWith("https://")) {
                raiseInternal(Test510Event.Error.Communication, EventMetadata.platform())
            } else {

            performHttpSend(_rt, "test", "", emptyMap(), "__send_1")
            }
            } // end of _resolvedTarget?.let

            raiseInternal(Test510Event.Internal)
            }
            is Test510State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test510.scxml:5
    override fun onExit(state: Test510State) {
        when (state) {
            is Test510State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test510State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test510State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test510State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test510.scxml:5
    override fun executeTransitionActions(
        source: Test510State,
        event: Test510Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
