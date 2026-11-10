// SCE-GENERATED — DO NOT EDIT
// source-hash: 29697707f7a74196e127bf6f7db189bad27f8db18c5d042ec16e1ad0e6f1d60a

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
    data object Wildcard : WildcardInDocumentOrderEvent
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

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

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


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: WildcardInDocumentOrderState): Boolean = when (state) {
        is WildcardInDocumentOrderState.GuardClosed -> true
        is WildcardInDocumentOrderState.GuardedInternal -> true
        is WildcardInDocumentOrderState.GuardOpen -> true
        is WildcardInDocumentOrderState.SealedInternal -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: WildcardInDocumentOrderState): Boolean = when (state) {
        is WildcardInDocumentOrderState.FailGuardedInternalReentered -> true
        is WildcardInDocumentOrderState.FailGuardIgnored -> true
        is WildcardInDocumentOrderState.FailGuardNeverFired -> true
        is WildcardInDocumentOrderState.FailSealedInternalReentered -> true
        is WildcardInDocumentOrderState.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: WildcardInDocumentOrderState): List<WildcardInDocumentOrderState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: WildcardInDocumentOrderState): List<EntryTarget<WildcardInDocumentOrderState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<WildcardInDocumentOrderState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<WildcardInDocumentOrderState, List<WildcardInDocumentOrderState>> = LinkedHashMap<WildcardInDocumentOrderState, List<WildcardInDocumentOrderState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<WildcardInDocumentOrderState, List<WildcardInDocumentOrderState>>) {
            table[WildcardInDocumentOrderState.GuardClosed] = listOf(WildcardInDocumentOrderState.GuardClosedLeaf)
            table[WildcardInDocumentOrderState.GuardedInternal] = listOf(WildcardInDocumentOrderState.GuardedFrom, WildcardInDocumentOrderState.GuardedTo)
            table[WildcardInDocumentOrderState.GuardOpen] = listOf(WildcardInDocumentOrderState.GuardOpenLeaf)
            table[WildcardInDocumentOrderState.SealedInternal] = listOf(WildcardInDocumentOrderState.SealedFrom, WildcardInDocumentOrderState.SealedTo)
        }


        val initialTargets: Map<WildcardInDocumentOrderState, List<EntryTarget<WildcardInDocumentOrderState, HistoryId>>> = LinkedHashMap<WildcardInDocumentOrderState, List<EntryTarget<WildcardInDocumentOrderState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<WildcardInDocumentOrderState, List<EntryTarget<WildcardInDocumentOrderState, HistoryId>>>) {
            table[WildcardInDocumentOrderState.GuardClosed] = listOf(StateTarget(WildcardInDocumentOrderState.GuardClosedLeaf))
            table[WildcardInDocumentOrderState.GuardedInternal] = listOf(StateTarget(WildcardInDocumentOrderState.GuardedFrom))
            table[WildcardInDocumentOrderState.GuardOpen] = listOf(StateTarget(WildcardInDocumentOrderState.GuardOpenLeaf))
            table[WildcardInDocumentOrderState.SealedInternal] = listOf(StateTarget(WildcardInDocumentOrderState.SealedFrom))
        }


        val documentInitialTargetList: List<EntryTarget<WildcardInDocumentOrderState, HistoryId>> =
            listOf(StateTarget(WildcardInDocumentOrderState.GuardClosed))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: guardClosed's transition 0, as the microstep reads it.
        val transitionGuardClosedAt0 = EnabledTransition<WildcardInDocumentOrderState, HistoryId>(
            WildcardInDocumentOrderState.GuardClosed,
            listOf(StateTarget(WildcardInDocumentOrderState.GuardOpen)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: guardClosedLeaf's transition 0, as the microstep reads it.
        val transitionGuardClosedLeafAt0 = EnabledTransition<WildcardInDocumentOrderState, HistoryId>(
            WildcardInDocumentOrderState.GuardClosedLeaf,
            listOf(StateTarget(WildcardInDocumentOrderState.FailGuardIgnored)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: guardOpen's transition 0, as the microstep reads it.
        val transitionGuardOpenAt0 = EnabledTransition<WildcardInDocumentOrderState, HistoryId>(
            WildcardInDocumentOrderState.GuardOpen,
            listOf(StateTarget(WildcardInDocumentOrderState.FailGuardNeverFired)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: guardOpenLeaf's transition 0, as the microstep reads it.
        val transitionGuardOpenLeafAt0 = EnabledTransition<WildcardInDocumentOrderState, HistoryId>(
            WildcardInDocumentOrderState.GuardOpenLeaf,
            listOf(StateTarget(WildcardInDocumentOrderState.GuardedInternal)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: guardedInternal's transition 0, as the microstep reads it.
        val transitionGuardedInternalAt0 = EnabledTransition<WildcardInDocumentOrderState, HistoryId>(
            WildcardInDocumentOrderState.GuardedInternal,
            listOf(StateTarget(WildcardInDocumentOrderState.GuardedTo)),
            0,
            hasActions = false,
            isInternal = true,
        )

        // W3C SCXML 3.13: guardedTo's transition 0, as the microstep reads it.
        val transitionGuardedToAt0 = EnabledTransition<WildcardInDocumentOrderState, HistoryId>(
            WildcardInDocumentOrderState.GuardedTo,
            listOf(StateTarget(WildcardInDocumentOrderState.SealedInternal)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: guardedTo's transition 1, as the microstep reads it.
        val transitionGuardedToAt1 = EnabledTransition<WildcardInDocumentOrderState, HistoryId>(
            WildcardInDocumentOrderState.GuardedTo,
            listOf(StateTarget(WildcardInDocumentOrderState.FailGuardedInternalReentered)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: sealedInternal's transition 0, as the microstep reads it.
        val transitionSealedInternalAt0 = EnabledTransition<WildcardInDocumentOrderState, HistoryId>(
            WildcardInDocumentOrderState.SealedInternal,
            listOf(StateTarget(WildcardInDocumentOrderState.SealedTo)),
            0,
            hasActions = false,
            isInternal = true,
        )

        // W3C SCXML 3.13: sealedTo's transition 0, as the microstep reads it.
        val transitionSealedToAt0 = EnabledTransition<WildcardInDocumentOrderState, HistoryId>(
            WildcardInDocumentOrderState.SealedTo,
            listOf(StateTarget(WildcardInDocumentOrderState.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: sealedTo's transition 1, as the microstep reads it.
        val transitionSealedToAt1 = EnabledTransition<WildcardInDocumentOrderState, HistoryId>(
            WildcardInDocumentOrderState.SealedTo,
            listOf(StateTarget(WildcardInDocumentOrderState.FailSealedInternalReentered)),
            1,
            hasActions = false,
            isInternal = false,
        )
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


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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
        "*" -> WildcardInDocumentOrderEvent.Wildcard
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: WildcardInDocumentOrderEvent): String? = when (event) {
        is WildcardInDocumentOrderEvent.Error.Execution -> "error.execution"
        is WildcardInDocumentOrderEvent.Hop -> "hop"
        is WildcardInDocumentOrderEvent.Probe -> "probe"
        is WildcardInDocumentOrderEvent.Wildcard -> "*"
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
            "wildcard_in_document_order",
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
            raisePlatformError(WildcardInDocumentOrderEvent.Error.Execution, reason)
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
            raisePlatformError(WildcardInDocumentOrderEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(WildcardInDocumentOrderEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: WildcardInDocumentOrderEvent) {
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
    override fun bindCurrentEvent(event: WildcardInDocumentOrderEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: WildcardInDocumentOrderState, event: WildcardInDocumentOrderEvent?): EnabledTransition<WildcardInDocumentOrderState, HistoryId>? = when (state) {
        is WildcardInDocumentOrderState.GuardClosed -> firstEnabledTransitionOfGuardClosed(state, event)
        is WildcardInDocumentOrderState.GuardClosedLeaf -> firstEnabledTransitionOfGuardClosedLeaf(state, event)
        is WildcardInDocumentOrderState.GuardedInternal -> firstEnabledTransitionOfGuardedInternal(state, event)
        is WildcardInDocumentOrderState.GuardedTo -> firstEnabledTransitionOfGuardedTo(state, event)
        is WildcardInDocumentOrderState.GuardOpen -> firstEnabledTransitionOfGuardOpen(state, event)
        is WildcardInDocumentOrderState.GuardOpenLeaf -> firstEnabledTransitionOfGuardOpenLeaf(state, event)
        is WildcardInDocumentOrderState.SealedInternal -> firstEnabledTransitionOfSealedInternal(state, event)
        is WildcardInDocumentOrderState.SealedTo -> firstEnabledTransitionOfSealedTo(state, event)
        else -> null
    }


    // guardClosed's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfGuardClosed(state: WildcardInDocumentOrderState, event: WildcardInDocumentOrderEvent?): EnabledTransition<WildcardInDocumentOrderState, HistoryId>? = when {
        event is WildcardInDocumentOrderEvent.Probe -> TransitionsPart0.transitionGuardClosedAt0
        else -> null
    }


    // guardClosedLeaf's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfGuardClosedLeaf(state: WildcardInDocumentOrderState, event: WildcardInDocumentOrderEvent?): EnabledTransition<WildcardInDocumentOrderState, HistoryId>? = when {
        event != null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_truthy(armed)", "armed")) -> TransitionsPart0.transitionGuardClosedLeafAt0
        else -> null
    }


    // guardedInternal's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfGuardedInternal(state: WildcardInDocumentOrderState, event: WildcardInDocumentOrderEvent?): EnabledTransition<WildcardInDocumentOrderState, HistoryId>? = when {
        event != null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_truthy(armed)", "armed")) -> TransitionsPart0.transitionGuardedInternalAt0
        else -> null
    }


    // guardedTo's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfGuardedTo(state: WildcardInDocumentOrderState, event: WildcardInDocumentOrderEvent?): EnabledTransition<WildcardInDocumentOrderState, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(guardedEntries, 1)", "guardedEntries == 1")) -> TransitionsPart0.transitionGuardedToAt0
        event == null -> TransitionsPart0.transitionGuardedToAt1
        else -> null
    }


    // guardOpen's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfGuardOpen(state: WildcardInDocumentOrderState, event: WildcardInDocumentOrderEvent?): EnabledTransition<WildcardInDocumentOrderState, HistoryId>? = when {
        event is WildcardInDocumentOrderEvent.Probe -> TransitionsPart0.transitionGuardOpenAt0
        else -> null
    }


    // guardOpenLeaf's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfGuardOpenLeaf(state: WildcardInDocumentOrderState, event: WildcardInDocumentOrderEvent?): EnabledTransition<WildcardInDocumentOrderState, HistoryId>? = when {
        event != null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_truthy(armed)", "armed")) -> TransitionsPart0.transitionGuardOpenLeafAt0
        else -> null
    }


    // sealedInternal's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSealedInternal(state: WildcardInDocumentOrderState, event: WildcardInDocumentOrderEvent?): EnabledTransition<WildcardInDocumentOrderState, HistoryId>? = when {
        event != null -> TransitionsPart0.transitionSealedInternalAt0
        else -> null
    }


    // sealedTo's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSealedTo(state: WildcardInDocumentOrderState, event: WildcardInDocumentOrderEvent?): EnabledTransition<WildcardInDocumentOrderState, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(sealedEntries, 1)", "sealedEntries == 1")) -> TransitionsPart0.transitionSealedToAt0
        event == null -> TransitionsPart0.transitionSealedToAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: wildcard_in_document_order.scxml:48 :: _machine
    override fun onEntry(state: WildcardInDocumentOrderState, isDefaultEntry: Boolean) {
        when (state) {
            is WildcardInDocumentOrderState.FailGuardedInternalReentered -> onEntryOfFailGuardedInternalReentered()
            is WildcardInDocumentOrderState.FailGuardIgnored -> onEntryOfFailGuardIgnored()
            is WildcardInDocumentOrderState.FailGuardNeverFired -> onEntryOfFailGuardNeverFired()
            is WildcardInDocumentOrderState.FailSealedInternalReentered -> onEntryOfFailSealedInternalReentered()
            is WildcardInDocumentOrderState.GuardClosed -> onEntryOfGuardClosed()
            is WildcardInDocumentOrderState.GuardClosedLeaf -> onEntryOfGuardClosedLeaf()
            is WildcardInDocumentOrderState.GuardedFrom -> onEntryOfGuardedFrom()
            is WildcardInDocumentOrderState.GuardedInternal -> onEntryOfGuardedInternal()
            is WildcardInDocumentOrderState.GuardedTo -> onEntryOfGuardedTo()
            is WildcardInDocumentOrderState.GuardOpen -> onEntryOfGuardOpen()
            is WildcardInDocumentOrderState.GuardOpenLeaf -> onEntryOfGuardOpenLeaf()
            is WildcardInDocumentOrderState.Pass -> onEntryOfPass()
            is WildcardInDocumentOrderState.SealedFrom -> onEntryOfSealedFrom()
            is WildcardInDocumentOrderState.SealedInternal -> onEntryOfSealedInternal()
            is WildcardInDocumentOrderState.SealedTo -> onEntryOfSealedTo()
        }
    }


    private fun onEntryOfFailGuardedInternalReentered() {
                // SCE-MAP: wildcard_in_document_order.scxml:114 :: failGuardedInternalReentered :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailGuardIgnored() {
                // SCE-MAP: wildcard_in_document_order.scxml:112 :: failGuardIgnored :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailGuardNeverFired() {
                // SCE-MAP: wildcard_in_document_order.scxml:113 :: failGuardNeverFired :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailSealedInternalReentered() {
                // SCE-MAP: wildcard_in_document_order.scxml:115 :: failSealedInternalReentered :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfGuardClosed() {
                // SCE-MAP: wildcard_in_document_order.scxml:64 :: guardClosed :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            raiseInternal(WildcardInDocumentOrderEvent.Probe)
                }
    }

    private fun onEntryOfGuardClosedLeaf() {
                // SCE-MAP: wildcard_in_document_order.scxml:69 :: guardClosedLeaf :: _state_body
    }

    private fun onEntryOfGuardedFrom() {
                // SCE-MAP: wildcard_in_document_order.scxml:91 :: guardedFrom :: _state_body
    }

    private fun onEntryOfGuardedInternal() {
                // SCE-MAP: wildcard_in_document_order.scxml:85 :: guardedInternal :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("guardedEntries", "guardedEntries"), com.sce.runtime.ScriptSource.lua("_scxml_add(guardedEntries, 1)", "guardedEntries + 1"))) {
                return@run
            }

            raiseInternal(WildcardInDocumentOrderEvent.Hop)
                }
    }

    private fun onEntryOfGuardedTo() {
                // SCE-MAP: wildcard_in_document_order.scxml:92 :: guardedTo :: _state_body
    }

    private fun onEntryOfGuardOpen() {
                // SCE-MAP: wildcard_in_document_order.scxml:74 :: guardOpen :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("armed", "armed"), com.sce.runtime.ScriptSource.lua("true", "true"))) {
                return@run
            }

            raiseInternal(WildcardInDocumentOrderEvent.Probe)
                }
    }

    private fun onEntryOfGuardOpenLeaf() {
                // SCE-MAP: wildcard_in_document_order.scxml:80 :: guardOpenLeaf :: _state_body
    }

    private fun onEntryOfPass() {
                // SCE-MAP: wildcard_in_document_order.scxml:111 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfSealedFrom() {
                // SCE-MAP: wildcard_in_document_order.scxml:104 :: sealedFrom :: _state_body
    }

    private fun onEntryOfSealedInternal() {
                // SCE-MAP: wildcard_in_document_order.scxml:98 :: sealedInternal :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("sealedEntries", "sealedEntries"), com.sce.runtime.ScriptSource.lua("_scxml_add(sealedEntries, 1)", "sealedEntries + 1"))) {
                return@run
            }

            raiseInternal(WildcardInDocumentOrderEvent.Hop)
                }
    }

    private fun onEntryOfSealedTo() {
                // SCE-MAP: wildcard_in_document_order.scxml:105 :: sealedTo :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: wildcard_in_document_order.scxml:48 :: _machine
    override fun onExit(state: WildcardInDocumentOrderState) {
        when (state) {
            is WildcardInDocumentOrderState.FailGuardedInternalReentered -> onExitOfFailGuardedInternalReentered()
            is WildcardInDocumentOrderState.FailGuardIgnored -> onExitOfFailGuardIgnored()
            is WildcardInDocumentOrderState.FailGuardNeverFired -> onExitOfFailGuardNeverFired()
            is WildcardInDocumentOrderState.FailSealedInternalReentered -> onExitOfFailSealedInternalReentered()
            is WildcardInDocumentOrderState.GuardClosed -> onExitOfGuardClosed()
            is WildcardInDocumentOrderState.GuardClosedLeaf -> onExitOfGuardClosedLeaf()
            is WildcardInDocumentOrderState.GuardedFrom -> onExitOfGuardedFrom()
            is WildcardInDocumentOrderState.GuardedInternal -> onExitOfGuardedInternal()
            is WildcardInDocumentOrderState.GuardedTo -> onExitOfGuardedTo()
            is WildcardInDocumentOrderState.GuardOpen -> onExitOfGuardOpen()
            is WildcardInDocumentOrderState.GuardOpenLeaf -> onExitOfGuardOpenLeaf()
            is WildcardInDocumentOrderState.Pass -> onExitOfPass()
            is WildcardInDocumentOrderState.SealedFrom -> onExitOfSealedFrom()
            is WildcardInDocumentOrderState.SealedInternal -> onExitOfSealedInternal()
            is WildcardInDocumentOrderState.SealedTo -> onExitOfSealedTo()
        }
    }


    private fun onExitOfFailGuardedInternalReentered() {
                // SCE-MAP: wildcard_in_document_order.scxml:114 :: failGuardedInternalReentered :: _state_body
    }

    private fun onExitOfFailGuardIgnored() {
                // SCE-MAP: wildcard_in_document_order.scxml:112 :: failGuardIgnored :: _state_body
    }

    private fun onExitOfFailGuardNeverFired() {
                // SCE-MAP: wildcard_in_document_order.scxml:113 :: failGuardNeverFired :: _state_body
    }

    private fun onExitOfFailSealedInternalReentered() {
                // SCE-MAP: wildcard_in_document_order.scxml:115 :: failSealedInternalReentered :: _state_body
    }

    private fun onExitOfGuardClosed() {
                // SCE-MAP: wildcard_in_document_order.scxml:64 :: guardClosed :: _state_body
    }

    private fun onExitOfGuardClosedLeaf() {
                // SCE-MAP: wildcard_in_document_order.scxml:69 :: guardClosedLeaf :: _state_body
    }

    private fun onExitOfGuardedFrom() {
                // SCE-MAP: wildcard_in_document_order.scxml:91 :: guardedFrom :: _state_body
    }

    private fun onExitOfGuardedInternal() {
                // SCE-MAP: wildcard_in_document_order.scxml:85 :: guardedInternal :: _state_body
    }

    private fun onExitOfGuardedTo() {
                // SCE-MAP: wildcard_in_document_order.scxml:92 :: guardedTo :: _state_body
    }

    private fun onExitOfGuardOpen() {
                // SCE-MAP: wildcard_in_document_order.scxml:74 :: guardOpen :: _state_body
    }

    private fun onExitOfGuardOpenLeaf() {
                // SCE-MAP: wildcard_in_document_order.scxml:80 :: guardOpenLeaf :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: wildcard_in_document_order.scxml:111 :: pass :: _state_body
    }

    private fun onExitOfSealedFrom() {
                // SCE-MAP: wildcard_in_document_order.scxml:104 :: sealedFrom :: _state_body
    }

    private fun onExitOfSealedInternal() {
                // SCE-MAP: wildcard_in_document_order.scxml:98 :: sealedInternal :: _state_body
    }

    private fun onExitOfSealedTo() {
                // SCE-MAP: wildcard_in_document_order.scxml:105 :: sealedTo :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: wildcard_in_document_order.scxml:48 :: _machine
    override fun executeTransitionContent(source: WildcardInDocumentOrderState, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
