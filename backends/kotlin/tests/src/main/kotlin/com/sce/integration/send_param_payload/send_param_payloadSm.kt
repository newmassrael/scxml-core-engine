// SCE-GENERATED — DO NOT EDIT
// source-hash: 226fda00e41ba56f394c3663b4d11f262f4b5c1805c4daae2daf63854be69165

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/send_param_payload/send_param_payload.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: send_param_payload.scxml:82 :: _machine

package com.sce.integration.send_param_payload

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface SendParamPayloadState : State {
    data object AwaitChild : SendParamPayloadState
    data object EscapePhase : SendParamPayloadState
    data object FailBrokenParamDelivered : SendParamPayloadState
    data object FailChildPayload : SendParamPayloadState
    data object FailDuplicateParams : SendParamPayloadState
    data object FailEscapedMemberName : SendParamPayloadState
    data object FailEscapedText : SendParamPayloadState
    data object FailInternalPayload : SendParamPayloadState
    data object FailNoParamError : SendParamPayloadState
    data object FailNumberType : SendParamPayloadState
    data object FailSiblingParamLost : SendParamPayloadState
    data object FailStringType : SendParamPayloadState
    data object InternalPhase : SendParamPayloadState
    data object ParamErrorPhase : SendParamPayloadState
    data object Pass : SendParamPayloadState
    data object TypedPhase : SendParamPayloadState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface SendParamPayloadEvent : Event {
    sealed interface Done : SendParamPayloadEvent {
        data object Invoke : Done
    }
    sealed interface Error : SendParamPayloadEvent {
        data object Execution : Error
    }
    data object Escaped : SendParamPayloadEvent
    data object FromChild : SendParamPayloadEvent
    data object Loopback : SendParamPayloadEvent
    data object Typed : SendParamPayloadEvent
    data object WithBadParam : SendParamPayloadEvent
}
// --- State Machine (W3C SCXML) ---

class SendParamPayloadStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<SendParamPayloadState, SendParamPayloadEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `sawParamError` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `sawParamError` was assigned a value of another type, or the engine refused.
     */
    fun sawParamError(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "sawParamError")

    /**
     * §scxml-5.3: what the `lines` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `lines` was assigned a value of another type, or the engine refused.
     */
    fun lines(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "lines")

    /**
     * §scxml-5.3: what the `keyed` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `keyed` was assigned a value of another type, or the engine refused.
     *
     * The value as JSON text, serialised by the engine's own `JSON.stringify`
     * (§scxml-B-2) so the key order is the document's.
     */
    fun keyed(): String? =
        com.sce.runtime.DatamodelRead.readJson(scriptEngine, scriptSessionId, "keyed")

    /**
     * §scxml-5.3: what the `tag` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `tag` was assigned a value of another type, or the engine refused.
     */
    fun tag(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "tag")

    override val initialState: SendParamPayloadState = SendParamPayloadState.AwaitChild

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

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: SendParamPayloadState): Boolean = when (state) {
        is SendParamPayloadState.FailBrokenParamDelivered -> true
        is SendParamPayloadState.FailChildPayload -> true
        is SendParamPayloadState.FailDuplicateParams -> true
        is SendParamPayloadState.FailEscapedMemberName -> true
        is SendParamPayloadState.FailEscapedText -> true
        is SendParamPayloadState.FailInternalPayload -> true
        is SendParamPayloadState.FailNoParamError -> true
        is SendParamPayloadState.FailNumberType -> true
        is SendParamPayloadState.FailSiblingParamLost -> true
        is SendParamPayloadState.FailStringType -> true
        is SendParamPayloadState.Pass -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<SendParamPayloadState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<SendParamPayloadState, HistoryId>> =
            listOf(StateTarget(SendParamPayloadState.AwaitChild))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: awaitChild's transition 0, as the microstep reads it.
        val transitionAwaitChildAt0 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.AwaitChild,
            listOf(StateTarget(SendParamPayloadState.InternalPhase)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: awaitChild's transition 1, as the microstep reads it.
        val transitionAwaitChildAt1 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.AwaitChild,
            listOf(StateTarget(SendParamPayloadState.FailChildPayload)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: escapePhase's transition 0, as the microstep reads it.
        val transitionEscapePhaseAt0 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.EscapePhase,
            listOf(StateTarget(SendParamPayloadState.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: escapePhase's transition 1, as the microstep reads it.
        val transitionEscapePhaseAt1 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.EscapePhase,
            listOf(StateTarget(SendParamPayloadState.FailEscapedMemberName)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: escapePhase's transition 2, as the microstep reads it.
        val transitionEscapePhaseAt2 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.EscapePhase,
            listOf(StateTarget(SendParamPayloadState.FailEscapedText)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: internalPhase's transition 0, as the microstep reads it.
        val transitionInternalPhaseAt0 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.InternalPhase,
            listOf(StateTarget(SendParamPayloadState.TypedPhase)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: internalPhase's transition 1, as the microstep reads it.
        val transitionInternalPhaseAt1 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.InternalPhase,
            listOf(StateTarget(SendParamPayloadState.FailInternalPayload)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: paramErrorPhase's transition 0, as the microstep reads it.
        val transitionParamErrorPhaseAt0 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.ParamErrorPhase,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: paramErrorPhase's transition 1, as the microstep reads it.
        val transitionParamErrorPhaseAt1 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.ParamErrorPhase,
            listOf(StateTarget(SendParamPayloadState.FailNoParamError)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: paramErrorPhase's transition 2, as the microstep reads it.
        val transitionParamErrorPhaseAt2 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.ParamErrorPhase,
            listOf(StateTarget(SendParamPayloadState.FailBrokenParamDelivered)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: paramErrorPhase's transition 3, as the microstep reads it.
        val transitionParamErrorPhaseAt3 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.ParamErrorPhase,
            listOf(StateTarget(SendParamPayloadState.EscapePhase)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: paramErrorPhase's transition 4, as the microstep reads it.
        val transitionParamErrorPhaseAt4 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.ParamErrorPhase,
            listOf(StateTarget(SendParamPayloadState.FailSiblingParamLost)),
            4,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: typedPhase's transition 0, as the microstep reads it.
        val transitionTypedPhaseAt0 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.TypedPhase,
            listOf(StateTarget(SendParamPayloadState.FailNumberType)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: typedPhase's transition 1, as the microstep reads it.
        val transitionTypedPhaseAt1 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.TypedPhase,
            listOf(StateTarget(SendParamPayloadState.FailStringType)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: typedPhase's transition 2, as the microstep reads it.
        val transitionTypedPhaseAt2 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.TypedPhase,
            listOf(StateTarget(SendParamPayloadState.ParamErrorPhase)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: typedPhase's transition 3, as the microstep reads it.
        val transitionTypedPhaseAt3 = EnabledTransition<SendParamPayloadState, HistoryId>(
            SendParamPayloadState.TypedPhase,
            listOf(StateTarget(SendParamPayloadState.FailDuplicateParams)),
            3,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): SendParamPayloadState? = when (stateId) {
        "awaitChild" -> SendParamPayloadState.AwaitChild
        "escapePhase" -> SendParamPayloadState.EscapePhase
        "failBrokenParamDelivered" -> SendParamPayloadState.FailBrokenParamDelivered
        "failChildPayload" -> SendParamPayloadState.FailChildPayload
        "failDuplicateParams" -> SendParamPayloadState.FailDuplicateParams
        "failEscapedMemberName" -> SendParamPayloadState.FailEscapedMemberName
        "failEscapedText" -> SendParamPayloadState.FailEscapedText
        "failInternalPayload" -> SendParamPayloadState.FailInternalPayload
        "failNoParamError" -> SendParamPayloadState.FailNoParamError
        "failNumberType" -> SendParamPayloadState.FailNumberType
        "failSiblingParamLost" -> SendParamPayloadState.FailSiblingParamLost
        "failStringType" -> SendParamPayloadState.FailStringType
        "internalPhase" -> SendParamPayloadState.InternalPhase
        "paramErrorPhase" -> SendParamPayloadState.ParamErrorPhase
        "pass" -> SendParamPayloadState.Pass
        "typedPhase" -> SendParamPayloadState.TypedPhase
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: SendParamPayloadState): String = when (state) {
        is SendParamPayloadState.AwaitChild -> "awaitChild"
        is SendParamPayloadState.EscapePhase -> "escapePhase"
        is SendParamPayloadState.FailBrokenParamDelivered -> "failBrokenParamDelivered"
        is SendParamPayloadState.FailChildPayload -> "failChildPayload"
        is SendParamPayloadState.FailDuplicateParams -> "failDuplicateParams"
        is SendParamPayloadState.FailEscapedMemberName -> "failEscapedMemberName"
        is SendParamPayloadState.FailEscapedText -> "failEscapedText"
        is SendParamPayloadState.FailInternalPayload -> "failInternalPayload"
        is SendParamPayloadState.FailNoParamError -> "failNoParamError"
        is SendParamPayloadState.FailNumberType -> "failNumberType"
        is SendParamPayloadState.FailSiblingParamLost -> "failSiblingParamLost"
        is SendParamPayloadState.FailStringType -> "failStringType"
        is SendParamPayloadState.InternalPhase -> "internalPhase"
        is SendParamPayloadState.ParamErrorPhase -> "paramErrorPhase"
        is SendParamPayloadState.Pass -> "pass"
        is SendParamPayloadState.TypedPhase -> "typedPhase"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: SendParamPayloadState): Int = when (state) {
        is SendParamPayloadState.AwaitChild -> 0
        is SendParamPayloadState.EscapePhase -> 4
        is SendParamPayloadState.FailBrokenParamDelivered -> 12
        is SendParamPayloadState.FailChildPayload -> 6
        is SendParamPayloadState.FailDuplicateParams -> 10
        is SendParamPayloadState.FailEscapedMemberName -> 15
        is SendParamPayloadState.FailEscapedText -> 14
        is SendParamPayloadState.FailInternalPayload -> 7
        is SendParamPayloadState.FailNoParamError -> 11
        is SendParamPayloadState.FailNumberType -> 8
        is SendParamPayloadState.FailSiblingParamLost -> 13
        is SendParamPayloadState.FailStringType -> 9
        is SendParamPayloadState.InternalPhase -> 1
        is SendParamPayloadState.ParamErrorPhase -> 3
        is SendParamPayloadState.Pass -> 5
        is SendParamPayloadState.TypedPhase -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): SendParamPayloadEvent? = when (name) {
        "done.invoke" -> SendParamPayloadEvent.Done.Invoke
        "error.execution" -> SendParamPayloadEvent.Error.Execution
        "escaped" -> SendParamPayloadEvent.Escaped
        "fromChild" -> SendParamPayloadEvent.FromChild
        "loopback" -> SendParamPayloadEvent.Loopback
        "typed" -> SendParamPayloadEvent.Typed
        "withBadParam" -> SendParamPayloadEvent.WithBadParam
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: SendParamPayloadEvent): String? = when (event) {
        is SendParamPayloadEvent.Done.Invoke -> "done.invoke"
        is SendParamPayloadEvent.Error.Execution -> "error.execution"
        is SendParamPayloadEvent.Escaped -> "escaped"
        is SendParamPayloadEvent.FromChild -> "fromChild"
        is SendParamPayloadEvent.Loopback -> "loopback"
        is SendParamPayloadEvent.Typed -> "typed"
        is SendParamPayloadEvent.WithBadParam -> "withBadParam"
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
            "send_param_payload",
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

        // W3C SCXML 5.3: Early binding — initialize state-level datamodel variables at startup
        declareStateVariablesPart0(engine, sid)

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
        // W3C SCXML 5.3: Initialize variable 'nothing' with expr
        try {
            val initResult_nothing = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("nil", "null"))
            engine.setVariable(sid, "nothing", initResult_nothing)
        } catch (e: Exception) {
            raisePlatformError(SendParamPayloadEvent.Error.Execution, "<data id='nothing'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'sawParamError' with expr
        try {
            val initResult_sawParamError = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "sawParamError", initResult_sawParamError)
        } catch (e: Exception) {
            raisePlatformError(SendParamPayloadEvent.Error.Execution, "<data id='sawParamError'> expr failed to evaluate")
        }
    }

    private fun declareStateVariablesPart0(engine: ScxmlScriptEngine, sid: String) {
        // State 'escapePhase' variable 'lines'
        try {
            val initResult_lines = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"x\\ny\"", "'x\\ny'"))
            engine.setVariable(sid, "lines", initResult_lines)
        } catch (e: Exception) {
            raisePlatformError(SendParamPayloadEvent.Error.Execution, "<data id='lines'> expr failed to evaluate")
        }
        // State 'escapePhase' variable 'keyed'
        try {
            val initResult_keyed = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{[\"q\\\"k\"] = 1, [\"b\\\\k\"] = 2}", "({'q\"k': 1, 'b\\\\k': 2})"))
            engine.setVariable(sid, "keyed", initResult_keyed)
        } catch (e: Exception) {
            raisePlatformError(SendParamPayloadEvent.Error.Execution, "<data id='keyed'> expr failed to evaluate")
        }
        // State 'typedPhase' variable 'tag'
        try {
            val initResult_tag = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"kept\"", "'kept'"))
            engine.setVariable(sid, "tag", initResult_tag)
        } catch (e: Exception) {
            raisePlatformError(SendParamPayloadEvent.Error.Execution, "<data id='tag'> expr failed to evaluate")
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
            raisePlatformError(SendParamPayloadEvent.Error.Execution, reason)
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
            raisePlatformError(SendParamPayloadEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(SendParamPayloadEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(SendParamPayloadEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: SendParamPayloadEvent) {
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
    override fun bindCurrentEvent(event: SendParamPayloadEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: SendParamPayloadState, event: SendParamPayloadEvent?): EnabledTransition<SendParamPayloadState, HistoryId>? = when (state) {
        is SendParamPayloadState.AwaitChild -> firstEnabledTransitionOfAwaitChild(state, event)
        is SendParamPayloadState.EscapePhase -> firstEnabledTransitionOfEscapePhase(state, event)
        is SendParamPayloadState.InternalPhase -> firstEnabledTransitionOfInternalPhase(state, event)
        is SendParamPayloadState.ParamErrorPhase -> firstEnabledTransitionOfParamErrorPhase(state, event)
        is SendParamPayloadState.TypedPhase -> firstEnabledTransitionOfTypedPhase(state, event)
        else -> null
    }


    // awaitChild's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAwaitChild(state: SendParamPayloadState, event: SendParamPayloadEvent?): EnabledTransition<SendParamPayloadState, HistoryId>? = when {
        event is SendParamPayloadEvent.FromChild && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_scxml_truthy(_event.data) and (_event.data.value == \"42\"))", "_event.data && _event.data.value === '42'")) -> TransitionsPart0.transitionAwaitChildAt0
        event is SendParamPayloadEvent.FromChild -> TransitionsPart0.transitionAwaitChildAt1
        else -> null
    }


    // escapePhase's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfEscapePhase(state: SendParamPayloadState, event: SendParamPayloadEvent?): EnabledTransition<SendParamPayloadState, HistoryId>? = when {
        event is SendParamPayloadEvent.Escaped && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(((_event.data.text == \"x\\ny\") and (_event.data.obj[\"q\\\"k\"] == 1)) and (_event.data.obj[\"b\\\\k\"] == 2))", "_event.data.text === 'x\\ny' && _event.data.obj['q\"k'] === 1 && _event.data.obj['b\\\\k'] === 2")) -> TransitionsPart0.transitionEscapePhaseAt0
        event is SendParamPayloadEvent.Escaped && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.text == \"x\\ny\")", "_event.data.text === 'x\\ny'")) -> TransitionsPart0.transitionEscapePhaseAt1
        event is SendParamPayloadEvent.Escaped -> TransitionsPart0.transitionEscapePhaseAt2
        else -> null
    }


    // internalPhase's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfInternalPhase(state: SendParamPayloadState, event: SendParamPayloadEvent?): EnabledTransition<SendParamPayloadState, HistoryId>? = when {
        event is SendParamPayloadEvent.Loopback && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_scxml_truthy(_event.data) and (_event.data.carried == \"kept\"))", "_event.data && _event.data.carried === 'kept'")) -> TransitionsPart0.transitionInternalPhaseAt0
        event is SendParamPayloadEvent.Loopback -> TransitionsPart0.transitionInternalPhaseAt1
        else -> null
    }


    // paramErrorPhase's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfParamErrorPhase(state: SendParamPayloadState, event: SendParamPayloadEvent?): EnabledTransition<SendParamPayloadState, HistoryId>? = when {
        event is SendParamPayloadEvent.Error.Execution -> TransitionsPart0.transitionParamErrorPhaseAt0
        event is SendParamPayloadEvent.WithBadParam && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(sawParamError ~= 1)", "sawParamError !== 1")) -> TransitionsPart0.transitionParamErrorPhaseAt1
        event is SendParamPayloadEvent.WithBadParam && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.broken == \"\")", "_event.data.broken === ''")) -> TransitionsPart0.transitionParamErrorPhaseAt2
        event is SendParamPayloadEvent.WithBadParam && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.kept == \"here\")", "_event.data.kept === 'here'")) -> TransitionsPart0.transitionParamErrorPhaseAt3
        event is SendParamPayloadEvent.WithBadParam -> TransitionsPart0.transitionParamErrorPhaseAt4
        else -> null
    }


    // typedPhase's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfTypedPhase(state: SendParamPayloadState, event: SendParamPayloadEvent?): EnabledTransition<SendParamPayloadState, HistoryId>? = when {
        event is SendParamPayloadEvent.Typed && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.n ~= 7)", "_event.data.n !== 7")) -> TransitionsPart0.transitionTypedPhaseAt0
        event is SendParamPayloadEvent.Typed && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.s ~= \"kept\")", "_event.data.s !== 'kept'")) -> TransitionsPart0.transitionTypedPhaseAt1
        event is SendParamPayloadEvent.Typed && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(((#_event.data.d == 2) and (_event.data.d[1] == 1)) and (_event.data.d[2] == 2))", "_event.data.d.length === 2 && _event.data.d[0] === 1 && _event.data.d[1] === 2")) -> TransitionsPart0.transitionTypedPhaseAt2
        event is SendParamPayloadEvent.Typed -> TransitionsPart0.transitionTypedPhaseAt3
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: send_param_payload.scxml:82 :: _machine
    override fun onEntry(state: SendParamPayloadState, isDefaultEntry: Boolean) {
        when (state) {
            is SendParamPayloadState.AwaitChild -> onEntryOfAwaitChild()
            is SendParamPayloadState.EscapePhase -> onEntryOfEscapePhase()
            is SendParamPayloadState.FailBrokenParamDelivered -> onEntryOfFailBrokenParamDelivered()
            is SendParamPayloadState.FailChildPayload -> onEntryOfFailChildPayload()
            is SendParamPayloadState.FailDuplicateParams -> onEntryOfFailDuplicateParams()
            is SendParamPayloadState.FailEscapedMemberName -> onEntryOfFailEscapedMemberName()
            is SendParamPayloadState.FailEscapedText -> onEntryOfFailEscapedText()
            is SendParamPayloadState.FailInternalPayload -> onEntryOfFailInternalPayload()
            is SendParamPayloadState.FailNoParamError -> onEntryOfFailNoParamError()
            is SendParamPayloadState.FailNumberType -> onEntryOfFailNumberType()
            is SendParamPayloadState.FailSiblingParamLost -> onEntryOfFailSiblingParamLost()
            is SendParamPayloadState.FailStringType -> onEntryOfFailStringType()
            is SendParamPayloadState.InternalPhase -> onEntryOfInternalPhase()
            is SendParamPayloadState.ParamErrorPhase -> onEntryOfParamErrorPhase()
            is SendParamPayloadState.Pass -> onEntryOfPass()
            is SendParamPayloadState.TypedPhase -> onEntryOfTypedPhase()
        }
    }


    private fun onEntryOfAwaitChild() {
                // SCE-MAP: send_param_payload.scxml:100 :: awaitChild :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("inv_emitter", SendParamPayloadState.AwaitChild)
    }

    private fun onEntryOfEscapePhase() {
                // SCE-MAP: send_param_payload.scxml:236 :: escapePhase :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            var paramFailed = false
            assertScriptSession()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "text", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("lines", "lines")))
            } catch (_: Exception) {
                raisePlatformError(SendParamPayloadEvent.Error.Execution, "<send> <param name='text'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "obj", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("keyed", "keyed")))
            } catch (_: Exception) {
                raisePlatformError(SendParamPayloadEvent.Error.Execution, "<send> <param name='obj'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(SendParamPayloadEvent.Escaped, EventMetadata.internal(sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfFailBrokenParamDelivered() {
                // SCE-MAP: send_param_payload.scxml:261 :: failBrokenParamDelivered :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailChildPayload() {
                // SCE-MAP: send_param_payload.scxml:255 :: failChildPayload :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailDuplicateParams() {
                // SCE-MAP: send_param_payload.scxml:259 :: failDuplicateParams :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailEscapedMemberName() {
                // SCE-MAP: send_param_payload.scxml:264 :: failEscapedMemberName :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailEscapedText() {
                // SCE-MAP: send_param_payload.scxml:263 :: failEscapedText :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailInternalPayload() {
                // SCE-MAP: send_param_payload.scxml:256 :: failInternalPayload :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailNoParamError() {
                // SCE-MAP: send_param_payload.scxml:260 :: failNoParamError :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailNumberType() {
                // SCE-MAP: send_param_payload.scxml:257 :: failNumberType :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailSiblingParamLost() {
                // SCE-MAP: send_param_payload.scxml:262 :: failSiblingParamLost :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailStringType() {
                // SCE-MAP: send_param_payload.scxml:258 :: failStringType :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfInternalPhase() {
                // SCE-MAP: send_param_payload.scxml:125 :: internalPhase :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "carried", "kept")
            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(SendParamPayloadEvent.Loopback, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfParamErrorPhase() {
                // SCE-MAP: send_param_payload.scxml:192 :: paramErrorPhase :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            var paramFailed = false
            assertScriptSession()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "kept", "here")
            try {
                putParam(sendPayload, "broken", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("nothing.deep", "nothing.deep")))
            } catch (_: Exception) {
                raisePlatformError(SendParamPayloadEvent.Error.Execution, "<send> <param name='broken'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(SendParamPayloadEvent.WithBadParam, EventMetadata.internal(sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfPass() {
                // SCE-MAP: send_param_payload.scxml:254 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfTypedPhase() {
                // SCE-MAP: send_param_payload.scxml:141 :: typedPhase :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            var paramFailed = false
            assertScriptSession()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "n", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("7", "7")))
            } catch (_: Exception) {
                raisePlatformError(SendParamPayloadEvent.Error.Execution, "<send> <param name='n'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "s", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("tag", "tag")))
            } catch (_: Exception) {
                raisePlatformError(SendParamPayloadEvent.Error.Execution, "<send> <param name='s'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "d", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("1", "1")))
            } catch (_: Exception) {
                raisePlatformError(SendParamPayloadEvent.Error.Execution, "<send> <param name='d'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "d", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("2", "2")))
            } catch (_: Exception) {
                raisePlatformError(SendParamPayloadEvent.Error.Execution, "<send> <param name='d'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(SendParamPayloadEvent.Typed, EventMetadata.internal(sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
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
    private fun deferStaticInvoke(invokeId: String, state: SendParamPayloadState) {
        when (invokeId) {
            "inv_emitter" -> startInvokeOfAwaitChildAt0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfAwaitChildAt0(state: SendParamPayloadState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "awaitChild.${nextInvokeStart("inv_emitter")}.inv_emitter"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(SendParamPayloadSceSynthInvokeInvEmitterStateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("inv_emitter", childSM, false, SendParamPayloadEvent.Done.Invoke, "", generatedInvokeId, "", "send_param_payload__sce_synth_invoke__inv_emitter", "inv_emitter")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: send_param_payload.scxml:82 :: _machine
    override fun onExit(state: SendParamPayloadState) {
        when (state) {
            is SendParamPayloadState.AwaitChild -> onExitOfAwaitChild()
            is SendParamPayloadState.EscapePhase -> onExitOfEscapePhase()
            is SendParamPayloadState.FailBrokenParamDelivered -> onExitOfFailBrokenParamDelivered()
            is SendParamPayloadState.FailChildPayload -> onExitOfFailChildPayload()
            is SendParamPayloadState.FailDuplicateParams -> onExitOfFailDuplicateParams()
            is SendParamPayloadState.FailEscapedMemberName -> onExitOfFailEscapedMemberName()
            is SendParamPayloadState.FailEscapedText -> onExitOfFailEscapedText()
            is SendParamPayloadState.FailInternalPayload -> onExitOfFailInternalPayload()
            is SendParamPayloadState.FailNoParamError -> onExitOfFailNoParamError()
            is SendParamPayloadState.FailNumberType -> onExitOfFailNumberType()
            is SendParamPayloadState.FailSiblingParamLost -> onExitOfFailSiblingParamLost()
            is SendParamPayloadState.FailStringType -> onExitOfFailStringType()
            is SendParamPayloadState.InternalPhase -> onExitOfInternalPhase()
            is SendParamPayloadState.ParamErrorPhase -> onExitOfParamErrorPhase()
            is SendParamPayloadState.Pass -> onExitOfPass()
            is SendParamPayloadState.TypedPhase -> onExitOfTypedPhase()
        }
    }


    private fun onExitOfAwaitChild() {
                // SCE-MAP: send_param_payload.scxml:100 :: awaitChild :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(SendParamPayloadState.AwaitChild)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_emitter")
    }

    private fun onExitOfEscapePhase() {
                // SCE-MAP: send_param_payload.scxml:236 :: escapePhase :: _state_body
    }

    private fun onExitOfFailBrokenParamDelivered() {
                // SCE-MAP: send_param_payload.scxml:261 :: failBrokenParamDelivered :: _state_body
    }

    private fun onExitOfFailChildPayload() {
                // SCE-MAP: send_param_payload.scxml:255 :: failChildPayload :: _state_body
    }

    private fun onExitOfFailDuplicateParams() {
                // SCE-MAP: send_param_payload.scxml:259 :: failDuplicateParams :: _state_body
    }

    private fun onExitOfFailEscapedMemberName() {
                // SCE-MAP: send_param_payload.scxml:264 :: failEscapedMemberName :: _state_body
    }

    private fun onExitOfFailEscapedText() {
                // SCE-MAP: send_param_payload.scxml:263 :: failEscapedText :: _state_body
    }

    private fun onExitOfFailInternalPayload() {
                // SCE-MAP: send_param_payload.scxml:256 :: failInternalPayload :: _state_body
    }

    private fun onExitOfFailNoParamError() {
                // SCE-MAP: send_param_payload.scxml:260 :: failNoParamError :: _state_body
    }

    private fun onExitOfFailNumberType() {
                // SCE-MAP: send_param_payload.scxml:257 :: failNumberType :: _state_body
    }

    private fun onExitOfFailSiblingParamLost() {
                // SCE-MAP: send_param_payload.scxml:262 :: failSiblingParamLost :: _state_body
    }

    private fun onExitOfFailStringType() {
                // SCE-MAP: send_param_payload.scxml:258 :: failStringType :: _state_body
    }

    private fun onExitOfInternalPhase() {
                // SCE-MAP: send_param_payload.scxml:125 :: internalPhase :: _state_body
    }

    private fun onExitOfParamErrorPhase() {
                // SCE-MAP: send_param_payload.scxml:192 :: paramErrorPhase :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: send_param_payload.scxml:254 :: pass :: _state_body
    }

    private fun onExitOfTypedPhase() {
                // SCE-MAP: send_param_payload.scxml:141 :: typedPhase :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: send_param_payload.scxml:82 :: _machine
    override fun executeTransitionContent(source: SendParamPayloadState, transitionIndex: Int) {
        when (source) {
            is SendParamPayloadState.ParamErrorPhase -> transitionContentOfParamErrorPhase(transitionIndex)
            else -> {}
        }
    }


    // The content of `paramErrorPhase`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfParamErrorPhase(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfParamErrorPhaseAt0()
            else -> {}
        }
    }


    private fun transitionContentOfParamErrorPhaseAt0() {
                // SCE-MAP: send_param_payload.scxml:199 :: paramErrorPhase :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("sawParamError", "sawParamError"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
    }
}
