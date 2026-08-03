// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: a721af75373ae9de49c4cdea1acca1394bb60a4994ec71ccf7cd0c509dda74e7
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/336/test336.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test336.scxml:6

package com.sce.generated.test336

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test336State : State {
    data object Fail : Test336State
    data object Pass : Test336State
    data object S0 : Test336State
    data object S1 : Test336State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test336Event : Event {
    data object Bar : Test336Event
    data object Baz : Test336Event
    sealed interface Error : Test336Event {
        data object Communication : Error
        data object Execution : Error
    }
    data object Foo : Test336Event
}
// --- State Machine (W3C SCXML) ---

class Test336StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test336State, Test336Event>(scriptEngine) {

    override val initialState: Test336State = Test336State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test336State? = when (stateId) {
        "fail" -> Test336State.Fail
        "pass" -> Test336State.Pass
        "s0" -> Test336State.S0
        "s1" -> Test336State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test336State): String = when (state) {
        is Test336State.Fail -> "fail"
        is Test336State.Pass -> "pass"
        is Test336State.S0 -> "s0"
        is Test336State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test336State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test336State): Int = when (state) {
        is Test336State.Fail -> 3
        is Test336State.Pass -> 2
        is Test336State.S0 -> 0
        is Test336State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test336Event? = when (name) {
        "bar" -> Test336Event.Bar
        "baz" -> Test336Event.Baz
        "error.communication" -> Test336Event.Error.Communication
        "error.execution" -> Test336Event.Error.Execution
        "foo" -> Test336Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test336Event): String? = when (event) {
        is Test336Event.Bar -> "bar"
        is Test336Event.Baz -> "baz"
        is Test336Event.Error.Communication -> "error.communication"
        is Test336Event.Error.Execution -> "error.execution"
        is Test336Event.Foo -> "foo"
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
            "test336",
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
            raiseInternal(Test336Event.Error.Execution)
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
            raiseInternal(Test336Event.Error.Execution)
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
            raiseInternal(Test336Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test336Event) {
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
        state: Test336State,
        event: Test336Event
    ): TransitionResult<Test336State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test336State.S0 -> processS0(event)
        is Test336State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test336Event
    ): TransitionResult<Test336State> = when {
        event is Test336Event.Foo -> TransitionResult.External(Test336State.S1, Test336State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test336State.Fail, Test336State.S0)
    }

    private fun processS1(
        event: Test336Event
    ): TransitionResult<Test336State> = when {
        event is Test336Event.Bar -> TransitionResult.External(Test336State.Pass, Test336State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test336State.Fail, Test336State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test336.scxml:6
    override fun onEntry(state: Test336State) {
        when (state) {
            is Test336State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test336State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test336State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            send(Test336Event.Foo, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            is Test336State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            send(Test336Event.Baz, EventMetadata.external(sendId = "__send_2", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test336.scxml:6
    override fun onExit(state: Test336State) {
        when (state) {
            is Test336State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test336State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test336State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test336State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test336.scxml:6
    override fun executeTransitionActions(
        source: Test336State,
        event: Test336Event?
    ) {
        when (source) {
        is Test336State.S0 -> when {
            event is Test336Event.Foo -> {


            // W3C SCXML 6.2: Resolve dynamic target (targetexpr="_event.origin")
            var _resolvedTarget: String? = null
            run resolveTarget@{
                ensureScriptEngine()
                val eng = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    val v = eng.evaluateExpr(sid, "_event.origin")
                    val target = v?.toString() ?: ""
                    // W3C SCXML 6.2 (test194): Invalid target (C++ SendHelper::isInvalidTarget)
                    if (target.startsWith("!")) {
                        raiseInternal(Test336Event.Error.Execution, EventMetadata(type = "platform", sendId = "__send_0"))
                        return@resolveTarget
                    }
                    // W3C SCXML C.1 (test496): Unreachable target (C++ SendHelper::isUnreachableTarget)
                    if (target.isEmpty() || target == "undefined") {
                        raiseInternal(Test336Event.Error.Communication, EventMetadata.platform())
                        return@resolveTarget
                    }
                    _resolvedTarget = target
                } catch (_: Exception) {
                    raiseInternal(Test336Event.Error.Execution, EventMetadata.platform())
                }
            }
            _resolvedTarget?.let { _rt ->
            // W3C SCXML 6.2: Dispatch to dynamically resolved target (C++ unified pattern)
            if (_rt == "#_internal") {
                raiseInternal(Test336Event.Bar)
            } else if (_rt == "#_parent") {
                onSendToParent?.invoke("bar", "")
            } else {
                send(Test336Event.Bar, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            } // end of _resolvedTarget?.let
            }
            else -> {}
        }
        else -> {}
        }
    }
}
