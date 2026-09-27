// SCE-GENERATED — DO NOT EDIT
// source-hash: a2c2cff50069419be8be154770909e1b36918691efa802737abe7a86046c62a8

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/an_error_ends_the_block_it_was_raised_in/an_error_ends_the_block_it_was_raised_in.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: an_error_ends_the_block_it_was_raised_in.scxml:33 :: _machine

package com.sce.integration.an_error_ends_the_block_it_was_raised_in

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AnErrorEndsTheBlockItWasRaisedInState : State {
    data object Done : AnErrorEndsTheBlockItWasRaisedInState
    data object P : AnErrorEndsTheBlockItWasRaisedInState
    data object Q : AnErrorEndsTheBlockItWasRaisedInState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AnErrorEndsTheBlockItWasRaisedInEvent : Event {
    sealed interface Error : AnErrorEndsTheBlockItWasRaisedInEvent {
        data object Execution : Error
    }
    data object Finish : AnErrorEndsTheBlockItWasRaisedInEvent
    data object T : AnErrorEndsTheBlockItWasRaisedInEvent
}
// --- State Machine (W3C SCXML) ---

class AnErrorEndsTheBlockItWasRaisedInStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<AnErrorEndsTheBlockItWasRaisedInState, AnErrorEndsTheBlockItWasRaisedInEvent>(scriptEngine) {

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
     * §scxml-5.3: what the `afterAssign` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterAssign` was assigned a value of another type, or the engine refused.
     */
    fun afterAssign(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterAssign")

    /**
     * §scxml-5.3: what the `afterScript` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterScript` was assigned a value of another type, or the engine refused.
     */
    fun afterScript(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterScript")

    /**
     * §scxml-5.3: what the `afterLog` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterLog` was assigned a value of another type, or the engine refused.
     */
    fun afterLog(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterLog")

    /**
     * §scxml-5.3: what the `afterCancel` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterCancel` was assigned a value of another type, or the engine refused.
     */
    fun afterCancel(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterCancel")

    /**
     * §scxml-5.3: what the `afterIfInner` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterIfInner` was assigned a value of another type, or the engine refused.
     */
    fun afterIfInner(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterIfInner")

    /**
     * §scxml-5.3: what the `afterIf` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterIf` was assigned a value of another type, or the engine refused.
     */
    fun afterIf(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterIf")

    /**
     * §scxml-5.3: what the `afterSingle` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterSingle` was assigned a value of another type, or the engine refused.
     */
    fun afterSingle(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterSingle")

    /**
     * §scxml-5.3: what the `afterTrans` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterTrans` was assigned a value of another type, or the engine refused.
     */
    fun afterTrans(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterTrans")

    /**
     * §scxml-5.3: what the `initRan` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `initRan` was assigned a value of another type, or the engine refused.
     */
    fun initRan(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "initRan")

    /**
     * §scxml-5.3: what the `pairs` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `pairs` was assigned a value of another type, or the engine refused.
     */
    fun pairs(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "pairs")

    /**
     * §scxml-5.3: what the `sum` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `sum` was assigned a value of another type, or the engine refused.
     */
    fun sum(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "sum")

    /**
     * §scxml-5.3: what the `outer` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `outer` was assigned a value of another type, or the engine refused.
     *
     * The value as JSON text, serialised by the engine's own `JSON.stringify`
     * (§scxml-B-2) so the key order is the document's.
     */
    fun outer(): String? =
        com.sce.runtime.DatamodelRead.readJson(scriptEngine, scriptSessionId, "outer")

    /**
     * §scxml-5.3: what the `inner` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `inner` was assigned a value of another type, or the engine refused.
     *
     * The value as JSON text, serialised by the engine's own `JSON.stringify`
     * (§scxml-B-2) so the key order is the document's.
     */
    fun inner(): String? =
        com.sce.runtime.DatamodelRead.readJson(scriptEngine, scriptSessionId, "inner")

    /**
     * §scxml-5.3: what the `o` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `o` was assigned a value of another type, or the engine refused.
     */
    fun o(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "o")

    /**
     * §scxml-5.3: what the `i` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `i` was assigned a value of another type, or the engine refused.
     */
    fun i(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "i")

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

    override val initialState: AnErrorEndsTheBlockItWasRaisedInState = AnErrorEndsTheBlockItWasRaisedInState.Q

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

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: AnErrorEndsTheBlockItWasRaisedInState): AnErrorEndsTheBlockItWasRaisedInState? = when (state) {
        is AnErrorEndsTheBlockItWasRaisedInState.Q -> AnErrorEndsTheBlockItWasRaisedInState.P
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: AnErrorEndsTheBlockItWasRaisedInState): Boolean = when (state) {
        is AnErrorEndsTheBlockItWasRaisedInState.P -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: AnErrorEndsTheBlockItWasRaisedInState): Boolean = when (state) {
        is AnErrorEndsTheBlockItWasRaisedInState.Done -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: AnErrorEndsTheBlockItWasRaisedInState): List<AnErrorEndsTheBlockItWasRaisedInState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: AnErrorEndsTheBlockItWasRaisedInState): List<EntryTarget<AnErrorEndsTheBlockItWasRaisedInState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AnErrorEndsTheBlockItWasRaisedInState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<AnErrorEndsTheBlockItWasRaisedInState, List<AnErrorEndsTheBlockItWasRaisedInState>> = mapOf(
            AnErrorEndsTheBlockItWasRaisedInState.P to listOf(AnErrorEndsTheBlockItWasRaisedInState.Q),
        )

        val initialTargets: Map<AnErrorEndsTheBlockItWasRaisedInState, List<EntryTarget<AnErrorEndsTheBlockItWasRaisedInState, HistoryId>>> = mapOf(
            AnErrorEndsTheBlockItWasRaisedInState.P to listOf(StateTarget(AnErrorEndsTheBlockItWasRaisedInState.Q)),
        )

        val documentInitialTargetList: List<EntryTarget<AnErrorEndsTheBlockItWasRaisedInState, HistoryId>> =
            listOf(StateTarget(AnErrorEndsTheBlockItWasRaisedInState.P))

        // W3C SCXML 3.13: p's transition 0, as the microstep reads it.
        val transitionPAt0 = EnabledTransition<AnErrorEndsTheBlockItWasRaisedInState, HistoryId>(
            AnErrorEndsTheBlockItWasRaisedInState.P,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: p's transition 1, as the microstep reads it.
        val transitionPAt1 = EnabledTransition<AnErrorEndsTheBlockItWasRaisedInState, HistoryId>(
            AnErrorEndsTheBlockItWasRaisedInState.P,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: p's transition 2, as the microstep reads it.
        val transitionPAt2 = EnabledTransition<AnErrorEndsTheBlockItWasRaisedInState, HistoryId>(
            AnErrorEndsTheBlockItWasRaisedInState.P,
            listOf(StateTarget(AnErrorEndsTheBlockItWasRaisedInState.Done)),
            2,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AnErrorEndsTheBlockItWasRaisedInState? = when (stateId) {
        "done" -> AnErrorEndsTheBlockItWasRaisedInState.Done
        "p" -> AnErrorEndsTheBlockItWasRaisedInState.P
        "q" -> AnErrorEndsTheBlockItWasRaisedInState.Q
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AnErrorEndsTheBlockItWasRaisedInState): String = when (state) {
        is AnErrorEndsTheBlockItWasRaisedInState.Done -> "done"
        is AnErrorEndsTheBlockItWasRaisedInState.P -> "p"
        is AnErrorEndsTheBlockItWasRaisedInState.Q -> "q"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AnErrorEndsTheBlockItWasRaisedInState): Int = when (state) {
        is AnErrorEndsTheBlockItWasRaisedInState.Done -> 2
        is AnErrorEndsTheBlockItWasRaisedInState.P -> 0
        is AnErrorEndsTheBlockItWasRaisedInState.Q -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AnErrorEndsTheBlockItWasRaisedInEvent? = when (name) {
        "error.execution" -> AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution
        "finish" -> AnErrorEndsTheBlockItWasRaisedInEvent.Finish
        "t" -> AnErrorEndsTheBlockItWasRaisedInEvent.T
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AnErrorEndsTheBlockItWasRaisedInEvent): String? = when (event) {
        is AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution -> "error.execution"
        is AnErrorEndsTheBlockItWasRaisedInEvent.Finish -> "finish"
        is AnErrorEndsTheBlockItWasRaisedInEvent.T -> "t"
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
            "an_error_ends_the_block_it_was_raised_in",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'errors' with expr
        try {
            val initResult_errors = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "errors", initResult_errors)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='errors'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterAssign' with expr
        try {
            val initResult_afterAssign = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterAssign", initResult_afterAssign)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='afterAssign'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterScript' with expr
        try {
            val initResult_afterScript = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterScript", initResult_afterScript)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='afterScript'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterLog' with expr
        try {
            val initResult_afterLog = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterLog", initResult_afterLog)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='afterLog'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterCancel' with expr
        try {
            val initResult_afterCancel = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterCancel", initResult_afterCancel)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='afterCancel'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterIfInner' with expr
        try {
            val initResult_afterIfInner = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterIfInner", initResult_afterIfInner)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='afterIfInner'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterIf' with expr
        try {
            val initResult_afterIf = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterIf", initResult_afterIf)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='afterIf'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterSingle' with expr
        try {
            val initResult_afterSingle = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterSingle", initResult_afterSingle)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='afterSingle'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterTrans' with expr
        try {
            val initResult_afterTrans = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterTrans", initResult_afterTrans)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='afterTrans'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'initRan' with expr
        try {
            val initResult_initRan = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "initRan", initResult_initRan)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='initRan'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'pairs' with expr
        try {
            val initResult_pairs = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "pairs", initResult_pairs)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='pairs'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'sum' with expr
        try {
            val initResult_sum = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "sum", initResult_sum)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='sum'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'outer' with expr
        try {
            val initResult_outer = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{1, 2}", "[1, 2]"))
            engine.setVariable(sid, "outer", initResult_outer)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='outer'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'inner' with expr
        try {
            val initResult_inner = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{10, 20}", "[10, 20]"))
            engine.setVariable(sid, "inner", initResult_inner)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='inner'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'o' with expr
        try {
            val initResult_o = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "o", initResult_o)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='o'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'i' with expr
        try {
            val initResult_i = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "i", initResult_i)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='i'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'obj' with expr
        try {
            val initResult_obj = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{}", "({})"))
            engine.setVariable(sid, "obj", initResult_obj)
        } catch (e: Exception) {
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<data id='obj'> expr failed to evaluate")
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
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AnErrorEndsTheBlockItWasRaisedInEvent) {
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
    override fun bindCurrentEvent(event: AnErrorEndsTheBlockItWasRaisedInEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AnErrorEndsTheBlockItWasRaisedInState,
        event: AnErrorEndsTheBlockItWasRaisedInEvent?
    ): EnabledTransition<AnErrorEndsTheBlockItWasRaisedInState, HistoryId>? = when (state) {
        is AnErrorEndsTheBlockItWasRaisedInState.P -> when {
            event is AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution -> transitionPAt0
            event is AnErrorEndsTheBlockItWasRaisedInEvent.T -> transitionPAt1
            event is AnErrorEndsTheBlockItWasRaisedInEvent.Finish -> transitionPAt2
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: an_error_ends_the_block_it_was_raised_in.scxml:33 :: _machine
    override fun onEntry(state: AnErrorEndsTheBlockItWasRaisedInState, isDefaultEntry: Boolean) {
        when (state) {
            is AnErrorEndsTheBlockItWasRaisedInState.Done -> {
                // SCE-MAP: an_error_ends_the_block_it_was_raised_in.scxml:112 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AnErrorEndsTheBlockItWasRaisedInState.P -> {
                // SCE-MAP: an_error_ends_the_block_it_was_raised_in.scxml:57 :: p :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterSingle", "afterSingle"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 3.3: the <initial> transition's content runs when,
                // and only when, this state's initial state is entered by
                // default — not when the state is entered only as the ancestor
                // of a deeper target.
                if (isDefaultEntry) {
                    // W3C SCXML 4.9: its own block, ended by `return@run`.
                    run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("initRan", "initRan"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                    }
                }
            }
            is AnErrorEndsTheBlockItWasRaisedInState.Q -> {
                // SCE-MAP: an_error_ends_the_block_it_was_raised_in.scxml:77 :: q :: _state_body
                // W3C SCXML 3.8: Onentry block 1/6
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterAssign", "afterAssign"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 2/6
                run {


            // W3C SCXML 4.9: a script that fails has raised error.execution, and
            // the error ends the block.
            if (!executeScriptBlock(com.sce.runtime.ScriptSource.lua("obj.missing.deep = 1", "obj.missing.deep = 1;"))) {
                return@run
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterScript", "afterScript"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 3/6
                run {

            // W3C SCXML 4.7: Log expression evaluation. An expression that fails
            // raises error.execution (W3C SCXML 5.9), and the error ends the
            // block (W3C SCXML 4.9).
            if (run log@{
                try {
                    println("unreadable: " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"))?.toString() ?: ""))
                    false
                } catch (_: Exception) {
                    raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<log> expr failed to evaluate")
                    true
                }
            }) {
                return@run
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterLog", "afterLog"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 4/6
                run {


            // W3C SCXML 6.3: Dynamic sendid evaluation (test210). A sendidexpr
            // that fails raises error.execution (W3C SCXML 5.9), and the error
            // ends the block (W3C SCXML 4.9).
            if (run cancel@{
                ensureScriptEngine()
                val engineCancel = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidCancel = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    val v = engineCancel.evaluateExpr(sidCancel, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"))
                    val sendidToCancel = v?.toString() ?: ""
                    if (sendidToCancel.isNotEmpty()) cancelSend(sendidToCancel)
                    false
                } catch (_: Exception) {
                    raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<cancel> sendidexpr failed to evaluate")
                    true
                }
            }) {
                return@run
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterCancel", "afterCancel"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 5/6
                run {


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("true", "true"))) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterIfInner", "afterIfInner"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterIf", "afterIf"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 6/6
                run {


            if (run foreach@{
                ensureScriptEngine()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    engine.executeForeach(sid, com.sce.runtime.ScriptSource.lua("outer", "outer"), "o", "") {


            if (run foreach@{
                ensureScriptEngine()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    engine.executeForeach(sid, com.sce.runtime.ScriptSource.lua("inner", "inner"), "i", "") {


            engine.assign(sid, com.sce.runtime.ScriptSource.lua("pairs", "pairs"), com.sce.runtime.ScriptSource.lua("_scxml_add(pairs, 1)", "pairs + 1"))


            engine.assign(sid, com.sce.runtime.ScriptSource.lua("sum", "sum"), com.sce.runtime.ScriptSource.lua("_scxml_add(sum, (o * i))", "sum + o * i"))
                    }
                    false
                } catch (e: com.sce.runtime.ActionBlockAbort) {
                    // W3C SCXML 4.9: a body element raised its error and ended the
                    // block; nothing more is raised here.
                    true
                } catch (e: Exception) {
                    raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<foreach array='inner'> failed to iterate")
                    true
                }
            }) {
                // W3C SCXML 4.6 + 4.9: the block that contains the <foreach> ends.
                throw com.sce.runtime.ActionBlockAbort()
            } // end of run foreach@
                    }
                    false
                } catch (e: com.sce.runtime.ActionBlockAbort) {
                    // W3C SCXML 4.9: a body element raised its error and ended the
                    // block; nothing more is raised here.
                    true
                } catch (e: Exception) {
                    raisePlatformError(AnErrorEndsTheBlockItWasRaisedInEvent.Error.Execution, "<foreach array='outer'> failed to iterate")
                    true
                }
            }) {
                // W3C SCXML 4.6 + 4.9: the block that contains the <foreach> ends.
                return@run
            } // end of run foreach@
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: an_error_ends_the_block_it_was_raised_in.scxml:33 :: _machine
    override fun onExit(state: AnErrorEndsTheBlockItWasRaisedInState) {
        when (state) {
            is AnErrorEndsTheBlockItWasRaisedInState.Done -> {
                // SCE-MAP: an_error_ends_the_block_it_was_raised_in.scxml:112 :: done :: _state_body
            }
            is AnErrorEndsTheBlockItWasRaisedInState.P -> {
                // SCE-MAP: an_error_ends_the_block_it_was_raised_in.scxml:57 :: p :: _state_body
            }
            is AnErrorEndsTheBlockItWasRaisedInState.Q -> {
                // SCE-MAP: an_error_ends_the_block_it_was_raised_in.scxml:77 :: q :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: an_error_ends_the_block_it_was_raised_in.scxml:33 :: _machine
    override fun executeTransitionContent(source: AnErrorEndsTheBlockItWasRaisedInState, transitionIndex: Int) {
        when (source) {
        is AnErrorEndsTheBlockItWasRaisedInState.P -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: an_error_ends_the_block_it_was_raised_in.scxml:68 :: p :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("errors", "errors"), com.sce.runtime.ScriptSource.lua("_scxml_add(errors, 1)", "errors + 1"))) {
                return
            }
            }
            1 -> {
                // SCE-MAP: an_error_ends_the_block_it_was_raised_in.scxml:71 :: p :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterTrans", "afterTrans"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
