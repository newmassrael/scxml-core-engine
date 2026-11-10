// SCE-GENERATED — DO NOT EDIT
// source-hash: 9f1d28aedc299ea1c7281cab0278613454849fe4e66eb28d8ffe4b4c74733c7f

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/onexit_runs_before_the_state_leaves/onexit_runs_before_the_state_leaves.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: onexit_runs_before_the_state_leaves.scxml:33 :: _machine

package com.sce.integration.onexit_runs_before_the_state_leaves

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface OnexitRunsBeforeTheStateLeavesState : State {
    data object Check : OnexitRunsBeforeTheStateLeavesState
    data object FailChildInOuter : OnexitRunsBeforeTheStateLeavesState
    data object FailExits : OnexitRunsBeforeTheStateLeavesState
    data object FailParentInInner : OnexitRunsBeforeTheStateLeavesState
    data object FailSelfInInner : OnexitRunsBeforeTheStateLeavesState
    data object FailSelfInOuter : OnexitRunsBeforeTheStateLeavesState
    data object Inner : OnexitRunsBeforeTheStateLeavesState
    data object Outer : OnexitRunsBeforeTheStateLeavesState
    data object Settled : OnexitRunsBeforeTheStateLeavesState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface OnexitRunsBeforeTheStateLeavesEvent : Event {
    sealed interface Error : OnexitRunsBeforeTheStateLeavesEvent {
        data object Execution : Error
    }
    data object Leave : OnexitRunsBeforeTheStateLeavesEvent
}
// --- State Machine (W3C SCXML) ---

class OnexitRunsBeforeTheStateLeavesStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<OnexitRunsBeforeTheStateLeavesState, OnexitRunsBeforeTheStateLeavesEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `selfInInner` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `selfInInner` was assigned a value of another type, or the engine refused.
     */
    fun selfInInner(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "selfInInner")

    /**
     * §scxml-5.3: what the `parentInInner` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `parentInInner` was assigned a value of another type, or the engine refused.
     */
    fun parentInInner(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "parentInInner")

    /**
     * §scxml-5.3: what the `selfInOuter` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `selfInOuter` was assigned a value of another type, or the engine refused.
     */
    fun selfInOuter(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "selfInOuter")

    /**
     * §scxml-5.3: what the `childInOuter` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `childInOuter` was assigned a value of another type, or the engine refused.
     */
    fun childInOuter(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "childInOuter")

    /**
     * §scxml-5.3: what the `exits` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `exits` was assigned a value of another type, or the engine refused.
     */
    fun exits(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "exits")

    override val initialState: OnexitRunsBeforeTheStateLeavesState = OnexitRunsBeforeTheStateLeavesState.Inner

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
    override fun parentOf(state: OnexitRunsBeforeTheStateLeavesState): OnexitRunsBeforeTheStateLeavesState? = when (state) {
        is OnexitRunsBeforeTheStateLeavesState.Inner -> OnexitRunsBeforeTheStateLeavesState.Outer
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: OnexitRunsBeforeTheStateLeavesState): Boolean = when (state) {
        is OnexitRunsBeforeTheStateLeavesState.Outer -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: OnexitRunsBeforeTheStateLeavesState): Boolean = when (state) {
        is OnexitRunsBeforeTheStateLeavesState.FailChildInOuter -> true
        is OnexitRunsBeforeTheStateLeavesState.FailExits -> true
        is OnexitRunsBeforeTheStateLeavesState.FailParentInInner -> true
        is OnexitRunsBeforeTheStateLeavesState.FailSelfInInner -> true
        is OnexitRunsBeforeTheStateLeavesState.FailSelfInOuter -> true
        is OnexitRunsBeforeTheStateLeavesState.Settled -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: OnexitRunsBeforeTheStateLeavesState): List<OnexitRunsBeforeTheStateLeavesState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: OnexitRunsBeforeTheStateLeavesState): List<EntryTarget<OnexitRunsBeforeTheStateLeavesState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<OnexitRunsBeforeTheStateLeavesState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<OnexitRunsBeforeTheStateLeavesState, List<OnexitRunsBeforeTheStateLeavesState>> = LinkedHashMap<OnexitRunsBeforeTheStateLeavesState, List<OnexitRunsBeforeTheStateLeavesState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<OnexitRunsBeforeTheStateLeavesState, List<OnexitRunsBeforeTheStateLeavesState>>) {
            table[OnexitRunsBeforeTheStateLeavesState.Outer] = listOf(OnexitRunsBeforeTheStateLeavesState.Inner)
        }


        val initialTargets: Map<OnexitRunsBeforeTheStateLeavesState, List<EntryTarget<OnexitRunsBeforeTheStateLeavesState, HistoryId>>> = LinkedHashMap<OnexitRunsBeforeTheStateLeavesState, List<EntryTarget<OnexitRunsBeforeTheStateLeavesState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<OnexitRunsBeforeTheStateLeavesState, List<EntryTarget<OnexitRunsBeforeTheStateLeavesState, HistoryId>>>) {
            table[OnexitRunsBeforeTheStateLeavesState.Outer] = listOf(StateTarget(OnexitRunsBeforeTheStateLeavesState.Inner))
        }


        val documentInitialTargetList: List<EntryTarget<OnexitRunsBeforeTheStateLeavesState, HistoryId>> =
            listOf(StateTarget(OnexitRunsBeforeTheStateLeavesState.Outer))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: check's transition 0, as the microstep reads it.
        val transitionCheckAt0 = EnabledTransition<OnexitRunsBeforeTheStateLeavesState, HistoryId>(
            OnexitRunsBeforeTheStateLeavesState.Check,
            listOf(StateTarget(OnexitRunsBeforeTheStateLeavesState.FailExits)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: check's transition 1, as the microstep reads it.
        val transitionCheckAt1 = EnabledTransition<OnexitRunsBeforeTheStateLeavesState, HistoryId>(
            OnexitRunsBeforeTheStateLeavesState.Check,
            listOf(StateTarget(OnexitRunsBeforeTheStateLeavesState.FailSelfInInner)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: check's transition 2, as the microstep reads it.
        val transitionCheckAt2 = EnabledTransition<OnexitRunsBeforeTheStateLeavesState, HistoryId>(
            OnexitRunsBeforeTheStateLeavesState.Check,
            listOf(StateTarget(OnexitRunsBeforeTheStateLeavesState.FailParentInInner)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: check's transition 3, as the microstep reads it.
        val transitionCheckAt3 = EnabledTransition<OnexitRunsBeforeTheStateLeavesState, HistoryId>(
            OnexitRunsBeforeTheStateLeavesState.Check,
            listOf(StateTarget(OnexitRunsBeforeTheStateLeavesState.FailSelfInOuter)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: check's transition 4, as the microstep reads it.
        val transitionCheckAt4 = EnabledTransition<OnexitRunsBeforeTheStateLeavesState, HistoryId>(
            OnexitRunsBeforeTheStateLeavesState.Check,
            listOf(StateTarget(OnexitRunsBeforeTheStateLeavesState.FailChildInOuter)),
            4,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: check's transition 5, as the microstep reads it.
        val transitionCheckAt5 = EnabledTransition<OnexitRunsBeforeTheStateLeavesState, HistoryId>(
            OnexitRunsBeforeTheStateLeavesState.Check,
            listOf(StateTarget(OnexitRunsBeforeTheStateLeavesState.Settled)),
            5,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: inner's transition 0, as the microstep reads it.
        val transitionInnerAt0 = EnabledTransition<OnexitRunsBeforeTheStateLeavesState, HistoryId>(
            OnexitRunsBeforeTheStateLeavesState.Inner,
            listOf(StateTarget(OnexitRunsBeforeTheStateLeavesState.Check)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): OnexitRunsBeforeTheStateLeavesState? = when (stateId) {
        "check" -> OnexitRunsBeforeTheStateLeavesState.Check
        "failChildInOuter" -> OnexitRunsBeforeTheStateLeavesState.FailChildInOuter
        "failExits" -> OnexitRunsBeforeTheStateLeavesState.FailExits
        "failParentInInner" -> OnexitRunsBeforeTheStateLeavesState.FailParentInInner
        "failSelfInInner" -> OnexitRunsBeforeTheStateLeavesState.FailSelfInInner
        "failSelfInOuter" -> OnexitRunsBeforeTheStateLeavesState.FailSelfInOuter
        "inner" -> OnexitRunsBeforeTheStateLeavesState.Inner
        "outer" -> OnexitRunsBeforeTheStateLeavesState.Outer
        "settled" -> OnexitRunsBeforeTheStateLeavesState.Settled
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: OnexitRunsBeforeTheStateLeavesState): String = when (state) {
        is OnexitRunsBeforeTheStateLeavesState.Check -> "check"
        is OnexitRunsBeforeTheStateLeavesState.FailChildInOuter -> "failChildInOuter"
        is OnexitRunsBeforeTheStateLeavesState.FailExits -> "failExits"
        is OnexitRunsBeforeTheStateLeavesState.FailParentInInner -> "failParentInInner"
        is OnexitRunsBeforeTheStateLeavesState.FailSelfInInner -> "failSelfInInner"
        is OnexitRunsBeforeTheStateLeavesState.FailSelfInOuter -> "failSelfInOuter"
        is OnexitRunsBeforeTheStateLeavesState.Inner -> "inner"
        is OnexitRunsBeforeTheStateLeavesState.Outer -> "outer"
        is OnexitRunsBeforeTheStateLeavesState.Settled -> "settled"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: OnexitRunsBeforeTheStateLeavesState): Int = when (state) {
        is OnexitRunsBeforeTheStateLeavesState.Check -> 2
        is OnexitRunsBeforeTheStateLeavesState.FailChildInOuter -> 8
        is OnexitRunsBeforeTheStateLeavesState.FailExits -> 4
        is OnexitRunsBeforeTheStateLeavesState.FailParentInInner -> 6
        is OnexitRunsBeforeTheStateLeavesState.FailSelfInInner -> 5
        is OnexitRunsBeforeTheStateLeavesState.FailSelfInOuter -> 7
        is OnexitRunsBeforeTheStateLeavesState.Inner -> 1
        is OnexitRunsBeforeTheStateLeavesState.Outer -> 0
        is OnexitRunsBeforeTheStateLeavesState.Settled -> 3
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): OnexitRunsBeforeTheStateLeavesEvent? = when (name) {
        "error.execution" -> OnexitRunsBeforeTheStateLeavesEvent.Error.Execution
        "leave" -> OnexitRunsBeforeTheStateLeavesEvent.Leave
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: OnexitRunsBeforeTheStateLeavesEvent): String? = when (event) {
        is OnexitRunsBeforeTheStateLeavesEvent.Error.Execution -> "error.execution"
        is OnexitRunsBeforeTheStateLeavesEvent.Leave -> "leave"
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
            "onexit_runs_before_the_state_leaves",
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
        // W3C SCXML 5.3: Initialize variable 'selfInInner' with expr
        try {
            val initResult_selfInInner = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "selfInInner", initResult_selfInInner)
        } catch (e: Exception) {
            raisePlatformError(OnexitRunsBeforeTheStateLeavesEvent.Error.Execution, "<data id='selfInInner'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'parentInInner' with expr
        try {
            val initResult_parentInInner = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "parentInInner", initResult_parentInInner)
        } catch (e: Exception) {
            raisePlatformError(OnexitRunsBeforeTheStateLeavesEvent.Error.Execution, "<data id='parentInInner'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'selfInOuter' with expr
        try {
            val initResult_selfInOuter = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "selfInOuter", initResult_selfInOuter)
        } catch (e: Exception) {
            raisePlatformError(OnexitRunsBeforeTheStateLeavesEvent.Error.Execution, "<data id='selfInOuter'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'childInOuter' with expr
        try {
            val initResult_childInOuter = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "childInOuter", initResult_childInOuter)
        } catch (e: Exception) {
            raisePlatformError(OnexitRunsBeforeTheStateLeavesEvent.Error.Execution, "<data id='childInOuter'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'exits' with expr
        try {
            val initResult_exits = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "exits", initResult_exits)
        } catch (e: Exception) {
            raisePlatformError(OnexitRunsBeforeTheStateLeavesEvent.Error.Execution, "<data id='exits'> expr failed to evaluate")
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
            raisePlatformError(OnexitRunsBeforeTheStateLeavesEvent.Error.Execution, reason)
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
            raisePlatformError(OnexitRunsBeforeTheStateLeavesEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(OnexitRunsBeforeTheStateLeavesEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(OnexitRunsBeforeTheStateLeavesEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: OnexitRunsBeforeTheStateLeavesEvent) {
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
    override fun bindCurrentEvent(event: OnexitRunsBeforeTheStateLeavesEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: OnexitRunsBeforeTheStateLeavesState, event: OnexitRunsBeforeTheStateLeavesEvent?): EnabledTransition<OnexitRunsBeforeTheStateLeavesState, HistoryId>? = when (state) {
        is OnexitRunsBeforeTheStateLeavesState.Check -> firstEnabledTransitionOfCheck(state, event)
        is OnexitRunsBeforeTheStateLeavesState.Inner -> firstEnabledTransitionOfInner(state, event)
        else -> null
    }


    // check's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfCheck(state: OnexitRunsBeforeTheStateLeavesState, event: OnexitRunsBeforeTheStateLeavesEvent?): EnabledTransition<OnexitRunsBeforeTheStateLeavesState, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(not _scxml_eq(exits, 2))", "exits != 2")) -> TransitionsPart0.transitionCheckAt0
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(not _scxml_eq(selfInInner, 1))", "selfInInner != 1")) -> TransitionsPart0.transitionCheckAt1
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(not _scxml_eq(parentInInner, 1))", "parentInInner != 1")) -> TransitionsPart0.transitionCheckAt2
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(not _scxml_eq(selfInOuter, 1))", "selfInOuter != 1")) -> TransitionsPart0.transitionCheckAt3
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(not _scxml_eq(childInOuter, 0))", "childInOuter != 0")) -> TransitionsPart0.transitionCheckAt4
        event == null -> TransitionsPart0.transitionCheckAt5
        else -> null
    }


    // inner's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfInner(state: OnexitRunsBeforeTheStateLeavesState, event: OnexitRunsBeforeTheStateLeavesEvent?): EnabledTransition<OnexitRunsBeforeTheStateLeavesState, HistoryId>? = when {
        event is OnexitRunsBeforeTheStateLeavesEvent.Leave -> TransitionsPart0.transitionInnerAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:33 :: _machine
    override fun onEntry(state: OnexitRunsBeforeTheStateLeavesState, isDefaultEntry: Boolean) {
        when (state) {
            is OnexitRunsBeforeTheStateLeavesState.Check -> onEntryOfCheck()
            is OnexitRunsBeforeTheStateLeavesState.FailChildInOuter -> onEntryOfFailChildInOuter()
            is OnexitRunsBeforeTheStateLeavesState.FailExits -> onEntryOfFailExits()
            is OnexitRunsBeforeTheStateLeavesState.FailParentInInner -> onEntryOfFailParentInInner()
            is OnexitRunsBeforeTheStateLeavesState.FailSelfInInner -> onEntryOfFailSelfInInner()
            is OnexitRunsBeforeTheStateLeavesState.FailSelfInOuter -> onEntryOfFailSelfInOuter()
            is OnexitRunsBeforeTheStateLeavesState.Inner -> onEntryOfInner()
            is OnexitRunsBeforeTheStateLeavesState.Outer -> onEntryOfOuter()
            is OnexitRunsBeforeTheStateLeavesState.Settled -> onEntryOfSettled()
        }
    }


    private fun onEntryOfCheck() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:71 :: check :: _state_body
    }

    private fun onEntryOfFailChildInOuter() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:86 :: failChildInOuter :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailExits() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:82 :: failExits :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailParentInInner() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:84 :: failParentInInner :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailSelfInInner() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:83 :: failSelfInInner :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailSelfInOuter() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:85 :: failSelfInOuter :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfInner() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:56 :: inner :: _state_body
    }

    private fun onEntryOfOuter() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:45 :: outer :: _state_body
    }

    private fun onEntryOfSettled() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:81 :: settled :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:33 :: _machine
    override fun onExit(state: OnexitRunsBeforeTheStateLeavesState) {
        when (state) {
            is OnexitRunsBeforeTheStateLeavesState.Check -> onExitOfCheck()
            is OnexitRunsBeforeTheStateLeavesState.FailChildInOuter -> onExitOfFailChildInOuter()
            is OnexitRunsBeforeTheStateLeavesState.FailExits -> onExitOfFailExits()
            is OnexitRunsBeforeTheStateLeavesState.FailParentInInner -> onExitOfFailParentInInner()
            is OnexitRunsBeforeTheStateLeavesState.FailSelfInInner -> onExitOfFailSelfInInner()
            is OnexitRunsBeforeTheStateLeavesState.FailSelfInOuter -> onExitOfFailSelfInOuter()
            is OnexitRunsBeforeTheStateLeavesState.Inner -> onExitOfInner()
            is OnexitRunsBeforeTheStateLeavesState.Outer -> onExitOfOuter()
            is OnexitRunsBeforeTheStateLeavesState.Settled -> onExitOfSettled()
        }
    }


    private fun onExitOfCheck() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:71 :: check :: _state_body
    }

    private fun onExitOfFailChildInOuter() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:86 :: failChildInOuter :: _state_body
    }

    private fun onExitOfFailExits() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:82 :: failExits :: _state_body
    }

    private fun onExitOfFailParentInInner() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:84 :: failParentInInner :: _state_body
    }

    private fun onExitOfFailSelfInInner() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:83 :: failSelfInInner :: _state_body
    }

    private fun onExitOfFailSelfInOuter() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:85 :: failSelfInOuter :: _state_body
    }

    private fun onExitOfInner() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:56 :: inner :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("exits", "exits"), com.sce.runtime.ScriptSource.lua("_scxml_add(exits, 1)", "exits + 1"))) {
                return@run
            }


            if (isStateActive("inner")) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("selfInInner", "selfInInner"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
            }


            if (isStateActive("outer")) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("parentInInner", "parentInInner"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
            }
                }
    }

    private fun onExitOfOuter() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:45 :: outer :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("exits", "exits"), com.sce.runtime.ScriptSource.lua("_scxml_add(exits, 1)", "exits + 1"))) {
                return@run
            }


            if (isStateActive("outer")) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("selfInOuter", "selfInOuter"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
            }


            if (isStateActive("inner")) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("childInOuter", "childInOuter"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
            }
                }
    }

    private fun onExitOfSettled() {
                // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:81 :: settled :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: onexit_runs_before_the_state_leaves.scxml:33 :: _machine
    override fun executeTransitionContent(source: OnexitRunsBeforeTheStateLeavesState, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
