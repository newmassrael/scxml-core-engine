// SCE-GENERATED — DO NOT EDIT
// source-hash: 982a23a5ec15b8216f29e9e8e4ba6c1047dee0d66671c7835c2776a4de5a13d2

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/test_profile/boundary/signed_tester.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: signed_tester.scxml:5 :: _machine

package com.sce.generated.signed_tester

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface SignedTesterState : State {
    data object Exchanging : SignedTesterState
    data object Failed : SignedTesterState
    data object Passed : SignedTesterState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface SignedTesterEvent : Event {
    sealed interface Error : SignedTesterEvent {
        data object Execution : Error
    }
    sealed interface Sce : SignedTesterEvent {
        sealed interface Test : Sce {
            data object Snapshot : Test
        }
    }
    sealed interface Signed : SignedTesterEvent {
        data object Reading : Signed
    }
}
// --- State Machine (W3C SCXML) ---

// ======================================================================
// The test component the profile's runtime drives (docs/SCE_TEST_PROFILE.md §12.3)
// ======================================================================

/**
 * The test component `signed_tester` as the profile's runtime drives it
 * (docs/SCE_TEST_PROFILE.md §12.3).
 *
 * It owns the machine below, unchanged, and reaches it only through the doors
 * the engine already has: the machine's sends on a port leave through the
 * test-port processor and its PTCs through the test-component invoker, both
 * registered here over the component's runtime context (docs/adr/0084), and
 * each round's snapshot is the one external event every branch is a
 * transition on. The runtime never reports a PTC's completion to the engine:
 * a done branch reads it from the snapshot, so the engine cancels every PTC it
 * started when the invoking state is exited, which is P9's kill. One thread;
 * the engine's scheduler takes no part, since a component sends nothing
 * delayed.
 *
 * The machine is built when the component starts, constructed with the
 * context, and there is none before.
 */
class SignedTesterComponent : com.sce.runtime.test.ComponentMachine {
    private var sceMachine: SignedTesterStateMachine? = null

    override fun start(context: com.sce.runtime.test.ComponentContext) {
        // The session is named by the component, and named rather than drawn.
        // The name is `mtc` or the id of the `<invoke>` that creates the PTC
        // (`ComponentContext.create`, which creates nothing under a name its
        // configuration has no PTC of), and every `<invoke id>` is refused
        // where its document is parsed unless it is an XML Name, which is an
        // Nmtoken (docs/adr/0045). Every `sce-static` root takes the session id
        // its host gives, whether or not it saves (docs/adr/0045, answer 2), so
        // the component's machine is named by the component.
        val machine = SignedTesterStateMachine(sceTestContext = context, sessionId = context.name)
        // P6: a send on a port is appended to the queue of the port it is
        // connected to, or handed to the system adapter where it is mapped. A
        // test-port send always carries its template's value
        // (`forge::static_lowering`, `StaticTarget::test_message`).
        machine.registerEventProcessor("http://sce.dev/ext/test-port") { request ->
            context.send(
                request.target,
                com.sce.runtime.test.Message(
                    request.eventName,
                    checkNotNull(request.value) { "a test-port send carries its template's value" },
                ),
            )
            emptyList()
        }
        // P9: the start of a PTC's invocation creates and starts it, and its
        // cancel removes it, whether it is still running or already done.
        machine.registerInvoker("http://sce.dev/ext/test-component") { event ->
            event.start?.let { context.create(it.invokeId) }
            event.cancel?.let { context.kill(it.invokeId) }
            null
        }
        machine.initialize()
        sceMachine = machine
    }

    // §3.2 step 3: the round's snapshot, on which the machine takes the first
    // branch whose guard holds (P3) and completes its macrostep (P4), or takes
    // none and discards the event (§7). The owner's tick runs the macrostep
    // the event starts.
    override fun deliverSnapshot() {
        val machine = sceMachine ?: return
        machine.send(SignedTesterEvent.Sce.Test.Snapshot)
        machine.tick()
    }

    // P10: the machine entered a top-level `<final>`.
    override fun terminated(): Boolean = sceMachine?.isInFinalState == true
}

class SignedTesterStateMachine(
    /**
     * docs/SCE_TEST_PROFILE.md §12.3: the runtime context of the test
     * component this machine is, which its branches' guards read the round's
     * snapshot through and its statements call. Given by the component the
     * runtime drives (SignedTesterComponent) when it builds the machine,
     * since the initial state's `<onentry>` may already start a timer or set a
     * verdict.
     */
    private val sceTestContext: com.sce.runtime.test.ComponentContext,
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<SignedTesterState, SignedTesterEvent>() {

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
        first = "bd8f188e8d5cbf95df762ac31a3dad4ae33d0a2af5267b3e06e20a36ed1aff60",
        revised = "776b0f69ff025faae7aed0679dd19a7c488f9013275c4fb0e5a50014a3b2afdd",
        format2First = "bd8f188e8d5cbf95df762ac31a3dad4ae33d0a2af5267b3e06e20a36ed1aff60",
        format2Revised = "776b0f69ff025faae7aed0679dd19a7c488f9013275c4fb0e5a50014a3b2afdd",
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

    override val initialState: SignedTesterState = SignedTesterState.Exchanging

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
    override fun isFinalState(state: SignedTesterState): Boolean = when (state) {
        is SignedTesterState.Failed -> true
        is SignedTesterState.Passed -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<SignedTesterState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<SignedTesterState, HistoryId>> =
            listOf(StateTarget(SignedTesterState.Exchanging))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: exchanging's transition 0, as the microstep reads it.
        val transitionExchangingAt0 = EnabledTransition<SignedTesterState, HistoryId>(
            SignedTesterState.Exchanging,
            listOf(StateTarget(SignedTesterState.Passed)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: exchanging's transition 1, as the microstep reads it.
        val transitionExchangingAt1 = EnabledTransition<SignedTesterState, HistoryId>(
            SignedTesterState.Exchanging,
            listOf(StateTarget(SignedTesterState.Failed)),
            1,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): SignedTesterState? = when (stateId) {
        "exchanging" -> SignedTesterState.Exchanging
        "failed" -> SignedTesterState.Failed
        "passed" -> SignedTesterState.Passed
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: SignedTesterState): String = when (state) {
        is SignedTesterState.Exchanging -> "exchanging"
        is SignedTesterState.Failed -> "failed"
        is SignedTesterState.Passed -> "passed"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: SignedTesterState): Int = when (state) {
        is SignedTesterState.Exchanging -> 0
        is SignedTesterState.Failed -> 2
        is SignedTesterState.Passed -> 1
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): SignedTesterEvent? = when (name) {
        "error.execution" -> SignedTesterEvent.Error.Execution
        "sce.test.snapshot" -> SignedTesterEvent.Sce.Test.Snapshot
        "signed.reading" -> SignedTesterEvent.Signed.Reading
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: SignedTesterEvent): String? = when (event) {
        is SignedTesterEvent.Error.Execution -> "error.execution"
        is SignedTesterEvent.Sce.Test.Snapshot -> "sce.test.snapshot"
        is SignedTesterEvent.Signed.Reading -> "signed.reading"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: SignedTesterState, event: SignedTesterEvent?): EnabledTransition<SignedTesterState, HistoryId>? = when (state) {
        is SignedTesterState.Exchanging -> firstEnabledTransitionOfExchanging(state, event)
        else -> null
    }


    // exchanging's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfExchanging(state: SignedTesterState, event: SignedTesterEvent?): EnabledTransition<SignedTesterState, HistoryId>? = when {
        event is SignedTesterEvent.Sce.Test.Snapshot && com.sce.runtime.test.receives<com.sce.generated.signed_reading_within.Value>(sceTestContext, "sut", "signed.reading", { sceMessage -> com.sce.generated.signed_reading_within.matches(sceMessage) }) -> TransitionsPart0.transitionExchangingAt0
        event is SignedTesterEvent.Sce.Test.Snapshot && sceTestContext.timedOut("deadline") -> TransitionsPart0.transitionExchangingAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: signed_tester.scxml:5 :: _machine
    override fun onEntry(state: SignedTesterState, isDefaultEntry: Boolean) {
        when (state) {
            is SignedTesterState.Exchanging -> onEntryOfExchanging()
            is SignedTesterState.Failed -> onEntryOfFailed()
            is SignedTesterState.Passed -> onEntryOfPassed()
        }
    }


    private fun onEntryOfExchanging() {
                // SCE-MAP: signed_tester.scxml:14 :: exchanging :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 6.2.5: "http://sce.dev/ext/test-port" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "http://sce.dev/ext/test-port",
                eventName = "signed.reading",
                target = "sut",
                content = "",
                params = sendWireParams,
                sendId = "__send_0",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId,
                // docs/SCE_TEST_PROFILE.md §12.3: a test component's message is
                // the value of its specific template, handed to the test-port
                // processor as a value and never as an encoding (docs/adr/0091,
                // C2), with the writer of its canonical form in the test log
                // (§12.5). Every other send leaves it null and carries `eventData`.
                value = com.sce.runtime.test.messageValue(com.sce.generated.signed_reading_sent.value(), com.sce.generated.signed_reading_sent.TYPE) { sceSent -> com.sce.generated.signed_reading_sent.canonical(sceSent) }
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("http://sce.dev/ext/test-port")) {
                raisePlatformError(SignedTesterEvent.Error.Execution, "<send type='http://sce.dev/ext/test-port'> names a processor the host declared but never registered", "__send_0")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)

            sceTestContext.startTimer("deadline", 1000000L)
                }
    }

    private fun onEntryOfFailed() {
                // SCE-MAP: signed_tester.scxml:28 :: failed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPassed() {
                // SCE-MAP: signed_tester.scxml:27 :: passed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: signed_tester.scxml:5 :: _machine
    override fun onExit(state: SignedTesterState) {
        when (state) {
            is SignedTesterState.Exchanging -> onExitOfExchanging()
            is SignedTesterState.Failed -> onExitOfFailed()
            is SignedTesterState.Passed -> onExitOfPassed()
        }
    }


    private fun onExitOfExchanging() {
                // SCE-MAP: signed_tester.scxml:14 :: exchanging :: _state_body
    }

    private fun onExitOfFailed() {
                // SCE-MAP: signed_tester.scxml:28 :: failed :: _state_body
    }

    private fun onExitOfPassed() {
                // SCE-MAP: signed_tester.scxml:27 :: passed :: _state_body
    }



    // docs/SCE_TEST_PROFILE.md §4.4 (P4 step 1), §12.3: a branch of the test
    // component is taken before the microstep exits any state: it removes what
    // it matched from the round's snapshot, writes its record in the test log,
    // and holds a message its content reads in the payload member of its
    // record's event (`Transition::native_take`).
    override fun takeTransition(source: SignedTesterState, transitionIndex: Int) {
        when {
            source is SignedTesterState.Exchanging && transitionIndex == 0 -> {
                sceTestContext.receive("sut")
            }
            source is SignedTesterState.Exchanging && transitionIndex == 1 -> {
                sceTestContext.timeout("deadline")
            }
        }
    }

    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: signed_tester.scxml:5 :: _machine
    override fun executeTransitionContent(source: SignedTesterState, transitionIndex: Int) {
        when (source) {
            is SignedTesterState.Exchanging -> transitionContentOfExchanging(transitionIndex)
            else -> {}
        }
    }


    // The content of `exchanging`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfExchanging(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfExchangingAt0()
            1 -> transitionContentOfExchangingAt1()
            else -> {}
        }
    }


    private fun transitionContentOfExchangingAt0() {
                // SCE-MAP: signed_tester.scxml:19 :: exchanging :: _transition_0

            sceTestContext.stopTimer("deadline")

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.PASS, "")
    }

    private fun transitionContentOfExchangingAt1() {
                // SCE-MAP: signed_tester.scxml:23 :: exchanging :: _transition_1

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.FAIL, "no reading in range in time")
    }
}
