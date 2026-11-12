// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_invoke_ending_order.scxml#worker
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_invoke_ending_order.scxml:45 :: _machine

package com.sce.integration.static_datamodel.static_invoke_ending_order

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticInvokeEndingOrderSceSynthInvokeWorkerState : State {
    data object Waiting : StaticInvokeEndingOrderSceSynthInvokeWorkerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticInvokeEndingOrderSceSynthInvokeWorkerEvent : Event {
    data object Answer : StaticInvokeEndingOrderSceSynthInvokeWorkerEvent
    sealed interface Done : StaticInvokeEndingOrderSceSynthInvokeWorkerEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Clock : Invoke
        }
    }
    sealed interface Error : StaticInvokeEndingOrderSceSynthInvokeWorkerEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Question : StaticInvokeEndingOrderSceSynthInvokeWorkerEvent
    data object Stamped : StaticInvokeEndingOrderSceSynthInvokeWorkerEvent
}
// --- State Machine (W3C SCXML) ---

// ── SCE Accepted Subset 2.12: typed host-run invoke interface ─────────
/** `sce:result`: the record the host completes `<invoke id="clock">` with (SCE Accepted Subset §2.12). */
data class StaticInvokeEndingOrderSceSynthInvokeWorkerClockResult(
    val error: String,
    val utc_ms: Long,
    val monotonic_ms: Long,
) {
    /** The JSON `done.invoke.clock` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "utc_ms" to utc_ms, "monotonic_ms" to monotonic_ms))
}

/**
 * The host side of this document's typed `<invoke type="sce-host-clock-now">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostClockNowInvoker`.
 */
interface StaticInvokeEndingOrderSceSynthInvokeWorkerSceHostClockNowInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="clock">`. [token] names this start;
     * a host that finishes later hands it back to `completeClock` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startClock(token: Long): StaticInvokeEndingOrderSceSynthInvokeWorkerClockResult?

    /** §scxml-6.4: `<invoke id="clock">`'s state exited while the start [token] names was still running. */
    fun cancelClock(token: Long)
}


class StaticInvokeEndingOrderSceSynthInvokeWorkerStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticInvokeEndingOrderSceSynthInvokeWorkerState, StaticInvokeEndingOrderSceSynthInvokeWorkerEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: saving this machine, restoring it ─────────

    /**
     * What a saved state of this document is bound to: a state saved from a
     * document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not, and one
     * saved at an earlier revision of a record schema is read with each field a
     * later revision added at its default.
     */
    val savedShape: SavedShape = SavedShape(
        first = "4139b120af2ce488ab337fea508e9741f1c720df00100a9174250f37cc3726c3",
        revised = "8cd275f671dde4e781774717538ab697ba1f6aa6506469cdccb56dcb53a3debe",
        format2First = "4139b120af2ce488ab337fea508e9741f1c720df00100a9174250f37cc3726c3",
        format2Revised = "8cd275f671dde4e781774717538ab697ba1f6aa6506469cdccb56dcb53a3debe",
        records = listOf(
        ),
        enums = listOf(
        ),
    )

    /**
     * This machine's whole state at the macrostep boundary it stands at —
     * every variable, the machine's own included, and where it stands — as
     * the `sce-saved-state` document every backend reads ([SavedState.toJson]).
     * Each delayed `<send>` still waiting is written as the moment it comes due
     * on the wall clock whose reading now is [wallNowMs], in milliseconds since
     * the Unix epoch.
     *
     * @throws StateRefusal for a machine that is not running, or whose last
     *   macrostep stopped at the microstep ceiling.
     */
    fun save(wallNowMs: Long): SavedState {
        val variables = LinkedHashMap<String, Any?>()
        return savedState(savedShape, variables, wallNowMs)
    }

    /** [save] at the host's wall clock now. */
    fun save(): SavedState = save(SavedState.wallClockMs())

    /**
     * Stand this machine where [saved] left one, in place of [initialize]: no
     * `<onentry>` runs and no `<data>` is evaluated, since the saved run
     * already did both. Every value is read before any is written, so a
     * refused restore leaves the machine as it was.
     *
     * The delayed sends [saved] holds are armed against this machine's `clock`,
     * which is installed before a restore as before [initialize]; [wallNowMs]
     * is what time it is on the wall clock the saved `due`s were written
     * against. A send comes due when its saved moment does, and one already due
     * comes due now.
     *
     * @throws StateRefusal for a machine that has already started, a state
     *   saved from a document of another shape, a configuration that is not
     *   one of this document, or a value its variable's type cannot hold.
     */
    fun restore(saved: SavedState, wallNowMs: Long) {
        // The state as one of this machine: each record saved at an earlier
        // revision of its schema holds the defaults of the fields added since.
        val savedGrown = beginRestore(saved, savedShape)
        enterSaved(savedGrown, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // docs/adr/0045: this machine as a child session of another's saved state —
    // saved with it, judged with it before anything is built, and restored in
    // place of initialized.
    override fun saveAsSession(wallNowMs: Long): SavedState = save(wallNowMs)

    override fun judgeAsSession(saved: SavedState) {
        val savedGrown = beginRestore(saved, savedShape)
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, StaticInvokeEndingOrderSceSynthInvokeWorkerState>> = listOf(
        Triple("sce-host-clock-now", "clock", StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting),
    )

    // SCE Accepted Subset 2.12: registration and typed completion for this
    // document's typed host-run invokes.

    /**
     * Complete `<invoke id="clock">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`). An owner-thread operation; from another
     * thread, [postCompleteClock].
     */
    fun completeClock(token: Long, result: StaticInvokeEndingOrderSceSynthInvokeWorkerClockResult): Boolean =
        completeHostInvoke("sce-host-clock-now", "clock", token, result.wire())

    /**
     * [completeClock] from any thread: the completion is posted to this
     * machine's `inbox`, and its owner applies it with `completeHostInvoke` at
     * its next macrostep boundary, where the token is checked. `true` when the
     * inbox took it.
     */
    fun postCompleteClock(token: Long, result: StaticInvokeEndingOrderSceSynthInvokeWorkerClockResult): Boolean =
        inbox.post(
            com.sce.runtime.InboxInput.HostInvokeEnding(
                sessionId = null,
                processorType = "sce-host-clock-now",
                invokeId = "clock",
                token = token,
                done = true,
                data = result.wire(),
            ),
        )

    /**
     * Register [invoker] as the handler for `type="sce-host-clock-now"`.
     */
    fun registerSceHostClockNowInvoker(invoker: StaticInvokeEndingOrderSceSynthInvokeWorkerSceHostClockNowInvoker) {
        registerInvoker("sce-host-clock-now") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "clock" -> {
                        invoker.startClock(start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "clock" -> {
                        invoker.cancelClock(cancel.token)
                        null
                    }
                    else -> null
                }
            } else {
                null
            }
        }
    }


    override val initialState: StaticInvokeEndingOrderSceSynthInvokeWorkerState = StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = true

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticInvokeEndingOrderSceSynthInvokeWorkerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticInvokeEndingOrderSceSynthInvokeWorkerState, HistoryId>> =
            listOf(StateTarget(StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StaticInvokeEndingOrderSceSynthInvokeWorkerState, HistoryId>(
            StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<StaticInvokeEndingOrderSceSynthInvokeWorkerState, HistoryId>(
            StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticInvokeEndingOrderSceSynthInvokeWorkerState? = when (stateId) {
        "waiting" -> StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticInvokeEndingOrderSceSynthInvokeWorkerState): String = when (state) {
        is StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting -> "waiting"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticInvokeEndingOrderSceSynthInvokeWorkerState): Int = when (state) {
        is StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticInvokeEndingOrderSceSynthInvokeWorkerEvent? = when (name) {
        "answer" -> StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Answer
        "done.invoke" -> StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Done.Invoke.Self
        "done.invoke.clock" -> StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Done.Invoke.Clock
        "error.communication" -> StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Error.Communication
        "error.execution" -> StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Error.Execution
        "question" -> StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Question
        "stamped" -> StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Stamped
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticInvokeEndingOrderSceSynthInvokeWorkerEvent): String? = when (event) {
        is StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Answer -> "answer"
        is StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Done.Invoke.Self -> "done.invoke"
        is StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Done.Invoke.Clock -> "done.invoke.clock"
        is StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Error.Communication -> "error.communication"
        is StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Error.Execution -> "error.execution"
        is StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Question -> "question"
        is StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Stamped -> "stamped"
    }


    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "clock",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticInvokeEndingOrderSceSynthInvokeWorkerState, event: StaticInvokeEndingOrderSceSynthInvokeWorkerEvent?): EnabledTransition<StaticInvokeEndingOrderSceSynthInvokeWorkerState, HistoryId>? = when (state) {
        is StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting -> firstEnabledTransitionOfWaiting(state, event)
    }


    // waiting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWaiting(state: StaticInvokeEndingOrderSceSynthInvokeWorkerState, event: StaticInvokeEndingOrderSceSynthInvokeWorkerEvent?): EnabledTransition<StaticInvokeEndingOrderSceSynthInvokeWorkerState, HistoryId>? = when {
        event is StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Done.Invoke.Clock -> TransitionsPart0.transitionWaitingAt0
        event is StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Question -> TransitionsPart0.transitionWaitingAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_invoke_ending_order.scxml:45 :: _machine
    override fun onEntry(state: StaticInvokeEndingOrderSceSynthInvokeWorkerState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting -> onEntryOfWaiting()
        }
    }


    private fun onEntryOfWaiting() {
                // SCE-MAP: static_invoke_ending_order.scxml:48 :: waiting :: _state_body
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "waiting.${System.identityHashCode(this)}.clock"
                    deferInvoke(StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-clock-now",
                                invokeId = "clock",
                                src = "",
                                params = hostInvokeParams,
                                eventData = if (hostInvokePayload.isEmpty()) "" else buildJsonFromParams(hostInvokePayload),
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_invoke_ending_order.scxml:45 :: _machine
    override fun onExit(state: StaticInvokeEndingOrderSceSynthInvokeWorkerState) {
        when (state) {
            is StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting -> onExitOfWaiting()
        }
    }


    private fun onExitOfWaiting() {
                // SCE-MAP: static_invoke_ending_order.scxml:48 :: waiting :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-clock-now", "clock")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_invoke_ending_order.scxml:45 :: _machine
    override fun executeTransitionContent(source: StaticInvokeEndingOrderSceSynthInvokeWorkerState, transitionIndex: Int) {
        when (source) {
            is StaticInvokeEndingOrderSceSynthInvokeWorkerState.Waiting -> transitionContentOfWaiting(transitionIndex)
        }
    }


    // The content of `waiting`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWaiting(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWaitingAt0()
            1 -> transitionContentOfWaitingAt1()
            else -> {}
        }
    }


    private fun transitionContentOfWaitingAt0() {
                // SCE-MAP: static_invoke_ending_order.scxml:50 :: waiting :: _transition_0


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_0")
                return@send true
            }
            toParent("stamped", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfWaitingAt1() {
                // SCE-MAP: static_invoke_ending_order.scxml:53 :: waiting :: _transition_1


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(StaticInvokeEndingOrderSceSynthInvokeWorkerEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_1")
                return@send true
            }
            toParent("answer", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }
}
