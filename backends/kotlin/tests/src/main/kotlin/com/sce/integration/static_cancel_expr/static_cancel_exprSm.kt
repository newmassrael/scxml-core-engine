// SCE-GENERATED — DO NOT EDIT
// source-hash: ee37533f857b01b43223ccdf57f0449c0b326ab00d940ee5b3e09fb6a557b07f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_cancel_expr.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_cancel_expr.scxml:19 :: _machine

package com.sce.integration.static_cancel_expr

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticCancelExprState : State {
    data object Idle : StaticCancelExprState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticCancelExprEvent : Event {
    sealed interface A : StaticCancelExprEvent {
        data object Due : A
    }
    data object Arm : StaticCancelExprEvent
    sealed interface B : StaticCancelExprEvent {
        data object Due : B
    }
    data object Cancel : StaticCancelExprEvent
    sealed interface Error : StaticCancelExprEvent {
        data object Execution : Error
    }
    data object Overflow : StaticCancelExprEvent
    sealed interface Pick : StaticCancelExprEvent {
        data object B : Pick
        data object None : Pick
    }
}
// --- State Machine (W3C SCXML) ---

class StaticCancelExprStateMachine(
) : StateMachineEngine<StaticCancelExprState, StaticCancelExprEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `which` datamodel variable, the machine's own. */
    private var which: String = "a"
    /** W3C SCXML 5.2: the `scale` datamodel variable, the machine's own. */
    private var scale: UInt = 3.toUInt()
    /** W3C SCXML 5.2: the `a_fired` datamodel variable, published (`sce:direction="out"`). */
    var aFired: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `b_fired` datamodel variable, published (`sce:direction="out"`). */
    var bFired: UInt = 0.toUInt()
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
        var which: String? = null
        var scale: UInt? = null
        var aFired: UInt? = null
        var bFired: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.which?.let { which = it }
        params.scale?.let { scale = it }
        params.aFired?.let { aFired = it }
        params.bFired?.let { bFired = it }
        params.refusals?.let { refusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val aFired: UInt,
        val bFired: UInt,
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
        val configuration: Set<StaticCancelExprState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        aFired = aFired,
        bFired = bFired,
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
    val savedShape: String = "3931ee52e0ca82c24127717dc9133799a7f0ac200b2fbd10b7fb89b77838dc7f"

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
            "which" to SavedValues.of(which),
            "scale" to SavedValues.of(scale),
            "a_fired" to SavedValues.of(aFired),
            "b_fired" to SavedValues.of(bFired),
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
        val saved1 = SavedValues.string(saved.variable("which"), "which", 8)
        val saved2 = SavedValues.uint32(saved.variable("scale"), "scale")
        val saved3 = SavedValues.uint32(saved.variable("a_fired"), "a_fired")
        val saved4 = SavedValues.uint32(saved.variable("b_fired"), "b_fired")
        val saved5 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        which = saved1
        scale = saved2
        aFired = saved3
        bFired = saved4
        refusals = saved5
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticCancelExprState = StaticCancelExprState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticCancelExprState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticCancelExprState, HistoryId>> =
            listOf(StateTarget(StaticCancelExprState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticCancelExprState, HistoryId>(
            StaticCancelExprState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticCancelExprState, HistoryId>(
            StaticCancelExprState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticCancelExprState, HistoryId>(
            StaticCancelExprState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticCancelExprState, HistoryId>(
            StaticCancelExprState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticCancelExprState, HistoryId>(
            StaticCancelExprState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<StaticCancelExprState, HistoryId>(
            StaticCancelExprState.Idle,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 6, as the microstep reads it.
        val transitionIdleAt6 = EnabledTransition<StaticCancelExprState, HistoryId>(
            StaticCancelExprState.Idle,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 7, as the microstep reads it.
        val transitionIdleAt7 = EnabledTransition<StaticCancelExprState, HistoryId>(
            StaticCancelExprState.Idle,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticCancelExprState? = when (stateId) {
        "idle" -> StaticCancelExprState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticCancelExprState): String = when (state) {
        is StaticCancelExprState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticCancelExprState): Int = when (state) {
        is StaticCancelExprState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticCancelExprEvent? = when (name) {
        "a.due" -> StaticCancelExprEvent.A.Due
        "arm" -> StaticCancelExprEvent.Arm
        "b.due" -> StaticCancelExprEvent.B.Due
        "cancel" -> StaticCancelExprEvent.Cancel
        "error.execution" -> StaticCancelExprEvent.Error.Execution
        "overflow" -> StaticCancelExprEvent.Overflow
        "pick.b" -> StaticCancelExprEvent.Pick.B
        "pick.none" -> StaticCancelExprEvent.Pick.None
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticCancelExprEvent): String? = when (event) {
        is StaticCancelExprEvent.A.Due -> "a.due"
        is StaticCancelExprEvent.Arm -> "arm"
        is StaticCancelExprEvent.B.Due -> "b.due"
        is StaticCancelExprEvent.Cancel -> "cancel"
        is StaticCancelExprEvent.Error.Execution -> "error.execution"
        is StaticCancelExprEvent.Overflow -> "overflow"
        is StaticCancelExprEvent.Pick.B -> "pick.b"
        is StaticCancelExprEvent.Pick.None -> "pick.none"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticCancelExprState,
        event: StaticCancelExprEvent?
    ): EnabledTransition<StaticCancelExprState, HistoryId>? = when (state) {
        is StaticCancelExprState.Idle -> when {
            event is StaticCancelExprEvent.Arm -> transitionIdleAt0
            event is StaticCancelExprEvent.Cancel -> transitionIdleAt1
            event is StaticCancelExprEvent.Overflow -> transitionIdleAt2
            event is StaticCancelExprEvent.Pick.B -> transitionIdleAt3
            event is StaticCancelExprEvent.Pick.None -> transitionIdleAt4
            event is StaticCancelExprEvent.A.Due -> transitionIdleAt5
            event is StaticCancelExprEvent.B.Due -> transitionIdleAt6
            event is StaticCancelExprEvent.Error.Execution -> transitionIdleAt7
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_cancel_expr.scxml:19 :: _machine
    override fun onEntry(state: StaticCancelExprState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticCancelExprState.Idle -> {
                // SCE-MAP: static_cancel_expr.scxml:28 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_cancel_expr.scxml:19 :: _machine
    override fun onExit(state: StaticCancelExprState) {
        when (state) {
            is StaticCancelExprState.Idle -> {
                // SCE-MAP: static_cancel_expr.scxml:28 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_cancel_expr.scxml:19 :: _machine
    override fun executeTransitionContent(source: StaticCancelExprState, transitionIndex: Int) {
        when (source) {
        is StaticCancelExprState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_cancel_expr.scxml:29 :: idle :: _transition_0


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("a", 200L, StaticCancelExprEvent.A.Due, EventMetadata.external(sendId = "a", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("b", 200L, StaticCancelExprEvent.B.Due, EventMetadata.external(sendId = "b", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: static_cancel_expr.scxml:33 :: idle :: _transition_1


            // `datamodel="sce-static"`: the id is a string computed from the
            // machine's fields now (`Action::native_sendid`); no script engine is
            // asked. An id no send holds cancels nothing; an operation that fails
            // cannot be evaluated, which raises error.execution and ends the block.
            cancelSend(which)
            }
            2 -> {
                // SCE-MAP: static_cancel_expr.scxml:36 :: idle :: _transition_2


            // `datamodel="sce-static"`: the id is a string computed from the
            // machine's fields now (`Action::native_sendid`); no script engine is
            // asked. An id no send holds cancels nothing; an operation that fails
            // cannot be evaluated, which raises error.execution and ends the block.
            if (run cancel@{
                val sendidToCancel: String? = try {
                    if (com.sce.forge.runtime.SceChecked.mul(scale, 2000000000.toUInt()) > 1.toUInt()) "a" else "b"
                } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                    null
                }
                if (sendidToCancel == null) {
                    raisePlatformError(StaticCancelExprEvent.Error.Execution, "<cancel> sendidexpr failed to evaluate")
                    true
                } else {
                    cancelSend(sendidToCancel)
                    false
                }
            }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_cancel_expr.scxml:39 :: idle :: _transition_3

            if (try { which = com.sce.forge.runtime.SceChecked.bounded("b", 8); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticCancelExprEvent.Error.Execution, "<assign location='which'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_cancel_expr.scxml:42 :: idle :: _transition_4

            if (try { which = com.sce.forge.runtime.SceChecked.bounded("", 8); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticCancelExprEvent.Error.Execution, "<assign location='which'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            5 -> {
                // SCE-MAP: static_cancel_expr.scxml:45 :: idle :: _transition_5

            if (try { aFired = com.sce.forge.runtime.SceChecked.add(aFired, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticCancelExprEvent.Error.Execution, "<assign location='a_fired'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            6 -> {
                // SCE-MAP: static_cancel_expr.scxml:48 :: idle :: _transition_6

            if (try { bFired = com.sce.forge.runtime.SceChecked.add(bFired, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticCancelExprEvent.Error.Execution, "<assign location='b_fired'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            7 -> {
                // SCE-MAP: static_cancel_expr.scxml:51 :: idle :: _transition_7

            if (try { refusals = com.sce.forge.runtime.SceChecked.add(refusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticCancelExprEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
