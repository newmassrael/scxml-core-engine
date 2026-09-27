// SCE-GENERATED — DO NOT EDIT
// source-hash: 3df514d5d4409dcae43d9342fc78a7d5cfc4a992b127150338c716e67cc1a388

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/an_error_inside_a_foreach_ends_its_block/an_error_inside_a_foreach_ends_its_block.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:38 :: _machine

package com.sce.integration.an_error_inside_a_foreach_ends_its_block

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AnErrorInsideAForeachEndsItsBlockState : State {
    data object Done : AnErrorInsideAForeachEndsItsBlockState
    data object Run : AnErrorInsideAForeachEndsItsBlockState
    data object S1 : AnErrorInsideAForeachEndsItsBlockState
    data object S2 : AnErrorInsideAForeachEndsItsBlockState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AnErrorInsideAForeachEndsItsBlockEvent : Event {
    sealed interface Error : AnErrorInsideAForeachEndsItsBlockEvent {
        data object Execution : Error
    }
    data object Finish : AnErrorInsideAForeachEndsItsBlockEvent
    data object Go : AnErrorInsideAForeachEndsItsBlockEvent
    data object Sent : AnErrorInsideAForeachEndsItsBlockEvent
    data object T : AnErrorInsideAForeachEndsItsBlockEvent
}
// --- State Machine (W3C SCXML) ---

class AnErrorInsideAForeachEndsItsBlockStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<AnErrorInsideAForeachEndsItsBlockState, AnErrorInsideAForeachEndsItsBlockEvent>(scriptEngine) {

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
     * §scxml-5.3: what the `sent` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `sent` was assigned a value of another type, or the engine refused.
     */
    fun sent(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "sent")

    /**
     * §scxml-5.3: what the `iters1` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `iters1` was assigned a value of another type, or the engine refused.
     */
    fun iters1(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "iters1")

    /**
     * §scxml-5.3: what the `after1` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `after1` was assigned a value of another type, or the engine refused.
     */
    fun after1(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "after1")

    /**
     * §scxml-5.3: what the `iters2` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `iters2` was assigned a value of another type, or the engine refused.
     */
    fun iters2(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "iters2")

    /**
     * §scxml-5.3: what the `after2` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `after2` was assigned a value of another type, or the engine refused.
     */
    fun after2(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "after2")

    /**
     * §scxml-5.3: what the `iters3` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `iters3` was assigned a value of another type, or the engine refused.
     */
    fun iters3(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "iters3")

    /**
     * §scxml-5.3: what the `after3` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `after3` was assigned a value of another type, or the engine refused.
     */
    fun after3(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "after3")

    /**
     * §scxml-5.3: what the `items` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `items` was assigned a value of another type, or the engine refused.
     *
     * The value as JSON text, serialised by the engine's own `JSON.stringify`
     * (§scxml-B-2) so the key order is the document's.
     */
    fun items(): String? =
        com.sce.runtime.DatamodelRead.readJson(scriptEngine, scriptSessionId, "items")

    /**
     * §scxml-5.3: what the `item` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `item` was assigned a value of another type, or the engine refused.
     */
    fun item(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "item")

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

    override val initialState: AnErrorInsideAForeachEndsItsBlockState = AnErrorInsideAForeachEndsItsBlockState.S1

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
    override fun parentOf(state: AnErrorInsideAForeachEndsItsBlockState): AnErrorInsideAForeachEndsItsBlockState? = when (state) {
        is AnErrorInsideAForeachEndsItsBlockState.S1 -> AnErrorInsideAForeachEndsItsBlockState.Run
        is AnErrorInsideAForeachEndsItsBlockState.S2 -> AnErrorInsideAForeachEndsItsBlockState.Run
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: AnErrorInsideAForeachEndsItsBlockState): Boolean = when (state) {
        is AnErrorInsideAForeachEndsItsBlockState.Run -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: AnErrorInsideAForeachEndsItsBlockState): Boolean = when (state) {
        is AnErrorInsideAForeachEndsItsBlockState.Done -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: AnErrorInsideAForeachEndsItsBlockState): List<AnErrorInsideAForeachEndsItsBlockState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: AnErrorInsideAForeachEndsItsBlockState): List<EntryTarget<AnErrorInsideAForeachEndsItsBlockState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AnErrorInsideAForeachEndsItsBlockState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<AnErrorInsideAForeachEndsItsBlockState, List<AnErrorInsideAForeachEndsItsBlockState>> = mapOf(
            AnErrorInsideAForeachEndsItsBlockState.Run to listOf(AnErrorInsideAForeachEndsItsBlockState.S1, AnErrorInsideAForeachEndsItsBlockState.S2),
        )

        val initialTargets: Map<AnErrorInsideAForeachEndsItsBlockState, List<EntryTarget<AnErrorInsideAForeachEndsItsBlockState, HistoryId>>> = mapOf(
            AnErrorInsideAForeachEndsItsBlockState.Run to listOf(StateTarget(AnErrorInsideAForeachEndsItsBlockState.S1)),
        )

        val documentInitialTargetList: List<EntryTarget<AnErrorInsideAForeachEndsItsBlockState, HistoryId>> =
            listOf(StateTarget(AnErrorInsideAForeachEndsItsBlockState.Run))

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<AnErrorInsideAForeachEndsItsBlockState, HistoryId>(
            AnErrorInsideAForeachEndsItsBlockState.Run,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 1, as the microstep reads it.
        val transitionRunAt1 = EnabledTransition<AnErrorInsideAForeachEndsItsBlockState, HistoryId>(
            AnErrorInsideAForeachEndsItsBlockState.Run,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 2, as the microstep reads it.
        val transitionRunAt2 = EnabledTransition<AnErrorInsideAForeachEndsItsBlockState, HistoryId>(
            AnErrorInsideAForeachEndsItsBlockState.Run,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 3, as the microstep reads it.
        val transitionRunAt3 = EnabledTransition<AnErrorInsideAForeachEndsItsBlockState, HistoryId>(
            AnErrorInsideAForeachEndsItsBlockState.Run,
            listOf(StateTarget(AnErrorInsideAForeachEndsItsBlockState.Done)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<AnErrorInsideAForeachEndsItsBlockState, HistoryId>(
            AnErrorInsideAForeachEndsItsBlockState.S1,
            listOf(StateTarget(AnErrorInsideAForeachEndsItsBlockState.S2)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AnErrorInsideAForeachEndsItsBlockState? = when (stateId) {
        "done" -> AnErrorInsideAForeachEndsItsBlockState.Done
        "run" -> AnErrorInsideAForeachEndsItsBlockState.Run
        "s1" -> AnErrorInsideAForeachEndsItsBlockState.S1
        "s2" -> AnErrorInsideAForeachEndsItsBlockState.S2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AnErrorInsideAForeachEndsItsBlockState): String = when (state) {
        is AnErrorInsideAForeachEndsItsBlockState.Done -> "done"
        is AnErrorInsideAForeachEndsItsBlockState.Run -> "run"
        is AnErrorInsideAForeachEndsItsBlockState.S1 -> "s1"
        is AnErrorInsideAForeachEndsItsBlockState.S2 -> "s2"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AnErrorInsideAForeachEndsItsBlockState): Int = when (state) {
        is AnErrorInsideAForeachEndsItsBlockState.Done -> 3
        is AnErrorInsideAForeachEndsItsBlockState.Run -> 0
        is AnErrorInsideAForeachEndsItsBlockState.S1 -> 1
        is AnErrorInsideAForeachEndsItsBlockState.S2 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AnErrorInsideAForeachEndsItsBlockEvent? = when (name) {
        "error.execution" -> AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution
        "finish" -> AnErrorInsideAForeachEndsItsBlockEvent.Finish
        "go" -> AnErrorInsideAForeachEndsItsBlockEvent.Go
        "sent" -> AnErrorInsideAForeachEndsItsBlockEvent.Sent
        "t" -> AnErrorInsideAForeachEndsItsBlockEvent.T
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AnErrorInsideAForeachEndsItsBlockEvent): String? = when (event) {
        is AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution -> "error.execution"
        is AnErrorInsideAForeachEndsItsBlockEvent.Finish -> "finish"
        is AnErrorInsideAForeachEndsItsBlockEvent.Go -> "go"
        is AnErrorInsideAForeachEndsItsBlockEvent.Sent -> "sent"
        is AnErrorInsideAForeachEndsItsBlockEvent.T -> "t"
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
            "an_error_inside_a_foreach_ends_its_block",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'errors' with expr
        try {
            val initResult_errors = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "errors", initResult_errors)
        } catch (e: Exception) {
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='errors'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'sent' with expr
        try {
            val initResult_sent = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "sent", initResult_sent)
        } catch (e: Exception) {
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='sent'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'iters1' with expr
        try {
            val initResult_iters1 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "iters1", initResult_iters1)
        } catch (e: Exception) {
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='iters1'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'after1' with expr
        try {
            val initResult_after1 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "after1", initResult_after1)
        } catch (e: Exception) {
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='after1'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'iters2' with expr
        try {
            val initResult_iters2 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "iters2", initResult_iters2)
        } catch (e: Exception) {
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='iters2'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'after2' with expr
        try {
            val initResult_after2 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "after2", initResult_after2)
        } catch (e: Exception) {
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='after2'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'iters3' with expr
        try {
            val initResult_iters3 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "iters3", initResult_iters3)
        } catch (e: Exception) {
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='iters3'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'after3' with expr
        try {
            val initResult_after3 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "after3", initResult_after3)
        } catch (e: Exception) {
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='after3'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'items' with expr
        try {
            val initResult_items = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{1, 2, 3}", "[1, 2, 3]"))
            engine.setVariable(sid, "items", initResult_items)
        } catch (e: Exception) {
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='items'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'item' with expr
        try {
            val initResult_item = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "item", initResult_item)
        } catch (e: Exception) {
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='item'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'obj' with expr
        try {
            val initResult_obj = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{}", "({})"))
            engine.setVariable(sid, "obj", initResult_obj)
        } catch (e: Exception) {
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='obj'> expr failed to evaluate")
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
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AnErrorInsideAForeachEndsItsBlockEvent) {
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
    override fun bindCurrentEvent(event: AnErrorInsideAForeachEndsItsBlockEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AnErrorInsideAForeachEndsItsBlockState,
        event: AnErrorInsideAForeachEndsItsBlockEvent?
    ): EnabledTransition<AnErrorInsideAForeachEndsItsBlockState, HistoryId>? = when (state) {
        is AnErrorInsideAForeachEndsItsBlockState.Run -> when {
            event is AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution -> transitionRunAt0
            event is AnErrorInsideAForeachEndsItsBlockEvent.Sent -> transitionRunAt1
            event is AnErrorInsideAForeachEndsItsBlockEvent.T -> transitionRunAt2
            event is AnErrorInsideAForeachEndsItsBlockEvent.Finish -> transitionRunAt3
            else -> null
        }
        is AnErrorInsideAForeachEndsItsBlockState.S1 -> when {
            event is AnErrorInsideAForeachEndsItsBlockEvent.Go -> transitionS1At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:38 :: _machine
    override fun onEntry(state: AnErrorInsideAForeachEndsItsBlockState, isDefaultEntry: Boolean) {
        when (state) {
            is AnErrorInsideAForeachEndsItsBlockState.Done -> {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:98 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AnErrorInsideAForeachEndsItsBlockState.Run -> {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:56 :: run :: _state_body
            }
            is AnErrorInsideAForeachEndsItsBlockState.S1 -> {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:74 :: s1 :: _state_body


            if (run foreach@{
                ensureScriptEngine()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    engine.executeForeach(sid, com.sce.runtime.ScriptSource.lua("items", "items"), "item", "") {


            engine.assign(sid, com.sce.runtime.ScriptSource.lua("iters1", "iters1"), com.sce.runtime.ScriptSource.lua("_scxml_add(iters1, 1)", "iters1 + 1"))


            if (run send@{
            var paramFailed = false
            // W3C SCXML 5.10: An internal send carries `_event.data` just as
            // an external one does. Before this the payload was dropped
            // silently — the event was queued with no data at all.
            run {
                ensureScriptEngine()
                val engineI = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidI = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsI = mutableMapOf<String, Any?>()
                try {
                    putParam(paramsI, "bad", engineI.evaluateExpr(sidI, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep")))
                } catch (_: Exception) {
                    raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<send> <param name='bad'> could not be read")
                    paramFailed = true
                }


                raiseInternal(AnErrorInsideAForeachEndsItsBlockEvent.Sent, EventMetadata.internal(buildJsonFromParams(paramsI)))
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                throw com.sce.runtime.ActionBlockAbort()
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                    }
                    false
                } catch (e: com.sce.runtime.ActionBlockAbort) {
                    // W3C SCXML 4.9: a body element raised its error and ended the
                    // block; nothing more is raised here.
                    true
                } catch (e: Exception) {
                    raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<foreach array='items'> failed to iterate")
                    true
                }
            }) {
                // W3C SCXML 4.6 + 4.9: the block that contains the <foreach> ends.
                return
            } // end of run foreach@


            executeAssign(com.sce.runtime.ScriptSource.lua("after1", "after1"), com.sce.runtime.ScriptSource.lua("1", "1"))
            }
            is AnErrorInsideAForeachEndsItsBlockState.S2 -> {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:87 :: s2 :: _state_body


            if (run foreach@{
                ensureScriptEngine()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    engine.executeForeach(sid, com.sce.runtime.ScriptSource.lua("items", "items"), "item", "") {


            engine.assign(sid, com.sce.runtime.ScriptSource.lua("iters2", "iters2"), com.sce.runtime.ScriptSource.lua("_scxml_add(iters2, 1)", "iters2 + 1"))


            engine.assign(sid, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep"), com.sce.runtime.ScriptSource.lua("1", "1"))
                    }
                    false
                } catch (e: com.sce.runtime.ActionBlockAbort) {
                    // W3C SCXML 4.9: a body element raised its error and ended the
                    // block; nothing more is raised here.
                    true
                } catch (e: Exception) {
                    raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<foreach array='items'> failed to iterate")
                    true
                }
            }) {
                // W3C SCXML 4.6 + 4.9: the block that contains the <foreach> ends.
                return
            } // end of run foreach@


            executeAssign(com.sce.runtime.ScriptSource.lua("after2", "after2"), com.sce.runtime.ScriptSource.lua("1", "1"))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:38 :: _machine
    override fun onExit(state: AnErrorInsideAForeachEndsItsBlockState) {
        when (state) {
            is AnErrorInsideAForeachEndsItsBlockState.Done -> {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:98 :: done :: _state_body
            }
            is AnErrorInsideAForeachEndsItsBlockState.Run -> {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:56 :: run :: _state_body
            }
            is AnErrorInsideAForeachEndsItsBlockState.S1 -> {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:74 :: s1 :: _state_body
            }
            is AnErrorInsideAForeachEndsItsBlockState.S2 -> {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:87 :: s2 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:38 :: _machine
    override fun executeTransitionContent(source: AnErrorInsideAForeachEndsItsBlockState, transitionIndex: Int) {
        when (source) {
        is AnErrorInsideAForeachEndsItsBlockState.Run -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:57 :: run :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("errors", "errors"), com.sce.runtime.ScriptSource.lua("_scxml_add(errors, 1)", "errors + 1"))
            }
            1 -> {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:60 :: run :: _transition_1


            executeAssign(com.sce.runtime.ScriptSource.lua("sent", "sent"), com.sce.runtime.ScriptSource.lua("_scxml_add(sent, 1)", "sent + 1"))
            }
            2 -> {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:63 :: run :: _transition_2


            if (run foreach@{
                ensureScriptEngine()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    engine.executeForeach(sid, com.sce.runtime.ScriptSource.lua("items", "items"), "item", "") {


            engine.assign(sid, com.sce.runtime.ScriptSource.lua("iters3", "iters3"), com.sce.runtime.ScriptSource.lua("_scxml_add(iters3, 1)", "iters3 + 1"))


            if (run send@{
            var paramFailed = false
            // W3C SCXML 5.10: An internal send carries `_event.data` just as
            // an external one does. Before this the payload was dropped
            // silently — the event was queued with no data at all.
            run {
                ensureScriptEngine()
                val engineI = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidI = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsI = mutableMapOf<String, Any?>()
                try {
                    putParam(paramsI, "bad", engineI.evaluateExpr(sidI, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep")))
                } catch (_: Exception) {
                    raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<send> <param name='bad'> could not be read")
                    paramFailed = true
                }


                raiseInternal(AnErrorInsideAForeachEndsItsBlockEvent.Sent, EventMetadata.internal(buildJsonFromParams(paramsI)))
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                throw com.sce.runtime.ActionBlockAbort()
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                    }
                    false
                } catch (e: com.sce.runtime.ActionBlockAbort) {
                    // W3C SCXML 4.9: a body element raised its error and ended the
                    // block; nothing more is raised here.
                    true
                } catch (e: Exception) {
                    raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<foreach array='items'> failed to iterate")
                    true
                }
            }) {
                // W3C SCXML 4.6 + 4.9: the block that contains the <foreach> ends.
                return
            } // end of run foreach@


            executeAssign(com.sce.runtime.ScriptSource.lua("after3", "after3"), com.sce.runtime.ScriptSource.lua("1", "1"))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
