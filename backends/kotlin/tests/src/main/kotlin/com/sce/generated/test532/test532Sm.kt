// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/532/test532.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test532.scxml:4

package com.sce.generated.test532

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test532State : State {
    data object Fail : Test532State
    data object Pass : Test532State
    data object S0 : Test532State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test532Event : Event {
    data object Empty : Test532Event
    sealed interface HTTP : Test532Event {
        data object POST : HTTP
    }
    sealed interface Error : Test532Event {
        data object Communication : Error
        data object Execution : Error
    }
    data object Timeout : Test532Event
}
// --- State Machine (W3C SCXML) ---

class Test532StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test532State, Test532Event>(scriptEngine) {

    override val initialState: Test532State = Test532State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test532State? = when (stateId) {
        "fail" -> Test532State.Fail
        "pass" -> Test532State.Pass
        "s0" -> Test532State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test532State): String = when (state) {
        is Test532State.Fail -> "fail"
        is Test532State.Pass -> "pass"
        is Test532State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test532State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test532State): Int = when (state) {
        is Test532State.Fail -> 2
        is Test532State.Pass -> 1
        is Test532State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test532Event? = when (name) {
        "" -> Test532Event.Empty
        "error.communication" -> Test532Event.Error.Communication
        "error.execution" -> Test532Event.Error.Execution
        "HTTP.POST" -> Test532Event.HTTP.POST
        "timeout" -> Test532Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test532Event): String? = when (event) {
        is Test532Event.Empty -> ""
        is Test532Event.Error.Communication -> "error.communication"
        is Test532Event.Error.Execution -> "error.execution"
        is Test532Event.HTTP.POST -> "HTTP.POST"
        is Test532Event.Timeout -> "timeout"
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
            "test532",
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
            raiseInternal(Test532Event.Error.Execution)
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
            raiseInternal(Test532Event.Error.Execution)
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
            raiseInternal(Test532Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test532Event) {
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
        state: Test532State,
        event: Test532Event
    ): TransitionResult<Test532State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test532State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test532Event
    ): TransitionResult<Test532State> = when {
        event is Test532Event.HTTP.POST -> TransitionResult.External(Test532State.Pass, Test532State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test532State.Fail, Test532State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test532.scxml:4
    override fun onEntry(state: Test532State) {
        when (state) {
            is Test532State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test532State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test532State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 3000L, Test532Event.Timeout)


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
                        raiseInternal(Test532Event.Error.Execution, EventMetadata(type = "platform", sendId = "__send_1"))
                        return@resolveTarget
                    }
                    // W3C SCXML C.1 (test496): Unreachable target (C++ SendHelper::isUnreachableTarget)
                    if (target.isEmpty() || target == "undefined") {
                        raiseInternal(Test532Event.Error.Communication, EventMetadata.platform())
                        return@resolveTarget
                    }
                    _resolvedTarget = target
                } catch (_: Exception) {
                    raiseInternal(Test532Event.Error.Execution, EventMetadata.platform())
                }
            }
            _resolvedTarget?.let { _rt ->
            // W3C SCXML C.2: Validate dynamic target is HTTP URL
            if (!_rt.startsWith("http://") && !_rt.startsWith("https://")) {
                raiseInternal(Test532Event.Error.Communication, EventMetadata.platform())
            } else {

            performHttpSend(_rt, "", "some content", emptyMap(), "__send_1")
            }
            } // end of _resolvedTarget?.let
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test532.scxml:4
    override fun onExit(state: Test532State) {
        when (state) {
            is Test532State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test532State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test532State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test532.scxml:4
    override fun executeTransitionActions(
        source: Test532State,
        event: Test532Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
