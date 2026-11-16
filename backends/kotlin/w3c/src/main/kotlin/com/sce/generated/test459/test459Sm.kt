// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/459/test459.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test459.scxml:5 :: _machine

package com.sce.generated.test459

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test459State : State {
    data object Fail : Test459State
    data object Pass : Test459State
    data object S0 : Test459State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test459Event : Event {
    sealed interface Error : Test459Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test459StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test459State, Test459Event>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `Var1` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var1` was assigned a value of another type, or the engine refused.
     */
    fun Var1(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var1")

    /**
     * §scxml-5.3: what the `Var4` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var4` was assigned a value of another type, or the engine refused.
     *
     * The value as JSON text, serialised by the engine's own `JSON.stringify`
     * (§scxml-B-2) so the key order is the document's.
     */
    fun Var4(): String? =
        com.sce.runtime.DatamodelRead.readJson(scriptEngine, scriptSessionId, "Var4")

    /**
     * §scxml-5.3: what the `Var5` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var5` was assigned a value of another type, or the engine refused.
     */
    fun Var5(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var5")

    override val initialState: Test459State = Test459State.S0

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

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test459State): Boolean = when (state) {
        is Test459State.Fail -> true
        is Test459State.Pass -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test459State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test459State, HistoryId>> =
            listOf(StateTarget(Test459State.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test459State, HistoryId>(
            Test459State.S0,
            listOf(StateTarget(Test459State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test459State, HistoryId>(
            Test459State.S0,
            listOf(StateTarget(Test459State.Pass)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test459State? = when (stateId) {
        "fail" -> Test459State.Fail
        "pass" -> Test459State.Pass
        "s0" -> Test459State.S0
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test459State): String = when (state) {
        is Test459State.Fail -> "fail"
        is Test459State.Pass -> "pass"
        is Test459State.S0 -> "s0"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test459State): Int = when (state) {
        is Test459State.Fail -> 2
        is Test459State.Pass -> 1
        is Test459State.S0 -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test459Event? = when (name) {
        "error.execution" -> Test459Event.Error.Execution
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test459Event): String? = when (event) {
        is Test459Event.Error.Execution -> "error.execution"
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
            "test459",
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
        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raisePlatformError(Test459Event.Error.Execution, "<data id='Var1'> expr failed to evaluate")
        }
        // W3C SCXML 5.2: Runtime variable 'Var2' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var2", null)
        } catch (_: Exception) {}
        // W3C SCXML 5.2: Runtime variable 'Var3' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var3", null)
        } catch (_: Exception) {}
        // W3C SCXML 5.3: Initialize variable 'Var4' with expr
        try {
            val initResult_Var4 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{1, 2, 3}", "[1,2,3]"))
            engine.setVariable(sid, "Var4", initResult_Var4)
        } catch (e: Exception) {
            raisePlatformError(Test459Event.Error.Execution, "<data id='Var4'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'Var5' with expr
        try {
            val initResult_Var5 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("1", "1"))
            engine.setVariable(sid, "Var5", initResult_Var5)
        } catch (e: Exception) {
            raisePlatformError(Test459Event.Error.Execution, "<data id='Var5'> expr failed to evaluate")
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
            raisePlatformError(Test459Event.Error.Execution, reason)
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
            raisePlatformError(Test459Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test459Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test459Event.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test459Event) {
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
    override fun bindCurrentEvent(event: Test459Event) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test459State, event: Test459Event?): EnabledTransition<Test459State, HistoryId>? = when (state) {
        is Test459State.S0 -> firstEnabledTransitionOfS0(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: Test459State, event: Test459Event?): EnabledTransition<Test459State, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_truthy(_scxml_bitor(_scxml_eq(Var4, 0), (not _scxml_eq(Var3, 2))))", "Var4==0 | Var3 != 2")) -> TransitionsPart0.transitionS0At0
        event == null -> TransitionsPart0.transitionS0At1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test459.scxml:5 :: _machine
    override fun onEntry(state: Test459State, isDefaultEntry: Boolean) {
        when (state) {
            is Test459State.Fail -> onEntryOfFail()
            is Test459State.Pass -> onEntryOfPass()
            is Test459State.S0 -> onEntryOfS0()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test459.scxml:32 :: fail :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            // W3C SCXML 4.7: Log expression evaluation. An expression that fails
            // raises error.execution (W3C SCXML 5.9), and the error ends the
            // block (W3C SCXML 4.9).
            if (run log@{
                try {
                    println("Outcome: " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", com.sce.runtime.ScriptSource.lua("\"fail\"", "'fail'"))?.toString() ?: ""))
                    false
                } catch (_: Exception) {
                    raisePlatformError(Test459Event.Error.Execution, "<log> expr failed to evaluate")
                    true
                }
            }) {
                return@run
            }
                }
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test459.scxml:31 :: pass :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            // W3C SCXML 4.7: Log expression evaluation. An expression that fails
            // raises error.execution (W3C SCXML 5.9), and the error ends the
            // block (W3C SCXML 4.9).
            if (run log@{
                try {
                    println("Outcome: " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", com.sce.runtime.ScriptSource.lua("\"pass\"", "'pass'"))?.toString() ?: ""))
                    false
                } catch (_: Exception) {
                    raisePlatformError(Test459Event.Error.Execution, "<log> expr failed to evaluate")
                    true
                }
            }) {
                return@run
            }
                }
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test459.scxml:14 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run foreach@{
                assertScriptSession()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
                try {
                    engine.executeForeach(sid, com.sce.runtime.ScriptSource.lua("Var4", "Var4"), "Var2", "Var3") {


            var ifCondFailed1 = false
            if (evaluateIfCond(com.sce.runtime.ScriptSource.lua("(Var1 < Var2)", "Var1<Var2")) { ifCondFailed1 = true }) {


            engine.assign(sid, com.sce.runtime.ScriptSource.lua("Var1", "Var1"), com.sce.runtime.ScriptSource.lua("Var2", "Var2"))
            } else {


            engine.assign(sid, com.sce.runtime.ScriptSource.lua("Var5", "Var5"), com.sce.runtime.ScriptSource.lua("0", "0"))
            }
            if (ifCondFailed1) {
                throw com.sce.runtime.ActionBlockAbort()
            }
                    }
                    false
                } catch (e: com.sce.runtime.ActionBlockAbort) {
                    // W3C SCXML 4.9: a body element raised its error and ended the
                    // block; nothing more is raised here.
                    true
                } catch (e: Exception) {
                    raisePlatformError(Test459Event.Error.Execution, "<foreach array='Var4'> failed to iterate")
                    true
                }
            }) {
                // W3C SCXML 4.6 + 4.9: the block that contains the <foreach> ends.
                return@run
            } // end of run foreach@
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test459.scxml:5 :: _machine
    override fun onExit(state: Test459State) {
        when (state) {
            is Test459State.Fail -> onExitOfFail()
            is Test459State.Pass -> onExitOfPass()
            is Test459State.S0 -> onExitOfS0()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test459.scxml:32 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test459.scxml:31 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test459.scxml:14 :: s0 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test459.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test459State, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
