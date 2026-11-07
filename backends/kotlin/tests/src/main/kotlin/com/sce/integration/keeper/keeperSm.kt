// SCE-GENERATED — DO NOT EDIT
// source-hash: 021fdf0d0c3ee151c664e2ecf6a7e5a797d68aaf3d399831e8dfd49b9e10bec9

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/test_profile/runtime/keeper.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: keeper.scxml:6 :: _machine

package com.sce.integration.keeper

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface KeeperState : State {
    data object Lingering : KeeperState
    data object Passed : KeeperState
    data object Watching : KeeperState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface KeeperEvent : Event {
    sealed interface Done : KeeperEvent {
        data object Invoke : Done
    }
    sealed interface Error : KeeperEvent {
        data object Execution : Error
    }
    sealed interface Sce : KeeperEvent {
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
 * The test component `keeper` as the profile's runtime drives it
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
class KeeperComponent : com.sce.runtime.test.ComponentMachine {
    private var sceMachine: KeeperStateMachine? = null

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
        val machine = KeeperStateMachine(sceTestContext = context, sessionId = context.name)
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
        machine.send(KeeperEvent.Sce.Test.Snapshot)
        machine.tick()
    }

    // P10: the machine entered a top-level `<final>`.
    override fun terminated(): Boolean = sceMachine?.isInFinalState == true
}

class KeeperStateMachine(
    /**
     * docs/SCE_TEST_PROFILE.md §12.3: the runtime context of the test
     * component this machine is, which its branches' guards read the round's
     * snapshot through and its statements call. Given by the component the
     * runtime drives (KeeperComponent) when it builds the machine,
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
) : StateMachineEngine<KeeperState, KeeperEvent>() {

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
        first = "8d2a02d26391f544bb6c2ace8a4dcaa59b0af130b5064a798765d989160222ba",
        revised = "7f0b4227ae6f0d69152dc506c4528497f2c02dacdd39fb625e868a90a61f8ab5",
        format2First = "8d2a02d26391f544bb6c2ace8a4dcaa59b0af130b5064a798765d989160222ba",
        format2Revised = "7f0b4227ae6f0d69152dc506c4528497f2c02dacdd39fb625e868a90a61f8ab5",
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
    override val staticHostInvokes: List<Triple<String, String, KeeperState>> = listOf(
        Triple("http://sce.dev/ext/test-component", "idler", KeeperState.Watching),
    )

    override val initialState: KeeperState = KeeperState.Watching

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
    override fun isFinalState(state: KeeperState): Boolean = when (state) {
        is KeeperState.Passed -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<KeeperState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<KeeperState, HistoryId>> =
            listOf(StateTarget(KeeperState.Watching))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: lingering's transition 0, as the microstep reads it.
        val transitionLingeringAt0 = EnabledTransition<KeeperState, HistoryId>(
            KeeperState.Lingering,
            listOf(StateTarget(KeeperState.Passed)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: watching's transition 0, as the microstep reads it.
        val transitionWatchingAt0 = EnabledTransition<KeeperState, HistoryId>(
            KeeperState.Watching,
            listOf(StateTarget(KeeperState.Lingering)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): KeeperState? = when (stateId) {
        "lingering" -> KeeperState.Lingering
        "passed" -> KeeperState.Passed
        "watching" -> KeeperState.Watching
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: KeeperState): String = when (state) {
        is KeeperState.Lingering -> "lingering"
        is KeeperState.Passed -> "passed"
        is KeeperState.Watching -> "watching"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: KeeperState): Int = when (state) {
        is KeeperState.Lingering -> 1
        is KeeperState.Passed -> 2
        is KeeperState.Watching -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): KeeperEvent? = when (name) {
        "done.invoke" -> KeeperEvent.Done.Invoke
        "error.execution" -> KeeperEvent.Error.Execution
        "sce.test.snapshot" -> KeeperEvent.Sce.Test.Snapshot
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: KeeperEvent): String? = when (event) {
        is KeeperEvent.Done.Invoke -> "done.invoke"
        is KeeperEvent.Error.Execution -> "error.execution"
        is KeeperEvent.Sce.Test.Snapshot -> "sce.test.snapshot"
    }


    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "idler",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: KeeperState, event: KeeperEvent?): EnabledTransition<KeeperState, HistoryId>? = when (state) {
        is KeeperState.Lingering -> firstEnabledTransitionOfLingering(state, event)
        is KeeperState.Watching -> firstEnabledTransitionOfWatching(state, event)
        else -> null
    }


    // lingering's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfLingering(state: KeeperState, event: KeeperEvent?): EnabledTransition<KeeperState, HistoryId>? = when {
        event is KeeperEvent.Sce.Test.Snapshot && sceTestContext.timedOut("linger") -> TransitionsPart0.transitionLingeringAt0
        else -> null
    }


    // watching's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWatching(state: KeeperState, event: KeeperEvent?): EnabledTransition<KeeperState, HistoryId>? = when {
        event is KeeperEvent.Sce.Test.Snapshot && sceTestContext.timedOut("patience") -> TransitionsPart0.transitionWatchingAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: keeper.scxml:6 :: _machine
    override fun onEntry(state: KeeperState, isDefaultEntry: Boolean) {
        when (state) {
            is KeeperState.Lingering -> onEntryOfLingering()
            is KeeperState.Passed -> onEntryOfPassed()
            is KeeperState.Watching -> onEntryOfWatching()
        }
    }


    private fun onEntryOfLingering() {
                // SCE-MAP: keeper.scxml:19 :: lingering :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            sceTestContext.startTimer("linger", 5000L)
                }
    }

    private fun onEntryOfPassed() {
                // SCE-MAP: keeper.scxml:27 :: passed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfWatching() {
                // SCE-MAP: keeper.scxml:12 :: watching :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            sceTestContext.startTimer("patience", 10000L)
                }
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
                    val generatedInvokeId = "watching.${System.identityHashCode(this)}.idler"
                    deferInvoke(KeeperState.Watching, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "http://sce.dev/ext/test-component",
                                invokeId = "idler",
                                src = "idler.scxml",
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
                            raisePlatformError(KeeperEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: keeper.scxml:6 :: _machine
    override fun onExit(state: KeeperState) {
        when (state) {
            is KeeperState.Lingering -> onExitOfLingering()
            is KeeperState.Passed -> onExitOfPassed()
            is KeeperState.Watching -> onExitOfWatching()
        }
    }


    private fun onExitOfLingering() {
                // SCE-MAP: keeper.scxml:19 :: lingering :: _state_body
    }

    private fun onExitOfPassed() {
                // SCE-MAP: keeper.scxml:27 :: passed :: _state_body
    }

    private fun onExitOfWatching() {
                // SCE-MAP: keeper.scxml:12 :: watching :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(KeeperState.Watching)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("http://sce.dev/ext/test-component", "idler")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: keeper.scxml:6 :: _machine
    override fun executeTransitionContent(source: KeeperState, transitionIndex: Int) {
        when (source) {
            is KeeperState.Lingering -> transitionContentOfLingering(transitionIndex)
            is KeeperState.Watching -> transitionContentOfWatching(transitionIndex)
            else -> {}
        }
    }


    // The content of `lingering`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfLingering(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfLingeringAt0()
            else -> {}
        }
    }


    private fun transitionContentOfLingeringAt0() {
                // SCE-MAP: keeper.scxml:23 :: lingering :: _transition_0
                // docs/SCE_TEST_PROFILE.md §12.3: taking the branch (P4 step 1).
                sceTestContext.timeout("linger")

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.PASS, "")
    }

    // The content of `watching`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWatching(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWatchingAt0()
            else -> {}
        }
    }


    private fun transitionContentOfWatchingAt0() {
                // SCE-MAP: keeper.scxml:17 :: watching :: _transition_0
                // docs/SCE_TEST_PROFILE.md §12.3: taking the branch (P4 step 1).
                sceTestContext.timeout("patience")
    }
}
