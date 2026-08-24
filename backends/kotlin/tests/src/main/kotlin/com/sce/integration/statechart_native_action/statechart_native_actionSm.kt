// SCE-GENERATED — DO NOT EDIT
// source-hash: d8762291f8adee4223c5af3de347a3acefddf00a392ed861e70d8d9802dc3abb
// template-hash: 580f12cd61336d7449660775c4fcc4f615ee3c32bffa0e9792363e260aed93e2
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/event_schema/statechart_native_action.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_native_action.scxml:31 :: _machine

package com.sce.integration.statechart_native_action

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartNativeActionState : State {
    data object Assembling : StatechartNativeActionState
    data object Faulted : StatechartNativeActionState
    data object Idle : StatechartNativeActionState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartNativeActionEvent : Event {
    sealed interface Error : StatechartNativeActionEvent {
        data object Execution : Error
    }
    sealed interface Fragment : StatechartNativeActionEvent {
        data object Received : Fragment
    }
    data object Reset : StatechartNativeActionEvent
    data object Selftest : StatechartNativeActionEvent
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StatechartNativeActionPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StatechartNativeActionFragmentReceivedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `fragment.received`. Consumers inject it via the `raiseFragmentReceived` seam
// on the machine — they never name this class directly.
data class StatechartNativeActionFragmentReceivedPayload(val payload: ByteArray, val offset: UInt)


// --- State Machine (W3C SCXML) ---

// ── W3C SCXML G.7: `<sce:action>` host dispatch ───────────────────────
/**
 * W3C SCXML G.7: host operations dispatched by `<sce:action>`.
 * The host supplies the side effects while the statechart keeps each
 * operation symbolic. No runtime script engine is involved.
 */
interface StatechartNativeActionActions {
    fun appendFragmentPayload(payload: ByteArray, offset: UInt)
    fun onAssemblingExit()
    fun onIdleEntry()
    fun resetSlot()
}

class StatechartNativeActionStateMachine(
    /**
     * W3C SCXML G.7: the host implementation every `<sce:action>` in this
     * document calls directly (`actions.<op>(…)`) instead of the script
     * engine.
     *
     * A constructor parameter rather than a setter, because the initial
     * state's `<onentry>` can already perform an act — a host installed
     * afterwards would arrive one act too late. It leads the parameter list
     * so it stays in the same position whether or not this machine also
     * takes a script engine.
     */
    private val actions: StatechartNativeActionActions,
) : StateMachineEngine<StatechartNativeActionState, StatechartNativeActionEvent>() {

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingFragmentReceivedPayload: StatechartNativeActionFragmentReceivedPayload? = null

    // NL→IR Item C1 Path A: lift the dequeued event's type-erased typed payload
    // into the matching nullable field (a non-typed carrier resets all to null,
    // so every typed guard fails). Twin of the Go policy's PopulateEventMetadata
    // type-switch / the C11 pop loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(metadata: EventMetadata) {
        pendingFragmentReceivedPayload = null
        when (val tp = metadata.typedPayload) {
            is StatechartNativeActionFragmentReceivedPayload -> pendingFragmentReceivedPayload = tp
            else -> {}
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `fragment.received` — binds the event name and the payload field values in one call.
    fun raiseFragmentReceived(payload: ByteArray, offset: UInt) {
        send(
            StatechartNativeActionEvent.Fragment.Received,
            EventMetadata(type = "external", typedPayload = StatechartNativeActionFragmentReceivedPayload(payload, offset))
        )
    }


    override val initialState: StatechartNativeActionState = StatechartNativeActionState.Idle

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StatechartNativeActionState? = when (stateId) {
        "assembling" -> StatechartNativeActionState.Assembling
        "faulted" -> StatechartNativeActionState.Faulted
        "idle" -> StatechartNativeActionState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartNativeActionState): String = when (state) {
        is StatechartNativeActionState.Assembling -> "assembling"
        is StatechartNativeActionState.Faulted -> "faulted"
        is StatechartNativeActionState.Idle -> "idle"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: StatechartNativeActionState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: StatechartNativeActionState): Int = when (state) {
        is StatechartNativeActionState.Assembling -> 1
        is StatechartNativeActionState.Faulted -> 2
        is StatechartNativeActionState.Idle -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: StatechartNativeActionState,
        event: StatechartNativeActionEvent
    ): TransitionResult<StatechartNativeActionState> = when (state) {
        is StatechartNativeActionState.Assembling -> processAssembling(event)
        is StatechartNativeActionState.Faulted -> processFaulted(event)
        is StatechartNativeActionState.Idle -> processIdle(event)
    }


    // --- Per-State Event Handlers ---

    private fun processAssembling(
        event: StatechartNativeActionEvent
    ): TransitionResult<StatechartNativeActionState> = when {
        event is StatechartNativeActionEvent.Reset -> TransitionResult.External(StatechartNativeActionState.Idle, StatechartNativeActionState.Assembling)

        event is StatechartNativeActionEvent.Error.Execution -> TransitionResult.External(StatechartNativeActionState.Faulted, StatechartNativeActionState.Assembling)

        else -> TransitionResult.Ignored
    }

    private fun processFaulted(
        event: StatechartNativeActionEvent
    ): TransitionResult<StatechartNativeActionState> = when {
        event is StatechartNativeActionEvent.Reset -> TransitionResult.External(StatechartNativeActionState.Idle, StatechartNativeActionState.Faulted)

        else -> TransitionResult.Ignored
    }

    private fun processIdle(
        event: StatechartNativeActionEvent
    ): TransitionResult<StatechartNativeActionState> = when {
        event is StatechartNativeActionEvent.Fragment.Received -> TransitionResult.External(StatechartNativeActionState.Assembling, StatechartNativeActionState.Idle)

        // W3C SCXML 3.13: Targetless transition (actions only)
        event is StatechartNativeActionEvent.Selftest -> TransitionResult.Internal
        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_native_action.scxml:31 :: _machine
    override fun onEntry(state: StatechartNativeActionState, pathChild: StatechartNativeActionState?) {
        when (state) {
            is StatechartNativeActionState.Assembling -> {
                // SCE-MAP: statechart_native_action.scxml:52 :: assembling :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("assembling")) return
            }
            is StatechartNativeActionState.Faulted -> {
                // SCE-MAP: statechart_native_action.scxml:64 :: faulted :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("faulted")) return
            }
            is StatechartNativeActionState.Idle -> {
                // SCE-MAP: statechart_native_action.scxml:38 :: idle :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("idle")) return

            // W3C SCXML G.7: <sce:action name="on_idle_entry">
            actions.onIdleEntry()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_native_action.scxml:31 :: _machine
    override fun onExit(state: StatechartNativeActionState) {
        when (state) {
            is StatechartNativeActionState.Assembling -> {
                // SCE-MAP: statechart_native_action.scxml:52 :: assembling :: _state_body
                activeStateIds.remove("assembling")

            // W3C SCXML G.7: <sce:action name="on_assembling_exit">
            actions.onAssemblingExit()
            }
            is StatechartNativeActionState.Faulted -> {
                // SCE-MAP: statechart_native_action.scxml:64 :: faulted :: _state_body
                activeStateIds.remove("faulted")
            }
            is StatechartNativeActionState.Idle -> {
                // SCE-MAP: statechart_native_action.scxml:38 :: idle :: _state_body
                activeStateIds.remove("idle")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: statechart_native_action.scxml:31 :: _machine
    override fun executeTransitionActions(
        source: StatechartNativeActionState,
        event: StatechartNativeActionEvent?
    ) {
        when (source) {
        is StatechartNativeActionState.Assembling -> when {
            event is StatechartNativeActionEvent.Reset -> {
                // SCE-MAP: statechart_native_action.scxml:56 :: assembling :: _transition_0

            // W3C SCXML G.7: <sce:action name="reset_slot">
            actions.resetSlot()
            }
            else -> {}
        }
        is StatechartNativeActionState.Idle -> when {
            event is StatechartNativeActionEvent.Fragment.Received -> {
                // SCE-MAP: statechart_native_action.scxml:42 :: idle :: _transition_0

            // W3C SCXML G.7: <sce:action name="append_fragment_payload">
            pendingFragmentReceivedPayload?.let { actions.appendFragmentPayload(it.payload, it.offset) } ?: run { raiseInternal(StatechartNativeActionEvent.Error.Execution, EventMetadata(data = "<sce:action name='append_fragment_payload'> needs the typed payload of 'fragment.received', which this delivery did not carry", type = "platform")) }
            }
            event is StatechartNativeActionEvent.Selftest -> {
                // SCE-MAP: statechart_native_action.scxml:48 :: idle :: _transition_1

            raiseInternal(StatechartNativeActionEvent.Fragment.Received)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
