// SCE-GENERATED — DO NOT EDIT
// source-hash: 239a1140d1db8dfa52f222a8f17ddb51efb6fc1d0cddb120dc5b4b0ee2173ab0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/nested_lifecycle/nested_lifecycle.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: nested_lifecycle.scxml:27 :: _machine

package com.sce.integration.nested_lifecycle

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface NestedLifecycleState : State {
    data object Active : NestedLifecycleState
    data object Closed : NestedLifecycleState
    data object Held : NestedLifecycleState
    data object Loading : NestedLifecycleState
    data object Ready : NestedLifecycleState
    data object Retrying : NestedLifecycleState
    data object Session : NestedLifecycleState
    data object Setup : NestedLifecycleState
    data object Working : NestedLifecycleState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface NestedLifecycleEvent : Event {
    data object Wildcard : NestedLifecycleEvent
    data object Abort : NestedLifecycleEvent
    sealed interface Data : NestedLifecycleEvent {
        data object Failed : Data
        data object Ready : Data
    }
    data object Done : NestedLifecycleEvent
    data object Go : NestedLifecycleEvent
    data object Pause : NestedLifecycleEvent
    data object Reset : NestedLifecycleEvent
    data object Resume : NestedLifecycleEvent
}
// --- State Machine (W3C SCXML) ---

class NestedLifecycleStateMachine(
) : StateMachineEngine<NestedLifecycleState, NestedLifecycleEvent>() {

    override val initialState: NestedLifecycleState = NestedLifecycleState.Loading

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
    override fun parentOf(state: NestedLifecycleState): NestedLifecycleState? = when (state) {
        is NestedLifecycleState.Active -> NestedLifecycleState.Session
        is NestedLifecycleState.Held -> NestedLifecycleState.Active
        is NestedLifecycleState.Loading -> NestedLifecycleState.Setup
        is NestedLifecycleState.Ready -> NestedLifecycleState.Setup
        is NestedLifecycleState.Retrying -> NestedLifecycleState.Setup
        is NestedLifecycleState.Setup -> NestedLifecycleState.Session
        is NestedLifecycleState.Working -> NestedLifecycleState.Active
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: NestedLifecycleState): Boolean = when (state) {
        is NestedLifecycleState.Active -> true
        is NestedLifecycleState.Session -> true
        is NestedLifecycleState.Setup -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: NestedLifecycleState): Boolean = when (state) {
        is NestedLifecycleState.Closed -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: NestedLifecycleState): List<NestedLifecycleState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: NestedLifecycleState): List<EntryTarget<NestedLifecycleState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<NestedLifecycleState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<NestedLifecycleState, List<NestedLifecycleState>> = LinkedHashMap<NestedLifecycleState, List<NestedLifecycleState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<NestedLifecycleState, List<NestedLifecycleState>>) {
            table[NestedLifecycleState.Active] = listOf(NestedLifecycleState.Working, NestedLifecycleState.Held)
            table[NestedLifecycleState.Session] = listOf(NestedLifecycleState.Active, NestedLifecycleState.Setup)
            table[NestedLifecycleState.Setup] = listOf(NestedLifecycleState.Retrying, NestedLifecycleState.Loading, NestedLifecycleState.Ready)
        }


        val initialTargets: Map<NestedLifecycleState, List<EntryTarget<NestedLifecycleState, HistoryId>>> = LinkedHashMap<NestedLifecycleState, List<EntryTarget<NestedLifecycleState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<NestedLifecycleState, List<EntryTarget<NestedLifecycleState, HistoryId>>>) {
            table[NestedLifecycleState.Active] = listOf(StateTarget(NestedLifecycleState.Working))
            table[NestedLifecycleState.Session] = listOf(StateTarget(NestedLifecycleState.Setup))
            table[NestedLifecycleState.Setup] = listOf(StateTarget(NestedLifecycleState.Loading))
        }


        val documentInitialTargetList: List<EntryTarget<NestedLifecycleState, HistoryId>> =
            listOf(StateTarget(NestedLifecycleState.Session))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: active's transition 0, as the microstep reads it.
        val transitionActiveAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Active,
            listOf(StateTarget(NestedLifecycleState.Held)),
            0,
            hasActions = false,
            isInternal = true,
        )

        // W3C SCXML 3.13: active's transition 1, as the microstep reads it.
        val transitionActiveAt1 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Active,
            listOf(StateTarget(NestedLifecycleState.Active)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: held's transition 0, as the microstep reads it.
        val transitionHeldAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Held,
            listOf(StateTarget(NestedLifecycleState.Working)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: held's transition 1, as the microstep reads it.
        val transitionHeldAt1 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Held,
            emptyList(),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: loading's transition 0, as the microstep reads it.
        val transitionLoadingAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Loading,
            listOf(StateTarget(NestedLifecycleState.Ready)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: loading's transition 1, as the microstep reads it.
        val transitionLoadingAt1 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Loading,
            listOf(StateTarget(NestedLifecycleState.Retrying)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: ready's transition 0, as the microstep reads it.
        val transitionReadyAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Ready,
            listOf(StateTarget(NestedLifecycleState.Active)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: retrying's transition 0, as the microstep reads it.
        val transitionRetryingAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Retrying,
            listOf(StateTarget(NestedLifecycleState.Ready)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: retrying's transition 1, as the microstep reads it.
        val transitionRetryingAt1 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Retrying,
            listOf(StateTarget(NestedLifecycleState.Loading)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: session's transition 0, as the microstep reads it.
        val transitionSessionAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Session,
            listOf(StateTarget(NestedLifecycleState.Closed)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Working,
            listOf(StateTarget(NestedLifecycleState.Closed)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 1, as the microstep reads it.
        val transitionWorkingAt1 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Working,
            emptyList(),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): NestedLifecycleState? = when (stateId) {
        "active" -> NestedLifecycleState.Active
        "closed" -> NestedLifecycleState.Closed
        "held" -> NestedLifecycleState.Held
        "loading" -> NestedLifecycleState.Loading
        "ready" -> NestedLifecycleState.Ready
        "retrying" -> NestedLifecycleState.Retrying
        "session" -> NestedLifecycleState.Session
        "setup" -> NestedLifecycleState.Setup
        "working" -> NestedLifecycleState.Working
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: NestedLifecycleState): String = when (state) {
        is NestedLifecycleState.Active -> "active"
        is NestedLifecycleState.Closed -> "closed"
        is NestedLifecycleState.Held -> "held"
        is NestedLifecycleState.Loading -> "loading"
        is NestedLifecycleState.Ready -> "ready"
        is NestedLifecycleState.Retrying -> "retrying"
        is NestedLifecycleState.Session -> "session"
        is NestedLifecycleState.Setup -> "setup"
        is NestedLifecycleState.Working -> "working"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: NestedLifecycleState): Int = when (state) {
        is NestedLifecycleState.Active -> 1
        is NestedLifecycleState.Closed -> 8
        is NestedLifecycleState.Held -> 3
        is NestedLifecycleState.Loading -> 6
        is NestedLifecycleState.Ready -> 7
        is NestedLifecycleState.Retrying -> 5
        is NestedLifecycleState.Session -> 0
        is NestedLifecycleState.Setup -> 4
        is NestedLifecycleState.Working -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): NestedLifecycleEvent? = when (name) {
        "abort" -> NestedLifecycleEvent.Abort
        "data.failed" -> NestedLifecycleEvent.Data.Failed
        "data.ready" -> NestedLifecycleEvent.Data.Ready
        "done" -> NestedLifecycleEvent.Done
        "go" -> NestedLifecycleEvent.Go
        "pause" -> NestedLifecycleEvent.Pause
        "reset" -> NestedLifecycleEvent.Reset
        "resume" -> NestedLifecycleEvent.Resume
        "*" -> NestedLifecycleEvent.Wildcard
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: NestedLifecycleEvent): String? = when (event) {
        is NestedLifecycleEvent.Abort -> "abort"
        is NestedLifecycleEvent.Data.Failed -> "data.failed"
        is NestedLifecycleEvent.Data.Ready -> "data.ready"
        is NestedLifecycleEvent.Done -> "done"
        is NestedLifecycleEvent.Go -> "go"
        is NestedLifecycleEvent.Pause -> "pause"
        is NestedLifecycleEvent.Reset -> "reset"
        is NestedLifecycleEvent.Resume -> "resume"
        is NestedLifecycleEvent.Wildcard -> "*"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: NestedLifecycleState, event: NestedLifecycleEvent?): EnabledTransition<NestedLifecycleState, HistoryId>? = when (state) {
        is NestedLifecycleState.Active -> firstEnabledTransitionOfActive(state, event)
        is NestedLifecycleState.Held -> firstEnabledTransitionOfHeld(state, event)
        is NestedLifecycleState.Loading -> firstEnabledTransitionOfLoading(state, event)
        is NestedLifecycleState.Ready -> firstEnabledTransitionOfReady(state, event)
        is NestedLifecycleState.Retrying -> firstEnabledTransitionOfRetrying(state, event)
        is NestedLifecycleState.Session -> firstEnabledTransitionOfSession(state, event)
        is NestedLifecycleState.Working -> firstEnabledTransitionOfWorking(state, event)
        else -> null
    }


    // active's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfActive(state: NestedLifecycleState, event: NestedLifecycleEvent?): EnabledTransition<NestedLifecycleState, HistoryId>? = when {
        event is NestedLifecycleEvent.Pause -> TransitionsPart0.transitionActiveAt0
        event is NestedLifecycleEvent.Reset -> TransitionsPart0.transitionActiveAt1
        else -> null
    }


    // held's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfHeld(state: NestedLifecycleState, event: NestedLifecycleEvent?): EnabledTransition<NestedLifecycleState, HistoryId>? = when {
        event is NestedLifecycleEvent.Resume -> TransitionsPart0.transitionHeldAt0
        event != null -> TransitionsPart0.transitionHeldAt1
        else -> null
    }


    // loading's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfLoading(state: NestedLifecycleState, event: NestedLifecycleEvent?): EnabledTransition<NestedLifecycleState, HistoryId>? = when {
        event is NestedLifecycleEvent.Data.Ready -> TransitionsPart0.transitionLoadingAt0
        event is NestedLifecycleEvent.Data.Failed -> TransitionsPart0.transitionLoadingAt1
        else -> null
    }


    // ready's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfReady(state: NestedLifecycleState, event: NestedLifecycleEvent?): EnabledTransition<NestedLifecycleState, HistoryId>? = when {
        event is NestedLifecycleEvent.Go -> TransitionsPart0.transitionReadyAt0
        else -> null
    }


    // retrying's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRetrying(state: NestedLifecycleState, event: NestedLifecycleEvent?): EnabledTransition<NestedLifecycleState, HistoryId>? = when {
        event is NestedLifecycleEvent.Data.Ready -> TransitionsPart0.transitionRetryingAt0
        (event is NestedLifecycleEvent.Data.Failed || event is NestedLifecycleEvent.Data.Ready) -> TransitionsPart0.transitionRetryingAt1
        else -> null
    }


    // session's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSession(state: NestedLifecycleState, event: NestedLifecycleEvent?): EnabledTransition<NestedLifecycleState, HistoryId>? = when {
        event is NestedLifecycleEvent.Abort -> TransitionsPart0.transitionSessionAt0
        else -> null
    }


    // working's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWorking(state: NestedLifecycleState, event: NestedLifecycleEvent?): EnabledTransition<NestedLifecycleState, HistoryId>? = when {
        event is NestedLifecycleEvent.Done -> TransitionsPart0.transitionWorkingAt0
        event is NestedLifecycleEvent.Resume -> TransitionsPart0.transitionWorkingAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: nested_lifecycle.scxml:27 :: _machine
    override fun onEntry(state: NestedLifecycleState, isDefaultEntry: Boolean) {
        when (state) {
            is NestedLifecycleState.Active -> onEntryOfActive()
            is NestedLifecycleState.Closed -> onEntryOfClosed()
            is NestedLifecycleState.Held -> onEntryOfHeld()
            is NestedLifecycleState.Loading -> onEntryOfLoading()
            is NestedLifecycleState.Ready -> onEntryOfReady()
            is NestedLifecycleState.Retrying -> onEntryOfRetrying()
            is NestedLifecycleState.Session -> onEntryOfSession()
            is NestedLifecycleState.Setup -> onEntryOfSetup()
            is NestedLifecycleState.Working -> onEntryOfWorking()
        }
    }


    private fun onEntryOfActive() {
                // SCE-MAP: nested_lifecycle.scxml:36 :: active :: _state_body
    }

    private fun onEntryOfClosed() {
                // SCE-MAP: nested_lifecycle.scxml:63 :: closed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfHeld() {
                // SCE-MAP: nested_lifecycle.scxml:43 :: held :: _state_body
    }

    private fun onEntryOfLoading() {
                // SCE-MAP: nested_lifecycle.scxml:54 :: loading :: _state_body
    }

    private fun onEntryOfReady() {
                // SCE-MAP: nested_lifecycle.scxml:58 :: ready :: _state_body
    }

    private fun onEntryOfRetrying() {
                // SCE-MAP: nested_lifecycle.scxml:50 :: retrying :: _state_body
    }

    private fun onEntryOfSession() {
                // SCE-MAP: nested_lifecycle.scxml:34 :: session :: _state_body
    }

    private fun onEntryOfSetup() {
                // SCE-MAP: nested_lifecycle.scxml:48 :: setup :: _state_body
    }

    private fun onEntryOfWorking() {
                // SCE-MAP: nested_lifecycle.scxml:39 :: working :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: nested_lifecycle.scxml:27 :: _machine
    override fun onExit(state: NestedLifecycleState) {
        when (state) {
            is NestedLifecycleState.Active -> onExitOfActive()
            is NestedLifecycleState.Closed -> onExitOfClosed()
            is NestedLifecycleState.Held -> onExitOfHeld()
            is NestedLifecycleState.Loading -> onExitOfLoading()
            is NestedLifecycleState.Ready -> onExitOfReady()
            is NestedLifecycleState.Retrying -> onExitOfRetrying()
            is NestedLifecycleState.Session -> onExitOfSession()
            is NestedLifecycleState.Setup -> onExitOfSetup()
            is NestedLifecycleState.Working -> onExitOfWorking()
        }
    }


    private fun onExitOfActive() {
                // SCE-MAP: nested_lifecycle.scxml:36 :: active :: _state_body
    }

    private fun onExitOfClosed() {
                // SCE-MAP: nested_lifecycle.scxml:63 :: closed :: _state_body
    }

    private fun onExitOfHeld() {
                // SCE-MAP: nested_lifecycle.scxml:43 :: held :: _state_body
    }

    private fun onExitOfLoading() {
                // SCE-MAP: nested_lifecycle.scxml:54 :: loading :: _state_body
    }

    private fun onExitOfReady() {
                // SCE-MAP: nested_lifecycle.scxml:58 :: ready :: _state_body
    }

    private fun onExitOfRetrying() {
                // SCE-MAP: nested_lifecycle.scxml:50 :: retrying :: _state_body
    }

    private fun onExitOfSession() {
                // SCE-MAP: nested_lifecycle.scxml:34 :: session :: _state_body
    }

    private fun onExitOfSetup() {
                // SCE-MAP: nested_lifecycle.scxml:48 :: setup :: _state_body
    }

    private fun onExitOfWorking() {
                // SCE-MAP: nested_lifecycle.scxml:39 :: working :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: nested_lifecycle.scxml:27 :: _machine
    override fun executeTransitionContent(source: NestedLifecycleState, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
