// SCE-GENERATED — DO NOT EDIT
// source-hash: 727dc4e641e5354370e1916310a7e6dd3837a7ec7fa73ba2f0888d444be5f4e4

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_target_expression_is_routed_as_its_literal_is/a_target_expression_is_routed_as_its_literal_is.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:47 :: _machine

package com.sce.integration.a_target_expression_is_routed_as_its_literal_is

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ATargetExpressionIsRoutedAsItsLiteralIsState : State {
    data object Done : ATargetExpressionIsRoutedAsItsLiteralIsState
    data object Run : ATargetExpressionIsRoutedAsItsLiteralIsState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ATargetExpressionIsRoutedAsItsLiteralIsEvent : Event {
    sealed interface Done : ATargetExpressionIsRoutedAsItsLiteralIsEvent {
        data object Invoke : Done
    }
    sealed interface Error : ATargetExpressionIsRoutedAsItsLiteralIsEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Hello : ATargetExpressionIsRoutedAsItsLiteralIsEvent
    data object InLater : ATargetExpressionIsRoutedAsItsLiteralIsEvent
    data object InNow : ATargetExpressionIsRoutedAsItsLiteralIsEvent
    data object KAckLater : ATargetExpressionIsRoutedAsItsLiteralIsEvent
    data object KAckNow : ATargetExpressionIsRoutedAsItsLiteralIsEvent
    data object KLater : ATargetExpressionIsRoutedAsItsLiteralIsEvent
    data object KNow : ATargetExpressionIsRoutedAsItsLiteralIsEvent
    data object Lost : ATargetExpressionIsRoutedAsItsLiteralIsEvent
    data object SAckLater : ATargetExpressionIsRoutedAsItsLiteralIsEvent
    data object SAckNow : ATargetExpressionIsRoutedAsItsLiteralIsEvent
    data object SLater : ATargetExpressionIsRoutedAsItsLiteralIsEvent
    data object SNow : ATargetExpressionIsRoutedAsItsLiteralIsEvent
    data object Settle : ATargetExpressionIsRoutedAsItsLiteralIsEvent
}
// --- State Machine (W3C SCXML) ---

class ATargetExpressionIsRoutedAsItsLiteralIsStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ATargetExpressionIsRoutedAsItsLiteralIsState, ATargetExpressionIsRoutedAsItsLiteralIsEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `tInternal` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `tInternal` was assigned a value of another type, or the engine refused.
     */
    fun tInternal(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "tInternal")

    /**
     * §scxml-5.3: what the `tKid` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `tKid` was assigned a value of another type, or the engine refused.
     */
    fun tKid(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "tKid")

    /**
     * §scxml-5.3: what the `tStranger` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `tStranger` was assigned a value of another type, or the engine refused.
     */
    fun tStranger(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "tStranger")

    /**
     * §scxml-5.3: what the `tParent` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `tParent` was assigned a value of another type, or the engine refused.
     */
    fun tParent(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "tParent")

    /**
     * §scxml-5.3: what the `tBogus` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `tBogus` was assigned a value of another type, or the engine refused.
     */
    fun tBogus(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "tBogus")

    /**
     * §scxml-5.3: what the `kidLoc` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `kidLoc` was assigned a value of another type, or the engine refused.
     */
    fun kidLoc(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "kidLoc")

    /**
     * §scxml-5.3: what the `internalNow` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `internalNow` was assigned a value of another type, or the engine refused.
     */
    fun internalNow(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "internalNow")

    /**
     * §scxml-5.3: what the `internalLater` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `internalLater` was assigned a value of another type, or the engine refused.
     */
    fun internalLater(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "internalLater")

    /**
     * §scxml-5.3: what the `kidNow` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `kidNow` was assigned a value of another type, or the engine refused.
     */
    fun kidNow(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "kidNow")

    /**
     * §scxml-5.3: what the `kidLater` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `kidLater` was assigned a value of another type, or the engine refused.
     */
    fun kidLater(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "kidLater")

    /**
     * §scxml-5.3: what the `sessNow` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `sessNow` was assigned a value of another type, or the engine refused.
     */
    fun sessNow(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "sessNow")

    /**
     * §scxml-5.3: what the `sessLater` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `sessLater` was assigned a value of another type, or the engine refused.
     */
    fun sessLater(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "sessLater")

    /**
     * §scxml-5.3: what the `commErrors` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `commErrors` was assigned a value of another type, or the engine refused.
     */
    fun commErrors(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "commErrors")

    /**
     * §scxml-5.3: what the `execErrors` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `execErrors` was assigned a value of another type, or the engine refused.
     */
    fun execErrors(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "execErrors")

    /**
     * §scxml-5.3: what the `afterStranger` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterStranger` was assigned a value of another type, or the engine refused.
     */
    fun afterStranger(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterStranger")

    /**
     * §scxml-5.3: what the `afterStrangerLater` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterStrangerLater` was assigned a value of another type, or the engine refused.
     */
    fun afterStrangerLater(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterStrangerLater")

    /**
     * §scxml-5.3: what the `afterOrphan` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterOrphan` was assigned a value of another type, or the engine refused.
     */
    fun afterOrphan(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterOrphan")

    /**
     * §scxml-5.3: what the `afterOrphanLater` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterOrphanLater` was assigned a value of another type, or the engine refused.
     */
    fun afterOrphanLater(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterOrphanLater")

    /**
     * §scxml-5.3: what the `afterBogus` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterBogus` was assigned a value of another type, or the engine refused.
     */
    fun afterBogus(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterBogus")

    /**
     * §scxml-5.3: what the `afterBogusLater` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `afterBogusLater` was assigned a value of another type, or the engine refused.
     */
    fun afterBogusLater(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "afterBogusLater")

    override val initialState: ATargetExpressionIsRoutedAsItsLiteralIsState = ATargetExpressionIsRoutedAsItsLiteralIsState.Run

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

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: ATargetExpressionIsRoutedAsItsLiteralIsState): Boolean = when (state) {
        is ATargetExpressionIsRoutedAsItsLiteralIsState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ATargetExpressionIsRoutedAsItsLiteralIsState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<ATargetExpressionIsRoutedAsItsLiteralIsState, HistoryId>> =
            listOf(StateTarget(ATargetExpressionIsRoutedAsItsLiteralIsState.Run))

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<ATargetExpressionIsRoutedAsItsLiteralIsState, HistoryId>(
            ATargetExpressionIsRoutedAsItsLiteralIsState.Run,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 1, as the microstep reads it.
        val transitionRunAt1 = EnabledTransition<ATargetExpressionIsRoutedAsItsLiteralIsState, HistoryId>(
            ATargetExpressionIsRoutedAsItsLiteralIsState.Run,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 2, as the microstep reads it.
        val transitionRunAt2 = EnabledTransition<ATargetExpressionIsRoutedAsItsLiteralIsState, HistoryId>(
            ATargetExpressionIsRoutedAsItsLiteralIsState.Run,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 3, as the microstep reads it.
        val transitionRunAt3 = EnabledTransition<ATargetExpressionIsRoutedAsItsLiteralIsState, HistoryId>(
            ATargetExpressionIsRoutedAsItsLiteralIsState.Run,
            emptyList(),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 4, as the microstep reads it.
        val transitionRunAt4 = EnabledTransition<ATargetExpressionIsRoutedAsItsLiteralIsState, HistoryId>(
            ATargetExpressionIsRoutedAsItsLiteralIsState.Run,
            emptyList(),
            4,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 5, as the microstep reads it.
        val transitionRunAt5 = EnabledTransition<ATargetExpressionIsRoutedAsItsLiteralIsState, HistoryId>(
            ATargetExpressionIsRoutedAsItsLiteralIsState.Run,
            emptyList(),
            5,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 6, as the microstep reads it.
        val transitionRunAt6 = EnabledTransition<ATargetExpressionIsRoutedAsItsLiteralIsState, HistoryId>(
            ATargetExpressionIsRoutedAsItsLiteralIsState.Run,
            emptyList(),
            6,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 7, as the microstep reads it.
        val transitionRunAt7 = EnabledTransition<ATargetExpressionIsRoutedAsItsLiteralIsState, HistoryId>(
            ATargetExpressionIsRoutedAsItsLiteralIsState.Run,
            emptyList(),
            7,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 8, as the microstep reads it.
        val transitionRunAt8 = EnabledTransition<ATargetExpressionIsRoutedAsItsLiteralIsState, HistoryId>(
            ATargetExpressionIsRoutedAsItsLiteralIsState.Run,
            emptyList(),
            8,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 9, as the microstep reads it.
        val transitionRunAt9 = EnabledTransition<ATargetExpressionIsRoutedAsItsLiteralIsState, HistoryId>(
            ATargetExpressionIsRoutedAsItsLiteralIsState.Run,
            listOf(StateTarget(ATargetExpressionIsRoutedAsItsLiteralIsState.Done)),
            9,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ATargetExpressionIsRoutedAsItsLiteralIsState? = when (stateId) {
        "done" -> ATargetExpressionIsRoutedAsItsLiteralIsState.Done
        "run" -> ATargetExpressionIsRoutedAsItsLiteralIsState.Run
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ATargetExpressionIsRoutedAsItsLiteralIsState): String = when (state) {
        is ATargetExpressionIsRoutedAsItsLiteralIsState.Done -> "done"
        is ATargetExpressionIsRoutedAsItsLiteralIsState.Run -> "run"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ATargetExpressionIsRoutedAsItsLiteralIsState): Int = when (state) {
        is ATargetExpressionIsRoutedAsItsLiteralIsState.Done -> 1
        is ATargetExpressionIsRoutedAsItsLiteralIsState.Run -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ATargetExpressionIsRoutedAsItsLiteralIsEvent? = when (name) {
        "done.invoke" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.Done.Invoke
        "error.communication" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication
        "error.execution" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution
        "hello" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.Hello
        "inLater" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.InLater
        "inNow" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.InNow
        "kAckLater" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.KAckLater
        "kAckNow" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.KAckNow
        "kLater" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.KLater
        "kNow" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.KNow
        "lost" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.Lost
        "sAckLater" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.SAckLater
        "sAckNow" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.SAckNow
        "settle" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.Settle
        "sLater" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.SLater
        "sNow" -> ATargetExpressionIsRoutedAsItsLiteralIsEvent.SNow
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ATargetExpressionIsRoutedAsItsLiteralIsEvent): String? = when (event) {
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.Done.Invoke -> "done.invoke"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication -> "error.communication"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution -> "error.execution"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.Hello -> "hello"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.InLater -> "inLater"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.InNow -> "inNow"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.KAckLater -> "kAckLater"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.KAckNow -> "kAckNow"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.KLater -> "kLater"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.KNow -> "kNow"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.Lost -> "lost"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.SAckLater -> "sAckLater"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.SAckNow -> "sAckNow"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.Settle -> "settle"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.SLater -> "sLater"
        is ATargetExpressionIsRoutedAsItsLiteralIsEvent.SNow -> "sNow"
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
            "a_target_expression_is_routed_as_its_literal_is",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'tInternal' with expr
        try {
            val initResult_tInternal = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"#_internal\"", "'#_internal'"))
            engine.setVariable(sid, "tInternal", initResult_tInternal)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='tInternal'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'tKid' with expr
        try {
            val initResult_tKid = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"#_kid\"", "'#_kid'"))
            engine.setVariable(sid, "tKid", initResult_tKid)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='tKid'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'tStranger' with expr
        try {
            val initResult_tStranger = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"#_scxml_nosuch\"", "'#_scxml_nosuch'"))
            engine.setVariable(sid, "tStranger", initResult_tStranger)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='tStranger'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'tParent' with expr
        try {
            val initResult_tParent = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"#_parent\"", "'#_parent'"))
            engine.setVariable(sid, "tParent", initResult_tParent)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='tParent'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'tBogus' with expr
        try {
            val initResult_tBogus = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"bogus\"", "'bogus'"))
            engine.setVariable(sid, "tBogus", initResult_tBogus)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='tBogus'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'kidLoc' with expr
        try {
            val initResult_kidLoc = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''"))
            engine.setVariable(sid, "kidLoc", initResult_kidLoc)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='kidLoc'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'internalNow' with expr
        try {
            val initResult_internalNow = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "internalNow", initResult_internalNow)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='internalNow'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'internalLater' with expr
        try {
            val initResult_internalLater = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "internalLater", initResult_internalLater)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='internalLater'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'kidNow' with expr
        try {
            val initResult_kidNow = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "kidNow", initResult_kidNow)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='kidNow'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'kidLater' with expr
        try {
            val initResult_kidLater = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "kidLater", initResult_kidLater)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='kidLater'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'sessNow' with expr
        try {
            val initResult_sessNow = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "sessNow", initResult_sessNow)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='sessNow'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'sessLater' with expr
        try {
            val initResult_sessLater = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "sessLater", initResult_sessLater)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='sessLater'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'commErrors' with expr
        try {
            val initResult_commErrors = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "commErrors", initResult_commErrors)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='commErrors'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'execErrors' with expr
        try {
            val initResult_execErrors = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "execErrors", initResult_execErrors)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='execErrors'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterStranger' with expr
        try {
            val initResult_afterStranger = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterStranger", initResult_afterStranger)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='afterStranger'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterStrangerLater' with expr
        try {
            val initResult_afterStrangerLater = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterStrangerLater", initResult_afterStrangerLater)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='afterStrangerLater'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterOrphan' with expr
        try {
            val initResult_afterOrphan = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterOrphan", initResult_afterOrphan)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='afterOrphan'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterOrphanLater' with expr
        try {
            val initResult_afterOrphanLater = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterOrphanLater", initResult_afterOrphanLater)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='afterOrphanLater'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterBogus' with expr
        try {
            val initResult_afterBogus = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterBogus", initResult_afterBogus)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='afterBogus'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'afterBogusLater' with expr
        try {
            val initResult_afterBogusLater = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "afterBogusLater", initResult_afterBogusLater)
        } catch (e: Exception) {
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<data id='afterBogusLater'> expr failed to evaluate")
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
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, reason)
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
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ATargetExpressionIsRoutedAsItsLiteralIsEvent) {
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
    override fun bindCurrentEvent(event: ATargetExpressionIsRoutedAsItsLiteralIsEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ATargetExpressionIsRoutedAsItsLiteralIsState,
        event: ATargetExpressionIsRoutedAsItsLiteralIsEvent?
    ): EnabledTransition<ATargetExpressionIsRoutedAsItsLiteralIsState, HistoryId>? = when (state) {
        is ATargetExpressionIsRoutedAsItsLiteralIsState.Run -> when {
            event is ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication -> transitionRunAt0
            event is ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution -> transitionRunAt1
            event is ATargetExpressionIsRoutedAsItsLiteralIsEvent.Hello -> transitionRunAt2
            event is ATargetExpressionIsRoutedAsItsLiteralIsEvent.InNow && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.type == \"internal\")", "_event.type === 'internal'")) -> transitionRunAt3
            event is ATargetExpressionIsRoutedAsItsLiteralIsEvent.InLater && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.type == \"internal\")", "_event.type === 'internal'")) -> transitionRunAt4
            event is ATargetExpressionIsRoutedAsItsLiteralIsEvent.KAckNow -> transitionRunAt5
            event is ATargetExpressionIsRoutedAsItsLiteralIsEvent.KAckLater -> transitionRunAt6
            event is ATargetExpressionIsRoutedAsItsLiteralIsEvent.SAckNow -> transitionRunAt7
            event is ATargetExpressionIsRoutedAsItsLiteralIsEvent.SAckLater -> transitionRunAt8
            event is ATargetExpressionIsRoutedAsItsLiteralIsEvent.Settle -> transitionRunAt9
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:47 :: _machine
    override fun onEntry(state: ATargetExpressionIsRoutedAsItsLiteralIsState, isDefaultEntry: Boolean) {
        when (state) {
            is ATargetExpressionIsRoutedAsItsLiteralIsState.Done -> {
                // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:164 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is ATargetExpressionIsRoutedAsItsLiteralIsState.Run -> {
                // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:73 :: run :: _state_body
                // W3C SCXML 3.8: Onentry block 1/6
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("tStranger", "tStranger")))
            } catch (_: Exception) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_7")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_7")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_7")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "lost",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_7",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_7")
            }
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ATargetExpressionIsRoutedAsItsLiteralIsEvent.Lost,
"lost",
                sendData,
0L,
                "__send_7",
                ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_7")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_7")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterStranger", "afterStranger"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 2/6
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("tStranger", "tStranger")))
            } catch (_: Exception) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_8")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_8")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_8")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "lost",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_8",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            // W3C SCXML 6.2.4: a `delay` is a property of the SEND, not of the
            // processor it named. The engine performs the act from its
            // scheduler drain at the deadline, including the W3C SCXML 6.2
            // report for an act nobody performed. W3C SCXML 6.3: it lands in
            // the delayed-send queue under the send id, so a `<cancel>`
            // reaches it and the host never sees the act.
            scheduleHostSend("__send_8", 10L, hostRequest)
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ATargetExpressionIsRoutedAsItsLiteralIsEvent.Lost,
"lost",
                sendData,
10L,
                "__send_8",
                ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_8")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_8")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterStrangerLater", "afterStrangerLater"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 3/6
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("tParent", "tParent")))
            } catch (_: Exception) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_9")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_9")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_9")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "lost",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_9",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_9")
            }
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ATargetExpressionIsRoutedAsItsLiteralIsEvent.Lost,
"lost",
                sendData,
0L,
                "__send_9",
                ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_9")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_9")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterOrphan", "afterOrphan"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 4/6
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("tParent", "tParent")))
            } catch (_: Exception) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_10")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_10")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_10")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "lost",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_10",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            // W3C SCXML 6.2.4: a `delay` is a property of the SEND, not of the
            // processor it named. The engine performs the act from its
            // scheduler drain at the deadline, including the W3C SCXML 6.2
            // report for an act nobody performed. W3C SCXML 6.3: it lands in
            // the delayed-send queue under the send id, so a `<cancel>`
            // reaches it and the host never sees the act.
            scheduleHostSend("__send_10", 10L, hostRequest)
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ATargetExpressionIsRoutedAsItsLiteralIsEvent.Lost,
"lost",
                sendData,
