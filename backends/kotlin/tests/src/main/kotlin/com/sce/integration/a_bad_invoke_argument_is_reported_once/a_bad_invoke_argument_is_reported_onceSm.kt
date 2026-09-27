// SCE-GENERATED — DO NOT EDIT
// source-hash: 61a11c1135bc9b5297b489b5bdeec8ed213e07a866294a88591eded59b857e99

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_bad_invoke_argument_is_reported_once/a_bad_invoke_argument_is_reported_once.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:42 :: _machine

package com.sce.integration.a_bad_invoke_argument_is_reported_once

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ABadInvokeArgumentIsReportedOnceState : State {
    data object Done : ABadInvokeArgumentIsReportedOnceState
    data object Run : ABadInvokeArgumentIsReportedOnceState
    data object S0 : ABadInvokeArgumentIsReportedOnceState
    data object S1 : ABadInvokeArgumentIsReportedOnceState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ABadInvokeArgumentIsReportedOnceEvent : Event {
    data object ChildUp : ABadInvokeArgumentIsReportedOnceEvent
    sealed interface Done : ABadInvokeArgumentIsReportedOnceEvent {
        data object Invoke : Done
    }
    sealed interface Error : ABadInvokeArgumentIsReportedOnceEvent {
        data object Execution : Error
    }
    data object Finish : ABadInvokeArgumentIsReportedOnceEvent
    data object Go : ABadInvokeArgumentIsReportedOnceEvent
}
// --- State Machine (W3C SCXML) ---

class ABadInvokeArgumentIsReportedOnceStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ABadInvokeArgumentIsReportedOnceState, ABadInvokeArgumentIsReportedOnceEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `v` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `v` was assigned a value of another type, or the engine refused.
     */
    fun v(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "v")

    /**
     * §scxml-5.3: what the `obj` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `obj` was assigned a value of another type, or the engine refused.
     *
     * The value as JSON text, serialised by the engine's own `JSON.stringify`
     * (§scxml-B-2) so the key order is the document's.
     */
    fun obj(): String? =
        com.sce.runtime.DatamodelRead.readJson(scriptEngine, scriptSessionId, "obj")

    /**
     * §scxml-5.3: what the `errors` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `errors` was assigned a value of another type, or the engine refused.
     */
    fun errors(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "errors")

    /**
     * §scxml-5.3: what the `started` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `started` was assigned a value of another type, or the engine refused.
     */
    fun started(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "started")

    /**
     * §scxml-5.3: what the `fromLocOk` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `fromLocOk` was assigned a value of another type, or the engine refused.
     */
    fun fromLocOk(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "fromLocOk")

    /**
     * §scxml-5.3: what the `emptyLocLeftOut` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `emptyLocLeftOut` was assigned a value of another type, or the engine refused.
     */
    fun emptyLocLeftOut(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "emptyLocLeftOut")

    /**
     * §scxml-5.3: what the `brokenLeftOut` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `brokenLeftOut` was assigned a value of another type, or the engine refused.
     */
    fun brokenLeftOut(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "brokenLeftOut")

    override val initialState: ABadInvokeArgumentIsReportedOnceState = ABadInvokeArgumentIsReportedOnceState.S0

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
    override fun parentOf(state: ABadInvokeArgumentIsReportedOnceState): ABadInvokeArgumentIsReportedOnceState? = when (state) {
        is ABadInvokeArgumentIsReportedOnceState.S0 -> ABadInvokeArgumentIsReportedOnceState.Run
        is ABadInvokeArgumentIsReportedOnceState.S1 -> ABadInvokeArgumentIsReportedOnceState.Run
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: ABadInvokeArgumentIsReportedOnceState): Boolean = when (state) {
        is ABadInvokeArgumentIsReportedOnceState.Run -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: ABadInvokeArgumentIsReportedOnceState): Boolean = when (state) {
        is ABadInvokeArgumentIsReportedOnceState.Done -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: ABadInvokeArgumentIsReportedOnceState): List<ABadInvokeArgumentIsReportedOnceState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: ABadInvokeArgumentIsReportedOnceState): List<EntryTarget<ABadInvokeArgumentIsReportedOnceState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ABadInvokeArgumentIsReportedOnceState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<ABadInvokeArgumentIsReportedOnceState, List<ABadInvokeArgumentIsReportedOnceState>> = mapOf(
            ABadInvokeArgumentIsReportedOnceState.Run to listOf(ABadInvokeArgumentIsReportedOnceState.S0, ABadInvokeArgumentIsReportedOnceState.S1),
        )

        val initialTargets: Map<ABadInvokeArgumentIsReportedOnceState, List<EntryTarget<ABadInvokeArgumentIsReportedOnceState, HistoryId>>> = mapOf(
            ABadInvokeArgumentIsReportedOnceState.Run to listOf(StateTarget(ABadInvokeArgumentIsReportedOnceState.S0)),
        )

        val documentInitialTargetList: List<EntryTarget<ABadInvokeArgumentIsReportedOnceState, HistoryId>> =
            listOf(StateTarget(ABadInvokeArgumentIsReportedOnceState.S0))

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<ABadInvokeArgumentIsReportedOnceState, HistoryId>(
            ABadInvokeArgumentIsReportedOnceState.Run,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 1, as the microstep reads it.
        val transitionRunAt1 = EnabledTransition<ABadInvokeArgumentIsReportedOnceState, HistoryId>(
            ABadInvokeArgumentIsReportedOnceState.Run,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 2, as the microstep reads it.
        val transitionRunAt2 = EnabledTransition<ABadInvokeArgumentIsReportedOnceState, HistoryId>(
            ABadInvokeArgumentIsReportedOnceState.Run,
            listOf(StateTarget(ABadInvokeArgumentIsReportedOnceState.Done)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<ABadInvokeArgumentIsReportedOnceState, HistoryId>(
            ABadInvokeArgumentIsReportedOnceState.S0,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<ABadInvokeArgumentIsReportedOnceState, HistoryId>(
            ABadInvokeArgumentIsReportedOnceState.S0,
            listOf(StateTarget(ABadInvokeArgumentIsReportedOnceState.S1)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ABadInvokeArgumentIsReportedOnceState? = when (stateId) {
        "done" -> ABadInvokeArgumentIsReportedOnceState.Done
        "run" -> ABadInvokeArgumentIsReportedOnceState.Run
        "s0" -> ABadInvokeArgumentIsReportedOnceState.S0
        "s1" -> ABadInvokeArgumentIsReportedOnceState.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ABadInvokeArgumentIsReportedOnceState): String = when (state) {
        is ABadInvokeArgumentIsReportedOnceState.Done -> "done"
        is ABadInvokeArgumentIsReportedOnceState.Run -> "run"
        is ABadInvokeArgumentIsReportedOnceState.S0 -> "s0"
        is ABadInvokeArgumentIsReportedOnceState.S1 -> "s1"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ABadInvokeArgumentIsReportedOnceState): Int = when (state) {
        is ABadInvokeArgumentIsReportedOnceState.Done -> 3
        is ABadInvokeArgumentIsReportedOnceState.Run -> 0
        is ABadInvokeArgumentIsReportedOnceState.S0 -> 1
        is ABadInvokeArgumentIsReportedOnceState.S1 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ABadInvokeArgumentIsReportedOnceEvent? = when (name) {
        "childUp" -> ABadInvokeArgumentIsReportedOnceEvent.ChildUp
        "done.invoke" -> ABadInvokeArgumentIsReportedOnceEvent.Done.Invoke
        "error.execution" -> ABadInvokeArgumentIsReportedOnceEvent.Error.Execution
        "finish" -> ABadInvokeArgumentIsReportedOnceEvent.Finish
        "go" -> ABadInvokeArgumentIsReportedOnceEvent.Go
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ABadInvokeArgumentIsReportedOnceEvent): String? = when (event) {
        is ABadInvokeArgumentIsReportedOnceEvent.ChildUp -> "childUp"
        is ABadInvokeArgumentIsReportedOnceEvent.Done.Invoke -> "done.invoke"
        is ABadInvokeArgumentIsReportedOnceEvent.Error.Execution -> "error.execution"
        is ABadInvokeArgumentIsReportedOnceEvent.Finish -> "finish"
        is ABadInvokeArgumentIsReportedOnceEvent.Go -> "go"
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
            "a_bad_invoke_argument_is_reported_once",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'v' with expr
        try {
            val initResult_v = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("7", "7"))
            engine.setVariable(sid, "v", initResult_v)
        } catch (e: Exception) {
            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<data id='v'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'obj' with expr
        try {
            val initResult_obj = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{}", "({})"))
            engine.setVariable(sid, "obj", initResult_obj)
        } catch (e: Exception) {
            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<data id='obj'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'errors' with expr
        try {
            val initResult_errors = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "errors", initResult_errors)
        } catch (e: Exception) {
            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<data id='errors'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'started' with expr
        try {
            val initResult_started = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "started", initResult_started)
        } catch (e: Exception) {
            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<data id='started'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'fromLocOk' with expr
        try {
            val initResult_fromLocOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "fromLocOk", initResult_fromLocOk)
        } catch (e: Exception) {
            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<data id='fromLocOk'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'emptyLocLeftOut' with expr
        try {
            val initResult_emptyLocLeftOut = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "emptyLocLeftOut", initResult_emptyLocLeftOut)
        } catch (e: Exception) {
            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<data id='emptyLocLeftOut'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'brokenLeftOut' with expr
        try {
            val initResult_brokenLeftOut = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "brokenLeftOut", initResult_brokenLeftOut)
        } catch (e: Exception) {
            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<data id='brokenLeftOut'> expr failed to evaluate")
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
            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "an expression could not be serialised to JSON")
            ""
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    //
    // Both halves carry a language: this engine's Lua arm splices the location
    // in front of `=` and runs the result, so a write target written in
    // ECMAScript has to have been lowered too. Same split as
    // `ScxmlScriptEngine.assign`.
    // Returns whether the assignment took place; on failure error.execution is
    // already raised, and the caller ends its block (W3C SCXML 4.9).
    private fun executeAssign(location: com.sce.runtime.ScriptSource, expr: com.sce.runtime.ScriptSource): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.assign(sid, location, expr)
            true
        } catch (e: Exception) {
            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<assign> failed")
            false
        }
    }

    // W3C SCXML 5.8: Script block execution
    // Returns whether the script ran; on failure error.execution is already
    // raised, and the caller ends its block (W3C SCXML 4.9).
    private fun executeScriptBlock(script: com.sce.runtime.ScriptSource): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.executeScript(sid, script)
            true
        } catch (e: Exception) {
            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ABadInvokeArgumentIsReportedOnceEvent) {
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
    override fun bindCurrentEvent(event: ABadInvokeArgumentIsReportedOnceEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ABadInvokeArgumentIsReportedOnceState,
        event: ABadInvokeArgumentIsReportedOnceEvent?
    ): EnabledTransition<ABadInvokeArgumentIsReportedOnceState, HistoryId>? = when (state) {
        is ABadInvokeArgumentIsReportedOnceState.Run -> when {
            event is ABadInvokeArgumentIsReportedOnceEvent.Error.Execution -> transitionRunAt0
            event is ABadInvokeArgumentIsReportedOnceEvent.ChildUp -> transitionRunAt1
            event is ABadInvokeArgumentIsReportedOnceEvent.Finish -> transitionRunAt2
            else -> null
        }
        is ABadInvokeArgumentIsReportedOnceState.S0 -> when {
            event is ABadInvokeArgumentIsReportedOnceEvent.ChildUp && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.fromLoc == 7)", "_event.data.fromLoc === 7")) -> transitionS0At0
            event is ABadInvokeArgumentIsReportedOnceEvent.Go -> transitionS0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:42 :: _machine
    override fun onEntry(state: ABadInvokeArgumentIsReportedOnceState, isDefaultEntry: Boolean) {
        when (state) {
            is ABadInvokeArgumentIsReportedOnceState.Done -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:139 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is ABadInvokeArgumentIsReportedOnceState.Run -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:56 :: run :: _state_body
            }
            is ABadInvokeArgumentIsReportedOnceState.S0 -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:65 :: s0 :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}.inv1"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        ensureScriptEngine()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["fromLoc"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("v", "v"))
                        } catch (_: Exception) {
                            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<invoke> <param name='fromLoc'> could not be read")
                        }
                        raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<invoke> <param name='emptyLoc'> names no location")
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["broken"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"))
                        } catch (_: Exception) {
                            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<invoke> <param name='broken'> could not be read")
                        }
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["notInChild"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"))
                        } catch (_: Exception) {
                            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<invoke> <param name='notInChild'> could not be read")
                        }
                        val childSM = ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv1StateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv1", childSM, false, ABadInvokeArgumentIsReportedOnceEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is ABadInvokeArgumentIsReportedOnceState.S1 -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:105 :: s1 :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s1.${System.identityHashCode(this)}.inv2"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        ensureScriptEngine()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        // W3C SCXML 6.4: "if the evaluation of its arguments produces an
                        // error, the SCXML Processor MUST terminate the processing of the
                        // element without further action". A name that is not a readable
                        // location is such an error: ONE error.execution for the element,
                        // however many names are bad, no child, and its <param>s are not
                        // evaluated.
                        if (!(engineInv.hasVariable(sidInv, "undeclaredA") && engineInv.hasVariable(sidInv, "undeclaredB"))) {
                            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<invoke> namelist names a location that cannot be read")
                            return@deferInvoke
                        }
                        invokeParams["undeclaredA"] = engineInv.getVariable(sidInv, "undeclaredA")
                        invokeParams["undeclaredB"] = engineInv.getVariable(sidInv, "undeclaredB")
                        val childSM = ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv2StateMachine()
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv2", childSM, false, ABadInvokeArgumentIsReportedOnceEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s1.${System.identityHashCode(this)}.inv3"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        ensureScriptEngine()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        // W3C SCXML 6.4: "if the evaluation of its arguments produces an
                        // error, the SCXML Processor MUST terminate the processing of the
                        // element without further action". A name that is not a readable
                        // location is such an error: ONE error.execution for the element,
                        // however many names are bad, no child, and its <param>s are not
                        // evaluated.
                        if (!(engineInv.hasVariable(sidInv, "obj.missing.deep"))) {
                            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<invoke> namelist names a location that cannot be read")
                            return@deferInvoke
                        }
                        invokeParams["obj.missing.deep"] = engineInv.getVariable(sidInv, "obj.missing.deep")
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["alsoBroken"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"))
                        } catch (_: Exception) {
                            raisePlatformError(ABadInvokeArgumentIsReportedOnceEvent.Error.Execution, "<invoke> <param name='alsoBroken'> could not be read")
                        }
                        val childSM = ABadInvokeArgumentIsReportedOnceSceSynthInvokeInv3StateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv3", childSM, false, ABadInvokeArgumentIsReportedOnceEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:42 :: _machine
    override fun onExit(state: ABadInvokeArgumentIsReportedOnceState) {
        when (state) {
            is ABadInvokeArgumentIsReportedOnceState.Done -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:139 :: done :: _state_body
            }
            is ABadInvokeArgumentIsReportedOnceState.Run -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:56 :: run :: _state_body
            }
            is ABadInvokeArgumentIsReportedOnceState.S0 -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:65 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv1")
            }
            is ABadInvokeArgumentIsReportedOnceState.S1 -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:105 :: s1 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv2")
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv3")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:42 :: _machine
    override fun executeTransitionContent(source: ABadInvokeArgumentIsReportedOnceState, transitionIndex: Int) {
        when (source) {
        is ABadInvokeArgumentIsReportedOnceState.Run -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:57 :: run :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("errors", "errors"), com.sce.runtime.ScriptSource.lua("_scxml_add(errors, 1)", "errors + 1"))) {
                return
            }
            }
            1 -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:60 :: run :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("started", "started"), com.sce.runtime.ScriptSource.lua("_scxml_add(started, 1)", "started + 1"))) {
                return
            }
            }
            else -> {}
        }
        is ABadInvokeArgumentIsReportedOnceState.S0 -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_bad_invoke_argument_is_reported_once.scxml:92 :: s0 :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("fromLocOk", "fromLocOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("started", "started"), com.sce.runtime.ScriptSource.lua("_scxml_add(started, 1)", "started + 1"))) {
                return
            }


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.emptyLoc == \"unset\")", "_event.data.emptyLoc === 'unset'"))) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("emptyLocLeftOut", "emptyLocLeftOut"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.broken == \"unset\")", "_event.data.broken === 'unset'"))) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("brokenLeftOut", "brokenLeftOut"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
