// SCE-GENERATED — DO NOT EDIT
// source-hash: 1c418d46a68c5397364cc0ec51e4f76987276c6c6be13f06fa56c7db7b51179f

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/an_origin_is_the_address_its_sender_published/an_origin_is_the_address_its_sender_published.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:34 :: _machine

package com.sce.integration.an_origin_is_the_address_its_sender_published

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AnOriginIsTheAddressItsSenderPublishedState : State {
    data object AwaitHello : AnOriginIsTheAddressItsSenderPublishedState
    data object AwaitHiBack : AnOriginIsTheAddressItsSenderPublishedState
    data object AwaitWelcomed : AnOriginIsTheAddressItsSenderPublishedState
    data object Done : AnOriginIsTheAddressItsSenderPublishedState
    data object Family : AnOriginIsTheAddressItsSenderPublishedState
    data object Run : AnOriginIsTheAddressItsSenderPublishedState
    data object Self_ : AnOriginIsTheAddressItsSenderPublishedState
    data object SelfReply : AnOriginIsTheAddressItsSenderPublishedState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AnOriginIsTheAddressItsSenderPublishedEvent : Event {
    sealed interface Done : AnOriginIsTheAddressItsSenderPublishedEvent {
        data object Invoke : Done
    }
    sealed interface Error : AnOriginIsTheAddressItsSenderPublishedEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Hello : AnOriginIsTheAddressItsSenderPublishedEvent
    data object Hi : AnOriginIsTheAddressItsSenderPublishedEvent
    data object HiBack : AnOriginIsTheAddressItsSenderPublishedEvent
    data object HiWrong : AnOriginIsTheAddressItsSenderPublishedEvent
    data object Ping : AnOriginIsTheAddressItsSenderPublishedEvent
    data object Pong : AnOriginIsTheAddressItsSenderPublishedEvent
    data object Welcome : AnOriginIsTheAddressItsSenderPublishedEvent
    data object Welcomed : AnOriginIsTheAddressItsSenderPublishedEvent
}
// --- State Machine (W3C SCXML) ---

class AnOriginIsTheAddressItsSenderPublishedStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<AnOriginIsTheAddressItsSenderPublishedState, AnOriginIsTheAddressItsSenderPublishedEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `processor` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `processor` was assigned a value of another type, or the engine refused.
     */
    fun processor(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "processor")

    /**
     * §scxml-5.3: what the `childLocation` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `childLocation` was assigned a value of another type, or the engine refused.
     */
    fun childLocation(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "childLocation")

    /**
     * §scxml-5.3: what the `selfOk` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `selfOk` was assigned a value of another type, or the engine refused.
     */
    fun selfOk(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "selfOk")

    /**
     * §scxml-5.3: what the `selfReplied` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `selfReplied` was assigned a value of another type, or the engine refused.
     */
    fun selfReplied(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "selfReplied")

    /**
     * §scxml-5.3: what the `childOk` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `childOk` was assigned a value of another type, or the engine refused.
     */
    fun childOk(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "childOk")

    /**
     * §scxml-5.3: what the `childReplied` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `childReplied` was assigned a value of another type, or the engine refused.
     */
    fun childReplied(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "childReplied")

    /**
     * §scxml-5.3: what the `parentOk` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `parentOk` was assigned a value of another type, or the engine refused.
     */
    fun parentOk(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "parentOk")

    /**
     * §scxml-5.3: what the `parentReplied` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `parentReplied` was assigned a value of another type, or the engine refused.
     */
    fun parentReplied(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "parentReplied")

    override val initialState: AnOriginIsTheAddressItsSenderPublishedState = AnOriginIsTheAddressItsSenderPublishedState.Self_

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

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
    override fun parentOf(state: AnOriginIsTheAddressItsSenderPublishedState): AnOriginIsTheAddressItsSenderPublishedState? = when (state) {
        is AnOriginIsTheAddressItsSenderPublishedState.AwaitHello -> AnOriginIsTheAddressItsSenderPublishedState.Family
        is AnOriginIsTheAddressItsSenderPublishedState.AwaitHiBack -> AnOriginIsTheAddressItsSenderPublishedState.Family
        is AnOriginIsTheAddressItsSenderPublishedState.AwaitWelcomed -> AnOriginIsTheAddressItsSenderPublishedState.Family
        is AnOriginIsTheAddressItsSenderPublishedState.Family -> AnOriginIsTheAddressItsSenderPublishedState.Run
        is AnOriginIsTheAddressItsSenderPublishedState.Self_ -> AnOriginIsTheAddressItsSenderPublishedState.Run
        is AnOriginIsTheAddressItsSenderPublishedState.SelfReply -> AnOriginIsTheAddressItsSenderPublishedState.Run
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: AnOriginIsTheAddressItsSenderPublishedState): Boolean = when (state) {
        is AnOriginIsTheAddressItsSenderPublishedState.Family -> true
        is AnOriginIsTheAddressItsSenderPublishedState.Run -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: AnOriginIsTheAddressItsSenderPublishedState): Boolean = when (state) {
        is AnOriginIsTheAddressItsSenderPublishedState.Done -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: AnOriginIsTheAddressItsSenderPublishedState): List<AnOriginIsTheAddressItsSenderPublishedState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: AnOriginIsTheAddressItsSenderPublishedState): List<EntryTarget<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<AnOriginIsTheAddressItsSenderPublishedState, List<AnOriginIsTheAddressItsSenderPublishedState>> = LinkedHashMap<AnOriginIsTheAddressItsSenderPublishedState, List<AnOriginIsTheAddressItsSenderPublishedState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<AnOriginIsTheAddressItsSenderPublishedState, List<AnOriginIsTheAddressItsSenderPublishedState>>) {
            table[AnOriginIsTheAddressItsSenderPublishedState.Family] = listOf(AnOriginIsTheAddressItsSenderPublishedState.AwaitHello, AnOriginIsTheAddressItsSenderPublishedState.AwaitWelcomed, AnOriginIsTheAddressItsSenderPublishedState.AwaitHiBack)
            table[AnOriginIsTheAddressItsSenderPublishedState.Run] = listOf(AnOriginIsTheAddressItsSenderPublishedState.Self_, AnOriginIsTheAddressItsSenderPublishedState.SelfReply, AnOriginIsTheAddressItsSenderPublishedState.Family)
        }


        val initialTargets: Map<AnOriginIsTheAddressItsSenderPublishedState, List<EntryTarget<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>>> = LinkedHashMap<AnOriginIsTheAddressItsSenderPublishedState, List<EntryTarget<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<AnOriginIsTheAddressItsSenderPublishedState, List<EntryTarget<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>>>) {
            table[AnOriginIsTheAddressItsSenderPublishedState.Family] = listOf(StateTarget(AnOriginIsTheAddressItsSenderPublishedState.AwaitHello))
            table[AnOriginIsTheAddressItsSenderPublishedState.Run] = listOf(StateTarget(AnOriginIsTheAddressItsSenderPublishedState.Self_))
        }


        val documentInitialTargetList: List<EntryTarget<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>> =
            listOf(StateTarget(AnOriginIsTheAddressItsSenderPublishedState.Run))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: awaitHello's transition 0, as the microstep reads it.
        val transitionAwaitHelloAt0 = EnabledTransition<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>(
            AnOriginIsTheAddressItsSenderPublishedState.AwaitHello,
            listOf(StateTarget(AnOriginIsTheAddressItsSenderPublishedState.AwaitWelcomed)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: awaitHiBack's transition 0, as the microstep reads it.
        val transitionAwaitHiBackAt0 = EnabledTransition<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>(
            AnOriginIsTheAddressItsSenderPublishedState.AwaitHiBack,
            listOf(StateTarget(AnOriginIsTheAddressItsSenderPublishedState.Done)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: awaitHiBack's transition 1, as the microstep reads it.
        val transitionAwaitHiBackAt1 = EnabledTransition<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>(
            AnOriginIsTheAddressItsSenderPublishedState.AwaitHiBack,
            listOf(StateTarget(AnOriginIsTheAddressItsSenderPublishedState.Done)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: awaitWelcomed's transition 0, as the microstep reads it.
        val transitionAwaitWelcomedAt0 = EnabledTransition<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>(
            AnOriginIsTheAddressItsSenderPublishedState.AwaitWelcomed,
            listOf(StateTarget(AnOriginIsTheAddressItsSenderPublishedState.AwaitHiBack)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: self's transition 0, as the microstep reads it.
        val transitionSelf_At0 = EnabledTransition<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>(
            AnOriginIsTheAddressItsSenderPublishedState.Self_,
            listOf(StateTarget(AnOriginIsTheAddressItsSenderPublishedState.SelfReply)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: selfReply's transition 0, as the microstep reads it.
        val transitionSelfReplyAt0 = EnabledTransition<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>(
            AnOriginIsTheAddressItsSenderPublishedState.SelfReply,
            listOf(StateTarget(AnOriginIsTheAddressItsSenderPublishedState.Family)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AnOriginIsTheAddressItsSenderPublishedState? = when (stateId) {
        "awaitHello" -> AnOriginIsTheAddressItsSenderPublishedState.AwaitHello
        "awaitHiBack" -> AnOriginIsTheAddressItsSenderPublishedState.AwaitHiBack
        "awaitWelcomed" -> AnOriginIsTheAddressItsSenderPublishedState.AwaitWelcomed
        "done" -> AnOriginIsTheAddressItsSenderPublishedState.Done
        "family" -> AnOriginIsTheAddressItsSenderPublishedState.Family
        "run" -> AnOriginIsTheAddressItsSenderPublishedState.Run
        "self" -> AnOriginIsTheAddressItsSenderPublishedState.Self_
        "selfReply" -> AnOriginIsTheAddressItsSenderPublishedState.SelfReply
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AnOriginIsTheAddressItsSenderPublishedState): String = when (state) {
        is AnOriginIsTheAddressItsSenderPublishedState.AwaitHello -> "awaitHello"
        is AnOriginIsTheAddressItsSenderPublishedState.AwaitHiBack -> "awaitHiBack"
        is AnOriginIsTheAddressItsSenderPublishedState.AwaitWelcomed -> "awaitWelcomed"
        is AnOriginIsTheAddressItsSenderPublishedState.Done -> "done"
        is AnOriginIsTheAddressItsSenderPublishedState.Family -> "family"
        is AnOriginIsTheAddressItsSenderPublishedState.Run -> "run"
        is AnOriginIsTheAddressItsSenderPublishedState.Self_ -> "self"
        is AnOriginIsTheAddressItsSenderPublishedState.SelfReply -> "selfReply"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AnOriginIsTheAddressItsSenderPublishedState): Int = when (state) {
        is AnOriginIsTheAddressItsSenderPublishedState.AwaitHello -> 4
        is AnOriginIsTheAddressItsSenderPublishedState.AwaitHiBack -> 6
        is AnOriginIsTheAddressItsSenderPublishedState.AwaitWelcomed -> 5
        is AnOriginIsTheAddressItsSenderPublishedState.Done -> 7
        is AnOriginIsTheAddressItsSenderPublishedState.Family -> 3
        is AnOriginIsTheAddressItsSenderPublishedState.Run -> 0
        is AnOriginIsTheAddressItsSenderPublishedState.Self_ -> 1
        is AnOriginIsTheAddressItsSenderPublishedState.SelfReply -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AnOriginIsTheAddressItsSenderPublishedEvent? = when (name) {
        "done.invoke" -> AnOriginIsTheAddressItsSenderPublishedEvent.Done.Invoke
        "error.communication" -> AnOriginIsTheAddressItsSenderPublishedEvent.Error.Communication
        "error.execution" -> AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution
        "hello" -> AnOriginIsTheAddressItsSenderPublishedEvent.Hello
        "hi" -> AnOriginIsTheAddressItsSenderPublishedEvent.Hi
        "hiBack" -> AnOriginIsTheAddressItsSenderPublishedEvent.HiBack
        "hiWrong" -> AnOriginIsTheAddressItsSenderPublishedEvent.HiWrong
        "ping" -> AnOriginIsTheAddressItsSenderPublishedEvent.Ping
        "pong" -> AnOriginIsTheAddressItsSenderPublishedEvent.Pong
        "welcome" -> AnOriginIsTheAddressItsSenderPublishedEvent.Welcome
        "welcomed" -> AnOriginIsTheAddressItsSenderPublishedEvent.Welcomed
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AnOriginIsTheAddressItsSenderPublishedEvent): String? = when (event) {
        is AnOriginIsTheAddressItsSenderPublishedEvent.Done.Invoke -> "done.invoke"
        is AnOriginIsTheAddressItsSenderPublishedEvent.Error.Communication -> "error.communication"
        is AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution -> "error.execution"
        is AnOriginIsTheAddressItsSenderPublishedEvent.Hello -> "hello"
        is AnOriginIsTheAddressItsSenderPublishedEvent.Hi -> "hi"
        is AnOriginIsTheAddressItsSenderPublishedEvent.HiBack -> "hiBack"
        is AnOriginIsTheAddressItsSenderPublishedEvent.HiWrong -> "hiWrong"
        is AnOriginIsTheAddressItsSenderPublishedEvent.Ping -> "ping"
        is AnOriginIsTheAddressItsSenderPublishedEvent.Pong -> "pong"
        is AnOriginIsTheAddressItsSenderPublishedEvent.Welcome -> "welcome"
        is AnOriginIsTheAddressItsSenderPublishedEvent.Welcomed -> "welcomed"
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
            "an_origin_is_the_address_its_sender_published",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        declareVariablesPart0(engine, sid)




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

    @Suppress("UNUSED_PARAMETER")
    private fun declareVariablesPart0(engine: ScxmlScriptEngine, sid: String) {
        // W3C SCXML 5.3: Initialize variable 'processor' with expr
        try {
            val initResult_processor = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"http://www.w3.org/TR/scxml/#SCXMLEventProcessor\"", "'http://www.w3.org/TR/scxml/#SCXMLEventProcessor'"))
            engine.setVariable(sid, "processor", initResult_processor)
        } catch (e: Exception) {
            raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<data id='processor'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'childLocation' with expr
        try {
            val initResult_childLocation = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''"))
            engine.setVariable(sid, "childLocation", initResult_childLocation)
        } catch (e: Exception) {
            raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<data id='childLocation'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'selfOk' with expr
        try {
            val initResult_selfOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "selfOk", initResult_selfOk)
        } catch (e: Exception) {
            raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<data id='selfOk'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'selfReplied' with expr
        try {
            val initResult_selfReplied = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "selfReplied", initResult_selfReplied)
        } catch (e: Exception) {
            raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<data id='selfReplied'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'childOk' with expr
        try {
            val initResult_childOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "childOk", initResult_childOk)
        } catch (e: Exception) {
            raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<data id='childOk'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'childReplied' with expr
        try {
            val initResult_childReplied = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "childReplied", initResult_childReplied)
        } catch (e: Exception) {
            raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<data id='childReplied'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'parentOk' with expr
        try {
            val initResult_parentOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "parentOk", initResult_parentOk)
        } catch (e: Exception) {
            raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<data id='parentOk'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'parentReplied' with expr
        try {
            val initResult_parentReplied = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "parentReplied", initResult_parentReplied)
        } catch (e: Exception) {
            raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<data id='parentReplied'> expr failed to evaluate")
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
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, reason)
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
            raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AnOriginIsTheAddressItsSenderPublishedEvent) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
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
    override fun bindCurrentEvent(event: AnOriginIsTheAddressItsSenderPublishedEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: AnOriginIsTheAddressItsSenderPublishedState, event: AnOriginIsTheAddressItsSenderPublishedEvent?): EnabledTransition<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>? = when (state) {
        is AnOriginIsTheAddressItsSenderPublishedState.AwaitHello -> firstEnabledTransitionOfAwaitHello(state, event)
        is AnOriginIsTheAddressItsSenderPublishedState.AwaitHiBack -> firstEnabledTransitionOfAwaitHiBack(state, event)
        is AnOriginIsTheAddressItsSenderPublishedState.AwaitWelcomed -> firstEnabledTransitionOfAwaitWelcomed(state, event)
        is AnOriginIsTheAddressItsSenderPublishedState.Self_ -> firstEnabledTransitionOfSelf_(state, event)
        is AnOriginIsTheAddressItsSenderPublishedState.SelfReply -> firstEnabledTransitionOfSelfReply(state, event)
        else -> null
    }


    // awaitHello's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAwaitHello(state: AnOriginIsTheAddressItsSenderPublishedState, event: AnOriginIsTheAddressItsSenderPublishedEvent?): EnabledTransition<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>? = when {
        event is AnOriginIsTheAddressItsSenderPublishedEvent.Hello -> TransitionsPart0.transitionAwaitHelloAt0
        else -> null
    }


    // awaitHiBack's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAwaitHiBack(state: AnOriginIsTheAddressItsSenderPublishedState, event: AnOriginIsTheAddressItsSenderPublishedEvent?): EnabledTransition<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>? = when {
        event is AnOriginIsTheAddressItsSenderPublishedEvent.HiBack -> TransitionsPart0.transitionAwaitHiBackAt0
        event is AnOriginIsTheAddressItsSenderPublishedEvent.HiWrong -> TransitionsPart0.transitionAwaitHiBackAt1
        else -> null
    }


    // awaitWelcomed's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAwaitWelcomed(state: AnOriginIsTheAddressItsSenderPublishedState, event: AnOriginIsTheAddressItsSenderPublishedEvent?): EnabledTransition<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>? = when {
        event is AnOriginIsTheAddressItsSenderPublishedEvent.Welcomed -> TransitionsPart0.transitionAwaitWelcomedAt0
        else -> null
    }


    // self's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSelf_(state: AnOriginIsTheAddressItsSenderPublishedState, event: AnOriginIsTheAddressItsSenderPublishedEvent?): EnabledTransition<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>? = when {
        event is AnOriginIsTheAddressItsSenderPublishedEvent.Ping -> TransitionsPart0.transitionSelf_At0
        else -> null
    }


    // selfReply's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSelfReply(state: AnOriginIsTheAddressItsSenderPublishedState, event: AnOriginIsTheAddressItsSenderPublishedEvent?): EnabledTransition<AnOriginIsTheAddressItsSenderPublishedState, HistoryId>? = when {
        event is AnOriginIsTheAddressItsSenderPublishedEvent.Pong -> TransitionsPart0.transitionSelfReplyAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:34 :: _machine
    override fun onEntry(state: AnOriginIsTheAddressItsSenderPublishedState, isDefaultEntry: Boolean) {
        when (state) {
            is AnOriginIsTheAddressItsSenderPublishedState.AwaitHello -> onEntryOfAwaitHello()
            is AnOriginIsTheAddressItsSenderPublishedState.AwaitHiBack -> onEntryOfAwaitHiBack()
            is AnOriginIsTheAddressItsSenderPublishedState.AwaitWelcomed -> onEntryOfAwaitWelcomed()
            is AnOriginIsTheAddressItsSenderPublishedState.Done -> onEntryOfDone()
            is AnOriginIsTheAddressItsSenderPublishedState.Family -> onEntryOfFamily()
            is AnOriginIsTheAddressItsSenderPublishedState.Run -> onEntryOfRun()
            is AnOriginIsTheAddressItsSenderPublishedState.Self_ -> onEntryOfSelf_()
            is AnOriginIsTheAddressItsSenderPublishedState.SelfReply -> onEntryOfSelfReply()
        }
    }


    private fun onEntryOfAwaitHello() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:103 :: awaitHello :: _state_body
    }

    private fun onEntryOfAwaitHiBack() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:122 :: awaitHiBack :: _state_body
    }

    private fun onEntryOfAwaitWelcomed() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:113 :: awaitWelcomed :: _state_body
    }

    private fun onEntryOfDone() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:134 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFamily() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:70 :: family :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("kid", AnOriginIsTheAddressItsSenderPublishedState.Family)
    }

    private fun onEntryOfRun() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:48 :: run :: _state_body
    }

    private fun onEntryOfSelf_() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:49 :: self :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(AnOriginIsTheAddressItsSenderPublishedEvent.Ping, EventMetadata.sentBy(sessionId, sendId = "__send_1", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfSelfReply() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:61 :: selfReply :: _state_body
    }

    // W3C SCXML 6.4: defer the start of the child session of the
    // `<invoke type="scxml">` (a static child, or a hybrid one that names its
    // child by `srcexpr`) `invokeId`, held by `state`, to the macrostep's
    // end. One body for the two things that start a child: entering the state
    // (`onEntry` above) and a restore, which starts again each running
    // invocation a saved state lists (`restartInvoke`). The child is not saved,
    // so what a restore needs is exactly what entering the state does, and two
    // spellings of it would be two places a change to one is forgotten in the
    // other. A state that exits before the macrostep ends cancels the entry
    // (`cancelPendingInvokesForState`) in either case.
    //
    // Each invoke's start is a method of its own (`startInvokeOf<State>At<n>`,
    // `n` its place among its state's invokes), and this dispatches to it with
    // one call an arm, for the reason `onEntry` does.
    private fun deferStaticInvoke(invokeId: String, state: AnOriginIsTheAddressItsSenderPublishedState) {
        when (invokeId) {
            "kid" -> startInvokeOfFamilyAt0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfFamilyAt0(state: AnOriginIsTheAddressItsSenderPublishedState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "family.${nextInvokeStart("kid")}.kid"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(AnOriginIsTheAddressItsSenderPublishedSceSynthInvokeKidStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)")), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("kid", childSM, false, AnOriginIsTheAddressItsSenderPublishedEvent.Done.Invoke, "", generatedInvokeId, "", "an_origin_is_the_address_its_sender_published__sce_synth_invoke__kid", "kid")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:34 :: _machine
    override fun onExit(state: AnOriginIsTheAddressItsSenderPublishedState) {
        when (state) {
            is AnOriginIsTheAddressItsSenderPublishedState.AwaitHello -> onExitOfAwaitHello()
            is AnOriginIsTheAddressItsSenderPublishedState.AwaitHiBack -> onExitOfAwaitHiBack()
            is AnOriginIsTheAddressItsSenderPublishedState.AwaitWelcomed -> onExitOfAwaitWelcomed()
            is AnOriginIsTheAddressItsSenderPublishedState.Done -> onExitOfDone()
            is AnOriginIsTheAddressItsSenderPublishedState.Family -> onExitOfFamily()
            is AnOriginIsTheAddressItsSenderPublishedState.Run -> onExitOfRun()
            is AnOriginIsTheAddressItsSenderPublishedState.Self_ -> onExitOfSelf_()
            is AnOriginIsTheAddressItsSenderPublishedState.SelfReply -> onExitOfSelfReply()
        }
    }


    private fun onExitOfAwaitHello() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:103 :: awaitHello :: _state_body
    }

    private fun onExitOfAwaitHiBack() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:122 :: awaitHiBack :: _state_body
    }

    private fun onExitOfAwaitWelcomed() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:113 :: awaitWelcomed :: _state_body
    }

    private fun onExitOfDone() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:134 :: done :: _state_body
    }

    private fun onExitOfFamily() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:70 :: family :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(AnOriginIsTheAddressItsSenderPublishedState.Family)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("kid")
    }

    private fun onExitOfRun() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:48 :: run :: _state_body
    }

    private fun onExitOfSelf_() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:49 :: self :: _state_body
    }

    private fun onExitOfSelfReply() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:61 :: selfReply :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:34 :: _machine
    override fun executeTransitionContent(source: AnOriginIsTheAddressItsSenderPublishedState, transitionIndex: Int) {
        when (source) {
            is AnOriginIsTheAddressItsSenderPublishedState.AwaitHello -> transitionContentOfAwaitHello(transitionIndex)
            is AnOriginIsTheAddressItsSenderPublishedState.AwaitHiBack -> transitionContentOfAwaitHiBack(transitionIndex)
            is AnOriginIsTheAddressItsSenderPublishedState.AwaitWelcomed -> transitionContentOfAwaitWelcomed(transitionIndex)
            is AnOriginIsTheAddressItsSenderPublishedState.Self_ -> transitionContentOfSelf_(transitionIndex)
            is AnOriginIsTheAddressItsSenderPublishedState.SelfReply -> transitionContentOfSelfReply(transitionIndex)
            else -> {}
        }
    }


    // The content of `awaitHello`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfAwaitHello(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfAwaitHelloAt0()
            else -> {}
        }
    }


    private fun transitionContentOfAwaitHelloAt0() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:104 :: awaitHello :: _transition_0


            var ifCondFailed2 = false
            if (evaluateIfCond(com.sce.runtime.ScriptSource.lua("((_event.origin == _event.data.location) and (_event.origintype == processor))", "_event.origin === _event.data.location && _event.origintype === processor")) { ifCondFailed2 = true }) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("childOk", "childOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            if (ifCondFailed2) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("childLocation", "childLocation"), com.sce.runtime.ScriptSource.lua("_event.origin", "_event.origin"))) {
                return
            }


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("_event.origin", "_event.origin")))
            } catch (_: Exception) {
                raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_2")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_2")
                return@send true
            }
            // W3C SCXML 6.2: a type the platform does not support is the same
            // error as one that cannot be evaluated.
            val sendTypeValue = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("_event.origintype", "_event.origintype")))
            } catch (_: Exception) {
                raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<send> typeexpr could not be evaluated", "__send_2")
                return@send true
            }
            if (!com.sce.runtime.SendHelper.isSupportedSendType(sendTypeValue)) {
                raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<send> typeexpr names a processor this platform does not support", "__send_2")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_2")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