10L,
                "__send_10",
                ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_10")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_10")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterOrphanLater", "afterOrphanLater"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 5/6
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("tBogus", "tBogus")))
            } catch (_: Exception) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_11")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_11")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_11")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "lost",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_11",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_11")
            }
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ATargetExpressionIsRoutedAsItsLiteralIsEvent.Lost,
"lost",
                sendData,
0L,
                "__send_11",
                ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_11")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_11")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterBogus", "afterBogus"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 6/6
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("tBogus", "tBogus")))
            } catch (_: Exception) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_12")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_12")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_12")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "lost",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_12",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            // W3C SCXML 6.2.4: a `delay` is a property of the SEND, not of the
            // processor it named. The engine performs the act from its
            // scheduler drain at the deadline, including the W3C SCXML 6.2
            // report for an act nobody performed. W3C SCXML 6.3: it lands in
            // the delayed-send queue under the send id, so a `<cancel>`
            // reaches it and the host never sees the act.
            scheduleHostSend("__send_12", 10L, hostRequest)
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ATargetExpressionIsRoutedAsItsLiteralIsEvent.Lost,
"lost",
                sendData,
10L,
                "__send_12",
                ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_12")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_12")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("afterBogusLater", "afterBogusLater"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return@run
            }
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "run.${System.identityHashCode(this)}.kid"
                    deferInvoke(state, generatedInvokeId) {

                        val childSM = ATargetExpressionIsRoutedAsItsLiteralIsSceSynthInvokeKidStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("kid", childSM, false, ATargetExpressionIsRoutedAsItsLiteralIsEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:47 :: _machine
    override fun onExit(state: ATargetExpressionIsRoutedAsItsLiteralIsState) {
        when (state) {
            is ATargetExpressionIsRoutedAsItsLiteralIsState.Done -> {
                // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:164 :: done :: _state_body
            }
            is ATargetExpressionIsRoutedAsItsLiteralIsState.Run -> {
                // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:73 :: run :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("kid")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:47 :: _machine
    override fun executeTransitionContent(source: ATargetExpressionIsRoutedAsItsLiteralIsState, transitionIndex: Int) {
        when (source) {
        is ATargetExpressionIsRoutedAsItsLiteralIsState.Run -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:127 :: run :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("commErrors", "commErrors"), com.sce.runtime.ScriptSource.lua("_scxml_add(commErrors, 1)", "commErrors + 1"))) {
                return
            }
            }
            1 -> {
                // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:130 :: run :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("execErrors", "execErrors"), com.sce.runtime.ScriptSource.lua("_scxml_add(execErrors, 1)", "execErrors + 1"))) {
                return
            }
            }
            2 -> {
                // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:133 :: run :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("kidLoc", "kidLoc"), com.sce.runtime.ScriptSource.lua("_event.origin", "_event.origin"))) {
                return
            }


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("tInternal", "tInternal")))
            } catch (_: Exception) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_0")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_0")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_0")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "inNow",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_0",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_0")
            }
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ATargetExpressionIsRoutedAsItsLiteralIsEvent.InNow,
"inNow",
                sendData,
