// SCE-GENERATED — DO NOT EDIT
// source-hash: 7dbd769710a01523ac9c83519fe34a7a99a82d0c08e4b4d8edc362d98a80aff4
// source: nested_tester.scxml

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/ttcn3_export/nested_tester.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: nested_tester.scxml:8 :: _machine

package com.sce.generated.nested_tester

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface NestedTesterState : State {
    data object Deep : NestedTesterState
    data object Failed : NestedTesterState
    data object First : NestedTesterState
    data object Outer : NestedTesterState
    data object Passed : NestedTesterState
    data object Second : NestedTesterState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface NestedTesterEvent : Event {
    sealed interface Sce : NestedTesterEvent {
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
 * The test component `nested_tester` as the profile's runtime drives it
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
class NestedTesterComponent : com.sce.runtime.test.ComponentMachine {
    private var sceMachine: NestedTesterStateMachine? = null

    override fun start(context: com.sce.runtime.test.ComponentContext) {
        // The session is named by the component, and named rather than drawn.
        // The name is `mtc` or the id of the `<invoke>` that creates the PTC
        // (`ComponentContext.create`, which creates nothing under a name its
        // configuration has no PTC of), and every `<invoke id>` is refused
        // where its document is parsed unless it is an XML Name, which is an
        // Nmtoken (docs/adr/0045). Every `sce-static` root takes the session id
        // its host gives, whether or not it saves (docs/adr/0045, answer 2), so
        // the component's machine is named by the component.
        val machine = NestedTesterStateMachine(sceTestContext = context, sessionId = context.name)
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
        machine.send(NestedTesterEvent.Sce.Test.Snapshot)
        machine.tick()
    }

    // P10: the machine entered a top-level `<final>`.
    override fun terminated(): Boolean = sceMachine?.isInFinalState == true
}

class NestedTesterStateMachine(
    /**
     * docs/SCE_TEST_PROFILE.md §12.3: the runtime context of the test
     * component this machine is, which its branches' guards read the round's
     * snapshot through and its statements call. Given by the component the
     * runtime drives (NestedTesterComponent) when it builds the machine,
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
) : StateMachineEngine<NestedTesterState, NestedTesterEvent>() {

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
        first = "4e2aa3c02445c4e741e1b40d6c1a36e775fae7b426c0365f223684825dd7e9bd",
        revised = "2829dbf988a6b5a310deb787e2b7e5b88b999175cb22ee6b4356e0473d41257e",
        format2First = "4e2aa3c02445c4e741e1b40d6c1a36e775fae7b426c0365f223684825dd7e9bd",
        format2Revised = "2829dbf988a6b5a310deb787e2b7e5b88b999175cb22ee6b4356e0473d41257e",
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

    override val initialState: NestedTesterState = NestedTesterState.First

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

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: NestedTesterState): NestedTesterState? = when (state) {
        is NestedTesterState.Deep -> NestedTesterState.Second
        is NestedTesterState.First -> NestedTesterState.Outer
        is NestedTesterState.Second -> NestedTesterState.Outer
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: NestedTesterState): Boolean = when (state) {
        is NestedTesterState.Outer -> true
        is NestedTesterState.Second -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: NestedTesterState): Boolean = when (state) {
        is NestedTesterState.Failed -> true
        is NestedTesterState.Passed -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: NestedTesterState): List<NestedTesterState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: NestedTesterState): List<EntryTarget<NestedTesterState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<NestedTesterState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<NestedTesterState, List<NestedTesterState>> = LinkedHashMap<NestedTesterState, List<NestedTesterState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<NestedTesterState, List<NestedTesterState>>) {
            table[NestedTesterState.Outer] = listOf(NestedTesterState.First, NestedTesterState.Second)
            table[NestedTesterState.Second] = listOf(NestedTesterState.Deep)
        }


        val initialTargets: Map<NestedTesterState, List<EntryTarget<NestedTesterState, HistoryId>>> = LinkedHashMap<NestedTesterState, List<EntryTarget<NestedTesterState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<NestedTesterState, List<EntryTarget<NestedTesterState, HistoryId>>>) {
            table[NestedTesterState.Outer] = listOf(StateTarget(NestedTesterState.First))
            table[NestedTesterState.Second] = listOf(StateTarget(NestedTesterState.Deep))
        }


        val documentInitialTargetList: List<EntryTarget<NestedTesterState, HistoryId>> =
            listOf(StateTarget(NestedTesterState.Outer))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: deep's transition 0, as the microstep reads it.
        val transitionDeepAt0 = EnabledTransition<NestedTesterState, HistoryId>(
            NestedTesterState.Deep,
            listOf(StateTarget(NestedTesterState.Passed)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: first's transition 0, as the microstep reads it.
        val transitionFirstAt0 = EnabledTransition<NestedTesterState, HistoryId>(
            NestedTesterState.First,
            listOf(StateTarget(NestedTesterState.Second)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: first's transition 1, as the microstep reads it.
        val transitionFirstAt1 = EnabledTransition<NestedTesterState, HistoryId>(
            NestedTesterState.First,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: outer's transition 0, as the microstep reads it.
        val transitionOuterAt0 = EnabledTransition<NestedTesterState, HistoryId>(
            NestedTesterState.Outer,
            listOf(StateTarget(NestedTesterState.Failed)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: outer's transition 1, as the microstep reads it.
        val transitionOuterAt1 = EnabledTransition<NestedTesterState, HistoryId>(
            NestedTesterState.Outer,
            listOf(StateTarget(NestedTesterState.First)),
            1,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): NestedTesterState? = when (stateId) {
        "deep" -> NestedTesterState.Deep
        "failed" -> NestedTesterState.Failed
        "first" -> NestedTesterState.First
        "outer" -> NestedTesterState.Outer
        "passed" -> NestedTesterState.Passed
        "second" -> NestedTesterState.Second
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: NestedTesterState): String = when (state) {
        is NestedTesterState.Deep -> "deep"
        is NestedTesterState.Failed -> "failed"
        is NestedTesterState.First -> "first"
        is NestedTesterState.Outer -> "outer"
        is NestedTesterState.Passed -> "passed"
        is NestedTesterState.Second -> "second"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: NestedTesterState): Int = when (state) {
        is NestedTesterState.Deep -> 3
        is NestedTesterState.Failed -> 5
        is NestedTesterState.First -> 1
        is NestedTesterState.Outer -> 0
        is NestedTesterState.Passed -> 4
        is NestedTesterState.Second -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): NestedTesterEvent? = when (name) {
        "sce.test.snapshot" -> NestedTesterEvent.Sce.Test.Snapshot
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: NestedTesterEvent): String? = when (event) {
        is NestedTesterEvent.Sce.Test.Snapshot -> "sce.test.snapshot"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: NestedTesterState, event: NestedTesterEvent?): EnabledTransition<NestedTesterState, HistoryId>? = when (state) {
        is NestedTesterState.Deep -> firstEnabledTransitionOfDeep(state, event)
        is NestedTesterState.First -> firstEnabledTransitionOfFirst(state, event)
        is NestedTesterState.Outer -> firstEnabledTransitionOfOuter(state, event)
        else -> null
    }


    // deep's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfDeep(state: NestedTesterState, event: NestedTesterEvent?): EnabledTransition<NestedTesterState, HistoryId>? = when {
        event is NestedTesterEvent.Sce.Test.Snapshot && sceTestContext.timedOut("tick") -> TransitionsPart0.transitionDeepAt0
        else -> null
    }


    // first's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfFirst(state: NestedTesterState, event: NestedTesterEvent?): EnabledTransition<NestedTesterState, HistoryId>? = when {
        event is NestedTesterEvent.Sce.Test.Snapshot && sceTestContext.timedOut("tick") -> TransitionsPart0.transitionFirstAt0
        event is NestedTesterEvent.Sce.Test.Snapshot && sceTestContext.snapshotPresent("sut") -> TransitionsPart0.transitionFirstAt1
        else -> null
    }


    // outer's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfOuter(state: NestedTesterState, event: NestedTesterEvent?): EnabledTransition<NestedTesterState, HistoryId>? = when {
        event is NestedTesterEvent.Sce.Test.Snapshot && sceTestContext.timedOut("limit") -> TransitionsPart0.transitionOuterAt0
        event is NestedTesterEvent.Sce.Test.Snapshot && sceTestContext.snapshotPresent("sut") -> TransitionsPart0.transitionOuterAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: nested_tester.scxml:8 :: _machine
    override fun onEntry(state: NestedTesterState, isDefaultEntry: Boolean) {
        when (state) {
            is NestedTesterState.Deep -> onEntryOfDeep()
            is NestedTesterState.Failed -> onEntryOfFailed()
            is NestedTesterState.First -> onEntryOfFirst()
            is NestedTesterState.Outer -> onEntryOfOuter()
            is NestedTesterState.Passed -> onEntryOfPassed()
            is NestedTesterState.Second -> onEntryOfSecond(isDefaultEntry)
        }
    }


    private fun onEntryOfDeep() {
                // SCE-MAP: nested_tester.scxml:53 :: deep :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            sceTestContext.startTimer("tick", 500000L)
                }
    }

    private fun onEntryOfFailed() {
                // SCE-MAP: nested_tester.scxml:67 :: failed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFirst() {
                // SCE-MAP: nested_tester.scxml:29 :: first :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            sceTestContext.startTimer("tick", 250000L)
                }
    }

    private fun onEntryOfOuter() {
                // SCE-MAP: nested_tester.scxml:16 :: outer :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            sceTestContext.startTimer("limit", 2000000L)
                }
    }

    private fun onEntryOfPassed() {
                // SCE-MAP: nested_tester.scxml:66 :: passed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfSecond(isDefaultEntry: Boolean) {
                // SCE-MAP: nested_tester.scxml:41 :: second :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            sceTestContext.log("entering second")
                }
                // W3C SCXML 3.3.2: the <initial> transition's content runs when,
                // and only when, this state's initial state is entered by
                // default — not when the state is entered only as the ancestor
                // of a deeper target.
                if (isDefaultEntry) {
                    // W3C SCXML 4.9: its own block, ended by `return@run`.
                    run {

            sceTestContext.log("into deep")
                    }
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: nested_tester.scxml:8 :: _machine
    override fun onExit(state: NestedTesterState) {
        when (state) {
            is NestedTesterState.Deep -> onExitOfDeep()
            is NestedTesterState.Failed -> onExitOfFailed()
            is NestedTesterState.First -> onExitOfFirst()
            is NestedTesterState.Outer -> onExitOfOuter()
            is NestedTesterState.Passed -> onExitOfPassed()
            is NestedTesterState.Second -> onExitOfSecond()
        }
    }


    private fun onExitOfDeep() {
                // SCE-MAP: nested_tester.scxml:53 :: deep :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {

            sceTestContext.log("leaving deep")
                }
    }

    private fun onExitOfFailed() {
                // SCE-MAP: nested_tester.scxml:67 :: failed :: _state_body
    }

    private fun onExitOfFirst() {
                // SCE-MAP: nested_tester.scxml:29 :: first :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {

            sceTestContext.log("leaving first")
                }
    }

    private fun onExitOfOuter() {
                // SCE-MAP: nested_tester.scxml:16 :: outer :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {

            sceTestContext.log("leaving outer")
                }
    }

    private fun onExitOfPassed() {
                // SCE-MAP: nested_tester.scxml:66 :: passed :: _state_body
    }

    private fun onExitOfSecond() {
                // SCE-MAP: nested_tester.scxml:41 :: second :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {

            sceTestContext.log("leaving second")
                }
    }



    // docs/SCE_TEST_PROFILE.md §4.4 (P4 step 1), §12.3: a branch of the test
    // component is taken before the microstep exits any state: it removes what
    // it matched from the round's snapshot, writes its record in the test log,
    // and holds a message its content reads in the payload member of its
    // record's event (`Transition::native_take`).
    override fun takeTransition(source: NestedTesterState, transitionIndex: Int) {
        when {
            source is NestedTesterState.Deep && transitionIndex == 0 -> {
                sceTestContext.timeout("tick")
            }
            source is NestedTesterState.First && transitionIndex == 0 -> {
                sceTestContext.timeout("tick")
            }
            source is NestedTesterState.First && transitionIndex == 1 -> {
                sceTestContext.receive("sut")
            }
            source is NestedTesterState.Outer && transitionIndex == 0 -> {
                sceTestContext.timeout("limit")
            }
            source is NestedTesterState.Outer && transitionIndex == 1 -> {
                sceTestContext.receive("sut")
            }
        }
    }

    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: nested_tester.scxml:8 :: _machine
    override fun executeTransitionContent(source: NestedTesterState, transitionIndex: Int) {
        when (source) {
            is NestedTesterState.Deep -> transitionContentOfDeep(transitionIndex)
            is NestedTesterState.First -> transitionContentOfFirst(transitionIndex)
            is NestedTesterState.Outer -> transitionContentOfOuter(transitionIndex)
            else -> {}
        }
    }


    // The content of `deep`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfDeep(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfDeepAt0()
            else -> {}
        }
    }


    private fun transitionContentOfDeepAt0() {
                // SCE-MAP: nested_tester.scxml:60 :: deep :: _transition_0

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.PASS, "")
    }

    // The content of `first`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfFirst(transitionIndex: Int) {
        when (transitionIndex) {
            1 -> transitionContentOfFirstAt1()
            else -> {}
        }
    }


    private fun transitionContentOfFirstAt1() {
                // SCE-MAP: nested_tester.scxml:37 :: first :: _transition_1

            sceTestContext.log("ignored")
    }

    // The content of `outer`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfOuter(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfOuterAt0()
            1 -> transitionContentOfOuterAt1()
            else -> {}
        }
    }


    private fun transitionContentOfOuterAt0() {
                // SCE-MAP: nested_tester.scxml:23 :: outer :: _transition_0

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.FAIL, "out of time")
    }

    private fun transitionContentOfOuterAt1() {
                // SCE-MAP: nested_tester.scxml:26 :: outer :: _transition_1

            sceTestContext.log("restart")
    }
}
