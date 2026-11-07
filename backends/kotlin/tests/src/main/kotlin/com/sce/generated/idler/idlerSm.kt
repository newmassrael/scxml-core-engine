// SCE-GENERATED — DO NOT EDIT
// source-hash: 021fdf0d0c3ee151c664e2ecf6a7e5a797d68aaf3d399831e8dfd49b9e10bec9

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/test_profile/runtime/idler.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: idler.scxml:5 :: _machine

package com.sce.generated.idler

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface IdlerState : State {
    data object Done : IdlerState
    data object Idling : IdlerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface IdlerEvent : Event {
    sealed interface Sce : IdlerEvent {
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
 * The test component `idler` as the profile's runtime drives it
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
class IdlerComponent : com.sce.runtime.test.ComponentMachine {
    private var sceMachine: IdlerStateMachine? = null

    override fun start(context: com.sce.runtime.test.ComponentContext) {
        // The session is named by the component, and named rather than drawn.
        // The name is `mtc` or the id of the `<invoke>` that creates the PTC
        // (`ComponentContext.create`, which creates nothing under a name its
        // configuration has no PTC of), and every `<invoke id>` is refused
        // where its document is parsed unless it is an XML Name, which is an
        // Nmtoken (docs/adr/0045). A component's machine always has a saved
        // shape, and so takes its session id: what would leave it none, a
        // delayed send or an invoke a restore cannot start, the profile
        // refuses.
        val machine = IdlerStateMachine(sceTestContext = context, sessionId = context.name)
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
        machine.send(IdlerEvent.Sce.Test.Snapshot)
        machine.tick()
    }

    // P10: the machine entered a top-level `<final>`.
    override fun terminated(): Boolean = sceMachine?.isInFinalState == true
}

class IdlerStateMachine(
    /**
     * docs/SCE_TEST_PROFILE.md §12.3: the runtime context of the test
     * component this machine is, which its branches' guards read the round's
     * snapshot through and its statements call. Given by the component the
     * runtime drives (IdlerComponent) when it builds the machine,
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
) : StateMachineEngine<IdlerState, IdlerEvent>() {

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
        first = "8456cd61b63b96657d0199f004d995f0a31a5fa47f09a7f1e55ab7e3c72e4897",
        revised = "950beca9ef63c7da1184bd0081014439f458da6add048ba13e3c29eac6f1f523",
        format2First = "8456cd61b63b96657d0199f004d995f0a31a5fa47f09a7f1e55ab7e3c72e4897",
        format2Revised = "950beca9ef63c7da1184bd0081014439f458da6add048ba13e3c29eac6f1f523",
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

    override val initialState: IdlerState = IdlerState.Idling

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
    override fun isFinalState(state: IdlerState): Boolean = when (state) {
        is IdlerState.Done -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<IdlerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<IdlerState, HistoryId>> =
            listOf(StateTarget(IdlerState.Idling))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: idling's transition 0, as the microstep reads it.
        val transitionIdlingAt0 = EnabledTransition<IdlerState, HistoryId>(
            IdlerState.Idling,
            listOf(StateTarget(IdlerState.Done)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): IdlerState? = when (stateId) {
        "done" -> IdlerState.Done
        "idling" -> IdlerState.Idling
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: IdlerState): String = when (state) {
        is IdlerState.Done -> "done"
        is IdlerState.Idling -> "idling"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: IdlerState): Int = when (state) {
        is IdlerState.Done -> 1
        is IdlerState.Idling -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): IdlerEvent? = when (name) {
        "sce.test.snapshot" -> IdlerEvent.Sce.Test.Snapshot
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: IdlerEvent): String? = when (event) {
        is IdlerEvent.Sce.Test.Snapshot -> "sce.test.snapshot"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: IdlerState, event: IdlerEvent?): EnabledTransition<IdlerState, HistoryId>? = when (state) {
        is IdlerState.Idling -> firstEnabledTransitionOfIdling(state, event)
        else -> null
    }


    // idling's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdling(state: IdlerState, event: IdlerEvent?): EnabledTransition<IdlerState, HistoryId>? = when {
        event is IdlerEvent.Sce.Test.Snapshot && sceTestContext.snapshotPresent("peer") -> TransitionsPart0.transitionIdlingAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: idler.scxml:5 :: _machine
    override fun onEntry(state: IdlerState, isDefaultEntry: Boolean) {
        when (state) {
            is IdlerState.Done -> onEntryOfDone()
            is IdlerState.Idling -> onEntryOfIdling()
        }
    }


    private fun onEntryOfDone() {
                // SCE-MAP: idler.scxml:17 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfIdling() {
                // SCE-MAP: idler.scxml:11 :: idling :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.INCONCLUSIVE, "")
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: idler.scxml:5 :: _machine
    override fun onExit(state: IdlerState) {
        when (state) {
            is IdlerState.Done -> onExitOfDone()
            is IdlerState.Idling -> onExitOfIdling()
        }
    }


    private fun onExitOfDone() {
                // SCE-MAP: idler.scxml:17 :: done :: _state_body
    }

    private fun onExitOfIdling() {
                // SCE-MAP: idler.scxml:11 :: idling :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: idler.scxml:5 :: _machine
    override fun executeTransitionContent(source: IdlerState, transitionIndex: Int) {
        when (source) {
            is IdlerState.Idling -> transitionContentOfIdling(transitionIndex)
            else -> {}
        }
    }


    // The content of `idling`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfIdling(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfIdlingAt0()
            else -> {}
        }
    }


    private fun transitionContentOfIdlingAt0() {
                // SCE-MAP: idler.scxml:15 :: idling :: _transition_0
                // docs/SCE_TEST_PROFILE.md §12.3: taking the branch (P4 step 1).
                sceTestContext.receive("peer")
    }
}
