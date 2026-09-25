// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/580/test580.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test580.scxml:5 :: _machine

package com.sce.generated.test580

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test580State : State {
    data object Fail : Test580State
    data object P1 : Test580State
    data object Pass : Test580State
    data object S0 : Test580State
    data object S1 : Test580State
    data object S11 : Test580State
    data object S12 : Test580State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test580Event : Event {
    sealed interface Error : Test580Event {
        data object Execution : Error
    }
    data object Timeout : Test580Event
}
// --- State Machine (W3C SCXML) ---

class Test580StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test580State, Test580Event>(scriptEngine) {

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

    override val initialState: Test580State = Test580State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test580State): Test580State? = when (state) {
        is Test580State.S0 -> Test580State.P1
        is Test580State.S1 -> Test580State.P1
        is Test580State.S11 -> Test580State.S1
        is Test580State.S12 -> Test580State.S1
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test580State): Boolean = when (state) {
        is Test580State.S1 -> true
        else -> false
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test580State): Boolean = when (state) {
        is Test580State.P1 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test580State): Boolean = when (state) {
        is Test580State.Fail, is Test580State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test580State): List<Test580State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test580State): List<EntryTarget<Test580State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test580State, HistoryId>>
        get() = documentInitialTargetList

    // W3C SCXML 3.10: the state a <history> is declared in.
    override fun historyParentOf(history: HistoryId): Test580State = historyParents.getValue(history)

    // W3C SCXML 3.10.2: a <history>'s default transition target, as written.
    override fun historyDefaultTargetsOf(history: HistoryId): List<EntryTarget<Test580State, HistoryId>> =
        historyDefaultTargets.getValue(history)

    // W3C SCXML 3.10: a state's <history> children, each with whether it is
    // deep — what the runtime records as the state is exited.
    override fun historiesOf(state: Test580State): List<Pair<HistoryId, Boolean>> =
        historiesByParent[state] ?: emptyList()

    private companion object {
        /** W3C SCXML 3.10: the `sh1` <history> (shallow). */
        val historySh1 = HistoryId(0)

        val childStates: Map<Test580State, List<Test580State>> = mapOf(
            Test580State.P1 to listOf(Test580State.S0, Test580State.S1),
            Test580State.S1 to listOf(Test580State.S11, Test580State.S12),
        )

        val initialTargets: Map<Test580State, List<EntryTarget<Test580State, HistoryId>>> = mapOf(
            Test580State.S1 to listOf(HistoryTarget(historySh1)),
        )

        val documentInitialTargetList: List<EntryTarget<Test580State, HistoryId>> =
            listOf(StateTarget(Test580State.P1))

        val historyParents: Map<HistoryId, Test580State> = mapOf(
            historySh1 to Test580State.S1,
        )

        val historyDefaultTargets: Map<HistoryId, List<EntryTarget<Test580State, HistoryId>>> = mapOf(
            historySh1 to listOf(StateTarget(Test580State.S11)),
        )

        val historiesByParent: Map<Test580State, List<Pair<HistoryId, Boolean>>> = mapOf(
            Test580State.S1 to listOf(historySh1 to false),
        )

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test580State, HistoryId>(
            Test580State.S0,
            listOf(StateTarget(Test580State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test580State, HistoryId>(
            Test580State.S0,
            listOf(StateTarget(Test580State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test580State, HistoryId>(
            Test580State.S1,
            listOf(StateTarget(Test580State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 1, as the microstep reads it.
        val transitionS1At1 = EnabledTransition<Test580State, HistoryId>(
            Test580State.S1,
            listOf(HistoryTarget(historySh1)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 2, as the microstep reads it.
        val transitionS1At2 = EnabledTransition<Test580State, HistoryId>(
            Test580State.S1,
            listOf(StateTarget(Test580State.Pass)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s11's transition 0, as the microstep reads it.
        val transitionS11At0 = EnabledTransition<Test580State, HistoryId>(
            Test580State.S11,
            listOf(StateTarget(Test580State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s11's transition 1, as the microstep reads it.
        val transitionS11At1 = EnabledTransition<Test580State, HistoryId>(
            Test580State.S11,
            listOf(StateTarget(Test580State.S12)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test580State? = when (stateId) {
        "fail" -> Test580State.Fail
        "p1" -> Test580State.P1
        "pass" -> Test580State.Pass
        "s0" -> Test580State.S0
        "s1" -> Test580State.S1
        "s11" -> Test580State.S11
        "s12" -> Test580State.S12
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test580State): String = when (state) {
        is Test580State.Fail -> "fail"
        is Test580State.P1 -> "p1"
        is Test580State.Pass -> "pass"
        is Test580State.S0 -> "s0"
        is Test580State.S1 -> "s1"
        is Test580State.S11 -> "s11"
        is Test580State.S12 -> "s12"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test580State): Int = when (state) {
        is Test580State.Fail -> 6
        is Test580State.P1 -> 0
        is Test580State.Pass -> 5
        is Test580State.S0 -> 1
        is Test580State.S1 -> 2
        is Test580State.S11 -> 3
        is Test580State.S12 -> 4
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test580Event? = when (name) {
        "error.execution" -> Test580Event.Error.Execution
        "timeout" -> Test580Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test580Event): String? = when (event) {
        is Test580Event.Error.Execution -> "error.execution"
        is Test580Event.Timeout -> "timeout"
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
            "test580",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raisePlatformError(Test580Event.Error.Execution, "<data id='Var1'> expr failed to evaluate")
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
            raisePlatformError(Test580Event.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(Test580Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test580Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test580Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test580Event) {
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



    // W3C SCXML 5.10: bind the event as the `_event` its transitions' guards
    // read — once, before the first guard runs, and not for an eventless
    // selection, which has no event of its own.
    override fun bindCurrentEvent(event: Test580Event) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test580State,
        event: Test580Event?
    ): EnabledTransition<Test580State, HistoryId>? = when (state) {
        is Test580State.S0 -> when {
            event == null && isStateActive("sh1") -> transitionS0At0
            event is Test580Event.Timeout -> transitionS0At1
            else -> null
        }
        is Test580State.S1 -> when {
            event == null && isStateActive("sh1") -> transitionS1At0
            event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(Var1, 0)", "Var1 == 0")) -> transitionS1At1
            event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(Var1, 1)", "Var1 == 1")) -> transitionS1At2
            else -> null
        }
        is Test580State.S11 -> when {
            event == null && isStateActive("sh1") -> transitionS11At0
            event == null -> transitionS11At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test580.scxml:5 :: _machine
    override fun onEntry(state: Test580State, isDefaultEntry: Boolean) {
        when (state) {
            is Test580State.Fail -> {
                // SCE-MAP: test580.scxml:50 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test580State.P1 -> {
                // SCE-MAP: test580.scxml:10 :: p1 :: _state_body


            scheduleSend("__send_0", 2000L, Test580Event.Timeout)
            }
            is Test580State.Pass -> {
                // SCE-MAP: test580.scxml:49 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test580State.S0 -> {
                // SCE-MAP: test580.scxml:16 :: s0 :: _state_body
            }
            is Test580State.S1 -> {
                // SCE-MAP: test580.scxml:22 :: s1 :: _state_body
            }
            is Test580State.S11 -> {
                // SCE-MAP: test580.scxml:32 :: s11 :: _state_body
            }
            is Test580State.S12 -> {
                // SCE-MAP: test580.scxml:37 :: s12 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test580.scxml:5 :: _machine
    override fun onExit(state: Test580State) {
        when (state) {
            is Test580State.Fail -> {
                // SCE-MAP: test580.scxml:50 :: fail :: _state_body
            }
            is Test580State.P1 -> {
                // SCE-MAP: test580.scxml:10 :: p1 :: _state_body
            }
            is Test580State.Pass -> {
                // SCE-MAP: test580.scxml:49 :: pass :: _state_body
            }
            is Test580State.S0 -> {
                // SCE-MAP: test580.scxml:16 :: s0 :: _state_body
            }
            is Test580State.S1 -> {
                // SCE-MAP: test580.scxml:22 :: s1 :: _state_body


            executeAssign(com.sce.runtime.ScriptSource.lua("Var1", "Var1"), com.sce.runtime.ScriptSource.lua("_scxml_add(Var1, 1)", "Var1 + 1"))
            }
            is Test580State.S11 -> {
                // SCE-MAP: test580.scxml:32 :: s11 :: _state_body
            }
            is Test580State.S12 -> {
                // SCE-MAP: test580.scxml:37 :: s12 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test580.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test580State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
