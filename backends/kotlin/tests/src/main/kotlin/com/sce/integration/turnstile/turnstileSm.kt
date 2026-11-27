// SCE-GENERATED — DO NOT EDIT
// source-hash: fe4fa4d749009cb7fbfccef450719db25d833a49d5a0fed035004f7f4d8f9354
// source: turnstile.scxml

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/formal_properties/turnstile.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: turnstile.scxml:5 :: _machine

package com.sce.integration.turnstile

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface TurnstileState : State {
    data object Locked : TurnstileState
    data object Unlocked : TurnstileState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface TurnstileEvent : Event {
    data object Coin : TurnstileEvent
    sealed interface Error : TurnstileEvent {
        data object Execution : Error
    }
    data object Push : TurnstileEvent
    data object Relock : TurnstileEvent
}
// --- State Machine (W3C SCXML) ---

class TurnstileStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<TurnstileState, TurnstileEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `coins` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `coins` was assigned a value of another type, or the engine refused.
     */
    fun coins(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "coins")

    override val initialState: TurnstileState = TurnstileState.Locked

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
    override val documentInitialTargets: List<EntryTarget<TurnstileState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<TurnstileState, HistoryId>> =
            listOf(StateTarget(TurnstileState.Locked))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: locked's transition 0, as the microstep reads it.
        val transitionLockedAt0 = EnabledTransition<TurnstileState, HistoryId>(
            TurnstileState.Locked,
            listOf(StateTarget(TurnstileState.Unlocked)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: unlocked's transition 0, as the microstep reads it.
        val transitionUnlockedAt0 = EnabledTransition<TurnstileState, HistoryId>(
            TurnstileState.Unlocked,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: unlocked's transition 1, as the microstep reads it.
        val transitionUnlockedAt1 = EnabledTransition<TurnstileState, HistoryId>(
            TurnstileState.Unlocked,
            listOf(StateTarget(TurnstileState.Locked)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: unlocked's transition 2, as the microstep reads it.
        val transitionUnlockedAt2 = EnabledTransition<TurnstileState, HistoryId>(
            TurnstileState.Unlocked,
            listOf(StateTarget(TurnstileState.Locked)),
            2,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): TurnstileState? = when (stateId) {
        "locked" -> TurnstileState.Locked
        "unlocked" -> TurnstileState.Unlocked
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: TurnstileState): String = when (state) {
        is TurnstileState.Locked -> "locked"
        is TurnstileState.Unlocked -> "unlocked"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: TurnstileState): Int = when (state) {
        is TurnstileState.Locked -> 0
        is TurnstileState.Unlocked -> 1
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): TurnstileEvent? = when (name) {
        "coin" -> TurnstileEvent.Coin
        "error.execution" -> TurnstileEvent.Error.Execution
        "push" -> TurnstileEvent.Push
        "relock" -> TurnstileEvent.Relock
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: TurnstileEvent): String? = when (event) {
        is TurnstileEvent.Coin -> "coin"
        is TurnstileEvent.Error.Execution -> "error.execution"
        is TurnstileEvent.Push -> "push"
        is TurnstileEvent.Relock -> "relock"
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
            "turnstile",
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
        // §scxml-6.4: 'coins' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "coins", TurnstileEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'coins' with expr
            try {
                engine.setVariable(sid, "coins", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0")))
            } catch (e: Exception) {
                raisePlatformError(TurnstileEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='coins'> expr failed to evaluate")
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
    private fun evaluateGuardRaising(guardExpr: com.sce.runtime.ScriptSource, detail: String): Boolean? {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(TurnstileEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, detail)
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
            raisePlatformError(TurnstileEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "an expression could not be serialised to JSON")
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
            raisePlatformError(TurnstileEvent.Error.Execution, ErrorReason.ASSIGNMENT_FAILED, "<assign> failed")
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
            raisePlatformError(TurnstileEvent.Error.Execution, ErrorReason.SCRIPT_FAILED, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: TurnstileEvent) {
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
    override fun bindCurrentEvent(event: TurnstileEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: TurnstileState, event: TurnstileEvent?): EnabledTransition<TurnstileState, HistoryId>? = when (state) {
        is TurnstileState.Locked -> firstEnabledTransitionOfLocked(state, event)
        is TurnstileState.Unlocked -> firstEnabledTransitionOfUnlocked(state, event)
    }


    // locked's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfLocked(state: TurnstileState, event: TurnstileEvent?): EnabledTransition<TurnstileState, HistoryId>? = when {
        event is TurnstileEvent.Coin -> TransitionsPart0.transitionLockedAt0
        else -> null
    }


    // unlocked's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfUnlocked(state: TurnstileState, event: TurnstileEvent?): EnabledTransition<TurnstileState, HistoryId>? = when {
        event is TurnstileEvent.Coin -> TransitionsPart0.transitionUnlockedAt0
        event is TurnstileEvent.Push -> TransitionsPart0.transitionUnlockedAt1
        event is TurnstileEvent.Relock -> TransitionsPart0.transitionUnlockedAt2
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: turnstile.scxml:5 :: _machine
    override fun onEntry(state: TurnstileState, isDefaultEntry: Boolean) {
        when (state) {
            is TurnstileState.Locked -> onEntryOfLocked()
            is TurnstileState.Unlocked -> onEntryOfUnlocked()
        }
    }


    private fun onEntryOfLocked() {
                // SCE-MAP: turnstile.scxml:10 :: locked :: _state_body
    }

    private fun onEntryOfUnlocked() {
                // SCE-MAP: turnstile.scxml:15 :: unlocked :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 5000L, TurnstileEvent.Relock, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: turnstile.scxml:5 :: _machine
    override fun onExit(state: TurnstileState) {
        when (state) {
            is TurnstileState.Locked -> onExitOfLocked()
            is TurnstileState.Unlocked -> onExitOfUnlocked()
        }
    }


    private fun onExitOfLocked() {
                // SCE-MAP: turnstile.scxml:10 :: locked :: _state_body
    }

    private fun onExitOfUnlocked() {
                // SCE-MAP: turnstile.scxml:15 :: unlocked :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: turnstile.scxml:5 :: _machine
    override fun executeTransitionContent(source: TurnstileState, transitionIndex: Int) {
        when (source) {
            is TurnstileState.Locked -> transitionContentOfLocked(transitionIndex)
            is TurnstileState.Unlocked -> transitionContentOfUnlocked(transitionIndex)
        }
    }


    // The content of `locked`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfLocked(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfLockedAt0()
            else -> {}
        }
    }


    private fun transitionContentOfLockedAt0() {
                // SCE-MAP: turnstile.scxml:11 :: locked :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("coins", "coins"), com.sce.runtime.ScriptSource.lua("_scxml_add(coins, 1)", "coins + 1"))) {
                return
            }
    }

    // The content of `unlocked`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfUnlocked(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfUnlockedAt0()
            else -> {}
        }
    }


    private fun transitionContentOfUnlockedAt0() {
                // SCE-MAP: turnstile.scxml:19 :: unlocked :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("coins", "coins"), com.sce.runtime.ScriptSource.lua("_scxml_add(coins, 1)", "coins + 1"))) {
                return
            }
    }
}
