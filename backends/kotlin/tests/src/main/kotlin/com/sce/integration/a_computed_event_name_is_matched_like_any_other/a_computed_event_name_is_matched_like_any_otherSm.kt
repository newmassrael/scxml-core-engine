// SCE-GENERATED — DO NOT EDIT
// source-hash: 292d1fc77f53bcad0ebbbb1eba40e4d11668b8d3c883688041c67e32a9df108c

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:38 :: _machine

package com.sce.integration.a_computed_event_name_is_matched_like_any_other

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AComputedEventNameIsMatchedLikeAnyOtherState : State {
    data object Pass : AComputedEventNameIsMatchedLikeAnyOtherState
    data object S1 : AComputedEventNameIsMatchedLikeAnyOtherState
    data object S2 : AComputedEventNameIsMatchedLikeAnyOtherState
    data object S3 : AComputedEventNameIsMatchedLikeAnyOtherState
    data object S4 : AComputedEventNameIsMatchedLikeAnyOtherState
    data object S5 : AComputedEventNameIsMatchedLikeAnyOtherState
    data object S6 : AComputedEventNameIsMatchedLikeAnyOtherState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AComputedEventNameIsMatchedLikeAnyOtherEvent : Event {
    data object Wildcard : AComputedEventNameIsMatchedLikeAnyOtherEvent
    sealed interface Error : AComputedEventNameIsMatchedLikeAnyOtherEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Request : AComputedEventNameIsMatchedLikeAnyOtherEvent
}
// --- State Machine (W3C SCXML) ---

class AComputedEventNameIsMatchedLikeAnyOtherStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<AComputedEventNameIsMatchedLikeAnyOtherState, AComputedEventNameIsMatchedLikeAnyOtherEvent>(scriptEngine) {

    override val initialState: AComputedEventNameIsMatchedLikeAnyOtherState = AComputedEventNameIsMatchedLikeAnyOtherState.S1

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

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: AComputedEventNameIsMatchedLikeAnyOtherState): Boolean = when (state) {
        is AComputedEventNameIsMatchedLikeAnyOtherState.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AComputedEventNameIsMatchedLikeAnyOtherState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AComputedEventNameIsMatchedLikeAnyOtherState, HistoryId>> =
            listOf(StateTarget(AComputedEventNameIsMatchedLikeAnyOtherState.S1))

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<AComputedEventNameIsMatchedLikeAnyOtherState, HistoryId>(
            AComputedEventNameIsMatchedLikeAnyOtherState.S1,
            listOf(StateTarget(AComputedEventNameIsMatchedLikeAnyOtherState.S2)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s2's transition 0, as the microstep reads it.
        val transitionS2At0 = EnabledTransition<AComputedEventNameIsMatchedLikeAnyOtherState, HistoryId>(
            AComputedEventNameIsMatchedLikeAnyOtherState.S2,
            listOf(StateTarget(AComputedEventNameIsMatchedLikeAnyOtherState.S3)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s3's transition 0, as the microstep reads it.
        val transitionS3At0 = EnabledTransition<AComputedEventNameIsMatchedLikeAnyOtherState, HistoryId>(
            AComputedEventNameIsMatchedLikeAnyOtherState.S3,
            listOf(StateTarget(AComputedEventNameIsMatchedLikeAnyOtherState.S4)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s4's transition 0, as the microstep reads it.
        val transitionS4At0 = EnabledTransition<AComputedEventNameIsMatchedLikeAnyOtherState, HistoryId>(
            AComputedEventNameIsMatchedLikeAnyOtherState.S4,
            listOf(StateTarget(AComputedEventNameIsMatchedLikeAnyOtherState.S5)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s5's transition 0, as the microstep reads it.
        val transitionS5At0 = EnabledTransition<AComputedEventNameIsMatchedLikeAnyOtherState, HistoryId>(
            AComputedEventNameIsMatchedLikeAnyOtherState.S5,
            listOf(StateTarget(AComputedEventNameIsMatchedLikeAnyOtherState.S6)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s6's transition 0, as the microstep reads it.
        val transitionS6At0 = EnabledTransition<AComputedEventNameIsMatchedLikeAnyOtherState, HistoryId>(
            AComputedEventNameIsMatchedLikeAnyOtherState.S6,
            listOf(StateTarget(AComputedEventNameIsMatchedLikeAnyOtherState.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AComputedEventNameIsMatchedLikeAnyOtherState? = when (stateId) {
        "pass" -> AComputedEventNameIsMatchedLikeAnyOtherState.Pass
        "s1" -> AComputedEventNameIsMatchedLikeAnyOtherState.S1
        "s2" -> AComputedEventNameIsMatchedLikeAnyOtherState.S2
        "s3" -> AComputedEventNameIsMatchedLikeAnyOtherState.S3
        "s4" -> AComputedEventNameIsMatchedLikeAnyOtherState.S4
        "s5" -> AComputedEventNameIsMatchedLikeAnyOtherState.S5
        "s6" -> AComputedEventNameIsMatchedLikeAnyOtherState.S6
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AComputedEventNameIsMatchedLikeAnyOtherState): String = when (state) {
        is AComputedEventNameIsMatchedLikeAnyOtherState.Pass -> "pass"
        is AComputedEventNameIsMatchedLikeAnyOtherState.S1 -> "s1"
        is AComputedEventNameIsMatchedLikeAnyOtherState.S2 -> "s2"
        is AComputedEventNameIsMatchedLikeAnyOtherState.S3 -> "s3"
        is AComputedEventNameIsMatchedLikeAnyOtherState.S4 -> "s4"
        is AComputedEventNameIsMatchedLikeAnyOtherState.S5 -> "s5"
        is AComputedEventNameIsMatchedLikeAnyOtherState.S6 -> "s6"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AComputedEventNameIsMatchedLikeAnyOtherState): Int = when (state) {
        is AComputedEventNameIsMatchedLikeAnyOtherState.Pass -> 6
        is AComputedEventNameIsMatchedLikeAnyOtherState.S1 -> 0
        is AComputedEventNameIsMatchedLikeAnyOtherState.S2 -> 1
        is AComputedEventNameIsMatchedLikeAnyOtherState.S3 -> 2
        is AComputedEventNameIsMatchedLikeAnyOtherState.S4 -> 3
        is AComputedEventNameIsMatchedLikeAnyOtherState.S5 -> 4
        is AComputedEventNameIsMatchedLikeAnyOtherState.S6 -> 5
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AComputedEventNameIsMatchedLikeAnyOtherEvent? = when (name) {
        "error.communication" -> AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Communication
        "error.execution" -> AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution
        "request" -> AComputedEventNameIsMatchedLikeAnyOtherEvent.Request
        "*" -> AComputedEventNameIsMatchedLikeAnyOtherEvent.Wildcard
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AComputedEventNameIsMatchedLikeAnyOtherEvent): String? = when (event) {
        is AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Communication -> "error.communication"
        is AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution -> "error.execution"
        is AComputedEventNameIsMatchedLikeAnyOtherEvent.Request -> "request"
        is AComputedEventNameIsMatchedLikeAnyOtherEvent.Wildcard -> "*"
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
            "a_computed_event_name_is_matched_like_any_other",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )





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
            raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, reason)
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
            raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AComputedEventNameIsMatchedLikeAnyOtherEvent) {
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
    override fun bindCurrentEvent(event: AComputedEventNameIsMatchedLikeAnyOtherEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AComputedEventNameIsMatchedLikeAnyOtherState,
        event: AComputedEventNameIsMatchedLikeAnyOtherEvent?
    ): EnabledTransition<AComputedEventNameIsMatchedLikeAnyOtherState, HistoryId>? = when (state) {
        is AComputedEventNameIsMatchedLikeAnyOtherState.S1 -> when {
            event is AComputedEventNameIsMatchedLikeAnyOtherEvent.Request && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.name == \"request.new\")", "_event.name === 'request.new'")) -> transitionS1At0
            else -> null
        }
        is AComputedEventNameIsMatchedLikeAnyOtherState.S2 -> when {
            event != null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.name == \"other.thing\")", "_event.name === 'other.thing'")) -> transitionS2At0
            else -> null
        }
        is AComputedEventNameIsMatchedLikeAnyOtherState.S3 -> when {
            event is AComputedEventNameIsMatchedLikeAnyOtherEvent.Request && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.name == \"request.again\")", "_event.name === 'request.again'")) -> transitionS3At0
            else -> null
        }
        is AComputedEventNameIsMatchedLikeAnyOtherState.S4 -> when {
            event != null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.name == \"last.one\")", "_event.name === 'last.one'")) -> transitionS4At0
            else -> null
        }
        is AComputedEventNameIsMatchedLikeAnyOtherState.S5 -> when {
            event is AComputedEventNameIsMatchedLikeAnyOtherEvent.Request && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.name == \"request.third\")", "_event.name === 'request.third'")) -> transitionS5At0
            else -> null
        }
        is AComputedEventNameIsMatchedLikeAnyOtherState.S6 -> when {
            event != null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.name == \"last.two\")", "_event.name === 'last.two'")) -> transitionS6At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:38 :: _machine
    override fun onEntry(state: AComputedEventNameIsMatchedLikeAnyOtherState, isDefaultEntry: Boolean) {
        when (state) {
            is AComputedEventNameIsMatchedLikeAnyOtherState.Pass -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:77 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AComputedEventNameIsMatchedLikeAnyOtherState.S1 -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:41 :: s1 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // An event name that evaluates to nothing names no event: the same
            // failure as one that does not evaluate (test172).
            val sendEventName = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("(\"request.\" .. \"new\")", "'request.' + 'new'")))
            } catch (_: Exception) {
                ""
            }
            if (sendEventName.isEmpty()) {
                raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<send> eventexpr could not be evaluated to an event name", "__send_0")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 3.12.1 + 5.10: a computed name is delivered as the event
            // the document's names resolve it to (its own, the longest token
            // prefix of it the document writes, or its wildcard), and `_event.name`
            // is the whole name when the member is not called that.
            val sendEvent = resolveArrivingEvent(sendEventName)
            // W3C SCXML 6.2: send to this session's external queue
            if (sendEvent != null) send(sendEvent, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData).copy(name = arrivalNameOf(sendEvent, sendEventName)))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is AComputedEventNameIsMatchedLikeAnyOtherState.S2 -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:47 :: s2 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // An event name that evaluates to nothing names no event: the same
            // failure as one that does not evaluate (test172).
            val sendEventName = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("(\"other.\" .. \"thing\")", "'other.' + 'thing'")))
            } catch (_: Exception) {
                ""
            }
            if (sendEventName.isEmpty()) {
                raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<send> eventexpr could not be evaluated to an event name", "__send_1")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 3.12.1 + 5.10: a computed name is delivered as the event
            // the document's names resolve it to (its own, the longest token
            // prefix of it the document writes, or its wildcard), and `_event.name`
            // is the whole name when the member is not called that.
            val sendEvent = resolveArrivingEvent(sendEventName)
            // W3C SCXML 6.2: Delayed send
            if (sendEvent != null) scheduleSend("__send_1", 20L, sendEvent, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData).copy(name = arrivalNameOf(sendEvent, sendEventName)))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is AComputedEventNameIsMatchedLikeAnyOtherState.S3 -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:53 :: s3 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // An event name that evaluates to nothing names no event: the same
            // failure as one that does not evaluate (test172).
            val sendEventName = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("(\"request.\" .. \"again\")", "'request.' + 'again'")))
            } catch (_: Exception) {
                ""
            }
            if (sendEventName.isEmpty()) {
                raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<send> eventexpr could not be evaluated to an event name", "__send_2")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 3.12.1 + 5.10: a computed name is delivered as the event
            // the document's names resolve it to (its own, the longest token
            // prefix of it the document writes, or its wildcard), and `_event.name`
            // is the whole name when the member is not called that.
            val sendEvent = resolveArrivingEvent(sendEventName)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            if (sendEvent != null) raiseInternal(sendEvent, EventMetadata.internal(sendData).copy(name = arrivalNameOf(sendEvent, sendEventName)))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is AComputedEventNameIsMatchedLikeAnyOtherState.S4 -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:59 :: s4 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // An event name that evaluates to nothing names no event: the same
            // failure as one that does not evaluate (test172).
            val sendEventName = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("(\"last.\" .. \"one\")", "'last.' + 'one'")))
            } catch (_: Exception) {
                ""
            }
            if (sendEventName.isEmpty()) {
                raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<send> eventexpr could not be evaluated to an event name", "__send_3")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 3.12.1 + 5.10: a computed name is delivered as the event
            // the document's names resolve it to (its own, the longest token
            // prefix of it the document writes, or its wildcard), and `_event.name`
            // is the whole name when the member is not called that.
            val sendEvent = resolveArrivingEvent(sendEventName)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            // W3C SCXML 6.2.4: a delay postpones the send, it does not change
            // where it goes — the event joins the internal queue when due.
            if (sendEvent != null) scheduleInternalSend("__send_3", 20L, sendEvent, EventMetadata.internal(sendData).copy(name = arrivalNameOf(sendEvent, sendEventName)))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is AComputedEventNameIsMatchedLikeAnyOtherState.S5 -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:65 :: s5 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // An event name that evaluates to nothing names no event: the same
            // failure as one that does not evaluate (test172).
            val sendEventName = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("(\"request.\" .. \"third\")", "'request.' + 'third'")))
            } catch (_: Exception) {
                ""
            }
            if (sendEventName.isEmpty()) {
                raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<send> eventexpr could not be evaluated to an event name", "__send_4")
                return@send true
            }
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("\"#_internal\"", "'#_internal'")))
            } catch (_: Exception) {
                raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_4")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_4")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_4")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 3.12.1 + 5.10: a computed name is delivered as the event
            // the document's names resolve it to (its own, the longest token
            // prefix of it the document writes, or its wildcard), and `_event.name`
            // is the whole name when the member is not called that.
            val sendEvent = resolveArrivingEvent(sendEventName)
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = sendEventName,
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_4",
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
                raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_4")
            }
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
sendEvent,
sendEventName,
                sendData,
