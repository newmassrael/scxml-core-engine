// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 392bbcde4466dbc0cb9cb0e8b35901796c2cabcfe17ca0552a2f1bf1fe87d8de
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/276/test276.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test276.scxml:6

package com.sce.generated.test276

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test276State : State {
    data object Fail : Test276State
    data object Pass : Test276State
    data object S0 : Test276State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test276Event : Event {
    sealed interface Cancel : Test276Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test276Event {
        data object Invoke : Done
    }
    sealed interface Error : Test276Event {
        data object Execution : Error
    }
    data object Event0 : Test276Event
    data object Event1 : Test276Event
}
// --- State Machine (W3C SCXML) ---

class Test276StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test276State, Test276Event>(scriptEngine) {

    override val initialState: Test276State = Test276State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test276State? = when (stateId) {
        "fail" -> Test276State.Fail
        "pass" -> Test276State.Pass
        "s0" -> Test276State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test276State): String = when (state) {
        is Test276State.Fail -> "fail"
        is Test276State.Pass -> "pass"
        is Test276State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test276State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test276State): Int = when (state) {
        is Test276State.Fail -> 2
        is Test276State.Pass -> 1
        is Test276State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test276Event? = when (name) {
        "cancel.invoke" -> Test276Event.Cancel.Invoke
        "done.invoke" -> Test276Event.Done.Invoke
        "error.execution" -> Test276Event.Error.Execution
        "event0" -> Test276Event.Event0
        "event1" -> Test276Event.Event1
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test276Event): String? = when (event) {
        is Test276Event.Cancel.Invoke -> "cancel.invoke"
        is Test276Event.Done.Invoke -> "done.invoke"
        is Test276Event.Error.Execution -> "error.execution"
        is Test276Event.Event0 -> "event0"
        is Test276Event.Event1 -> "event1"
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
            "test276",
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
            raiseInternal(Test276Event.Error.Execution)
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
            raiseInternal(Test276Event.Error.Execution)
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
            raiseInternal(Test276Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test276Event) {
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
        state: Test276State,
        event: Test276Event
    ): TransitionResult<Test276State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test276State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test276Event
    ): TransitionResult<Test276State> = when {
        event is Test276Event.Event1 -> TransitionResult.External(Test276State.Pass, Test276State.S0)

        event is Test276Event.Event0 -> TransitionResult.External(Test276State.Fail, Test276State.S0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test276.scxml:6
    override fun onEntry(state: Test276State) {
        when (state) {
            is Test276State.Fail -> {
                // SCE-MAP: test276.scxml:17
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test276State.Pass -> {
                // SCE-MAP: test276.scxml:16
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test276State.S0 -> {
                // SCE-MAP: test276.scxml:8
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    // W3C SCXML 6.4: Evaluate params at defer time (parent context)
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    val invokeParams = mutableMapOf<String, Any?>()
                    // W3C SCXML 6.4: Param expr evaluation failure cancels invoke
                    try {
                        invokeParams["Var1"] = engineInv.evaluateExpr(sidInv, "1")
                    } catch (_: Exception) {
                        return@run  // C++ pattern: invoke cancelled on param error
                    }
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test276sub1StateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test276Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test276.scxml:6
    override fun onExit(state: Test276State) {
        when (state) {
            is Test276State.Fail -> {
                // SCE-MAP: test276.scxml:17
                activeStateIds.remove("fail")
            }
            is Test276State.Pass -> {
                // SCE-MAP: test276.scxml:16
                activeStateIds.remove("pass")
            }
            is Test276State.S0 -> {
                // SCE-MAP: test276.scxml:8
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test276.scxml:6
    override fun executeTransitionActions(
        source: Test276State,
        event: Test276Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
