// SCE-GENERATED — DO NOT EDIT
// source-hash: 183308bb9ac2854514dbfa50ef82390d0b2816c664d2a62988f48136f185c708

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
    }

    private object TransitionsPart0 {

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
            "external_chain_is_bounded",
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
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
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
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
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
    override fun bindCurrentEvent(event: ExternalChainIsBoundedEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: ExternalChainIsBoundedState, event: ExternalChainIsBoundedEvent?): EnabledTransition<ExternalChainIsBoundedState, HistoryId>? = when (state) {
        is ExternalChainIsBoundedState.Bounded -> firstEnabledTransitionOfBounded(state, event)
        is ExternalChainIsBoundedState.Idle -> firstEnabledTransitionOfIdle(state, event)
        is ExternalChainIsBoundedState.Resuming -> firstEnabledTransitionOfResuming(state, event)
        is ExternalChainIsBoundedState.Spin -> firstEnabledTransitionOfSpin(state, event)
        is ExternalChainIsBoundedState.Timing -> firstEnabledTransitionOfTiming(state, event)
        is ExternalChainIsBoundedState.Zeroing -> firstEnabledTransitionOfZeroing(state, event)
        is ExternalChainIsBoundedState.ZeroingExpr -> firstEnabledTransitionOfZeroingExpr(state, event)
    }


    // bounded's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfBounded(state: ExternalChainIsBoundedState, event: ExternalChainIsBoundedEvent?): EnabledTransition<ExternalChainIsBoundedState, HistoryId>? = when {
        event is ExternalChainIsBoundedEvent.Lap && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(laps < 4)", "laps < 4")) -> TransitionsPart0.transitionBoundedAt0
        event is ExternalChainIsBoundedEvent.Lap -> TransitionsPart0.transitionBoundedAt1
        event is ExternalChainIsBoundedEvent.Poke -> TransitionsPart0.transitionBoundedAt2
        else -> null
    }


    // idle's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdle(state: ExternalChainIsBoundedState, event: ExternalChainIsBoundedEvent?): EnabledTransition<ExternalChainIsBoundedState, HistoryId>? = when {
        event is ExternalChainIsBoundedEvent.Poke -> TransitionsPart0.transitionIdleAt0
        event is ExternalChainIsBoundedEvent.Spin -> TransitionsPart0.transitionIdleAt1
        event is ExternalChainIsBoundedEvent.Bounded -> TransitionsPart0.transitionIdleAt2
        event is ExternalChainIsBoundedEvent.Resume -> TransitionsPart0.transitionIdleAt3
        event is ExternalChainIsBoundedEvent.Zero -> TransitionsPart0.transitionIdleAt4
        event is ExternalChainIsBoundedEvent.ZeroExpr -> TransitionsPart0.transitionIdleAt5
        event is ExternalChainIsBoundedEvent.Timed -> TransitionsPart0.transitionIdleAt6
        else -> null
    }


    // resuming's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfResuming(state: ExternalChainIsBoundedState, event: ExternalChainIsBoundedEvent?): EnabledTransition<ExternalChainIsBoundedState, HistoryId>? = when {
        event is ExternalChainIsBoundedEvent.Beat && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(beats < 29)", "beats < 29")) -> TransitionsPart0.transitionResumingAt0
        event is ExternalChainIsBoundedEvent.Beat -> TransitionsPart0.transitionResumingAt1
        event is ExternalChainIsBoundedEvent.Poke -> TransitionsPart0.transitionResumingAt2
        else -> null
    }


    // spin's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSpin(state: ExternalChainIsBoundedState, event: ExternalChainIsBoundedEvent?): EnabledTransition<ExternalChainIsBoundedState, HistoryId>? = when {
        event is ExternalChainIsBoundedEvent.Link -> TransitionsPart0.transitionSpinAt0
        event is ExternalChainIsBoundedEvent.Poke -> TransitionsPart0.transitionSpinAt1
        else -> null
    }


    // timing's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfTiming(state: ExternalChainIsBoundedState, event: ExternalChainIsBoundedEvent?): EnabledTransition<ExternalChainIsBoundedState, HistoryId>? = when {
        event is ExternalChainIsBoundedEvent.Pulse -> TransitionsPart0.transitionTimingAt0
        event is ExternalChainIsBoundedEvent.Poke -> TransitionsPart0.transitionTimingAt1
        else -> null
    }


    // zeroing's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfZeroing(state: ExternalChainIsBoundedState, event: ExternalChainIsBoundedEvent?): EnabledTransition<ExternalChainIsBoundedState, HistoryId>? = when {
        event is ExternalChainIsBoundedEvent.Blink -> TransitionsPart0.transitionZeroingAt0
        event is ExternalChainIsBoundedEvent.Poke -> TransitionsPart0.transitionZeroingAt1
        else -> null
    }


    // zeroing_expr's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfZeroingExpr(state: ExternalChainIsBoundedState, event: ExternalChainIsBoundedEvent?): EnabledTransition<ExternalChainIsBoundedState, HistoryId>? = when {
        event is ExternalChainIsBoundedEvent.Blink -> TransitionsPart0.transitionZeroingExprAt0
        event is ExternalChainIsBoundedEvent.Poke -> TransitionsPart0.transitionZeroingExprAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: external_chain_is_bounded.scxml:83 :: _machine
    override fun onEntry(state: ExternalChainIsBoundedState, isDefaultEntry: Boolean) {
        when (state) {
            is ExternalChainIsBoundedState.Bounded -> onEntryOfBounded()
            is ExternalChainIsBoundedState.Idle -> onEntryOfIdle()
            is ExternalChainIsBoundedState.Resuming -> onEntryOfResuming()
            is ExternalChainIsBoundedState.Spin -> onEntryOfSpin()
            is ExternalChainIsBoundedState.Timing -> onEntryOfTiming()
            is ExternalChainIsBoundedState.Zeroing -> onEntryOfZeroing()
            is ExternalChainIsBoundedState.ZeroingExpr -> onEntryOfZeroingExpr()
        }
    }


    private fun onEntryOfBounded() {
                // SCE-MAP: external_chain_is_bounded.scxml:141 :: bounded :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ExternalChainIsBoundedEvent.Lap, EventMetadata.sentBy(sessionId, sendId = "__send_3", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfIdle() {
                // SCE-MAP: external_chain_is_bounded.scxml:109 :: idle :: _state_body
    }

    private fun onEntryOfResuming() {
                // SCE-MAP: external_chain_is_bounded.scxml:161 :: resuming :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ExternalChainIsBoundedEvent.Beat, EventMetadata.sentBy(sessionId, sendId = "__send_5", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfSpin() {
                // SCE-MAP: external_chain_is_bounded.scxml:126 :: spin :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ExternalChainIsBoundedEvent.Link, EventMetadata.sentBy(sessionId, sendId = "__send_1", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfTiming() {
                // SCE-MAP: external_chain_is_bounded.scxml:214 :: timing :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_10", 1L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.sentBy(sessionId, sendId = "__send_10", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_11", 2L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.sentBy(sessionId, sendId = "__send_11", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_12", 3L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.sentBy(sessionId, sendId = "__send_12", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_13", 4L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.sentBy(sessionId, sendId = "__send_13", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_14", 5L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.sentBy(sessionId, sendId = "__send_14", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_15", 6L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.sentBy(sessionId, sendId = "__send_15", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_16", 7L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.sentBy(sessionId, sendId = "__send_16", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_17", 8L, ExternalChainIsBoundedEvent.Pulse, EventMetadata.sentBy(sessionId, sendId = "__send_17", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfZeroing() {
                // SCE-MAP: external_chain_is_bounded.scxml:181 :: zeroing :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_7", 0L, ExternalChainIsBoundedEvent.Blink, EventMetadata.sentBy(sessionId, sendId = "__send_7", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfZeroingExpr() {
                // SCE-MAP: external_chain_is_bounded.scxml:197 :: zeroing_expr :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            assertScriptSession()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
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
            scheduleSend("__send_9", sendDelayMs, ExternalChainIsBoundedEvent.Blink, EventMetadata.sentBy(sessionId, sendId = "__send_9", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: external_chain_is_bounded.scxml:83 :: _machine
    override fun onExit(state: ExternalChainIsBoundedState) {
        when (state) {
            is ExternalChainIsBoundedState.Bounded -> onExitOfBounded()
            is ExternalChainIsBoundedState.Idle -> onExitOfIdle()
            is ExternalChainIsBoundedState.Resuming -> onExitOfResuming()
            is ExternalChainIsBoundedState.Spin -> onExitOfSpin()
            is ExternalChainIsBoundedState.Timing -> onExitOfTiming()
            is ExternalChainIsBoundedState.Zeroing -> onExitOfZeroing()
            is ExternalChainIsBoundedState.ZeroingExpr -> onExitOfZeroingExpr()
        }
    }


    private fun onExitOfBounded() {
                // SCE-MAP: external_chain_is_bounded.scxml:141 :: bounded :: _state_body
    }

    private fun onExitOfIdle() {
                // SCE-MAP: external_chain_is_bounded.scxml:109 :: idle :: _state_body
    }

    private fun onExitOfResuming() {
                // SCE-MAP: external_chain_is_bounded.scxml:161 :: resuming :: _state_body
    }

    private fun onExitOfSpin() {
                // SCE-MAP: external_chain_is_bounded.scxml:126 :: spin :: _state_body
    }

    private fun onExitOfTiming() {
                // SCE-MAP: external_chain_is_bounded.scxml:214 :: timing :: _state_body
    }

    private fun onExitOfZeroing() {
                // SCE-MAP: external_chain_is_bounded.scxml:181 :: zeroing :: _state_body
    }

    private fun onExitOfZeroingExpr() {
                // SCE-MAP: external_chain_is_bounded.scxml:197 :: zeroing_expr :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: external_chain_is_bounded.scxml:83 :: _machine
    override fun executeTransitionContent(source: ExternalChainIsBoundedState, transitionIndex: Int) {
        when (source) {
            is ExternalChainIsBoundedState.Bounded -> transitionContentOfBounded(transitionIndex)
            is ExternalChainIsBoundedState.Idle -> transitionContentOfIdle(transitionIndex)
            is ExternalChainIsBoundedState.Resuming -> transitionContentOfResuming(transitionIndex)
            is ExternalChainIsBoundedState.Spin -> transitionContentOfSpin(transitionIndex)
            is ExternalChainIsBoundedState.Timing -> transitionContentOfTiming(transitionIndex)
            is ExternalChainIsBoundedState.Zeroing -> transitionContentOfZeroing(transitionIndex)
            is ExternalChainIsBoundedState.ZeroingExpr -> transitionContentOfZeroingExpr(transitionIndex)
        }
    }


    // The content of `bounded`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfBounded(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfBoundedAt0()
            1 -> transitionContentOfBoundedAt1()
            2 -> transitionContentOfBoundedAt2()
            else -> {}
        }
    }


    private fun transitionContentOfBoundedAt0() {
                // SCE-MAP: external_chain_is_bounded.scxml:143 :: bounded :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("laps", "laps"), com.sce.runtime.ScriptSource.lua("_scxml_add(laps, 1)", "laps + 1"))) {
                return
            }


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ExternalChainIsBoundedEvent.Lap, EventMetadata.sentBy(sessionId, sendId = "__send_2", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfBoundedAt1() {
                // SCE-MAP: external_chain_is_bounded.scxml:147 :: bounded :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("laps", "laps"), com.sce.runtime.ScriptSource.lua("_scxml_add(laps, 1)", "laps + 1"))) {
                return
            }
    }

    private fun transitionContentOfBoundedAt2() {
                // SCE-MAP: external_chain_is_bounded.scxml:150 :: bounded :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
    }

    // The content of `idle`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfIdle(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfIdleAt0()
            else -> {}
        }
    }


    private fun transitionContentOfIdleAt0() {
                // SCE-MAP: external_chain_is_bounded.scxml:110 :: idle :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
    }

    // The content of `resuming`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfResuming(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfResumingAt0()
            1 -> transitionContentOfResumingAt1()
            2 -> transitionContentOfResumingAt2()
            else -> {}
        }
    }


    private fun transitionContentOfResumingAt0() {
                // SCE-MAP: external_chain_is_bounded.scxml:163 :: resuming :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("beats", "beats"), com.sce.runtime.ScriptSource.lua("_scxml_add(beats, 1)", "beats + 1"))) {
                return
            }


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ExternalChainIsBoundedEvent.Beat, EventMetadata.sentBy(sessionId, sendId = "__send_4", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfResumingAt1() {
                // SCE-MAP: external_chain_is_bounded.scxml:167 :: resuming :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("beats", "beats"), com.sce.runtime.ScriptSource.lua("_scxml_add(beats, 1)", "beats + 1"))) {
                return
            }
    }

    private fun transitionContentOfResumingAt2() {
                // SCE-MAP: external_chain_is_bounded.scxml:170 :: resuming :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
    }

    // The content of `spin`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfSpin(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfSpinAt0()
            1 -> transitionContentOfSpinAt1()
            else -> {}
        }
    }


    private fun transitionContentOfSpinAt0() {
                // SCE-MAP: external_chain_is_bounded.scxml:128 :: spin :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("links", "links"), com.sce.runtime.ScriptSource.lua("_scxml_add(links, 1)", "links + 1"))) {
                return
            }


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(ExternalChainIsBoundedEvent.Link, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfSpinAt1() {
                // SCE-MAP: external_chain_is_bounded.scxml:132 :: spin :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
    }

    // The content of `timing`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfTiming(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfTimingAt0()
            1 -> transitionContentOfTimingAt1()
            else -> {}
        }
    }


    private fun transitionContentOfTimingAt0() {
                // SCE-MAP: external_chain_is_bounded.scxml:225 :: timing :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pulses", "pulses"), com.sce.runtime.ScriptSource.lua("_scxml_add(pulses, 1)", "pulses + 1"))) {
                return
            }
    }

    private fun transitionContentOfTimingAt1() {
                // SCE-MAP: external_chain_is_bounded.scxml:228 :: timing :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
    }

    // The content of `zeroing`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfZeroing(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfZeroingAt0()
            1 -> transitionContentOfZeroingAt1()
            else -> {}
        }
    }


    private fun transitionContentOfZeroingAt0() {
                // SCE-MAP: external_chain_is_bounded.scxml:183 :: zeroing :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("blinks", "blinks"), com.sce.runtime.ScriptSource.lua("_scxml_add(blinks, 1)", "blinks + 1"))) {
                return
            }


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_6", 0L, ExternalChainIsBoundedEvent.Blink, EventMetadata.sentBy(sessionId, sendId = "__send_6", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfZeroingAt1() {
                // SCE-MAP: external_chain_is_bounded.scxml:187 :: zeroing :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
    }

    // The content of `zeroing_expr`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfZeroingExpr(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfZeroingExprAt0()
            1 -> transitionContentOfZeroingExprAt1()
            else -> {}
        }
    }


    private fun transitionContentOfZeroingExprAt0() {
                // SCE-MAP: external_chain_is_bounded.scxml:199 :: zeroing_expr :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("exprs", "exprs"), com.sce.runtime.ScriptSource.lua("_scxml_add(exprs, 1)", "exprs + 1"))) {
                return
            }


            if (run send@{
            assertScriptSession()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
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
            scheduleSend("__send_8", sendDelayMs, ExternalChainIsBoundedEvent.Blink, EventMetadata.sentBy(sessionId, sendId = "__send_8", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfZeroingExprAt1() {
                // SCE-MAP: external_chain_is_bounded.scxml:203 :: zeroing_expr :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
    }
}