0L,
                "__send_4",
                AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_4")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_4")
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
            }
            is AComputedEventNameIsMatchedLikeAnyOtherState.S6 -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:71 :: s6 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // An event name that evaluates to nothing names no event: the same
            // failure as one that does not evaluate (test172).
            val sendEventName = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("(\"last.\" .. \"two\")", "'last.' + 'two'")))
            } catch (_: Exception) {
                ""
            }
            if (sendEventName.isEmpty()) {
                raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<send> eventexpr could not be evaluated to an event name", "__send_5")
                return@send true
            }
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("(\"#_scxml_\" .. _scxml_tostring(_sessionid))", "'#_scxml_' + _sessionid")))
            } catch (_: Exception) {
                raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_5")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_5")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_5")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 3.12.1 + 5.10: a computed name is delivered as the event
            // the document's names resolve it to (its own, the longest token
            // prefix of it the document writes, or its wildcard), and `_event.name`
            // is the whole name when the member is not called that.
            val sendEvent = resolveArrivingEvent(sendEventName)
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = sendEventName,
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_5",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            // W3C SCXML 6.2.4: a `delay` is a property of the SEND, not of the
            // processor it named. The engine performs the act from its
            // scheduler drain at the deadline, including the W3C SCXML 6.2
            // report for an act nobody performed. W3C SCXML 6.3: it lands in
            // the delayed-send queue under the send id, so a `<cancel>`
            // reaches it and the host never sees the act.
            scheduleHostSend("__send_5", 20L, hostRequest)
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
sendEvent,
sendEventName,
                sendData,
