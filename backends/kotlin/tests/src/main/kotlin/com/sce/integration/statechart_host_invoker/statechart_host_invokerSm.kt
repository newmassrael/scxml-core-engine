// SCE-GENERATED — DO NOT EDIT
// source-hash: caf137939845119236b0f1ba1b6c76cb935d951485e256adfbd77b5618ac9fc9

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_host_invoker.scxml:56 :: _machine

package com.sce.integration.statechart_host_invoker

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartHostInvokerState : State {
    data object Done : StatechartHostInvokerState
    data object Evaluating : StatechartHostInvokerState
    data object Invoking : StatechartHostInvokerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartHostInvokerEvent : Event {
    data object Again : StatechartHostInvokerEvent
    sealed interface Done : StatechartHostInvokerEvent {
        sealed interface Invoke : Done {
            data object Probe : Invoke
            data object Probe2 : Invoke
        }
    }
    sealed interface Error : StatechartHostInvokerEvent {
        data object Execution : Error
    }
    data object Evaluate : StatechartHostInvokerEvent
    data object Leave : StatechartHostInvokerEvent
}
// --- State Machine (W3C SCXML) ---

class StatechartHostInvokerStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<StatechartHostInvokerState, StatechartHostInvokerEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

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
     * §scxml-5.3: what the `started2` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `started2` was assigned a value of another type, or the engine refused.
     */
    fun started2(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "started2")

    /**
     * §scxml-5.3: what the `refused` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `refused` was assigned a value of another type, or the engine refused.
     */
    fun refused(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "refused")

    /**
     * §scxml-5.3: what the `ended` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `ended` was assigned a value of another type, or the engine refused.
     */
    fun ended(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "ended")

    /**
     * §scxml-5.3: what the `entered` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `entered` was assigned a value of another type, or the engine refused.
     */
    fun entered(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "entered")

    /**
     * §scxml-5.3: what the `destination` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `destination` was assigned a value of another type, or the engine refused.
     */
    fun destination(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "destination")

    /**
     * §scxml-5.3: what the `n` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `n` was assigned a value of another type, or the engine refused.
     */
    fun n(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "n")

    /**
     * §scxml-5.3: what the `dropped` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `dropped` was assigned a value of another type, or the engine refused.
     */
    fun dropped(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "dropped")

    override val initialState: StatechartHostInvokerState = StatechartHostInvokerState.Invoking

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

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StatechartHostInvokerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StatechartHostInvokerState, HistoryId>> =
            listOf(StateTarget(StatechartHostInvokerState.Invoking))

        // W3C SCXML 3.13: done's transition 0, as the microstep reads it.
        val transitionDoneAt0 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Done,
            listOf(StateTarget(StatechartHostInvokerState.Invoking)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: evaluating's transition 0, as the microstep reads it.
        val transitionEvaluatingAt0 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Evaluating,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: invoking's transition 0, as the microstep reads it.
        val transitionInvokingAt0 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Invoking,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: invoking's transition 1, as the microstep reads it.
        val transitionInvokingAt1 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Invoking,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: invoking's transition 2, as the microstep reads it.
        val transitionInvokingAt2 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Invoking,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: invoking's transition 3, as the microstep reads it.
        val transitionInvokingAt3 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Invoking,
            listOf(StateTarget(StatechartHostInvokerState.Done)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: invoking's transition 4, as the microstep reads it.
        val transitionInvokingAt4 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Invoking,
            listOf(StateTarget(StatechartHostInvokerState.Evaluating)),
            4,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StatechartHostInvokerState? = when (stateId) {
        "done" -> StatechartHostInvokerState.Done
        "evaluating" -> StatechartHostInvokerState.Evaluating
        "invoking" -> StatechartHostInvokerState.Invoking
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartHostInvokerState): String = when (state) {
        is StatechartHostInvokerState.Done -> "done"
        is StatechartHostInvokerState.Evaluating -> "evaluating"
        is StatechartHostInvokerState.Invoking -> "invoking"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StatechartHostInvokerState): Int = when (state) {
        is StatechartHostInvokerState.Done -> 2
        is StatechartHostInvokerState.Evaluating -> 1
        is StatechartHostInvokerState.Invoking -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartHostInvokerEvent? = when (name) {
        "again" -> StatechartHostInvokerEvent.Again
        "done.invoke.probe" -> StatechartHostInvokerEvent.Done.Invoke.Probe
        "done.invoke.probe2" -> StatechartHostInvokerEvent.Done.Invoke.Probe2
        "error.execution" -> StatechartHostInvokerEvent.Error.Execution
        "evaluate" -> StatechartHostInvokerEvent.Evaluate
        "leave" -> StatechartHostInvokerEvent.Leave
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartHostInvokerEvent): String? = when (event) {
        is StatechartHostInvokerEvent.Again -> "again"
        is StatechartHostInvokerEvent.Done.Invoke.Probe -> "done.invoke.probe"
        is StatechartHostInvokerEvent.Done.Invoke.Probe2 -> "done.invoke.probe2"
        is StatechartHostInvokerEvent.Error.Execution -> "error.execution"
        is StatechartHostInvokerEvent.Evaluate -> "evaluate"
        is StatechartHostInvokerEvent.Leave -> "leave"
    }

    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "probe",
        "probe2",
        "req",
        "req2",
        "req3",
    )



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
            "statechart_host_invoker",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'started' with expr
        try {
            val initResult_started = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "started", initResult_started)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='started'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'started2' with expr
        try {
            val initResult_started2 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "started2", initResult_started2)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='started2'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'refused' with expr
        try {
            val initResult_refused = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "refused", initResult_refused)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='refused'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'ended' with expr
        try {
            val initResult_ended = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "ended", initResult_ended)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='ended'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'entered' with expr
        try {
            val initResult_entered = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "entered", initResult_entered)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='entered'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'destination' with expr
        try {
            val initResult_destination = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"pane://dyn\"", "'pane://dyn'"))
            engine.setVariable(sid, "destination", initResult_destination)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='destination'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'n' with expr
        try {
            val initResult_n = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("7", "7"))
            engine.setVariable(sid, "n", initResult_n)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='n'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'dropped' with expr
        try {
            val initResult_dropped = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "dropped", initResult_dropped)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='dropped'> expr failed to evaluate")
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
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: StatechartHostInvokerEvent) {
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
    override fun bindCurrentEvent(event: StatechartHostInvokerEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StatechartHostInvokerState,
        event: StatechartHostInvokerEvent?
    ): EnabledTransition<StatechartHostInvokerState, HistoryId>? = when (state) {
        is StatechartHostInvokerState.Done -> when {
            event is StatechartHostInvokerEvent.Again -> transitionDoneAt0
            else -> null
        }
        is StatechartHostInvokerState.Evaluating -> when {
            event is StatechartHostInvokerEvent.Error.Execution -> transitionEvaluatingAt0
            else -> null
        }
        is StatechartHostInvokerState.Invoking -> when {
            event is StatechartHostInvokerEvent.Done.Invoke.Probe && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.invokeid == \"probe\")", "_event.invokeid === 'probe'")) -> transitionInvokingAt0
            event is StatechartHostInvokerEvent.Done.Invoke.Probe2 && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.invokeid == \"probe2\")", "_event.invokeid === 'probe2'")) -> transitionInvokingAt1
            event is StatechartHostInvokerEvent.Error.Execution -> transitionInvokingAt2
            event is StatechartHostInvokerEvent.Leave -> transitionInvokingAt3
            event is StatechartHostInvokerEvent.Evaluate -> transitionInvokingAt4
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_host_invoker.scxml:56 :: _machine
    override fun onEntry(state: StatechartHostInvokerState, isDefaultEntry: Boolean) {
        when (state) {
            is StatechartHostInvokerState.Done -> {
                // SCE-MAP: statechart_host_invoker.scxml:111 :: done :: _state_body


            executeAssign(com.sce.runtime.ScriptSource.lua("ended", "ended"), com.sce.runtime.ScriptSource.lua("_scxml_add(ended, 1)", "ended + 1"))
            }
            is StatechartHostInvokerState.Evaluating -> {
                // SCE-MAP: statechart_host_invoker.scxml:95 :: evaluating :: _state_body
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "evaluating.${System.identityHashCode(this)}.req"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4.1: what the request says is evaluated
                        // now, when the invocation starts. An attribute that
                        // cannot be evaluated raises error.execution and starts
                        // nothing; a `<param>` that cannot is reported and
                        // dropped (W3C SCXML 5.7.1) while the invocation starts.
                        ensureScriptEngine()
                        val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val hostInvokeSrc = try {
                            valueToWireString(hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("destination", "destination")))
                        } catch (_: Exception) {
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> srcexpr failed to evaluate")
                            return@deferInvoke
                        }
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        if (!hostEngine.hasVariable(hostSid, "n")) {
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> namelist names 'n', which is not declared")
                            return@deferInvoke
                        }
                        try {
                            hostInvokeParams["n"] =
                                (hostInvokeParams["n"] ?: emptyList()) + valueToWireString(hostEngine.getVariable(hostSid, "n"))
                        } catch (_: Exception) {
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> namelist entry 'n' failed to evaluate")
                            return@deferInvoke
                        }
                        hostInvokeParams["twice"] =
                            (hostInvokeParams["twice"] ?: emptyList()) + "a"
                        try {
                            // The param crosses as text, and `toString()` is the platform's
                            // spelling of the value; this is the document's.
                            val v = hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("_scxml_add(n, 1)", "n + 1"))
                            hostInvokeParams["twice"] =
                                (hostInvokeParams["twice"] ?: emptyList()) + valueToWireString(v)
                        } catch (_: Exception) {
                            // W3C SCXML 5.7.1: report the failure and omit the name and the
                            // value — the act still happens, without a field the document
                            // could not produce.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> <param name='twice'> expr failed to evaluate")
                        }
                        try {
                            // The param crosses as text, and `toString()` is the platform's
                            // spelling of the value; this is the document's.
                            val v = hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("n.nope.deeper", "n.nope.deeper"))
                            hostInvokeParams["bad"] =
                                (hostInvokeParams["bad"] ?: emptyList()) + valueToWireString(v)
                        } catch (_: Exception) {
                            // W3C SCXML 5.7.1: report the failure and omit the name and the
                            // value — the act still happens, without a field the document
                            // could not produce.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> <param name='bad'> expr failed to evaluate")
                        }
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "req",
                                src = hostInvokeSrc,
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "evaluating.${System.identityHashCode(this)}.req2"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4.1: what the request says is evaluated
                        // now, when the invocation starts. An attribute that
                        // cannot be evaluated raises error.execution and starts
                        // nothing; a `<param>` that cannot is reported and
                        // dropped (W3C SCXML 5.7.1) while the invocation starts.
                        ensureScriptEngine()
                        val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val hostInvokeContent = try {
                            valueToWireString(hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("(\"body:\" .. _scxml_tostring(n))", "'body:' + n")))
                        } catch (_: Exception) {
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> contentexpr failed to evaluate")
                            return@deferInvoke
                        }
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "req2",
                                src = "",
                                params = hostInvokeParams,
                                content = hostInvokeContent                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "evaluating.${System.identityHashCode(this)}.req3"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4.1: what the request says is evaluated
                        // now, when the invocation starts. An attribute that
                        // cannot be evaluated raises error.execution and starts
                        // nothing; a `<param>` that cannot is reported and
                        // dropped (W3C SCXML 5.7.1) while the invocation starts.
                        ensureScriptEngine()
                        val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val hostInvokeSrc = try {
                            valueToWireString(hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("n.nope.deeper", "n.nope.deeper")))
                        } catch (_: Exception) {
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> srcexpr failed to evaluate")
                            return@deferInvoke
                        }
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "req3",
                                src = hostInvokeSrc,
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StatechartHostInvokerState.Invoking -> {
                // SCE-MAP: statechart_host_invoker.scxml:70 :: invoking :: _state_body


            executeAssign(com.sce.runtime.ScriptSource.lua("entered", "entered"), com.sce.runtime.ScriptSource.lua("_scxml_add(entered, 1)", "entered + 1"))
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "invoking.${System.identityHashCode(this)}.probe"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        hostInvokeParams["within"] =
                            (hostInvokeParams["within"] ?: emptyList()) + "2500"
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "probe",
                                src = "pane://turn",
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "invoking.${System.identityHashCode(this)}.probe2"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "probe2",
                                src = "pane://other",
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_host_invoker.scxml:56 :: _machine
    override fun onExit(state: StatechartHostInvokerState) {
        when (state) {
            is StatechartHostInvokerState.Done -> {
                // SCE-MAP: statechart_host_invoker.scxml:111 :: done :: _state_body
            }
            is StatechartHostInvokerState.Evaluating -> {
                // SCE-MAP: statechart_host_invoker.scxml:95 :: evaluating :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "req")
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "req2")
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "req3")
            }
            is StatechartHostInvokerState.Invoking -> {
                // SCE-MAP: statechart_host_invoker.scxml:70 :: invoking :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "probe")
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "probe2")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: statechart_host_invoker.scxml:56 :: _machine
    override fun executeTransitionContent(source: StatechartHostInvokerState, transitionIndex: Int) {
        when (source) {
        is StatechartHostInvokerState.Evaluating -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_host_invoker.scxml:106 :: evaluating :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("dropped", "dropped"), com.sce.runtime.ScriptSource.lua("_scxml_add(dropped, 1)", "dropped + 1"))
            }
            else -> {}
        }
        is StatechartHostInvokerState.Invoking -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_host_invoker.scxml:80 :: invoking :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("started", "started"), com.sce.runtime.ScriptSource.lua("_scxml_add(started, 1)", "started + 1"))
            }
            1 -> {
                // SCE-MAP: statechart_host_invoker.scxml:83 :: invoking :: _transition_1


            executeAssign(com.sce.runtime.ScriptSource.lua("started2", "started2"), com.sce.runtime.ScriptSource.lua("_scxml_add(started2, 1)", "started2 + 1"))
            }
            2 -> {
                // SCE-MAP: statechart_host_invoker.scxml:86 :: invoking :: _transition_2


            executeAssign(com.sce.runtime.ScriptSource.lua("refused", "refused"), com.sce.runtime.ScriptSource.lua("_scxml_add(refused, 1)", "refused + 1"))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
