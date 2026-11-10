// SCE-GENERATED — DO NOT EDIT
// source-hash: f57c85a480d19551bf19c9634af42126614c04ca02a56cf3e71a6176ba7b9f85

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_delayed_http_send_is_posted_when_due/a_delayed_http_send_is_posted_when_due.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_delayed_http_send_is_posted_when_due.scxml:39 :: _machine

package com.sce.integration.a_delayed_http_send_is_posted_when_due

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ADelayedHttpSendIsPostedWhenDueState : State {
    data object Done : ADelayedHttpSendIsPostedWhenDueState
    data object Run : ADelayedHttpSendIsPostedWhenDueState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ADelayedHttpSendIsPostedWhenDueEvent : Event {
    data object Dropped : ADelayedHttpSendIsPostedWhenDueEvent
    data object Dynamic : ADelayedHttpSendIsPostedWhenDueEvent
    sealed interface Error : ADelayedHttpSendIsPostedWhenDueEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Later : ADelayedHttpSendIsPostedWhenDueEvent
    data object Now : ADelayedHttpSendIsPostedWhenDueEvent
    data object Settle : ADelayedHttpSendIsPostedWhenDueEvent
    data object Zero : ADelayedHttpSendIsPostedWhenDueEvent
    data object Zeroexpr : ADelayedHttpSendIsPostedWhenDueEvent
}
// --- State Machine (W3C SCXML) ---

class ADelayedHttpSendIsPostedWhenDueStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ADelayedHttpSendIsPostedWhenDueState, ADelayedHttpSendIsPostedWhenDueEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `dynamicUrl` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `dynamicUrl` was assigned a value of another type, or the engine refused.
     */
    fun dynamicUrl(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "dynamicUrl")

    override val initialState: ADelayedHttpSendIsPostedWhenDueState = ADelayedHttpSendIsPostedWhenDueState.Run

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: ADelayedHttpSendIsPostedWhenDueState): Boolean = when (state) {
        is ADelayedHttpSendIsPostedWhenDueState.Done -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ADelayedHttpSendIsPostedWhenDueState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<ADelayedHttpSendIsPostedWhenDueState, HistoryId>> =
            listOf(StateTarget(ADelayedHttpSendIsPostedWhenDueState.Run))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<ADelayedHttpSendIsPostedWhenDueState, HistoryId>(
            ADelayedHttpSendIsPostedWhenDueState.Run,
            listOf(StateTarget(ADelayedHttpSendIsPostedWhenDueState.Done)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ADelayedHttpSendIsPostedWhenDueState? = when (stateId) {
        "done" -> ADelayedHttpSendIsPostedWhenDueState.Done
        "run" -> ADelayedHttpSendIsPostedWhenDueState.Run
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ADelayedHttpSendIsPostedWhenDueState): String = when (state) {
        is ADelayedHttpSendIsPostedWhenDueState.Done -> "done"
        is ADelayedHttpSendIsPostedWhenDueState.Run -> "run"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ADelayedHttpSendIsPostedWhenDueState): Int = when (state) {
        is ADelayedHttpSendIsPostedWhenDueState.Done -> 1
        is ADelayedHttpSendIsPostedWhenDueState.Run -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ADelayedHttpSendIsPostedWhenDueEvent? = when (name) {
        "dropped" -> ADelayedHttpSendIsPostedWhenDueEvent.Dropped
        "dynamic" -> ADelayedHttpSendIsPostedWhenDueEvent.Dynamic
        "error.communication" -> ADelayedHttpSendIsPostedWhenDueEvent.Error.Communication
        "error.execution" -> ADelayedHttpSendIsPostedWhenDueEvent.Error.Execution
        "later" -> ADelayedHttpSendIsPostedWhenDueEvent.Later
        "now" -> ADelayedHttpSendIsPostedWhenDueEvent.Now
        "settle" -> ADelayedHttpSendIsPostedWhenDueEvent.Settle
        "zero" -> ADelayedHttpSendIsPostedWhenDueEvent.Zero
        "zeroexpr" -> ADelayedHttpSendIsPostedWhenDueEvent.Zeroexpr
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ADelayedHttpSendIsPostedWhenDueEvent): String? = when (event) {
        is ADelayedHttpSendIsPostedWhenDueEvent.Dropped -> "dropped"
        is ADelayedHttpSendIsPostedWhenDueEvent.Dynamic -> "dynamic"
        is ADelayedHttpSendIsPostedWhenDueEvent.Error.Communication -> "error.communication"
        is ADelayedHttpSendIsPostedWhenDueEvent.Error.Execution -> "error.execution"
        is ADelayedHttpSendIsPostedWhenDueEvent.Later -> "later"
        is ADelayedHttpSendIsPostedWhenDueEvent.Now -> "now"
        is ADelayedHttpSendIsPostedWhenDueEvent.Settle -> "settle"
        is ADelayedHttpSendIsPostedWhenDueEvent.Zero -> "zero"
        is ADelayedHttpSendIsPostedWhenDueEvent.Zeroexpr -> "zeroexpr"
    }




    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // docs/adr/0096: make this machine's script session, the first step of
    // every start (`initialize`, `start`, `enterAt`, and the start of a child
    // this machine is). The runtime's `makeScriptSession` takes the three steps
    // (create the session, which loads the shared library into it, then bind
    // the system variables), releases what a failed step made and answers the
    // failure, which refuses the start with nothing entered; a later start
    // asks for the session again.
    override fun establishScriptSession(): com.sce.runtime.ScriptSessionFailure? {
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        makeScriptSession(
            engine,
            "a_delayed_http_send_is_posted_when_due",
        )?.let { return it }
        return null
    }

    // W3C SCXML 5.3: initialize the datamodel in the session the start
    // established, before any state is entered: the document's <data>, its
    // states' early-bound <data>, the invoke params the parent handed this
    // session, then the top-level <script>s. The params are applied before the
    // scripts, which read the datamodel the invocation started it with: the
    // order of interpret in Appendix D, and that of every other backend.
    override fun declareDatamodel() {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")

        declareVariablesPart0(engine, sid)


        // W3C SCXML 6.4: Apply pending invoke params from parent, overriding the
        // <data> default just bound, and only for a name this session declares
        // (C++ DatamodelValidationHelper). The parent hands them before this
        // session's start (`setInvokeParams`) and this session applies them, so
        // a child's session is made by the child.
        if (pendingInvokeParams.isNotEmpty()) {
            for ((pName, pValue) in pendingInvokeParams) {
                if (engine.hasVariable(sid, pName)) {
                    try { engine.setVariable(sid, pName, pValue) } catch (_: Exception) {}
                }
            }
            pendingInvokeParams = emptyMap()
        }
    }

    @Suppress("UNUSED_PARAMETER")
    private fun declareVariablesPart0(engine: ScxmlScriptEngine, sid: String) {
        // W3C SCXML 5.3: Initialize variable 'dynamicUrl' with expr
        try {
            val initResult_dynamicUrl = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"http://127.0.0.1:18081/dynamic\"", "'http://127.0.0.1:18081/dynamic'"))
            engine.setVariable(sid, "dynamicUrl", initResult_dynamicUrl)
        } catch (e: Exception) {
            raisePlatformError(ADelayedHttpSendIsPostedWhenDueEvent.Error.Execution, "<data id='dynamicUrl'> expr failed to evaluate")
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
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(ADelayedHttpSendIsPostedWhenDueEvent.Error.Execution, reason)
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
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
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
            raisePlatformError(ADelayedHttpSendIsPostedWhenDueEvent.Error.Execution, "an expression could not be serialised to JSON")
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
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        return try {
            engine.assign(sid, location, expr)
            true
        } catch (e: Exception) {
            raisePlatformError(ADelayedHttpSendIsPostedWhenDueEvent.Error.Execution, "<assign> failed")
            false
        }
    }

    // W3C SCXML 5.8: Script block execution
    // Returns whether the script ran; on failure error.execution is already
    // raised, and the caller ends its block (W3C SCXML 4.9).
    private fun executeScriptBlock(script: com.sce.runtime.ScriptSource): Boolean {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        return try {
            engine.executeScript(sid, script)
            true
        } catch (e: Exception) {
            raisePlatformError(ADelayedHttpSendIsPostedWhenDueEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ADelayedHttpSendIsPostedWhenDueEvent) {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
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
    override fun bindCurrentEvent(event: ADelayedHttpSendIsPostedWhenDueEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: ADelayedHttpSendIsPostedWhenDueState, event: ADelayedHttpSendIsPostedWhenDueEvent?): EnabledTransition<ADelayedHttpSendIsPostedWhenDueState, HistoryId>? = when (state) {
        is ADelayedHttpSendIsPostedWhenDueState.Run -> firstEnabledTransitionOfRun(state, event)
        else -> null
    }


    // run's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRun(state: ADelayedHttpSendIsPostedWhenDueState, event: ADelayedHttpSendIsPostedWhenDueEvent?): EnabledTransition<ADelayedHttpSendIsPostedWhenDueState, HistoryId>? = when {
        event is ADelayedHttpSendIsPostedWhenDueEvent.Settle -> TransitionsPart0.transitionRunAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_delayed_http_send_is_posted_when_due.scxml:39 :: _machine
    override fun onEntry(state: ADelayedHttpSendIsPostedWhenDueState, isDefaultEntry: Boolean) {
        when (state) {
            is ADelayedHttpSendIsPostedWhenDueState.Done -> onEntryOfDone()
            is ADelayedHttpSendIsPostedWhenDueState.Run -> onEntryOfRun()
        }
    }


    private fun onEntryOfDone() {
                // SCE-MAP: a_delayed_http_send_is_posted_when_due.scxml:68 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfRun() {
                // SCE-MAP: a_delayed_http_send_is_posted_when_due.scxml:46 :: run :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML C.2: BasicHTTP send — one arm for a static and a dynamic target
            val httpContent = ""
            performHttpSend("http://127.0.0.1:18081/now", "now", httpContent, sendWireParams, "__send_0")
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML C.2: BasicHTTP send — one arm for a static and a dynamic target
            val httpContent = ""
            scheduleHttpSend(0L, "http://127.0.0.1:18081/zero", "zero", httpContent, sendWireParams, "__send_1")
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            assertScriptSession()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            // The expression failing is the argument error, and so is a value
            // that is not the CSS2 time the clause names (ARCHITECTURE.md,
            // "Durations"): the message is not scheduled under some default wait.
            val sendDelayText = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("\"0ms\"", "'0ms'")))
            } catch (_: Exception) {
                raisePlatformError(ADelayedHttpSendIsPostedWhenDueEvent.Error.Execution, "<send> delayexpr could not be evaluated", "__send_2")
                return@send true
            }
            val sendDelayMs = com.sce.runtime.SendHelper.parseDelayMs(sendDelayText) ?: run {
                raisePlatformError(ADelayedHttpSendIsPostedWhenDueEvent.Error.Execution, "<send> delayexpr is not a CSS2 time", "__send_2")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML C.2: BasicHTTP send — one arm for a static and a dynamic target
            val httpContent = ""
            scheduleHttpSend(sendDelayMs, "http://127.0.0.1:18081/zeroexpr", "zeroexpr", httpContent, sendWireParams, "__send_2")
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML C.2: BasicHTTP send — one arm for a static and a dynamic target
            val httpContent = ""
            scheduleHttpSend(100L, "http://127.0.0.1:18081/later", "later", httpContent, sendWireParams, "later")
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML C.2: BasicHTTP send — one arm for a static and a dynamic target
            val httpContent = ""
            scheduleHttpSend(100L, "http://127.0.0.1:18081/dropped", "dropped", httpContent, sendWireParams, "dropped")
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            cancelSend("dropped")


            if (run send@{
            assertScriptSession()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("dynamicUrl", "dynamicUrl")))
            } catch (_: Exception) {
                raisePlatformError(ADelayedHttpSendIsPostedWhenDueEvent.Error.Execution, "<send> targetexpr could not be evaluated", "dynamic")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ADelayedHttpSendIsPostedWhenDueEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "dynamic")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ADelayedHttpSendIsPostedWhenDueEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "dynamic")
                return@send true
            }
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "k", "v")
            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML C.2: BasicHTTP send — one arm for a static and a dynamic target
            if (!_rt.startsWith("http://") && !_rt.startsWith("https://")) {
                raisePlatformError(ADelayedHttpSendIsPostedWhenDueEvent.Error.Communication, "<send> over BasicHTTPEventProcessor resolved a target that is not an http(s) URL", "dynamic")
                return@send true
            }
            val httpContent = ""
            scheduleHttpSend(200L, _rt, "dynamic", httpContent, sendWireParams, "dynamic")
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_3", 300L, ADelayedHttpSendIsPostedWhenDueEvent.Settle, EventMetadata.sentBy(sessionId, sendId = "__send_3", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_delayed_http_send_is_posted_when_due.scxml:39 :: _machine
    override fun onExit(state: ADelayedHttpSendIsPostedWhenDueState) {
        when (state) {
            is ADelayedHttpSendIsPostedWhenDueState.Done -> onExitOfDone()
            is ADelayedHttpSendIsPostedWhenDueState.Run -> onExitOfRun()
        }
    }


    private fun onExitOfDone() {
                // SCE-MAP: a_delayed_http_send_is_posted_when_due.scxml:68 :: done :: _state_body
    }

    private fun onExitOfRun() {
                // SCE-MAP: a_delayed_http_send_is_posted_when_due.scxml:46 :: run :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_delayed_http_send_is_posted_when_due.scxml:39 :: _machine
    override fun executeTransitionContent(source: ADelayedHttpSendIsPostedWhenDueState, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
