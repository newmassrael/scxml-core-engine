// SCE-GENERATED — DO NOT EDIT
// source-hash: 3d00f3fb685d7db5391eb1dd1a16f454a494f9ac5306212c01c169f328172a40
// template-hash: abb4cb0927b4f829905a6959dcd61b1d31691dba43790340291744aa63315db6
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/wildcard_in_document_order/wildcard_in_document_order.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: wildcard_in_document_order.scxml:48 :: _machine

package com.sce.integration.wildcard_in_document_order

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface WildcardInDocumentOrderState : State {
    data object FailGuardedInternalReentered : WildcardInDocumentOrderState
    data object FailGuardIgnored : WildcardInDocumentOrderState
    data object FailGuardNeverFired : WildcardInDocumentOrderState
    data object FailSealedInternalReentered : WildcardInDocumentOrderState
    data object GuardClosed : WildcardInDocumentOrderState
    data object GuardClosedLeaf : WildcardInDocumentOrderState
    data object GuardedFrom : WildcardInDocumentOrderState
    data object GuardedInternal : WildcardInDocumentOrderState
    data object GuardedTo : WildcardInDocumentOrderState
    data object GuardOpen : WildcardInDocumentOrderState
    data object GuardOpenLeaf : WildcardInDocumentOrderState
    data object Pass : WildcardInDocumentOrderState
    data object SealedFrom : WildcardInDocumentOrderState
    data object SealedInternal : WildcardInDocumentOrderState
    data object SealedTo : WildcardInDocumentOrderState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface WildcardInDocumentOrderEvent : Event {
    sealed interface Error : WildcardInDocumentOrderEvent {
        data object Execution : Error
    }
    data object Hop : WildcardInDocumentOrderEvent
    data object Probe : WildcardInDocumentOrderEvent
}
// --- State Machine (W3C SCXML) ---

class WildcardInDocumentOrderStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<WildcardInDocumentOrderState, WildcardInDocumentOrderEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `armed` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `armed` was assigned a value of another type, or the engine refused.
     */
    fun armed(): Boolean? =
        com.sce.runtime.DatamodelRead.readBool(scriptEngine, scriptSessionId, "armed")

    /**
     * §scxml-5.3: what the `guardedEntries` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `guardedEntries` was assigned a value of another type, or the engine refused.
     */
    fun guardedEntries(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "guardedEntries")

    /**
     * §scxml-5.3: what the `sealedEntries` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `sealedEntries` was assigned a value of another type, or the engine refused.
     */
    fun sealedEntries(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "sealedEntries")

    override val initialState: WildcardInDocumentOrderState = WildcardInDocumentOrderState.GuardClosedLeaf

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: WildcardInDocumentOrderState): WildcardInDocumentOrderState? = when (state) {
        is WildcardInDocumentOrderState.GuardClosedLeaf -> WildcardInDocumentOrderState.GuardClosed
        is WildcardInDocumentOrderState.GuardedFrom -> WildcardInDocumentOrderState.GuardedInternal
        is WildcardInDocumentOrderState.GuardedTo -> WildcardInDocumentOrderState.GuardedInternal
        is WildcardInDocumentOrderState.GuardOpenLeaf -> WildcardInDocumentOrderState.GuardOpen
        is WildcardInDocumentOrderState.SealedFrom -> WildcardInDocumentOrderState.SealedInternal
        is WildcardInDocumentOrderState.SealedTo -> WildcardInDocumentOrderState.SealedInternal
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: WildcardInDocumentOrderState): WildcardInDocumentOrderState = when (state) {
        is WildcardInDocumentOrderState.GuardClosed -> WildcardInDocumentOrderState.GuardClosedLeaf
        is WildcardInDocumentOrderState.GuardedInternal -> WildcardInDocumentOrderState.GuardedFrom
        is WildcardInDocumentOrderState.GuardOpen -> WildcardInDocumentOrderState.GuardOpenLeaf
        is WildcardInDocumentOrderState.SealedInternal -> WildcardInDocumentOrderState.SealedFrom
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): WildcardInDocumentOrderState? = when (stateId) {
        "failGuardedInternalReentered" -> WildcardInDocumentOrderState.FailGuardedInternalReentered
        "failGuardIgnored" -> WildcardInDocumentOrderState.FailGuardIgnored
        "failGuardNeverFired" -> WildcardInDocumentOrderState.FailGuardNeverFired
        "failSealedInternalReentered" -> WildcardInDocumentOrderState.FailSealedInternalReentered
        "guardClosed" -> WildcardInDocumentOrderState.GuardClosed
        "guardClosedLeaf" -> WildcardInDocumentOrderState.GuardClosedLeaf
        "guardedFrom" -> WildcardInDocumentOrderState.GuardedFrom
        "guardedInternal" -> WildcardInDocumentOrderState.GuardedInternal
        "guardedTo" -> WildcardInDocumentOrderState.GuardedTo
        "guardOpen" -> WildcardInDocumentOrderState.GuardOpen
        "guardOpenLeaf" -> WildcardInDocumentOrderState.GuardOpenLeaf
        "pass" -> WildcardInDocumentOrderState.Pass
        "sealedFrom" -> WildcardInDocumentOrderState.SealedFrom
        "sealedInternal" -> WildcardInDocumentOrderState.SealedInternal
        "sealedTo" -> WildcardInDocumentOrderState.SealedTo
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: WildcardInDocumentOrderState): String = when (state) {
        is WildcardInDocumentOrderState.FailGuardedInternalReentered -> "failGuardedInternalReentered"
        is WildcardInDocumentOrderState.FailGuardIgnored -> "failGuardIgnored"
        is WildcardInDocumentOrderState.FailGuardNeverFired -> "failGuardNeverFired"
        is WildcardInDocumentOrderState.FailSealedInternalReentered -> "failSealedInternalReentered"
        is WildcardInDocumentOrderState.GuardClosed -> "guardClosed"
        is WildcardInDocumentOrderState.GuardClosedLeaf -> "guardClosedLeaf"
        is WildcardInDocumentOrderState.GuardedFrom -> "guardedFrom"
        is WildcardInDocumentOrderState.GuardedInternal -> "guardedInternal"
        is WildcardInDocumentOrderState.GuardedTo -> "guardedTo"
        is WildcardInDocumentOrderState.GuardOpen -> "guardOpen"
        is WildcardInDocumentOrderState.GuardOpenLeaf -> "guardOpenLeaf"
        is WildcardInDocumentOrderState.Pass -> "pass"
        is WildcardInDocumentOrderState.SealedFrom -> "sealedFrom"
        is WildcardInDocumentOrderState.SealedInternal -> "sealedInternal"
        is WildcardInDocumentOrderState.SealedTo -> "sealedTo"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: WildcardInDocumentOrderState): Boolean = when (state) {
        is WildcardInDocumentOrderState.GuardClosed -> false
        is WildcardInDocumentOrderState.GuardedInternal -> false
        is WildcardInDocumentOrderState.GuardOpen -> false
        is WildcardInDocumentOrderState.SealedInternal -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: WildcardInDocumentOrderState): Int = when (state) {
        is WildcardInDocumentOrderState.FailGuardedInternalReentered -> 13
        is WildcardInDocumentOrderState.FailGuardIgnored -> 11
        is WildcardInDocumentOrderState.FailGuardNeverFired -> 12
        is WildcardInDocumentOrderState.FailSealedInternalReentered -> 14
        is WildcardInDocumentOrderState.GuardClosed -> 0
        is WildcardInDocumentOrderState.GuardClosedLeaf -> 1
        is WildcardInDocumentOrderState.GuardedFrom -> 5
        is WildcardInDocumentOrderState.GuardedInternal -> 4
        is WildcardInDocumentOrderState.GuardedTo -> 6
        is WildcardInDocumentOrderState.GuardOpen -> 2
        is WildcardInDocumentOrderState.GuardOpenLeaf -> 3
        is WildcardInDocumentOrderState.Pass -> 10
        is WildcardInDocumentOrderState.SealedFrom -> 8
        is WildcardInDocumentOrderState.SealedInternal -> 7
        is WildcardInDocumentOrderState.SealedTo -> 9
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): WildcardInDocumentOrderEvent? = when (name) {
        "error.execution" -> WildcardInDocumentOrderEvent.Error.Execution
        "hop" -> WildcardInDocumentOrderEvent.Hop
        "probe" -> WildcardInDocumentOrderEvent.Probe
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: WildcardInDocumentOrderEvent): String? = when (event) {
        is WildcardInDocumentOrderEvent.Error.Execution -> "error.execution"
        is WildcardInDocumentOrderEvent.Hop -> "hop"
        is WildcardInDocumentOrderEvent.Probe -> "probe"
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
            "wildcard_in_document_order",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'armed' with expr
        try {
            val initResult_armed = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("false", "false"))
            engine.setVariable(sid, "armed", initResult_armed)
        } catch (e: Exception) {
            raisePlatformError(WildcardInDocumentOrderEvent.Error.Execution, "<data id='armed'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'guardedEntries' with expr
        try {
            val initResult_guardedEntries = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "guardedEntries", initResult_guardedEntries)
        } catch (e: Exception) {
            raisePlatformError(WildcardInDocumentOrderEvent.Error.Execution, "<data id='guardedEntries'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'sealedEntries' with expr
        try {
            val initResult_sealedEntries = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "sealedEntries", initResult_sealedEntries)
        } catch (e: Exception) {
            raisePlatformError(WildcardInDocumentOrderEvent.Error.Execution, "<data id='sealedEntries'> expr failed to evaluate")
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
            raisePlatformError(WildcardInDocumentOrderEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(WildcardInDocumentOrderEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(WildcardInDocumentOrderEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(WildcardInDocumentOrderEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: WildcardInDocumentOrderEvent) {
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
        state: WildcardInDocumentOrderState,
        event: WildcardInDocumentOrderEvent
    ): TransitionResult<WildcardInDocumentOrderState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is WildcardInDocumentOrderState.GuardClosed -> processGuardClosed(event)
        is WildcardInDocumentOrderState.GuardClosedLeaf -> {
            val result = processGuardClosedLeaf(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processGuardClosed(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (guardedFrom has no own event transitions)
        is WildcardInDocumentOrderState.GuardedFrom -> {
            val anc1 = processGuardedInternal(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is WildcardInDocumentOrderState.GuardedInternal -> processGuardedInternal(event)
        // W3C SCXML 3.13: Ancestor-only routing (guardedTo has no own event transitions)
        is WildcardInDocumentOrderState.GuardedTo -> {
            val anc1 = processGuardedInternal(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is WildcardInDocumentOrderState.GuardOpen -> processGuardOpen(event)
        is WildcardInDocumentOrderState.GuardOpenLeaf -> {
            val result = processGuardOpenLeaf(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processGuardOpen(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (sealedFrom has no own event transitions)
        is WildcardInDocumentOrderState.SealedFrom -> {
            val anc1 = processSealedInternal(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is WildcardInDocumentOrderState.SealedInternal -> processSealedInternal(event)
        // W3C SCXML 3.13: Ancestor-only routing (sealedTo has no own event transitions)
        is WildcardInDocumentOrderState.SealedTo -> {
            val anc1 = processSealedInternal(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: WildcardInDocumentOrderState
    ): TransitionResult<WildcardInDocumentOrderState> = when (state) {
        is WildcardInDocumentOrderState.GuardedTo -> processNullGuardedTo()
        is WildcardInDocumentOrderState.SealedTo -> processNullSealedTo()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullGuardedTo(
    ): TransitionResult<WildcardInDocumentOrderState> = when {
        safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(guardedEntries, 1)", "guardedEntries == 1")) -> TransitionResult.External(WildcardInDocumentOrderState.SealedInternal, WildcardInDocumentOrderState.GuardedTo, 5)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(WildcardInDocumentOrderState.FailGuardedInternalReentered, WildcardInDocumentOrderState.GuardedTo, 6)
    }

    private fun processNullSealedTo(
    ): TransitionResult<WildcardInDocumentOrderState> = when {
        safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(sealedEntries, 1)", "sealedEntries == 1")) -> TransitionResult.External(WildcardInDocumentOrderState.Pass, WildcardInDocumentOrderState.SealedTo, 8)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(WildcardInDocumentOrderState.FailSealedInternalReentered, WildcardInDocumentOrderState.SealedTo, 9)
    }

    // --- Per-State Event Handlers ---

    private fun processGuardClosed(
        event: WildcardInDocumentOrderEvent
    ): TransitionResult<WildcardInDocumentOrderState> = when {
        event is WildcardInDocumentOrderEvent.Probe -> TransitionResult.External(WildcardInDocumentOrderState.GuardOpen, WildcardInDocumentOrderState.GuardClosed, 0)

        else -> TransitionResult.Ignored
    }

    private fun processGuardClosedLeaf(
        event: WildcardInDocumentOrderEvent
    ): TransitionResult<WildcardInDocumentOrderState> = when {
        // W3C SCXML 3.12.1: guarded wildcard, in the place it was written
        safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_truthy(armed)", "armed")) -> TransitionResult.External(WildcardInDocumentOrderState.FailGuardIgnored, WildcardInDocumentOrderState.GuardClosedLeaf, 1)

        else -> TransitionResult.Ignored
    }

    private fun processGuardedInternal(
        event: WildcardInDocumentOrderEvent
    ): TransitionResult<WildcardInDocumentOrderState> = when {
        // W3C SCXML 3.12.1: guarded wildcard, in the place it was written
        safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_truthy(armed)", "armed")) -> TransitionResult.InternalToTarget(WildcardInDocumentOrderState.GuardedTo, WildcardInDocumentOrderState.GuardedInternal, 4)

        else -> TransitionResult.Ignored
    }

    private fun processGuardOpen(
        event: WildcardInDocumentOrderEvent
    ): TransitionResult<WildcardInDocumentOrderState> = when {
        event is WildcardInDocumentOrderEvent.Probe -> TransitionResult.External(WildcardInDocumentOrderState.FailGuardNeverFired, WildcardInDocumentOrderState.GuardOpen, 2)

        else -> TransitionResult.Ignored
    }

    private fun processGuardOpenLeaf(
        event: WildcardInDocumentOrderEvent
    ): TransitionResult<WildcardInDocumentOrderState> = when {
        // W3C SCXML 3.12.1: guarded wildcard, in the place it was written
        safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_truthy(armed)", "armed")) -> TransitionResult.External(WildcardInDocumentOrderState.GuardedInternal, WildcardInDocumentOrderState.GuardOpenLeaf, 3)

        else -> TransitionResult.Ignored
    }

    private fun processSealedInternal(
        event: WildcardInDocumentOrderEvent
    ): TransitionResult<WildcardInDocumentOrderState> = when {
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.InternalToTarget(WildcardInDocumentOrderState.SealedTo, WildcardInDocumentOrderState.SealedInternal, 7)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: wildcard_in_document_order.scxml:48 :: _machine
    override fun onEntry(state: WildcardInDocumentOrderState, pathChild: WildcardInDocumentOrderState?) {
        when (state) {
            is WildcardInDocumentOrderState.FailGuardedInternalReentered -> {
                // SCE-MAP: wildcard_in_document_order.scxml:109 :: failGuardedInternalReentered :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failGuardedInternalReentered")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is WildcardInDocumentOrderState.FailGuardIgnored -> {
                // SCE-MAP: wildcard_in_document_order.scxml:107 :: failGuardIgnored :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failGuardIgnored")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is WildcardInDocumentOrderState.FailGuardNeverFired -> {
                // SCE-MAP: wildcard_in_document_order.scxml:108 :: failGuardNeverFired :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failGuardNeverFired")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is WildcardInDocumentOrderState.FailSealedInternalReentered -> {
                // SCE-MAP: wildcard_in_document_order.scxml:110 :: failSealedInternalReentered :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failSealedInternalReentered")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is WildcardInDocumentOrderState.GuardClosed -> {
                // SCE-MAP: wildcard_in_document_order.scxml:58 :: guardClosed :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("guardClosed")) return

            raiseInternal(WildcardInDocumentOrderEvent.Probe)
            }
            is WildcardInDocumentOrderState.GuardClosedLeaf -> {
                // SCE-MAP: wildcard_in_document_order.scxml:65 :: guardClosedLeaf :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("guardClosedLeaf")) return
            }
            is WildcardInDocumentOrderState.GuardedFrom -> {
                // SCE-MAP: wildcard_in_document_order.scxml:86 :: guardedFrom :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("guardedFrom")) return
            }
            is WildcardInDocumentOrderState.GuardedInternal -> {
                // SCE-MAP: wildcard_in_document_order.scxml:80 :: guardedInternal :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("guardedInternal")) return


            executeAssign(com.sce.runtime.ScriptSource.lua("guardedEntries", "guardedEntries"), com.sce.runtime.ScriptSource.lua("_scxml_add(guardedEntries, 1)", "guardedEntries + 1"))

            raiseInternal(WildcardInDocumentOrderEvent.Hop)
            }
            is WildcardInDocumentOrderState.GuardedTo -> {
                // SCE-MAP: wildcard_in_document_order.scxml:87 :: guardedTo :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("guardedTo")) return
            }
            is WildcardInDocumentOrderState.GuardOpen -> {
                // SCE-MAP: wildcard_in_document_order.scxml:70 :: guardOpen :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("guardOpen")) return

            raiseInternal(WildcardInDocumentOrderEvent.Probe)
            }
            is WildcardInDocumentOrderState.GuardOpenLeaf -> {
                // SCE-MAP: wildcard_in_document_order.scxml:75 :: guardOpenLeaf :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("guardOpenLeaf")) return
            }
            is WildcardInDocumentOrderState.Pass -> {
                // SCE-MAP: wildcard_in_document_order.scxml:106 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is WildcardInDocumentOrderState.SealedFrom -> {
                // SCE-MAP: wildcard_in_document_order.scxml:99 :: sealedFrom :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sealedFrom")) return
            }
            is WildcardInDocumentOrderState.SealedInternal -> {
                // SCE-MAP: wildcard_in_document_order.scxml:93 :: sealedInternal :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sealedInternal")) return


            executeAssign(com.sce.runtime.ScriptSource.lua("sealedEntries", "sealedEntries"), com.sce.runtime.ScriptSource.lua("_scxml_add(sealedEntries, 1)", "sealedEntries + 1"))

            raiseInternal(WildcardInDocumentOrderEvent.Hop)
            }
            is WildcardInDocumentOrderState.SealedTo -> {
                // SCE-MAP: wildcard_in_document_order.scxml:100 :: sealedTo :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sealedTo")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: wildcard_in_document_order.scxml:48 :: _machine
    override fun onExit(state: WildcardInDocumentOrderState) {
        when (state) {
            is WildcardInDocumentOrderState.FailGuardedInternalReentered -> {
                // SCE-MAP: wildcard_in_document_order.scxml:109 :: failGuardedInternalReentered :: _state_body
                activeStateIds.remove("failGuardedInternalReentered")
            }
            is WildcardInDocumentOrderState.FailGuardIgnored -> {
                // SCE-MAP: wildcard_in_document_order.scxml:107 :: failGuardIgnored :: _state_body
                activeStateIds.remove("failGuardIgnored")
            }
            is WildcardInDocumentOrderState.FailGuardNeverFired -> {
                // SCE-MAP: wildcard_in_document_order.scxml:108 :: failGuardNeverFired :: _state_body
                activeStateIds.remove("failGuardNeverFired")
            }
            is WildcardInDocumentOrderState.FailSealedInternalReentered -> {
                // SCE-MAP: wildcard_in_document_order.scxml:110 :: failSealedInternalReentered :: _state_body
                activeStateIds.remove("failSealedInternalReentered")
            }
            is WildcardInDocumentOrderState.GuardClosed -> {
                // SCE-MAP: wildcard_in_document_order.scxml:58 :: guardClosed :: _state_body
                activeStateIds.remove("guardClosed")
            }
            is WildcardInDocumentOrderState.GuardClosedLeaf -> {
                // SCE-MAP: wildcard_in_document_order.scxml:65 :: guardClosedLeaf :: _state_body
                activeStateIds.remove("guardClosedLeaf")
            }
            is WildcardInDocumentOrderState.GuardedFrom -> {
                // SCE-MAP: wildcard_in_document_order.scxml:86 :: guardedFrom :: _state_body
                activeStateIds.remove("guardedFrom")
            }
            is WildcardInDocumentOrderState.GuardedInternal -> {
                // SCE-MAP: wildcard_in_document_order.scxml:80 :: guardedInternal :: _state_body
                activeStateIds.remove("guardedInternal")
            }
            is WildcardInDocumentOrderState.GuardedTo -> {
                // SCE-MAP: wildcard_in_document_order.scxml:87 :: guardedTo :: _state_body
                activeStateIds.remove("guardedTo")
            }
            is WildcardInDocumentOrderState.GuardOpen -> {
                // SCE-MAP: wildcard_in_document_order.scxml:70 :: guardOpen :: _state_body
                activeStateIds.remove("guardOpen")
            }
            is WildcardInDocumentOrderState.GuardOpenLeaf -> {
                // SCE-MAP: wildcard_in_document_order.scxml:75 :: guardOpenLeaf :: _state_body
                activeStateIds.remove("guardOpenLeaf")
            }
            is WildcardInDocumentOrderState.Pass -> {
                // SCE-MAP: wildcard_in_document_order.scxml:106 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is WildcardInDocumentOrderState.SealedFrom -> {
                // SCE-MAP: wildcard_in_document_order.scxml:99 :: sealedFrom :: _state_body
                activeStateIds.remove("sealedFrom")
            }
            is WildcardInDocumentOrderState.SealedInternal -> {
                // SCE-MAP: wildcard_in_document_order.scxml:93 :: sealedInternal :: _state_body
                activeStateIds.remove("sealedInternal")
            }
            is WildcardInDocumentOrderState.SealedTo -> {
                // SCE-MAP: wildcard_in_document_order.scxml:100 :: sealedTo :: _state_body
                activeStateIds.remove("sealedTo")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: wildcard_in_document_order.scxml:48 :: _machine
    override fun executeTransitionActions(
        source: WildcardInDocumentOrderState,
        event: WildcardInDocumentOrderEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        is WildcardInDocumentOrderState.GuardClosed -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: wildcard_in_document_order.scxml:62 :: guardClosed :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("armed", "armed"), com.sce.runtime.ScriptSource.lua("true", "true"))
            }
            else -> {}
        }
        is WildcardInDocumentOrderState.GuardClosedLeaf -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: wildcard_in_document_order.scxml:62 :: guardClosed :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("armed", "armed"), com.sce.runtime.ScriptSource.lua("true", "true"))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