0L,
                "__send_0",
                ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_0")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_0")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("tInternal", "tInternal")))
            } catch (_: Exception) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_1")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_1")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_1")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "inLater",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_1",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            // W3C SCXML 6.2.4: a `delay` is a property of the SEND, not of the
            // processor it named. The engine performs the act from its
            // scheduler drain at the deadline, including the W3C SCXML 6.2
            // report for an act nobody performed. W3C SCXML 6.3: it lands in
            // the delayed-send queue under the send id, so a `<cancel>`
            // reaches it and the host never sees the act.
            scheduleHostSend("__send_1", 10L, hostRequest)
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ATargetExpressionIsRoutedAsItsLiteralIsEvent.InLater,
"inLater",
                sendData,
10L,
                "__send_1",
                ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_1")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_1")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("tKid", "tKid")))
            } catch (_: Exception) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_2")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_2")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_2")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "kNow",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_2",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_2")
            }
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ATargetExpressionIsRoutedAsItsLiteralIsEvent.KNow,
"kNow",
                sendData,
0L,
                "__send_2",
                ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_2")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_2")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("tKid", "tKid")))
            } catch (_: Exception) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_3")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_3")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_3")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "kLater",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_3",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            // W3C SCXML 6.2.4: a `delay` is a property of the SEND, not of the
            // processor it named. The engine performs the act from its
            // scheduler drain at the deadline, including the W3C SCXML 6.2
            // report for an act nobody performed. W3C SCXML 6.3: it lands in
            // the delayed-send queue under the send id, so a `<cancel>`
            // reaches it and the host never sees the act.
            scheduleHostSend("__send_3", 10L, hostRequest)
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ATargetExpressionIsRoutedAsItsLiteralIsEvent.KLater,
"kLater",
                sendData,
