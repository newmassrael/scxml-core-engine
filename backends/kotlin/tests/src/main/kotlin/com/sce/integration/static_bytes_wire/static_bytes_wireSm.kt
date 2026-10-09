// SCE-GENERATED — DO NOT EDIT
// source-hash: cb41954d893211ff980559eb7566d5cfca26a8d312a941a8b0426661b4fb8dcd

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_bytes_wire.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_bytes_wire.scxml:15 :: _machine

package com.sce.integration.static_bytes_wire

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticBytesWireState : State {
    data object Done : StaticBytesWireState
    data object Idle : StaticBytesWireState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticBytesWireEvent : Event {
    sealed interface Error : StaticBytesWireEvent {
        data object Execution : Error
    }
    data object Finish : StaticBytesWireEvent
    sealed interface Framed : StaticBytesWireEvent {
        data object Relayed : Framed
        data object Taken : Framed
    }
    data object Relay : StaticBytesWireEvent
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticBytesWirePayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticBytesWireFramedRelayedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `framed.relayed`. Consumers inject it via the `raiseFramedRelayed` seam
// on the machine — they never name this class directly.
data class StaticBytesWireFramedRelayedPayload(val sensor: UByte, val frame: ByteArray)

// StaticBytesWireFramedTakenPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `framed.taken`. Consumers inject it via the `raiseFramedTaken` seam
// on the machine — they never name this class directly.
data class StaticBytesWireFramedTakenPayload(val sensor: UByte, val frame: ByteArray)


// --- State Machine (W3C SCXML) ---

class StaticBytesWireStateMachine(
) : StateMachineEngine<StaticBytesWireState, StaticBytesWireEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `held` datamodel variable, published (`sce:direction="out"`). */
    var held: ByteArray = "ab".toByteArray()
        // A byte string is handed out as a copy: the array is the machine's own,
        // and a host that wrote into it would change the variable behind its bound.
        get() = field.copyOf()
        private set
    /** W3C SCXML 5.2: the `echo` datamodel variable, published (`sce:direction="out"`). */
    var echo: ByteArray = "".toByteArray()
        // A byte string is handed out as a copy: the array is the machine's own,
        // and a host that wrote into it would change the variable behind its bound.
        get() = field.copyOf()
        private set
    /** W3C SCXML 5.2: the `relays` datamodel variable, published (`sce:direction="out"`). */
    var relays: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    var errors: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var relays: UInt? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.relays?.let { relays = it }
        params.errors?.let { errors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val held: ByteArray,
        val echo: ByteArray,
        val relays: UInt,
        val errors: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticBytesWireState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        held = held,
        echo = echo,
        relays = relays,
        errors = errors,
    )

    private val _snapshot = kotlinx.coroutines.flow.MutableStateFlow(
        Snapshot(emptySet(), currentData(), false)
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
        _snapshot.value = Snapshot(activeConfiguration, currentData(), truncated)
    }

    // ── SCE Accepted Subset §2.15: saving this machine, restoring it ─────────

    /**
     * The shape a saved state of this document is bound to: a state saved
     * from a document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not.
     */
    val savedShape: String = "5fad297f365d60541d683880cd6295ab14643581b611eda79152e6085f6ef5da"

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
    fun save(wallNowMs: Long): SavedState = savedState(
        savedShape,
        linkedMapOf(
            "held" to SavedValues.of(held),
            "echo" to SavedValues.of(echo),
            "relays" to SavedValues.of(relays),
            "errors" to SavedValues.of(errors),
        ),
        wallNowMs,
    )

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
        beginRestore(saved, savedShape)
        val saved1 = SavedValues.bytes(saved.variable("held"), "held", 8)
        val saved2 = SavedValues.bytes(saved.variable("echo"), "echo", 8)
        val saved3 = SavedValues.uint32(saved.variable("relays"), "relays")
        val saved4 = SavedValues.uint32(saved.variable("errors"), "errors")
        held = saved1
        echo = saved2
        relays = saved3
        errors = saved4
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingFramedRelayedPayload: StaticBytesWireFramedRelayedPayload? = null
    private var pendingFramedTakenPayload: StaticBytesWireFramedTakenPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticBytesWireEvent, metadata: EventMetadata) {
        pendingFramedRelayedPayload = null
        pendingFramedTakenPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticBytesWireFramedRelayedPayload -> pendingFramedRelayedPayload = tp
            is StaticBytesWireFramedTakenPayload -> pendingFramedTakenPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticBytesWireEvent.Framed.Relayed) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingFramedRelayedPayload = StaticBytesWireFramedRelayedPayload(fields.uint8("sensor"), fields.bytes("frame"))
                } else if (event == StaticBytesWireEvent.Framed.Taken) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingFramedTakenPayload = StaticBytesWireFramedTakenPayload(fields.uint8("sensor"), fields.bytes("frame"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `framed.relayed` — binds the event name and the payload field values in one call.
    fun raiseFramedRelayed(sensor: UByte, frame: ByteArray) {
        send(
            StaticBytesWireEvent.Framed.Relayed,
            EventMetadata(
                type = "external",
                typedPayload = StaticBytesWireFramedRelayedPayload(sensor, frame),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("sensor" to sensor, "frame" to frame))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `framed.taken` — binds the event name and the payload field values in one call.
    fun raiseFramedTaken(sensor: UByte, frame: ByteArray) {
        send(
            StaticBytesWireEvent.Framed.Taken,
            EventMetadata(
                type = "external",
                typedPayload = StaticBytesWireFramedTakenPayload(sensor, frame),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("sensor" to sensor, "frame" to frame))
            )
        )
    }


    override val initialState: StaticBytesWireState = StaticBytesWireState.Idle

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
    override fun isFinalState(state: StaticBytesWireState): Boolean = when (state) {
        is StaticBytesWireState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticBytesWireState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticBytesWireState, HistoryId>> =
            listOf(StateTarget(StaticBytesWireState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticBytesWireState, HistoryId>(
            StaticBytesWireState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticBytesWireState, HistoryId>(
            StaticBytesWireState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticBytesWireState, HistoryId>(
            StaticBytesWireState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticBytesWireState, HistoryId>(
            StaticBytesWireState.Idle,
            listOf(StateTarget(StaticBytesWireState.Done)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticBytesWireState, HistoryId>(
            StaticBytesWireState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticBytesWireState? = when (stateId) {
        "done" -> StaticBytesWireState.Done
        "idle" -> StaticBytesWireState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticBytesWireState): String = when (state) {
        is StaticBytesWireState.Done -> "done"
        is StaticBytesWireState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticBytesWireState): Int = when (state) {
        is StaticBytesWireState.Done -> 1
        is StaticBytesWireState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticBytesWireEvent? = when (name) {
        "error.execution" -> StaticBytesWireEvent.Error.Execution
        "finish" -> StaticBytesWireEvent.Finish
        "framed.relayed" -> StaticBytesWireEvent.Framed.Relayed
        "framed.taken" -> StaticBytesWireEvent.Framed.Taken
        "relay" -> StaticBytesWireEvent.Relay
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticBytesWireEvent): String? = when (event) {
        is StaticBytesWireEvent.Error.Execution -> "error.execution"
        is StaticBytesWireEvent.Finish -> "finish"
        is StaticBytesWireEvent.Framed.Relayed -> "framed.relayed"
        is StaticBytesWireEvent.Framed.Taken -> "framed.taken"
        is StaticBytesWireEvent.Relay -> "relay"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticBytesWireState,
        event: StaticBytesWireEvent?
    ): EnabledTransition<StaticBytesWireState, HistoryId>? = when (state) {
        is StaticBytesWireState.Idle -> when {
            event is StaticBytesWireEvent.Framed.Taken -> transitionIdleAt0
            event is StaticBytesWireEvent.Relay -> transitionIdleAt1
            event is StaticBytesWireEvent.Framed.Relayed -> transitionIdleAt2
            event is StaticBytesWireEvent.Finish -> transitionIdleAt3
            event is StaticBytesWireEvent.Error.Execution -> transitionIdleAt4
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_bytes_wire.scxml:15 :: _machine
    override fun onEntry(state: StaticBytesWireState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticBytesWireState.Done -> {
                // SCE-MAP: static_bytes_wire.scxml:44 :: done :: _state_body
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    var doneEventData = ""
                    // W3C SCXML 5.5: Evaluate <param> elements (C++ DoneDataHelper::evaluateParams pattern)
                    val doneParams = mutableMapOf<String, Any?>()
                    doneParams["frame"] = com.sce.runtime.EventPayload.bytesAsText(echo)
                    // §scxml-5.5: the pairs that survived, `{}` when none did
                    // (C++ DoneDataHelper::evaluateParams). Not left to
                    // buildJsonFromParams, whose empty answer is a <send>'s.
                    doneEventData = if (doneParams.isEmpty()) "{}" else buildJsonFromParams(doneParams)
                    // W3C SCXML 5.5 + 6.3.1: stash onto the engine so the invoking parent's
                    // startInvoke completion callback can lift the payload onto
                    // done.invoke.<id>._event.data. Mirrors C++ AOT stashDonedataAtFinal.
                    stashDonedataAtFinal(doneEventData)
                }
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StaticBytesWireState.Idle -> {
                // SCE-MAP: static_bytes_wire.scxml:25 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_bytes_wire.scxml:15 :: _machine
    override fun onExit(state: StaticBytesWireState) {
        when (state) {
            is StaticBytesWireState.Done -> {
                // SCE-MAP: static_bytes_wire.scxml:44 :: done :: _state_body
            }
            is StaticBytesWireState.Idle -> {
                // SCE-MAP: static_bytes_wire.scxml:25 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_bytes_wire.scxml:15 :: _machine
    override fun executeTransitionContent(source: StaticBytesWireState, transitionIndex: Int) {
        when (source) {
        is StaticBytesWireState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_bytes_wire.scxml:26 :: idle :: _transition_0
                if (pendingFramedTakenPayload == null) {
                    return
                }

            if (try { held = com.sce.forge.runtime.SceChecked.bounded(pendingFramedTakenPayload!!.frame, 8); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBytesWireEvent.Error.Execution, "<assign location='held'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_bytes_wire.scxml:29 :: idle :: _transition_1


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "sensor", (3).toLong())

            putParam(sendPayload, "frame", com.sce.runtime.EventPayload.bytesAsText(held))

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticBytesWireEvent.Framed.Relayed, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            2 -> {
                // SCE-MAP: static_bytes_wire.scxml:35 :: idle :: _transition_2
                if (pendingFramedRelayedPayload == null) {
                    return
                }

            if (try { echo = com.sce.forge.runtime.SceChecked.bounded(pendingFramedRelayedPayload!!.frame, 8); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBytesWireEvent.Error.Execution, "<assign location='echo'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { relays = com.sce.forge.runtime.SceChecked.add(relays, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBytesWireEvent.Error.Execution, "<assign location='relays'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_bytes_wire.scxml:40 :: idle :: _transition_4

            if (try { errors = com.sce.forge.runtime.SceChecked.add(errors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBytesWireEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
