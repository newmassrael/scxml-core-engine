// SCE-GENERATED — DO NOT EDIT
// source-hash: a3ca38872087c2314f9630b7be1b12a428c8ffa1560d49369c9f599f3caa573c

// GENERATED CODE — DO NOT EDIT
// Source: tests/integration/external_chain_is_bounded.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: external_chain_is_bounded.scxml:83 :: _machine

package com.sce.integration.external_chain_is_bounded

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ExternalChainIsBoundedState : State {
    data object Bounded : ExternalChainIsBoundedState
    data object Idle : ExternalChainIsBoundedState
    data object Resuming : ExternalChainIsBoundedState
    data object Spin : ExternalChainIsBoundedState
    data object Timing : ExternalChainIsBoundedState
    data object Zeroing : ExternalChainIsBoundedState
    data object ZeroingExpr : ExternalChainIsBoundedState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ExternalChainIsBoundedEvent : Event {
    data object Beat : ExternalChainIsBoundedEvent
    data object Blink : ExternalChainIsBoundedEvent
    data object Bounded : ExternalChainIsBoundedEvent
    sealed interface Error : ExternalChainIsBoundedEvent {
        data object Execution : Error
    }
    data object Lap : ExternalChainIsBoundedEvent
    data object Link : ExternalChainIsBoundedEvent
    data object Poke : ExternalChainIsBoundedEvent
    data object Pulse : ExternalChainIsBoundedEvent
    data object Resume : ExternalChainIsBoundedEvent
    data object Spin : ExternalChainIsBoundedEvent
    data object Timed : ExternalChainIsBoundedEvent
    data object Zero : ExternalChainIsBoundedEvent
    data object ZeroExpr : ExternalChainIsBoundedEvent
}
// --- State Machine (W3C SCXML) ---

class ExternalChainIsBoundedStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ExternalChainIsBoundedState, ExternalChainIsBoundedEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `pokes` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `pokes` was assigned a value of another type, or the engine refused.
     */
    fun pokes(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "pokes")

    /**
     * §scxml-5.3: what the `links` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `links` was assigned a value of another type, or the engine refused.
     */
    fun links(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "links")

    /**
     * §scxml-5.3: what the `laps` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `laps` was assigned a value of another type, or the engine refused.
     */
    fun laps(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "laps")

    /**
     * §scxml-5.3: what the `beats` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `beats` was assigned a value of another type, or the engine refused.
     */
    fun beats(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "beats")

    /**
     * §scxml-5.3: what the `blinks` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `blinks` was assigned a value of another type, or the engine refused.
     */
    fun blinks(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "blinks")

    /**
     * §scxml-5.3: what the `exprs` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `exprs` was assigned a value of another type, or the engine refused.
     */
    fun exprs(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "exprs")

    /**
     * §scxml-5.3: what the `pulses` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `pulses` was assigned a value of another type, or the engine refused.
     */
    fun pulses(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "pulses")

    override val initialState: ExternalChainIsBoundedState = ExternalChainIsBoundedState.Idle

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

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ExternalChainIsBoundedState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<ExternalChainIsBoundedState, HistoryId>> =
            listOf(StateTarget(ExternalChainIsBoundedState.Idle))

        // W3C SCXML 3.13: bounded's transition 0, as the microstep reads it.
        val transitionBoundedAt0 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Bounded,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: bounded's transition 1, as the microstep reads it.
        val transitionBoundedAt1 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Bounded,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: bounded's transition 2, as the microstep reads it.
        val transitionBoundedAt2 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Bounded,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Idle,
            listOf(StateTarget(ExternalChainIsBoundedState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Idle,
            listOf(StateTarget(ExternalChainIsBoundedState.Spin)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Idle,
            listOf(StateTarget(ExternalChainIsBoundedState.Bounded)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Idle,
            listOf(StateTarget(ExternalChainIsBoundedState.Resuming)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Idle,
            listOf(StateTarget(ExternalChainIsBoundedState.Zeroing)),
            4,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Idle,
            listOf(StateTarget(ExternalChainIsBoundedState.ZeroingExpr)),
            5,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 6, as the microstep reads it.
        val transitionIdleAt6 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Idle,
            listOf(StateTarget(ExternalChainIsBoundedState.Timing)),
            6,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: resuming's transition 0, as the microstep reads it.
        val transitionResumingAt0 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Resuming,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: resuming's transition 1, as the microstep reads it.
        val transitionResumingAt1 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Resuming,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: resuming's transition 2, as the microstep reads it.
        val transitionResumingAt2 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Resuming,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: spin's transition 0, as the microstep reads it.
        val transitionSpinAt0 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Spin,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: spin's transition 1, as the microstep reads it.
        val transitionSpinAt1 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Spin,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: timing's transition 0, as the microstep reads it.
        val transitionTimingAt0 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Timing,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: timing's transition 1, as the microstep reads it.
        val transitionTimingAt1 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Timing,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: zeroing's transition 0, as the microstep reads it.
        val transitionZeroingAt0 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Zeroing,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: zeroing's transition 1, as the microstep reads it.
        val transitionZeroingAt1 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.Zeroing,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: zeroing_expr's transition 0, as the microstep reads it.
        val transitionZeroingExprAt0 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.ZeroingExpr,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: zeroing_expr's transition 1, as the microstep reads it.
        val transitionZeroingExprAt1 = EnabledTransition<ExternalChainIsBoundedState, HistoryId>(
            ExternalChainIsBoundedState.ZeroingExpr,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ExternalChainIsBoundedState? = when (stateId) {
        "bounded" -> ExternalChainIsBoundedState.Bounded
        "idle" -> ExternalChainIsBoundedState.Idle
        "resuming" -> ExternalChainIsBoundedState.Resuming
        "spin" -> ExternalChainIsBoundedState.Spin
        "timing" -> ExternalChainIsBoundedState.Timing
        "zeroing" -> ExternalChainIsBoundedState.Zeroing
        "zeroing_expr" -> ExternalChainIsBoundedState.ZeroingExpr
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ExternalChainIsBoundedState): String = when (state) {
        is ExternalChainIsBoundedState.Bounded -> "bounded"
        is ExternalChainIsBoundedState.Idle -> "idle"
        is ExternalChainIsBoundedState.Resuming -> "resuming"
        is ExternalChainIsBoundedState.Spin -> "spin"
        is ExternalChainIsBoundedState.Timing -> "timing"
        is ExternalChainIsBoundedState.Zeroing -> "zeroing"
        is ExternalChainIsBoundedState.ZeroingExpr -> "zeroing_expr"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ExternalChainIsBoundedState): Int = when (state) {
        is ExternalChainIsBoundedState.Bounded -> 2
        is ExternalChainIsBoundedState.Idle -> 0
        is ExternalChainIsBoundedState.Resuming -> 3
        is ExternalChainIsBoundedState.Spin -> 1
        is ExternalChainIsBoundedState.Timing -> 6
        is ExternalChainIsBoundedState.Zeroing -> 4
        is ExternalChainIsBoundedState.ZeroingExpr -> 5
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ExternalChainIsBoundedEvent? = when (name) {
        "beat" -> ExternalChainIsBoundedEvent.Beat
        "blink" -> ExternalChainIsBoundedEvent.Blink
        "bounded" -> ExternalChainIsBoundedEvent.Bounded
        "error.execution" -> ExternalChainIsBoundedEvent.Error.Execution
        "lap" -> ExternalChainIsBoundedEvent.Lap
        "link" -> ExternalChainIsBoundedEvent.Link
        "poke" -> ExternalChainIsBoundedEvent.Poke
        "pulse" -> ExternalChainIsBoundedEvent.Pulse
        "resume" -> ExternalChainIsBoundedEvent.Resume
        "spin" -> ExternalChainIsBoundedEvent.Spin
        "timed" -> ExternalChainIsBoundedEvent.Timed
        "zero" -> ExternalChainIsBoundedEvent.Zero
        "zero_expr" -> ExternalChainIsBoundedEvent.ZeroExpr
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ExternalChainIsBoundedEvent): String? = when (event) {
        is ExternalChainIsBoundedEvent.Beat -> "beat"
        is ExternalChainIsBoundedEvent.Blink -> "blink"
        is ExternalChainIsBoundedEvent.Bounded -> "bounded"
        is ExternalChainIsBoundedEvent.Error.Execution -> "error.execution"
        is ExternalChainIsBoundedEvent.Lap -> "lap"
        is ExternalChainIsBoundedEvent.Link -> "link"
        is ExternalChainIsBoundedEvent.Poke -> "poke"
        is ExternalChainIsBoundedEvent.Pulse -> "pulse"
        is ExternalChainIsBoundedEvent.Resume -> "resume"
        is ExternalChainIsBoundedEvent.Spin -> "spin"
        is ExternalChainIsBoundedEvent.Timed -> "timed"
        is ExternalChainIsBoundedEvent.Zero -> "zero"
        is ExternalChainIsBoundedEvent.ZeroExpr -> "zero_expr"
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
            "external_chain_is_bounded",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'pokes' with expr
        try {
            val initResult_pokes = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "pokes", initResult_pokes)
        } catch (e: Exception) {
            raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "<data id='pokes'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'links' with expr
        try {
            val initResult_links = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "links", initResult_links)
        } catch (e: Exception) {
            raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "<data id='links'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'laps' with expr
        try {
            val initResult_laps = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "laps", initResult_laps)
        } catch (e: Exception) {
            raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "<data id='laps'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'beats' with expr
        try {
            val initResult_beats = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "beats", initResult_beats)
        } catch (e: Exception) {
            raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "<data id='beats'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'blinks' with expr
        try {
            val initResult_blinks = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "blinks", initResult_blinks)
        } catch (e: Exception) {
            raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "<data id='blinks'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'exprs' with expr
        try {
            val initResult_exprs = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "exprs", initResult_exprs)
        } catch (e: Exception) {
            raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "<data id='exprs'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'pulses' with expr
        try {
            val initResult_pulses = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "pulses", initResult_pulses)
        } catch (e: Exception) {
            raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "<data id='pulses'> expr failed to evaluate")
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
            raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, reason)
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
            raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ExternalChainIsBoundedEvent) {
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
    override fun bindCurrentEvent(event: ExternalChainIsBoundedEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ExternalChainIsBoundedState,
        event: ExternalChainIsBoundedEvent?
    ): EnabledTransition<ExternalChainIsBoundedState, HistoryId>? = when (state) {
        is ExternalChainIsBoundedState.Bounded -> when {
            event is ExternalChainIsBoundedEvent.Lap && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(laps < 4)", "laps < 4")) -> transitionBoundedAt0
            event is ExternalChainIsBoundedEvent.Lap -> transitionBoundedAt1
            event is ExternalChainIsBoundedEvent.Poke -> transitionBoundedAt2
            else -> null
        }
        is ExternalChainIsBoundedState.Idle -> when {
            event is ExternalChainIsBoundedEvent.Poke -> transitionIdleAt0
            event is ExternalChainIsBoundedEvent.Spin -> transitionIdleAt1
            event is ExternalChainIsBoundedEvent.Bounded -> transitionIdleAt2
            event is ExternalChainIsBoundedEvent.Resume -> transitionIdleAt3
            event is ExternalChainIsBoundedEvent.Zero -> transitionIdleAt4
            event is ExternalChainIsBoundedEvent.ZeroExpr -> transitionIdleAt5
            event is ExternalChainIsBoundedEvent.Timed -> transitionIdleAt6
            else -> null
        }
        is ExternalChainIsBoundedState.Resuming -> when {
            event is ExternalChainIsBoundedEvent.Beat && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(beats < 29)", "beats < 29")) -> transitionResumingAt0
            event is ExternalChainIsBoundedEvent.Beat -> transitionResumingAt1
            event is ExternalChainIsBoundedEvent.Poke -> transitionResumingAt2
            else -> null
        }
        is ExternalChainIsBoundedState.Spin -> when {
            event is ExternalChainIsBoundedEvent.Link -> transitionSpinAt0
            event is ExternalChainIsBoundedEvent.Poke -> transitionSpinAt1
            else -> null
        }
        is ExternalChainIsBoundedState.Timing -> when {
            event is ExternalChainIsBoundedEvent.Pulse -> transitionTimingAt0
            event is ExternalChainIsBoundedEvent.Poke -> transitionTimingAt1
            else -> null
        }
        is ExternalChainIsBoundedState.Zeroing -> when {
            event is ExternalChainIsBoundedEvent.Blink -> transitionZeroingAt0
            event is ExternalChainIsBoundedEvent.Poke -> transitionZeroingAt1
            else -> null
        }
        is ExternalChainIsBoundedState.ZeroingExpr -> when {
            event is ExternalChainIsBoundedEvent.Blink -> transitionZeroingExprAt0
            event is ExternalChainIsBoundedEvent.Poke -> transitionZeroingExprAt1
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: external_chain_is_bounded.scxml:83 :: _machine
    override fun onEntry(state: ExternalChainIsBoundedState, isDefaultEntry: Boolean) {
        when (state) {
            is ExternalChainIsBoundedState.Bounded -> {
                // SCE-MAP: external_chain_is_bounded.scxml:141 :: bounded :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ExternalChainIsBoundedEvent.Lap, EventMetadata.external(sendId = "__send_3", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is ExternalChainIsBoundedState.Idle -> {
                // SCE-MAP: external_chain_is_bounded.scxml:109 :: idle :: _state_body
            }
            is ExternalChainIsBoundedState.Resuming -> {
                // SCE-MAP: external_chain_is_bounded.scxml:161 :: resuming :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ExternalChainIsBoundedEvent.Beat, EventMetadata.external(sendId = "__send_5", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is ExternalChainIsBoundedState.Spin -> {
                // SCE-MAP: external_chain_is_bounded.scxml:126 :: spin :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ExternalChainIsBoundedEvent.Link, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is ExternalChainIsBoundedState.Timing -> {
                // SCE-MAP: external_chain_is_bounded.scxml:214 :: timing :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_10", 1L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.external(sendId = "__send_10", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_11", 2L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.external(sendId = "__send_11", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_12", 3L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.external(sendId = "__send_12", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_13", 4L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.external(sendId = "__send_13", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_14", 5L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.external(sendId = "__send_14", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_15", 6L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.external(sendId = "__send_15", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_16", 7L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.external(sendId = "__send_16", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_17", 8L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.external(sendId = "__send_17", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is ExternalChainIsBoundedState.Zeroing -> {
                // SCE-MAP: external_chain_is_bounded.scxml:181 :: zeroing :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_7", 0L, ExternalChainIsBoundedEvent.Blink, EventMetadata.external(sendId = "__send_7", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is ExternalChainIsBoundedState.ZeroingExpr -> {
                // SCE-MAP: external_chain_is_bounded.scxml:197 :: zeroing_expr :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // The expression failing is the argument error, and so is a value
            // that is not the CSS2 time the clause names (ARCHITECTURE.md,
            // "Durations"): the message is not scheduled under some default wait.
            val sendDelayText = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("\"0ms\"", "'0ms'")))
            } catch (_: Exception) {
                raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "<send> delayexpr could not be evaluated", "__send_9")
                return@send true
            }
            val sendDelayMs = com.sce.runtime.SendHelper.parseDelayMs(sendDelayText) ?: run {
                raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "<send> delayexpr is not a CSS2 time", "__send_9")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_9", sendDelayMs, ExternalChainIsBoundedEvent.Blink, EventMetadata.external(sendId = "__send_9", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: external_chain_is_bounded.scxml:83 :: _machine
    override fun onExit(state: ExternalChainIsBoundedState) {
        when (state) {
            is ExternalChainIsBoundedState.Bounded -> {
                // SCE-MAP: external_chain_is_bounded.scxml:141 :: bounded :: _state_body
            }
            is ExternalChainIsBoundedState.Idle -> {
                // SCE-MAP: external_chain_is_bounded.scxml:109 :: idle :: _state_body
            }
            is ExternalChainIsBoundedState.Resuming -> {
                // SCE-MAP: external_chain_is_bounded.scxml:161 :: resuming :: _state_body
            }
            is ExternalChainIsBoundedState.Spin -> {
                // SCE-MAP: external_chain_is_bounded.scxml:126 :: spin :: _state_body
            }
            is ExternalChainIsBoundedState.Timing -> {
                // SCE-MAP: external_chain_is_bounded.scxml:214 :: timing :: _state_body
            }
            is ExternalChainIsBoundedState.Zeroing -> {
                // SCE-MAP: external_chain_is_bounded.scxml:181 :: zeroing :: _state_body
            }
            is ExternalChainIsBoundedState.ZeroingExpr -> {
                // SCE-MAP: external_chain_is_bounded.scxml:197 :: zeroing_expr :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: external_chain_is_bounded.scxml:83 :: _machine
    override fun executeTransitionContent(source: ExternalChainIsBoundedState, transitionIndex: Int) {
        when (source) {
        is ExternalChainIsBoundedState.Bounded -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:143 :: bounded :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("laps", "laps"), com.sce.runtime.ScriptSource.lua("_scxml_add(laps, 1)", "laps + 1"))) {
                return
            }


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ExternalChainIsBoundedEvent.Lap, EventMetadata.external(sendId = "__send_2", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:147 :: bounded :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("laps", "laps"), com.sce.runtime.ScriptSource.lua("_scxml_add(laps, 1)", "laps + 1"))) {
                return
            }
            }
            2 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:150 :: bounded :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
            }
            else -> {}
        }
        is ExternalChainIsBoundedState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:110 :: idle :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
            }
            else -> {}
        }
        is ExternalChainIsBoundedState.Resuming -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:163 :: resuming :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("beats", "beats"), com.sce.runtime.ScriptSource.lua("_scxml_add(beats, 1)", "beats + 1"))) {
                return
            }


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ExternalChainIsBoundedEvent.Beat, EventMetadata.external(sendId = "__send_4", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:167 :: resuming :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("beats", "beats"), com.sce.runtime.ScriptSource.lua("_scxml_add(beats, 1)", "beats + 1"))) {
                return
            }
            }
            2 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:170 :: resuming :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
            }
            else -> {}
        }
        is ExternalChainIsBoundedState.Spin -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:128 :: spin :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("links", "links"), com.sce.runtime.ScriptSource.lua("_scxml_add(links, 1)", "links + 1"))) {
                return
            }


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ExternalChainIsBoundedEvent.Link, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:132 :: spin :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
            }
            else -> {}
        }
        is ExternalChainIsBoundedState.Timing -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:225 :: timing :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pulses", "pulses"), com.sce.runtime.ScriptSource.lua("_scxml_add(pulses, 1)", "pulses + 1"))) {
                return
            }
            }
            1 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:228 :: timing :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
            }
            else -> {}
        }
        is ExternalChainIsBoundedState.Zeroing -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:183 :: zeroing :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("blinks", "blinks"), com.sce.runtime.ScriptSource.lua("_scxml_add(blinks, 1)", "blinks + 1"))) {
                return
            }


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_6", 0L, ExternalChainIsBoundedEvent.Blink, EventMetadata.external(sendId = "__send_6", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:187 :: zeroing :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
            }
            else -> {}
        }
        is ExternalChainIsBoundedState.ZeroingExpr -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:199 :: zeroing_expr :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("exprs", "exprs"), com.sce.runtime.ScriptSource.lua("_scxml_add(exprs, 1)", "exprs + 1"))) {
                return
            }


            if (run send@{
            ensureScriptEngine()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            // The expression failing is the argument error, and so is a value
            // that is not the CSS2 time the clause names (ARCHITECTURE.md,
            // "Durations"): the message is not scheduled under some default wait.
            val sendDelayText = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("\"0ms\"", "'0ms'")))
            } catch (_: Exception) {
                raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "<send> delayexpr could not be evaluated", "__send_8")
                return@send true
            }
            val sendDelayMs = com.sce.runtime.SendHelper.parseDelayMs(sendDelayText) ?: run {
                raisePlatformError(ExternalChainIsBoundedEvent.Error.Execution, "<send> delayexpr is not a CSS2 time", "__send_8")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_8", sendDelayMs, ExternalChainIsBoundedEvent.Blink, EventMetadata.external(sendId = "__send_8", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: external_chain_is_bounded.scxml:203 :: zeroing_expr :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
