// SCE-GENERATED — DO NOT EDIT
// source-hash: f0b11c032ed1e2694bb49d516788c50005f68b4fc73d25543193ba694c46c41b

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/cancelling_an_invoke_raises_nothing/cancelling_an_invoke_raises_nothing.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: cancelling_an_invoke_raises_nothing.scxml:13 :: _machine

package com.sce.integration.cancelling_an_invoke_raises_nothing

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface CancellingAnInvokeRaisesNothingState : State {
    data object Done : CancellingAnInvokeRaisesNothingState
    data object P : CancellingAnInvokeRaisesNothingState
    data object S2 : CancellingAnInvokeRaisesNothingState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface CancellingAnInvokeRaisesNothingEvent : Event {
    sealed interface Cancel : CancellingAnInvokeRaisesNothingEvent {
        sealed interface Invoke : Cancel {
            data object Self : Invoke
            data object Child : Invoke
        }
    }
    sealed interface Done : CancellingAnInvokeRaisesNothingEvent {
        data object Invoke : Done
    }
    sealed interface Error : CancellingAnInvokeRaisesNothingEvent {
        data object Execution : Error
    }
    data object Finish : CancellingAnInvokeRaisesNothingEvent
    data object Leave : CancellingAnInvokeRaisesNothingEvent
}
// --- State Machine (W3C SCXML) ---

class CancellingAnInvokeRaisesNothingStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<CancellingAnInvokeRaisesNothingState, CancellingAnInvokeRaisesNothingEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `spurious` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `spurious` was assigned a value of another type, or the engine refused.
     */
    fun spurious(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "spurious")

    override val initialState: CancellingAnInvokeRaisesNothingState = CancellingAnInvokeRaisesNothingState.P

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
    override fun isFinalState(state: CancellingAnInvokeRaisesNothingState): Boolean = when (state) {
        is CancellingAnInvokeRaisesNothingState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<CancellingAnInvokeRaisesNothingState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<CancellingAnInvokeRaisesNothingState, HistoryId>> =
            listOf(StateTarget(CancellingAnInvokeRaisesNothingState.P))

        // W3C SCXML 3.13: p's transition 0, as the microstep reads it.
        val transitionPAt0 = EnabledTransition<CancellingAnInvokeRaisesNothingState, HistoryId>(
            CancellingAnInvokeRaisesNothingState.P,
            listOf(StateTarget(CancellingAnInvokeRaisesNothingState.S2)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s2's transition 0, as the microstep reads it.
        val transitionS2At0 = EnabledTransition<CancellingAnInvokeRaisesNothingState, HistoryId>(
            CancellingAnInvokeRaisesNothingState.S2,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s2's transition 1, as the microstep reads it.
        val transitionS2At1 = EnabledTransition<CancellingAnInvokeRaisesNothingState, HistoryId>(
            CancellingAnInvokeRaisesNothingState.S2,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s2's transition 2, as the microstep reads it.
        val transitionS2At2 = EnabledTransition<CancellingAnInvokeRaisesNothingState, HistoryId>(
            CancellingAnInvokeRaisesNothingState.S2,
            listOf(StateTarget(CancellingAnInvokeRaisesNothingState.Done)),
            2,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): CancellingAnInvokeRaisesNothingState? = when (stateId) {
        "done" -> CancellingAnInvokeRaisesNothingState.Done
        "p" -> CancellingAnInvokeRaisesNothingState.P
        "s2" -> CancellingAnInvokeRaisesNothingState.S2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: CancellingAnInvokeRaisesNothingState): String = when (state) {
        is CancellingAnInvokeRaisesNothingState.Done -> "done"
        is CancellingAnInvokeRaisesNothingState.P -> "p"
        is CancellingAnInvokeRaisesNothingState.S2 -> "s2"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: CancellingAnInvokeRaisesNothingState): Int = when (state) {
        is CancellingAnInvokeRaisesNothingState.Done -> 2
        is CancellingAnInvokeRaisesNothingState.P -> 0
        is CancellingAnInvokeRaisesNothingState.S2 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): CancellingAnInvokeRaisesNothingEvent? = when (name) {
        "cancel.invoke" -> CancellingAnInvokeRaisesNothingEvent.Cancel.Invoke.Self
        "cancel.invoke.child" -> CancellingAnInvokeRaisesNothingEvent.Cancel.Invoke.Child
        "done.invoke" -> CancellingAnInvokeRaisesNothingEvent.Done.Invoke
        "error.execution" -> CancellingAnInvokeRaisesNothingEvent.Error.Execution
        "finish" -> CancellingAnInvokeRaisesNothingEvent.Finish
        "leave" -> CancellingAnInvokeRaisesNothingEvent.Leave
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: CancellingAnInvokeRaisesNothingEvent): String? = when (event) {
        is CancellingAnInvokeRaisesNothingEvent.Cancel.Invoke.Self -> "cancel.invoke"
        is CancellingAnInvokeRaisesNothingEvent.Cancel.Invoke.Child -> "cancel.invoke.child"
        is CancellingAnInvokeRaisesNothingEvent.Done.Invoke -> "done.invoke"
        is CancellingAnInvokeRaisesNothingEvent.Error.Execution -> "error.execution"
        is CancellingAnInvokeRaisesNothingEvent.Finish -> "finish"
        is CancellingAnInvokeRaisesNothingEvent.Leave -> "leave"
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
            "cancelling_an_invoke_raises_nothing",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'spurious' with expr
        try {
            val initResult_spurious = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "spurious", initResult_spurious)
        } catch (e: Exception) {
            raisePlatformError(CancellingAnInvokeRaisesNothingEvent.Error.Execution, "<data id='spurious'> expr failed to evaluate")
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
            raisePlatformError(CancellingAnInvokeRaisesNothingEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(CancellingAnInvokeRaisesNothingEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(CancellingAnInvokeRaisesNothingEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(CancellingAnInvokeRaisesNothingEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: CancellingAnInvokeRaisesNothingEvent) {
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
    override fun bindCurrentEvent(event: CancellingAnInvokeRaisesNothingEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: CancellingAnInvokeRaisesNothingState,
        event: CancellingAnInvokeRaisesNothingEvent?
    ): EnabledTransition<CancellingAnInvokeRaisesNothingState, HistoryId>? = when (state) {
        is CancellingAnInvokeRaisesNothingState.P -> when {
            event is CancellingAnInvokeRaisesNothingEvent.Leave -> transitionPAt0
            else -> null
        }
        is CancellingAnInvokeRaisesNothingState.S2 -> when {
            (event is CancellingAnInvokeRaisesNothingEvent.Cancel.Invoke || event is CancellingAnInvokeRaisesNothingEvent.Cancel.Invoke.Child) -> transitionS2At0
            event is CancellingAnInvokeRaisesNothingEvent.Cancel.Invoke.Child -> transitionS2At1
            event is CancellingAnInvokeRaisesNothingEvent.Finish -> transitionS2At2
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: cancelling_an_invoke_raises_nothing.scxml:13 :: _machine
    override fun onEntry(state: CancellingAnInvokeRaisesNothingState, isDefaultEntry: Boolean) {
        when (state) {
            is CancellingAnInvokeRaisesNothingState.Done -> {
                // SCE-MAP: cancelling_an_invoke_raises_nothing.scxml:42 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is CancellingAnInvokeRaisesNothingState.P -> {
                // SCE-MAP: cancelling_an_invoke_raises_nothing.scxml:21 :: p :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "p.${System.identityHashCode(this)}.child"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = CancellingAnInvokeRaisesNothingSceSynthInvokeChildStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("child", childSM, false, CancellingAnInvokeRaisesNothingEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is CancellingAnInvokeRaisesNothingState.S2 -> {
                // SCE-MAP: cancelling_an_invoke_raises_nothing.scxml:32 :: s2 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: cancelling_an_invoke_raises_nothing.scxml:13 :: _machine
    override fun onExit(state: CancellingAnInvokeRaisesNothingState) {
        when (state) {
            is CancellingAnInvokeRaisesNothingState.Done -> {
                // SCE-MAP: cancelling_an_invoke_raises_nothing.scxml:42 :: done :: _state_body
            }
            is CancellingAnInvokeRaisesNothingState.P -> {
                // SCE-MAP: cancelling_an_invoke_raises_nothing.scxml:21 :: p :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("child")
            }
            is CancellingAnInvokeRaisesNothingState.S2 -> {
                // SCE-MAP: cancelling_an_invoke_raises_nothing.scxml:32 :: s2 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: cancelling_an_invoke_raises_nothing.scxml:13 :: _machine
    override fun executeTransitionContent(source: CancellingAnInvokeRaisesNothingState, transitionIndex: Int) {
        when (source) {
        is CancellingAnInvokeRaisesNothingState.S2 -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: cancelling_an_invoke_raises_nothing.scxml:33 :: s2 :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("spurious", "spurious"), com.sce.runtime.ScriptSource.lua("_scxml_add(spurious, 1)", "spurious + 1"))
            }
            1 -> {
                // SCE-MAP: cancelling_an_invoke_raises_nothing.scxml:36 :: s2 :: _transition_1


            executeAssign(com.sce.runtime.ScriptSource.lua("spurious", "spurious"), com.sce.runtime.ScriptSource.lua("_scxml_add(spurious, 1)", "spurious + 1"))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
