// SCE-GENERATED — DO NOT EDIT
// source-hash: 12784c68459b3d7eace7410d1b4623224107fed95aa065f0d94c55e5663fd9e2

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_self_sent_event_names_its_origin/a_self_sent_event_names_its_origin.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_self_sent_event_names_its_origin.scxml:32 :: _machine

package com.sce.integration.a_self_sent_event_names_its_origin

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ASelfSentEventNamesItsOriginState : State {
    data object BlankTarget : ASelfSentEventNamesItsOriginState
    data object Delayed : ASelfSentEventNamesItsOriginState
    data object Done : ASelfSentEventNamesItsOriginState
    data object Immediate : ASelfSentEventNamesItsOriginState
    data object Replying : ASelfSentEventNamesItsOriginState
    data object Run : ASelfSentEventNamesItsOriginState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ASelfSentEventNamesItsOriginEvent : Event {
    sealed interface Error : ASelfSentEventNamesItsOriginEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Later : ASelfSentEventNamesItsOriginEvent
    data object Nowhere : ASelfSentEventNamesItsOriginEvent
    data object Ping : ASelfSentEventNamesItsOriginEvent
    data object Pong : ASelfSentEventNamesItsOriginEvent
    data object Settle : ASelfSentEventNamesItsOriginEvent
}
// --- State Machine (W3C SCXML) ---

class ASelfSentEventNamesItsOriginStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ASelfSentEventNamesItsOriginState, ASelfSentEventNamesItsOriginEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `blank` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `blank` was assigned a value of another type, or the engine refused.
     */
    fun blank(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "blank")

    /**
     * §scxml-5.3: what the `immediateOk` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `immediateOk` was assigned a value of another type, or the engine refused.
     */
    fun immediateOk(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "immediateOk")

    /**
     * §scxml-5.3: what the `replied` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `replied` was assigned a value of another type, or the engine refused.
     */
    fun replied(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "replied")

    /**
     * §scxml-5.3: what the `delayedOk` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `delayedOk` was assigned a value of another type, or the engine refused.
     */
    fun delayedOk(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "delayedOk")

    /**
     * §scxml-5.3: what the `unreachable` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `unreachable` was assigned a value of another type, or the engine refused.
     */
    fun unreachable(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "unreachable")

    /**
     * §scxml-5.3: what the `strayed` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `strayed` was assigned a value of another type, or the engine refused.
     */
    fun strayed(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "strayed")

    override val initialState: ASelfSentEventNamesItsOriginState = ASelfSentEventNamesItsOriginState.Immediate

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
    override fun parentOf(state: ASelfSentEventNamesItsOriginState): ASelfSentEventNamesItsOriginState? = when (state) {
        is ASelfSentEventNamesItsOriginState.BlankTarget -> ASelfSentEventNamesItsOriginState.Run
        is ASelfSentEventNamesItsOriginState.Delayed -> ASelfSentEventNamesItsOriginState.Run
        is ASelfSentEventNamesItsOriginState.Immediate -> ASelfSentEventNamesItsOriginState.Run
        is ASelfSentEventNamesItsOriginState.Replying -> ASelfSentEventNamesItsOriginState.Run
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: ASelfSentEventNamesItsOriginState): Boolean = when (state) {
        is ASelfSentEventNamesItsOriginState.Run -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: ASelfSentEventNamesItsOriginState): Boolean = when (state) {
        is ASelfSentEventNamesItsOriginState.Done -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: ASelfSentEventNamesItsOriginState): List<ASelfSentEventNamesItsOriginState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: ASelfSentEventNamesItsOriginState): List<EntryTarget<ASelfSentEventNamesItsOriginState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ASelfSentEventNamesItsOriginState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<ASelfSentEventNamesItsOriginState, List<ASelfSentEventNamesItsOriginState>> = mapOf(
            ASelfSentEventNamesItsOriginState.Run to listOf(ASelfSentEventNamesItsOriginState.Immediate, ASelfSentEventNamesItsOriginState.Replying, ASelfSentEventNamesItsOriginState.Delayed, ASelfSentEventNamesItsOriginState.BlankTarget),
        )

        val initialTargets: Map<ASelfSentEventNamesItsOriginState, List<EntryTarget<ASelfSentEventNamesItsOriginState, HistoryId>>> = mapOf(
            ASelfSentEventNamesItsOriginState.Run to listOf(StateTarget(ASelfSentEventNamesItsOriginState.Immediate)),
        )

        val documentInitialTargetList: List<EntryTarget<ASelfSentEventNamesItsOriginState, HistoryId>> =
            listOf(StateTarget(ASelfSentEventNamesItsOriginState.Run))

        // W3C SCXML 3.13: blankTarget's transition 0, as the microstep reads it.
        val transitionBlankTargetAt0 = EnabledTransition<ASelfSentEventNamesItsOriginState, HistoryId>(
            ASelfSentEventNamesItsOriginState.BlankTarget,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: blankTarget's transition 1, as the microstep reads it.
        val transitionBlankTargetAt1 = EnabledTransition<ASelfSentEventNamesItsOriginState, HistoryId>(
            ASelfSentEventNamesItsOriginState.BlankTarget,
            listOf(StateTarget(ASelfSentEventNamesItsOriginState.Done)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: delayed's transition 0, as the microstep reads it.
        val transitionDelayedAt0 = EnabledTransition<ASelfSentEventNamesItsOriginState, HistoryId>(
            ASelfSentEventNamesItsOriginState.Delayed,
            listOf(StateTarget(ASelfSentEventNamesItsOriginState.BlankTarget)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: immediate's transition 0, as the microstep reads it.
        val transitionImmediateAt0 = EnabledTransition<ASelfSentEventNamesItsOriginState, HistoryId>(
            ASelfSentEventNamesItsOriginState.Immediate,
            listOf(StateTarget(ASelfSentEventNamesItsOriginState.Replying)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: replying's transition 0, as the microstep reads it.
        val transitionReplyingAt0 = EnabledTransition<ASelfSentEventNamesItsOriginState, HistoryId>(
            ASelfSentEventNamesItsOriginState.Replying,
            listOf(StateTarget(ASelfSentEventNamesItsOriginState.Delayed)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<ASelfSentEventNamesItsOriginState, HistoryId>(
            ASelfSentEventNamesItsOriginState.Run,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ASelfSentEventNamesItsOriginState? = when (stateId) {
        "blankTarget" -> ASelfSentEventNamesItsOriginState.BlankTarget
        "delayed" -> ASelfSentEventNamesItsOriginState.Delayed
        "done" -> ASelfSentEventNamesItsOriginState.Done
        "immediate" -> ASelfSentEventNamesItsOriginState.Immediate
        "replying" -> ASelfSentEventNamesItsOriginState.Replying
        "run" -> ASelfSentEventNamesItsOriginState.Run
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ASelfSentEventNamesItsOriginState): String = when (state) {
        is ASelfSentEventNamesItsOriginState.BlankTarget -> "blankTarget"
        is ASelfSentEventNamesItsOriginState.Delayed -> "delayed"
        is ASelfSentEventNamesItsOriginState.Done -> "done"
        is ASelfSentEventNamesItsOriginState.Immediate -> "immediate"
        is ASelfSentEventNamesItsOriginState.Replying -> "replying"
        is ASelfSentEventNamesItsOriginState.Run -> "run"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ASelfSentEventNamesItsOriginState): Int = when (state) {
        is ASelfSentEventNamesItsOriginState.BlankTarget -> 4
        is ASelfSentEventNamesItsOriginState.Delayed -> 3
        is ASelfSentEventNamesItsOriginState.Done -> 5
        is ASelfSentEventNamesItsOriginState.Immediate -> 1
        is ASelfSentEventNamesItsOriginState.Replying -> 2
        is ASelfSentEventNamesItsOriginState.Run -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ASelfSentEventNamesItsOriginEvent? = when (name) {
        "error.communication" -> ASelfSentEventNamesItsOriginEvent.Error.Communication
        "error.execution" -> ASelfSentEventNamesItsOriginEvent.Error.Execution
        "later" -> ASelfSentEventNamesItsOriginEvent.Later
        "nowhere" -> ASelfSentEventNamesItsOriginEvent.Nowhere
        "ping" -> ASelfSentEventNamesItsOriginEvent.Ping
        "pong" -> ASelfSentEventNamesItsOriginEvent.Pong
        "settle" -> ASelfSentEventNamesItsOriginEvent.Settle
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ASelfSentEventNamesItsOriginEvent): String? = when (event) {
        is ASelfSentEventNamesItsOriginEvent.Error.Communication -> "error.communication"
        is ASelfSentEventNamesItsOriginEvent.Error.Execution -> "error.execution"
        is ASelfSentEventNamesItsOriginEvent.Later -> "later"
        is ASelfSentEventNamesItsOriginEvent.Nowhere -> "nowhere"
        is ASelfSentEventNamesItsOriginEvent.Ping -> "ping"
        is ASelfSentEventNamesItsOriginEvent.Pong -> "pong"
        is ASelfSentEventNamesItsOriginEvent.Settle -> "settle"
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
            "a_self_sent_event_names_its_origin",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'blank' with expr
        try {
            val initResult_blank = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''"))
            engine.setVariable(sid, "blank", initResult_blank)
        } catch (e: Exception) {
            raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<data id='blank'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'immediateOk' with expr
        try {
            val initResult_immediateOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "immediateOk", initResult_immediateOk)
        } catch (e: Exception) {
            raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<data id='immediateOk'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'replied' with expr
        try {
            val initResult_replied = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "replied", initResult_replied)
        } catch (e: Exception) {
            raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<data id='replied'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'delayedOk' with expr
        try {
            val initResult_delayedOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "delayedOk", initResult_delayedOk)
        } catch (e: Exception) {
            raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<data id='delayedOk'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'unreachable' with expr
        try {
            val initResult_unreachable = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "unreachable", initResult_unreachable)
        } catch (e: Exception) {
            raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<data id='unreachable'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'strayed' with expr
        try {
            val initResult_strayed = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "strayed", initResult_strayed)
        } catch (e: Exception) {
            raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<data id='strayed'> expr failed to evaluate")
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
            raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ASelfSentEventNamesItsOriginEvent) {
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
    override fun bindCurrentEvent(event: ASelfSentEventNamesItsOriginEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ASelfSentEventNamesItsOriginState,
        event: ASelfSentEventNamesItsOriginEvent?
    ): EnabledTransition<ASelfSentEventNamesItsOriginState, HistoryId>? = when (state) {
        is ASelfSentEventNamesItsOriginState.BlankTarget -> when {
            event is ASelfSentEventNamesItsOriginEvent.Error.Communication -> transitionBlankTargetAt0
            event is ASelfSentEventNamesItsOriginEvent.Settle -> transitionBlankTargetAt1
            else -> null
        }
        is ASelfSentEventNamesItsOriginState.Delayed -> when {
            event is ASelfSentEventNamesItsOriginEvent.Later -> transitionDelayedAt0
            else -> null
        }
        is ASelfSentEventNamesItsOriginState.Immediate -> when {
            event is ASelfSentEventNamesItsOriginEvent.Ping -> transitionImmediateAt0
            else -> null
        }
        is ASelfSentEventNamesItsOriginState.Replying -> when {
            event is ASelfSentEventNamesItsOriginEvent.Pong -> transitionReplyingAt0
            else -> null
        }
        is ASelfSentEventNamesItsOriginState.Run -> when {
            event is ASelfSentEventNamesItsOriginEvent.Nowhere -> transitionRunAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_self_sent_event_names_its_origin.scxml:32 :: _machine
    override fun onEntry(state: ASelfSentEventNamesItsOriginState, isDefaultEntry: Boolean) {
        when (state) {
            is ASelfSentEventNamesItsOriginState.BlankTarget -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:78 :: blankTarget :: _state_body
                // W3C SCXML 3.8: Onentry block 1/2
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("blank", "blank")))
            } catch (_: Exception) {
                raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_3")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_3")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_3")
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
                eventName = "nowhere",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_3",
                eventData = sendData
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_3")
            }
            } else {
            // W3C SCXML 6.2: Dispatch to dynamically resolved target (C++ unified pattern)
            if (_rt == "#_internal") {
                raiseInternal(ASelfSentEventNamesItsOriginEvent.Nowhere, EventMetadata.internal(sendData))
            } else if (_rt == "#_parent") {
                onSendToParent?.invoke("nowhere", sendData)
            } else if (deliverToChildSession(
                    com.sce.runtime.IoProcessors.sessionIdFromScxmlLocation(_rt),
"nowhere",
                    sendData)) {
                // W3C SCXML C.1: the target decoded to one of our children's
                // published locations, so it is addressed to that child.
                // Without this arm the address a peer was told to answer at
                // routes back into the sender's own queue, so the location
                // compares equal and still reaches nobody.
            } else {
                send(ASelfSentEventNamesItsOriginEvent.Nowhere, EventMetadata.external(sendId = "__send_3", origin = scriptSessionId ?: "", data = sendData))
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 3.8: Onentry block 2/2
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ASelfSentEventNamesItsOriginEvent.Settle, EventMetadata.external(sendId = "__send_4", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is ASelfSentEventNamesItsOriginState.Delayed -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:67 :: delayed :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_2", 10L, ASelfSentEventNamesItsOriginEvent.Later, EventMetadata.external(sendId = "__send_2", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is ASelfSentEventNamesItsOriginState.Done -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:92 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is ASelfSentEventNamesItsOriginState.Immediate -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:49 :: immediate :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ASelfSentEventNamesItsOriginEvent.Ping, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is ASelfSentEventNamesItsOriginState.Replying -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:61 :: replying :: _state_body
            }
            is ASelfSentEventNamesItsOriginState.Run -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:44 :: run :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_self_sent_event_names_its_origin.scxml:32 :: _machine
    override fun onExit(state: ASelfSentEventNamesItsOriginState) {
        when (state) {
            is ASelfSentEventNamesItsOriginState.BlankTarget -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:78 :: blankTarget :: _state_body
            }
            is ASelfSentEventNamesItsOriginState.Delayed -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:67 :: delayed :: _state_body
            }
            is ASelfSentEventNamesItsOriginState.Done -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:92 :: done :: _state_body
            }
            is ASelfSentEventNamesItsOriginState.Immediate -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:49 :: immediate :: _state_body
            }
            is ASelfSentEventNamesItsOriginState.Replying -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:61 :: replying :: _state_body
            }
            is ASelfSentEventNamesItsOriginState.Run -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:44 :: run :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_self_sent_event_names_its_origin.scxml:32 :: _machine
    override fun executeTransitionContent(source: ASelfSentEventNamesItsOriginState, transitionIndex: Int) {
        when (source) {
        is ASelfSentEventNamesItsOriginState.BlankTarget -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:85 :: blankTarget :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("unreachable", "unreachable"), com.sce.runtime.ScriptSource.lua("_scxml_add(unreachable, 1)", "unreachable + 1"))) {
                return
            }
            }
            else -> {}
        }
        is ASelfSentEventNamesItsOriginState.Delayed -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:71 :: delayed :: _transition_0


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.origin == _ioprocessors.scxml.location)", "_event.origin === _ioprocessors['scxml'].location"))) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("delayedOk", "delayedOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            }
            else -> {}
        }
        is ASelfSentEventNamesItsOriginState.Immediate -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:53 :: immediate :: _transition_0


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.origin == _ioprocessors.scxml.location)", "_event.origin === _ioprocessors['scxml'].location"))) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("immediateOk", "immediateOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("_event.origin", "_event.origin")))
            } catch (_: Exception) {
                raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_0")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_0")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_0")
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
                eventName = "pong",
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
                raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_0")
            }
            } else {
            // W3C SCXML 6.2: Dispatch to dynamically resolved target (C++ unified pattern)
            if (_rt == "#_internal") {
                raiseInternal(ASelfSentEventNamesItsOriginEvent.Pong, EventMetadata.internal(sendData))
            } else if (_rt == "#_parent") {
                onSendToParent?.invoke("pong", sendData)
            } else if (deliverToChildSession(
                    com.sce.runtime.IoProcessors.sessionIdFromScxmlLocation(_rt),
"pong",
                    sendData)) {
                // W3C SCXML C.1: the target decoded to one of our children's
                // published locations, so it is addressed to that child.
                // Without this arm the address a peer was told to answer at
                // routes back into the sender's own queue, so the location
                // compares equal and still reaches nobody.
            } else {
                send(ASelfSentEventNamesItsOriginEvent.Pong, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            else -> {}
        }
        is ASelfSentEventNamesItsOriginState.Replying -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:62 :: replying :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("replied", "replied"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            else -> {}
        }
        is ASelfSentEventNamesItsOriginState.Run -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:45 :: run :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("strayed", "strayed"), com.sce.runtime.ScriptSource.lua("_scxml_add(strayed, 1)", "strayed + 1"))) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
