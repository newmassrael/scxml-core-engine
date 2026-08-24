// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 580f12cd61336d7449660775c4fcc4f615ee3c32bffa0e9792363e260aed93e2
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/557/test557.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test557.scxml:5 :: _machine

package com.sce.generated.test557

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test557State : State {
    data object Fail : Test557State
    data object Pass : Test557State
    data object S0 : Test557State
    data object S1 : Test557State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test557Event : Event {
    sealed interface Error : Test557Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test557StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test557State, Test557Event>(scriptEngine) {

    override val initialState: Test557State = Test557State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test557State? = when (stateId) {
        "fail" -> Test557State.Fail
        "pass" -> Test557State.Pass
        "s0" -> Test557State.S0
        "s1" -> Test557State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test557State): String = when (state) {
        is Test557State.Fail -> "fail"
        is Test557State.Pass -> "pass"
        is Test557State.S0 -> "s0"
        is Test557State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test557State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test557State): Int = when (state) {
        is Test557State.Fail -> 3
        is Test557State.Pass -> 2
        is Test557State.S0 -> 0
        is Test557State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test557Event? = when (name) {
        "error.execution" -> Test557Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test557Event): String? = when (event) {
        is Test557Event.Error.Execution -> "error.execution"
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
            "test557",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML B.2: Initialize variable 'var1' with inline content (C++ parseEventData pattern)
        try {
            val initResult_var1 = engine.parseDataValue(sid, "<books xmlns=\"\">\n     <book title=\"title1\"></book>\n     <book title=\"title2\"></book>\n   </books>")
            engine.setVariable(sid, "var1", initResult_var1)
        } catch (e: Exception) {
            raisePlatformError(Test557Event.Error.Execution, "<data id='var1'> content failed to initialise")
        }
        // W3C SCXML 5.2.2: Load variable 'var2' from external source (C++ DataModelInitHelper pattern)
        try {
            val srcContent_var2 = engine.loadDataFromSrc("file:test557.txt", "resources/557")
            if (srcContent_var2 != null) {
                val srcValue_var2 = engine.parseDataValue(sid, srcContent_var2)
                engine.setVariable(sid, "var2", srcValue_var2)
            }
        } catch (e: Exception) {
            raisePlatformError(Test557Event.Error.Execution, "<data id='var2' src='file:test557.txt'> could not be loaded")
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
            raisePlatformError(Test557Event.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(Test557Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test557Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test557Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test557Event) {
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
        state: Test557State,
        event: Test557Event
    ): TransitionResult<Test557State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test557State
    ): TransitionResult<Test557State> = when (state) {
        is Test557State.S0 -> processNullS0()
        is Test557State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test557State> = when {
        safeEvaluateGuard("var1.getElementsByTagName('book')[0].getAttribute('title') == 'title1'") -> TransitionResult.External(Test557State.S1, Test557State.S0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test557State.Fail, Test557State.S0)
    }

    private fun processNullS1(
    ): TransitionResult<Test557State> = when {
        safeEvaluateGuard("var2.getElementsByTagName('book')[1].getAttribute('title') == 'title2'") -> TransitionResult.External(Test557State.Pass, Test557State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test557State.Fail, Test557State.S1)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test557.scxml:5 :: _machine
    override fun onEntry(state: Test557State, pathChild: Test557State?) {
        when (state) {
            is Test557State.Fail -> {
                // SCE-MAP: test557.scxml:27 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test557State.Pass -> {
                // SCE-MAP: test557.scxml:26 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test557State.S0 -> {
                // SCE-MAP: test557.scxml:16 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test557State.S1 -> {
                // SCE-MAP: test557.scxml:21 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test557.scxml:5 :: _machine
    override fun onExit(state: Test557State) {
        when (state) {
            is Test557State.Fail -> {
                // SCE-MAP: test557.scxml:27 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test557State.Pass -> {
                // SCE-MAP: test557.scxml:26 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test557State.S0 -> {
                // SCE-MAP: test557.scxml:16 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test557State.S1 -> {
                // SCE-MAP: test557.scxml:21 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test557.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test557State,
        event: Test557Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
