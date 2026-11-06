// SCE-GENERATED — DO NOT EDIT
// source-hash: 5eba0d45073fc837c42ae33f20795f0ee658705613f4c4e42a59b557f47ce142

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/internal_chain_is_bounded/internal_chain_is_bounded.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: internal_chain_is_bounded.scxml:90 :: _machine

package com.sce.integration.internal_chain_is_bounded

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InternalChainIsBoundedState : State {
    data object Alt : InternalChainIsBoundedState
    data object Bounded : InternalChainIsBoundedState
    data object Idle : InternalChainIsBoundedState
    data object Ignoring : InternalChainIsBoundedState
    data object Resuming : InternalChainIsBoundedState
    data object Spin : InternalChainIsBoundedState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InternalChainIsBoundedEvent : Event {
    data object Alternate : InternalChainIsBoundedEvent
    data object Beat : InternalChainIsBoundedEvent
    data object Beatless : InternalChainIsBoundedEvent
    data object Bounded : InternalChainIsBoundedEvent
    sealed interface Error : InternalChainIsBoundedEvent {
        data object Execution : Error
    }
    data object Link : InternalChainIsBoundedEvent
    data object Poke : InternalChainIsBoundedEvent
    data object Resume : InternalChainIsBoundedEvent
    data object Spin : InternalChainIsBoundedEvent
    data object Tick : InternalChainIsBoundedEvent
    data object Unanswered : InternalChainIsBoundedEvent
    data object Unheard : InternalChainIsBoundedEvent
}
// --- State Machine (W3C SCXML) ---

class InternalChainIsBoundedStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<InternalChainIsBoundedState, InternalChainIsBoundedEvent>(scriptEngine) {

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
     * §scxml-5.3: what the `alts` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `alts` was assigned a value of another type, or the engine refused.
     */
    fun alts(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "alts")

    /**
     * §scxml-5.3: what the `ignores` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `ignores` was assigned a value of another type, or the engine refused.
     */
    fun ignores(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "ignores")

    /**
     * §scxml-5.3: what the `pending` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `pending` was assigned a value of another type, or the engine refused.
     */
    fun pending(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "pending")

    override val initialState: InternalChainIsBoundedState = InternalChainIsBoundedState.Idle

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

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
    override val documentInitialTargets: List<EntryTarget<InternalChainIsBoundedState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<InternalChainIsBoundedState, HistoryId>> =
            listOf(StateTarget(InternalChainIsBoundedState.Idle))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: alt's transition 0, as the microstep reads it.
        val transitionAltAt0 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Alt,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: alt's transition 1, as the microstep reads it.
        val transitionAltAt1 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Alt,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: alt's transition 2, as the microstep reads it.
        val transitionAltAt2 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Alt,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: bounded's transition 0, as the microstep reads it.
        val transitionBoundedAt0 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Bounded,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: bounded's transition 1, as the microstep reads it.
        val transitionBoundedAt1 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Bounded,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: bounded's transition 2, as the microstep reads it.
        val transitionBoundedAt2 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Bounded,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Idle,
            listOf(StateTarget(InternalChainIsBoundedState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Idle,
            listOf(StateTarget(InternalChainIsBoundedState.Bounded)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Idle,
            listOf(StateTarget(InternalChainIsBoundedState.Spin)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Idle,
            listOf(StateTarget(InternalChainIsBoundedState.Resuming)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Idle,
            listOf(StateTarget(InternalChainIsBoundedState.Alt)),
            4,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Idle,
            listOf(StateTarget(InternalChainIsBoundedState.Ignoring)),
            5,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: ignoring's transition 0, as the microstep reads it.
        val transitionIgnoringAt0 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Ignoring,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: ignoring's transition 1, as the microstep reads it.
        val transitionIgnoringAt1 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Ignoring,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: ignoring's transition 2, as the microstep reads it.
        val transitionIgnoringAt2 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Ignoring,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: resuming's transition 0, as the microstep reads it.
        val transitionResumingAt0 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Resuming,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: resuming's transition 1, as the microstep reads it.
        val transitionResumingAt1 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Resuming,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: resuming's transition 2, as the microstep reads it.
        val transitionResumingAt2 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Resuming,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: spin's transition 0, as the microstep reads it.
        val transitionSpinAt0 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Spin,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: spin's transition 1, as the microstep reads it.
        val transitionSpinAt1 = EnabledTransition<InternalChainIsBoundedState, HistoryId>(
            InternalChainIsBoundedState.Spin,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): InternalChainIsBoundedState? = when (stateId) {
        "alt" -> InternalChainIsBoundedState.Alt
        "bounded" -> InternalChainIsBoundedState.Bounded
        "idle" -> InternalChainIsBoundedState.Idle
        "ignoring" -> InternalChainIsBoundedState.Ignoring
        "resuming" -> InternalChainIsBoundedState.Resuming
        "spin" -> InternalChainIsBoundedState.Spin
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InternalChainIsBoundedState): String = when (state) {
        is InternalChainIsBoundedState.Alt -> "alt"
        is InternalChainIsBoundedState.Bounded -> "bounded"
        is InternalChainIsBoundedState.Idle -> "idle"
        is InternalChainIsBoundedState.Ignoring -> "ignoring"
        is InternalChainIsBoundedState.Resuming -> "resuming"
        is InternalChainIsBoundedState.Spin -> "spin"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: InternalChainIsBoundedState): Int = when (state) {
        is InternalChainIsBoundedState.Alt -> 4
        is InternalChainIsBoundedState.Bounded -> 1
        is InternalChainIsBoundedState.Idle -> 0
        is InternalChainIsBoundedState.Ignoring -> 5
        is InternalChainIsBoundedState.Resuming -> 3
        is InternalChainIsBoundedState.Spin -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InternalChainIsBoundedEvent? = when (name) {
        "alternate" -> InternalChainIsBoundedEvent.Alternate
        "beat" -> InternalChainIsBoundedEvent.Beat
        "beatless" -> InternalChainIsBoundedEvent.Beatless
        "bounded" -> InternalChainIsBoundedEvent.Bounded
        "error.execution" -> InternalChainIsBoundedEvent.Error.Execution
        "link" -> InternalChainIsBoundedEvent.Link
        "poke" -> InternalChainIsBoundedEvent.Poke
        "resume" -> InternalChainIsBoundedEvent.Resume
        "spin" -> InternalChainIsBoundedEvent.Spin
        "tick" -> InternalChainIsBoundedEvent.Tick
        "unanswered" -> InternalChainIsBoundedEvent.Unanswered
        "unheard" -> InternalChainIsBoundedEvent.Unheard
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InternalChainIsBoundedEvent): String? = when (event) {
        is InternalChainIsBoundedEvent.Alternate -> "alternate"
        is InternalChainIsBoundedEvent.Beat -> "beat"
        is InternalChainIsBoundedEvent.Beatless -> "beatless"
        is InternalChainIsBoundedEvent.Bounded -> "bounded"
        is InternalChainIsBoundedEvent.Error.Execution -> "error.execution"
        is InternalChainIsBoundedEvent.Link -> "link"
        is InternalChainIsBoundedEvent.Poke -> "poke"
        is InternalChainIsBoundedEvent.Resume -> "resume"
        is InternalChainIsBoundedEvent.Spin -> "spin"
        is InternalChainIsBoundedEvent.Tick -> "tick"
        is InternalChainIsBoundedEvent.Unanswered -> "unanswered"
        is InternalChainIsBoundedEvent.Unheard -> "unheard"
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
            "internal_chain_is_bounded",
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
        // W3C SCXML 5.3: Initialize variable 'pokes' with expr
        try {
            val initResult_pokes = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "pokes", initResult_pokes)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='pokes'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'laps' with expr
        try {
            val initResult_laps = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "laps", initResult_laps)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='laps'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'links' with expr
        try {
            val initResult_links = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "links", initResult_links)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='links'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'beats' with expr
        try {
            val initResult_beats = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "beats", initResult_beats)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='beats'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'alts' with expr
        try {
            val initResult_alts = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "alts", initResult_alts)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='alts'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'ignores' with expr
        try {
            val initResult_ignores = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "ignores", initResult_ignores)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='ignores'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'pending' with expr
        try {
            val initResult_pending = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "pending", initResult_pending)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='pending'> expr failed to evaluate")
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
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, reason)
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
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: InternalChainIsBoundedEvent) {
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
    override fun bindCurrentEvent(event: InternalChainIsBoundedEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: InternalChainIsBoundedState, event: InternalChainIsBoundedEvent?): EnabledTransition<InternalChainIsBoundedState, HistoryId>? = when (state) {
        is InternalChainIsBoundedState.Alt -> firstEnabledTransitionOfAlt(state, event)
        is InternalChainIsBoundedState.Bounded -> firstEnabledTransitionOfBounded(state, event)
        is InternalChainIsBoundedState.Idle -> firstEnabledTransitionOfIdle(state, event)
        is InternalChainIsBoundedState.Ignoring -> firstEnabledTransitionOfIgnoring(state, event)
        is InternalChainIsBoundedState.Resuming -> firstEnabledTransitionOfResuming(state, event)
        is InternalChainIsBoundedState.Spin -> firstEnabledTransitionOfSpin(state, event)
    }


    // alt's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAlt(state: InternalChainIsBoundedState, event: InternalChainIsBoundedEvent?): EnabledTransition<InternalChainIsBoundedState, HistoryId>? = when {
        event is InternalChainIsBoundedEvent.Tick -> TransitionsPart0.transitionAltAt0
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(pending, 1)", "pending == 1")) -> TransitionsPart0.transitionAltAt1
        event is InternalChainIsBoundedEvent.Poke -> TransitionsPart0.transitionAltAt2
        else -> null
    }


    // bounded's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfBounded(state: InternalChainIsBoundedState, event: InternalChainIsBoundedEvent?): EnabledTransition<InternalChainIsBoundedState, HistoryId>? = when {
        event is InternalChainIsBoundedEvent.Link && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(laps < 999)", "laps < 999")) -> TransitionsPart0.transitionBoundedAt0
        event is InternalChainIsBoundedEvent.Link -> TransitionsPart0.transitionBoundedAt1
        event is InternalChainIsBoundedEvent.Poke -> TransitionsPart0.transitionBoundedAt2
        else -> null
    }


    // idle's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdle(state: InternalChainIsBoundedState, event: InternalChainIsBoundedEvent?): EnabledTransition<InternalChainIsBoundedState, HistoryId>? = when {
        event is InternalChainIsBoundedEvent.Poke -> TransitionsPart0.transitionIdleAt0
        event is InternalChainIsBoundedEvent.Bounded -> TransitionsPart0.transitionIdleAt1
        event is InternalChainIsBoundedEvent.Spin -> TransitionsPart0.transitionIdleAt2
        event is InternalChainIsBoundedEvent.Resume -> TransitionsPart0.transitionIdleAt3
        event is InternalChainIsBoundedEvent.Alternate -> TransitionsPart0.transitionIdleAt4
        event is InternalChainIsBoundedEvent.Unanswered -> TransitionsPart0.transitionIdleAt5
        else -> null
    }


    // ignoring's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIgnoring(state: InternalChainIsBoundedState, event: InternalChainIsBoundedEvent?): EnabledTransition<InternalChainIsBoundedState, HistoryId>? = when {
        event is InternalChainIsBoundedEvent.Beatless && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(ignores < 999)", "ignores < 999")) -> TransitionsPart0.transitionIgnoringAt0
        event is InternalChainIsBoundedEvent.Beatless -> TransitionsPart0.transitionIgnoringAt1
        event is InternalChainIsBoundedEvent.Poke -> TransitionsPart0.transitionIgnoringAt2
        else -> null
    }


    // resuming's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfResuming(state: InternalChainIsBoundedState, event: InternalChainIsBoundedEvent?): EnabledTransition<InternalChainIsBoundedState, HistoryId>? = when {
        event is InternalChainIsBoundedEvent.Beat && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(beats < 1499)", "beats < 1499")) -> TransitionsPart0.transitionResumingAt0
        event is InternalChainIsBoundedEvent.Beat -> TransitionsPart0.transitionResumingAt1
        event is InternalChainIsBoundedEvent.Poke -> TransitionsPart0.transitionResumingAt2
        else -> null
    }


    // spin's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSpin(state: InternalChainIsBoundedState, event: InternalChainIsBoundedEvent?): EnabledTransition<InternalChainIsBoundedState, HistoryId>? = when {
        event is InternalChainIsBoundedEvent.Link -> TransitionsPart0.transitionSpinAt0
        event is InternalChainIsBoundedEvent.Poke -> TransitionsPart0.transitionSpinAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: internal_chain_is_bounded.scxml:90 :: _machine
    override fun onEntry(state: InternalChainIsBoundedState, isDefaultEntry: Boolean) {
        when (state) {
            is InternalChainIsBoundedState.Alt -> onEntryOfAlt()
            is InternalChainIsBoundedState.Bounded -> onEntryOfBounded()
            is InternalChainIsBoundedState.Idle -> onEntryOfIdle()
            is InternalChainIsBoundedState.Ignoring -> onEntryOfIgnoring()
            is InternalChainIsBoundedState.Resuming -> onEntryOfResuming()
            is InternalChainIsBoundedState.Spin -> onEntryOfSpin()
        }
    }


    private fun onEntryOfAlt() {
                // SCE-MAP: internal_chain_is_bounded.scxml:211 :: alt :: _state_body
    }

    private fun onEntryOfBounded() {
                // SCE-MAP: internal_chain_is_bounded.scxml:153 :: bounded :: _state_body
    }

    private fun onEntryOfIdle() {
                // SCE-MAP: internal_chain_is_bounded.scxml:122 :: idle :: _state_body
    }

    private fun onEntryOfIgnoring() {
                // SCE-MAP: internal_chain_is_bounded.scxml:244 :: ignoring :: _state_body
    }

    private fun onEntryOfResuming() {
                // SCE-MAP: internal_chain_is_bounded.scxml:192 :: resuming :: _state_body
    }

    private fun onEntryOfSpin() {
                // SCE-MAP: internal_chain_is_bounded.scxml:174 :: spin :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: internal_chain_is_bounded.scxml:90 :: _machine
    override fun onExit(state: InternalChainIsBoundedState) {
        when (state) {
            is InternalChainIsBoundedState.Alt -> onExitOfAlt()
            is InternalChainIsBoundedState.Bounded -> onExitOfBounded()
            is InternalChainIsBoundedState.Idle -> onExitOfIdle()
            is InternalChainIsBoundedState.Ignoring -> onExitOfIgnoring()
            is InternalChainIsBoundedState.Resuming -> onExitOfResuming()
            is InternalChainIsBoundedState.Spin -> onExitOfSpin()
        }
    }


    private fun onExitOfAlt() {
                // SCE-MAP: internal_chain_is_bounded.scxml:211 :: alt :: _state_body
    }

    private fun onExitOfBounded() {
                // SCE-MAP: internal_chain_is_bounded.scxml:153 :: bounded :: _state_body
    }

    private fun onExitOfIdle() {
                // SCE-MAP: internal_chain_is_bounded.scxml:122 :: idle :: _state_body
    }

    private fun onExitOfIgnoring() {
                // SCE-MAP: internal_chain_is_bounded.scxml:244 :: ignoring :: _state_body
    }

    private fun onExitOfResuming() {
                // SCE-MAP: internal_chain_is_bounded.scxml:192 :: resuming :: _state_body
    }

    private fun onExitOfSpin() {
                // SCE-MAP: internal_chain_is_bounded.scxml:174 :: spin :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: internal_chain_is_bounded.scxml:90 :: _machine
    override fun executeTransitionContent(source: InternalChainIsBoundedState, transitionIndex: Int) {
        when (source) {
            is InternalChainIsBoundedState.Alt -> transitionContentOfAlt(transitionIndex)
            is InternalChainIsBoundedState.Bounded -> transitionContentOfBounded(transitionIndex)
            is InternalChainIsBoundedState.Idle -> transitionContentOfIdle(transitionIndex)
            is InternalChainIsBoundedState.Ignoring -> transitionContentOfIgnoring(transitionIndex)
            is InternalChainIsBoundedState.Resuming -> transitionContentOfResuming(transitionIndex)
            is InternalChainIsBoundedState.Spin -> transitionContentOfSpin(transitionIndex)
        }
    }


    // The content of `alt`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfAlt(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfAltAt0()
            1 -> transitionContentOfAltAt1()
            2 -> transitionContentOfAltAt2()
            else -> {}
        }
    }


    private fun transitionContentOfAltAt0() {
                // SCE-MAP: internal_chain_is_bounded.scxml:212 :: alt :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("alts", "alts"), com.sce.runtime.ScriptSource.lua("_scxml_add(alts, 1)", "alts + 1"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pending", "pending"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
    }

    private fun transitionContentOfAltAt1() {
                // SCE-MAP: internal_chain_is_bounded.scxml:216 :: alt :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pending", "pending"), com.sce.runtime.ScriptSource.lua("0", "0"))) {
                return
            }

            raiseInternal(InternalChainIsBoundedEvent.Tick)
    }

    private fun transitionContentOfAltAt2() {
                // SCE-MAP: internal_chain_is_bounded.scxml:220 :: alt :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
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
                // SCE-MAP: internal_chain_is_bounded.scxml:154 :: bounded :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("laps", "laps"), com.sce.runtime.ScriptSource.lua("_scxml_add(laps, 1)", "laps + 1"))) {
                return
            }

            raiseInternal(InternalChainIsBoundedEvent.Link)
    }

    private fun transitionContentOfBoundedAt1() {
                // SCE-MAP: internal_chain_is_bounded.scxml:158 :: bounded :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("laps", "laps"), com.sce.runtime.ScriptSource.lua("_scxml_add(laps, 1)", "laps + 1"))) {
                return
            }
    }

    private fun transitionContentOfBoundedAt2() {
                // SCE-MAP: internal_chain_is_bounded.scxml:163 :: bounded :: _transition_2


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
            1 -> transitionContentOfIdleAt1()
            2 -> transitionContentOfIdleAt2()
            3 -> transitionContentOfIdleAt3()
            4 -> transitionContentOfIdleAt4()
            5 -> transitionContentOfIdleAt5()
            else -> {}
        }
    }


    private fun transitionContentOfIdleAt0() {
                // SCE-MAP: internal_chain_is_bounded.scxml:123 :: idle :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
    }

    private fun transitionContentOfIdleAt1() {
                // SCE-MAP: internal_chain_is_bounded.scxml:126 :: idle :: _transition_1

            raiseInternal(InternalChainIsBoundedEvent.Link)
    }

    private fun transitionContentOfIdleAt2() {
                // SCE-MAP: internal_chain_is_bounded.scxml:129 :: idle :: _transition_2

            raiseInternal(InternalChainIsBoundedEvent.Link)
    }

    private fun transitionContentOfIdleAt3() {
                // SCE-MAP: internal_chain_is_bounded.scxml:132 :: idle :: _transition_3

            raiseInternal(InternalChainIsBoundedEvent.Beat)
    }

    private fun transitionContentOfIdleAt4() {
                // SCE-MAP: internal_chain_is_bounded.scxml:135 :: idle :: _transition_4

            raiseInternal(InternalChainIsBoundedEvent.Tick)
    }

    private fun transitionContentOfIdleAt5() {
                // SCE-MAP: internal_chain_is_bounded.scxml:138 :: idle :: _transition_5

            raiseInternal(InternalChainIsBoundedEvent.Beatless)
    }

    // The content of `ignoring`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfIgnoring(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfIgnoringAt0()
            1 -> transitionContentOfIgnoringAt1()
            2 -> transitionContentOfIgnoringAt2()
            else -> {}
        }
    }


    private fun transitionContentOfIgnoringAt0() {
                // SCE-MAP: internal_chain_is_bounded.scxml:245 :: ignoring :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("ignores", "ignores"), com.sce.runtime.ScriptSource.lua("_scxml_add(ignores, 1)", "ignores + 1"))) {
                return
            }

            raiseInternal(InternalChainIsBoundedEvent.Unheard)

            raiseInternal(InternalChainIsBoundedEvent.Beatless)
    }

    private fun transitionContentOfIgnoringAt1() {
                // SCE-MAP: internal_chain_is_bounded.scxml:250 :: ignoring :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("ignores", "ignores"), com.sce.runtime.ScriptSource.lua("_scxml_add(ignores, 1)", "ignores + 1"))) {
                return
            }
    }

    private fun transitionContentOfIgnoringAt2() {
                // SCE-MAP: internal_chain_is_bounded.scxml:253 :: ignoring :: _transition_2


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
                // SCE-MAP: internal_chain_is_bounded.scxml:193 :: resuming :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("beats", "beats"), com.sce.runtime.ScriptSource.lua("_scxml_add(beats, 1)", "beats + 1"))) {
                return
            }

            raiseInternal(InternalChainIsBoundedEvent.Beat)
    }

    private fun transitionContentOfResumingAt1() {
                // SCE-MAP: internal_chain_is_bounded.scxml:197 :: resuming :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("beats", "beats"), com.sce.runtime.ScriptSource.lua("_scxml_add(beats, 1)", "beats + 1"))) {
                return
            }
    }

    private fun transitionContentOfResumingAt2() {
                // SCE-MAP: internal_chain_is_bounded.scxml:200 :: resuming :: _transition_2


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
                // SCE-MAP: internal_chain_is_bounded.scxml:175 :: spin :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("links", "links"), com.sce.runtime.ScriptSource.lua("_scxml_add(links, 1)", "links + 1"))) {
                return
            }

            raiseInternal(InternalChainIsBoundedEvent.Link)
    }

    private fun transitionContentOfSpinAt1() {
                // SCE-MAP: internal_chain_is_bounded.scxml:179 :: spin :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("pokes", "pokes"), com.sce.runtime.ScriptSource.lua("_scxml_add(pokes, 1)", "pokes + 1"))) {
                return
            }
    }
}
