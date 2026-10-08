// SCE-GENERATED — DO NOT EDIT
// source-hash: 596ac4b1afa5b66720218d6a44ed9e9093342daa23116ea6b40585ea85ed56ab

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_send_event.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_send_event.scxml:16 :: _machine

package com.sce.integration.static_send_event

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticSendEventState : State {
    data object Idle : StaticSendEventState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticSendEventEvent : Event {
    data object Blank : StaticSendEventEvent
    sealed interface Error : StaticSendEventEvent {
        data object Execution : Error
    }
    data object Go : StaticSendEventEvent
    data object Overflow : StaticSendEventEvent
    data object Ping : StaticSendEventEvent
    data object Pong : StaticSendEventEvent
    sealed interface Use : StaticSendEventEvent {
        data object Ping : Use
        data object Pong : Use
    }
}
// --- State Machine (W3C SCXML) ---

class StaticSendEventStateMachine(
) : StateMachineEngine<StaticSendEventState, StaticSendEventEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `name` datamodel variable, the machine's own. */
    private var name: String = "ping"
    /** W3C SCXML 5.2: the `scale` datamodel variable, the machine's own. */
    private var scale: UInt = 3.toUInt()
    /** W3C SCXML 5.2: the `pings` datamodel variable, published (`sce:direction="out"`). */
    var pings: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `pongs` datamodel variable, published (`sce:direction="out"`). */
    var pongs: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    var refusals: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var name: String? = null
        var scale: UInt? = null
        var pings: UInt? = null
        var pongs: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.name?.let { name = it }
        params.scale?.let { scale = it }
        params.pings?.let { pings = it }
        params.pongs?.let { pongs = it }
        params.refusals?.let { refusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val pings: UInt,
        val pongs: UInt,
        val refusals: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticSendEventState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        pings = pings,
        pongs = pongs,
        refusals = refusals,
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
    val savedShape: String = "3807dd2a8e853bc086919f654e88cd772085d40587e90de3cf30cd769e4ab373"

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
            "name" to SavedValues.of(name),
            "scale" to SavedValues.of(scale),
            "pings" to SavedValues.of(pings),
            "pongs" to SavedValues.of(pongs),
            "refusals" to SavedValues.of(refusals),
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
        val saved1 = SavedValues.string(saved.variable("name"), "name", 16)
        val saved2 = SavedValues.uint32(saved.variable("scale"), "scale")
        val saved3 = SavedValues.uint32(saved.variable("pings"), "pings")
        val saved4 = SavedValues.uint32(saved.variable("pongs"), "pongs")
        val saved5 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        name = saved1
        scale = saved2
        pings = saved3
        pongs = saved4
        refusals = saved5
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticSendEventState = StaticSendEventState.Idle

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

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticSendEventState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticSendEventState, HistoryId>> =
            listOf(StateTarget(StaticSendEventState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticSendEventState, HistoryId>(
            StaticSendEventState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticSendEventState, HistoryId>(
            StaticSendEventState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticSendEventState, HistoryId>(
            StaticSendEventState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticSendEventState, HistoryId>(
            StaticSendEventState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticSendEventState, HistoryId>(
            StaticSendEventState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<StaticSendEventState, HistoryId>(
            StaticSendEventState.Idle,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 6, as the microstep reads it.
        val transitionIdleAt6 = EnabledTransition<StaticSendEventState, HistoryId>(
            StaticSendEventState.Idle,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 7, as the microstep reads it.
        val transitionIdleAt7 = EnabledTransition<StaticSendEventState, HistoryId>(
            StaticSendEventState.Idle,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticSendEventState? = when (stateId) {
        "idle" -> StaticSendEventState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticSendEventState): String = when (state) {
        is StaticSendEventState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticSendEventState): Int = when (state) {
        is StaticSendEventState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticSendEventEvent? = when (name) {
        "blank" -> StaticSendEventEvent.Blank
        "error.execution" -> StaticSendEventEvent.Error.Execution
        "go" -> StaticSendEventEvent.Go
        "overflow" -> StaticSendEventEvent.Overflow
        "ping" -> StaticSendEventEvent.Ping
        "pong" -> StaticSendEventEvent.Pong
        "use.ping" -> StaticSendEventEvent.Use.Ping
        "use.pong" -> StaticSendEventEvent.Use.Pong
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticSendEventEvent): String? = when (event) {
        is StaticSendEventEvent.Blank -> "blank"
        is StaticSendEventEvent.Error.Execution -> "error.execution"
        is StaticSendEventEvent.Go -> "go"
        is StaticSendEventEvent.Overflow -> "overflow"
        is StaticSendEventEvent.Ping -> "ping"
        is StaticSendEventEvent.Pong -> "pong"
        is StaticSendEventEvent.Use.Ping -> "use.ping"
        is StaticSendEventEvent.Use.Pong -> "use.pong"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticSendEventState,
        event: StaticSendEventEvent?
    ): EnabledTransition<StaticSendEventState, HistoryId>? = when (state) {
        is StaticSendEventState.Idle -> when {
            event is StaticSendEventEvent.Go -> transitionIdleAt0
            event is StaticSendEventEvent.Overflow -> transitionIdleAt1
            event is StaticSendEventEvent.Use.Pong -> transitionIdleAt2
            event is StaticSendEventEvent.Use.Ping -> transitionIdleAt3
            event is StaticSendEventEvent.Blank -> transitionIdleAt4
            event is StaticSendEventEvent.Ping -> transitionIdleAt5
            event is StaticSendEventEvent.Pong -> transitionIdleAt6
            event is StaticSendEventEvent.Error.Execution -> transitionIdleAt7
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_send_event.scxml:16 :: _machine
    override fun onEntry(state: StaticSendEventState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticSendEventState.Idle -> {
                // SCE-MAP: static_send_event.scxml:25 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_send_event.scxml:16 :: _machine
    override fun onExit(state: StaticSendEventState) {
        when (state) {
            is StaticSendEventState.Idle -> {
                // SCE-MAP: static_send_event.scxml:25 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_send_event.scxml:16 :: _machine
    override fun executeTransitionContent(source: StaticSendEventState, transitionIndex: Int) {
        when (source) {
        is StaticSendEventState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_send_event.scxml:26 :: idle :: _transition_0


            if (run send@{
            // The event's name is a string computed from the machine's fields now.
            // A name that evaluates to nothing names no event, and an operation
            // that fails cannot be evaluated: either is the argument error.
            val sendEventName: String = name
            if (sendEventName.isEmpty()) {
                raisePlatformError(StaticSendEventEvent.Error.Execution, "<send> eventexpr could not be evaluated to an event name", "__send_0")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 3.12.1 + 5.10: a computed name is delivered as the event
            // the document's names resolve it to (its own, the longest token
            // prefix of it the document writes, or its wildcard), and `_event.name`
            // is the whole name when the member is not called that.
            val sendEvent = resolveArrivingEvent(sendEventName)
            // W3C SCXML 6.2: send to this session's external queue
            if (sendEvent != null) send(sendEvent, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData).copy(name = arrivalNameOf(sendEvent, sendEventName)))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: static_send_event.scxml:29 :: idle :: _transition_1


            if (run send@{
            // The event's name is a string computed from the machine's fields now.
            // A name that evaluates to nothing names no event, and an operation
            // that fails cannot be evaluated: either is the argument error.
            val sendEventName: String = try {
                if (com.sce.forge.runtime.SceChecked.mul(scale, 2000000000.toUInt()) > 1.toUInt()) "big" else "small"
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                ""
            }
            if (sendEventName.isEmpty()) {
                raisePlatformError(StaticSendEventEvent.Error.Execution, "<send> eventexpr could not be evaluated to an event name", "__send_1")
                return@send true
            }
            val sendData = ""
            // W3C SCXML 3.12.1 + 5.10: a computed name is delivered as the event
            // the document's names resolve it to (its own, the longest token
            // prefix of it the document writes, or its wildcard), and `_event.name`
            // is the whole name when the member is not called that.
            val sendEvent = resolveArrivingEvent(sendEventName)
            // W3C SCXML 6.2: send to this session's external queue
            if (sendEvent != null) send(sendEvent, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData).copy(name = arrivalNameOf(sendEvent, sendEventName)))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            2 -> {
                // SCE-MAP: static_send_event.scxml:32 :: idle :: _transition_2

            if (try { name = com.sce.forge.runtime.SceChecked.bounded("pong", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendEventEvent.Error.Execution, "<assign location='name'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_send_event.scxml:35 :: idle :: _transition_3

            if (try { name = com.sce.forge.runtime.SceChecked.bounded("ping", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendEventEvent.Error.Execution, "<assign location='name'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_send_event.scxml:38 :: idle :: _transition_4

            if (try { name = com.sce.forge.runtime.SceChecked.bounded("", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendEventEvent.Error.Execution, "<assign location='name'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            5 -> {
                // SCE-MAP: static_send_event.scxml:41 :: idle :: _transition_5

            if (try { pings = com.sce.forge.runtime.SceChecked.add(pings, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendEventEvent.Error.Execution, "<assign location='pings'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            6 -> {
                // SCE-MAP: static_send_event.scxml:44 :: idle :: _transition_6

            if (try { pongs = com.sce.forge.runtime.SceChecked.add(pongs, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendEventEvent.Error.Execution, "<assign location='pongs'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            7 -> {
                // SCE-MAP: static_send_event.scxml:47 :: idle :: _transition_7

            if (try { refusals = com.sce.forge.runtime.SceChecked.add(refusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendEventEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
