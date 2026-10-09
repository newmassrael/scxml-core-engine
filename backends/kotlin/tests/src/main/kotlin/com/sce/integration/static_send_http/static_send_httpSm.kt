// SCE-GENERATED — DO NOT EDIT
// source-hash: 552d5eb22ef933056085dce88fe5367b344dcf477c9ae66190ff1867ab30429e

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_send_http.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_send_http.scxml:34 :: _machine

package com.sce.integration.static_send_http

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticSendHttpState : State {
    data object Idle : StaticSendHttpState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticSendHttpEvent : Event {
    data object Boom : StaticSendHttpEvent
    data object Bump : StaticSendHttpEvent
    sealed interface Error : StaticSendHttpEvent {
        data object Execution : Error
    }
    data object Go : StaticSendHttpEvent
    data object Note : StaticSendHttpEvent
}
// --- State Machine (W3C SCXML) ---

class StaticSendHttpStateMachine(
) : StateMachineEngine<StaticSendHttpState, StaticSendHttpEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `count` datamodel variable, the machine's own. */
    private var count: UInt = 3.toUInt()
    /** W3C SCXML 5.2: the `ready` datamodel variable, the machine's own. */
    private var ready: Boolean = false
    /** W3C SCXML 5.2: the `label` datamodel variable, the machine's own. */
    private var label: String = "idle"
    /** W3C SCXML 5.2: the `delta` datamodel variable, the machine's own. */
    private var delta: Short = -5
    /** W3C SCXML 5.2: the `ratio` datamodel variable, the machine's own. */
    private var ratio: Double = 1.5
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    var errors: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var count: UInt? = null
        var ready: Boolean? = null
        var label: String? = null
        var delta: Short? = null
        var ratio: Double? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.count?.let { count = it }
        params.ready?.let { ready = it }
        params.label?.let { label = it }
        params.delta?.let { delta = it }
        params.ratio?.let { ratio = it }
        params.errors?.let { errors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
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
        val configuration: Set<StaticSendHttpState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
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
    val savedShape: String = "00fa415a572dac70afc92855a696126c6cba10c087c24cf7bb2a7d49fa3af55e"

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
            "count" to SavedValues.of(count),
            "ready" to SavedValues.of(ready),
            "label" to SavedValues.of(label),
            "delta" to SavedValues.of(delta),
            "ratio" to SavedValues.of(ratio),
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
        val saved1 = SavedValues.uint32(saved.variable("count"), "count")
        val saved2 = SavedValues.bool(saved.variable("ready"), "ready")
        val saved3 = SavedValues.string(saved.variable("label"), "label", 16)
        val saved4 = SavedValues.int16(saved.variable("delta"), "delta")
        val saved5 = SavedValues.float64(saved.variable("ratio"), "ratio")
        val saved6 = SavedValues.uint32(saved.variable("errors"), "errors")
        count = saved1
        ready = saved2
        label = saved3
        delta = saved4
        ratio = saved5
        errors = saved6
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticSendHttpState = StaticSendHttpState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticSendHttpState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticSendHttpState, HistoryId>> =
            listOf(StateTarget(StaticSendHttpState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticSendHttpState, HistoryId>(
            StaticSendHttpState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticSendHttpState, HistoryId>(
            StaticSendHttpState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticSendHttpState, HistoryId>(
            StaticSendHttpState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticSendHttpState, HistoryId>(
            StaticSendHttpState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticSendHttpState? = when (stateId) {
        "idle" -> StaticSendHttpState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticSendHttpState): String = when (state) {
        is StaticSendHttpState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticSendHttpState): Int = when (state) {
        is StaticSendHttpState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticSendHttpEvent? = when (name) {
        "boom" -> StaticSendHttpEvent.Boom
        "bump" -> StaticSendHttpEvent.Bump
        "error.execution" -> StaticSendHttpEvent.Error.Execution
        "go" -> StaticSendHttpEvent.Go
        "note" -> StaticSendHttpEvent.Note
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticSendHttpEvent): String? = when (event) {
        is StaticSendHttpEvent.Boom -> "boom"
        is StaticSendHttpEvent.Bump -> "bump"
        is StaticSendHttpEvent.Error.Execution -> "error.execution"
        is StaticSendHttpEvent.Go -> "go"
        is StaticSendHttpEvent.Note -> "note"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticSendHttpState,
        event: StaticSendHttpEvent?
    ): EnabledTransition<StaticSendHttpState, HistoryId>? = when (state) {
        is StaticSendHttpState.Idle -> when {
            event is StaticSendHttpEvent.Bump -> transitionIdleAt0
            event is StaticSendHttpEvent.Go -> transitionIdleAt1
            event is StaticSendHttpEvent.Boom -> transitionIdleAt2
            event is StaticSendHttpEvent.Error.Execution -> transitionIdleAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_send_http.scxml:34 :: _machine
    override fun onEntry(state: StaticSendHttpState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticSendHttpState.Idle -> {
                // SCE-MAP: static_send_http.scxml:44 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_send_http.scxml:34 :: _machine
    override fun onExit(state: StaticSendHttpState) {
        when (state) {
            is StaticSendHttpState.Idle -> {
                // SCE-MAP: static_send_http.scxml:44 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_send_http.scxml:34 :: _machine
    override fun executeTransitionContent(source: StaticSendHttpState, transitionIndex: Int) {
        when (source) {
        is StaticSendHttpState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_send_http.scxml:45 :: idle :: _transition_0

            count = 4.toUInt()

            ready = true

            if (try { label = com.sce.forge.runtime.SceChecked.bounded("busy", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendHttpEvent.Error.Execution, "<assign location='label'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_send_http.scxml:50 :: idle :: _transition_1


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "count", (count).toLong())

            putParam(sendPayload, "ready", ready)

            putParam(sendPayload, "label", label)

            try {
                putParam(sendPayload, "twice", (com.sce.forge.runtime.SceChecked.mul(count, 2.toUInt())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(StaticSendHttpEvent.Error.Execution, "<send> <param name='twice'> could not be read")
                paramFailed = true
            }

            putParam(sendPayload, "delta", (delta).toLong())

            putParam(sendPayload, "ratio", ratio)

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML C.2: BasicHTTP send — one arm for a static and a dynamic target
            val httpContent = ""
            performHttpSend("http://example.invalid/hook", "note", httpContent, sendWireParams, "__send_0")
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            2 -> {
                // SCE-MAP: static_send_http.scxml:60 :: idle :: _transition_2


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "count", (count).toLong())

            try {
                putParam(sendPayload, "big", (com.sce.forge.runtime.SceChecked.mul(count, 2000000000.toUInt())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(StaticSendHttpEvent.Error.Execution, "<send> <param name='big'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML C.2: BasicHTTP send — one arm for a static and a dynamic target
            val httpContent = ""
            performHttpSend("http://example.invalid/hook", "note", httpContent, sendWireParams, "__send_1")
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            3 -> {
                // SCE-MAP: static_send_http.scxml:66 :: idle :: _transition_3

            if (try { errors = com.sce.forge.runtime.SceChecked.add(errors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendHttpEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
