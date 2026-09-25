// SCE-GENERATED — DO NOT EDIT
// source-hash: ca5f07e498f08e9c44fe0c543fc369f243205362af8b0e260e1a44e3bfa1bd0d

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

/**
 * [StatechartNativeActionActions] that performs nothing and records every call in order —
 * the host a test drives the machine with. Read [calls] after the machine
 * has run; each call is compared by value.
 */
class RecordingStatechartNativeActionActions : StatechartNativeActionActions {
    /** One recorded host call. */
    sealed interface Call {
        data class AppendFragmentPayload(val payload: List<Byte>, val offset: UInt) : Call
        data object OnAssemblingExit : Call
        data object OnIdleEntry : Call
        data object ResetSlot : Call
    }

    private val recorded = mutableListOf<Call>()

    /** Every call so far, oldest first. */
    val calls: List<Call>
        get() = recorded.toList()

    /** Forget the calls recorded so far. */
    fun clear() {
        recorded.clear()
    }

    override fun appendFragmentPayload(payload: ByteArray, offset: UInt) {
        recorded += Call.AppendFragmentPayload(payload.toList(), offset)
    }
    override fun onAssemblingExit() {
        recorded += Call.OnAssemblingExit
    }
    override fun onIdleEntry() {
        recorded += Call.OnIdleEntry
    }
    override fun resetSlot() {
        recorded += Call.ResetSlot
    }
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

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StatechartNativeActionEvent, metadata: EventMetadata) {
        pendingFragmentReceivedPayload = null
        when (val tp = metadata.typedPayload) {
            is StatechartNativeActionFragmentReceivedPayload -> pendingFragmentReceivedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StatechartNativeActionEvent.Fragment.Received) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingFragmentReceivedPayload = StatechartNativeActionFragmentReceivedPayload(fields.bytes("payload"), fields.uint32("offset"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `fragment.received` — binds the event name and the payload field values in one call.
    fun raiseFragmentReceived(payload: ByteArray, offset: UInt) {
        send(
            StatechartNativeActionEvent.Fragment.Received,
            EventMetadata(
                type = "external",
                typedPayload = StatechartNativeActionFragmentReceivedPayload(payload, offset),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("payload" to payload, "offset" to offset))
            )
        )
    }


    override val initialState: StatechartNativeActionState = StatechartNativeActionState.Idle

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StatechartNativeActionState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StatechartNativeActionState, HistoryId>> =
            listOf(StateTarget(StatechartNativeActionState.Idle))

        // W3C SCXML 3.13: assembling's transition 0, as the microstep reads it.
        val transitionAssemblingAt0 = EnabledTransition<StatechartNativeActionState, HistoryId>(
            StatechartNativeActionState.Assembling,
            listOf(StateTarget(StatechartNativeActionState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: assembling's transition 1, as the microstep reads it.
        val transitionAssemblingAt1 = EnabledTransition<StatechartNativeActionState, HistoryId>(
            StatechartNativeActionState.Assembling,
            listOf(StateTarget(StatechartNativeActionState.Faulted)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: faulted's transition 0, as the microstep reads it.
        val transitionFaultedAt0 = EnabledTransition<StatechartNativeActionState, HistoryId>(
            StatechartNativeActionState.Faulted,
            listOf(StateTarget(StatechartNativeActionState.Idle)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StatechartNativeActionState, HistoryId>(
            StatechartNativeActionState.Idle,
            listOf(StateTarget(StatechartNativeActionState.Assembling)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StatechartNativeActionState, HistoryId>(
            StatechartNativeActionState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )
    }

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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StatechartNativeActionState): Int = when (state) {
        is StatechartNativeActionState.Assembling -> 1
        is StatechartNativeActionState.Faulted -> 2
        is StatechartNativeActionState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartNativeActionEvent? = when (name) {
        "error.execution" -> StatechartNativeActionEvent.Error.Execution
        "fragment.received" -> StatechartNativeActionEvent.Fragment.Received
        "reset" -> StatechartNativeActionEvent.Reset
        "selftest" -> StatechartNativeActionEvent.Selftest
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartNativeActionEvent): String? = when (event) {
        is StatechartNativeActionEvent.Error.Execution -> "error.execution"
        is StatechartNativeActionEvent.Fragment.Received -> "fragment.received"
        is StatechartNativeActionEvent.Reset -> "reset"
        is StatechartNativeActionEvent.Selftest -> "selftest"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StatechartNativeActionState,
        event: StatechartNativeActionEvent?
    ): EnabledTransition<StatechartNativeActionState, HistoryId>? = when (state) {
        is StatechartNativeActionState.Assembling -> when {
            event is StatechartNativeActionEvent.Reset -> transitionAssemblingAt0
            event is StatechartNativeActionEvent.Error.Execution -> transitionAssemblingAt1
            else -> null
        }
        is StatechartNativeActionState.Faulted -> when {
            event is StatechartNativeActionEvent.Reset -> transitionFaultedAt0
            else -> null
        }
        is StatechartNativeActionState.Idle -> when {
            event is StatechartNativeActionEvent.Fragment.Received -> transitionIdleAt0
            event is StatechartNativeActionEvent.Selftest -> transitionIdleAt1
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_native_action.scxml:31 :: _machine
    override fun onEntry(state: StatechartNativeActionState, isDefaultEntry: Boolean) {
        when (state) {
            is StatechartNativeActionState.Assembling -> {
                // SCE-MAP: statechart_native_action.scxml:59 :: assembling :: _state_body
            }
            is StatechartNativeActionState.Faulted -> {
                // SCE-MAP: statechart_native_action.scxml:71 :: faulted :: _state_body
            }
            is StatechartNativeActionState.Idle -> {
                // SCE-MAP: statechart_native_action.scxml:38 :: idle :: _state_body

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
                // SCE-MAP: statechart_native_action.scxml:59 :: assembling :: _state_body

            // W3C SCXML G.7: <sce:action name="on_assembling_exit">
            actions.onAssemblingExit()
            }
            is StatechartNativeActionState.Faulted -> {
                // SCE-MAP: statechart_native_action.scxml:71 :: faulted :: _state_body
            }
            is StatechartNativeActionState.Idle -> {
                // SCE-MAP: statechart_native_action.scxml:38 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: statechart_native_action.scxml:31 :: _machine
    override fun executeTransitionContent(source: StatechartNativeActionState, transitionIndex: Int) {
        when (source) {
        is StatechartNativeActionState.Assembling -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_native_action.scxml:63 :: assembling :: _transition_0

            // W3C SCXML G.7: <sce:action name="reset_slot">
            actions.resetSlot()
            }
            else -> {}
        }
        is StatechartNativeActionState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_native_action.scxml:42 :: idle :: _transition_0

            // W3C SCXML G.7: <sce:action name="append_fragment_payload">
            pendingFragmentReceivedPayload?.let { actions.appendFragmentPayload(it.payload, it.offset) } ?: run { raiseInternal(StatechartNativeActionEvent.Error.Execution, EventMetadata(data = "<sce:action name='append_fragment_payload'> needs the typed payload of 'fragment.received', which this delivery did not carry", type = "platform")) }
            }
            1 -> {
                // SCE-MAP: statechart_native_action.scxml:55 :: idle :: _transition_1

            raiseInternal(StatechartNativeActionEvent.Fragment.Received)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
