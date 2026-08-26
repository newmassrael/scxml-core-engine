// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b9b6d5a256b534ee1bf3d5ad94af0afa9df9e54bf19008d6dd27d12f1bc9a55e
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test230__sce_synth_invoke__invoke_0.scxml:3 :: _machine

package com.sce.generated.test230

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test230SceSynthInvokeInvoke0State : State {
    data object Sub0 : Test230SceSynthInvokeInvoke0State
    data object SubFinal : Test230SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test230SceSynthInvokeInvoke0Event : Event {
    data object ChildToParent : Test230SceSynthInvokeInvoke0Event
    sealed interface Error : Test230SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
    data object Timeout : Test230SceSynthInvokeInvoke0Event
}
// --- State Machine (W3C SCXML) ---

class Test230SceSynthInvokeInvoke0StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test230SceSynthInvokeInvoke0State, Test230SceSynthInvokeInvoke0Event>(scriptEngine) {

    override val initialState: Test230SceSynthInvokeInvoke0State = Test230SceSynthInvokeInvoke0State.Sub0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test230SceSynthInvokeInvoke0State? = when (stateId) {
        "sub0" -> Test230SceSynthInvokeInvoke0State.Sub0
        "subFinal" -> Test230SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test230SceSynthInvokeInvoke0State): String = when (state) {
        is Test230SceSynthInvokeInvoke0State.Sub0 -> "sub0"
        is Test230SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test230SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test230SceSynthInvokeInvoke0State): Int = when (state) {
        is Test230SceSynthInvokeInvoke0State.Sub0 -> 0
        is Test230SceSynthInvokeInvoke0State.SubFinal -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test230SceSynthInvokeInvoke0Event? = when (name) {
        "childToParent" -> Test230SceSynthInvokeInvoke0Event.ChildToParent
        "error.execution" -> Test230SceSynthInvokeInvoke0Event.Error.Execution
        "timeout" -> Test230SceSynthInvokeInvoke0Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test230SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test230SceSynthInvokeInvoke0Event.ChildToParent -> "childToParent"
        is Test230SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
        is Test230SceSynthInvokeInvoke0Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML 5.3: the declaration hook `enterAt` reaches. Every other caller
    // arrives through a guard, an assign or a script block, all of which run
    // `ensureScriptEngine()` on their own way in; a resume runs none of them,
    // and a host putting saved values back needs the variables to exist first.
    override fun declareDatamodel() {
        ensureScriptEngine()
    }

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
            "test230__sce_synth_invoke__invoke_0",
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
            raisePlatformError(Test230SceSynthInvokeInvoke0Event.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(Test230SceSynthInvokeInvoke0Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test230SceSynthInvokeInvoke0Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test230SceSynthInvokeInvoke0Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test230SceSynthInvokeInvoke0Event) {
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
        // §scxml-B-2-8-1: the binding answers which rung the payload got, and
        // that answer used to end here. The ladder decided between a DOM, a
        // value and a space-normalized string, and the decision was dropped —
        // so a payload that announced structure and would not parse reached
        // the document as raw characters, every `_event.data.<field>` read
        // empty, and nothing anywhere could say so.
        //
        // Recorded on the spot rather than returned up: this class extends
        // `StateMachineEngine`, so the frame that binds already holds both the
        // reading and the event it belongs to — which is the pairing the count
        // needs.
        val payloadReading = engine.setCurrentEvent(
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
        notePayloadReading(event, payloadReading)
    }


    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: Test230SceSynthInvokeInvoke0State,
        event: Test230SceSynthInvokeInvoke0Event
    ): TransitionResult<Test230SceSynthInvokeInvoke0State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test230SceSynthInvokeInvoke0State.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test230SceSynthInvokeInvoke0Event
    ): TransitionResult<Test230SceSynthInvokeInvoke0State> = when {
        event is Test230SceSynthInvokeInvoke0Event.ChildToParent -> TransitionResult.External(Test230SceSynthInvokeInvoke0State.SubFinal, Test230SceSynthInvokeInvoke0State.Sub0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test230SceSynthInvokeInvoke0State.SubFinal, Test230SceSynthInvokeInvoke0State.Sub0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test230__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onEntry(state: Test230SceSynthInvokeInvoke0State, pathChild: Test230SceSynthInvokeInvoke0State?) {
        when (state) {
            is Test230SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test230__sce_synth_invoke__invoke_0.scxml:4 :: sub0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")


            scheduleSend("__send_1", 2000L, Test230SceSynthInvokeInvoke0Event.Timeout)
            }
            is Test230SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test230__sce_synth_invoke__invoke_0.scxml:20 :: subFinal :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test230__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onExit(state: Test230SceSynthInvokeInvoke0State) {
        when (state) {
            is Test230SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test230__sce_synth_invoke__invoke_0.scxml:4 :: sub0 :: _state_body
                activeStateIds.remove("sub0")
            }
            is Test230SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test230__sce_synth_invoke__invoke_0.scxml:20 :: subFinal :: _state_body
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test230__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: Test230SceSynthInvokeInvoke0State,
        event: Test230SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        is Test230SceSynthInvokeInvoke0State.Sub0 -> when {
            event is Test230SceSynthInvokeInvoke0Event.ChildToParent -> {
                // SCE-MAP: test230__sce_synth_invoke__invoke_0.scxml:9 :: sub0 :: _transition_0

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("name is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.name")?.toString() ?: ""))
            } catch (_: Exception) {}

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("type is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.type")?.toString() ?: ""))
            } catch (_: Exception) {}

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("sendid is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.sendid")?.toString() ?: ""))
            } catch (_: Exception) {}

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("origin is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.origin")?.toString() ?: ""))
            } catch (_: Exception) {}

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("origintype is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.origintype")?.toString() ?: ""))
            } catch (_: Exception) {}

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("invokeid is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.invokeid")?.toString() ?: ""))
            } catch (_: Exception) {}

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("data is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.data")?.toString() ?: ""))
            } catch (_: Exception) {}
            }
            else -> {}
        }
        else -> {}
        }
    }
}
