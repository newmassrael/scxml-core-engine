// SCE-GENERATED — DO NOT EDIT
// source-hash: dd128732f0d0cc6bb147aae13f97e39edf9438b451af2c8f42fc5cd000534f68

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_send_idlocation.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_send_idlocation.scxml:25 :: _machine

package com.sce.integration.static_send_idlocation

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticSendIdlocationState : State {
    data object Idle : StaticSendIdlocationState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticSendIdlocationEvent : Event {
    data object Arm : StaticSendIdlocationEvent
    data object Armfail : StaticSendIdlocationEvent
    sealed interface Cancel : StaticSendIdlocationEvent {
        data object First : Cancel
        data object Second : Cancel
    }
    sealed interface Error : StaticSendIdlocationEvent {
        data object Execution : Error
    }
    sealed interface First : StaticSendIdlocationEvent {
        data object Due : First
    }
    sealed interface Second : StaticSendIdlocationEvent {
        data object Due : Second
    }
}
// --- State Machine (W3C SCXML) ---

class StaticSendIdlocationStateMachine(
) : StateMachineEngine<StaticSendIdlocationState, StaticSendIdlocationEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `first` datamodel variable, the machine's own. */
    private var first: String = ""
    /** W3C SCXML 5.2: the `second` datamodel variable, the machine's own. */
    private var second: String = ""
    /** W3C SCXML 5.2: the `scale` datamodel variable, the machine's own. */
    private var scale: UInt = 3.toUInt()
    /** W3C SCXML 5.2: the `first_fired` datamodel variable, published (`sce:direction="out"`). */
    var firstFired: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `second_fired` datamodel variable, published (`sce:direction="out"`). */
    var secondFired: UInt = 0.toUInt()
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
        var first: String? = null
        var second: String? = null
        var scale: UInt? = null
        var firstFired: UInt? = null
        var secondFired: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.first?.let { first = it }
        params.second?.let { second = it }
        params.scale?.let { scale = it }
        params.firstFired?.let { firstFired = it }
        params.secondFired?.let { secondFired = it }
        params.refusals?.let { refusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val firstFired: UInt,
        val secondFired: UInt,
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
        val configuration: Set<StaticSendIdlocationState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        firstFired = firstFired,
        secondFired = secondFired,
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
    val savedShape: String = "9d26aca9bf550afa9c6223d9778d1e41fb770b42e83e11b043af12f5939a6ad5"

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
            "first" to SavedValues.of(first),
            "second" to SavedValues.of(second),
            "scale" to SavedValues.of(scale),
            "first_fired" to SavedValues.of(firstFired),
            "second_fired" to SavedValues.of(secondFired),
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
        val saved1 = SavedValues.string(saved.variable("first"), "first", 32)
        val saved2 = SavedValues.string(saved.variable("second"), "second", 32)
        val saved3 = SavedValues.uint32(saved.variable("scale"), "scale")
        val saved4 = SavedValues.uint32(saved.variable("first_fired"), "first_fired")
        val saved5 = SavedValues.uint32(saved.variable("second_fired"), "second_fired")
        val saved6 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        first = saved1
        second = saved2
        scale = saved3
        firstFired = saved4
        secondFired = saved5
        refusals = saved6
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticSendIdlocationState = StaticSendIdlocationState.Idle

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

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticSendIdlocationState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticSendIdlocationState, HistoryId>> =
            listOf(StateTarget(StaticSendIdlocationState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticSendIdlocationState, HistoryId>(
            StaticSendIdlocationState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticSendIdlocationState, HistoryId>(
            StaticSendIdlocationState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticSendIdlocationState, HistoryId>(
            StaticSendIdlocationState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticSendIdlocationState, HistoryId>(
            StaticSendIdlocationState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticSendIdlocationState, HistoryId>(
            StaticSendIdlocationState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<StaticSendIdlocationState, HistoryId>(
            StaticSendIdlocationState.Idle,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 6, as the microstep reads it.
        val transitionIdleAt6 = EnabledTransition<StaticSendIdlocationState, HistoryId>(
            StaticSendIdlocationState.Idle,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticSendIdlocationState? = when (stateId) {
        "idle" -> StaticSendIdlocationState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticSendIdlocationState): String = when (state) {
        is StaticSendIdlocationState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticSendIdlocationState): Int = when (state) {
        is StaticSendIdlocationState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticSendIdlocationEvent? = when (name) {
        "arm" -> StaticSendIdlocationEvent.Arm
        "armfail" -> StaticSendIdlocationEvent.Armfail
        "cancel.first" -> StaticSendIdlocationEvent.Cancel.First
        "cancel.second" -> StaticSendIdlocationEvent.Cancel.Second
        "error.execution" -> StaticSendIdlocationEvent.Error.Execution
        "first.due" -> StaticSendIdlocationEvent.First.Due
        "second.due" -> StaticSendIdlocationEvent.Second.Due
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticSendIdlocationEvent): String? = when (event) {
        is StaticSendIdlocationEvent.Arm -> "arm"
        is StaticSendIdlocationEvent.Armfail -> "armfail"
        is StaticSendIdlocationEvent.Cancel.First -> "cancel.first"
        is StaticSendIdlocationEvent.Cancel.Second -> "cancel.second"
        is StaticSendIdlocationEvent.Error.Execution -> "error.execution"
        is StaticSendIdlocationEvent.First.Due -> "first.due"
        is StaticSendIdlocationEvent.Second.Due -> "second.due"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticSendIdlocationState,
        event: StaticSendIdlocationEvent?
    ): EnabledTransition<StaticSendIdlocationState, HistoryId>? = when (state) {
        is StaticSendIdlocationState.Idle -> when {
            event is StaticSendIdlocationEvent.Arm -> transitionIdleAt0
            event is StaticSendIdlocationEvent.Armfail -> transitionIdleAt1
            event is StaticSendIdlocationEvent.Cancel.First -> transitionIdleAt2
            event is StaticSendIdlocationEvent.Cancel.Second -> transitionIdleAt3
            event is StaticSendIdlocationEvent.First.Due -> transitionIdleAt4
            event is StaticSendIdlocationEvent.Second.Due -> transitionIdleAt5
            event is StaticSendIdlocationEvent.Error.Execution -> transitionIdleAt6
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_send_idlocation.scxml:25 :: _machine
    override fun onEntry(state: StaticSendIdlocationState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticSendIdlocationState.Idle -> {
                // SCE-MAP: static_send_idlocation.scxml:35 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_send_idlocation.scxml:25 :: _machine
    override fun onExit(state: StaticSendIdlocationState) {
        when (state) {
            is StaticSendIdlocationState.Idle -> {
                // SCE-MAP: static_send_idlocation.scxml:35 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_send_idlocation.scxml:25 :: _machine
    override fun executeTransitionContent(source: StaticSendIdlocationState, transitionIndex: Int) {
        when (source) {
        is StaticSendIdlocationState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_send_idlocation.scxml:36 :: idle :: _transition_0


            if (run send@{
            // W3C SCXML 6.2.4: under `datamodel="sce-static"` the machine generates
            // the id and writes it to the variable `idlocation` names before it
            // reads any other argument (`Action::native_idlocation`); that variable
            // is then the id the send is known by (`Action::native_sendid`).
            first = nextAutoSendId()
            val sendIdGenerated: String = first
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend(sendIdGenerated, 200L, StaticSendIdlocationEvent.First.Due, EventMetadata.external(sendId = sendIdGenerated, origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            // W3C SCXML 6.2.4: under `datamodel="sce-static"` the machine generates
            // the id and writes it to the variable `idlocation` names before it
            // reads any other argument (`Action::native_idlocation`); that variable
            // is then the id the send is known by (`Action::native_sendid`).
            second = nextAutoSendId()
            val sendIdGenerated: String = second
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend(sendIdGenerated, 200L, StaticSendIdlocationEvent.Second.Due, EventMetadata.external(sendId = sendIdGenerated, origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: static_send_idlocation.scxml:40 :: idle :: _transition_1


            if (run send@{
            // W3C SCXML 6.2.4: under `datamodel="sce-static"` the machine generates
            // the id and writes it to the variable `idlocation` names before it
            // reads any other argument (`Action::native_idlocation`); that variable
            // is then the id the send is known by (`Action::native_sendid`).
            first = nextAutoSendId()
            val sendIdGenerated: String = first
            // The delay is a string computed from the machine's fields now, and
            // read as the CSS2 time it must be (ARCHITECTURE.md, "Durations"):
            // an operation that fails, or a value that is no time, is the
            // argument error, and the message is not scheduled under some
            // default wait.
            val sendDelayText: String = try {
                if (com.sce.forge.runtime.SceChecked.mul(scale, 2000000000.toUInt()) > 1.toUInt()) "200ms" else "300ms"
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(StaticSendIdlocationEvent.Error.Execution, "<send> delayexpr could not be evaluated", sendIdGenerated)
                return@send true
            }
            val sendDelayMs = com.sce.runtime.SendHelper.parseDelayMs(sendDelayText) ?: run {
                raisePlatformError(StaticSendIdlocationEvent.Error.Execution, "<send> delayexpr is not a CSS2 time", sendIdGenerated)
                return@send true
            }
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend(sendIdGenerated, sendDelayMs, StaticSendIdlocationEvent.First.Due, EventMetadata.external(sendId = sendIdGenerated, origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            2 -> {
                // SCE-MAP: static_send_idlocation.scxml:44 :: idle :: _transition_2


            // `datamodel="sce-static"`: the id is a string computed from the
            // machine's fields now (`Action::native_sendid`); no script engine is
            // asked. An id no send holds cancels nothing; an operation that fails
            // cannot be evaluated, which raises error.execution and ends the block.
            cancelSend(first)
            }
            3 -> {
                // SCE-MAP: static_send_idlocation.scxml:47 :: idle :: _transition_3


            // `datamodel="sce-static"`: the id is a string computed from the
            // machine's fields now (`Action::native_sendid`); no script engine is
            // asked. An id no send holds cancels nothing; an operation that fails
            // cannot be evaluated, which raises error.execution and ends the block.
            cancelSend(second)
            }
            4 -> {
                // SCE-MAP: static_send_idlocation.scxml:50 :: idle :: _transition_4

            if (try { firstFired = com.sce.forge.runtime.SceChecked.add(firstFired, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendIdlocationEvent.Error.Execution, "<assign location='first_fired'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            5 -> {
                // SCE-MAP: static_send_idlocation.scxml:53 :: idle :: _transition_5

            if (try { secondFired = com.sce.forge.runtime.SceChecked.add(secondFired, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendIdlocationEvent.Error.Execution, "<assign location='second_fired'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            6 -> {
                // SCE-MAP: static_send_idlocation.scxml:56 :: idle :: _transition_6

            if (try { refusals = com.sce.forge.runtime.SceChecked.add(refusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendIdlocationEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
