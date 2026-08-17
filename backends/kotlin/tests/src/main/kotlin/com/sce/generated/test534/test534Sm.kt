// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e4db48621f9961b90c5af89337aad8d33d4505a169c6468912558965970158e9
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/534/test534.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test534.scxml:3 :: _machine

package com.sce.generated.test534

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test534State : State {
    data object Fail : Test534State
    data object Pass : Test534State
    data object S0 : Test534State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test534Event : Event {
    sealed interface Error : Test534Event {
        data object Communication : Error
        data object Execution : Error
    }
    data object Test : Test534Event
    data object Timeout : Test534Event
}
// --- State Machine (W3C SCXML) ---

class Test534StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test534State, Test534Event>(scriptEngine) {

    override val initialState: Test534State = Test534State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test534State? = when (stateId) {
        "fail" -> Test534State.Fail
        "pass" -> Test534State.Pass
        "s0" -> Test534State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test534State): String = when (state) {
        is Test534State.Fail -> "fail"
        is Test534State.Pass -> "pass"
        is Test534State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test534State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test534State): Int = when (state) {
        is Test534State.Fail -> 2
        is Test534State.Pass -> 1
        is Test534State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test534Event? = when (name) {
        "error.communication" -> Test534Event.Error.Communication
        "error.execution" -> Test534Event.Error.Execution
        "test" -> Test534Event.Test
        "timeout" -> Test534Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test534Event): String? = when (event) {
        is Test534Event.Error.Communication -> "error.communication"
        is Test534Event.Error.Execution -> "error.execution"
        is Test534Event.Test -> "test"
        is Test534Event.Timeout -> "timeout"
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
            "test534",
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
            raiseInternal(Test534Event.Error.Execution)
            false
        }
    }

    // W3C SCXML B.2: the value of an inline `<content>` body, serialized
    // for transport.
    //
    // The reading is decided at build time — `source` is already the
    // expression or string literal the clause's ordered readings give —
    // and this evaluates it *here*, at send time, rather than handing the
    // expression to whatever reads `_event.data` later. That distinction
    // is not academic: the two engines this backend runs on disagree
    // about what a data string is. QuickJS tries a JS evaluation before
    // falling back; Rhino goes straight from JSON to the normalized
    // string, so an expression handed to it arrives as its own source
    // text. `JSON.stringify` is what both of them can read back, and it
    // is the same shape the C++ backend transports.
    private fun evaluateSendContent(source: String): String {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateExpr(sid, "JSON.stringify((" + source + "))")?.toString() ?: ""
        } catch (e: Exception) {
            raiseInternal(Test534Event.Error.Execution)
            ""
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
            raiseInternal(Test534Event.Error.Execution)
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
            raiseInternal(Test534Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test534Event) {
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
        state: Test534State,
        event: Test534Event
    ): TransitionResult<Test534State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test534State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test534Event
    ): TransitionResult<Test534State> = when {
        event is Test534Event.Test && safeEvaluateGuard("_event.data[\"_scxmleventname\"] == \"test\"") -> TransitionResult.External(Test534State.Pass, Test534State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test534State.Fail, Test534State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test534.scxml:3 :: _machine
    override fun onEntry(state: Test534State, pathChild: Test534State?) {
        when (state) {
            is Test534State.Fail -> {
                // SCE-MAP: test534.scxml:19 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test534State.Pass -> {
                // SCE-MAP: test534.scxml:18 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test534State.S0 -> {
                // SCE-MAP: test534.scxml:6 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 30000L, Test534Event.Timeout)


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
                        raiseInternal(Test534Event.Error.Execution, EventMetadata(type = "platform", sendId = "__send_1"))
                        return@resolveTarget
                    }
                    // W3C SCXML C.1 (test496): Unreachable target (C++ SendHelper::isUnreachableTarget)
                    if (target.isEmpty() || target == "undefined") {
                        raiseInternal(Test534Event.Error.Communication, EventMetadata.platform())
                        return@resolveTarget
                    }
                    _resolvedTarget = target
                } catch (_: Exception) {
                    raiseInternal(Test534Event.Error.Execution, EventMetadata.platform())
                }
            }
            _resolvedTarget?.let { _rt ->
            // W3C SCXML C.2: Validate dynamic target is HTTP URL
            if (!_rt.startsWith("http://") && !_rt.startsWith("https://")) {
                raiseInternal(Test534Event.Error.Communication, EventMetadata.platform())
            } else {

            performHttpSend(_rt, "test", "", emptyMap(), "__send_1")
            }
            } // end of _resolvedTarget?.let
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test534.scxml:3 :: _machine
    override fun onExit(state: Test534State) {
        when (state) {
            is Test534State.Fail -> {
                // SCE-MAP: test534.scxml:19 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test534State.Pass -> {
                // SCE-MAP: test534.scxml:18 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test534State.S0 -> {
                // SCE-MAP: test534.scxml:6 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test534.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: Test534State,
        event: Test534Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
