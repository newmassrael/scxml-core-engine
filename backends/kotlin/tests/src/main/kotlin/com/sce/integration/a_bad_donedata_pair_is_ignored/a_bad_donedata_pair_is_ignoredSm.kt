// SCE-GENERATED — DO NOT EDIT
// source-hash: ca41bb31388364a653df5c43642b7f70894bfe28acab04d3f8f0ea8b795da311

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_bad_donedata_pair_is_ignored/a_bad_donedata_pair_is_ignored.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:23 :: _machine

package com.sce.integration.a_bad_donedata_pair_is_ignored

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ABadDonedataPairIsIgnoredState : State {
    data object Done : ABadDonedataPairIsIgnoredState
    data object P : ABadDonedataPairIsIgnoredState
    data object R1 : ABadDonedataPairIsIgnoredState
    data object R1a : ABadDonedataPairIsIgnoredState
    data object R1f : ABadDonedataPairIsIgnoredState
    data object R2 : ABadDonedataPairIsIgnoredState
    data object R2f : ABadDonedataPairIsIgnoredState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ABadDonedataPairIsIgnoredEvent : Event {
    sealed interface Done : ABadDonedataPairIsIgnoredEvent {
        sealed interface State : Done {
            data object P : State
            data object R1 : State
            data object R2 : State
        }
    }
    sealed interface Error : ABadDonedataPairIsIgnoredEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class ABadDonedataPairIsIgnoredStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ABadDonedataPairIsIgnoredState, ABadDonedataPairIsIgnoredEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

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
     * §scxml-5.3: what the `shape` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `shape` was assigned a value of another type, or the engine refused.
     */
    fun shape(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "shape")

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

    override val initialState: ABadDonedataPairIsIgnoredState = ABadDonedataPairIsIgnoredState.R1a

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: ABadDonedataPairIsIgnoredState): ABadDonedataPairIsIgnoredState? = when (state) {
        is ABadDonedataPairIsIgnoredState.R1 -> ABadDonedataPairIsIgnoredState.P
        is ABadDonedataPairIsIgnoredState.R1a -> ABadDonedataPairIsIgnoredState.R1
        is ABadDonedataPairIsIgnoredState.R1f -> ABadDonedataPairIsIgnoredState.R1
        is ABadDonedataPairIsIgnoredState.R2 -> ABadDonedataPairIsIgnoredState.P
        is ABadDonedataPairIsIgnoredState.R2f -> ABadDonedataPairIsIgnoredState.R2
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: ABadDonedataPairIsIgnoredState): Boolean = when (state) {
        is ABadDonedataPairIsIgnoredState.R1 -> true
        is ABadDonedataPairIsIgnoredState.R2 -> true
        else -> false
    }


    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: ABadDonedataPairIsIgnoredState): Boolean = when (state) {
        is ABadDonedataPairIsIgnoredState.P -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: ABadDonedataPairIsIgnoredState): Boolean = when (state) {
        is ABadDonedataPairIsIgnoredState.Done -> true
        is ABadDonedataPairIsIgnoredState.R1f -> true
        is ABadDonedataPairIsIgnoredState.R2f -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: ABadDonedataPairIsIgnoredState): List<ABadDonedataPairIsIgnoredState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: ABadDonedataPairIsIgnoredState): List<EntryTarget<ABadDonedataPairIsIgnoredState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ABadDonedataPairIsIgnoredState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<ABadDonedataPairIsIgnoredState, List<ABadDonedataPairIsIgnoredState>> = LinkedHashMap<ABadDonedataPairIsIgnoredState, List<ABadDonedataPairIsIgnoredState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<ABadDonedataPairIsIgnoredState, List<ABadDonedataPairIsIgnoredState>>) {
            table[ABadDonedataPairIsIgnoredState.P] = listOf(ABadDonedataPairIsIgnoredState.R1, ABadDonedataPairIsIgnoredState.R2)
            table[ABadDonedataPairIsIgnoredState.R1] = listOf(ABadDonedataPairIsIgnoredState.R1a, ABadDonedataPairIsIgnoredState.R1f)
            table[ABadDonedataPairIsIgnoredState.R2] = listOf(ABadDonedataPairIsIgnoredState.R2f)
        }


        val initialTargets: Map<ABadDonedataPairIsIgnoredState, List<EntryTarget<ABadDonedataPairIsIgnoredState, HistoryId>>> = LinkedHashMap<ABadDonedataPairIsIgnoredState, List<EntryTarget<ABadDonedataPairIsIgnoredState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<ABadDonedataPairIsIgnoredState, List<EntryTarget<ABadDonedataPairIsIgnoredState, HistoryId>>>) {
            table[ABadDonedataPairIsIgnoredState.R1] = listOf(StateTarget(ABadDonedataPairIsIgnoredState.R1a))
            table[ABadDonedataPairIsIgnoredState.R2] = listOf(StateTarget(ABadDonedataPairIsIgnoredState.R2f))
        }


        val documentInitialTargetList: List<EntryTarget<ABadDonedataPairIsIgnoredState, HistoryId>> =
            listOf(StateTarget(ABadDonedataPairIsIgnoredState.P))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: p's transition 0, as the microstep reads it.
        val transitionPAt0 = EnabledTransition<ABadDonedataPairIsIgnoredState, HistoryId>(
            ABadDonedataPairIsIgnoredState.P,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: p's transition 1, as the microstep reads it.
        val transitionPAt1 = EnabledTransition<ABadDonedataPairIsIgnoredState, HistoryId>(
            ABadDonedataPairIsIgnoredState.P,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: p's transition 2, as the microstep reads it.
        val transitionPAt2 = EnabledTransition<ABadDonedataPairIsIgnoredState, HistoryId>(
            ABadDonedataPairIsIgnoredState.P,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: p's transition 3, as the microstep reads it.
        val transitionPAt3 = EnabledTransition<ABadDonedataPairIsIgnoredState, HistoryId>(
            ABadDonedataPairIsIgnoredState.P,
            listOf(StateTarget(ABadDonedataPairIsIgnoredState.Done)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: r1a's transition 0, as the microstep reads it.
        val transitionR1aAt0 = EnabledTransition<ABadDonedataPairIsIgnoredState, HistoryId>(
            ABadDonedataPairIsIgnoredState.R1a,
            listOf(StateTarget(ABadDonedataPairIsIgnoredState.R1f)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ABadDonedataPairIsIgnoredState? = when (stateId) {
        "done" -> ABadDonedataPairIsIgnoredState.Done
        "p" -> ABadDonedataPairIsIgnoredState.P
        "r1" -> ABadDonedataPairIsIgnoredState.R1
        "r1a" -> ABadDonedataPairIsIgnoredState.R1a
        "r1f" -> ABadDonedataPairIsIgnoredState.R1f
        "r2" -> ABadDonedataPairIsIgnoredState.R2
        "r2f" -> ABadDonedataPairIsIgnoredState.R2f
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ABadDonedataPairIsIgnoredState): String = when (state) {
        is ABadDonedataPairIsIgnoredState.Done -> "done"
        is ABadDonedataPairIsIgnoredState.P -> "p"
        is ABadDonedataPairIsIgnoredState.R1 -> "r1"
        is ABadDonedataPairIsIgnoredState.R1a -> "r1a"
        is ABadDonedataPairIsIgnoredState.R1f -> "r1f"
        is ABadDonedataPairIsIgnoredState.R2 -> "r2"
        is ABadDonedataPairIsIgnoredState.R2f -> "r2f"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ABadDonedataPairIsIgnoredState): Int = when (state) {
        is ABadDonedataPairIsIgnoredState.Done -> 6
        is ABadDonedataPairIsIgnoredState.P -> 0
        is ABadDonedataPairIsIgnoredState.R1 -> 1
        is ABadDonedataPairIsIgnoredState.R1a -> 2
        is ABadDonedataPairIsIgnoredState.R1f -> 3
        is ABadDonedataPairIsIgnoredState.R2 -> 4
        is ABadDonedataPairIsIgnoredState.R2f -> 5
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ABadDonedataPairIsIgnoredEvent? = when (name) {
        "done.state.p" -> ABadDonedataPairIsIgnoredEvent.Done.State.P
        "done.state.r1" -> ABadDonedataPairIsIgnoredEvent.Done.State.R1
        "done.state.r2" -> ABadDonedataPairIsIgnoredEvent.Done.State.R2
        "error.execution" -> ABadDonedataPairIsIgnoredEvent.Error.Execution
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ABadDonedataPairIsIgnoredEvent): String? = when (event) {
        is ABadDonedataPairIsIgnoredEvent.Done.State.P -> "done.state.p"
        is ABadDonedataPairIsIgnoredEvent.Done.State.R1 -> "done.state.r1"
        is ABadDonedataPairIsIgnoredEvent.Done.State.R2 -> "done.state.r2"
        is ABadDonedataPairIsIgnoredEvent.Error.Execution -> "error.execution"
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
            "a_bad_donedata_pair_is_ignored",
        )?.let { return it }

        // W3C SCXML 5.9.2: Register In() predicate callback, before any
        // `<data>` or top-level `<script>` can ask it
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        engine.setStateQueryCallback(sid) { stateId -> isStateActive(stateId) }
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
        // W3C SCXML 5.3: Initialize variable 'errors' with expr
        try {
            val initResult_errors = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "errors", initResult_errors)
        } catch (e: Exception) {
            raisePlatformError(ABadDonedataPairIsIgnoredEvent.Error.Execution, "<data id='errors'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'shape' with expr
        try {
            val initResult_shape = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "shape", initResult_shape)
        } catch (e: Exception) {
            raisePlatformError(ABadDonedataPairIsIgnoredEvent.Error.Execution, "<data id='shape'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'v' with expr
        try {
            val initResult_v = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("7", "7"))
            engine.setVariable(sid, "v", initResult_v)
        } catch (e: Exception) {
            raisePlatformError(ABadDonedataPairIsIgnoredEvent.Error.Execution, "<data id='v'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'obj' with expr
        try {
            val initResult_obj = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{}", "({})"))
            engine.setVariable(sid, "obj", initResult_obj)
        } catch (e: Exception) {
            raisePlatformError(ABadDonedataPairIsIgnoredEvent.Error.Execution, "<data id='obj'> expr failed to evaluate")
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
            raisePlatformError(ABadDonedataPairIsIgnoredEvent.Error.Execution, reason)
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
            raisePlatformError(ABadDonedataPairIsIgnoredEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(ABadDonedataPairIsIgnoredEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(ABadDonedataPairIsIgnoredEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ABadDonedataPairIsIgnoredEvent) {
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
    override fun bindCurrentEvent(event: ABadDonedataPairIsIgnoredEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: ABadDonedataPairIsIgnoredState, event: ABadDonedataPairIsIgnoredEvent?): EnabledTransition<ABadDonedataPairIsIgnoredState, HistoryId>? = when (state) {
        is ABadDonedataPairIsIgnoredState.P -> firstEnabledTransitionOfP(state, event)
        is ABadDonedataPairIsIgnoredState.R1a -> firstEnabledTransitionOfR1a(state, event)
        else -> null
    }


    // p's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfP(state: ABadDonedataPairIsIgnoredState, event: ABadDonedataPairIsIgnoredEvent?): EnabledTransition<ABadDonedataPairIsIgnoredState, HistoryId>? = when {
        event is ABadDonedataPairIsIgnoredEvent.Error.Execution -> TransitionsPart0.transitionPAt0
        event is ABadDonedataPairIsIgnoredEvent.Done.State.R1 && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(((_event.data.good == 7) and (_typeof(_event.data.empty) == \"undefined\")) and (_typeof(_event.data.bad) == \"undefined\"))", "_event.data.good === 7 && typeof _event.data.empty === 'undefined' && typeof _event.data.bad === 'undefined'")) -> TransitionsPart0.transitionPAt1
        event is ABadDonedataPairIsIgnoredEvent.Done.State.R1 -> TransitionsPart0.transitionPAt2
        event is ABadDonedataPairIsIgnoredEvent.Done.State.P -> TransitionsPart0.transitionPAt3
        else -> null
    }


    // r1a's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfR1a(state: ABadDonedataPairIsIgnoredState, event: ABadDonedataPairIsIgnoredEvent?): EnabledTransition<ABadDonedataPairIsIgnoredState, HistoryId>? = when {
        event == null -> TransitionsPart0.transitionR1aAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:23 :: _machine
    override fun onEntry(state: ABadDonedataPairIsIgnoredState, isDefaultEntry: Boolean) {
        when (state) {
            is ABadDonedataPairIsIgnoredState.Done -> onEntryOfDone()
            is ABadDonedataPairIsIgnoredState.P -> onEntryOfP()
            is ABadDonedataPairIsIgnoredState.R1 -> onEntryOfR1()
            is ABadDonedataPairIsIgnoredState.R1a -> onEntryOfR1a()
            is ABadDonedataPairIsIgnoredState.R1f -> onEntryOfR1f()
            is ABadDonedataPairIsIgnoredState.R2 -> onEntryOfR2()
            is ABadDonedataPairIsIgnoredState.R2f -> onEntryOfR2f()
        }
    }


    private fun onEntryOfDone() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:64 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfP() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:34 :: p :: _state_body
    }

    private fun onEntryOfR1() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:35 :: r1 :: _state_body
    }

    private fun onEntryOfR1a() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:36 :: r1a :: _state_body
    }

    private fun onEntryOfR1f() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:39 :: r1f :: _state_body
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    assertScriptSession()
                    val engineDD = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidDD = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
                    var doneEventData = ""
                    // W3C SCXML 5.5: Evaluate <param> elements (C++ DoneDataHelper::evaluateParams pattern)
                    val doneParams = mutableMapOf<String, Any?>()
                    // §scxml-5.7: an empty location names no location —
                    // error.execution, and this pair is ignored.
                    raisePlatformError(ABadDonedataPairIsIgnoredEvent.Error.Execution, "<donedata> <param name='empty'> has an empty location")
                    try {
                        doneParams["good"] = engineDD.evaluateExpr(sidDD, com.sce.runtime.ScriptSource.lua("v", "v"))
                    } catch (_: Exception) {
                        // §scxml-5.7: error.execution, and this pair is ignored.
                        raisePlatformError(ABadDonedataPairIsIgnoredEvent.Error.Execution, "<donedata> <param name='good'> failed to evaluate")
                    }
                    try {
                        doneParams["bad"] = engineDD.evaluateExpr(sidDD, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"))
                    } catch (_: Exception) {
                        // §scxml-5.7: error.execution, and this pair is ignored.
                        raisePlatformError(ABadDonedataPairIsIgnoredEvent.Error.Execution, "<donedata> <param name='bad'> failed to evaluate")
                    }
                    // §scxml-5.5: the pairs that survived, `{}` when none did
                    // (C++ DoneDataHelper::evaluateParams). Not left to
                    // buildJsonFromParams, whose empty answer is a <send>'s.
                    doneEventData = if (doneParams.isEmpty()) "{}" else buildJsonFromParams(doneParams)
                    // W3C SCXML 3.7: Final child state reached, raise done.state with data
                    raiseInternal(ABadDonedataPairIsIgnoredEvent.Done.State.R1, EventMetadata.platform(doneEventData))
                }
                // W3C SCXML 3.7.1: this <final> may have completed the
                // <parallel> grandparent — Appendix D's isInFinalState, which
                // counts a region that is itself a <parallel> only once all
                // of ITS regions are final.
                if (isStateInFinalState(ABadDonedataPairIsIgnoredState.P)) {
                    raiseInternal(ABadDonedataPairIsIgnoredEvent.Done.State.P)
                }
    }

    private fun onEntryOfR2() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:47 :: r2 :: _state_body
    }

    private fun onEntryOfR2f() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:48 :: r2f :: _state_body
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(ABadDonedataPairIsIgnoredEvent.Done.State.R2, EventMetadata.platform())
                // W3C SCXML 3.7.1: this <final> may have completed the
                // <parallel> grandparent — Appendix D's isInFinalState, which
                // counts a region that is itself a <parallel> only once all
                // of ITS regions are final.
                if (isStateInFinalState(ABadDonedataPairIsIgnoredState.P)) {
                    raiseInternal(ABadDonedataPairIsIgnoredEvent.Done.State.P)
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:23 :: _machine
    override fun onExit(state: ABadDonedataPairIsIgnoredState) {
        when (state) {
            is ABadDonedataPairIsIgnoredState.Done -> onExitOfDone()
            is ABadDonedataPairIsIgnoredState.P -> onExitOfP()
            is ABadDonedataPairIsIgnoredState.R1 -> onExitOfR1()
            is ABadDonedataPairIsIgnoredState.R1a -> onExitOfR1a()
            is ABadDonedataPairIsIgnoredState.R1f -> onExitOfR1f()
            is ABadDonedataPairIsIgnoredState.R2 -> onExitOfR2()
            is ABadDonedataPairIsIgnoredState.R2f -> onExitOfR2f()
        }
    }


    private fun onExitOfDone() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:64 :: done :: _state_body
    }

    private fun onExitOfP() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:34 :: p :: _state_body
    }

    private fun onExitOfR1() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:35 :: r1 :: _state_body
    }

    private fun onExitOfR1a() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:36 :: r1a :: _state_body
    }

    private fun onExitOfR1f() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:39 :: r1f :: _state_body
    }

    private fun onExitOfR2() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:47 :: r2 :: _state_body
    }

    private fun onExitOfR2f() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:48 :: r2f :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:23 :: _machine
    override fun executeTransitionContent(source: ABadDonedataPairIsIgnoredState, transitionIndex: Int) {
        when (source) {
            is ABadDonedataPairIsIgnoredState.P -> transitionContentOfP(transitionIndex)
            else -> {}
        }
    }


    // The content of `p`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfP(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfPAt0()
            1 -> transitionContentOfPAt1()
            2 -> transitionContentOfPAt2()
            else -> {}
        }
    }


    private fun transitionContentOfPAt0() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:51 :: p :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("errors", "errors"), com.sce.runtime.ScriptSource.lua("_scxml_add(errors, 1)", "errors + 1"))) {
                return
            }
    }

    private fun transitionContentOfPAt1() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:54 :: p :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("shape", "shape"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
    }

    private fun transitionContentOfPAt2() {
                // SCE-MAP: a_bad_donedata_pair_is_ignored.scxml:58 :: p :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("shape", "shape"), com.sce.runtime.ScriptSource.lua("2", "2"))) {
                return
            }
    }
}
