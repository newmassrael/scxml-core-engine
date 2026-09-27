// SCE-GENERATED — DO NOT EDIT
// source-hash: f57870c3c6b1a75ec807db1101d97dc2a55af4fb9881653e69f654058afd92fa

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_bad_send_argument_discards_its_message/a_bad_send_argument_discards_its_message.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_bad_send_argument_discards_its_message.scxml:31 :: _machine

package com.sce.integration.a_bad_send_argument_discards_its_message

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ABadSendArgumentDiscardsItsMessageState : State {
    data object Done : ABadSendArgumentDiscardsItsMessageState
    data object S0 : ABadSendArgumentDiscardsItsMessageState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ABadSendArgumentDiscardsItsMessageEvent : Event {
    sealed interface Error : ABadSendArgumentDiscardsItsMessageEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Finish : ABadSendArgumentDiscardsItsMessageEvent
    data object Sent : ABadSendArgumentDiscardsItsMessageEvent
}
// --- State Machine (W3C SCXML) ---

class ABadSendArgumentDiscardsItsMessageStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ABadSendArgumentDiscardsItsMessageState, ABadSendArgumentDiscardsItsMessageEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

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
     * §scxml-5.3: what the `sent` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `sent` was assigned a value of another type, or the engine refused.
     */
    fun sent(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "sent")

    /**
     * §scxml-5.3: what the `after` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `after` was assigned a value of another type, or the engine refused.
     */
    fun after(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "after")

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

    override val initialState: ABadSendArgumentDiscardsItsMessageState = ABadSendArgumentDiscardsItsMessageState.S0

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

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: ABadSendArgumentDiscardsItsMessageState): Boolean = when (state) {
        is ABadSendArgumentDiscardsItsMessageState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ABadSendArgumentDiscardsItsMessageState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<ABadSendArgumentDiscardsItsMessageState, HistoryId>> =
            listOf(StateTarget(ABadSendArgumentDiscardsItsMessageState.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<ABadSendArgumentDiscardsItsMessageState, HistoryId>(
            ABadSendArgumentDiscardsItsMessageState.S0,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<ABadSendArgumentDiscardsItsMessageState, HistoryId>(
            ABadSendArgumentDiscardsItsMessageState.S0,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 2, as the microstep reads it.
        val transitionS0At2 = EnabledTransition<ABadSendArgumentDiscardsItsMessageState, HistoryId>(
            ABadSendArgumentDiscardsItsMessageState.S0,
            listOf(StateTarget(ABadSendArgumentDiscardsItsMessageState.Done)),
            2,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ABadSendArgumentDiscardsItsMessageState? = when (stateId) {
        "done" -> ABadSendArgumentDiscardsItsMessageState.Done
        "s0" -> ABadSendArgumentDiscardsItsMessageState.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ABadSendArgumentDiscardsItsMessageState): String = when (state) {
        is ABadSendArgumentDiscardsItsMessageState.Done -> "done"
        is ABadSendArgumentDiscardsItsMessageState.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ABadSendArgumentDiscardsItsMessageState): Int = when (state) {
        is ABadSendArgumentDiscardsItsMessageState.Done -> 1
        is ABadSendArgumentDiscardsItsMessageState.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ABadSendArgumentDiscardsItsMessageEvent? = when (name) {
        "error.communication" -> ABadSendArgumentDiscardsItsMessageEvent.Error.Communication
        "error.execution" -> ABadSendArgumentDiscardsItsMessageEvent.Error.Execution
        "finish" -> ABadSendArgumentDiscardsItsMessageEvent.Finish
        "sent" -> ABadSendArgumentDiscardsItsMessageEvent.Sent
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ABadSendArgumentDiscardsItsMessageEvent): String? = when (event) {
        is ABadSendArgumentDiscardsItsMessageEvent.Error.Communication -> "error.communication"
        is ABadSendArgumentDiscardsItsMessageEvent.Error.Execution -> "error.execution"
        is ABadSendArgumentDiscardsItsMessageEvent.Finish -> "finish"
        is ABadSendArgumentDiscardsItsMessageEvent.Sent -> "sent"
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
            "a_bad_send_argument_discards_its_message",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'errors' with expr
        try {
            val initResult_errors = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "errors", initResult_errors)
        } catch (e: Exception) {
            raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<data id='errors'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'sent' with expr
        try {
            val initResult_sent = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "sent", initResult_sent)
        } catch (e: Exception) {
            raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<data id='sent'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'after' with expr
        try {
            val initResult_after = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "after", initResult_after)
        } catch (e: Exception) {
            raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<data id='after'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'obj' with expr
        try {
            val initResult_obj = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{}", "({})"))
            engine.setVariable(sid, "obj", initResult_obj)
        } catch (e: Exception) {
            raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<data id='obj'> expr failed to evaluate")
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
            raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<assign> failed")
            false
        }
    }

    // W3C SCXML 6.2.4 / 6.4.1: write a generated id to an `idlocation`.
    //
    // The location is a location expression, so this is the assignment
    // `executeAssign` makes: lowered, so a member path lands, and one that
    // cannot take the id raises error.execution (W3C SCXML 5.9.2) and answers
    // false for the caller to abandon the element. The id is quoted here, in
    // the location's own language, because an invoke's is a run-time value.
    private fun storeIdInLocation(location: com.sce.runtime.ScriptSource, id: String, element: String): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.assign(sid, location, com.sce.runtime.ScriptSource.stringLiteral(id, location.language))
            true
        } catch (e: Exception) {
            raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "$element idlocation could not take the id")
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
            raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ABadSendArgumentDiscardsItsMessageEvent) {
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
    override fun bindCurrentEvent(event: ABadSendArgumentDiscardsItsMessageEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ABadSendArgumentDiscardsItsMessageState,
        event: ABadSendArgumentDiscardsItsMessageEvent?
    ): EnabledTransition<ABadSendArgumentDiscardsItsMessageState, HistoryId>? = when (state) {
        is ABadSendArgumentDiscardsItsMessageState.S0 -> when {
            event is ABadSendArgumentDiscardsItsMessageEvent.Error.Execution -> transitionS0At0
            event is ABadSendArgumentDiscardsItsMessageEvent.Sent -> transitionS0At1
            event is ABadSendArgumentDiscardsItsMessageEvent.Finish -> transitionS0At2
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_bad_send_argument_discards_its_message.scxml:31 :: _machine
    override fun onEntry(state: ABadSendArgumentDiscardsItsMessageState, isDefaultEntry: Boolean) {
        when (state) {
            is ABadSendArgumentDiscardsItsMessageState.Done -> {
                // SCE-MAP: a_bad_send_argument_discards_its_message.scxml:77 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is ABadSendArgumentDiscardsItsMessageState.S0 -> {
                // SCE-MAP: a_bad_send_argument_discards_its_message.scxml:42 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/6
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // W3C SCXML 6.2 + B.2 (test553): a namelist names locations, and one
            // that is not declared is an argument that cannot be evaluated —
            // one error however many of its names are bad.
            val sendNamelist = mutableListOf<Pair<String, Any?>>()
            if (!argEngine.hasVariable(argSid, "undeclaredA")) {
                raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<send> namelist names 'undeclaredA', which is not declared", "__send_0")
                return@send true
            }
            try {
                sendNamelist.add("undeclaredA" to argEngine.getVariable(argSid, "undeclaredA"))
            } catch (_: Exception) {
                raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<send> namelist entry 'undeclaredA' could not be read", "__send_0")
                return@send true
            }
            if (!argEngine.hasVariable(argSid, "undeclaredB")) {
                raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<send> namelist names 'undeclaredB', which is not declared", "__send_0")
                return@send true
            }
            try {
                sendNamelist.add("undeclaredB" to argEngine.getVariable(argSid, "undeclaredB"))
            } catch (_: Exception) {
                raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<send> namelist entry 'undeclaredB' could not be read", "__send_0")
                return@send true
            }
            val sendPayload = mutableMapOf<String, Any?>()
            // W3C SCXML C.1: namelist variables become top-level keys in the
            // data table — the values the prologue read, after the params.
            for ((name, value) in sendNamelist) sendPayload[name] = value
            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(ABadSendArgumentDiscardsItsMessageEvent.Sent, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("after", "after"), com.sce.runtime.ScriptSource.lua("_scxml_add(after, 1)", "after + 1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 2/6
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // An event name that evaluates to nothing names no event: the same
            // failure as one that does not evaluate (test172).
            val sendEventName = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep")))
            } catch (_: Exception) {
                ""
            }
            if (sendEventName.isEmpty()) {
                raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<send> eventexpr could not be evaluated to an event name", "__send_1")
                return@send true
            }
            val sendData = ""
            val sendEvent = resolveEventByName(sendEventName)
            // W3C SCXML 6.2: send to this session's external queue
            if (sendEvent != null) send(sendEvent, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("after", "after"), com.sce.runtime.ScriptSource.lua("_scxml_add(after, 1)", "after + 1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 3/6
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep")))
            } catch (_: Exception) {
                raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_2")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_2")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_2")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 6.2: Dispatch to dynamically resolved target (C++ unified pattern)
            if (_rt == "#_internal") {
                raiseInternal(ABadSendArgumentDiscardsItsMessageEvent.Sent, EventMetadata.internal(sendData))
            } else if (_rt == "#_parent") {
                onSendToParent?.invoke("sent", sendData)
            } else if (deliverToChildSession(
                    com.sce.runtime.IoProcessors.sessionIdFromScxmlLocation(_rt),
"sent",
                    sendData)) {
                // W3C SCXML C.1: the target decoded to one of our children's
                // published locations, so it is addressed to that child.
                // Without this arm the address a peer was told to answer at
                // routes back into the sender's own queue, so the location
                // compares equal and still reaches nobody.
            } else {
                send(ABadSendArgumentDiscardsItsMessageEvent.Sent, EventMetadata.external(sendId = "__send_2", origin = scriptSessionId ?: "", data = sendData))
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("after", "after"), com.sce.runtime.ScriptSource.lua("_scxml_add(after, 1)", "after + 1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 4/6
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // The expression failing is the argument error; the message is not
            // scheduled under some default wait. A value that does not read as
            // a duration is sent at once, as the Interpreter reads it.
            val sendDelayMs = try {
                parseDelay(valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"))))
            } catch (_: Exception) {
                raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<send> delayexpr could not be evaluated", "__send_3")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_3", sendDelayMs, ABadSendArgumentDiscardsItsMessageEvent.Sent, EventMetadata.external(sendId = "__send_3", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("after", "after"), com.sce.runtime.ScriptSource.lua("_scxml_add(after, 1)", "after + 1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 5/6
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // W3C SCXML 6.2.4: the send id goes to `idlocation` first, so it is
            // there even when a later argument fails (test183, test332),
            // through the assignment `<assign>` makes — the location is lowered,
            // so a member path lands. A location that cannot take the id is an
            // argument that cannot be evaluated (W3C SCXML 5.9.2).
            if (!storeIdInLocation(com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"), "__send_4", "<send>")) return@send true
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ABadSendArgumentDiscardsItsMessageEvent.Sent, EventMetadata.external(sendId = "__send_4", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("after", "after"), com.sce.runtime.ScriptSource.lua("_scxml_add(after, 1)", "after + 1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 6/6
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // W3C SCXML 6.2: a type the platform does not support is the same
            // error as one that cannot be evaluated.
            val sendTypeValue = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep")))
            } catch (_: Exception) {
                raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<send> typeexpr could not be evaluated", "__send_5")
                return@send true
            }
            if (!com.sce.runtime.SendHelper.isSupportedSendType(sendTypeValue)) {
                raisePlatformError(ABadSendArgumentDiscardsItsMessageEvent.Error.Execution, "<send> typeexpr names a processor this platform does not support", "__send_5")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ABadSendArgumentDiscardsItsMessageEvent.Sent, EventMetadata.external(sendId = "__send_5", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("after", "after"), com.sce.runtime.ScriptSource.lua("_scxml_add(after, 1)", "after + 1"))) {
                return@run
            }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_bad_send_argument_discards_its_message.scxml:31 :: _machine
    override fun onExit(state: ABadSendArgumentDiscardsItsMessageState) {
        when (state) {
            is ABadSendArgumentDiscardsItsMessageState.Done -> {
                // SCE-MAP: a_bad_send_argument_discards_its_message.scxml:77 :: done :: _state_body
            }
            is ABadSendArgumentDiscardsItsMessageState.S0 -> {
                // SCE-MAP: a_bad_send_argument_discards_its_message.scxml:42 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_bad_send_argument_discards_its_message.scxml:31 :: _machine
    override fun executeTransitionContent(source: ABadSendArgumentDiscardsItsMessageState, transitionIndex: Int) {
        when (source) {
        is ABadSendArgumentDiscardsItsMessageState.S0 -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_bad_send_argument_discards_its_message.scxml:68 :: s0 :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("errors", "errors"), com.sce.runtime.ScriptSource.lua("_scxml_add(errors, 1)", "errors + 1"))) {
                return
            }
            }
            1 -> {
                // SCE-MAP: a_bad_send_argument_discards_its_message.scxml:71 :: s0 :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("sent", "sent"), com.sce.runtime.ScriptSource.lua("_scxml_add(sent, 1)", "sent + 1"))) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
