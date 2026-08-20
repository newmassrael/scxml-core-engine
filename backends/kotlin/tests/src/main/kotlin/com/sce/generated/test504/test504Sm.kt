// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4382370ad28e3e273e1d105876d814053809a7d5b704c5d43426b4c872443a55
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/504/test504.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test504.scxml:5 :: _machine

package com.sce.generated.test504

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test504State : State {
    data object Fail : Test504State
    data object P : Test504State
    data object Pass : Test504State
    data object Ps1 : Test504State
    data object Ps2 : Test504State
    data object S1 : Test504State
    data object S2 : Test504State
    data object S3 : Test504State
    data object S4 : Test504State
    data object S5 : Test504State
    data object S6 : Test504State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test504Event : Event {
    data object Bar : Test504Event
    sealed interface Error : Test504Event {
        data object Execution : Error
    }
    data object Foo : Test504Event
}
// --- State Machine (W3C SCXML) ---

class Test504StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test504State, Test504Event>(scriptEngine) {

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

    /**
     * §scxml-5.3: what the `Var2` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var2` was assigned a value of another type, or the engine refused.
     */
    fun Var2(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var2")

    /**
     * §scxml-5.3: what the `Var3` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var3` was assigned a value of another type, or the engine refused.
     */
    fun Var3(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var3")

    /**
     * §scxml-5.3: what the `Var4` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var4` was assigned a value of another type, or the engine refused.
     */
    fun Var4(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var4")

    /**
     * §scxml-5.3: what the `Var5` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var5` was assigned a value of another type, or the engine refused.
     */
    fun Var5(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var5")

    override val initialState: Test504State = Test504State.S1

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
    override fun parentOf(state: Test504State): Test504State? = when (state) {
        is Test504State.P -> Test504State.S2
        is Test504State.Ps1 -> Test504State.P
        is Test504State.Ps2 -> Test504State.P
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test504State): Test504State = when (state) {
        is Test504State.P -> Test504State.Ps1
        is Test504State.S2 -> Test504State.Ps1
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test504State? = when (stateId) {
        "fail" -> Test504State.Fail
        "p" -> Test504State.P
        "pass" -> Test504State.Pass
        "ps1" -> Test504State.Ps1
        "ps2" -> Test504State.Ps2
        "s1" -> Test504State.S1
        "s2" -> Test504State.S2
        "s3" -> Test504State.S3
        "s4" -> Test504State.S4
        "s5" -> Test504State.S5
        "s6" -> Test504State.S6
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test504State): String = when (state) {
        is Test504State.Fail -> "fail"
        is Test504State.P -> "p"
        is Test504State.Pass -> "pass"
        is Test504State.Ps1 -> "ps1"
        is Test504State.Ps2 -> "ps2"
        is Test504State.S1 -> "s1"
        is Test504State.S2 -> "s2"
        is Test504State.S3 -> "s3"
        is Test504State.S4 -> "s4"
        is Test504State.S5 -> "s5"
        is Test504State.S6 -> "s6"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test504State): Boolean = when (state) {
        is Test504State.P -> false
        is Test504State.S2 -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test504State): Boolean = when (state) {
        is Test504State.P -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test504State): List<Test504State> = when (state) {
        is Test504State.P -> listOf(Test504State.Ps1, Test504State.Ps2)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test504State): Int = when (state) {
        is Test504State.Fail -> 10
        is Test504State.P -> 2
        is Test504State.Pass -> 9
        is Test504State.Ps1 -> 3
        is Test504State.Ps2 -> 4
        is Test504State.S1 -> 0
        is Test504State.S2 -> 1
        is Test504State.S3 -> 5
        is Test504State.S4 -> 6
        is Test504State.S5 -> 7
        is Test504State.S6 -> 8
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test504Event? = when (name) {
        "bar" -> Test504Event.Bar
        "error.execution" -> Test504Event.Error.Execution
        "foo" -> Test504Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test504Event): String? = when (event) {
        is Test504Event.Bar -> "bar"
        is Test504Event.Error.Execution -> "error.execution"
        is Test504Event.Foo -> "foo"
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
            "test504",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raisePlatformError(Test504Event.Error.Execution, "<data id='Var1'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'Var2' with expr
        try {
            val initResult_Var2 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var2", initResult_Var2)
        } catch (e: Exception) {
            raisePlatformError(Test504Event.Error.Execution, "<data id='Var2'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'Var3' with expr
        try {
            val initResult_Var3 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var3", initResult_Var3)
        } catch (e: Exception) {
            raisePlatformError(Test504Event.Error.Execution, "<data id='Var3'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'Var4' with expr
        try {
            val initResult_Var4 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var4", initResult_Var4)
        } catch (e: Exception) {
            raisePlatformError(Test504Event.Error.Execution, "<data id='Var4'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'Var5' with expr
        try {
            val initResult_Var5 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var5", initResult_Var5)
        } catch (e: Exception) {
            raisePlatformError(Test504Event.Error.Execution, "<data id='Var5'> expr failed to evaluate")
        }



        // W3C SCXML 5.9.2: Register In() predicate callback
        engine.setStateQueryCallback(sid) { stateId -> isStateActive(stateId) }

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
            raisePlatformError(Test504Event.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(Test504Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test504Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test504Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test504Event) {
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
        state: Test504State,
        event: Test504Event
    ): TransitionResult<Test504State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        // W3C SCXML 3.13: Ancestor-only routing (ps1 has no own event transitions)
        is Test504State.Ps1 -> {
            val anc1 = processP(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (ps2 has no own event transitions)
        is Test504State.Ps2 -> {
            val anc1 = processP(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test504State
    ): TransitionResult<Test504State> = when (state) {
        is Test504State.S1 -> processNullS1()
        is Test504State.S3 -> processNullS3()
        is Test504State.S4 -> processNullS4()
        is Test504State.S5 -> processNullS5()
        is Test504State.S6 -> processNullS6()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test504State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test504State.P, Test504State.S1)
    }

    private fun processNullS3(
    ): TransitionResult<Test504State> = when {
        safeEvaluateGuard("Var1 == 2") -> TransitionResult.External(Test504State.S4, Test504State.S3)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test504State.Fail, Test504State.S3)
    }

    private fun processNullS4(
    ): TransitionResult<Test504State> = when {
        safeEvaluateGuard("Var2 == 2") -> TransitionResult.External(Test504State.S5, Test504State.S4)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test504State.Fail, Test504State.S4)
    }

    private fun processNullS5(
    ): TransitionResult<Test504State> = when {
        safeEvaluateGuard("Var3 == 2") -> TransitionResult.External(Test504State.S6, Test504State.S5)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test504State.Fail, Test504State.S5)
    }

    private fun processNullS6(
    ): TransitionResult<Test504State> = when {
        safeEvaluateGuard("Var5 == 1") -> TransitionResult.External(Test504State.Pass, Test504State.S6)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test504State.Fail, Test504State.S6)
    }

    // --- Per-State Event Handlers ---

    private fun processP(
        event: Test504Event
    ): TransitionResult<Test504State> = when {
        event is Test504Event.Foo -> TransitionResult.External(Test504State.Ps1, Test504State.P)

        event is Test504Event.Bar && safeEvaluateGuard("Var4 == 1") -> TransitionResult.External(Test504State.S3, Test504State.P)

        event is Test504Event.Bar -> TransitionResult.External(Test504State.Fail, Test504State.P)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test504.scxml:5 :: _machine
    override fun onEntry(state: Test504State, pathChild: Test504State?) {
        when (state) {
            is Test504State.Fail -> {
                // SCE-MAP: test504.scxml:77 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test504State.P -> {
                // SCE-MAP: test504.scxml:27 :: p :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p")) return
                // W3C SCXML 3.4 + §scxml-D-addDescendantStatesToEnter: a
                // `<parallel>` hands out defaults even when it is only an
                // ancestor — Appendix D's one exception to the ancestor rule.
                // The exception has its own exception: not the region the entry
                // set is already descending into, which `pathChild` names and
                // which the caller enters with the target's own path.
                if (pathChild != Test504State.Ps1) {
                    onEntry(Test504State.Ps1)
                }
                if (pathChild != Test504State.Ps2) {
                    onEntry(Test504State.Ps2)
                }
            }
            is Test504State.Pass -> {
                // SCE-MAP: test504.scxml:76 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test504State.Ps1 -> {
                // SCE-MAP: test504.scxml:39 :: ps1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("ps1")) return
            }
            is Test504State.Ps2 -> {
                // SCE-MAP: test504.scxml:44 :: ps2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("ps2")) return
            }
            is Test504State.S1 -> {
                // SCE-MAP: test504.scxml:14 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return

            raiseInternal(Test504Event.Foo)

            raiseInternal(Test504Event.Bar)
            }
            is Test504State.S2 -> {
                // SCE-MAP: test504.scxml:22 :: s2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test504State.P)
                }
            }
            is Test504State.S3 -> {
                // SCE-MAP: test504.scxml:52 :: s3 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return
            }
            is Test504State.S4 -> {
                // SCE-MAP: test504.scxml:58 :: s4 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s4")) return
            }
            is Test504State.S5 -> {
                // SCE-MAP: test504.scxml:64 :: s5 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s5")) return
            }
            is Test504State.S6 -> {
                // SCE-MAP: test504.scxml:70 :: s6 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s6")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test504.scxml:5 :: _machine
    override fun onExit(state: Test504State) {
        when (state) {
            is Test504State.Fail -> {
                // SCE-MAP: test504.scxml:77 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test504State.P -> {
                // SCE-MAP: test504.scxml:27 :: p :: _state_body
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test504State, Int>>()
                if (activeStateIds.contains("ps1")) {
                    toExit.add(Test504State.Ps1 to 3)
                }
                if (activeStateIds.contains("ps2")) {
                    toExit.add(Test504State.Ps2 to 4)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("p")


            executeAssign("Var1", "Var1 + 1")
            }
            is Test504State.Pass -> {
                // SCE-MAP: test504.scxml:76 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test504State.Ps1 -> {
                // SCE-MAP: test504.scxml:39 :: ps1 :: _state_body
                activeStateIds.remove("ps1")


            executeAssign("Var2", "Var2 + 1")
            }
            is Test504State.Ps2 -> {
                // SCE-MAP: test504.scxml:44 :: ps2 :: _state_body
                activeStateIds.remove("ps2")


            executeAssign("Var3", "Var3 + 1")
            }
            is Test504State.S1 -> {
                // SCE-MAP: test504.scxml:14 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
            is Test504State.S2 -> {
                // SCE-MAP: test504.scxml:22 :: s2 :: _state_body
                activeStateIds.remove("s2")


            executeAssign("Var5", "Var5 + 1")
            }
            is Test504State.S3 -> {
                // SCE-MAP: test504.scxml:52 :: s3 :: _state_body
                activeStateIds.remove("s3")
            }
            is Test504State.S4 -> {
                // SCE-MAP: test504.scxml:58 :: s4 :: _state_body
                activeStateIds.remove("s4")
            }
            is Test504State.S5 -> {
                // SCE-MAP: test504.scxml:64 :: s5 :: _state_body
                activeStateIds.remove("s5")
            }
            is Test504State.S6 -> {
                // SCE-MAP: test504.scxml:70 :: s6 :: _state_body
                activeStateIds.remove("s6")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test504.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test504State,
        event: Test504Event?
    ) {
        when (source) {
        is Test504State.P -> when {
            event is Test504Event.Foo -> {
                // SCE-MAP: test504.scxml:31 :: p :: _transition_0


            executeAssign("Var4", "Var4 + 1")
            }
            else -> {}
        }
        is Test504State.Ps1 -> when {
            event is Test504Event.Foo -> {
                // SCE-MAP: test504.scxml:31 :: p :: _transition_0


            executeAssign("Var4", "Var4 + 1")
            }
            else -> {}
        }
        is Test504State.Ps2 -> when {
            event is Test504Event.Foo -> {
                // SCE-MAP: test504.scxml:31 :: p :: _transition_0


            executeAssign("Var4", "Var4 + 1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
