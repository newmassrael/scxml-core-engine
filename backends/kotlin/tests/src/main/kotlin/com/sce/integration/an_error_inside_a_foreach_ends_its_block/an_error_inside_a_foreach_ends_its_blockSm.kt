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

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

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
        val childStates: Map<AnErrorInsideAForeachEndsItsBlockState, List<AnErrorInsideAForeachEndsItsBlockState>> = LinkedHashMap<AnErrorInsideAForeachEndsItsBlockState, List<AnErrorInsideAForeachEndsItsBlockState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<AnErrorInsideAForeachEndsItsBlockState, List<AnErrorInsideAForeachEndsItsBlockState>>) {
            table[AnErrorInsideAForeachEndsItsBlockState.Run] = listOf(AnErrorInsideAForeachEndsItsBlockState.S1, AnErrorInsideAForeachEndsItsBlockState.S2)
        }


        val initialTargets: Map<AnErrorInsideAForeachEndsItsBlockState, List<EntryTarget<AnErrorInsideAForeachEndsItsBlockState, HistoryId>>> = LinkedHashMap<AnErrorInsideAForeachEndsItsBlockState, List<EntryTarget<AnErrorInsideAForeachEndsItsBlockState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<AnErrorInsideAForeachEndsItsBlockState, List<EntryTarget<AnErrorInsideAForeachEndsItsBlockState, HistoryId>>>) {
            table[AnErrorInsideAForeachEndsItsBlockState.Run] = listOf(StateTarget(AnErrorInsideAForeachEndsItsBlockState.S1))
        }


        val documentInitialTargetList: List<EntryTarget<AnErrorInsideAForeachEndsItsBlockState, HistoryId>> =
            listOf(StateTarget(AnErrorInsideAForeachEndsItsBlockState.Run))
    }

    private object TransitionsPart0 {

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
            "an_error_inside_a_foreach_ends_its_block",
        )?.let { return it }
        return null
    }

    // W3C SCXML 5.3: initialize the datamodel in the session the start
    // established, before any state is entered: the document's <data>, its
    // states' early-bound <data>, then the top-level <script>s, the order of
    // interpret in Appendix D. §scxml-6.4: a value the invoke that started
    // this session handed it under the id of a top-level <data> is that
    // <data>'s initial value, declared in place of its own, so the scripts,
    // the <onentry>s and the conditions of the first macrostep read it; a
    // state's <data> never takes one, and a name no top-level <data> has is
    // not added (docs/adr/0098).
    override fun declareDatamodel() {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")

        declareVariablesPart0(engine, sid)

    }

    @Suppress("UNUSED_PARAMETER")
    private fun declareVariablesPart0(engine: ScxmlScriptEngine, sid: String) {
        // §scxml-6.4: 'errors' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "errors", AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'errors' with expr
            try {
                engine.setVariable(sid, "errors", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0")))
            } catch (e: Exception) {
                raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='errors'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'sent' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "sent", AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'sent' with expr
            try {
                engine.setVariable(sid, "sent", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0")))
            } catch (e: Exception) {
                raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='sent'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'iters1' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "iters1", AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'iters1' with expr
            try {
                engine.setVariable(sid, "iters1", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0")))
            } catch (e: Exception) {
                raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='iters1'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'after1' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "after1", AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'after1' with expr
            try {
                engine.setVariable(sid, "after1", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0")))
            } catch (e: Exception) {
                raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='after1'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'iters2' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "iters2", AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'iters2' with expr
            try {
                engine.setVariable(sid, "iters2", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0")))
            } catch (e: Exception) {
                raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='iters2'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'after2' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "after2", AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'after2' with expr
            try {
                engine.setVariable(sid, "after2", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0")))
            } catch (e: Exception) {
                raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='after2'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'iters3' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "iters3", AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'iters3' with expr
            try {
                engine.setVariable(sid, "iters3", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0")))
            } catch (e: Exception) {
                raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='iters3'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'after3' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "after3", AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'after3' with expr
            try {
                engine.setVariable(sid, "after3", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0")))
            } catch (e: Exception) {
                raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='after3'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'items' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "items", AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'items' with expr
            try {
                engine.setVariable(sid, "items", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{1, 2, 3}", "[1, 2, 3]")))
            } catch (e: Exception) {
                raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='items'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'item' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "item", AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'item' with expr
            try {
                engine.setVariable(sid, "item", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0")))
            } catch (e: Exception) {
                raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='item'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'obj' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "obj", AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'obj' with expr
            try {
                engine.setVariable(sid, "obj", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{}", "({})")))
            } catch (e: Exception) {
                raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<data id='obj'> expr failed to evaluate")
            }
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
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, reason)
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
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AnErrorInsideAForeachEndsItsBlockEvent) {
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
    override fun bindCurrentEvent(event: AnErrorInsideAForeachEndsItsBlockEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: AnErrorInsideAForeachEndsItsBlockState, event: AnErrorInsideAForeachEndsItsBlockEvent?): EnabledTransition<AnErrorInsideAForeachEndsItsBlockState, HistoryId>? = when (state) {
        is AnErrorInsideAForeachEndsItsBlockState.Run -> firstEnabledTransitionOfRun(state, event)
        is AnErrorInsideAForeachEndsItsBlockState.S1 -> firstEnabledTransitionOfS1(state, event)
        else -> null
    }


    // run's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRun(state: AnErrorInsideAForeachEndsItsBlockState, event: AnErrorInsideAForeachEndsItsBlockEvent?): EnabledTransition<AnErrorInsideAForeachEndsItsBlockState, HistoryId>? = when {
        event is AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution -> TransitionsPart0.transitionRunAt0
        event is AnErrorInsideAForeachEndsItsBlockEvent.Sent -> TransitionsPart0.transitionRunAt1
        event is AnErrorInsideAForeachEndsItsBlockEvent.T -> TransitionsPart0.transitionRunAt2
        event is AnErrorInsideAForeachEndsItsBlockEvent.Finish -> TransitionsPart0.transitionRunAt3
        else -> null
    }


    // s1's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS1(state: AnErrorInsideAForeachEndsItsBlockState, event: AnErrorInsideAForeachEndsItsBlockEvent?): EnabledTransition<AnErrorInsideAForeachEndsItsBlockState, HistoryId>? = when {
        event is AnErrorInsideAForeachEndsItsBlockEvent.Go -> TransitionsPart0.transitionS1At0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:38 :: _machine
    override fun onEntry(state: AnErrorInsideAForeachEndsItsBlockState, isDefaultEntry: Boolean) {
        when (state) {
            is AnErrorInsideAForeachEndsItsBlockState.Done -> onEntryOfDone()
            is AnErrorInsideAForeachEndsItsBlockState.Run -> onEntryOfRun()
            is AnErrorInsideAForeachEndsItsBlockState.S1 -> onEntryOfS1()
            is AnErrorInsideAForeachEndsItsBlockState.S2 -> onEntryOfS2()
        }
    }


    private fun onEntryOfDone() {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:98 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfRun() {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:56 :: run :: _state_body
    }

    private fun onEntryOfS1() {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:74 :: s1 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run foreach@{
                assertScriptSession()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
                try {
                    engine.executeForeach(sid, com.sce.runtime.ScriptSource.lua("items", "items"), "item", "") {


            engine.assign(sid, com.sce.runtime.ScriptSource.lua("iters1", "iters1"), com.sce.runtime.ScriptSource.lua("_scxml_add(iters1, 1)", "iters1 + 1"))


            if (run send@{
            var paramFailed = false
            assertScriptSession()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "bad", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep")))
            } catch (_: Exception) {
                raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<send> <param name='bad'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(AnErrorInsideAForeachEndsItsBlockEvent.Sent, EventMetadata.internal(sendData))
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
                return@run
            } // end of run foreach@


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("after1", "after1"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
    }

    private fun onEntryOfS2() {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:87 :: s2 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run foreach@{
                assertScriptSession()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
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
                return@run
            } // end of run foreach@


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("after2", "after2"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:38 :: _machine
    override fun onExit(state: AnErrorInsideAForeachEndsItsBlockState) {
        when (state) {
            is AnErrorInsideAForeachEndsItsBlockState.Done -> onExitOfDone()
            is AnErrorInsideAForeachEndsItsBlockState.Run -> onExitOfRun()
            is AnErrorInsideAForeachEndsItsBlockState.S1 -> onExitOfS1()
            is AnErrorInsideAForeachEndsItsBlockState.S2 -> onExitOfS2()
        }
    }


    private fun onExitOfDone() {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:98 :: done :: _state_body
    }

    private fun onExitOfRun() {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:56 :: run :: _state_body
    }

    private fun onExitOfS1() {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:74 :: s1 :: _state_body
    }

    private fun onExitOfS2() {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:87 :: s2 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:38 :: _machine
    override fun executeTransitionContent(source: AnErrorInsideAForeachEndsItsBlockState, transitionIndex: Int) {
        when (source) {
            is AnErrorInsideAForeachEndsItsBlockState.Run -> transitionContentOfRun(transitionIndex)
            else -> {}
        }
    }


    // The content of `run`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfRun(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfRunAt0()
            1 -> transitionContentOfRunAt1()
            2 -> transitionContentOfRunAt2()
            else -> {}
        }
    }


    private fun transitionContentOfRunAt0() {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:57 :: run :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("errors", "errors"), com.sce.runtime.ScriptSource.lua("_scxml_add(errors, 1)", "errors + 1"))) {
                return
            }
    }

    private fun transitionContentOfRunAt1() {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:60 :: run :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("sent", "sent"), com.sce.runtime.ScriptSource.lua("_scxml_add(sent, 1)", "sent + 1"))) {
                return
            }
    }

    private fun transitionContentOfRunAt2() {
                // SCE-MAP: an_error_inside_a_foreach_ends_its_block.scxml:63 :: run :: _transition_2


            if (run foreach@{
                assertScriptSession()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
                try {
                    engine.executeForeach(sid, com.sce.runtime.ScriptSource.lua("items", "items"), "item", "") {


            engine.assign(sid, com.sce.runtime.ScriptSource.lua("iters3", "iters3"), com.sce.runtime.ScriptSource.lua("_scxml_add(iters3, 1)", "iters3 + 1"))


            if (run send@{
            var paramFailed = false
            assertScriptSession()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "bad", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep")))
            } catch (_: Exception) {
                raisePlatformError(AnErrorInsideAForeachEndsItsBlockEvent.Error.Execution, "<send> <param name='bad'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(AnErrorInsideAForeachEndsItsBlockEvent.Sent, EventMetadata.internal(sendData))
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


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("after3", "after3"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
    }
}
