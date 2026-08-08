// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e9541de728219e5b918752124cad2b5ba2950a5da7bb328f3588c49d2bba35c4
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/346/test346.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test346.scxml:7

package com.sce.generated.test346

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test346State : State {
    data object Fail : Test346State
    data object Pass : Test346State
    data object S0 : Test346State
    data object S1 : Test346State
    data object S2 : Test346State
    data object S3 : Test346State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test346Event : Event {
    sealed interface Error : Test346Event {
        data object Execution : Error
    }
    data object Event1 : Test346Event
    data object Event2 : Test346Event
    data object Event3 : Test346Event
    data object Event4 : Test346Event
}
// --- State Machine (W3C SCXML) ---

class Test346StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test346State, Test346Event>(scriptEngine) {

    override val initialState: Test346State = Test346State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test346State? = when (stateId) {
        "fail" -> Test346State.Fail
        "pass" -> Test346State.Pass
        "s0" -> Test346State.S0
        "s1" -> Test346State.S1
        "s2" -> Test346State.S2
        "s3" -> Test346State.S3
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test346State): String = when (state) {
        is Test346State.Fail -> "fail"
        is Test346State.Pass -> "pass"
        is Test346State.S0 -> "s0"
        is Test346State.S1 -> "s1"
        is Test346State.S2 -> "s2"
        is Test346State.S3 -> "s3"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test346State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test346State): Int = when (state) {
        is Test346State.Fail -> 5
        is Test346State.Pass -> 4
        is Test346State.S0 -> 0
        is Test346State.S1 -> 1
        is Test346State.S2 -> 2
        is Test346State.S3 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test346Event? = when (name) {
        "error.execution" -> Test346Event.Error.Execution
        "event1" -> Test346Event.Event1
        "event2" -> Test346Event.Event2
        "event3" -> Test346Event.Event3
        "event4" -> Test346Event.Event4
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test346Event): String? = when (event) {
        is Test346Event.Error.Execution -> "error.execution"
        is Test346Event.Event1 -> "event1"
        is Test346Event.Event2 -> "event2"
        is Test346Event.Event3 -> "event3"
        is Test346Event.Event4 -> "event4"
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
            "test346",
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
            raiseInternal(Test346Event.Error.Execution)
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
            raiseInternal(Test346Event.Error.Execution)
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
            raiseInternal(Test346Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test346Event) {
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
        state: Test346State,
        event: Test346Event
    ): TransitionResult<Test346State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test346State.S0 -> processS0(event)
        is Test346State.S1 -> processS1(event)
        is Test346State.S2 -> processS2(event)
        is Test346State.S3 -> processS3(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test346Event
    ): TransitionResult<Test346State> = when {
        event is Test346Event.Error.Execution -> TransitionResult.External(Test346State.S1, Test346State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test346State.Fail, Test346State.S0)
    }

    private fun processS1(
        event: Test346Event
    ): TransitionResult<Test346State> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is Test346Event.Event1 -> TransitionResult.Internal
        event is Test346Event.Error.Execution -> TransitionResult.External(Test346State.S2, Test346State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test346State.Fail, Test346State.S1)
    }

    private fun processS2(
        event: Test346Event
    ): TransitionResult<Test346State> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is Test346Event.Event2 -> TransitionResult.Internal
        event is Test346Event.Error.Execution -> TransitionResult.External(Test346State.S3, Test346State.S2)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test346State.Fail, Test346State.S2)
    }

    private fun processS3(
        event: Test346Event
    ): TransitionResult<Test346State> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is Test346Event.Event3 -> TransitionResult.Internal
        event is Test346Event.Error.Execution -> TransitionResult.External(Test346State.Pass, Test346State.S3)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test346State.Fail, Test346State.S3)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test346.scxml:7
    override fun onEntry(state: Test346State) {
        when (state) {
            is Test346State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test346State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test346State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            executeAssign("_sessionid", "'otherName'")

            raiseInternal(Test346Event.Event1)
            }
            is Test346State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            executeAssign("_event", "'otherName'")

            raiseInternal(Test346Event.Event2)
            }
            is Test346State.S2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return


            executeAssign("_ioprocessors", "'otherName'")

            raiseInternal(Test346Event.Event3)
            }
            is Test346State.S3 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return


            executeAssign("_name", "'otherName'")

            raiseInternal(Test346Event.Event4)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test346.scxml:7
    override fun onExit(state: Test346State) {
        when (state) {
            is Test346State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test346State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test346State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test346State.S1 -> {
                activeStateIds.remove("s1")
            }
            is Test346State.S2 -> {
                activeStateIds.remove("s2")
            }
            is Test346State.S3 -> {
                activeStateIds.remove("s3")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test346.scxml:7
    override fun executeTransitionActions(
        source: Test346State,
        event: Test346Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
