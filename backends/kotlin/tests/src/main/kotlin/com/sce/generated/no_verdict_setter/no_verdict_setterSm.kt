// SCE-GENERATED — DO NOT EDIT
// source-hash: 982a23a5ec15b8216f29e9e8e4ba6c1047dee0d66671c7835c2776a4de5a13d2

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/test_profile/boundary/no_verdict_setter.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: no_verdict_setter.scxml:5 :: _machine

package com.sce.generated.no_verdict_setter

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface NoVerdictSetterState : State {
    data object Deciding : NoVerdictSetterState
    data object Done : NoVerdictSetterState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface NoVerdictSetterEvent : Event {
    sealed interface Sce : NoVerdictSetterEvent {
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
 * The test component `no_verdict_setter` as the profile's runtime drives it
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
class NoVerdictSetterComponent : com.sce.runtime.test.ComponentMachine {
    private var sceMachine: NoVerdictSetterStateMachine? = null

    override fun start(context: com.sce.runtime.test.ComponentContext) {
        // The session is named by the component, and named rather than drawn.
        // The name is `mtc` or the id of the `<invoke>` that creates the PTC
        // (`ComponentContext.create`, which creates nothing under a name its
        // configuration has no PTC of), and every `<invoke id>` is refused
        // where its document is parsed unless it is an XML Name, which is an
        // Nmtoken (docs/adr/0045). Every `sce-static` root takes the session id
        // its host gives, whether or not it saves (docs/adr/0045, answer 2), so
        // the component's machine is named by the component.
        val machine = NoVerdictSetterStateMachine(sceTestContext = context, sessionId = context.name)
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
        machine.send(NoVerdictSetterEvent.Sce.Test.Snapshot)
        machine.tick()
    }

    // P10: the machine entered a top-level `<final>`.
    override fun terminated(): Boolean = sceMachine?.isInFinalState == true
}

class NoVerdictSetterStateMachine(
    /**
     * docs/SCE_TEST_PROFILE.md §12.3: the runtime context of the test
     * component this machine is, which its branches' guards read the round's
     * snapshot through and its statements call. Given by the component the
     * runtime drives (NoVerdictSetterComponent) when it builds the machine,
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
) : StateMachineEngine<NoVerdictSetterState, NoVerdictSetterEvent>() {

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
        first = "ff9b52aa9906c9d1f95e1b5014b2b01b2b228ee0d751b0a4a6856e2a87f6e3d6",
        revised = "579d75764ec09d9c0e884d1b6764813c56614c455b2aa58cee1a7a26788aaed7",
        format2First = "ff9b52aa9906c9d1f95e1b5014b2b01b2b228ee0d751b0a4a6856e2a87f6e3d6",
        format2Revised = "579d75764ec09d9c0e884d1b6764813c56614c455b2aa58cee1a7a26788aaed7",
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

    override val initialState: NoVerdictSetterState = NoVerdictSetterState.Deciding

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
    override fun isFinalState(state: NoVerdictSetterState): Boolean = when (state) {
        is NoVerdictSetterState.Done -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<NoVerdictSetterState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<NoVerdictSetterState, HistoryId>> =
            listOf(StateTarget(NoVerdictSetterState.Deciding))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: deciding's transition 0, as the microstep reads it.
        val transitionDecidingAt0 = EnabledTransition<NoVerdictSetterState, HistoryId>(
            NoVerdictSetterState.Deciding,
            listOf(StateTarget(NoVerdictSetterState.Done)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): NoVerdictSetterState? = when (stateId) {
        "deciding" -> NoVerdictSetterState.Deciding
        "done" -> NoVerdictSetterState.Done
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: NoVerdictSetterState): String = when (state) {
        is NoVerdictSetterState.Deciding -> "deciding"
        is NoVerdictSetterState.Done -> "done"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: NoVerdictSetterState): Int = when (state) {
        is NoVerdictSetterState.Deciding -> 0
        is NoVerdictSetterState.Done -> 1
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): NoVerdictSetterEvent? = when (name) {
        "sce.test.snapshot" -> NoVerdictSetterEvent.Sce.Test.Snapshot
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: NoVerdictSetterEvent): String? = when (event) {
        is NoVerdictSetterEvent.Sce.Test.Snapshot -> "sce.test.snapshot"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: NoVerdictSetterState, event: NoVerdictSetterEvent?): EnabledTransition<NoVerdictSetterState, HistoryId>? = when (state) {
        is NoVerdictSetterState.Deciding -> firstEnabledTransitionOfDeciding(state, event)
        else -> null
    }


    // deciding's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfDeciding(state: NoVerdictSetterState, event: NoVerdictSetterEvent?): EnabledTransition<NoVerdictSetterState, HistoryId>? = when {
        event == null -> TransitionsPart0.transitionDecidingAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: no_verdict_setter.scxml:5 :: _machine
    override fun onEntry(state: NoVerdictSetterState, isDefaultEntry: Boolean) {
        when (state) {
            is NoVerdictSetterState.Deciding -> onEntryOfDeciding()
            is NoVerdictSetterState.Done -> onEntryOfDone()
        }
    }


    private fun onEntryOfDeciding() {
                // SCE-MAP: no_verdict_setter.scxml:9 :: deciding :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.NONE, "")
                }
    }

    private fun onEntryOfDone() {
                // SCE-MAP: no_verdict_setter.scxml:15 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: no_verdict_setter.scxml:5 :: _machine
    override fun onExit(state: NoVerdictSetterState) {
        when (state) {
            is NoVerdictSetterState.Deciding -> onExitOfDeciding()
            is NoVerdictSetterState.Done -> onExitOfDone()
        }
    }


    private fun onExitOfDeciding() {
                // SCE-MAP: no_verdict_setter.scxml:9 :: deciding :: _state_body
    }

    private fun onExitOfDone() {
                // SCE-MAP: no_verdict_setter.scxml:15 :: done :: _state_body
    }



    // docs/SCE_TEST_PROFILE.md §4.4 (P4 step 1), §12.3: a branch of the test
    // component is taken before the microstep exits any state: it removes what
    // it matched from the round's snapshot, writes its record in the test log,
    // and holds a message its content reads in the payload member of its
    // record's event (`Transition::native_take`).
    override fun takeTransition(source: NoVerdictSetterState, transitionIndex: Int) {
        when {
        }
    }

    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: no_verdict_setter.scxml:5 :: _machine
    override fun executeTransitionContent(source: NoVerdictSetterState, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
