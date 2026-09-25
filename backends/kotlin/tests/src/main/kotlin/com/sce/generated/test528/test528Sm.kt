// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/528/test528.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test528.scxml:4 :: _machine

package com.sce.generated.test528

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test528State : State {
    data object Fail : Test528State
    data object Pass : Test528State
    data object S0 : Test528State
    data object S01 : Test528State
    data object S02 : Test528State
    data object S1 : Test528State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test528Event : Event {
    sealed interface Done : Test528Event {
        sealed interface State : Done {
            data object S0 : State
        }
    }
    sealed interface Error : Test528Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test528StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test528State, Test528Event>(scriptEngine) {

    override val initialState: Test528State = Test528State.S01

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

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
    override fun parentOf(state: Test528State): Test528State? = when (state) {
        is Test528State.S01 -> Test528State.S0
        is Test528State.S02 -> Test528State.S0
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test528State): Boolean = when (state) {
        is Test528State.S0 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test528State): Boolean = when (state) {
        is Test528State.Fail, is Test528State.Pass, is Test528State.S02 -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test528State): List<Test528State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test528State): List<EntryTarget<Test528State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test528State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test528State, List<Test528State>> = mapOf(
            Test528State.S0 to listOf(Test528State.S01, Test528State.S02),
        )

        val initialTargets: Map<Test528State, List<EntryTarget<Test528State, HistoryId>>> = mapOf(
            Test528State.S0 to listOf(StateTarget(Test528State.S01)),
        )

        val documentInitialTargetList: List<EntryTarget<Test528State, HistoryId>> =
            listOf(StateTarget(Test528State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test528State, HistoryId>(
            Test528State.S0,
            listOf(StateTarget(Test528State.S1)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test528State, HistoryId>(
            Test528State.S0,
            listOf(StateTarget(Test528State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 0, as the microstep reads it.
        val transitionS01At0 = EnabledTransition<Test528State, HistoryId>(
            Test528State.S01,
            listOf(StateTarget(Test528State.S02)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test528State, HistoryId>(
            Test528State.S1,
            listOf(StateTarget(Test528State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 1, as the microstep reads it.
        val transitionS1At1 = EnabledTransition<Test528State, HistoryId>(
            Test528State.S1,
            listOf(StateTarget(Test528State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test528State? = when (stateId) {
        "fail" -> Test528State.Fail
        "pass" -> Test528State.Pass
        "s0" -> Test528State.S0
        "s01" -> Test528State.S01
        "s02" -> Test528State.S02
        "s1" -> Test528State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test528State): String = when (state) {
        is Test528State.Fail -> "fail"
        is Test528State.Pass -> "pass"
        is Test528State.S0 -> "s0"
        is Test528State.S01 -> "s01"
        is Test528State.S02 -> "s02"
        is Test528State.S1 -> "s1"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test528State): Int = when (state) {
        is Test528State.Fail -> 5
        is Test528State.Pass -> 4
        is Test528State.S0 -> 0
        is Test528State.S01 -> 1
        is Test528State.S02 -> 2
        is Test528State.S1 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test528Event? = when (name) {
        "done.state.s0" -> Test528Event.Done.State.S0
        "error.execution" -> Test528Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test528Event): String? = when (event) {
        is Test528Event.Done.State.S0 -> "done.state.s0"
        is Test528Event.Error.Execution -> "error.execution"
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
            "test528",
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
            raisePlatformError(Test528Event.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(Test528Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test528Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test528Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test528Event) {
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
    override fun bindCurrentEvent(event: Test528Event) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test528State,
        event: Test528Event?
    ): EnabledTransition<Test528State, HistoryId>? = when (state) {
        is Test528State.S0 -> when {
            event is Test528Event.Error.Execution -> transitionS0At0
            event is Test528Event.Done.State.S0 -> transitionS0At1
            else -> null
        }
        is Test528State.S01 -> when {
            event == null -> transitionS01At0
            else -> null
        }
        is Test528State.S1 -> when {
            event is Test528Event.Done.State.S0 -> transitionS1At0
            event != null -> transitionS1At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test528.scxml:4 :: _machine
    override fun onEntry(state: Test528State, isDefaultEntry: Boolean) {
        when (state) {
            is Test528State.Fail -> {
                // SCE-MAP: test528.scxml:33 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test528State.Pass -> {
                // SCE-MAP: test528.scxml:32 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test528State.S0 -> {
                // SCE-MAP: test528.scxml:7 :: s0 :: _state_body
            }
            is Test528State.S01 -> {
                // SCE-MAP: test528.scxml:13 :: s01 :: _state_body
            }
            is Test528State.S02 -> {
                // SCE-MAP: test528.scxml:16 :: s02 :: _state_body
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    ensureScriptEngine()
                    val engineDD = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidDD = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    var doneEventData = ""
                    // W3C SCXML 5.5: Evaluate <content expr="..."/>
                    try {
                        val contentResult = engineDD.evaluateExpr(sidDD, com.sce.runtime.ScriptSource.lua("nil.invalidProperty", "undefined.invalidProperty"))
                        // C++ DoneDataHelper::evaluateContent: EventDataHelper::scriptValueToJsonString
                        doneEventData = if (contentResult != null) valueToJson(contentResult) else ""
                    } catch (_: Exception) {
                        raisePlatformError(Test528Event.Error.Execution, "<donedata> <content expr> failed to evaluate")
                    }
                    // W3C SCXML 3.7: Final child state reached, raise done.state with data
                    raiseInternal(Test528Event.Done.State.S0, EventMetadata.platform(doneEventData))
                }
            }
            is Test528State.S1 -> {
                // SCE-MAP: test528.scxml:27 :: s1 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test528.scxml:4 :: _machine
    override fun onExit(state: Test528State) {
        when (state) {
            is Test528State.Fail -> {
                // SCE-MAP: test528.scxml:33 :: fail :: _state_body
            }
            is Test528State.Pass -> {
                // SCE-MAP: test528.scxml:32 :: pass :: _state_body
            }
            is Test528State.S0 -> {
                // SCE-MAP: test528.scxml:7 :: s0 :: _state_body
            }
            is Test528State.S01 -> {
                // SCE-MAP: test528.scxml:13 :: s01 :: _state_body
            }
            is Test528State.S02 -> {
                // SCE-MAP: test528.scxml:16 :: s02 :: _state_body
            }
            is Test528State.S1 -> {
                // SCE-MAP: test528.scxml:27 :: s1 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test528.scxml:4 :: _machine
    override fun executeTransitionContent(source: Test528State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