AnOriginIsTheAddressItsSenderPublishedEvent.Welcome,
"welcome",
                sendData,
0L,
                "__send_2",
                AnOriginIsTheAddressItsSenderPublishedEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_2")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_2")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    // The content of `awaitHiBack`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfAwaitHiBack(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfAwaitHiBackAt0()
            else -> {}
        }
    }


    private fun transitionContentOfAwaitHiBackAt0() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:123 :: awaitHiBack :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("parentOk", "parentOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }


            var ifCondFailed3 = false
            if (evaluateIfCond(com.sce.runtime.ScriptSource.lua("(_event.origin == childLocation)", "_event.origin === childLocation")) { ifCondFailed3 = true }) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("parentReplied", "parentReplied"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            if (ifCondFailed3) {
                return
            }
    }

    // The content of `awaitWelcomed`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfAwaitWelcomed(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfAwaitWelcomedAt0()
            else -> {}
        }
    }


    private fun transitionContentOfAwaitWelcomedAt0() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:114 :: awaitWelcomed :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("childReplied", "childReplied"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }


            if (run send@{
            var paramFailed = false
            ensureScriptEngine()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "location", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("_ioprocessors.scxml.location", "_ioprocessors['scxml'].location")))
            } catch (_: Exception) {
                raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<send> <param name='location'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.4 (test192): Send event to invoked child. The build
            // refused a target naming no invocation above; one that is not
            // running is not there to reach (W3C SCXML C.1).
            if (!sendToChild("kid", "hi", sendData)) {
                raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Communication, "<send target='#_kid'> names an invocation that is not running", "__send_3")
                return@send true
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    // The content of `self`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfSelf_(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfSelf_At0()
            else -> {}
        }
    }


    private fun transitionContentOfSelf_At0() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:53 :: self :: _transition_0


            var ifCondFailed1 = false
            if (evaluateIfCond(com.sce.runtime.ScriptSource.lua("((_event.origin == _ioprocessors.scxml.location) and (_event.origintype == processor))", "_event.origin === _ioprocessors['scxml'].location && _event.origintype === processor")) { ifCondFailed1 = true }) {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("selfOk", "selfOk"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            if (ifCondFailed1) {
                return
            }


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("_event.origin", "_event.origin")))
            } catch (_: Exception) {
                raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_0")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_0")
                return@send true
            }
            // W3C SCXML 6.2: a type the platform does not support is the same
            // error as one that cannot be evaluated.
            val sendTypeValue = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("_event.origintype", "_event.origintype")))
            } catch (_: Exception) {
                raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<send> typeexpr could not be evaluated", "__send_0")
                return@send true
            }
            if (!com.sce.runtime.SendHelper.isSupportedSendType(sendTypeValue)) {
                raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<send> typeexpr names a processor this platform does not support", "__send_0")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_0")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
AnOriginIsTheAddressItsSenderPublishedEvent.Pong,
"pong",
                sendData,
0L,
                "__send_0",
                AnOriginIsTheAddressItsSenderPublishedEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_0")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(AnOriginIsTheAddressItsSenderPublishedEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_0")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    // The content of `selfReply`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfSelfReply(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfSelfReplyAt0()
            else -> {}
        }
    }


    private fun transitionContentOfSelfReplyAt0() {
                // SCE-MAP: an_origin_is_the_address_its_sender_published.scxml:62 :: selfReply :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("selfReplied", "selfReplied"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
    }
}
