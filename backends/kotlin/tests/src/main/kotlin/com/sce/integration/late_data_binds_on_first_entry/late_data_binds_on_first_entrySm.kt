// SCE-GENERATED — DO NOT EDIT
// source-hash: f405954883fe8b5ac761892a68f03f553375b1707e757f6f2f41cd84b4cef228

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/late_data_binds_on_first_entry/late_data_binds_on_first_entry.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: late_data_binds_on_first_entry.scxml:26 :: _machine

package com.sce.integration.late_data_binds_on_first_entry

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface LateDataBindsOnFirstEntryState : State {
    data object Idle : LateDataBindsOnFirstEntryState
    data object S : LateDataBindsOnFirstEntryState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface LateDataBindsOnFirstEntryEvent : Event {
    data object Back : LateDataBindsOnFirstEntryEvent
    data object Bump : LateDataBindsOnFirstEntryEvent
    sealed interface Error : LateDataBindsOnFirstEntryEvent {
        data object Execution : Error
    }
    data object Go : LateDataBindsOnFirstEntryEvent
}
// --- State Machine (W3C SCXML) ---

class LateDataBindsOnFirstEntryStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<LateDataBindsOnFirstEntryState, LateDataBindsOnFirstEntryEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `entries` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `entries` was assigned a value of another type, or the engine refused.
     */
    fun entries(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "entries")

    /**
     * §scxml-5.3: what the `seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `seen` was assigned a value of another type, or the engine refused.
     */
    fun seen(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "seen")

    /**
     * §scxml-5.3: what the `contentSeen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `contentSeen` was assigned a value of another type, or the engine refused.
     */
    fun contentSeen(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "contentSeen")

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

    override val initialState: LateDataBindsOnFirstEntryState = LateDataBindsOnFirstEntryState.Idle

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
    override val documentInitialTargets: List<EntryTarget<LateDataBindsOnFirstEntryState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<LateDataBindsOnFirstEntryState, HistoryId>> =
            listOf(StateTarget(LateDataBindsOnFirstEntryState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<LateDataBindsOnFirstEntryState, HistoryId>(
            LateDataBindsOnFirstEntryState.Idle,
            listOf(StateTarget(LateDataBindsOnFirstEntryState.S)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s's transition 0, as the microstep reads it.
        val transitionSAt0 = EnabledTransition<LateDataBindsOnFirstEntryState, HistoryId>(
            LateDataBindsOnFirstEntryState.S,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s's transition 1, as the microstep reads it.
        val transitionSAt1 = EnabledTransition<LateDataBindsOnFirstEntryState, HistoryId>(
            LateDataBindsOnFirstEntryState.S,
            listOf(StateTarget(LateDataBindsOnFirstEntryState.Idle)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): LateDataBindsOnFirstEntryState? = when (stateId) {
        "idle" -> LateDataBindsOnFirstEntryState.Idle
        "s" -> LateDataBindsOnFirstEntryState.S
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: LateDataBindsOnFirstEntryState): String = when (state) {
        is LateDataBindsOnFirstEntryState.Idle -> "idle"
        is LateDataBindsOnFirstEntryState.S -> "s"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: LateDataBindsOnFirstEntryState): Int = when (state) {
        is LateDataBindsOnFirstEntryState.Idle -> 0
        is LateDataBindsOnFirstEntryState.S -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): LateDataBindsOnFirstEntryEvent? = when (name) {
        "back" -> LateDataBindsOnFirstEntryEvent.Back
        "bump" -> LateDataBindsOnFirstEntryEvent.Bump
        "error.execution" -> LateDataBindsOnFirstEntryEvent.Error.Execution
        "go" -> LateDataBindsOnFirstEntryEvent.Go
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: LateDataBindsOnFirstEntryEvent): String? = when (event) {
        is LateDataBindsOnFirstEntryEvent.Back -> "back"
        is LateDataBindsOnFirstEntryEvent.Bump -> "bump"
        is LateDataBindsOnFirstEntryEvent.Error.Execution -> "error.execution"
        is LateDataBindsOnFirstEntryEvent.Go -> "go"
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
            "late_data_binds_on_first_entry",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'entries' with expr
        try {
            val initResult_entries = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "entries", initResult_entries)
        } catch (e: Exception) {
            raisePlatformError(LateDataBindsOnFirstEntryEvent.Error.Execution, "<data id='entries'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'seen' with expr
        try {
            val initResult_seen = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "seen", initResult_seen)
        } catch (e: Exception) {
            raisePlatformError(LateDataBindsOnFirstEntryEvent.Error.Execution, "<data id='seen'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'contentSeen' with expr
        try {
            val initResult_contentSeen = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "contentSeen", initResult_contentSeen)
        } catch (e: Exception) {
            raisePlatformError(LateDataBindsOnFirstEntryEvent.Error.Execution, "<data id='contentSeen'> expr failed to evaluate")
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
            raisePlatformError(LateDataBindsOnFirstEntryEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(LateDataBindsOnFirstEntryEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(LateDataBindsOnFirstEntryEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(LateDataBindsOnFirstEntryEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: LateDataBindsOnFirstEntryEvent) {
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
    override fun bindCurrentEvent(event: LateDataBindsOnFirstEntryEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: LateDataBindsOnFirstEntryState,
        event: LateDataBindsOnFirstEntryEvent?
    ): EnabledTransition<LateDataBindsOnFirstEntryState, HistoryId>? = when (state) {
        is LateDataBindsOnFirstEntryState.Idle -> when {
            event is LateDataBindsOnFirstEntryEvent.Go -> transitionIdleAt0
            else -> null
        }
        is LateDataBindsOnFirstEntryState.S -> when {
            event is LateDataBindsOnFirstEntryEvent.Bump -> transitionSAt0
            event is LateDataBindsOnFirstEntryEvent.Back -> transitionSAt1
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: late_data_binds_on_first_entry.scxml:26 :: _machine
    override fun onEntry(state: LateDataBindsOnFirstEntryState, isDefaultEntry: Boolean) {
        when (state) {
            is LateDataBindsOnFirstEntryState.Idle -> {
                // SCE-MAP: late_data_binds_on_first_entry.scxml:36 :: idle :: _state_body
            }
            is LateDataBindsOnFirstEntryState.S -> {
                // SCE-MAP: late_data_binds_on_first_entry.scxml:40 :: s :: _state_body
                // §scxml-D-enterStates, late binding: the state's <data> is bound
                // on its FIRST entry only (s.isFirstEntry), before its <onentry>.
                // The engine owns the first-entry rule; this is only what binding
                // means for this state.
                if (claimLateBindingFirstEntry(state)) {
                    ensureScriptEngine()
                    val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    try {
                        val v = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("1", "1"))
                        engine.setVariable(sid, "v", v)
                    } catch (e: Exception) {
                        raisePlatformError(LateDataBindsOnFirstEntryEvent.Error.Execution, "<data id='v'> expr failed to evaluate")
                    }
                    try {
                        val v = engine.parseDataValue(sid, "7")
                        engine.setVariable(sid, "c", v)
                    } catch (e: Exception) {
                        raisePlatformError(LateDataBindsOnFirstEntryEvent.Error.Execution, "<data id='c'> content failed to initialise")
                    }
                }


            executeAssign(com.sce.runtime.ScriptSource.lua("entries", "entries"), com.sce.runtime.ScriptSource.lua("_scxml_add(entries, 1)", "entries + 1"))


            executeAssign(com.sce.runtime.ScriptSource.lua("seen", "seen"), com.sce.runtime.ScriptSource.lua("_scxml_add((seen * 10), v)", "seen * 10 + v"))


            executeAssign(com.sce.runtime.ScriptSource.lua("contentSeen", "contentSeen"), com.sce.runtime.ScriptSource.lua("c", "c"))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: late_data_binds_on_first_entry.scxml:26 :: _machine
    override fun onExit(state: LateDataBindsOnFirstEntryState) {
        when (state) {
            is LateDataBindsOnFirstEntryState.Idle -> {
                // SCE-MAP: late_data_binds_on_first_entry.scxml:36 :: idle :: _state_body
            }
            is LateDataBindsOnFirstEntryState.S -> {
                // SCE-MAP: late_data_binds_on_first_entry.scxml:40 :: s :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: late_data_binds_on_first_entry.scxml:26 :: _machine
    override fun executeTransitionContent(source: LateDataBindsOnFirstEntryState, transitionIndex: Int) {
        when (source) {
        is LateDataBindsOnFirstEntryState.S -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: late_data_binds_on_first_entry.scxml:50 :: s :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("v", "v"), com.sce.runtime.ScriptSource.lua("5", "5"))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
