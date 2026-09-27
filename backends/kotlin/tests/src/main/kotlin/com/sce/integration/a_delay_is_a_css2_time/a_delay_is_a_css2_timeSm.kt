// SCE-GENERATED — DO NOT EDIT
// source-hash: 7998039ec52a7642f700c0295686317f66c38570370059241bbd8b2108f4d9d2

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_delay_is_a_css2_time/a_delay_is_a_css2_time.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_delay_is_a_css2_time.scxml:26 :: _machine

package com.sce.integration.a_delay_is_a_css2_time

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ADelayIsACss2TimeState : State {
    data object Done : ADelayIsACss2TimeState
    data object S0 : ADelayIsACss2TimeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ADelayIsACss2TimeEvent : Event {
    data object A : ADelayIsACss2TimeEvent
    data object B : ADelayIsACss2TimeEvent
    data object Bad : ADelayIsACss2TimeEvent
    sealed interface Error : ADelayIsACss2TimeEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class ADelayIsACss2TimeStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ADelayIsACss2TimeState, ADelayIsACss2TimeEvent>(scriptEngine) {

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
     * §scxml-5.3: what the `bad` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `bad` was assigned a value of another type, or the engine refused.
     */
    fun bad(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "bad")

    /**
     * §scxml-5.3: what the `bSeen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `bSeen` was assigned a value of another type, or the engine refused.
     */
    fun bSeen(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "bSeen")

    /**
     * §scxml-5.3: what the `aAfterB` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `aAfterB` was assigned a value of another type, or the engine refused.
     */
    fun aAfterB(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "aAfterB")

    /**
     * §scxml-5.3: what the `bare` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `bare` was assigned a value of another type, or the engine refused.
     */
    fun bare(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "bare")

    override val initialState: ADelayIsACss2TimeState = ADelayIsACss2TimeState.S0

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
    override fun isFinalState(state: ADelayIsACss2TimeState): Boolean = when (state) {
        is ADelayIsACss2TimeState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ADelayIsACss2TimeState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<ADelayIsACss2TimeState, HistoryId>> =
            listOf(StateTarget(ADelayIsACss2TimeState.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<ADelayIsACss2TimeState, HistoryId>(
            ADelayIsACss2TimeState.S0,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<ADelayIsACss2TimeState, HistoryId>(
            ADelayIsACss2TimeState.S0,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 2, as the microstep reads it.
        val transitionS0At2 = EnabledTransition<ADelayIsACss2TimeState, HistoryId>(
            ADelayIsACss2TimeState.S0,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 3, as the microstep reads it.
        val transitionS0At3 = EnabledTransition<ADelayIsACss2TimeState, HistoryId>(
            ADelayIsACss2TimeState.S0,
            listOf(StateTarget(ADelayIsACss2TimeState.Done)),
            3,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ADelayIsACss2TimeState? = when (stateId) {
        "done" -> ADelayIsACss2TimeState.Done
        "s0" -> ADelayIsACss2TimeState.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ADelayIsACss2TimeState): String = when (state) {
        is ADelayIsACss2TimeState.Done -> "done"
        is ADelayIsACss2TimeState.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ADelayIsACss2TimeState): Int = when (state) {
        is ADelayIsACss2TimeState.Done -> 1
        is ADelayIsACss2TimeState.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ADelayIsACss2TimeEvent? = when (name) {
        "a" -> ADelayIsACss2TimeEvent.A
        "b" -> ADelayIsACss2TimeEvent.B
        "bad" -> ADelayIsACss2TimeEvent.Bad
        "error.execution" -> ADelayIsACss2TimeEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ADelayIsACss2TimeEvent): String? = when (event) {
        is ADelayIsACss2TimeEvent.A -> "a"
        is ADelayIsACss2TimeEvent.B -> "b"
        is ADelayIsACss2TimeEvent.Bad -> "bad"
        is ADelayIsACss2TimeEvent.Error.Execution -> "error.execution"
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
            "a_delay_is_a_css2_time",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'errors' with expr
        try {
            val initResult_errors = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "errors", initResult_errors)
        } catch (e: Exception) {
            raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "<data id='errors'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'after' with expr
        try {
            val initResult_after = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "after", initResult_after)
        } catch (e: Exception) {
            raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "<data id='after'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'bad' with expr
        try {
            val initResult_bad = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "bad", initResult_bad)
        } catch (e: Exception) {
            raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "<data id='bad'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'bSeen' with expr
        try {
            val initResult_bSeen = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "bSeen", initResult_bSeen)
        } catch (e: Exception) {
            raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "<data id='bSeen'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'aAfterB' with expr
        try {
            val initResult_aAfterB = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("(-1)", "-1"))
            engine.setVariable(sid, "aAfterB", initResult_aAfterB)
        } catch (e: Exception) {
            raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "<data id='aAfterB'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'bare' with expr
        try {
            val initResult_bare = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("5", "5"))
            engine.setVariable(sid, "bare", initResult_bare)
        } catch (e: Exception) {
            raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "<data id='bare'> expr failed to evaluate")
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
            raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ADelayIsACss2TimeEvent) {
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
    override fun bindCurrentEvent(event: ADelayIsACss2TimeEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ADelayIsACss2TimeState,
        event: ADelayIsACss2TimeEvent?
    ): EnabledTransition<ADelayIsACss2TimeState, HistoryId>? = when (state) {
        is ADelayIsACss2TimeState.S0 -> when {
            event is ADelayIsACss2TimeEvent.Error.Execution -> transitionS0At0
            event is ADelayIsACss2TimeEvent.Bad -> transitionS0At1
            event is ADelayIsACss2TimeEvent.B -> transitionS0At2
            event is ADelayIsACss2TimeEvent.A -> transitionS0At3
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_delay_is_a_css2_time.scxml:26 :: _machine
    override fun onEntry(state: ADelayIsACss2TimeState, isDefaultEntry: Boolean) {
        when (state) {
            is ADelayIsACss2TimeState.Done -> {
                // SCE-MAP: a_delay_is_a_css2_time.scxml:67 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is ADelayIsACss2TimeState.S0 -> {
                // SCE-MAP: a_delay_is_a_css2_time.scxml:39 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/3
                run {


            if (run send@{
            // W3C SCXML 6.2: the written delay is not a CSS2 time
            // (ARCHITECTURE.md, "Durations"; the build decided it once). It is
            // known before anything is evaluated, so the send ends here with
            // the argument error, as it does on every engine.
            raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "<send> delay is not a CSS2 time", "__send_0")
            true
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("after", "after"), com.sce.runtime.ScriptSource.lua("_scxml_add(after, 1)", "after + 1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 2/3
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // The expression failing is the argument error, and so is a value
            // that is not the CSS2 time the clause names (ARCHITECTURE.md,
            // "Durations"): the message is not scheduled under some default wait.
            val sendDelayText = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("bare", "bare")))
            } catch (_: Exception) {
                raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "<send> delayexpr could not be evaluated", "__send_1")
                return@send true
            }
            val sendDelayMs = com.sce.runtime.SendHelper.parseDelayMs(sendDelayText) ?: run {
                raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "<send> delayexpr is not a CSS2 time", "__send_1")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_1", sendDelayMs, ADelayIsACss2TimeEvent.Bad, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData))
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
                // W3C SCXML 3.8: Onentry block 3/3
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // The expression failing is the argument error, and so is a value
            // that is not the CSS2 time the clause names (ARCHITECTURE.md,
            // "Durations"): the message is not scheduled under some default wait.
            val sendDelayText = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("\"0.57s\"", "'0.57s'")))
            } catch (_: Exception) {
                raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "<send> delayexpr could not be evaluated", "__send_2")
                return@send true
            }
            val sendDelayMs = com.sce.runtime.SendHelper.parseDelayMs(sendDelayText) ?: run {
                raisePlatformError(ADelayIsACss2TimeEvent.Error.Execution, "<send> delayexpr is not a CSS2 time", "__send_2")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_2", sendDelayMs, ADelayIsACss2TimeEvent.A, EventMetadata.external(sendId = "__send_2", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_3", 569L, ADelayIsACss2TimeEvent.B, EventMetadata.external(sendId = "__send_3", origin = scriptSessionId ?: "", data = sendData))
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
    // SCE-MAP: a_delay_is_a_css2_time.scxml:26 :: _machine
    override fun onExit(state: ADelayIsACss2TimeState) {
        when (state) {
            is ADelayIsACss2TimeState.Done -> {
                // SCE-MAP: a_delay_is_a_css2_time.scxml:67 :: done :: _state_body
            }
            is ADelayIsACss2TimeState.S0 -> {
                // SCE-MAP: a_delay_is_a_css2_time.scxml:39 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_delay_is_a_css2_time.scxml:26 :: _machine
    override fun executeTransitionContent(source: ADelayIsACss2TimeState, transitionIndex: Int) {
        when (source) {
        is ADelayIsACss2TimeState.S0 -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_delay_is_a_css2_time.scxml:53 :: s0 :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("errors", "errors"), com.sce.runtime.ScriptSource.lua("_scxml_add(errors, 1)", "errors + 1"))) {
                return
            }
            }
            1 -> {
                // SCE-MAP: a_delay_is_a_css2_time.scxml:56 :: s0 :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("bad", "bad"), com.sce.runtime.ScriptSource.lua("_scxml_add(bad, 1)", "bad + 1"))) {
                return
            }
            }
            2 -> {
                // SCE-MAP: a_delay_is_a_css2_time.scxml:59 :: s0 :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("bSeen", "bSeen"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            3 -> {
                // SCE-MAP: a_delay_is_a_css2_time.scxml:62 :: s0 :: _transition_3


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("aAfterB", "aAfterB"), com.sce.runtime.ScriptSource.lua("bSeen", "bSeen"))) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
