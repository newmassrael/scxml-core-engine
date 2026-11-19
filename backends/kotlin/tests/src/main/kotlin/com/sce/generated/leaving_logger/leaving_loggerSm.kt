// SCE-GENERATED — DO NOT EDIT
// source-hash: 7dafe1d5fad80f4f556fdd9d5dbf037d9f08d80b2548680f42dd939faeec728c
// source: leaving_logger.scxml

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/test_profile/boundary/leaving_logger.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: leaving_logger.scxml:7 :: _machine

package com.sce.generated.leaving_logger

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface LeavingLoggerState : State {
    data object Done : LeavingLoggerState
    data object Listening : LeavingLoggerState
    data object Waiting : LeavingLoggerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface LeavingLoggerEvent : Event {
    sealed interface Sce : LeavingLoggerEvent {
        sealed interface Test : Sce {
            data object Snapshot : Test
        }
    }
}
// --- State Machine (W3C SCXML) ---

// ======================================================================
// The test component the profile's runtime drives (docs/SCE_TEST_PROFILE.md §12.3)
// ======================================================================

/**
 * The test component `leaving_logger` as the profile's runtime drives it
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
class LeavingLoggerComponent : com.sce.runtime.test.ComponentMachine {
    private var sceMachine: LeavingLoggerStateMachine? = null

    override fun start(context: com.sce.runtime.test.ComponentContext) {
        // The session is named by the component, and named rather than drawn.
        // The name is `mtc` or the id of the `<invoke>` that creates the PTC
        // (`ComponentContext.create`, which creates nothing under a name its
        // configuration has no PTC of), and every `<invoke id>` is refused
        // where its document is parsed unless it is an XML Name, which is an
        // Nmtoken (docs/adr/0045). Every `sce-static` root takes the session id
        // its host gives, whether or not it saves (docs/adr/0045, answer 2), so
        // the component's machine is named by the component.
        val machine = LeavingLoggerStateMachine(sceTestContext = context, sessionId = context.name)
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
        machine.send(LeavingLoggerEvent.Sce.Test.Snapshot)
        machine.tick()
    }

    // P10: the machine entered a top-level `<final>`.
    override fun terminated(): Boolean = sceMachine?.isInFinalState == true
}

class LeavingLoggerStateMachine(
    /**
     * docs/SCE_TEST_PROFILE.md §12.3: the runtime context of the test
     * component this machine is, which its branches' guards read the round's
     * snapshot through and its statements call. Given by the component the
     * runtime drives (LeavingLoggerComponent) when it builds the machine,
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
) : StateMachineEngine<LeavingLoggerState, LeavingLoggerEvent>() {

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
        first = "0c06ad9154ec15cc272aaa26e5421d5ba0cd41575ae3d35d08bc0f0adfa0fb1c",
        revised = "4d768b205f0e4cfca8e33c445305723615c891fc68d69976e5552899abb8d72f",
        format2First = "0c06ad9154ec15cc272aaa26e5421d5ba0cd41575ae3d35d08bc0f0adfa0fb1c",
        format2Revised = "4d768b205f0e4cfca8e33c445305723615c891fc68d69976e5552899abb8d72f",
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
        // Every part is judged before any is written, so a refused value leaves
        // the machine as it was; then each part reads its values again and
        // writes them. A part answers no closure over the values it read: a
        // closure's captures are the arguments of a JVM method, at most 255
        // slots with a `Long` or a `Double` taking two, and a part of
        // `kotlin_method_items` variables passed that, so the class failed to
        // load (`ClassFormatError: Too many arguments in method signature`).
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

    override val initialState: LeavingLoggerState = LeavingLoggerState.Listening

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
    override fun isFinalState(state: LeavingLoggerState): Boolean = when (state) {
        is LeavingLoggerState.Done -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<LeavingLoggerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<LeavingLoggerState, HistoryId>> =
            listOf(StateTarget(LeavingLoggerState.Listening))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: listening's transition 0, as the microstep reads it.
        val transitionListeningAt0 = EnabledTransition<LeavingLoggerState, HistoryId>(
            LeavingLoggerState.Listening,
            listOf(StateTarget(LeavingLoggerState.Waiting)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<LeavingLoggerState, HistoryId>(
            LeavingLoggerState.Waiting,
            listOf(StateTarget(LeavingLoggerState.Done)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): LeavingLoggerState? = when (stateId) {
        "done" -> LeavingLoggerState.Done
        "listening" -> LeavingLoggerState.Listening
        "waiting" -> LeavingLoggerState.Waiting
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: LeavingLoggerState): String = when (state) {
        is LeavingLoggerState.Done -> "done"
        is LeavingLoggerState.Listening -> "listening"
        is LeavingLoggerState.Waiting -> "waiting"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: LeavingLoggerState): Int = when (state) {
        is LeavingLoggerState.Done -> 2
        is LeavingLoggerState.Listening -> 0
        is LeavingLoggerState.Waiting -> 1
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): LeavingLoggerEvent? = when (name) {
        "sce.test.snapshot" -> LeavingLoggerEvent.Sce.Test.Snapshot
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: LeavingLoggerEvent): String? = when (event) {
        is LeavingLoggerEvent.Sce.Test.Snapshot -> "sce.test.snapshot"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: LeavingLoggerState, event: LeavingLoggerEvent?): EnabledTransition<LeavingLoggerState, HistoryId>? = when (state) {
        is LeavingLoggerState.Listening -> firstEnabledTransitionOfListening(state, event)
        is LeavingLoggerState.Waiting -> firstEnabledTransitionOfWaiting(state, event)
        else -> null
    }


    // listening's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfListening(state: LeavingLoggerState, event: LeavingLoggerEvent?): EnabledTransition<LeavingLoggerState, HistoryId>? = when {
        event is LeavingLoggerEvent.Sce.Test.Snapshot && sceTestContext.snapshotPresent("sut") -> TransitionsPart0.transitionListeningAt0
        else -> null
    }


    // waiting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWaiting(state: LeavingLoggerState, event: LeavingLoggerEvent?): EnabledTransition<LeavingLoggerState, HistoryId>? = when {
        event is LeavingLoggerEvent.Sce.Test.Snapshot && sceTestContext.timedOut("patience") -> TransitionsPart0.transitionWaitingAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: leaving_logger.scxml:7 :: _machine
    override fun onEntry(state: LeavingLoggerState, isDefaultEntry: Boolean) {
        when (state) {
            is LeavingLoggerState.Done -> onEntryOfDone()
            is LeavingLoggerState.Listening -> onEntryOfListening()
            is LeavingLoggerState.Waiting -> onEntryOfWaiting()
        }
    }


    private fun onEntryOfDone() {
                // SCE-MAP: leaving_logger.scxml:33 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfListening() {
                // SCE-MAP: leaving_logger.scxml:14 :: listening :: _state_body
    }

    private fun onEntryOfWaiting() {
                // SCE-MAP: leaving_logger.scxml:22 :: waiting :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            sceTestContext.startTimer("patience", 1000L)
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: leaving_logger.scxml:7 :: _machine
    override fun onExit(state: LeavingLoggerState) {
        when (state) {
            is LeavingLoggerState.Done -> onExitOfDone()
            is LeavingLoggerState.Listening -> onExitOfListening()
            is LeavingLoggerState.Waiting -> onExitOfWaiting()
        }
    }


    private fun onExitOfDone() {
                // SCE-MAP: leaving_logger.scxml:33 :: done :: _state_body
    }

    private fun onExitOfListening() {
                // SCE-MAP: leaving_logger.scxml:14 :: listening :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {

            sceTestContext.log("leaving listening")
                }
    }

    private fun onExitOfWaiting() {
                // SCE-MAP: leaving_logger.scxml:22 :: waiting :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {

            sceTestContext.log("leaving waiting")
                }
    }



    // docs/SCE_TEST_PROFILE.md §4.4 (P4 step 1), §12.3: a branch of the test
    // component is taken before the microstep exits any state: it removes what
    // it matched from the round's snapshot, writes its record in the test log,
    // and holds a message its content reads in the payload member of its
    // record's event (`Transition::native_take`).
    override fun takeTransition(source: LeavingLoggerState, transitionIndex: Int) {
        when {
            source is LeavingLoggerState.Listening && transitionIndex == 0 -> {
                sceTestContext.receive("sut")
            }
            source is LeavingLoggerState.Waiting && transitionIndex == 0 -> {
                sceTestContext.timeout("patience")
            }
        }
    }

    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: leaving_logger.scxml:7 :: _machine
    override fun executeTransitionContent(source: LeavingLoggerState, transitionIndex: Int) {
        when (source) {
            is LeavingLoggerState.Listening -> transitionContentOfListening(transitionIndex)
            is LeavingLoggerState.Waiting -> transitionContentOfWaiting(transitionIndex)
            else -> {}
        }
    }


    // The content of `listening`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfListening(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfListeningAt0()
            else -> {}
        }
    }


    private fun transitionContentOfListeningAt0() {
                // SCE-MAP: leaving_logger.scxml:18 :: listening :: _transition_0

            sceTestContext.log("received")
    }

    // The content of `waiting`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWaiting(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWaitingAt0()
            else -> {}
        }
    }


    private fun transitionContentOfWaitingAt0() {
                // SCE-MAP: leaving_logger.scxml:29 :: waiting :: _transition_0

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.PASS, "")
    }
}
