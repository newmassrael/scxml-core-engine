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
        is AncestorEntryIsNotDefaultEntryState.Drive, is AncestorEntryIsNotDefaultEntryState.Outer, is AncestorEntryIsNotDefaultEntryState.Watch -> true
        else -> false
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: AncestorEntryIsNotDefaultEntryState): Boolean = when (state) {
        is AncestorEntryIsNotDefaultEntryState.Run -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: AncestorEntryIsNotDefaultEntryState): Boolean = when (state) {
        is AncestorEntryIsNotDefaultEntryState.FailDefaulted, is AncestorEntryIsNotDefaultEntryState.FailIdled, is AncestorEntryIsNotDefaultEntryState.FailLobbied, is AncestorEntryIsNotDefaultEntryState.FailTargeted, is AncestorEntryIsNotDefaultEntryState.Settled -> true
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
        val childStates: Map<AncestorEntryIsNotDefaultEntryState, List<AncestorEntryIsNotDefaultEntryState>> = mapOf(
            AncestorEntryIsNotDefaultEntryState.Drive to listOf(AncestorEntryIsNotDefaultEntryState.Lobby, AncestorEntryIsNotDefaultEntryState.Outer),
            AncestorEntryIsNotDefaultEntryState.Outer to listOf(AncestorEntryIsNotDefaultEntryState.ByDefault, AncestorEntryIsNotDefaultEntryState.Chosen),
            AncestorEntryIsNotDefaultEntryState.Run to listOf(AncestorEntryIsNotDefaultEntryState.Drive, AncestorEntryIsNotDefaultEntryState.Watch),
            AncestorEntryIsNotDefaultEntryState.Watch to listOf(AncestorEntryIsNotDefaultEntryState.Idle),
        )

        val initialTargets: Map<AncestorEntryIsNotDefaultEntryState, List<EntryTarget<AncestorEntryIsNotDefaultEntryState, HistoryId>>> = mapOf(
            AncestorEntryIsNotDefaultEntryState.Drive to listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.Lobby)),
            AncestorEntryIsNotDefaultEntryState.Outer to listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.ByDefault)),
            AncestorEntryIsNotDefaultEntryState.Watch to listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.Idle)),
        )

        val documentInitialTargetList: List<EntryTarget<AncestorEntryIsNotDefaultEntryState, HistoryId>> =
            listOf(StateTarget(AncestorEntryIsNotDefaultEntryState.Away))

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
            "ancestor_entry_is_not_default_entry",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

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
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
    private fun executeAssign(location: com.sce.runtime.ScriptSource, expr: com.sce.runtime.ScriptSource) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AncestorEntryIsNotDefaultEntryEvent) {
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
    override fun bindCurrentEvent(event: AncestorEntryIsNotDefaultEntryEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AncestorEntryIsNotDefaultEntryState,
        event: AncestorEntryIsNotDefaultEntryEvent?
    ): EnabledTransition<AncestorEntryIsNotDefaultEntryState, HistoryId>? = when (state) {
        is AncestorEntryIsNotDefaultEntryState.Away -> when {
            event is AncestorEntryIsNotDefaultEntryEvent.Cross -> transitionAwayAt0
            else -> null
        }
        is AncestorEntryIsNotDefaultEntryState.Chosen -> when {
            event is AncestorEntryIsNotDefaultEntryEvent.Back -> transitionChosenAt0
            event is AncestorEntryIsNotDefaultEntryEvent.Check && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(not _scxml_eq(defaulted, 0))", "defaulted != 0")) -> transitionChosenAt1
            event is AncestorEntryIsNotDefaultEntryEvent.Check && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(not _scxml_eq(lobbied, 1))", "lobbied != 1")) -> transitionChosenAt2
            event is AncestorEntryIsNotDefaultEntryEvent.Check && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(not _scxml_eq(idled, 1))", "idled != 1")) -> transitionChosenAt3
            event is AncestorEntryIsNotDefaultEntryEvent.Check && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(not _scxml_eq(targeted, 2))", "targeted != 2")) -> transitionChosenAt4
            event is AncestorEntryIsNotDefaultEntryEvent.Check -> transitionChosenAt5
            else -> null
        }
        is AncestorEntryIsNotDefaultEntryState.Lobby -> when {
            event is AncestorEntryIsNotDefaultEntryEvent.Again -> transitionLobbyAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:69 :: _machine
    override fun onEntry(state: AncestorEntryIsNotDefaultEntryState, isDefaultEntry: Boolean) {
        when (state) {
            is AncestorEntryIsNotDefaultEntryState.Away -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:89 :: away :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.ByDefault -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:116 :: by_default :: _state_body


            executeAssign(com.sce.runtime.ScriptSource.lua("defaulted", "defaulted"), com.sce.runtime.ScriptSource.lua("_scxml_add(defaulted, 1)", "defaulted + 1"))
            }
            is AncestorEntryIsNotDefaultEntryState.Chosen -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:127 :: chosen :: _state_body


            executeAssign(com.sce.runtime.ScriptSource.lua("targeted", "targeted"), com.sce.runtime.ScriptSource.lua("_scxml_add(targeted, 1)", "targeted + 1"))
            }
            is AncestorEntryIsNotDefaultEntryState.Drive -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:95 :: drive :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.FailDefaulted -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:160 :: failDefaulted :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AncestorEntryIsNotDefaultEntryState.FailIdled -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:162 :: failIdled :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AncestorEntryIsNotDefaultEntryState.FailLobbied -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:161 :: failLobbied :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AncestorEntryIsNotDefaultEntryState.FailTargeted -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:163 :: failTargeted :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AncestorEntryIsNotDefaultEntryState.Idle -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:150 :: idle :: _state_body


            executeAssign(com.sce.runtime.ScriptSource.lua("idled", "idled"), com.sce.runtime.ScriptSource.lua("_scxml_add(idled, 1)", "idled + 1"))
            }
            is AncestorEntryIsNotDefaultEntryState.Lobby -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:102 :: lobby :: _state_body


            executeAssign(com.sce.runtime.ScriptSource.lua("lobbied", "lobbied"), com.sce.runtime.ScriptSource.lua("_scxml_add(lobbied, 1)", "lobbied + 1"))
            }
            is AncestorEntryIsNotDefaultEntryState.Outer -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:109 :: outer :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.Run -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:93 :: run :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.Settled -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:159 :: settled :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AncestorEntryIsNotDefaultEntryState.Watch -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:149 :: watch :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:69 :: _machine
    override fun onExit(state: AncestorEntryIsNotDefaultEntryState) {
        when (state) {
            is AncestorEntryIsNotDefaultEntryState.Away -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:89 :: away :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.ByDefault -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:116 :: by_default :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.Chosen -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:127 :: chosen :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.Drive -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:95 :: drive :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.FailDefaulted -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:160 :: failDefaulted :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.FailIdled -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:162 :: failIdled :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.FailLobbied -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:161 :: failLobbied :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.FailTargeted -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:163 :: failTargeted :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.Idle -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:150 :: idle :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.Lobby -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:102 :: lobby :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.Outer -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:109 :: outer :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.Run -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:93 :: run :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.Settled -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:159 :: settled :: _state_body
            }
            is AncestorEntryIsNotDefaultEntryState.Watch -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:149 :: watch :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:69 :: _machine
    override fun executeTransitionContent(source: AncestorEntryIsNotDefaultEntryState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
