// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e9541de728219e5b918752124cad2b5ba2950a5da7bb328f3588c49d2bba35c4
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/253/test253.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test253.scxml:8

package com.sce.generated.test253

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test253State : State {
    data object Fail : Test253State
    data object Pass : Test253State
    data object S0 : Test253State
    data object S01 : Test253State
    data object S02 : Test253State
    data object S03 : Test253State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test253Event : Event {
    sealed interface Cancel : Test253Event {
        data object Invoke : Cancel
    }
    data object ChildRunning : Test253Event
    sealed interface Done : Test253Event {
        data object Invoke : Done
    }
    sealed interface Error : Test253Event {
        data object Execution : Error
    }
    data object Fail : Test253Event
    data object Failure : Test253Event
    data object ParentToChild : Test253Event
    data object Success : Test253Event
    data object Timeout : Test253Event
}
// --- State Machine (W3C SCXML) ---

class Test253StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test253State, Test253Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test253State = Test253State.S01

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test253State): Test253State? = when (state) {
        is Test253State.S01 -> Test253State.S0
        is Test253State.S02 -> Test253State.S0
        is Test253State.S03 -> Test253State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test253State): Test253State = when (state) {
        is Test253State.S0 -> Test253State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test253State? = when (stateId) {
        "fail" -> Test253State.Fail
        "pass" -> Test253State.Pass
        "s0" -> Test253State.S0
        "s01" -> Test253State.S01
        "s02" -> Test253State.S02
        "s03" -> Test253State.S03
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test253State): String = when (state) {
        is Test253State.Fail -> "fail"
        is Test253State.Pass -> "pass"
        is Test253State.S0 -> "s0"
        is Test253State.S01 -> "s01"
        is Test253State.S02 -> "s02"
        is Test253State.S03 -> "s03"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test253State): Boolean = when (state) {
        is Test253State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test253State): Int = when (state) {
        is Test253State.Fail -> 5
        is Test253State.Pass -> 4
        is Test253State.S0 -> 0
        is Test253State.S01 -> 1
        is Test253State.S02 -> 2
        is Test253State.S03 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test253Event? = when (name) {
        "cancel.invoke" -> Test253Event.Cancel.Invoke
        "childRunning" -> Test253Event.ChildRunning
        "done.invoke" -> Test253Event.Done.Invoke
        "error.execution" -> Test253Event.Error.Execution
        "fail" -> Test253Event.Fail
        "failure" -> Test253Event.Failure
        "parentToChild" -> Test253Event.ParentToChild
        "success" -> Test253Event.Success
        "timeout" -> Test253Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test253Event): String? = when (event) {
        is Test253Event.Cancel.Invoke -> "cancel.invoke"
        is Test253Event.ChildRunning -> "childRunning"
        is Test253Event.Done.Invoke -> "done.invoke"
        is Test253Event.Error.Execution -> "error.execution"
        is Test253Event.Fail -> "fail"
        is Test253Event.Failure -> "failure"
        is Test253Event.ParentToChild -> "parentToChild"
        is Test253Event.Success -> "success"
        is Test253Event.Timeout -> "timeout"
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
            "test253",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.2: Runtime variable 'Var1' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var1", null)
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
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raiseInternal(Test253Event.Error.Execution)
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
            raiseInternal(Test253Event.Error.Execution)
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
            raiseInternal(Test253Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test253Event) {
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
        state: Test253State,
        event: Test253Event
    ): TransitionResult<Test253State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test253State.S0 -> processS0(event)
        is Test253State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test253State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test253State.S03 -> {
            val result = processS03(event)
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

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test253State
    ): TransitionResult<Test253State> = when (state) {
        is Test253State.S02 -> processNullS02()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS02(
    ): TransitionResult<Test253State> = when {
        safeEvaluateGuard("Var1 == 'http://www.w3.org/TR/scxml/#SCXMLEventProcessor'") -> TransitionResult.External(Test253State.S03, Test253State.S02)
        safeEvaluateGuard("Var1 == 'scxml'") -> TransitionResult.External(Test253State.S03, Test253State.S02)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test253State.Fail, Test253State.S02)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test253Event
    ): TransitionResult<Test253State> = when {
        event is Test253Event.Timeout -> TransitionResult.External(Test253State.Fail, Test253State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test253Event
    ): TransitionResult<Test253State> = when {
        event is Test253Event.ChildRunning -> TransitionResult.External(Test253State.S02, Test253State.S01)

        else -> TransitionResult.Ignored
    }

    private fun processS03(
        event: Test253Event
    ): TransitionResult<Test253State> = when {
        event is Test253Event.Success -> TransitionResult.External(Test253State.Pass, Test253State.S03)

        event is Test253Event.Fail -> TransitionResult.External(Test253State.Fail, Test253State.S03)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test253.scxml:8
    override fun onEntry(state: Test253State) {
        when (state) {
            is Test253State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test253State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test253State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 2000L, Test253Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}.foo"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test253SceSynthInvokeFooStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("foo", childSM, false, Test253Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test253State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test253State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
            is Test253State.S03 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s03")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test253.scxml:8
    override fun onExit(state: Test253State) {
        when (state) {
            is Test253State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test253State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test253State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("foo")
                activeStateIds.remove("s0")
            }
            is Test253State.S01 -> {
                activeStateIds.remove("s01")
            }
            is Test253State.S02 -> {
                activeStateIds.remove("s02")
            }
            is Test253State.S03 -> {
                activeStateIds.remove("s03")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test253.scxml:8
    override fun executeTransitionActions(
        source: Test253State,
        event: Test253Event?
    ) {
        when (source) {
        is Test253State.S01 -> when {
            event is Test253Event.ChildRunning -> {


            executeAssign("Var1", "_event.origintype")
            }
            else -> {}
        }
        is Test253State.S02 -> when {
            event == null && safeEvaluateGuard("Var1 == 'http://www.w3.org/TR/scxml/#SCXMLEventProcessor'") -> {


            // W3C SCXML 6.4 (test192): Send event to invoked child
            sendToChild("foo", "parentToChild")
            }
            event == null && safeEvaluateGuard("Var1 == 'scxml'") -> {


            // W3C SCXML 6.4 (test192): Send event to invoked child
            sendToChild("foo", "parentToChild")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
