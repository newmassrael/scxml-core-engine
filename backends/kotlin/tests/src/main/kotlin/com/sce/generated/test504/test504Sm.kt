// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/504/test504.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test504.scxml:5 :: _machine

package com.sce.generated.test504

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test504State : State {
    data object Fail : Test504State
    data object P : Test504State
    data object Pass : Test504State
    data object Ps1 : Test504State
    data object Ps2 : Test504State
    data object S1 : Test504State
    data object S2 : Test504State
    data object S3 : Test504State
    data object S4 : Test504State
    data object S5 : Test504State
    data object S6 : Test504State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test504Event : Event {
    data object Bar : Test504Event
    sealed interface Error : Test504Event {
        data object Execution : Error
    }
    data object Foo : Test504Event
}
// --- State Machine (W3C SCXML) ---

class Test504StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test504State, Test504Event>(scriptEngine) {

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
     * §scxml-5.3: what the `Var2` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var2` was assigned a value of another type, or the engine refused.
     */
    fun Var2(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var2")

    /**
     * §scxml-5.3: what the `Var3` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var3` was assigned a value of another type, or the engine refused.
     */
    fun Var3(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var3")

    /**
     * §scxml-5.3: what the `Var4` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var4` was assigned a value of another type, or the engine refused.
     */
    fun Var4(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var4")

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

    override val initialState: Test504State = Test504State.S1

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
    override fun parentOf(state: Test504State): Test504State? = when (state) {
        is Test504State.P -> Test504State.S2
        is Test504State.Ps1 -> Test504State.P
        is Test504State.Ps2 -> Test504State.P
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test504State): Boolean = when (state) {
        is Test504State.S2 -> true
        else -> false
    }


    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test504State): Boolean = when (state) {
        is Test504State.P -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test504State): Boolean = when (state) {
        is Test504State.Fail -> true
        is Test504State.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test504State): List<Test504State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test504State): List<EntryTarget<Test504State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test504State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test504State, List<Test504State>> = LinkedHashMap<Test504State, List<Test504State>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<Test504State, List<Test504State>>) {
            table[Test504State.P] = listOf(Test504State.Ps1, Test504State.Ps2)
            table[Test504State.S2] = listOf(Test504State.P)
        }


        val initialTargets: Map<Test504State, List<EntryTarget<Test504State, HistoryId>>> = LinkedHashMap<Test504State, List<EntryTarget<Test504State, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<Test504State, List<EntryTarget<Test504State, HistoryId>>>) {
            table[Test504State.S2] = listOf(StateTarget(Test504State.P))
        }


        val documentInitialTargetList: List<EntryTarget<Test504State, HistoryId>> =
            listOf(StateTarget(Test504State.S1))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: p's transition 0, as the microstep reads it.
        val transitionPAt0 = EnabledTransition<Test504State, HistoryId>(
            Test504State.P,
            listOf(StateTarget(Test504State.Ps1)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: p's transition 1, as the microstep reads it.
        val transitionPAt1 = EnabledTransition<Test504State, HistoryId>(
            Test504State.P,
            listOf(StateTarget(Test504State.S3)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: p's transition 2, as the microstep reads it.
        val transitionPAt2 = EnabledTransition<Test504State, HistoryId>(
            Test504State.P,
            listOf(StateTarget(Test504State.Fail)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test504State, HistoryId>(
            Test504State.S1,
            listOf(StateTarget(Test504State.P)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s3's transition 0, as the microstep reads it.
        val transitionS3At0 = EnabledTransition<Test504State, HistoryId>(
            Test504State.S3,
            listOf(StateTarget(Test504State.S4)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s3's transition 1, as the microstep reads it.
        val transitionS3At1 = EnabledTransition<Test504State, HistoryId>(
            Test504State.S3,
            listOf(StateTarget(Test504State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s4's transition 0, as the microstep reads it.
        val transitionS4At0 = EnabledTransition<Test504State, HistoryId>(
            Test504State.S4,
            listOf(StateTarget(Test504State.S5)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s4's transition 1, as the microstep reads it.
        val transitionS4At1 = EnabledTransition<Test504State, HistoryId>(
            Test504State.S4,
            listOf(StateTarget(Test504State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s5's transition 0, as the microstep reads it.
        val transitionS5At0 = EnabledTransition<Test504State, HistoryId>(
            Test504State.S5,
            listOf(StateTarget(Test504State.S6)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s5's transition 1, as the microstep reads it.
        val transitionS5At1 = EnabledTransition<Test504State, HistoryId>(
            Test504State.S5,
            listOf(StateTarget(Test504State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s6's transition 0, as the microstep reads it.
        val transitionS6At0 = EnabledTransition<Test504State, HistoryId>(
            Test504State.S6,
            listOf(StateTarget(Test504State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s6's transition 1, as the microstep reads it.
        val transitionS6At1 = EnabledTransition<Test504State, HistoryId>(
            Test504State.S6,
            listOf(StateTarget(Test504State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test504State? = when (stateId) {
        "fail" -> Test504State.Fail
        "p" -> Test504State.P
        "pass" -> Test504State.Pass
        "ps1" -> Test504State.Ps1
        "ps2" -> Test504State.Ps2
        "s1" -> Test504State.S1
        "s2" -> Test504State.S2
        "s3" -> Test504State.S3
        "s4" -> Test504State.S4
        "s5" -> Test504State.S5
        "s6" -> Test504State.S6
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test504State): String = when (state) {
        is Test504State.Fail -> "fail"
        is Test504State.P -> "p"
        is Test504State.Pass -> "pass"
        is Test504State.Ps1 -> "ps1"
        is Test504State.Ps2 -> "ps2"
        is Test504State.S1 -> "s1"
        is Test504State.S2 -> "s2"
        is Test504State.S3 -> "s3"
        is Test504State.S4 -> "s4"
        is Test504State.S5 -> "s5"
        is Test504State.S6 -> "s6"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test504State): Int = when (state) {
        is Test504State.Fail -> 10
        is Test504State.P -> 2
        is Test504State.Pass -> 9
        is Test504State.Ps1 -> 3
        is Test504State.Ps2 -> 4
        is Test504State.S1 -> 0
        is Test504State.S2 -> 1
        is Test504State.S3 -> 5
        is Test504State.S4 -> 6
        is Test504State.S5 -> 7
        is Test504State.S6 -> 8
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test504Event? = when (name) {
        "bar" -> Test504Event.Bar
        "error.execution" -> Test504Event.Error.Execution
        "foo" -> Test504Event.Foo
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test504Event): String? = when (event) {
        is Test504Event.Bar -> "bar"
        is Test504Event.Error.Execution -> "error.execution"
        is Test504Event.Foo -> "foo"
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
            "test504",
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
        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raisePlatformError(Test504Event.Error.Execution, "<data id='Var1'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'Var2' with expr
        try {
            val initResult_Var2 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "Var2", initResult_Var2)
        } catch (e: Exception) {
            raisePlatformError(Test504Event.Error.Execution, "<data id='Var2'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'Var3' with expr
        try {
            val initResult_Var3 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "Var3", initResult_Var3)
        } catch (e: Exception) {
            raisePlatformError(Test504Event.Error.Execution, "<data id='Var3'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'Var4' with expr
        try {
            val initResult_Var4 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "Var4", initResult_Var4)
        } catch (e: Exception) {
            raisePlatformError(Test504Event.Error.Execution, "<data id='Var4'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'Var5' with expr
        try {
            val initResult_Var5 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "Var5", initResult_Var5)
        } catch (e: Exception) {
            raisePlatformError(Test504Event.Error.Execution, "<data id='Var5'> expr failed to evaluate")
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
            raisePlatformError(Test504Event.Error.Execution, reason)
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
            raisePlatformError(Test504Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test504Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test504Event.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test504Event) {
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
    override fun bindCurrentEvent(event: Test504Event) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test504State, event: Test504Event?): EnabledTransition<Test504State, HistoryId>? = when (state) {
        is Test504State.P -> firstEnabledTransitionOfP(state, event)
        is Test504State.S1 -> firstEnabledTransitionOfS1(state, event)
        is Test504State.S3 -> firstEnabledTransitionOfS3(state, event)
        is Test504State.S4 -> firstEnabledTransitionOfS4(state, event)
        is Test504State.S5 -> firstEnabledTransitionOfS5(state, event)
        is Test504State.S6 -> firstEnabledTransitionOfS6(state, event)
        else -> null
    }


    // p's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfP(state: Test504State, event: Test504Event?): EnabledTransition<Test504State, HistoryId>? = when {
        event is Test504Event.Foo -> TransitionsPart0.transitionPAt0
        event is Test504Event.Bar && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(Var4, 1)", "Var4 == 1")) -> TransitionsPart0.transitionPAt1
        event is Test504Event.Bar -> TransitionsPart0.transitionPAt2
        else -> null
    }


    // s1's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS1(state: Test504State, event: Test504Event?): EnabledTransition<Test504State, HistoryId>? = when {
        event == null -> TransitionsPart0.transitionS1At0
        else -> null
    }


    // s3's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS3(state: Test504State, event: Test504Event?): EnabledTransition<Test504State, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(Var1, 2)", "Var1 == 2")) -> TransitionsPart0.transitionS3At0
        event == null -> TransitionsPart0.transitionS3At1
        else -> null
    }


    // s4's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS4(state: Test504State, event: Test504Event?): EnabledTransition<Test504State, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(Var2, 2)", "Var2 == 2")) -> TransitionsPart0.transitionS4At0
        event == null -> TransitionsPart0.transitionS4At1
        else -> null
    }


    // s5's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS5(state: Test504State, event: Test504Event?): EnabledTransition<Test504State, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(Var3, 2)", "Var3 == 2")) -> TransitionsPart0.transitionS5At0
        event == null -> TransitionsPart0.transitionS5At1
        else -> null
    }


    // s6's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS6(state: Test504State, event: Test504Event?): EnabledTransition<Test504State, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(Var5, 1)", "Var5 == 1")) -> TransitionsPart0.transitionS6At0
        event == null -> TransitionsPart0.transitionS6At1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test504.scxml:5 :: _machine
    override fun onEntry(state: Test504State, isDefaultEntry: Boolean) {
        when (state) {
            is Test504State.Fail -> onEntryOfFail()
            is Test504State.P -> onEntryOfP()
            is Test504State.Pass -> onEntryOfPass()
            is Test504State.Ps1 -> onEntryOfPs1()
            is Test504State.Ps2 -> onEntryOfPs2()
            is Test504State.S1 -> onEntryOfS1()
            is Test504State.S2 -> onEntryOfS2()
            is Test504State.S3 -> onEntryOfS3()
            is Test504State.S4 -> onEntryOfS4()
            is Test504State.S5 -> onEntryOfS5()
            is Test504State.S6 -> onEntryOfS6()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test504.scxml:77 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfP() {
                // SCE-MAP: test504.scxml:27 :: p :: _state_body
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test504.scxml:76 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPs1() {
                // SCE-MAP: test504.scxml:39 :: ps1 :: _state_body
    }

    private fun onEntryOfPs2() {
                // SCE-MAP: test504.scxml:44 :: ps2 :: _state_body
    }

    private fun onEntryOfS1() {
                // SCE-MAP: test504.scxml:14 :: s1 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            raiseInternal(Test504Event.Foo)

            raiseInternal(Test504Event.Bar)
                }
    }

    private fun onEntryOfS2() {
                // SCE-MAP: test504.scxml:22 :: s2 :: _state_body
    }

    private fun onEntryOfS3() {
                // SCE-MAP: test504.scxml:52 :: s3 :: _state_body
    }

    private fun onEntryOfS4() {
                // SCE-MAP: test504.scxml:58 :: s4 :: _state_body
    }

    private fun onEntryOfS5() {
                // SCE-MAP: test504.scxml:64 :: s5 :: _state_body
    }

    private fun onEntryOfS6() {
                // SCE-MAP: test504.scxml:70 :: s6 :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test504.scxml:5 :: _machine
    override fun onExit(state: Test504State) {
        when (state) {
            is Test504State.Fail -> onExitOfFail()
            is Test504State.P -> onExitOfP()
            is Test504State.Pass -> onExitOfPass()
            is Test504State.Ps1 -> onExitOfPs1()
            is Test504State.Ps2 -> onExitOfPs2()
            is Test504State.S1 -> onExitOfS1()
            is Test504State.S2 -> onExitOfS2()
            is Test504State.S3 -> onExitOfS3()
            is Test504State.S4 -> onExitOfS4()
            is Test504State.S5 -> onExitOfS5()
            is Test504State.S6 -> onExitOfS6()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test504.scxml:77 :: fail :: _state_body
    }

    private fun onExitOfP() {
                // SCE-MAP: test504.scxml:27 :: p :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("Var1", "Var1"), com.sce.runtime.ScriptSource.lua("_scxml_add(Var1, 1)", "Var1 + 1"))) {
                return@run
            }
                }
    }

    private fun onExitOfPass() {
                // SCE-MAP: test504.scxml:76 :: pass :: _state_body
    }

    private fun onExitOfPs1() {
                // SCE-MAP: test504.scxml:39 :: ps1 :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("Var2", "Var2"), com.sce.runtime.ScriptSource.lua("_scxml_add(Var2, 1)", "Var2 + 1"))) {
                return@run
            }
                }
    }

    private fun onExitOfPs2() {
                // SCE-MAP: test504.scxml:44 :: ps2 :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("Var3", "Var3"), com.sce.runtime.ScriptSource.lua("_scxml_add(Var3, 1)", "Var3 + 1"))) {
                return@run
            }
                }
    }

    private fun onExitOfS1() {
                // SCE-MAP: test504.scxml:14 :: s1 :: _state_body
    }

    private fun onExitOfS2() {
                // SCE-MAP: test504.scxml:22 :: s2 :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("Var5", "Var5"), com.sce.runtime.ScriptSource.lua("_scxml_add(Var5, 1)", "Var5 + 1"))) {
                return@run
            }
                }
    }

    private fun onExitOfS3() {
                // SCE-MAP: test504.scxml:52 :: s3 :: _state_body
    }

    private fun onExitOfS4() {
                // SCE-MAP: test504.scxml:58 :: s4 :: _state_body
    }

    private fun onExitOfS5() {
                // SCE-MAP: test504.scxml:64 :: s5 :: _state_body
    }

    private fun onExitOfS6() {
                // SCE-MAP: test504.scxml:70 :: s6 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test504.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test504State, transitionIndex: Int) {
        when (source) {
            is Test504State.P -> transitionContentOfP(transitionIndex)
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
            else -> {}
        }
    }


    private fun transitionContentOfPAt0() {
                // SCE-MAP: test504.scxml:31 :: p :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("Var4", "Var4"), com.sce.runtime.ScriptSource.lua("_scxml_add(Var4, 1)", "Var4 + 1"))) {
                return
            }
    }
}
