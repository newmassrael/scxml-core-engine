// SCE-GENERATED — DO NOT EDIT
// source-hash: 215c3b8c048d546a929c95bb520cc0c508e71ce4c95c9630e94bb32b22528dc2

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/ancestor_entry_is_not_default_entry/ancestor_entry_is_not_default_entry.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: ancestor_entry_is_not_default_entry.scxml:69 :: _machine

package com.sce.integration.ancestor_entry_is_not_default_entry

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AncestorEntryIsNotDefaultEntryState : State {
    data object Away : AncestorEntryIsNotDefaultEntryState
    data object ByDefault : AncestorEntryIsNotDefaultEntryState
    data object Chosen : AncestorEntryIsNotDefaultEntryState
    data object Drive : AncestorEntryIsNotDefaultEntryState
    data object FailDefaulted : AncestorEntryIsNotDefaultEntryState
    data object FailIdled : AncestorEntryIsNotDefaultEntryState
    data object FailLobbied : AncestorEntryIsNotDefaultEntryState
    data object FailTargeted : AncestorEntryIsNotDefaultEntryState
    data object Idle : AncestorEntryIsNotDefaultEntryState
    data object Lobby : AncestorEntryIsNotDefaultEntryState
    data object Outer : AncestorEntryIsNotDefaultEntryState
    data object Run : AncestorEntryIsNotDefaultEntryState
    data object Settled : AncestorEntryIsNotDefaultEntryState
    data object Watch : AncestorEntryIsNotDefaultEntryState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AncestorEntryIsNotDefaultEntryEvent : Event {
    data object Again : AncestorEntryIsNotDefaultEntryEvent
    data object Back : AncestorEntryIsNotDefaultEntryEvent
    data object Check : AncestorEntryIsNotDefaultEntryEvent
    data object Cross : AncestorEntryIsNotDefaultEntryEvent
    sealed interface Error : AncestorEntryIsNotDefaultEntryEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class AncestorEntryIsNotDefaultEntryStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<AncestorEntryIsNotDefaultEntryState, AncestorEntryIsNotDefaultEntryEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `defaulted` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `defaulted` was assigned a value of another type, or the engine refused.
     */
    fun defaulted(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "defaulted")

    /**
     * §scxml-5.3: what the `lobbied` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `lobbied` was assigned a value of another type, or the engine refused.
     */
    fun lobbied(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "lobbied")

    /**
     * §scxml-5.3: what the `idled` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `idled` was assigned a value of another type, or the engine refused.
     */
    fun idled(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "idled")

    /**
     * §scxml-5.3: what the `targeted` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `targeted` was assigned a value of another type, or the engine refused.
     */
    fun targeted(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "targeted")

    override val initialState: AncestorEntryIsNotDefaultEntryState = AncestorEntryIsNotDefaultEntryState.Away

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
    override fun parentOf(state: AncestorEntryIsNotDefaultEntryState): AncestorEntryIsNotDefaultEntryState? = when (state) {
        is AncestorEntryIsNotDefaultEntryState.ByDefault -> AncestorEntryIsNotDefaultEntryState.Outer
        is AncestorEntryIsNotDefaultEntryState.Chosen -> AncestorEntryIsNotDefaultEntryState.Outer
        is AncestorEntryIsNotDefaultEntryState.Drive -> AncestorEntryIsNotDefaultEntryState.Run
        is AncestorEntryIsNotDefaultEntryState.Idle -> AncestorEntryIsNotDefaultEntryState.Watch
        is AncestorEntryIsNotDefaultEntryState.Lobby -> AncestorEntryIsNotDefaultEntryState.Drive
        is AncestorEntryIsNotDefaultEntryState.Outer -> AncestorEntryIsNotDefaultEntryState.Drive
        is AncestorEntryIsNotDefaultEntryState.Watch -> AncestorEntryIsNotDefaultEntryState.Run
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: AncestorEntryIsNotDefaultEntryState): Boolean = when (state) {
        is AncestorEntryIsNotDefaultEntryState.Drive -> true
        is AncestorEntryIsNotDefaultEntryState.Outer -> true
        is AncestorEntryIsNotDefaultEntryState.Watch -> true
        else -> false
    }


    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: AncestorEntryIsNotDefaultEntryState): Boolean = when (state) {
        is AncestorEntryIsNotDefaultEntryState.Run -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: AncestorEntryIsNotDefaultEntryState): Boolean = when (state) {
        is AncestorEntryIsNotDefaultEntryState.FailDefaulted -> true
        is AncestorEntryIsNotDefaultEntryState.FailIdled -> true
        is AncestorEntryIsNotDefaultEntryState.FailLobbied -> true
        is AncestorEntryIsNotDefaultEntryState.FailTargeted -> true
        is AncestorEntryIsNotDefaultEntryState.Settled -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: AncestorEntryIsNotDefaultEntryState): List<AncestorEntryIsNotDefaultEntryState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: AncestorEntryIsNotDefaultEntryState): List<EntryTarget<AncestorEntryIsNotDefaultEntryState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AncestorEntryIsNotDefaultEntryState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<AncestorEntryIsNotDefaultEntryState, List<AncestorEntryIsNotDefaultEntryState>> = LinkedHashMap<AncestorEntryIsNotDefaultEntryState, List<AncestorEntryIsNotDefaultEntryState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<AncestorEntryIsNotDefaultEntryState, List<AncestorEntryIsNotDefaultEntryState>>) {
            table[AncestorEntryIsNotDefaultEntryState.Drive] = listOf(AncestorEntryIsNotDefaultEntryState.Lobby, AncestorEntryIsNotDefaultEntryState.Outer)
            table[AncestorEntryIsNotDefaultEntryState.Outer] = listOf(AncestorEntryIsNotDefaultEntryState.ByDefault, AncestorEntryIsNotDefaultEntryState.Chosen)
            table[AncestorEntryIsNotDefaultEntryState.Run] = listOf(AncestorEntryIsNotDefaultEntryState.Drive, AncestorEntryIsNotDefaultEntryState.Watch)
            table[AncestorEntryIsNotDefaultEntryState.Watch] = listOf(AncestorEntryIsNotDefaultEntryState.Idle)
        }


        val initialTargets: Map<AncestorEntryIsNotDefaultEntryState, List<EntryTarget<AncestorEntryIsNotDefaultEntryState, HistoryId>>> = LinkedHashMap<AncestorEntryIsNotDefaultEntryState, List<EntryTarget<AncestorEntryIsNotDefaultEntryState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<AncestorEntryIsNotDefaultEntryState, List<EntryTarget<AncestorEntryIsNotDefaultEntryState, HistoryId>>>) {
            table[AncestorEntryIsNotDefaultEntryState.Drive] = listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.Lobby))
            table[AncestorEntryIsNotDefaultEntryState.Outer] = listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.ByDefault))
            table[AncestorEntryIsNotDefaultEntryState.Watch] = listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.Idle))
        }


        val documentInitialTargetList: List<EntryTarget<AncestorEntryIsNotDefaultEntryState, HistoryId>> =
            listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.Away))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: away's transition 0, as the microstep reads it.
        val transitionAwayAt0 = EnabledTransition<AncestorEntryIsNotDefaultEntryState, HistoryId>(
            AncestorEntryIsNotDefaultEntryState.Away,
            listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.Chosen)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: chosen's transition 0, as the microstep reads it.
        val transitionChosenAt0 = EnabledTransition<AncestorEntryIsNotDefaultEntryState, HistoryId>(
            AncestorEntryIsNotDefaultEntryState.Chosen,
            listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.Lobby)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: chosen's transition 1, as the microstep reads it.
        val transitionChosenAt1 = EnabledTransition<AncestorEntryIsNotDefaultEntryState, HistoryId>(
            AncestorEntryIsNotDefaultEntryState.Chosen,
            listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.FailDefaulted)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: chosen's transition 2, as the microstep reads it.
        val transitionChosenAt2 = EnabledTransition<AncestorEntryIsNotDefaultEntryState, HistoryId>(
            AncestorEntryIsNotDefaultEntryState.Chosen,
            listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.FailLobbied)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: chosen's transition 3, as the microstep reads it.
        val transitionChosenAt3 = EnabledTransition<AncestorEntryIsNotDefaultEntryState, HistoryId>(
            AncestorEntryIsNotDefaultEntryState.Chosen,
            listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.FailIdled)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: chosen's transition 4, as the microstep reads it.
        val transitionChosenAt4 = EnabledTransition<AncestorEntryIsNotDefaultEntryState, HistoryId>(
            AncestorEntryIsNotDefaultEntryState.Chosen,
            listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.FailTargeted)),
            4,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: chosen's transition 5, as the microstep reads it.
        val transitionChosenAt5 = EnabledTransition<AncestorEntryIsNotDefaultEntryState, HistoryId>(
            AncestorEntryIsNotDefaultEntryState.Chosen,
            listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.Settled)),
            5,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: lobby's transition 0, as the microstep reads it.
        val transitionLobbyAt0 = EnabledTransition<AncestorEntryIsNotDefaultEntryState, HistoryId>(
            AncestorEntryIsNotDefaultEntryState.Lobby,
            listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.Chosen)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AncestorEntryIsNotDefaultEntryState? = when (stateId) {
        "away" -> AncestorEntryIsNotDefaultEntryState.Away
        "by_default" -> AncestorEntryIsNotDefaultEntryState.ByDefault
        "chosen" -> AncestorEntryIsNotDefaultEntryState.Chosen
        "drive" -> AncestorEntryIsNotDefaultEntryState.Drive
        "failDefaulted" -> AncestorEntryIsNotDefaultEntryState.FailDefaulted
        "failIdled" -> AncestorEntryIsNotDefaultEntryState.FailIdled
        "failLobbied" -> AncestorEntryIsNotDefaultEntryState.FailLobbied
        "failTargeted" -> AncestorEntryIsNotDefaultEntryState.FailTargeted
        "idle" -> AncestorEntryIsNotDefaultEntryState.Idle
        "lobby" -> AncestorEntryIsNotDefaultEntryState.Lobby
        "outer" -> AncestorEntryIsNotDefaultEntryState.Outer
        "run" -> AncestorEntryIsNotDefaultEntryState.Run
        "settled" -> AncestorEntryIsNotDefaultEntryState.Settled
        "watch" -> AncestorEntryIsNotDefaultEntryState.Watch
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AncestorEntryIsNotDefaultEntryState): String = when (state) {
        is AncestorEntryIsNotDefaultEntryState.Away -> "away"
        is AncestorEntryIsNotDefaultEntryState.ByDefault -> "by_default"
        is AncestorEntryIsNotDefaultEntryState.Chosen -> "chosen"
        is AncestorEntryIsNotDefaultEntryState.Drive -> "drive"
        is AncestorEntryIsNotDefaultEntryState.FailDefaulted -> "failDefaulted"
        is AncestorEntryIsNotDefaultEntryState.FailIdled -> "failIdled"
        is AncestorEntryIsNotDefaultEntryState.FailLobbied -> "failLobbied"
        is AncestorEntryIsNotDefaultEntryState.FailTargeted -> "failTargeted"
        is AncestorEntryIsNotDefaultEntryState.Idle -> "idle"
        is AncestorEntryIsNotDefaultEntryState.Lobby -> "lobby"
        is AncestorEntryIsNotDefaultEntryState.Outer -> "outer"
        is AncestorEntryIsNotDefaultEntryState.Run -> "run"
        is AncestorEntryIsNotDefaultEntryState.Settled -> "settled"
        is AncestorEntryIsNotDefaultEntryState.Watch -> "watch"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AncestorEntryIsNotDefaultEntryState): Int = when (state) {
        is AncestorEntryIsNotDefaultEntryState.Away -> 0
        is AncestorEntryIsNotDefaultEntryState.ByDefault -> 5
        is AncestorEntryIsNotDefaultEntryState.Chosen -> 6
        is AncestorEntryIsNotDefaultEntryState.Drive -> 2
        is AncestorEntryIsNotDefaultEntryState.FailDefaulted -> 10
        is AncestorEntryIsNotDefaultEntryState.FailIdled -> 12
        is AncestorEntryIsNotDefaultEntryState.FailLobbied -> 11
        is AncestorEntryIsNotDefaultEntryState.FailTargeted -> 13
        is AncestorEntryIsNotDefaultEntryState.Idle -> 8
        is AncestorEntryIsNotDefaultEntryState.Lobby -> 3
        is AncestorEntryIsNotDefaultEntryState.Outer -> 4
        is AncestorEntryIsNotDefaultEntryState.Run -> 1
        is AncestorEntryIsNotDefaultEntryState.Settled -> 9
        is AncestorEntryIsNotDefaultEntryState.Watch -> 7
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AncestorEntryIsNotDefaultEntryEvent? = when (name) {
        "again" -> AncestorEntryIsNotDefaultEntryEvent.Again
        "back" -> AncestorEntryIsNotDefaultEntryEvent.Back
        "check" -> AncestorEntryIsNotDefaultEntryEvent.Check
        "cross" -> AncestorEntryIsNotDefaultEntryEvent.Cross
        "error.execution" -> AncestorEntryIsNotDefaultEntryEvent.Error.Execution
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AncestorEntryIsNotDefaultEntryEvent): String? = when (event) {
        is AncestorEntryIsNotDefaultEntryEvent.Again -> "again"
        is AncestorEntryIsNotDefaultEntryEvent.Back -> "back"
        is AncestorEntryIsNotDefaultEntryEvent.Check -> "check"
        is AncestorEntryIsNotDefaultEntryEvent.Cross -> "cross"
        is AncestorEntryIsNotDefaultEntryEvent.Error.Execution -> "error.execution"
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
            "ancestor_entry_is_not_default_entry",
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
        // W3C SCXML 5.3: Initialize variable 'defaulted' with expr
        try {
            val initResult_defaulted = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "defaulted", initResult_defaulted)
        } catch (e: Exception) {
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<data id='defaulted'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'lobbied' with expr
        try {
            val initResult_lobbied = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "lobbied", initResult_lobbied)
        } catch (e: Exception) {
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<data id='lobbied'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'idled' with expr
        try {
            val initResult_idled = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "idled", initResult_idled)
        } catch (e: Exception) {
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<data id='idled'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'targeted' with expr
        try {
            val initResult_targeted = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "targeted", initResult_targeted)
        } catch (e: Exception) {
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<data id='targeted'> expr failed to evaluate")
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
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, reason)
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
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AncestorEntryIsNotDefaultEntryEvent) {
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
    override fun bindCurrentEvent(event: AncestorEntryIsNotDefaultEntryEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: AncestorEntryIsNotDefaultEntryState, event: AncestorEntryIsNotDefaultEntryEvent?): EnabledTransition<AncestorEntryIsNotDefaultEntryState, HistoryId>? = when (state) {
        is AncestorEntryIsNotDefaultEntryState.Away -> firstEnabledTransitionOfAway(state, event)
        is AncestorEntryIsNotDefaultEntryState.Chosen -> firstEnabledTransitionOfChosen(state, event)
        is AncestorEntryIsNotDefaultEntryState.Lobby -> firstEnabledTransitionOfLobby(state, event)
        else -> null
    }


    // away's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAway(state: AncestorEntryIsNotDefaultEntryState, event: AncestorEntryIsNotDefaultEntryEvent?): EnabledTransition<AncestorEntryIsNotDefaultEntryState, HistoryId>? = when {
        event is AncestorEntryIsNotDefaultEntryEvent.Cross -> TransitionsPart0.transitionAwayAt0
        else -> null
    }


    // chosen's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfChosen(state: AncestorEntryIsNotDefaultEntryState, event: AncestorEntryIsNotDefaultEntryEvent?): EnabledTransition<AncestorEntryIsNotDefaultEntryState, HistoryId>? = when {
        event is AncestorEntryIsNotDefaultEntryEvent.Back -> TransitionsPart0.transitionChosenAt0
        event is AncestorEntryIsNotDefaultEntryEvent.Check && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(not _scxml_eq(defaulted, 0))", "defaulted != 0")) -> TransitionsPart0.transitionChosenAt1
        event is AncestorEntryIsNotDefaultEntryEvent.Check && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(not _scxml_eq(lobbied, 1))", "lobbied != 1")) -> TransitionsPart0.transitionChosenAt2
        event is AncestorEntryIsNotDefaultEntryEvent.Check && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(not _scxml_eq(idled, 1))", "idled != 1")) -> TransitionsPart0.transitionChosenAt3
        event is AncestorEntryIsNotDefaultEntryEvent.Check && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(not _scxml_eq(targeted, 2))", "targeted != 2")) -> TransitionsPart0.transitionChosenAt4
        event is AncestorEntryIsNotDefaultEntryEvent.Check -> TransitionsPart0.transitionChosenAt5
        else -> null
    }


    // lobby's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfLobby(state: AncestorEntryIsNotDefaultEntryState, event: AncestorEntryIsNotDefaultEntryEvent?): EnabledTransition<AncestorEntryIsNotDefaultEntryState, HistoryId>? = when {
        event is AncestorEntryIsNotDefaultEntryEvent.Again -> TransitionsPart0.transitionLobbyAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:69 :: _machine
    override fun onEntry(state: AncestorEntryIsNotDefaultEntryState, isDefaultEntry: Boolean) {
        when (state) {
            is AncestorEntryIsNotDefaultEntryState.Away -> onEntryOfAway()
            is AncestorEntryIsNotDefaultEntryState.ByDefault -> onEntryOfByDefault()
            is AncestorEntryIsNotDefaultEntryState.Chosen -> onEntryOfChosen()
            is AncestorEntryIsNotDefaultEntryState.Drive -> onEntryOfDrive()
            is AncestorEntryIsNotDefaultEntryState.FailDefaulted -> onEntryOfFailDefaulted()
            is AncestorEntryIsNotDefaultEntryState.FailIdled -> onEntryOfFailIdled()
            is AncestorEntryIsNotDefaultEntryState.FailLobbied -> onEntryOfFailLobbied()
            is AncestorEntryIsNotDefaultEntryState.FailTargeted -> onEntryOfFailTargeted()
            is AncestorEntryIsNotDefaultEntryState.Idle -> onEntryOfIdle()
            is AncestorEntryIsNotDefaultEntryState.Lobby -> onEntryOfLobby()
            is AncestorEntryIsNotDefaultEntryState.Outer -> onEntryOfOuter()
            is AncestorEntryIsNotDefaultEntryState.Run -> onEntryOfRun()
            is AncestorEntryIsNotDefaultEntryState.Settled -> onEntryOfSettled()
            is AncestorEntryIsNotDefaultEntryState.Watch -> onEntryOfWatch()
        }
    }


    private fun onEntryOfAway() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:89 :: away :: _state_body
    }

    private fun onEntryOfByDefault() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:116 :: by_default :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("defaulted", "defaulted"), com.sce.runtime.ScriptSource.lua("_scxml_add(defaulted, 1)", "defaulted + 1"))) {
                return@run
            }
                }
    }

    private fun onEntryOfChosen() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:127 :: chosen :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("targeted", "targeted"), com.sce.runtime.ScriptSource.lua("_scxml_add(targeted, 1)", "targeted + 1"))) {
                return@run
            }
                }
    }

    private fun onEntryOfDrive() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:95 :: drive :: _state_body
    }

    private fun onEntryOfFailDefaulted() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:160 :: failDefaulted :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailIdled() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:162 :: failIdled :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailLobbied() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:161 :: failLobbied :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailTargeted() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:163 :: failTargeted :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfIdle() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:150 :: idle :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("idled", "idled"), com.sce.runtime.ScriptSource.lua("_scxml_add(idled, 1)", "idled + 1"))) {
                return@run
            }
                }
    }

    private fun onEntryOfLobby() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:102 :: lobby :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("lobbied", "lobbied"), com.sce.runtime.ScriptSource.lua("_scxml_add(lobbied, 1)", "lobbied + 1"))) {
                return@run
            }
                }
    }

    private fun onEntryOfOuter() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:109 :: outer :: _state_body
    }

    private fun onEntryOfRun() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:93 :: run :: _state_body
    }

    private fun onEntryOfSettled() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:159 :: settled :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfWatch() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:149 :: watch :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:69 :: _machine
    override fun onExit(state: AncestorEntryIsNotDefaultEntryState) {
        when (state) {
            is AncestorEntryIsNotDefaultEntryState.Away -> onExitOfAway()
            is AncestorEntryIsNotDefaultEntryState.ByDefault -> onExitOfByDefault()
            is AncestorEntryIsNotDefaultEntryState.Chosen -> onExitOfChosen()
            is AncestorEntryIsNotDefaultEntryState.Drive -> onExitOfDrive()
            is AncestorEntryIsNotDefaultEntryState.FailDefaulted -> onExitOfFailDefaulted()
            is AncestorEntryIsNotDefaultEntryState.FailIdled -> onExitOfFailIdled()
            is AncestorEntryIsNotDefaultEntryState.FailLobbied -> onExitOfFailLobbied()
            is AncestorEntryIsNotDefaultEntryState.FailTargeted -> onExitOfFailTargeted()
            is AncestorEntryIsNotDefaultEntryState.Idle -> onExitOfIdle()
            is AncestorEntryIsNotDefaultEntryState.Lobby -> onExitOfLobby()
            is AncestorEntryIsNotDefaultEntryState.Outer -> onExitOfOuter()
            is AncestorEntryIsNotDefaultEntryState.Run -> onExitOfRun()
            is AncestorEntryIsNotDefaultEntryState.Settled -> onExitOfSettled()
            is AncestorEntryIsNotDefaultEntryState.Watch -> onExitOfWatch()
        }
    }


    private fun onExitOfAway() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:89 :: away :: _state_body
    }

    private fun onExitOfByDefault() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:116 :: by_default :: _state_body
    }

    private fun onExitOfChosen() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:127 :: chosen :: _state_body
    }

    private fun onExitOfDrive() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:95 :: drive :: _state_body
    }

    private fun onExitOfFailDefaulted() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:160 :: failDefaulted :: _state_body
    }

    private fun onExitOfFailIdled() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:162 :: failIdled :: _state_body
    }

    private fun onExitOfFailLobbied() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:161 :: failLobbied :: _state_body
    }

    private fun onExitOfFailTargeted() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:163 :: failTargeted :: _state_body
    }

    private fun onExitOfIdle() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:150 :: idle :: _state_body
    }

    private fun onExitOfLobby() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:102 :: lobby :: _state_body
    }

    private fun onExitOfOuter() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:109 :: outer :: _state_body
    }

    private fun onExitOfRun() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:93 :: run :: _state_body
    }

    private fun onExitOfSettled() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:159 :: settled :: _state_body
    }

    private fun onExitOfWatch() {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:149 :: watch :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:69 :: _machine
    override fun executeTransitionContent(source: AncestorEntryIsNotDefaultEntryState, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
