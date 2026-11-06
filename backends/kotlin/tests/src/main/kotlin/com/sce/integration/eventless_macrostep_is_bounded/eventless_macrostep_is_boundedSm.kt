// SCE-GENERATED — DO NOT EDIT
// source-hash: 448efc1945f51a00d346a070a50c9e40a8fdb0d3297033414fa43984fe293f6e

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/eventless_macrostep_is_bounded/eventless_macrostep_is_bounded.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: eventless_macrostep_is_bounded.scxml:53 :: _machine

package com.sce.integration.eventless_macrostep_is_bounded

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface EventlessMacrostepIsBoundedState : State {
    data object BoundedA : EventlessMacrostepIsBoundedState
    data object BoundedB : EventlessMacrostepIsBoundedState
    data object Idle : EventlessMacrostepIsBoundedState
    data object SpinA : EventlessMacrostepIsBoundedState
    data object SpinB : EventlessMacrostepIsBoundedState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface EventlessMacrostepIsBoundedEvent : Event {
    data object Bounded : EventlessMacrostepIsBoundedEvent
    sealed interface Error : EventlessMacrostepIsBoundedEvent {
        data object Execution : Error
    }
    data object Poke : EventlessMacrostepIsBoundedEvent
    data object Reset : EventlessMacrostepIsBoundedEvent
    data object Spin : EventlessMacrostepIsBoundedEvent
}
// --- State Machine (W3C SCXML) ---

class EventlessMacrostepIsBoundedStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<EventlessMacrostepIsBoundedState, EventlessMacrostepIsBoundedEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `pokes` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `pokes` was assigned a value of another type, or the engine refused.
     */
    fun pokes(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "pokes")

    /**
     * §scxml-5.3: what the `laps` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `laps` was assigned a value of another type, or the engine refused.
     */
    fun laps(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "laps")

    /**
     * §scxml-5.3: what the `spins` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `spins` was assigned a value of another type, or the engine refused.
     */
    fun spins(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "spins")

    override val initialState: EventlessMacrostepIsBoundedState = EventlessMacrostepIsBoundedState.Idle

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

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

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<EventlessMacrostepIsBoundedState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<EventlessMacrostepIsBoundedState, HistoryId>> =
            listOf(StateTarget(EventlessMacrostepIsBoundedState.Idle))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: bounded_a's transition 0, as the microstep reads it.
        val transitionBoundedAAt0 = EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>(
            EventlessMacrostepIsBoundedState.BoundedA,
            listOf(StateTarget(EventlessMacrostepIsBoundedState.BoundedB)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: bounded_a's transition 1, as the microstep reads it.
        val transitionBoundedAAt1 = EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>(
            EventlessMacrostepIsBoundedState.BoundedA,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: bounded_b's transition 0, as the microstep reads it.
        val transitionBoundedBAt0 = EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>(
            EventlessMacrostepIsBoundedState.BoundedB,
            listOf(StateTarget(EventlessMacrostepIsBoundedState.BoundedA)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>(
            EventlessMacrostepIsBoundedState.Idle,
            listOf(StateTarget(EventlessMacrostepIsBoundedState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>(
            EventlessMacrostepIsBoundedState.Idle,
            listOf(StateTarget(EventlessMacrostepIsBoundedState.BoundedA)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>(
            EventlessMacrostepIsBoundedState.Idle,
            listOf(StateTarget(EventlessMacrostepIsBoundedState.SpinA)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: spin_a's transition 0, as the microstep reads it.
        val transitionSpinAAt0 = EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>(
            EventlessMacrostepIsBoundedState.SpinA,
            listOf(StateTarget(EventlessMacrostepIsBoundedState.SpinB)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: spin_a's transition 1, as the microstep reads it.
        val transitionSpinAAt1 = EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>(
            EventlessMacrostepIsBoundedState.SpinA,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: spin_a's transition 2, as the microstep reads it.
        val transitionSpinAAt2 = EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>(
            EventlessMacrostepIsBoundedState.SpinA,
            listOf(StateTarget(EventlessMacrostepIsBoundedState.Idle)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: spin_b's transition 0, as the microstep reads it.
        val transitionSpinBAt0 = EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>(
            EventlessMacrostepIsBoundedState.SpinB,
            listOf(StateTarget(EventlessMacrostepIsBoundedState.SpinA)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: spin_b's transition 1, as the microstep reads it.
        val transitionSpinBAt1 = EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>(
            EventlessMacrostepIsBoundedState.SpinB,
            listOf(StateTarget(EventlessMacrostepIsBoundedState.Idle)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): EventlessMacrostepIsBoundedState? = when (stateId) {
        "bounded_a" -> EventlessMacrostepIsBoundedState.BoundedA
        "bounded_b" -> EventlessMacrostepIsBoundedState.BoundedB
        "idle" -> EventlessMacrostepIsBoundedState.Idle
        "spin_a" -> EventlessMacrostepIsBoundedState.SpinA
        "spin_b" -> EventlessMacrostepIsBoundedState.SpinB
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: EventlessMacrostepIsBoundedState): String = when (state) {
        is EventlessMacrostepIsBoundedState.BoundedA -> "bounded_a"
        is EventlessMacrostepIsBoundedState.BoundedB -> "bounded_b"
        is EventlessMacrostepIsBoundedState.Idle -> "idle"
        is EventlessMacrostepIsBoundedState.SpinA -> "spin_a"
        is EventlessMacrostepIsBoundedState.SpinB -> "spin_b"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: EventlessMacrostepIsBoundedState): Int = when (state) {
        is EventlessMacrostepIsBoundedState.BoundedA -> 1
        is EventlessMacrostepIsBoundedState.BoundedB -> 2
        is EventlessMacrostepIsBoundedState.Idle -> 0
        is EventlessMacrostepIsBoundedState.SpinA -> 3
        is EventlessMacrostepIsBoundedState.SpinB -> 4
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): EventlessMacrostepIsBoundedEvent? = when (name) {
        "bounded" -> EventlessMacrostepIsBoundedEvent.Bounded
        "error.execution" -> EventlessMacrostepIsBoundedEvent.Error.Execution
        "poke" -> EventlessMacrostepIsBoundedEvent.Poke
        "reset" -> EventlessMacrostepIsBoundedEvent.Reset
        "spin" -> EventlessMacrostepIsBoundedEvent.Spin
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: EventlessMacrostepIsBoundedEvent): String? = when (event) {
        is EventlessMacrostepIsBoundedEvent.Bounded -> "bounded"
        is EventlessMacrostepIsBoundedEvent.Error.Execution -> "error.execution"
        is EventlessMacrostepIsBoundedEvent.Poke -> "poke"
        is EventlessMacrostepIsBoundedEvent.Reset -> "reset"
        is EventlessMacrostepIsBoundedEvent.Spin -> "spin"
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
            "eventless_macrostep_is_bounded",
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
        // W3C SCXML 5.3: Initialize variable 'pokes' with expr
        try {
            val initResult_pokes = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "pokes", initResult_pokes)
        } catch (e: Exception) {
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, "<data id='pokes'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'laps' with expr
        try {
            val initResult_laps = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "laps", initResult_laps)
        } catch (e: Exception) {
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, "<data id='laps'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'spins' with expr
        try {
            val initResult_spins = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "spins", initResult_spins)
        } catch (e: Exception) {
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, "<data id='spins'> expr failed to evaluate")
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
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, reason)
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
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: EventlessMacrostepIsBoundedEvent) {
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
    override fun bindCurrentEvent(event: EventlessMacrostepIsBoundedEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: EventlessMacrostepIsBoundedState, event: EventlessMacrostepIsBoundedEvent?): EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>? = when (state) {
        is EventlessMacrostepIsBoundedState.BoundedA -> firstEnabledTransitionOfBoundedA(state, event)
        is EventlessMacrostepIsBoundedState.BoundedB -> firstEnabledTransitionOfBoundedB(state, event)
        is EventlessMacrostepIsBoundedState.Idle -> firstEnabledTransitionOfIdle(state, event)
        is EventlessMacrostepIsBoundedState.SpinA -> firstEnabledTransitionOfSpinA(state, event)
        is EventlessMacrostepIsBoundedState.SpinB -> firstEnabledTransitionOfSpinB(state, event)
    }


    // bounded_a's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfBoundedA(state: EventlessMacrostepIsBoundedState, event: EventlessMacrostepIsBoundedEvent?): EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(laps < 500)", "laps < 500")) -> TransitionsPart0.transitionBoundedAAt0
        event is EventlessMacrostepIsBoundedEvent.Poke -> TransitionsPart0.transitionBoundedAAt1
        else -> null
    }


    // bounded_b's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfBoundedB(state: EventlessMacrostepIsBoundedState, event: EventlessMacrostepIsBoundedEvent?): EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>? = when {
        event == null -> TransitionsPart0.transitionBoundedBAt0
        else -> null
    }


    // idle's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdle(state: EventlessMacrostepIsBoundedState, event: EventlessMacrostepIsBoundedEvent?): EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>? = when {
        event is EventlessMacrostepIsBoundedEvent.Poke -> TransitionsPart0.transitionIdleAt0
        event is EventlessMacrostepIsBoundedEvent.Bounded -> TransitionsPart0.transitionIdleAt1
        event is EventlessMacrostepIsBoundedEvent.Spin -> TransitionsPart0.transitionIdleAt2
        else -> null
    }


    // spin_a's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSpinA(state: EventlessMacrostepIsBoundedState, event: EventlessMacrostepIsBoundedEvent?): EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>? = when {
        event == null -> TransitionsPart0.transitionSpinAAt0
        event is EventlessMacrostepIsBoundedEvent.Poke -> TransitionsPart0.transitionSpinAAt1
        event is EventlessMacrostepIsBoundedEvent.Reset -> TransitionsPart0.transitionSpinAAt2
        else -> null
    }


    // spin_b's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSpinB(state: EventlessMacrostepIsBoundedState, event: EventlessMacrostepIsBoundedEvent?): EnabledTransition<EventlessMacrostepIsBoundedState, HistoryId>? = when {
        event == null -> TransitionsPart0.transitionSpinBAt0
        event is EventlessMacrostepIsBoundedEvent.Reset -> TransitionsPart0.transitionSpinBAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: eventless_macrostep_is_bounded.scxml:53 :: _machine
    override fun onEntry(state: EventlessMacrostepIsBoundedState, isDefaultEntry: Boolean) {
        when (state) {
            is EventlessMacrostepIsBoundedState.BoundedA -> onEntryOfBoundedA()
            is EventlessMacrostepIsBoundedState.BoundedB -> onEntryOfBoundedB()
            is EventlessMacrostepIsBoundedState.Idle -> onEntryOfIdle()
            is EventlessMacrostepIsBoundedState.SpinA -> onEntryOfSpinA()
            is EventlessMacrostepIsBoundedState.SpinB -> onEntryOfSpinB()
        }
    }


    private fun onEntryOfBoundedA() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:84 :: bounded_a :: _state_body
    }

    private fun onEntryOfBoundedB() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:95 :: bounded_b :: _state_body
    }

    private fun onEntryOfIdle() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:71 :: idle :: _state_body
    }

    private fun onEntryOfSpinA() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:104 :: spin_a :: _state_body
    }

    private fun onEntryOfSpinB() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:125 :: spin_b :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: eventless_macrostep_is_bounded.scxml:53 :: _machine
    override fun onExit(state: EventlessMacrostepIsBoundedState) {
        when (state) {
            is EventlessMacrostepIsBoundedState.BoundedA -> onExitOfBoundedA()
            is EventlessMacrostepIsBoundedState.BoundedB -> onExitOfBoundedB()
            is EventlessMacrostepIsBoundedState.Idle -> onExitOfIdle()
            is EventlessMacrostepIsBoundedState.SpinA -> onExitOfSpinA()
            is EventlessMacrostepIsBoundedState.SpinB -> onExitOfSpinB()
        }
    }


    private fun onExitOfBoundedA() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:84 :: bounded_a :: _state_body
    }

    private fun onExitOfBoundedB() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:95 :: bounded_b :: _state_body
    }

    private fun onExitOfIdle() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:71 :: idle :: _state_body
    }

    private fun onExitOfSpinA() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:104 :: spin_a :: _state_body
    }

    private fun onExitOfSpinB() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:125 :: spin_b :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: eventless_macrostep_is_bounded.scxml:53 :: _machine
    override fun executeTransitionContent(source: EventlessMacrostepIsBoundedState, transitionIndex: Int) {
        when (source) {
            is EventlessMacrostepIsBoundedState.BoundedA -> transitionContentOfBoundedA(transitionIndex)
            is EventlessMacrostepIsBoundedState.Idle -> transitionContentOfIdle(transitionIndex)
            is EventlessMacrostepIsBoundedState.SpinA -> transitionContentOfSpinA(transitionIndex)
            else -> {}
        }
    }


    // The content of `bounded_a`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfBoundedA(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfBoundedAAt0()
            1 -> transitionContentOfBoundedAAt1()
            else -> {}
        }
    }


    private fun transitionContentOfBoundedAAt0() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:85 :: bounded_a :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("laps", "laps"), com.sce.runtime.ScriptSource.lua("_scxml_add(laps, 1)", "laps + 1"))) {
                return
            }
    }

    private fun transitionContentOfBoundedAAt1() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:90 :: bounded_a :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
    }

    // The content of `idle`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfIdle(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfIdleAt0()
            else -> {}
        }
    }


    private fun transitionContentOfIdleAt0() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:72 :: idle :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
    }

    // The content of `spin_a`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfSpinA(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfSpinAAt0()
            1 -> transitionContentOfSpinAAt1()
            else -> {}
        }
    }


    private fun transitionContentOfSpinAAt0() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:105 :: spin_a :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("spins", "spins"), com.sce.runtime.ScriptSource.lua("_scxml_add(spins, 1)", "spins + 1"))) {
                return
            }
    }

    private fun transitionContentOfSpinAAt1() {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:108 :: spin_a :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
    }
}
