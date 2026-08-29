// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d849bd6da318bf2e0e2ded479e492140d12b6fd36b79eec0dafdecf30c12263b
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/343/test343.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test343.scxml:4 :: _machine

package com.sce.generated.test343

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test343State : State {
    data object Fail : Test343State
    data object Pass : Test343State
    data object S0 : Test343State
    data object S01 : Test343State
    data object S02 : Test343State
    data object S1 : Test343State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test343Event : Event {
    sealed interface Done : Test343Event {
        sealed interface State : Done {
            data object S0 : State
        }
    }
    sealed interface Error : Test343Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test343StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test343State, Test343Event>(scriptEngine) {

    override val initialState: Test343State = Test343State.S01

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test343State): Test343State? = when (state) {
        is Test343State.S01 -> Test343State.S0
        is Test343State.S02 -> Test343State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test343State): Test343State = when (state) {
        is Test343State.S0 -> Test343State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test343State? = when (stateId) {
        "fail" -> Test343State.Fail
        "pass" -> Test343State.Pass
        "s0" -> Test343State.S0
        "s01" -> Test343State.S01
        "s02" -> Test343State.S02
        "s1" -> Test343State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test343State): String = when (state) {
        is Test343State.Fail -> "fail"
        is Test343State.Pass -> "pass"
        is Test343State.S0 -> "s0"
        is Test343State.S01 -> "s01"
        is Test343State.S02 -> "s02"
        is Test343State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test343State): Boolean = when (state) {
        is Test343State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test343State): Int = when (state) {
        is Test343State.Fail -> 5
        is Test343State.Pass -> 4
        is Test343State.S0 -> 0
        is Test343State.S01 -> 1
        is Test343State.S02 -> 2
        is Test343State.S1 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test343Event? = when (name) {
        "done.state.s0" -> Test343Event.Done.State.S0
        "error.execution" -> Test343Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test343Event): String? = when (event) {
        is Test343Event.Done.State.S0 -> "done.state.s0"
        is Test343Event.Error.Execution -> "error.execution"
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
            "test343",
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
    //
    // The guard arrives as a `ScriptSource`, not a `String`: it carries the
    // language its text is in, so a machine generated for a Lua engine hands
    // over Lua the build-time frontend produced and one generated for an
    // ECMAScript engine hands over the author's own text — and the engine is
    // never left to guess which it got. The C++ sibling
    // (`process_transition.jinja2`) takes the same argument for the same
    // reason.
    private fun safeEvaluateGuard(guardExpr: com.sce.runtime.ScriptSource): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(Test343Event.Error.Execution, "a <transition> cond failed to evaluate")
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
    //
    // The serialization wraps BOTH halves, in each half's own language. A
    // wrapper composed around one of them only would build a `ScriptSource`
    // whose two strings no longer say the same thing, and the diagnostic that
    // reads `source` would name an expression the engine never ran. `JSON` is
    // a §scxml-B-2-9 name both engines carry, so the wrapper is the same eight
    // characters on either arm — what differs is what it wraps.
    private fun evaluateSendContent(source: com.sce.runtime.ScriptSource): String {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        val serialized = when (source.language) {
            com.sce.runtime.ScriptLanguage.ECMAScript ->
                com.sce.runtime.ScriptSource.ecmascript("JSON.stringify((" + source.source + "))")
            com.sce.runtime.ScriptLanguage.Lua ->
                com.sce.runtime.ScriptSource.lua(
                    "JSON.stringify((" + source.text + "))",
                    "JSON.stringify((" + source.source + "))",
                )
        }
        return try {
            engine.evaluateExpr(sid, serialized)?.toString() ?: ""
        } catch (e: Exception) {
            raisePlatformError(Test343Event.Error.Execution, "an expression could not be serialised to JSON")
            ""
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    //
    // Both halves carry a language: this engine's Lua arm splices the location
    // in front of `=` and runs the result, so a write target written in
    // ECMAScript has to have been lowered too. Same split as
    // `ScxmlScriptEngine.assign`.
    private fun executeAssign(location: com.sce.runtime.ScriptSource, expr: com.sce.runtime.ScriptSource) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raisePlatformError(Test343Event.Error.Execution, "<assign> failed")
        }
    }

    // W3C SCXML 5.8: Script block execution
    private fun executeScriptBlock(script: com.sce.runtime.ScriptSource) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raisePlatformError(Test343Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test343Event) {
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
        state: Test343State,
        event: Test343Event
    ): TransitionResult<Test343State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test343State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test343State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test343State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test343State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test343State
    ): TransitionResult<Test343State> = when (state) {
        is Test343State.S01 -> processNullS01()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS01(
    ): TransitionResult<Test343State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test343State.S02, Test343State.S01)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test343Event
    ): TransitionResult<Test343State> = when {
        event is Test343Event.Error.Execution -> TransitionResult.External(Test343State.S1, Test343State.S0)

        event is Test343Event.Done.State.S0 -> TransitionResult.External(Test343State.Fail, Test343State.S0)

        event is Test343Event.Done.State.S0 -> TransitionResult.External(Test343State.Fail, Test343State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test343Event
    ): TransitionResult<Test343State> = when {
        event is Test343Event.Done.State.S0 -> TransitionResult.External(Test343State.Pass, Test343State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test343State.Fail, Test343State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test343.scxml:4 :: _machine
    override fun onEntry(state: Test343State, pathChild: Test343State?) {
        when (state) {
            is Test343State.Fail -> {
                // SCE-MAP: test343.scxml:35 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test343State.Pass -> {
                // SCE-MAP: test343.scxml:34 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test343State.S0 -> {
                // SCE-MAP: test343.scxml:7 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test343State.S01 -> {
                // SCE-MAP: test343.scxml:15 :: s01 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test343State.S02 -> {
                // SCE-MAP: test343.scxml:18 :: s02 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    ensureScriptEngine()
                    val engineDD = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidDD = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    var doneEventData = ""
                    // W3C SCXML 5.5: Evaluate <param> elements (C++ DoneDataHelper::evaluateParams pattern)
                    val doneParams = mutableMapOf<String, Any?>()
                    var doneParamStructuralError = false
                    try {
                        val locVal = engineDD.evaluateExpr(sidDD, com.sce.runtime.ScriptSource.ecmascript("foo"))
                        doneParams["someParam"] = locVal
                    } catch (_: Exception) {
                        // W3C SCXML 5.7: Runtime location error — raise error.execution but continue
                        raisePlatformError(Test343Event.Error.Execution, "<donedata> <param name='someParam'> location could not be read")
                    }
                    // C++ DoneDataHelper pattern: if (!success) break — skip done.state on structural error only
                    if (doneParamStructuralError) return@run
                    if (doneParams.isNotEmpty()) {
                        doneEventData = buildJsonFromParams(doneParams)
                    }
                    // W3C SCXML 3.7: Final child state reached, raise done.state with data
                    raiseInternal(Test343Event.Done.State.S0, EventMetadata.platform(doneEventData))
                }
            }
            is Test343State.S1 -> {
                // SCE-MAP: test343.scxml:29 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test343.scxml:4 :: _machine
    override fun onExit(state: Test343State) {
        when (state) {
            is Test343State.Fail -> {
                // SCE-MAP: test343.scxml:35 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test343State.Pass -> {
                // SCE-MAP: test343.scxml:34 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test343State.S0 -> {
                // SCE-MAP: test343.scxml:7 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test343State.S01 -> {
                // SCE-MAP: test343.scxml:15 :: s01 :: _state_body
                activeStateIds.remove("s01")
            }
            is Test343State.S02 -> {
                // SCE-MAP: test343.scxml:18 :: s02 :: _state_body
                activeStateIds.remove("s02")
            }
            is Test343State.S1 -> {
                // SCE-MAP: test343.scxml:29 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test343.scxml:4 :: _machine
    override fun executeTransitionActions(
        source: Test343State,
        event: Test343Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
