// SCE-GENERATED — DO NOT EDIT
// source-hash: 91c96c5aae6c2839b0a4564f003a89035e84e73317ceabe8eeedb37630133d39
// source: a_static_child_declares_its_data_with_a_handed_value.scxml

// GENERATED CODE — DO NOT EDIT
// Source: tests/integration/a_static_child_declares_its_data_with_a_handed_value.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:22 :: _machine

package com.sce.integration.a_static_child_declares_its_data_with_a_handed_value

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AStaticChildDeclaresItsDataWithAHandedValueState : State {
    data object Passed : AStaticChildDeclaresItsDataWithAHandedValueState
    data object Working : AStaticChildDeclaresItsDataWithAHandedValueState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AStaticChildDeclaresItsDataWithAHandedValueEvent : Event {
    sealed interface Done : AStaticChildDeclaresItsDataWithAHandedValueEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Worker : Invoke
        }
    }
    sealed interface Error : AStaticChildDeclaresItsDataWithAHandedValueEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class AStaticChildDeclaresItsDataWithAHandedValueStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<AStaticChildDeclaresItsDataWithAHandedValueState, AStaticChildDeclaresItsDataWithAHandedValueEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `base` datamodel variable, the machine's own. */
    private var vBase: UInt = 7.toUInt()

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var base: UInt? = null
    }

    /**
     * §scxml-6.4: give this machine the values [params] carries, each as the
     * initial value of its variable in place of the one its `<data>` gives, so
     * that a variable declared after it whose initial value reads it reads the
     * handed value (docs/adr/0098). Called before [initialize].
     */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    @Suppress("UNUSED_PARAMETER")
    private fun acceptParamsPart0(params: InvokeParams) {
        vBase = params.base ?: (7.toUInt())
    }

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<AStaticChildDeclaresItsDataWithAHandedValueState>,
        val truncated: Boolean,
    )

    private val _snapshot = kotlinx.coroutines.flow.MutableStateFlow(
        Snapshot(emptySet(), false)
    )

    /**
     * The machine as the host sees it: one [Snapshot] per completed macrostep
     * (W3C SCXML Appendix D), never a state between two microsteps.
     *
     * Compose integration: `val s by sm.snapshot.collectAsState()`
     */
    val snapshot: kotlinx.coroutines.flow.StateFlow<Snapshot>
        get() = _snapshot

    override fun onMacrostepComplete(truncated: Boolean) {
        _snapshot.value = Snapshot(activeConfiguration, truncated)
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
        first = "22eb7113d6b10ba28d8e4065bb6465ee4ca806712406fb8bce5f2f7cf979eba0",
        revised = "3c5375af0ea6c230a735b2b6dc4317184d10c1b6c6aea0e686bbf6a050b495f7",
        format2First = "22eb7113d6b10ba28d8e4065bb6465ee4ca806712406fb8bce5f2f7cf979eba0",
        format2Revised = "3c5375af0ea6c230a735b2b6dc4317184d10c1b6c6aea0e686bbf6a050b495f7",
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
        saveVariablesPart0(variables)
        return savedState(savedShape, variables, wallNowMs)
    }

    private fun saveVariablesPart0(variables: MutableMap<String, Any?>) {
        variables += "base" to SavedValues.of(vBase)
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
        judgeVariablesPart0(savedGrown)
        restoreVariablesPart0(savedGrown)
        enterSaved(savedGrown, wallNowMs)
    }

    // Writes this part's variables from [savedGrown], which every judge part
    // has already held to its variables' types.
    private fun restoreVariablesPart0(savedGrown: SavedState) {
        vBase = SavedValues.uint32(savedGrown.variable("base"), "base")
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // docs/adr/0045: this machine as a child session of another's saved state —
    // saved with it, judged with it before anything is built, and restored in
    // place of initialized.
    override fun saveAsSession(wallNowMs: Long): SavedState = save(wallNowMs)

    override fun judgeAsSession(saved: SavedState) {
        val savedGrown = beginRestore(saved, savedShape)
        judgeVariablesPart0(savedGrown)
    }

    private fun judgeVariablesPart0(savedGrown: SavedState) {
        SavedValues.uint32(savedGrown.variable("base"), "base")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4: a saved state names the `<invoke>`s whose child is running,
    // and a restore starts each again from its beginning. This document saves
    // only when every invoke is a static child session, a hybrid one among
    // declared candidates, or one a declared host invoker serves.
    override val staticInvokes: List<Pair<String, AStaticChildDeclaresItsDataWithAHandedValueState>> = listOf(
        "worker" to AStaticChildDeclaresItsDataWithAHandedValueState.Working,
    )

    override fun restartInvoke(invokeId: String, state: AStaticChildDeclaresItsDataWithAHandedValueState) = deferStaticInvoke(invokeId, state)

    override val initialState: AStaticChildDeclaresItsDataWithAHandedValueState = AStaticChildDeclaresItsDataWithAHandedValueState.Working

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
    override fun isFinalState(state: AStaticChildDeclaresItsDataWithAHandedValueState): Boolean = when (state) {
        is AStaticChildDeclaresItsDataWithAHandedValueState.Passed -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AStaticChildDeclaresItsDataWithAHandedValueState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AStaticChildDeclaresItsDataWithAHandedValueState, HistoryId>> =
            listOf(StateTarget(AStaticChildDeclaresItsDataWithAHandedValueState.Working))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<AStaticChildDeclaresItsDataWithAHandedValueState, HistoryId>(
            AStaticChildDeclaresItsDataWithAHandedValueState.Working,
            listOf(StateTarget(AStaticChildDeclaresItsDataWithAHandedValueState.Passed)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AStaticChildDeclaresItsDataWithAHandedValueState? = when (stateId) {
        "passed" -> AStaticChildDeclaresItsDataWithAHandedValueState.Passed
        "working" -> AStaticChildDeclaresItsDataWithAHandedValueState.Working
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AStaticChildDeclaresItsDataWithAHandedValueState): String = when (state) {
        is AStaticChildDeclaresItsDataWithAHandedValueState.Passed -> "passed"
        is AStaticChildDeclaresItsDataWithAHandedValueState.Working -> "working"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AStaticChildDeclaresItsDataWithAHandedValueState): Int = when (state) {
        is AStaticChildDeclaresItsDataWithAHandedValueState.Passed -> 1
        is AStaticChildDeclaresItsDataWithAHandedValueState.Working -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AStaticChildDeclaresItsDataWithAHandedValueEvent? = when (name) {
        "done.invoke" -> AStaticChildDeclaresItsDataWithAHandedValueEvent.Done.Invoke.Self
        "done.invoke.worker" -> AStaticChildDeclaresItsDataWithAHandedValueEvent.Done.Invoke.Worker
        "error.execution" -> AStaticChildDeclaresItsDataWithAHandedValueEvent.Error.Execution
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AStaticChildDeclaresItsDataWithAHandedValueEvent): String? = when (event) {
        is AStaticChildDeclaresItsDataWithAHandedValueEvent.Done.Invoke.Self -> "done.invoke"
        is AStaticChildDeclaresItsDataWithAHandedValueEvent.Done.Invoke.Worker -> "done.invoke.worker"
        is AStaticChildDeclaresItsDataWithAHandedValueEvent.Error.Execution -> "error.execution"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: AStaticChildDeclaresItsDataWithAHandedValueState, event: AStaticChildDeclaresItsDataWithAHandedValueEvent?): EnabledTransition<AStaticChildDeclaresItsDataWithAHandedValueState, HistoryId>? = when (state) {
        is AStaticChildDeclaresItsDataWithAHandedValueState.Working -> firstEnabledTransitionOfWorking(state, event)
        else -> null
    }


    // working's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWorking(state: AStaticChildDeclaresItsDataWithAHandedValueState, event: AStaticChildDeclaresItsDataWithAHandedValueEvent?): EnabledTransition<AStaticChildDeclaresItsDataWithAHandedValueState, HistoryId>? = when {
        event is AStaticChildDeclaresItsDataWithAHandedValueEvent.Done.Invoke.Worker -> TransitionsPart0.transitionWorkingAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:22 :: _machine
    override fun onEntry(state: AStaticChildDeclaresItsDataWithAHandedValueState, isDefaultEntry: Boolean) {
        when (state) {
            is AStaticChildDeclaresItsDataWithAHandedValueState.Passed -> onEntryOfPassed()
            is AStaticChildDeclaresItsDataWithAHandedValueState.Working -> onEntryOfWorking()
        }
    }


    private fun onEntryOfPassed() {
                // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:47 :: passed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfWorking() {
                // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:28 :: working :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("worker", AStaticChildDeclaresItsDataWithAHandedValueState.Working)
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
    private fun deferStaticInvoke(invokeId: String, state: AStaticChildDeclaresItsDataWithAHandedValueState) {
        when (invokeId) {
            "worker" -> startInvokeOfWorkingAt0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfWorkingAt0(state: AStaticChildDeclaresItsDataWithAHandedValueState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "working.${nextInvokeStart("worker")}.worker"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerStateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                    val childSeed = AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerStateMachine.InvokeParams()
                    childSeed.start = vBase
                    childSM.acceptParams(childSeed)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("worker", childSM, false, AStaticChildDeclaresItsDataWithAHandedValueEvent.Done.Invoke.Worker, "", generatedInvokeId, "", "a_static_child_declares_its_data_with_a_handed_value__sce_synth_invoke__worker", "worker")
                }
            }
    }

    // docs/adr/0045, section 3, steps 1 and 3: the machine of a running child a
    // saved state lists, built as the `<invoke>` that started it builds it and
    // with what a start does with the child once built. Nothing is evaluated here:
    // no `<param>`, no `srcexpr`, no `idlocation` — the child resumes where it
    // stood, and a hybrid child is the candidate its saved `document` names,
    // whatever the `srcexpr` reads now.
    //
    // Each invoke's child is built by a method of its own
    // (`savedChildOf<State>At<n>`), and this dispatches to it with one
    // call an arm, for the reason `onEntry` does.
    override fun buildSavedChild(child: SavedChild): ResumedChild<AStaticChildDeclaresItsDataWithAHandedValueEvent> = when (child.id) {
        "worker" -> savedChildOfWorkingAt0(child)
        else -> throw StateRefusal("the document has no child session '${child.id}'")
    }


    private fun savedChildOfWorkingAt0(child: SavedChild): ResumedChild<AStaticChildDeclaresItsDataWithAHandedValueEvent> =
        if (child.document == "a_static_child_declares_its_data_with_a_handed_value__sce_synth_invoke__worker") {
            ResumedChild(
                AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerStateMachine(),
                false, AStaticChildDeclaresItsDataWithAHandedValueEvent.Done.Invoke.Worker, "", "", "worker",
            )
        } else {
            throw StateRefusal("invokes names '${child.id}' running '${child.document}', a document that `<invoke>` does not start")
        }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:22 :: _machine
    override fun onExit(state: AStaticChildDeclaresItsDataWithAHandedValueState) {
        when (state) {
            is AStaticChildDeclaresItsDataWithAHandedValueState.Passed -> onExitOfPassed()
            is AStaticChildDeclaresItsDataWithAHandedValueState.Working -> onExitOfWorking()
        }
    }


    private fun onExitOfPassed() {
                // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:47 :: passed :: _state_body
    }

    private fun onExitOfWorking() {
                // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:28 :: working :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(AStaticChildDeclaresItsDataWithAHandedValueState.Working)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("worker")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:22 :: _machine
    override fun executeTransitionContent(source: AStaticChildDeclaresItsDataWithAHandedValueState, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
