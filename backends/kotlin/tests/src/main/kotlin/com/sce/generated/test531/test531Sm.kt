// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/531/test531.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test531.scxml:4

package com.sce.generated.test531

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test531State : State {
    data object Fail : Test531State
    data object Pass : Test531State
    data object S0 : Test531State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test531Event : Event {
    sealed interface Error : Test531Event {
        data object Communication : Error
        data object Execution : Error
    }
    data object Test : Test531Event
    data object Timeout : Test531Event
}
// --- State Machine (W3C SCXML) ---

class Test531StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test531State, Test531Event>(scriptEngine) {

    override val initialState: Test531State = Test531State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test531State? = when (stateId) {
        "fail" -> Test531State.Fail
        "pass" -> Test531State.Pass
        "s0" -> Test531State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test531State): String = when (state) {
        is Test531State.Fail -> "fail"
        is Test531State.Pass -> "pass"
        is Test531State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test531State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test531State): Int = when (state) {
        is Test531State.Fail -> 2
        is Test531State.Pass -> 1
        is Test531State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test531Event? = when (name) {
        "error.communication" -> Test531Event.Error.Communication
        "error.execution" -> Test531Event.Error.Execution
        "test" -> Test531Event.Test
        "timeout" -> Test531Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test531Event): String? = when (event) {
        is Test531Event.Error.Communication -> "error.communication"
        is Test531Event.Error.Execution -> "error.execution"
        is Test531Event.Test -> "test"
        is Test531Event.Timeout -> "timeout"
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
            "test531",
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
            raiseInternal(Test531Event.Error.Execution)
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
            raiseInternal(Test531Event.Error.Execution)
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
            raiseInternal(Test531Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test531Event) {
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
        state: Test531State,
        event: Test531Event
    ): TransitionResult<Test531State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test531State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test531Event
    ): TransitionResult<Test531State> = when {
        event is Test531Event.Test -> TransitionResult.External(Test531State.Pass, Test531State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test531State.Fail, Test531State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test531.scxml:4
    override fun onEntry(state: Test531State) {
        when (state) {
            is Test531State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test531State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test531State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 3000L, Test531Event.Timeout)


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
                        raiseInternal(Test531Event.Error.Execution, EventMetadata(type = "platform", sendId = "__send_1"))
                        return@resolveTarget
                    }
                    // W3C SCXML C.1 (test496): Unreachable target (C++ SendHelper::isUnreachableTarget)
                    if (target.isEmpty() || target == "undefined") {
                        raiseInternal(Test531Event.Error.Communication, EventMetadata.platform())
                        return@resolveTarget
                    }
                    _resolvedTarget = target
                } catch (_: Exception) {
                    raiseInternal(Test531Event.Error.Execution, EventMetadata.platform())
                }
            }
            _resolvedTarget?.let { _rt ->
            // W3C SCXML C.2: Validate dynamic target is HTTP URL
            if (!_rt.startsWith("http://") && !_rt.startsWith("https://")) {
                raiseInternal(Test531Event.Error.Communication, EventMetadata.platform())
            } else {

            // W3C SCXML C.2: BasicHTTP send with static params
            run {
                val httpParams = mutableMapOf<String, List<String>>()
                httpParams["_scxmleventname"] = listOf("test")
                performHttpSend(_rt, "", "", httpParams, "__send_1")
            }
            }
            } // end of _resolvedTarget?.let
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test531.scxml:4
    override fun onExit(state: Test531State) {
        when (state) {
            is Test531State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test531State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test531State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test531.scxml:4
    override fun executeTransitionActions(
        source: Test531State,
        event: Test531Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
