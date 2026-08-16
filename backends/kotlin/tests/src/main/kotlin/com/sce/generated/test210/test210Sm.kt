// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: eef83a0380a6f32e69bd8e491d75a942150e8193a11c5aedb68d2fc11fa47b6e
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/210/test210.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test210.scxml:6 :: _machine

package com.sce.generated.test210

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test210State : State {
    data object Fail : Test210State
    data object Pass : Test210State
    data object S0 : Test210State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test210Event : Event {
    sealed interface Error : Test210Event {
        data object Execution : Error
    }
    data object Event1 : Test210Event
    data object Event2 : Test210Event
}
// --- State Machine (W3C SCXML) ---

class Test210StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test210State, Test210Event>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `Var1` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var1` was assigned a value of another type, or the engine refused.
     */
    fun Var1(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "Var1")

    override val initialState: Test210State = Test210State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test210State? = when (stateId) {
        "fail" -> Test210State.Fail
        "pass" -> Test210State.Pass
        "s0" -> Test210State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test210State): String = when (state) {
        is Test210State.Fail -> "fail"
        is Test210State.Pass -> "pass"
        is Test210State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test210State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test210State): Int = when (state) {
        is Test210State.Fail -> 2
        is Test210State.Pass -> 1
        is Test210State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test210Event? = when (name) {
        "error.execution" -> Test210Event.Error.Execution
        "event1" -> Test210Event.Event1
        "event2" -> Test210Event.Event2
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test210Event): String? = when (event) {
        is Test210Event.Error.Execution -> "error.execution"
        is Test210Event.Event1 -> "event1"
        is Test210Event.Event2 -> "event2"
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
            "test210",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "'bar'")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test210Event.Error.Execution)
        }




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
            raiseInternal(Test210Event.Error.Execution)
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
            raiseInternal(Test210Event.Error.Execution)
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
            raiseInternal(Test210Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test210Event) {
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
        // W3C SCXML C.1: `_event.origin` is the sender's published
        // `_ioprocessors` location, not its bare session id — and this is the
        // one place that publishes `_event` to the document, so this is where
        // the id becomes a location. The engine keeps the bare id in
        // `EventMetadata.origin` because its session-keyed lookups (`<finalize>`
        // dispatch, cancelled-invoke filtering) match on it; converting at the
        // raise would make one value serve two consumers that need different
        // spellings. The conversion itself lives in
        // `com.sce.runtime.IoProcessors.publishedOrigin`, the port of the
        // `IOProcessorHelper::publishedOrigin` the C++ engines share: a second
        // spelling of the rule is how the backends would stop agreeing.
        val effectiveOrigin = com.sce.runtime.IoProcessors.publishedOrigin(
            if (meta.type == "external") meta.origin.ifEmpty { scriptSessionId ?: "" } else meta.origin
        )
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
        state: Test210State,
        event: Test210Event
    ): TransitionResult<Test210State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test210State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test210Event
    ): TransitionResult<Test210State> = when {
        event is Test210Event.Event2 -> TransitionResult.External(Test210State.Pass, Test210State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test210State.Fail, Test210State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test210.scxml:6 :: _machine
    override fun onEntry(state: Test210State, pathChild: Test210State?) {
        when (state) {
            is Test210State.Fail -> {
                // SCE-MAP: test210.scxml:26 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test210State.Pass -> {
                // SCE-MAP: test210.scxml:25 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test210State.S0 -> {
                // SCE-MAP: test210.scxml:11 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("foo", 1000L, Test210Event.Event1)


            scheduleSend("__send_0", 1500L, Test210Event.Event2)


            executeAssign("Var1", "'foo'")


            // W3C SCXML 6.3: Dynamic sendid evaluation (test210)
            run {
                ensureScriptEngine()
                val engineCancel = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidCancel = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    val v = engineCancel.evaluateExpr(sidCancel, "Var1")
                    val sendidToCancel = v?.toString() ?: ""
                    if (sendidToCancel.isNotEmpty()) cancelSend(sendidToCancel)
                } catch (_: Exception) {}
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test210.scxml:6 :: _machine
    override fun onExit(state: Test210State) {
        when (state) {
            is Test210State.Fail -> {
                // SCE-MAP: test210.scxml:26 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test210State.Pass -> {
                // SCE-MAP: test210.scxml:25 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test210State.S0 -> {
                // SCE-MAP: test210.scxml:11 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test210.scxml:6 :: _machine
    override fun executeTransitionActions(
        source: Test210State,
        event: Test210Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