10L,
                "__send_3",
                ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_3")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_3")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("kidLoc", "kidLoc")))
            } catch (_: Exception) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_4")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_4")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_4")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sNow",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_4",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_4")
            }
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ATargetExpressionIsRoutedAsItsLiteralIsEvent.SNow,
"sNow",
                sendData,
0L,
                "__send_4",
                ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_4")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_4")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("kidLoc", "kidLoc")))
            } catch (_: Exception) {
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_5")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_5")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_5")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sLater",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_5",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            // W3C SCXML 6.2.4: a `delay` is a property of the SEND, not of the
            // processor it named. The engine performs the act from its
            // scheduler drain at the deadline, including the W3C SCXML 6.2
            // report for an act nobody performed. W3C SCXML 6.3: it lands in
            // the delayed-send queue under the send id, so a `<cancel>`
            // reaches it and the host never sees the act.
            scheduleHostSend("__send_5", 10L, hostRequest)
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
ATargetExpressionIsRoutedAsItsLiteralIsEvent.SLater,
"sLater",
                sendData,
10L,
                "__send_5",
                ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_5")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(ATargetExpressionIsRoutedAsItsLiteralIsEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_5")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_6", 300L, ATargetExpressionIsRoutedAsItsLiteralIsEvent.Settle, EventMetadata.external(sendId = "__send_6", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            3 -> {
                // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:143 :: run :: _transition_3


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("internalNow", "internalNow"), com.sce.runtime.ScriptSource.lua("_scxml_add(internalNow, 1)", "internalNow + 1"))) {
                return
            }
            }
            4 -> {
                // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:146 :: run :: _transition_4


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("internalLater", "internalLater"), com.sce.runtime.ScriptSource.lua("_scxml_add(internalLater, 1)", "internalLater + 1"))) {
                return
            }
            }
            5 -> {
                // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:149 :: run :: _transition_5


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("kidNow", "kidNow"), com.sce.runtime.ScriptSource.lua("_scxml_add(kidNow, 1)", "kidNow + 1"))) {
                return
            }
            }
            6 -> {
                // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:152 :: run :: _transition_6


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("kidLater", "kidLater"), com.sce.runtime.ScriptSource.lua("_scxml_add(kidLater, 1)", "kidLater + 1"))) {
                return
            }
            }
            7 -> {
                // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:155 :: run :: _transition_7


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("sessNow", "sessNow"), com.sce.runtime.ScriptSource.lua("_scxml_add(sessNow, 1)", "sessNow + 1"))) {
                return
            }
            }
            8 -> {
                // SCE-MAP: a_target_expression_is_routed_as_its_literal_is.scxml:158 :: run :: _transition_8


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("sessLater", "sessLater"), com.sce.runtime.ScriptSource.lua("_scxml_add(sessLater, 1)", "sessLater + 1"))) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
