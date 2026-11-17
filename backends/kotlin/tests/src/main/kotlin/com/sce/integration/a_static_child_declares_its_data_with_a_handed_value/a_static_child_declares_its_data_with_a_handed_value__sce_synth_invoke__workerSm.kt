// SCE-GENERATED — DO NOT EDIT
// source-hash: 4b8a823bdcf1a180eb4e80b1d700a62b8e56e4feb419ba4853f2dd996d3e97e0

// GENERATED CODE — DO NOT EDIT
// Source: tests/integration/a_static_child_declares_its_data_with_a_handed_value.scxml#worker
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:32 :: _machine

package com.sce.integration.a_static_child_declares_its_data_with_a_handed_value

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState : State {
    data object Leaf : AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState
    data object Waiting : AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerEvent : Event {

}
// --- State Machine (W3C SCXML) ---

class AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState, AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `start` datamodel variable, the machine's own. */
    private var vStart: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `copied` datamodel variable, the machine's own. */
    private var vCopied: UInt = vStart

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var start: UInt? = null
        var copied: UInt? = null
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
        vStart = params.start ?: (0.toUInt())
        vCopied = params.copied ?: (vStart)
    }

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState>,
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
        first = "74a24654af29dd5e13bb2995c88299b7a9e3615124aea0cd139f7da602122166",
        revised = "5a949e935b8fde75b001e083801cc2487b2c4426a6e1c899539f912ca8244883",
        format2First = "74a24654af29dd5e13bb2995c88299b7a9e3615124aea0cd139f7da602122166",
        format2Revised = "5a949e935b8fde75b001e083801cc2487b2c4426a6e1c899539f912ca8244883",
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
        variables += "start" to SavedValues.of(vStart)
        variables += "copied" to SavedValues.of(vCopied)
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
        vStart = SavedValues.uint32(savedGrown.variable("start"), "start")
        vCopied = SavedValues.uint32(savedGrown.variable("copied"), "copied")
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
        SavedValues.uint32(savedGrown.variable("start"), "start")
        SavedValues.uint32(savedGrown.variable("copied"), "copied")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    override val initialState: AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState = AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Waiting

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
    override fun isFinalState(state: AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState): Boolean = when (state) {
        is AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Leaf -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState, HistoryId>> =
            listOf(StateTarget(AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Waiting))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState, HistoryId>(
            AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Waiting,
            listOf(StateTarget(AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Leaf)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState? = when (stateId) {
        "leaf" -> AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Leaf
        "waiting" -> AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Waiting
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState): String = when (state) {
        is AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Leaf -> "leaf"
        is AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Waiting -> "waiting"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState): Int = when (state) {
        is AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Leaf -> 1
        is AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Waiting -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerEvent? = when (name) {
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    // A child SM that inherits the has_parent_communication override while
    // declaring no events of its own leaves the sealed hierarchy with zero
    // implementors, so `AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerEvent` is uninhabited: no caller can
    // construct an argument and the body is unreachable. A `when` over an
    // uninhabited sealed subject is vacuously exhaustive, so any branch —
    // `else` included — is dead code the compiler rejects under -Werror.
    // Returning the null directly is the honest expression of "unreachable".
    override fun eventNameOf(event: AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerEvent): String? = null





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState, event: AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerEvent?): EnabledTransition<AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState, HistoryId>? = when (state) {
        is AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Waiting -> firstEnabledTransitionOfWaiting(state, event)
        else -> null
    }


    // waiting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWaiting(state: AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState, event: AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerEvent?): EnabledTransition<AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState, HistoryId>? = when {
        event == null && vCopied == 7.toUInt() -> TransitionsPart0.transitionWaitingAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:32 :: _machine
    override fun onEntry(state: AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState, isDefaultEntry: Boolean) {
        when (state) {
            is AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Leaf -> onEntryOfLeaf()
            is AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Waiting -> onEntryOfWaiting()
        }
    }


    private fun onEntryOfLeaf() {
                // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:41 :: leaf :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfWaiting() {
                // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:38 :: waiting :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:32 :: _machine
    override fun onExit(state: AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState) {
        when (state) {
            is AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Leaf -> onExitOfLeaf()
            is AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState.Waiting -> onExitOfWaiting()
        }
    }


    private fun onExitOfLeaf() {
                // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:41 :: leaf :: _state_body
    }

    private fun onExitOfWaiting() {
                // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:38 :: waiting :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_static_child_declares_its_data_with_a_handed_value.scxml:32 :: _machine
    override fun executeTransitionContent(source: AStaticChildDeclaresItsDataWithAHandedValueSceSynthInvokeWorkerState, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
