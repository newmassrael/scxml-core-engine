// SCE-GENERATED — DO NOT EDIT
// source-hash: cb41954d893211ff980559eb7566d5cfca26a8d312a941a8b0426661b4fb8dcd

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_send_type.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_send_type.scxml:17 :: _machine

package com.sce.integration.static_send_type

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticSendTypeState : State {
    data object Idle : StaticSendTypeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticSendTypeEvent : Event {
    sealed interface Aim : StaticSendTypeEvent {
        data object Bogus : Aim
        data object Http : Aim
        data object Scxml : Aim
    }
    sealed interface Error : StaticSendTypeEvent {
        data object Execution : Error
    }
    data object Fire : StaticSendTypeEvent
    data object Landed : StaticSendTypeEvent
}
// --- State Machine (W3C SCXML) ---

class StaticSendTypeStateMachine(
) : StateMachineEngine<StaticSendTypeState, StaticSendTypeEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `kind` datamodel variable, the machine's own. */
    private var kind: String = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"
    /** W3C SCXML 5.2: the `landed` datamodel variable, published (`sce:direction="out"`). */
    var landed: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `refused` datamodel variable, published (`sce:direction="out"`). */
    var refused: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var kind: String? = null
        var landed: UInt? = null
        var refused: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.kind?.let { kind = it }
        params.landed?.let { landed = it }
        params.refused?.let { refused = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val landed: UInt,
        val refused: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticSendTypeState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        landed = landed,
        refused = refused,
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
    val savedShape: String = "59c0b29c6b354c06d9c7e5411b951a189a3ab82fbb80098ee96cfed5053c9bb0"

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
            "kind" to SavedValues.of(kind),
            "landed" to SavedValues.of(landed),
            "refused" to SavedValues.of(refused),
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
        val saved1 = SavedValues.string(saved.variable("kind"), "kind", 64)
        val saved2 = SavedValues.uint32(saved.variable("landed"), "landed")
        val saved3 = SavedValues.uint32(saved.variable("refused"), "refused")
        kind = saved1
        landed = saved2
        refused = saved3
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticSendTypeState = StaticSendTypeState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticSendTypeState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticSendTypeState, HistoryId>> =
            listOf(StateTarget(StaticSendTypeState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticSendTypeState, HistoryId>(
            StaticSendTypeState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticSendTypeState, HistoryId>(
            StaticSendTypeState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticSendTypeState, HistoryId>(
            StaticSendTypeState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticSendTypeState, HistoryId>(
            StaticSendTypeState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticSendTypeState, HistoryId>(
            StaticSendTypeState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<StaticSendTypeState, HistoryId>(
            StaticSendTypeState.Idle,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticSendTypeState? = when (stateId) {
        "idle" -> StaticSendTypeState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticSendTypeState): String = when (state) {
        is StaticSendTypeState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticSendTypeState): Int = when (state) {
        is StaticSendTypeState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticSendTypeEvent? = when (name) {
        "aim.bogus" -> StaticSendTypeEvent.Aim.Bogus
        "aim.http" -> StaticSendTypeEvent.Aim.Http
        "aim.scxml" -> StaticSendTypeEvent.Aim.Scxml
        "error.execution" -> StaticSendTypeEvent.Error.Execution
        "fire" -> StaticSendTypeEvent.Fire
        "landed" -> StaticSendTypeEvent.Landed
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticSendTypeEvent): String? = when (event) {
        is StaticSendTypeEvent.Aim.Bogus -> "aim.bogus"
        is StaticSendTypeEvent.Aim.Http -> "aim.http"
        is StaticSendTypeEvent.Aim.Scxml -> "aim.scxml"
        is StaticSendTypeEvent.Error.Execution -> "error.execution"
        is StaticSendTypeEvent.Fire -> "fire"
        is StaticSendTypeEvent.Landed -> "landed"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticSendTypeState,
        event: StaticSendTypeEvent?
    ): EnabledTransition<StaticSendTypeState, HistoryId>? = when (state) {
        is StaticSendTypeState.Idle -> when {
            event is StaticSendTypeEvent.Fire -> transitionIdleAt0
            event is StaticSendTypeEvent.Aim.Scxml -> transitionIdleAt1
            event is StaticSendTypeEvent.Aim.Bogus -> transitionIdleAt2
            event is StaticSendTypeEvent.Aim.Http -> transitionIdleAt3
            event is StaticSendTypeEvent.Landed -> transitionIdleAt4
            event is StaticSendTypeEvent.Error.Execution -> transitionIdleAt5
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_send_type.scxml:17 :: _machine
    override fun onEntry(state: StaticSendTypeState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticSendTypeState.Idle -> {
                // SCE-MAP: static_send_type.scxml:24 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_send_type.scxml:17 :: _machine
    override fun onExit(state: StaticSendTypeState) {
        when (state) {
            is StaticSendTypeState.Idle -> {
                // SCE-MAP: static_send_type.scxml:24 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_send_type.scxml:17 :: _machine
    override fun executeTransitionContent(source: StaticSendTypeState, transitionIndex: Int) {
        when (source) {
        is StaticSendTypeState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_send_type.scxml:25 :: idle :: _transition_0


            if (kind == "http://www.w3.org/TR/scxml/#SCXMLEventProcessor") {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticSendTypeEvent.Landed, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            } else {


            if (run send@{
            // W3C SCXML 6.2 (test199): Unsupported send type raises error.execution
            raisePlatformError(StaticSendTypeEvent.Error.Execution, "<send type='sce:undeclared-type'> names a processor this platform does not support", "__send_0")
            true  // W3C SCXML 5.10: discarded; the block stops below
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            1 -> {
                // SCE-MAP: static_send_type.scxml:28 :: idle :: _transition_1

            if (try { kind = com.sce.forge.runtime.SceChecked.bounded("http://www.w3.org/TR/scxml/#SCXMLEventProcessor", 64); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendTypeEvent.Error.Execution, "<assign location='kind'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_send_type.scxml:31 :: idle :: _transition_2

            if (try { kind = com.sce.forge.runtime.SceChecked.bounded("urn:example:no-such-processor", 64); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendTypeEvent.Error.Execution, "<assign location='kind'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_send_type.scxml:34 :: idle :: _transition_3

            if (try { kind = com.sce.forge.runtime.SceChecked.bounded("http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor", 64); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendTypeEvent.Error.Execution, "<assign location='kind'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_send_type.scxml:37 :: idle :: _transition_4

            if (try { landed = com.sce.forge.runtime.SceChecked.add(landed, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendTypeEvent.Error.Execution, "<assign location='landed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            5 -> {
                // SCE-MAP: static_send_type.scxml:40 :: idle :: _transition_5

            if (try { refused = com.sce.forge.runtime.SceChecked.add(refused, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendTypeEvent.Error.Execution, "<assign location='refused'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
