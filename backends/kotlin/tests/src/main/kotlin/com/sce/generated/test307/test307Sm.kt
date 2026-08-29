// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: d849bd6da318bf2e0e2ded479e492140d12b6fd36b79eec0dafdecf30c12263b
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/307/test307.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test307.scxml:1 :: _machine

package com.sce.generated.test307

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test307State : State {
    data object Final : Test307State
    data object S0 : Test307State
    data object S1 : Test307State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test307Event : Event {
    data object Bar : Test307Event
    sealed interface Error : Test307Event {
        data object Self : Error
        data object Execution : Error
    }
    data object Foo : Test307Event
}
// --- State Machine (W3C SCXML) ---

class Test307StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test307State, Test307Event>(scriptEngine) {

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
    fun Var1(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var1")

    override val initialState: Test307State = Test307State.S0

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
    override fun resolveState(stateId: String): Test307State? = when (stateId) {
        "final" -> Test307State.Final
        "s0" -> Test307State.S0
        "s1" -> Test307State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test307State): String = when (state) {
        is Test307State.Final -> "final"
        is Test307State.S0 -> "s0"
        is Test307State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test307State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test307State): Int = when (state) {
        is Test307State.Final -> 2
        is Test307State.S0 -> 0
        is Test307State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test307Event? = when (name) {
        "bar" -> Test307Event.Bar
        "error" -> Test307Event.Error.Self
        "error.execution" -> Test307Event.Error.Execution
        "foo" -> Test307Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test307Event): String? = when (event) {
        is Test307Event.Bar -> "bar"
        is Test307Event.Error.Self -> "error"
        is Test307Event.Error.Execution -> "error.execution"
        is Test307Event.Foo -> "foo"
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
            "test307",
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
            raisePlatformError(Test307Event.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(Test307Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test307Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test307Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test307Event) {
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
        state: Test307State,
        event: Test307Event
    ): TransitionResult<Test307State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test307State.S0 -> processS0(event)
        is Test307State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test307Event
    ): TransitionResult<Test307State> = when {
        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test307Event.Error || event is Test307Event.Error.Execution) -> TransitionResult.External(Test307State.S1, Test307State.S0)

        event is Test307Event.Foo -> TransitionResult.External(Test307State.S1, Test307State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test307Event
    ): TransitionResult<Test307State> = when {
        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test307Event.Error || event is Test307Event.Error.Execution) -> TransitionResult.External(Test307State.Final, Test307State.S1)

        event is Test307Event.Bar -> TransitionResult.External(Test307State.Final, Test307State.S1)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test307.scxml:1 :: _machine
    override fun onEntry(state: Test307State, pathChild: Test307State?) {
        when (state) {
            is Test307State.Final -> {
                // SCE-MAP: test307.scxml:40 :: final :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("final")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test307State.S0 -> {
                // SCE-MAP: test307.scxml:8 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("entering s0 value of Var 1 is: : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", com.sce.runtime.ScriptSource.ecmascript("Var1"))?.toString() ?: ""))
            } catch (_: Exception) {}

            raiseInternal(Test307Event.Foo)
            }
            is Test307State.S1 -> {
                // SCE-MAP: test307.scxml:21 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
                // W3C SCXML 5.3: Late binding — initialize state-level datamodel on entry
                run {
                    ensureScriptEngine()
                    val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    try {
                        val v = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.ecmascript("1"))
                        engine.setVariable(sid, "Var1", v)
                    } catch (e: Exception) {
                        raisePlatformError(Test307Event.Error.Execution, "<data id='Var1'> expr failed to evaluate")
                    }
                }

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("entering s1, value of non-existent substructure of Var 1 is: : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", com.sce.runtime.ScriptSource.ecmascript("Var1.foo"))?.toString() ?: ""))
            } catch (_: Exception) {}

            raiseInternal(Test307Event.Bar)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test307.scxml:1 :: _machine
    override fun onExit(state: Test307State) {
        when (state) {
            is Test307State.Final -> {
                // SCE-MAP: test307.scxml:40 :: final :: _state_body
                activeStateIds.remove("final")
            }
            is Test307State.S0 -> {
                // SCE-MAP: test307.scxml:8 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test307State.S1 -> {
                // SCE-MAP: test307.scxml:21 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test307.scxml:1 :: _machine
    override fun executeTransitionActions(
        source: Test307State,
        event: Test307Event?
    ) {
        when (source) {
        is Test307State.S0 -> when {
            (event is Test307Event.Error || event is Test307Event.Error.Execution) -> {
                // SCE-MAP: test307.scxml:13 :: s0 :: _transition_0

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("error in state s0: " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", com.sce.runtime.ScriptSource.ecmascript("_event"))?.toString() ?: ""))
            } catch (_: Exception) {}
            }
            event is Test307Event.Foo -> {
                // SCE-MAP: test307.scxml:16 :: s0 :: _transition_1

            println("no error in s0")
            }
            else -> {}
        }
        is Test307State.S1 -> when {
            (event is Test307Event.Error || event is Test307Event.Error.Execution) -> {
                // SCE-MAP: test307.scxml:31 :: s1 :: _transition_0

            // W3C SCXML 4.7: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("error in state s1: " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", com.sce.runtime.ScriptSource.ecmascript("_event"))?.toString() ?: ""))
            } catch (_: Exception) {}
            }
            event is Test307Event.Bar -> {
                // SCE-MAP: test307.scxml:34 :: s1 :: _transition_1

            println("No error in s1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
