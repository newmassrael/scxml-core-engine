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

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

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
        val childStates: Map<ASelfSentEventNamesItsOriginState, List<ASelfSentEventNamesItsOriginState>> = LinkedHashMap<ASelfSentEventNamesItsOriginState, List<ASelfSentEventNamesItsOriginState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<ASelfSentEventNamesItsOriginState, List<ASelfSentEventNamesItsOriginState>>) {
            table[ASelfSentEventNamesItsOriginState.Run] = listOf(ASelfSentEventNamesItsOriginState.Immediate, ASelfSentEventNamesItsOriginState.Replying, ASelfSentEventNamesItsOriginState.Delayed, ASelfSentEventNamesItsOriginState.BlankTarget)
        }


        val initialTargets: Map<ASelfSentEventNamesItsOriginState, List<EntryTarget<ASelfSentEventNamesItsOriginState, HistoryId>>> = LinkedHashMap<ASelfSentEventNamesItsOriginState, List<EntryTarget<ASelfSentEventNamesItsOriginState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<ASelfSentEventNamesItsOriginState, List<EntryTarget<ASelfSentEventNamesItsOriginState, HistoryId>>>) {
            table[ASelfSentEventNamesItsOriginState.Run] = listOf(StateTarget(ASelfSentEventNamesItsOriginState.Immediate))
        }


        val documentInitialTargetList: List<EntryTarget<ASelfSentEventNamesItsOriginState, HistoryId>> =
            listOf(StateTarget(ASelfSentEventNamesItsOriginState.Run))
    }

    private object TransitionsPart0 {

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

        declareVariablesPart0(engine, sid)




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

    @Suppress("UNUSED_PARAMETER")
    private fun declareVariablesPart0(engine: ScxmlScriptEngine, sid: String) {
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
    private fun safeEvaluateGuard(guardExpr: com.sce.runtime.ScriptSource): Boolean =
        evaluateGuardRaising(guardExpr, "a <transition> cond failed to evaluate") ?: false

    // W3C SCXML 5.9.1 + 4.9: an <if> or <elseif> cond, evaluated and reported
    // as a transition guard is. A failure also runs [onFailure]: the <if>
    // still selects on `false`, and is then the element whose processing
    // raised, so its block ends after it.
    @Suppress("unused")
    private inline fun evaluateIfCond(guardExpr: com.sce.runtime.ScriptSource, onFailure: () -> Unit): Boolean {
        val result = evaluateGuardRaising(guardExpr, "an <if> cond failed to evaluate")
        if (result == null) onFailure()
        return result ?: false
    }

    // W3C SCXML 5.9.1: a cond that cannot be evaluated raises error.execution;
    // `null` says so, where a bare `false` could not.
    private fun evaluateGuardRaising(guardExpr: com.sce.runtime.ScriptSource, reason: String): Boolean? {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, reason)
            null
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
        val meta = currentEventMetadata
        // §scxml-5.10: the name the event ARRIVED under, which is longer than its
        // member's when a name the document does not write was matched through a
        // prefix (§scxml-3.12.1).
        val eventName = meta.name.ifEmpty { eventNameOf(event) ?: return }
        // W3C SCXML 5.10.1: C++ classifyEventType — platform events override type
        val effectiveType = when {
            eventName.startsWith("done.") || eventName.startsWith("error.") -> "platform"
            else -> meta.type
        }
        // W3C SCXML 5.10.1 + C.1: `_event.origin` and `_event.origintype` are the
        // reply address the processor that delivered the event wrote where it
        // built it, as one value (docs/adr/0090): the SCXML processor's type and
        // the sender's published `_ioprocessors` location for an event a session
        // sent, a host's own pair for what a host named, the Mesh transport's
        // for a peer. Read here as written, with no conversion. An event no
        // processor stamped — an internal or platform event, a host-posted event
        // or reply that names no party, a BasicHTTP request — has none and is
        // handed over as the empty string, which the binding surfaces as blank
        // (undefined, §scxml-B-2-8): nothing is filled in for it, neither this
        // session's own address nor the SCXML processor's type.
        val eventOrigin = meta.origin
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
                origin = eventOrigin?.address ?: "",
                originType = eventOrigin?.processor ?: "",
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
    override fun firstEnabledTransition(state: ASelfSentEventNamesItsOriginState, event: ASelfSentEventNamesItsOriginEvent?): EnabledTransition<ASelfSentEventNamesItsOriginState, HistoryId>? = when (state) {
        is ASelfSentEventNamesItsOriginState.BlankTarget -> firstEnabledTransitionOfBlankTarget(state, event)
        is ASelfSentEventNamesItsOriginState.Delayed -> firstEnabledTransitionOfDelayed(state, event)
        is ASelfSentEventNamesItsOriginState.Immediate -> firstEnabledTransitionOfImmediate(state, event)
        is ASelfSentEventNamesItsOriginState.Replying -> firstEnabledTransitionOfReplying(state, event)
        is ASelfSentEventNamesItsOriginState.Run -> firstEnabledTransitionOfRun(state, event)
        else -> null
    }


    // blankTarget's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfBlankTarget(state: ASelfSentEventNamesItsOriginState, event: ASelfSentEventNamesItsOriginEvent?): EnabledTransition<ASelfSentEventNamesItsOriginState, HistoryId>? = when {
        event is ASelfSentEventNamesItsOriginEvent.Error.Communication -> TransitionsPart0.transitionBlankTargetAt0
        event is ASelfSentEventNamesItsOriginEvent.Settle -> TransitionsPart0.transitionBlankTargetAt1
        else -> null
    }


    // delayed's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfDelayed(state: ASelfSentEventNamesItsOriginState, event: ASelfSentEventNamesItsOriginEvent?): EnabledTransition<ASelfSentEventNamesItsOriginState, HistoryId>? = when {
        event is ASelfSentEventNamesItsOriginEvent.Later -> TransitionsPart0.transitionDelayedAt0
        else -> null
    }


    // immediate's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfImmediate(state: ASelfSentEventNamesItsOriginState, event: ASelfSentEventNamesItsOriginEvent?): EnabledTransition<ASelfSentEventNamesItsOriginState, HistoryId>? = when {
        event is ASelfSentEventNamesItsOriginEvent.Ping -> TransitionsPart0.transitionImmediateAt0
        else -> null
    }


    // replying's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfReplying(state: ASelfSentEventNamesItsOriginState, event: ASelfSentEventNamesItsOriginEvent?): EnabledTransition<ASelfSentEventNamesItsOriginState, HistoryId>? = when {
        event is ASelfSentEventNamesItsOriginEvent.Pong -> TransitionsPart0.transitionReplyingAt0
        else -> null
    }


    // run's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRun(state: ASelfSentEventNamesItsOriginState, event: ASelfSentEventNamesItsOriginEvent?): EnabledTransition<ASelfSentEventNamesItsOriginState, HistoryId>? = when {
        event is ASelfSentEventNamesItsOriginEvent.Nowhere -> TransitionsPart0.transitionRunAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_self_sent_event_names_its_origin.scxml:32 :: _machine
    override fun onEntry(state: ASelfSentEventNamesItsOriginState, isDefaultEntry: Boolean) {
        when (state) {
            is ASelfSentEventNamesItsOriginState.BlankTarget -> onEntryOfBlankTarget()
            is ASelfSentEventNamesItsOriginState.Delayed -> onEntryOfDelayed()
            is ASelfSentEventNamesItsOriginState.Done -> onEntryOfDone()
            is ASelfSentEventNamesItsOriginState.Immediate -> onEntryOfImmediate()
            is ASelfSentEventNamesItsOriginState.Replying -> onEntryOfReplying()
            is ASelfSentEventNamesItsOriginState.Run -> onEntryOfRun()
        }
    }


    private fun onEntryOfBlankTarget() {
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
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
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
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ASelfSentEventNamesItsOriginEvent.Nowhere,
"nowhere",
                sendData,
0L,
                "__send_3",
                ASelfSentEventNamesItsOriginEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_3")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_3")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
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
            send(ASelfSentEventNamesItsOriginEvent.Settle, EventMetadata.sentBy(sessionId, sendId = "__send_4", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfDelayed() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:67 :: delayed :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_2", 10L, ASelfSentEventNamesItsOriginEvent.Later, EventMetadata.sentBy(sessionId, sendId = "__send_2", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfDone() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:92 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfImmediate() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:49 :: immediate :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ASelfSentEventNamesItsOriginEvent.Ping, EventMetadata.sentBy(sessionId, sendId = "__send_1", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfReplying() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:61 :: replying :: _state_body
    }

    private fun onEntryOfRun() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:44 :: run :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_self_sent_event_names_its_origin.scxml:32 :: _machine
    override fun onExit(state: ASelfSentEventNamesItsOriginState) {
        when (state) {
            is ASelfSentEventNamesItsOriginState.BlankTarget -> onExitOfBlankTarget()
            is ASelfSentEventNamesItsOriginState.Delayed -> onExitOfDelayed()
            is ASelfSentEventNamesItsOriginState.Done -> onExitOfDone()
            is ASelfSentEventNamesItsOriginState.Immediate -> onExitOfImmediate()
            is ASelfSentEventNamesItsOriginState.Replying -> onExitOfReplying()
            is ASelfSentEventNamesItsOriginState.Run -> onExitOfRun()
        }
    }


    private fun onExitOfBlankTarget() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:78 :: blankTarget :: _state_body
    }

    private fun onExitOfDelayed() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:67 :: delayed :: _state_body
    }

    private fun onExitOfDone() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:92 :: done :: _state_body
    }

    private fun onExitOfImmediate() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:49 :: immediate :: _state_body
    }

    private fun onExitOfReplying() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:61 :: replying :: _state_body
    }

    private fun onExitOfRun() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:44 :: run :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_self_sent_event_names_its_origin.scxml:32 :: _machine
    override fun executeTransitionContent(source: ASelfSentEventNamesItsOriginState, transitionIndex: Int) {
        when (source) {
            is ASelfSentEventNamesItsOriginState.BlankTarget -> transitionContentOfBlankTarget(transitionIndex)
            is ASelfSentEventNamesItsOriginState.Delayed -> transitionContentOfDelayed(transitionIndex)
            is ASelfSentEventNamesItsOriginState.Immediate -> transitionContentOfImmediate(transitionIndex)
            is ASelfSentEventNamesItsOriginState.Replying -> transitionContentOfReplying(transitionIndex)
            is ASelfSentEventNamesItsOriginState.Run -> transitionContentOfRun(transitionIndex)
            else -> {}
        }
    }


    // The content of `blankTarget`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfBlankTarget(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfBlankTargetAt0()
            else -> {}
        }
    }


    private fun transitionContentOfBlankTargetAt0() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:85 :: blankTarget :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("unreachable", "unreachable"), com.sce.runtime.ScriptSource.lua("_scxml_add(unreachable, 1)", "unreachable + 1"))) {
                return
            }
    }

    // The content of `delayed`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfDelayed(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfDelayedAt0()
            else -> {}
        }
    }


    private fun transitionContentOfDelayedAt0() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:71 :: delayed :: _transition_0


            var ifCondFailed2 = false
            if (evaluateIfCond(com.sce.runtime.ScriptSource.lua("(_event.origin == _ioprocessors.scxml.location)", "_event.origin === _ioprocessors['scxml'].location")) { ifCondFailed2 = true }) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("delayedOk", "delayedOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            if (ifCondFailed2) {
                return
            }
    }

    // The content of `immediate`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfImmediate(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfImmediateAt0()
            else -> {}
        }
    }


    private fun transitionContentOfImmediateAt0() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:53 :: immediate :: _transition_0


            var ifCondFailed1 = false
            if (evaluateIfCond(com.sce.runtime.ScriptSource.lua("(_event.origin == _ioprocessors.scxml.location)", "_event.origin === _ioprocessors['scxml'].location")) { ifCondFailed1 = true }) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("immediateOk", "immediateOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            if (ifCondFailed1) {
                return
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
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
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
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ASelfSentEventNamesItsOriginEvent.Pong,
"pong",
                sendData,
0L,
                "__send_0",
                ASelfSentEventNamesItsOriginEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_0")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ASelfSentEventNamesItsOriginEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_0")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    // The content of `replying`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfReplying(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfReplyingAt0()
            else -> {}
        }
    }


    private fun transitionContentOfReplyingAt0() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:62 :: replying :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("replied", "replied"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
    }

    // The content of `run`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfRun(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfRunAt0()
            else -> {}
        }
    }


    private fun transitionContentOfRunAt0() {
                // SCE-MAP: a_self_sent_event_names_its_origin.scxml:45 :: run :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("strayed", "strayed"), com.sce.runtime.ScriptSource.lua("_scxml_add(strayed, 1)", "strayed + 1"))) {
                return
            }
    }
}
