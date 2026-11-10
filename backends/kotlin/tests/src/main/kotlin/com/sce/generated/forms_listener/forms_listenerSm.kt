// SCE-GENERATED — DO NOT EDIT
// source-hash: 72891728be38ed1db13e4394197bad5a7eaecc0eeac062f21e8fdc061c4ae890

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/test_profile/trace_forms/forms_listener.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: forms_listener.scxml:4 :: _machine

package com.sce.generated.forms_listener

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface FormsListenerState : State {
    data object Done : FormsListenerState
    data object First : FormsListenerState
    data object Second : FormsListenerState
    data object Third : FormsListenerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface FormsListenerEvent : Event {
    sealed interface Sce : FormsListenerEvent {
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
 * The test component `forms_listener` as the profile's runtime drives it
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
class FormsListenerComponent : com.sce.runtime.test.ComponentMachine {
    private var sceMachine: FormsListenerStateMachine? = null

    override fun start(context: com.sce.runtime.test.ComponentContext) {
        // The session is named by the component, and named rather than drawn.
        // The name is `mtc` or the id of the `<invoke>` that creates the PTC
        // (`ComponentContext.create`, which creates nothing under a name its
        // configuration has no PTC of), and every `<invoke id>` is refused
        // where its document is parsed unless it is an XML Name, which is an
        // Nmtoken (docs/adr/0045). Every `sce-static` root takes the session id
        // its host gives, whether or not it saves (docs/adr/0045, answer 2), so
        // the component's machine is named by the component.
        val machine = FormsListenerStateMachine(sceTestContext = context, sessionId = context.name)
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
        machine.send(FormsListenerEvent.Sce.Test.Snapshot)
        machine.tick()
    }

    // P10: the machine entered a top-level `<final>`.
    override fun terminated(): Boolean = sceMachine?.isInFinalState == true
}

class FormsListenerStateMachine(
    /**
     * docs/SCE_TEST_PROFILE.md §12.3: the runtime context of the test
     * component this machine is, which its branches' guards read the round's
     * snapshot through and its statements call. Given by the component the
     * runtime drives (FormsListenerComponent) when it builds the machine,
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
) : StateMachineEngine<FormsListenerState, FormsListenerEvent>() {

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
        first = "59c9831d8d2a9cf650a27eccf32d5fd593c8fa7ce020f4fff1096b0515ce6fbf",
        revised = "dbedc96bb26a5f09be4289160792d5b127795219afad8ac2bea9493d09347c0a",
        format2First = "59c9831d8d2a9cf650a27eccf32d5fd593c8fa7ce020f4fff1096b0515ce6fbf",
        format2Revised = "dbedc96bb26a5f09be4289160792d5b127795219afad8ac2bea9493d09347c0a",
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

    override val initialState: FormsListenerState = FormsListenerState.First

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
    override fun isFinalState(state: FormsListenerState): Boolean = when (state) {
        is FormsListenerState.Done -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<FormsListenerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<FormsListenerState, HistoryId>> =
            listOf(StateTarget(FormsListenerState.First))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: first's transition 0, as the microstep reads it.
        val transitionFirstAt0 = EnabledTransition<FormsListenerState, HistoryId>(
            FormsListenerState.First,
            listOf(StateTarget(FormsListenerState.Second)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: second's transition 0, as the microstep reads it.
        val transitionSecondAt0 = EnabledTransition<FormsListenerState, HistoryId>(
            FormsListenerState.Second,
            listOf(StateTarget(FormsListenerState.Third)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: third's transition 0, as the microstep reads it.
        val transitionThirdAt0 = EnabledTransition<FormsListenerState, HistoryId>(
            FormsListenerState.Third,
            listOf(StateTarget(FormsListenerState.Done)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): FormsListenerState? = when (stateId) {
        "done" -> FormsListenerState.Done
        "first" -> FormsListenerState.First
        "second" -> FormsListenerState.Second
        "third" -> FormsListenerState.Third
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: FormsListenerState): String = when (state) {
        is FormsListenerState.Done -> "done"
        is FormsListenerState.First -> "first"
        is FormsListenerState.Second -> "second"
        is FormsListenerState.Third -> "third"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: FormsListenerState): Int = when (state) {
        is FormsListenerState.Done -> 3
        is FormsListenerState.First -> 0
        is FormsListenerState.Second -> 1
        is FormsListenerState.Third -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): FormsListenerEvent? = when (name) {
        "sce.test.snapshot" -> FormsListenerEvent.Sce.Test.Snapshot
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: FormsListenerEvent): String? = when (event) {
        is FormsListenerEvent.Sce.Test.Snapshot -> "sce.test.snapshot"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: FormsListenerState, event: FormsListenerEvent?): EnabledTransition<FormsListenerState, HistoryId>? = when (state) {
        is FormsListenerState.First -> firstEnabledTransitionOfFirst(state, event)
        is FormsListenerState.Second -> firstEnabledTransitionOfSecond(state, event)
        is FormsListenerState.Third -> firstEnabledTransitionOfThird(state, event)
        else -> null
    }


    // first's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfFirst(state: FormsListenerState, event: FormsListenerEvent?): EnabledTransition<FormsListenerState, HistoryId>? = when {
        event is FormsListenerEvent.Sce.Test.Snapshot && sceTestContext.snapshotPresent("sut") -> TransitionsPart0.transitionFirstAt0
        else -> null
    }


    // second's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSecond(state: FormsListenerState, event: FormsListenerEvent?): EnabledTransition<FormsListenerState, HistoryId>? = when {
        event is FormsListenerEvent.Sce.Test.Snapshot && sceTestContext.snapshotPresent("sut") -> TransitionsPart0.transitionSecondAt0
        else -> null
    }


    // third's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfThird(state: FormsListenerState, event: FormsListenerEvent?): EnabledTransition<FormsListenerState, HistoryId>? = when {
        event is FormsListenerEvent.Sce.Test.Snapshot && sceTestContext.snapshotPresent("sut") -> TransitionsPart0.transitionThirdAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: forms_listener.scxml:4 :: _machine
    override fun onEntry(state: FormsListenerState, isDefaultEntry: Boolean) {
        when (state) {
            is FormsListenerState.Done -> onEntryOfDone()
            is FormsListenerState.First -> onEntryOfFirst()
            is FormsListenerState.Second -> onEntryOfSecond()
            is FormsListenerState.Third -> onEntryOfThird()
        }
    }


    private fun onEntryOfDone() {
                // SCE-MAP: forms_listener.scxml:21 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFirst() {
                // SCE-MAP: forms_listener.scxml:10 :: first :: _state_body
    }

    private fun onEntryOfSecond() {
                // SCE-MAP: forms_listener.scxml:13 :: second :: _state_body
    }

    private fun onEntryOfThird() {
                // SCE-MAP: forms_listener.scxml:16 :: third :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: forms_listener.scxml:4 :: _machine
    override fun onExit(state: FormsListenerState) {
        when (state) {
            is FormsListenerState.Done -> onExitOfDone()
            is FormsListenerState.First -> onExitOfFirst()
            is FormsListenerState.Second -> onExitOfSecond()
            is FormsListenerState.Third -> onExitOfThird()
        }
    }


    private fun onExitOfDone() {
                // SCE-MAP: forms_listener.scxml:21 :: done :: _state_body
    }

    private fun onExitOfFirst() {
                // SCE-MAP: forms_listener.scxml:10 :: first :: _state_body
    }

    private fun onExitOfSecond() {
                // SCE-MAP: forms_listener.scxml:13 :: second :: _state_body
    }

    private fun onExitOfThird() {
                // SCE-MAP: forms_listener.scxml:16 :: third :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: forms_listener.scxml:4 :: _machine
    override fun executeTransitionContent(source: FormsListenerState, transitionIndex: Int) {
        when (source) {
            is FormsListenerState.First -> transitionContentOfFirst(transitionIndex)
            is FormsListenerState.Second -> transitionContentOfSecond(transitionIndex)
            is FormsListenerState.Third -> transitionContentOfThird(transitionIndex)
            else -> {}
        }
    }


    // The content of `first`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfFirst(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfFirstAt0()
            else -> {}
        }
    }


    private fun transitionContentOfFirstAt0() {
                // SCE-MAP: forms_listener.scxml:11 :: first :: _transition_0
                // docs/SCE_TEST_PROFILE.md §12.3: taking the branch (P4 step 1).
                sceTestContext.receive("sut")
    }

    // The content of `second`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfSecond(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfSecondAt0()
            else -> {}
        }
    }


    private fun transitionContentOfSecondAt0() {
                // SCE-MAP: forms_listener.scxml:14 :: second :: _transition_0
                // docs/SCE_TEST_PROFILE.md §12.3: taking the branch (P4 step 1).
                sceTestContext.receive("sut")
    }

    // The content of `third`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfThird(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfThirdAt0()
            else -> {}
        }
    }


    private fun transitionContentOfThirdAt0() {
                // SCE-MAP: forms_listener.scxml:17 :: third :: _transition_0
                // docs/SCE_TEST_PROFILE.md §12.3: taking the branch (P4 step 1).
                sceTestContext.receive("sut")

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.PASS, "")
    }
}
