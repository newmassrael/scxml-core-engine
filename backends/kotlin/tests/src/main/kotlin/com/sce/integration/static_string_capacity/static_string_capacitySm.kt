// SCE-GENERATED — DO NOT EDIT
// source-hash: f9bc2e3a10834cc236cb284a28c51c227951af5f2785c6dda5a820794a76e2ca

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_string_capacity.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_string_capacity.scxml:17 :: _machine

package com.sce.integration.static_string_capacity

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticStringCapacityState : State {
    data object Idle : StaticStringCapacityState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticStringCapacityEvent : Event {
    data object BodyAscii : StaticStringCapacityEvent
    data object BodyExact : StaticStringCapacityEvent
    data object BodyWide : StaticStringCapacityEvent
    data object Copy : StaticStringCapacityEvent
    sealed interface Error : StaticStringCapacityEvent {
        data object Execution : Error
    }
    data object Fill : StaticStringCapacityEvent
    data object Reset : StaticStringCapacityEvent
}
// --- State Machine (W3C SCXML) ---

class StaticStringCapacityStateMachine(
) : StateMachineEngine<StaticStringCapacityState, StaticStringCapacityEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `title` datamodel variable, published (`sce:direction="out"`). */
    var title: String = "ab"
        private set
    /** W3C SCXML 5.2: the `body` datamodel variable, published (`sce:direction="out"`). */
    var body: String = "hello"
        private set
    /** W3C SCXML 5.2: the `copied` datamodel variable, published (`sce:direction="out"`). */
    var copied: UInt = 0.toUInt()
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
        var title: String? = null
        var body: String? = null
        var copied: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.title?.let { title = it }
        params.body?.let { body = it }
        params.copied?.let { copied = it }
        params.refusals?.let { refusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val title: String,
        val body: String,
        val copied: UInt,
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
        val configuration: Set<StaticStringCapacityState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        title = title,
        body = body,
        copied = copied,
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
    val savedShape: String = "135ad12b6f68018262518e29d4184121311cd30e7c5a354debfd73c4d828b891"

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
            "title" to SavedValues.of(title),
            "body" to SavedValues.of(body),
            "copied" to SavedValues.of(copied),
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
        val saved1 = SavedValues.string(saved.variable("title"), "title", 4)
        val saved2 = SavedValues.string(saved.variable("body"), "body", 16)
        val saved3 = SavedValues.uint32(saved.variable("copied"), "copied")
        val saved4 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        title = saved1
        body = saved2
        copied = saved3
        refusals = saved4
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticStringCapacityState = StaticStringCapacityState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticStringCapacityState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticStringCapacityState, HistoryId>> =
            listOf(StateTarget(StaticStringCapacityState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticStringCapacityState, HistoryId>(
            StaticStringCapacityState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticStringCapacityState, HistoryId>(
            StaticStringCapacityState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticStringCapacityState, HistoryId>(
            StaticStringCapacityState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticStringCapacityState, HistoryId>(
            StaticStringCapacityState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticStringCapacityState, HistoryId>(
            StaticStringCapacityState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<StaticStringCapacityState, HistoryId>(
            StaticStringCapacityState.Idle,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 6, as the microstep reads it.
        val transitionIdleAt6 = EnabledTransition<StaticStringCapacityState, HistoryId>(
            StaticStringCapacityState.Idle,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticStringCapacityState? = when (stateId) {
        "idle" -> StaticStringCapacityState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticStringCapacityState): String = when (state) {
        is StaticStringCapacityState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticStringCapacityState): Int = when (state) {
        is StaticStringCapacityState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticStringCapacityEvent? = when (name) {
        "body_ascii" -> StaticStringCapacityEvent.BodyAscii
        "body_exact" -> StaticStringCapacityEvent.BodyExact
        "body_wide" -> StaticStringCapacityEvent.BodyWide
        "copy" -> StaticStringCapacityEvent.Copy
        "error.execution" -> StaticStringCapacityEvent.Error.Execution
        "fill" -> StaticStringCapacityEvent.Fill
        "reset" -> StaticStringCapacityEvent.Reset
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticStringCapacityEvent): String? = when (event) {
        is StaticStringCapacityEvent.BodyAscii -> "body_ascii"
        is StaticStringCapacityEvent.BodyExact -> "body_exact"
        is StaticStringCapacityEvent.BodyWide -> "body_wide"
        is StaticStringCapacityEvent.Copy -> "copy"
        is StaticStringCapacityEvent.Error.Execution -> "error.execution"
        is StaticStringCapacityEvent.Fill -> "fill"
        is StaticStringCapacityEvent.Reset -> "reset"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticStringCapacityState,
        event: StaticStringCapacityEvent?
    ): EnabledTransition<StaticStringCapacityState, HistoryId>? = when (state) {
        is StaticStringCapacityState.Idle -> when {
            event is StaticStringCapacityEvent.Fill -> transitionIdleAt0
            event is StaticStringCapacityEvent.Reset -> transitionIdleAt1
            event is StaticStringCapacityEvent.BodyAscii -> transitionIdleAt2
            event is StaticStringCapacityEvent.BodyExact -> transitionIdleAt3
            event is StaticStringCapacityEvent.BodyWide -> transitionIdleAt4
            event is StaticStringCapacityEvent.Copy -> transitionIdleAt5
            event is StaticStringCapacityEvent.Error.Execution -> transitionIdleAt6
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_string_capacity.scxml:17 :: _machine
    override fun onEntry(state: StaticStringCapacityState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticStringCapacityState.Idle -> {
                // SCE-MAP: static_string_capacity.scxml:25 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_string_capacity.scxml:17 :: _machine
    override fun onExit(state: StaticStringCapacityState) {
        when (state) {
            is StaticStringCapacityState.Idle -> {
                // SCE-MAP: static_string_capacity.scxml:25 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_string_capacity.scxml:17 :: _machine
    override fun executeTransitionContent(source: StaticStringCapacityState, transitionIndex: Int) {
        when (source) {
        is StaticStringCapacityState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_string_capacity.scxml:27 :: idle :: _transition_0

            if (try { title = com.sce.forge.runtime.SceChecked.bounded("abcd", 4); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStringCapacityEvent.Error.Execution, "<assign location='title'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_string_capacity.scxml:30 :: idle :: _transition_1

            if (try { title = com.sce.forge.runtime.SceChecked.bounded("ab", 4); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStringCapacityEvent.Error.Execution, "<assign location='title'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_string_capacity.scxml:33 :: idle :: _transition_2

            if (try { body = com.sce.forge.runtime.SceChecked.bounded("hello", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStringCapacityEvent.Error.Execution, "<assign location='body'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_string_capacity.scxml:37 :: idle :: _transition_3

            if (try { body = com.sce.forge.runtime.SceChecked.bounded("éé", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStringCapacityEvent.Error.Execution, "<assign location='body'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_string_capacity.scxml:41 :: idle :: _transition_4

            if (try { body = com.sce.forge.runtime.SceChecked.bounded("é€", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStringCapacityEvent.Error.Execution, "<assign location='body'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            5 -> {
                // SCE-MAP: static_string_capacity.scxml:44 :: idle :: _transition_5

            if (try { title = com.sce.forge.runtime.SceChecked.bounded(body, 4); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStringCapacityEvent.Error.Execution, "<assign location='title'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { copied = com.sce.forge.runtime.SceChecked.add(copied, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStringCapacityEvent.Error.Execution, "<assign location='copied'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            6 -> {
                // SCE-MAP: static_string_capacity.scxml:48 :: idle :: _transition_6

            if (try { refusals = com.sce.forge.runtime.SceChecked.add(refusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStringCapacityEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
