// SCE-GENERATED — DO NOT EDIT
// source-hash: 7a40d7a412582033feb4526f3896e8a5885911608eff3ee490a884cb8f4f7523

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_peer_named_at_run_time_reaches_the_router/a_peer_named_at_run_time_reaches_the_router.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_peer_named_at_run_time_reaches_the_router.scxml:20 :: _machine

package com.sce.integration.a_peer_named_at_run_time_reaches_the_router

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface APeerNamedAtRunTimeReachesTheRouterState : State {
    data object Done : APeerNamedAtRunTimeReachesTheRouterState
    data object S0 : APeerNamedAtRunTimeReachesTheRouterState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface APeerNamedAtRunTimeReachesTheRouterEvent : Event {
    sealed interface Error : APeerNamedAtRunTimeReachesTheRouterEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Loopback : APeerNamedAtRunTimeReachesTheRouterEvent
    data object Ping : APeerNamedAtRunTimeReachesTheRouterEvent
}
// --- State Machine (W3C SCXML) ---

class APeerNamedAtRunTimeReachesTheRouterStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<APeerNamedAtRunTimeReachesTheRouterState, APeerNamedAtRunTimeReachesTheRouterEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `peer` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `peer` was assigned a value of another type, or the engine refused.
     */
    fun peer(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "peer")

    /**
     * §scxml-5.3: what the `here` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `here` was assigned a value of another type, or the engine refused.
     */
    fun here(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "here")

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
     * §scxml-5.3: what the `looped` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `looped` was assigned a value of another type, or the engine refused.
     */
    fun looped(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "looped")

    override val initialState: APeerNamedAtRunTimeReachesTheRouterState = APeerNamedAtRunTimeReachesTheRouterState.S0

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

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: APeerNamedAtRunTimeReachesTheRouterState): Boolean = when (state) {
        is APeerNamedAtRunTimeReachesTheRouterState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<APeerNamedAtRunTimeReachesTheRouterState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<APeerNamedAtRunTimeReachesTheRouterState, HistoryId>> =
            listOf(StateTarget(APeerNamedAtRunTimeReachesTheRouterState.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<APeerNamedAtRunTimeReachesTheRouterState, HistoryId>(
            APeerNamedAtRunTimeReachesTheRouterState.S0,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<APeerNamedAtRunTimeReachesTheRouterState, HistoryId>(
            APeerNamedAtRunTimeReachesTheRouterState.S0,
            listOf(StateTarget(APeerNamedAtRunTimeReachesTheRouterState.Done)),
            1,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): APeerNamedAtRunTimeReachesTheRouterState? = when (stateId) {
        "done" -> APeerNamedAtRunTimeReachesTheRouterState.Done
        "s0" -> APeerNamedAtRunTimeReachesTheRouterState.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: APeerNamedAtRunTimeReachesTheRouterState): String = when (state) {
        is APeerNamedAtRunTimeReachesTheRouterState.Done -> "done"
        is APeerNamedAtRunTimeReachesTheRouterState.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: APeerNamedAtRunTimeReachesTheRouterState): Int = when (state) {
        is APeerNamedAtRunTimeReachesTheRouterState.Done -> 1
        is APeerNamedAtRunTimeReachesTheRouterState.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): APeerNamedAtRunTimeReachesTheRouterEvent? = when (name) {
        "error.communication" -> APeerNamedAtRunTimeReachesTheRouterEvent.Error.Communication
        "error.execution" -> APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution
        "loopback" -> APeerNamedAtRunTimeReachesTheRouterEvent.Loopback
        "ping" -> APeerNamedAtRunTimeReachesTheRouterEvent.Ping
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: APeerNamedAtRunTimeReachesTheRouterEvent): String? = when (event) {
        is APeerNamedAtRunTimeReachesTheRouterEvent.Error.Communication -> "error.communication"
        is APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution -> "error.execution"
        is APeerNamedAtRunTimeReachesTheRouterEvent.Loopback -> "loopback"
        is APeerNamedAtRunTimeReachesTheRouterEvent.Ping -> "ping"
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
            "a_peer_named_at_run_time_reaches_the_router",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'peer' with expr
        try {
            val initResult_peer = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"#hmi\"", "'#hmi'"))
            engine.setVariable(sid, "peer", initResult_peer)
        } catch (e: Exception) {
            raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "<data id='peer'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'here' with expr
        try {
            val initResult_here = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"#_internal\"", "'#_internal'"))
            engine.setVariable(sid, "here", initResult_here)
        } catch (e: Exception) {
            raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "<data id='here'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'refused' with expr
        try {
            val initResult_refused = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "refused", initResult_refused)
        } catch (e: Exception) {
            raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "<data id='refused'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'looped' with expr
        try {
            val initResult_looped = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "looped", initResult_looped)
        } catch (e: Exception) {
            raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "<data id='looped'> expr failed to evaluate")
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
            raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: APeerNamedAtRunTimeReachesTheRouterEvent) {
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
    override fun bindCurrentEvent(event: APeerNamedAtRunTimeReachesTheRouterEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: APeerNamedAtRunTimeReachesTheRouterState,
        event: APeerNamedAtRunTimeReachesTheRouterEvent?
    ): EnabledTransition<APeerNamedAtRunTimeReachesTheRouterState, HistoryId>? = when (state) {
        is APeerNamedAtRunTimeReachesTheRouterState.S0 -> when {
            event is APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution -> transitionS0At0
            event is APeerNamedAtRunTimeReachesTheRouterEvent.Loopback -> transitionS0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_peer_named_at_run_time_reaches_the_router.scxml:20 :: _machine
    override fun onEntry(state: APeerNamedAtRunTimeReachesTheRouterState, isDefaultEntry: Boolean) {
        when (state) {
            is APeerNamedAtRunTimeReachesTheRouterState.Done -> {
                // SCE-MAP: a_peer_named_at_run_time_reaches_the_router.scxml:45 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is APeerNamedAtRunTimeReachesTheRouterState.S0 -> {
                // SCE-MAP: a_peer_named_at_run_time_reaches_the_router.scxml:31 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("peer", "peer")))
            } catch (_: Exception) {
                raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_0")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_0")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_0")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "ping",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_0",
                eventData = sendData
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_0")
            }
            } else {
            // W3C SCXML 6.2: Dispatch to dynamically resolved target (C++ unified pattern)
            if (_rt == "#_internal") {
                raiseInternal(APeerNamedAtRunTimeReachesTheRouterEvent.Ping, EventMetadata.internal(sendData))
            } else if (_rt == "#_parent") {
                onSendToParent?.invoke("ping", sendData)
            } else if (deliverToChildSession(
                    com.sce.runtime.IoProcessors.sessionIdFromScxmlLocation(_rt),
"ping",
                    sendData)) {
                // W3C SCXML C.1: the target decoded to one of our children's
                // published locations, so it is addressed to that child.
                // Without this arm the address a peer was told to answer at
                // routes back into the sender's own queue, so the location
                // compares equal and still reaches nobody.
            } else {
                send(APeerNamedAtRunTimeReachesTheRouterEvent.Ping, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("here", "here")))
            } catch (_: Exception) {
                raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_1")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_1")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_1")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "loopback",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_1",
                eventData = sendData
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(APeerNamedAtRunTimeReachesTheRouterEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_1")
            }
            } else {
            // W3C SCXML 6.2: Dispatch to dynamically resolved target (C++ unified pattern)
            if (_rt == "#_internal") {
                raiseInternal(APeerNamedAtRunTimeReachesTheRouterEvent.Loopback, EventMetadata.internal(sendData))
            } else if (_rt == "#_parent") {
                onSendToParent?.invoke("loopback", sendData)
            } else if (deliverToChildSession(
                    com.sce.runtime.IoProcessors.sessionIdFromScxmlLocation(_rt),
"loopback",
                    sendData)) {
                // W3C SCXML C.1: the target decoded to one of our children's
                // published locations, so it is addressed to that child.
                // Without this arm the address a peer was told to answer at
                // routes back into the sender's own queue, so the location
                // compares equal and still reaches nobody.
            } else {
                send(APeerNamedAtRunTimeReachesTheRouterEvent.Loopback, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData))
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_peer_named_at_run_time_reaches_the_router.scxml:20 :: _machine
    override fun onExit(state: APeerNamedAtRunTimeReachesTheRouterState) {
        when (state) {
            is APeerNamedAtRunTimeReachesTheRouterState.Done -> {
                // SCE-MAP: a_peer_named_at_run_time_reaches_the_router.scxml:45 :: done :: _state_body
            }
            is APeerNamedAtRunTimeReachesTheRouterState.S0 -> {
                // SCE-MAP: a_peer_named_at_run_time_reaches_the_router.scxml:31 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_peer_named_at_run_time_reaches_the_router.scxml:20 :: _machine
    override fun executeTransitionContent(source: APeerNamedAtRunTimeReachesTheRouterState, transitionIndex: Int) {
        when (source) {
        is APeerNamedAtRunTimeReachesTheRouterState.S0 -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_peer_named_at_run_time_reaches_the_router.scxml:37 :: s0 :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("refused", "refused"), com.sce.runtime.ScriptSource.lua("_scxml_add(refused, 1)", "refused + 1"))) {
                return
            }
            }
            1 -> {
                // SCE-MAP: a_peer_named_at_run_time_reaches_the_router.scxml:40 :: s0 :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("looped", "looped"), com.sce.runtime.ScriptSource.lua("_scxml_add(looped, 1)", "looped + 1"))) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