20L,
                "__send_5",
                AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_5")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(AComputedEventNameIsMatchedLikeAnyOtherEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_5")
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
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:38 :: _machine
    override fun onExit(state: AComputedEventNameIsMatchedLikeAnyOtherState) {
        when (state) {
            is AComputedEventNameIsMatchedLikeAnyOtherState.Pass -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:77 :: pass :: _state_body
            }
            is AComputedEventNameIsMatchedLikeAnyOtherState.S1 -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:41 :: s1 :: _state_body
            }
            is AComputedEventNameIsMatchedLikeAnyOtherState.S2 -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:47 :: s2 :: _state_body
            }
            is AComputedEventNameIsMatchedLikeAnyOtherState.S3 -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:53 :: s3 :: _state_body
            }
            is AComputedEventNameIsMatchedLikeAnyOtherState.S4 -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:59 :: s4 :: _state_body
            }
            is AComputedEventNameIsMatchedLikeAnyOtherState.S5 -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:65 :: s5 :: _state_body
            }
            is AComputedEventNameIsMatchedLikeAnyOtherState.S6 -> {
                // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:71 :: s6 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_computed_event_name_is_matched_like_any_other.scxml:38 :: _machine
    override fun executeTransitionContent(source: AComputedEventNameIsMatchedLikeAnyOtherState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
