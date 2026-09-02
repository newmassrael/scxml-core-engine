// SCE-GENERATED — DO NOT EDIT
// source-hash: 484e7440f07c529b155abfa6f79282de908af5e2fc4314e70bd834573adce55b
// template-hash: 85660c1341dd8abf7326f61f4efe828117f6cbaf56814ccb03d3fd81b42e6ed0
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_host_processor.scxml:27 :: _machine

package com.sce.integration.statechart_host_processor

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartHostProcessorState : State {
    data object Dispatching : StatechartHostProcessorState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartHostProcessorEvent : Event {
    sealed interface Error : StatechartHostProcessorEvent {
        data object Execution : Error
    }
    sealed interface Plain : StatechartHostProcessorEvent {
        data object Arrived : Plain
    }
    sealed interface Turn : StatechartHostProcessorEvent {
        data object Done : Turn
    }
    sealed interface Watch : StatechartHostProcessorEvent {
        data object Turn : Watch
    }
}
// --- State Machine (W3C SCXML) ---

class StatechartHostProcessorStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<StatechartHostProcessorState, StatechartHostProcessorEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `served` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `served` was assigned a value of another type, or the engine refused.
     */
    fun served(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "served")

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
     * §scxml-5.3: what the `plain` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `plain` was assigned a value of another type, or the engine refused.
     */
    fun plain(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "plain")

    override val initialState: StatechartHostProcessorState = StatechartHostProcessorState.Dispatching

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
    override fun resolveState(stateId: String): StatechartHostProcessorState? = when (stateId) {
        "dispatching" -> StatechartHostProcessorState.Dispatching
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartHostProcessorState): String = when (state) {
        is StatechartHostProcessorState.Dispatching -> "dispatching"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: StatechartHostProcessorState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: StatechartHostProcessorState): Int = when (state) {
        is StatechartHostProcessorState.Dispatching -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartHostProcessorEvent? = when (name) {
        "error.execution" -> StatechartHostProcessorEvent.Error.Execution
        "plain.arrived" -> StatechartHostProcessorEvent.Plain.Arrived
        "turn.done" -> StatechartHostProcessorEvent.Turn.Done
        "watch.turn" -> StatechartHostProcessorEvent.Watch.Turn
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartHostProcessorEvent): String? = when (event) {
        is StatechartHostProcessorEvent.Error.Execution -> "error.execution"
        is StatechartHostProcessorEvent.Plain.Arrived -> "plain.arrived"
        is StatechartHostProcessorEvent.Turn.Done -> "turn.done"
        is StatechartHostProcessorEvent.Watch.Turn -> "watch.turn"
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
            "statechart_host_processor",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'served' with expr
        try {
            val initResult_served = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "served", initResult_served)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<data id='served'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'refused' with expr
        try {
            val initResult_refused = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "refused", initResult_refused)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<data id='refused'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'plain' with expr
        try {
            val initResult_plain = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "plain", initResult_plain)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<data id='plain'> expr failed to evaluate")
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
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: StatechartHostProcessorEvent) {
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
        state: StatechartHostProcessorState,
        event: StatechartHostProcessorEvent
    ): TransitionResult<StatechartHostProcessorState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is StatechartHostProcessorState.Dispatching -> processDispatching(event)
    }
    }


    // --- Per-State Event Handlers ---

    private fun processDispatching(
        event: StatechartHostProcessorEvent
    ): TransitionResult<StatechartHostProcessorState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is StatechartHostProcessorEvent.Plain.Arrived -> TransitionResult.Internal(0)
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is StatechartHostProcessorEvent.Turn.Done -> TransitionResult.Internal(1)
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is StatechartHostProcessorEvent.Error.Execution -> TransitionResult.Internal(2)
        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_host_processor.scxml:27 :: _machine
    override fun onEntry(state: StatechartHostProcessorState, pathChild: StatechartHostProcessorState?) {
        when (state) {
            is StatechartHostProcessorState.Dispatching -> {
                // SCE-MAP: statechart_host_processor.scxml:36 :: dispatching :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("dispatching")) return


            send(StatechartHostProcessorEvent.Plain.Arrived, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))


            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            run {
                val hostParams = mutableMapOf<String, List<String>>()
                hostParams["within"] = listOf("2500")
                val hostEventName = "watch.turn"
                val hostRequest = HostSendRequest(
                    processorType = "x-sce-host",
                    eventName = hostEventName,
                    target = "",
                    content = "",
                    params = hostParams,
                    sendId = "__send_1"
                )
                val hostServed = performHostSend(hostRequest)
                // W3C SCXML 6.2: a declared type with no handler registered is,
                // from the document's side, a processor the platform does not
                // support — the act it asked for was performed by nobody. Same
                // event as an undeclared type, so a wiring mistake cannot read
                // as success.
                if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                    raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_1")
                }
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_host_processor.scxml:27 :: _machine
    override fun onExit(state: StatechartHostProcessorState) {
        when (state) {
            is StatechartHostProcessorState.Dispatching -> {
                // SCE-MAP: statechart_host_processor.scxml:36 :: dispatching :: _state_body
                activeStateIds.remove("dispatching")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: statechart_host_processor.scxml:27 :: _machine
    override fun executeTransitionActions(
        source: StatechartHostProcessorState,
        event: StatechartHostProcessorEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        is StatechartHostProcessorState.Dispatching -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_host_processor.scxml:48 :: dispatching :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("plain", "plain"), com.sce.runtime.ScriptSource.lua("_scxml_add(plain, 1)", "plain + 1"))
            }
            1 -> {
                // SCE-MAP: statechart_host_processor.scxml:51 :: dispatching :: _transition_1


            executeAssign(com.sce.runtime.ScriptSource.lua("served", "served"), com.sce.runtime.ScriptSource.lua("_scxml_add(served, 1)", "served + 1"))
            }
            2 -> {
                // SCE-MAP: statechart_host_processor.scxml:54 :: dispatching :: _transition_2


            executeAssign(com.sce.runtime.ScriptSource.lua("refused", "refused"), com.sce.runtime.ScriptSource.lua("_scxml_add(refused, 1)", "refused + 1"))
            }
            else -> {}
        }
        }
    }
}
