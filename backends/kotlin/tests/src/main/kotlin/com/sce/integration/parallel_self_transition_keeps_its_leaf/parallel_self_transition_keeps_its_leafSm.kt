// SCE-GENERATED — DO NOT EDIT
// source-hash: 09b7454cf9165bde8e92b3225905fad8bae3b40d103c1bd2ce5da264bfe36345

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/parallel_self_transition_keeps_its_leaf/parallel_self_transition_keeps_its_leaf.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:53 :: _machine

package com.sce.integration.parallel_self_transition_keeps_its_leaf

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ParallelSelfTransitionKeepsItsLeafState : State {
    data object Budget : ParallelSelfTransitionKeepsItsLeafState
    data object Drive : ParallelSelfTransitionKeepsItsLeafState
    data object Judging : ParallelSelfTransitionKeepsItsLeafState
    data object Run : ParallelSelfTransitionKeepsItsLeafState
    data object Running : ParallelSelfTransitionKeepsItsLeafState
    data object Settled : ParallelSelfTransitionKeepsItsLeafState
    data object Within : ParallelSelfTransitionKeepsItsLeafState
    data object Working : ParallelSelfTransitionKeepsItsLeafState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ParallelSelfTransitionKeepsItsLeafEvent : Event {
    data object Check : ParallelSelfTransitionKeepsItsLeafEvent
    data object E : ParallelSelfTransitionKeepsItsLeafEvent
    sealed interface Error : ParallelSelfTransitionKeepsItsLeafEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class ParallelSelfTransitionKeepsItsLeafStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ParallelSelfTransitionKeepsItsLeafState, ParallelSelfTransitionKeepsItsLeafEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `n` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `n` was assigned a value of another type, or the engine refused.
     */
    fun n(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "n")

    /**
     * §scxml-5.3: what the `m` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `m` was assigned a value of another type, or the engine refused.
     */
    fun m(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "m")

    override val initialState: ParallelSelfTransitionKeepsItsLeafState = ParallelSelfTransitionKeepsItsLeafState.Within

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

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: ParallelSelfTransitionKeepsItsLeafState): ParallelSelfTransitionKeepsItsLeafState? = when (state) {
        is ParallelSelfTransitionKeepsItsLeafState.Budget -> ParallelSelfTransitionKeepsItsLeafState.Run
        is ParallelSelfTransitionKeepsItsLeafState.Drive -> ParallelSelfTransitionKeepsItsLeafState.Run
        is ParallelSelfTransitionKeepsItsLeafState.Judging -> ParallelSelfTransitionKeepsItsLeafState.Running
        is ParallelSelfTransitionKeepsItsLeafState.Running -> ParallelSelfTransitionKeepsItsLeafState.Drive
        is ParallelSelfTransitionKeepsItsLeafState.Within -> ParallelSelfTransitionKeepsItsLeafState.Budget
        is ParallelSelfTransitionKeepsItsLeafState.Working -> ParallelSelfTransitionKeepsItsLeafState.Running
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: ParallelSelfTransitionKeepsItsLeafState): Boolean = when (state) {
        is ParallelSelfTransitionKeepsItsLeafState.Budget -> true
        is ParallelSelfTransitionKeepsItsLeafState.Drive -> true
        is ParallelSelfTransitionKeepsItsLeafState.Running -> true
        else -> false
    }


    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: ParallelSelfTransitionKeepsItsLeafState): Boolean = when (state) {
        is ParallelSelfTransitionKeepsItsLeafState.Run -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: ParallelSelfTransitionKeepsItsLeafState): Boolean = when (state) {
        is ParallelSelfTransitionKeepsItsLeafState.Settled -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: ParallelSelfTransitionKeepsItsLeafState): List<ParallelSelfTransitionKeepsItsLeafState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: ParallelSelfTransitionKeepsItsLeafState): List<EntryTarget<ParallelSelfTransitionKeepsItsLeafState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ParallelSelfTransitionKeepsItsLeafState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<ParallelSelfTransitionKeepsItsLeafState, List<ParallelSelfTransitionKeepsItsLeafState>> = LinkedHashMap<ParallelSelfTransitionKeepsItsLeafState, List<ParallelSelfTransitionKeepsItsLeafState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<ParallelSelfTransitionKeepsItsLeafState, List<ParallelSelfTransitionKeepsItsLeafState>>) {
            table[ParallelSelfTransitionKeepsItsLeafState.Budget] = listOf(ParallelSelfTransitionKeepsItsLeafState.Within)
            table[ParallelSelfTransitionKeepsItsLeafState.Drive] = listOf(ParallelSelfTransitionKeepsItsLeafState.Running)
            table[ParallelSelfTransitionKeepsItsLeafState.Run] = listOf(ParallelSelfTransitionKeepsItsLeafState.Budget, ParallelSelfTransitionKeepsItsLeafState.Drive)
            table[ParallelSelfTransitionKeepsItsLeafState.Running] = listOf(ParallelSelfTransitionKeepsItsLeafState.Working, ParallelSelfTransitionKeepsItsLeafState.Judging)
        }


        val initialTargets: Map<ParallelSelfTransitionKeepsItsLeafState, List<EntryTarget<ParallelSelfTransitionKeepsItsLeafState, HistoryId>>> = LinkedHashMap<ParallelSelfTransitionKeepsItsLeafState, List<EntryTarget<ParallelSelfTransitionKeepsItsLeafState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<ParallelSelfTransitionKeepsItsLeafState, List<EntryTarget<ParallelSelfTransitionKeepsItsLeafState, HistoryId>>>) {
            table[ParallelSelfTransitionKeepsItsLeafState.Budget] = listOf(StateTarget(ParallelSelfTransitionKeepsItsLeafState.Within))
            table[ParallelSelfTransitionKeepsItsLeafState.Drive] = listOf(StateTarget(ParallelSelfTransitionKeepsItsLeafState.Running))
            table[ParallelSelfTransitionKeepsItsLeafState.Running] = listOf(StateTarget(ParallelSelfTransitionKeepsItsLeafState.Working))
        }


        val documentInitialTargetList: List<EntryTarget<ParallelSelfTransitionKeepsItsLeafState, HistoryId>> =
            listOf(StateTarget(ParallelSelfTransitionKeepsItsLeafState.Run))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: judging's transition 0, as the microstep reads it.
        val transitionJudgingAt0 = EnabledTransition<ParallelSelfTransitionKeepsItsLeafState, HistoryId>(
            ParallelSelfTransitionKeepsItsLeafState.Judging,
            listOf(StateTarget(ParallelSelfTransitionKeepsItsLeafState.Settled)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: within's transition 0, as the microstep reads it.
        val transitionWithinAt0 = EnabledTransition<ParallelSelfTransitionKeepsItsLeafState, HistoryId>(
            ParallelSelfTransitionKeepsItsLeafState.Within,
            listOf(StateTarget(ParallelSelfTransitionKeepsItsLeafState.Within)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<ParallelSelfTransitionKeepsItsLeafState, HistoryId>(
            ParallelSelfTransitionKeepsItsLeafState.Working,
            listOf(StateTarget(ParallelSelfTransitionKeepsItsLeafState.Judging)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ParallelSelfTransitionKeepsItsLeafState? = when (stateId) {
        "budget" -> ParallelSelfTransitionKeepsItsLeafState.Budget
        "drive" -> ParallelSelfTransitionKeepsItsLeafState.Drive
        "judging" -> ParallelSelfTransitionKeepsItsLeafState.Judging
        "run" -> ParallelSelfTransitionKeepsItsLeafState.Run
        "running" -> ParallelSelfTransitionKeepsItsLeafState.Running
        "settled" -> ParallelSelfTransitionKeepsItsLeafState.Settled
        "within" -> ParallelSelfTransitionKeepsItsLeafState.Within
        "working" -> ParallelSelfTransitionKeepsItsLeafState.Working
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ParallelSelfTransitionKeepsItsLeafState): String = when (state) {
        is ParallelSelfTransitionKeepsItsLeafState.Budget -> "budget"
        is ParallelSelfTransitionKeepsItsLeafState.Drive -> "drive"
        is ParallelSelfTransitionKeepsItsLeafState.Judging -> "judging"
        is ParallelSelfTransitionKeepsItsLeafState.Run -> "run"
        is ParallelSelfTransitionKeepsItsLeafState.Running -> "running"
        is ParallelSelfTransitionKeepsItsLeafState.Settled -> "settled"
        is ParallelSelfTransitionKeepsItsLeafState.Within -> "within"
        is ParallelSelfTransitionKeepsItsLeafState.Working -> "working"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ParallelSelfTransitionKeepsItsLeafState): Int = when (state) {
        is ParallelSelfTransitionKeepsItsLeafState.Budget -> 1
        is ParallelSelfTransitionKeepsItsLeafState.Drive -> 3
        is ParallelSelfTransitionKeepsItsLeafState.Judging -> 6
        is ParallelSelfTransitionKeepsItsLeafState.Run -> 0
        is ParallelSelfTransitionKeepsItsLeafState.Running -> 4
        is ParallelSelfTransitionKeepsItsLeafState.Settled -> 7
        is ParallelSelfTransitionKeepsItsLeafState.Within -> 2
        is ParallelSelfTransitionKeepsItsLeafState.Working -> 5
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ParallelSelfTransitionKeepsItsLeafEvent? = when (name) {
        "check" -> ParallelSelfTransitionKeepsItsLeafEvent.Check
        "e" -> ParallelSelfTransitionKeepsItsLeafEvent.E
        "error.execution" -> ParallelSelfTransitionKeepsItsLeafEvent.Error.Execution
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ParallelSelfTransitionKeepsItsLeafEvent): String? = when (event) {
        is ParallelSelfTransitionKeepsItsLeafEvent.Check -> "check"
        is ParallelSelfTransitionKeepsItsLeafEvent.E -> "e"
        is ParallelSelfTransitionKeepsItsLeafEvent.Error.Execution -> "error.execution"
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
            "parallel_self_transition_keeps_its_leaf",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        declareVariablesPart0(engine, sid)



        // W3C SCXML 5.9.2: Register In() predicate callback
        engine.setStateQueryCallback(sid) { stateId -> isStateActive(stateId) }

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
        // W3C SCXML 5.3: Initialize variable 'n' with expr
        try {
            val initResult_n = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "n", initResult_n)
        } catch (e: Exception) {
            raisePlatformError(ParallelSelfTransitionKeepsItsLeafEvent.Error.Execution, "<data id='n'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'm' with expr
        try {
            val initResult_m = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "m", initResult_m)
        } catch (e: Exception) {
            raisePlatformError(ParallelSelfTransitionKeepsItsLeafEvent.Error.Execution, "<data id='m'> expr failed to evaluate")
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
            raisePlatformError(ParallelSelfTransitionKeepsItsLeafEvent.Error.Execution, reason)
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
            raisePlatformError(ParallelSelfTransitionKeepsItsLeafEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(ParallelSelfTransitionKeepsItsLeafEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(ParallelSelfTransitionKeepsItsLeafEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ParallelSelfTransitionKeepsItsLeafEvent) {
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
    override fun bindCurrentEvent(event: ParallelSelfTransitionKeepsItsLeafEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: ParallelSelfTransitionKeepsItsLeafState, event: ParallelSelfTransitionKeepsItsLeafEvent?): EnabledTransition<ParallelSelfTransitionKeepsItsLeafState, HistoryId>? = when (state) {
        is ParallelSelfTransitionKeepsItsLeafState.Judging -> firstEnabledTransitionOfJudging(state, event)
        is ParallelSelfTransitionKeepsItsLeafState.Within -> firstEnabledTransitionOfWithin(state, event)
        is ParallelSelfTransitionKeepsItsLeafState.Working -> firstEnabledTransitionOfWorking(state, event)
        else -> null
    }


    // judging's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfJudging(state: ParallelSelfTransitionKeepsItsLeafState, event: ParallelSelfTransitionKeepsItsLeafEvent?): EnabledTransition<ParallelSelfTransitionKeepsItsLeafState, HistoryId>? = when {
        event is ParallelSelfTransitionKeepsItsLeafEvent.Check && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_scxml_eq(n, 1) and _scxml_eq(m, 2))", "n == 1 && m == 2")) -> TransitionsPart0.transitionJudgingAt0
        else -> null
    }


    // within's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWithin(state: ParallelSelfTransitionKeepsItsLeafState, event: ParallelSelfTransitionKeepsItsLeafEvent?): EnabledTransition<ParallelSelfTransitionKeepsItsLeafState, HistoryId>? = when {
        event is ParallelSelfTransitionKeepsItsLeafEvent.E -> TransitionsPart0.transitionWithinAt0
        else -> null
    }


    // working's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWorking(state: ParallelSelfTransitionKeepsItsLeafState, event: ParallelSelfTransitionKeepsItsLeafEvent?): EnabledTransition<ParallelSelfTransitionKeepsItsLeafState, HistoryId>? = when {
        event is ParallelSelfTransitionKeepsItsLeafEvent.E -> TransitionsPart0.transitionWorkingAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:53 :: _machine
    override fun onEntry(state: ParallelSelfTransitionKeepsItsLeafState, isDefaultEntry: Boolean) {
        when (state) {
            is ParallelSelfTransitionKeepsItsLeafState.Budget -> onEntryOfBudget()
            is ParallelSelfTransitionKeepsItsLeafState.Drive -> onEntryOfDrive()
            is ParallelSelfTransitionKeepsItsLeafState.Judging -> onEntryOfJudging()
            is ParallelSelfTransitionKeepsItsLeafState.Run -> onEntryOfRun()
            is ParallelSelfTransitionKeepsItsLeafState.Running -> onEntryOfRunning()
            is ParallelSelfTransitionKeepsItsLeafState.Settled -> onEntryOfSettled()
            is ParallelSelfTransitionKeepsItsLeafState.Within -> onEntryOfWithin()
            is ParallelSelfTransitionKeepsItsLeafState.Working -> onEntryOfWorking()
        }
    }


    private fun onEntryOfBudget() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:71 :: budget :: _state_body
    }

    private fun onEntryOfDrive() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:83 :: drive :: _state_body
    }

    private fun onEntryOfJudging() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:90 :: judging :: _state_body
    }

    private fun onEntryOfRun() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:62 :: run :: _state_body
    }

    private fun onEntryOfRunning() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:84 :: running :: _state_body
    }

    private fun onEntryOfSettled() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:103 :: settled :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfWithin() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:72 :: within :: _state_body
    }

    private fun onEntryOfWorking() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:85 :: working :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:53 :: _machine
    override fun onExit(state: ParallelSelfTransitionKeepsItsLeafState) {
        when (state) {
            is ParallelSelfTransitionKeepsItsLeafState.Budget -> onExitOfBudget()
            is ParallelSelfTransitionKeepsItsLeafState.Drive -> onExitOfDrive()
            is ParallelSelfTransitionKeepsItsLeafState.Judging -> onExitOfJudging()
            is ParallelSelfTransitionKeepsItsLeafState.Run -> onExitOfRun()
            is ParallelSelfTransitionKeepsItsLeafState.Running -> onExitOfRunning()
            is ParallelSelfTransitionKeepsItsLeafState.Settled -> onExitOfSettled()
            is ParallelSelfTransitionKeepsItsLeafState.Within -> onExitOfWithin()
            is ParallelSelfTransitionKeepsItsLeafState.Working -> onExitOfWorking()
        }
    }


    private fun onExitOfBudget() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:71 :: budget :: _state_body
    }

    private fun onExitOfDrive() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:83 :: drive :: _state_body
    }

    private fun onExitOfJudging() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:90 :: judging :: _state_body
    }

    private fun onExitOfRun() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:62 :: run :: _state_body
    }

    private fun onExitOfRunning() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:84 :: running :: _state_body
    }

    private fun onExitOfSettled() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:103 :: settled :: _state_body
    }

    private fun onExitOfWithin() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:72 :: within :: _state_body
    }

    private fun onExitOfWorking() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:85 :: working :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:53 :: _machine
    override fun executeTransitionContent(source: ParallelSelfTransitionKeepsItsLeafState, transitionIndex: Int) {
        when (source) {
            is ParallelSelfTransitionKeepsItsLeafState.Within -> transitionContentOfWithin(transitionIndex)
            is ParallelSelfTransitionKeepsItsLeafState.Working -> transitionContentOfWorking(transitionIndex)
            else -> {}
        }
    }


    // The content of `within`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWithin(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWithinAt0()
            else -> {}
        }
    }


    private fun transitionContentOfWithinAt0() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:73 :: within :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("m", "m"), com.sce.runtime.ScriptSource.lua("_scxml_add(m, 1)", "m + 1"))) {
                return
            }
    }

    // The content of `working`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWorking(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWorkingAt0()
            else -> {}
        }
    }


    private fun transitionContentOfWorkingAt0() {
                // SCE-MAP: parallel_self_transition_keeps_its_leaf.scxml:86 :: working :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("n", "n"), com.sce.runtime.ScriptSource.lua("_scxml_add(n, 1)", "n + 1"))) {
                return
            }
    }
}
