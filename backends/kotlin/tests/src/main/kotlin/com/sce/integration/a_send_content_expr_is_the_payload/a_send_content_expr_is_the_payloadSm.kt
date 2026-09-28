// SCE-GENERATED — DO NOT EDIT
// source-hash: edc8151308e968ebf2d38f51e45d352c0d4593d6e370d53dd707aeb48d704734

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_send_content_expr_is_the_payload/a_send_content_expr_is_the_payload.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_send_content_expr_is_the_payload.scxml:36 :: _machine

package com.sce.integration.a_send_content_expr_is_the_payload

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ASendContentExprIsThePayloadState : State {
    data object Done : ASendContentExprIsThePayloadState
    data object Run : ASendContentExprIsThePayloadState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ASendContentExprIsThePayloadEvent : Event {
    data object Bad : ASendContentExprIsThePayloadEvent
    sealed interface Error : ASendContentExprIsThePayloadEvent {
        data object Execution : Error
    }
    data object Number : ASendContentExprIsThePayloadEvent
    data object Object : ASendContentExprIsThePayloadEvent
    data object Settle : ASendContentExprIsThePayloadEvent
    data object Text : ASendContentExprIsThePayloadEvent
}
// --- State Machine (W3C SCXML) ---

class ASendContentExprIsThePayloadStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ASendContentExprIsThePayloadState, ASendContentExprIsThePayloadEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

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
     * §scxml-5.3: what the `numberOk` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `numberOk` was assigned a value of another type, or the engine refused.
     */
    fun numberOk(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "numberOk")

    /**
     * §scxml-5.3: what the `objectOk` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `objectOk` was assigned a value of another type, or the engine refused.
     */
    fun objectOk(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "objectOk")

    /**
     * §scxml-5.3: what the `textOk` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `textOk` was assigned a value of another type, or the engine refused.
     */
    fun textOk(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "textOk")

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
     * §scxml-5.3: what the `badArrived` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `badArrived` was assigned a value of another type, or the engine refused.
     */
    fun badArrived(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "badArrived")

    /**
     * §scxml-5.3: what the `badEmpty` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `badEmpty` was assigned a value of another type, or the engine refused.
     */
    fun badEmpty(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "badEmpty")

    /**
     * §scxml-5.3: what the `afterBad` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterBad` was assigned a value of another type, or the engine refused.
     */
    fun afterBad(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterBad")

    override val initialState: ASendContentExprIsThePayloadState = ASendContentExprIsThePayloadState.Run

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
    override fun isFinalState(state: ASendContentExprIsThePayloadState): Boolean = when (state) {
        is ASendContentExprIsThePayloadState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ASendContentExprIsThePayloadState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<ASendContentExprIsThePayloadState, HistoryId>> =
            listOf(StateTarget(ASendContentExprIsThePayloadState.Run))

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<ASendContentExprIsThePayloadState, HistoryId>(
            ASendContentExprIsThePayloadState.Run,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 1, as the microstep reads it.
        val transitionRunAt1 = EnabledTransition<ASendContentExprIsThePayloadState, HistoryId>(
            ASendContentExprIsThePayloadState.Run,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 2, as the microstep reads it.
        val transitionRunAt2 = EnabledTransition<ASendContentExprIsThePayloadState, HistoryId>(
            ASendContentExprIsThePayloadState.Run,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 3, as the microstep reads it.
        val transitionRunAt3 = EnabledTransition<ASendContentExprIsThePayloadState, HistoryId>(
            ASendContentExprIsThePayloadState.Run,
            emptyList(),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 4, as the microstep reads it.
        val transitionRunAt4 = EnabledTransition<ASendContentExprIsThePayloadState, HistoryId>(
            ASendContentExprIsThePayloadState.Run,
            emptyList(),
            4,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 5, as the microstep reads it.
        val transitionRunAt5 = EnabledTransition<ASendContentExprIsThePayloadState, HistoryId>(
            ASendContentExprIsThePayloadState.Run,
            listOf(StateTarget(ASendContentExprIsThePayloadState.Done)),
            5,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ASendContentExprIsThePayloadState? = when (stateId) {
        "done" -> ASendContentExprIsThePayloadState.Done
        "run" -> ASendContentExprIsThePayloadState.Run
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ASendContentExprIsThePayloadState): String = when (state) {
        is ASendContentExprIsThePayloadState.Done -> "done"
        is ASendContentExprIsThePayloadState.Run -> "run"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ASendContentExprIsThePayloadState): Int = when (state) {
        is ASendContentExprIsThePayloadState.Done -> 1
        is ASendContentExprIsThePayloadState.Run -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ASendContentExprIsThePayloadEvent? = when (name) {
        "bad" -> ASendContentExprIsThePayloadEvent.Bad
        "error.execution" -> ASendContentExprIsThePayloadEvent.Error.Execution
        "number" -> ASendContentExprIsThePayloadEvent.Number
        "object" -> ASendContentExprIsThePayloadEvent.Object
        "settle" -> ASendContentExprIsThePayloadEvent.Settle
        "text" -> ASendContentExprIsThePayloadEvent.Text
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ASendContentExprIsThePayloadEvent): String? = when (event) {
        is ASendContentExprIsThePayloadEvent.Bad -> "bad"
        is ASendContentExprIsThePayloadEvent.Error.Execution -> "error.execution"
        is ASendContentExprIsThePayloadEvent.Number -> "number"
        is ASendContentExprIsThePayloadEvent.Object -> "object"
        is ASendContentExprIsThePayloadEvent.Settle -> "settle"
        is ASendContentExprIsThePayloadEvent.Text -> "text"
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
            "a_send_content_expr_is_the_payload",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'obj' with expr
        try {
            val initResult_obj = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{}", "({})"))
            engine.setVariable(sid, "obj", initResult_obj)
        } catch (e: Exception) {
            raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<data id='obj'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'numberOk' with expr
        try {
            val initResult_numberOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "numberOk", initResult_numberOk)
        } catch (e: Exception) {
            raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<data id='numberOk'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'objectOk' with expr
        try {
            val initResult_objectOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "objectOk", initResult_objectOk)
        } catch (e: Exception) {
            raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<data id='objectOk'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'textOk' with expr
        try {
            val initResult_textOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "textOk", initResult_textOk)
        } catch (e: Exception) {
            raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<data id='textOk'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'errors' with expr
        try {
            val initResult_errors = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "errors", initResult_errors)
        } catch (e: Exception) {
            raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<data id='errors'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'badArrived' with expr
        try {
            val initResult_badArrived = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "badArrived", initResult_badArrived)
        } catch (e: Exception) {
            raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<data id='badArrived'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'badEmpty' with expr
        try {
            val initResult_badEmpty = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "badEmpty", initResult_badEmpty)
        } catch (e: Exception) {
            raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<data id='badEmpty'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterBad' with expr
        try {
            val initResult_afterBad = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterBad", initResult_afterBad)
        } catch (e: Exception) {
            raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<data id='afterBad'> expr failed to evaluate")
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
            raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ASendContentExprIsThePayloadEvent) {
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
    override fun bindCurrentEvent(event: ASendContentExprIsThePayloadEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ASendContentExprIsThePayloadState,
        event: ASendContentExprIsThePayloadEvent?
    ): EnabledTransition<ASendContentExprIsThePayloadState, HistoryId>? = when (state) {
        is ASendContentExprIsThePayloadState.Run -> when {
            event is ASendContentExprIsThePayloadEvent.Error.Execution -> transitionRunAt0
            event is ASendContentExprIsThePayloadEvent.Number && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data == 42)", "_event.data === 42")) -> transitionRunAt1
            event is ASendContentExprIsThePayloadEvent.Object && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.a == 1)", "_event.data.a === 1")) -> transitionRunAt2
            event is ASendContentExprIsThePayloadEvent.Text && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data == \"hi\")", "_event.data === 'hi'")) -> transitionRunAt3
            event is ASendContentExprIsThePayloadEvent.Bad -> transitionRunAt4
            event is ASendContentExprIsThePayloadEvent.Settle -> transitionRunAt5
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_send_content_expr_is_the_payload.scxml:36 :: _machine
    override fun onEntry(state: ASendContentExprIsThePayloadState, isDefaultEntry: Boolean) {
        when (state) {
            is ASendContentExprIsThePayloadState.Done -> {
                // SCE-MAP: a_send_content_expr_is_the_payload.scxml:85 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is ASendContentExprIsThePayloadState.Run -> {
                // SCE-MAP: a_send_content_expr_is_the_payload.scxml:50 :: run :: _state_body
                // W3C SCXML 3.8: Onentry block 1/3
                run {


            if (run send@{
            var paramFailed = false
            // W3C SCXML 5.6.2: the value of <content expr> is the event's data.
            // "If the evaluation of 'expr' produces an error, the Processor MUST
            // place error.execution in the internal event queue and use the
            // empty string as the value of the <content> element": the message
            // still goes, carrying "", and — like a failing <param> — the error
            // ends the block once it has (§scxml-4.9).
            ensureScriptEngine()
            val sendData = try {
                val sendContentValue = (scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)"))
                    .evaluateExpr(scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)"), com.sce.runtime.ScriptSource.lua("(40 + 2)", "40 + 2"))
                if (sendContentValue != null) valueToJson(sendContentValue) else ""
            } catch (_: Exception) {
                raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<send> contentexpr failed to evaluate", "__send_0")
                paramFailed = true
                valueToJson("")
            }
            // W3C SCXML 6.2: send to this session's external queue
            send(ASendContentExprIsThePayloadEvent.Number, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            var paramFailed = false
            // W3C SCXML 5.6.2: the value of <content expr> is the event's data.
            // "If the evaluation of 'expr' produces an error, the Processor MUST
            // place error.execution in the internal event queue and use the
            // empty string as the value of the <content> element": the message
            // still goes, carrying "", and — like a failing <param> — the error
            // ends the block once it has (§scxml-4.9).
            ensureScriptEngine()
            val sendData = try {
                val sendContentValue = (scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)"))
                    .evaluateExpr(scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)"), com.sce.runtime.ScriptSource.lua("{[\"a\"] = 1}", "({a: 1})"))
                if (sendContentValue != null) valueToJson(sendContentValue) else ""
            } catch (_: Exception) {
                raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<send> contentexpr failed to evaluate", "__send_1")
                paramFailed = true
                valueToJson("")
            }
            // W3C SCXML 6.2: send to this session's external queue
            send(ASendContentExprIsThePayloadEvent.Object, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            var paramFailed = false
            // W3C SCXML 5.6.2: the value of <content expr> is the event's data.
            // "If the evaluation of 'expr' produces an error, the Processor MUST
            // place error.execution in the internal event queue and use the
            // empty string as the value of the <content> element": the message
            // still goes, carrying "", and — like a failing <param> — the error
            // ends the block once it has (§scxml-4.9).
            ensureScriptEngine()
            val sendData = try {
                val sendContentValue = (scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)"))
                    .evaluateExpr(scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)"), com.sce.runtime.ScriptSource.lua("\"hi\"", "'hi'"))
                if (sendContentValue != null) valueToJson(sendContentValue) else ""
            } catch (_: Exception) {
                raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<send> contentexpr failed to evaluate", "__send_2")
                paramFailed = true
                valueToJson("")
            }
            // W3C SCXML 6.2: send to this session's external queue
            send(ASendContentExprIsThePayloadEvent.Text, EventMetadata.external(sendId = "__send_2", origin = scriptSessionId ?: "", data = sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 3.8: Onentry block 2/3
                run {


            if (run send@{
            var paramFailed = false
            // W3C SCXML 5.6.2: the value of <content expr> is the event's data.
            // "If the evaluation of 'expr' produces an error, the Processor MUST
            // place error.execution in the internal event queue and use the
            // empty string as the value of the <content> element": the message
            // still goes, carrying "", and — like a failing <param> — the error
            // ends the block once it has (§scxml-4.9).
            ensureScriptEngine()
            val sendData = try {
                val sendContentValue = (scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)"))
                    .evaluateExpr(scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)"), com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"))
                if (sendContentValue != null) valueToJson(sendContentValue) else ""
            } catch (_: Exception) {
                raisePlatformError(ASendContentExprIsThePayloadEvent.Error.Execution, "<send> contentexpr failed to evaluate", "__send_3")
                paramFailed = true
                valueToJson("")
            }
            // W3C SCXML 6.2: send to this session's external queue
            send(ASendContentExprIsThePayloadEvent.Bad, EventMetadata.external(sendId = "__send_3", origin = scriptSessionId ?: "", data = sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterBad", "afterBad"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 3/3
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ASendContentExprIsThePayloadEvent.Settle, EventMetadata.external(sendId = "__send_4", origin = scriptSessionId ?: "", data = sendData))
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
    // SCE-MAP: a_send_content_expr_is_the_payload.scxml:36 :: _machine
    override fun onExit(state: ASendContentExprIsThePayloadState) {
        when (state) {
            is ASendContentExprIsThePayloadState.Done -> {
                // SCE-MAP: a_send_content_expr_is_the_payload.scxml:85 :: done :: _state_body
            }
            is ASendContentExprIsThePayloadState.Run -> {
                // SCE-MAP: a_send_content_expr_is_the_payload.scxml:50 :: run :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_send_content_expr_is_the_payload.scxml:36 :: _machine
    override fun executeTransitionContent(source: ASendContentExprIsThePayloadState, transitionIndex: Int) {
        when (source) {
        is ASendContentExprIsThePayloadState.Run -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_send_content_expr_is_the_payload.scxml:64 :: run :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("errors", "errors"), com.sce.runtime.ScriptSource.lua("_scxml_add(errors, 1)", "errors + 1"))) {
                return
            }
            }
            1 -> {
                // SCE-MAP: a_send_content_expr_is_the_payload.scxml:67 :: run :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("numberOk", "numberOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            2 -> {
                // SCE-MAP: a_send_content_expr_is_the_payload.scxml:70 :: run :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("objectOk", "objectOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            3 -> {
                // SCE-MAP: a_send_content_expr_is_the_payload.scxml:73 :: run :: _transition_3


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("textOk", "textOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            4 -> {
                // SCE-MAP: a_send_content_expr_is_the_payload.scxml:76 :: run :: _transition_4


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("badArrived", "badArrived"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data == \"\")", "_event.data === ''"))) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("badEmpty", "badEmpty"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
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
