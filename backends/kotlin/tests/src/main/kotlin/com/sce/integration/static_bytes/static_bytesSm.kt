// SCE-GENERATED — DO NOT EDIT
// source-hash: 552d5eb22ef933056085dce88fe5367b344dcf477c9ae66190ff1867ab30429e

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_bytes.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_bytes.scxml:25 :: _machine

package com.sce.integration.static_bytes

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticBytesState : State {
    data object Idle : StaticBytesState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticBytesEvent : Event {
    data object Check : StaticBytesEvent
    data object Copy : StaticBytesEvent
    sealed interface Error : StaticBytesEvent {
        data object Execution : Error
    }
    data object Fill : StaticBytesEvent
    data object Measure : StaticBytesEvent
    data object Other : StaticBytesEvent
    data object Reset : StaticBytesEvent
    data object Toowide : StaticBytesEvent
}
// --- State Machine (W3C SCXML) ---

class StaticBytesStateMachine(
) : StateMachineEngine<StaticBytesState, StaticBytesEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `frame` datamodel variable, published (`sce:direction="out"`). */
    var frame: ByteArray = "ab".toByteArray()
        // A byte string is handed out as a copy: the array is the machine's own,
        // and a host that wrote into it would change the variable behind its bound.
        get() = field.copyOf()
        private set
    /** W3C SCXML 5.2: the `tail` datamodel variable, published (`sce:direction="out"`). */
    var tail: ByteArray = "xy".toByteArray()
        // A byte string is handed out as a copy: the array is the machine's own,
        // and a host that wrote into it would change the variable behind its bound.
        get() = field.copyOf()
        private set
    /** W3C SCXML 5.2: the `size` datamodel variable, published (`sce:direction="out"`). */
    var size: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `matches` datamodel variable, published (`sce:direction="out"`). */
    var matches: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `misses` datamodel variable, published (`sce:direction="out"`). */
    var misses: UInt = 0.toUInt()
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
        var size: UInt? = null
        var matches: UInt? = null
        var misses: UInt? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.size?.let { size = it }
        params.matches?.let { matches = it }
        params.misses?.let { misses = it }
        params.errors?.let { errors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val frame: ByteArray,
        val tail: ByteArray,
        val size: UInt,
        val matches: UInt,
        val misses: UInt,
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
        val configuration: Set<StaticBytesState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        frame = frame,
        tail = tail,
        size = size,
        matches = matches,
        misses = misses,
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
    val savedShape: String = "fcf32d76d2b60386c38cfd8fd7745bc78c43607cdef76f053c793de0e1e93351"

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
            "frame" to SavedValues.of(frame),
            "tail" to SavedValues.of(tail),
            "size" to SavedValues.of(size),
            "matches" to SavedValues.of(matches),
            "misses" to SavedValues.of(misses),
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
        val saved1 = SavedValues.bytes(saved.variable("frame"), "frame", 8)
        val saved2 = SavedValues.bytes(saved.variable("tail"), "tail", 4)
        val saved3 = SavedValues.uint32(saved.variable("size"), "size")
        val saved4 = SavedValues.uint32(saved.variable("matches"), "matches")
        val saved5 = SavedValues.uint32(saved.variable("misses"), "misses")
        val saved6 = SavedValues.uint32(saved.variable("errors"), "errors")
        frame = saved1
        tail = saved2
        size = saved3
        matches = saved4
        misses = saved5
        errors = saved6
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticBytesState = StaticBytesState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticBytesState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticBytesState, HistoryId>> =
            listOf(StateTarget(StaticBytesState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticBytesState, HistoryId>(
            StaticBytesState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticBytesState, HistoryId>(
            StaticBytesState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticBytesState, HistoryId>(
            StaticBytesState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticBytesState, HistoryId>(
            StaticBytesState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticBytesState, HistoryId>(
            StaticBytesState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<StaticBytesState, HistoryId>(
            StaticBytesState.Idle,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 6, as the microstep reads it.
        val transitionIdleAt6 = EnabledTransition<StaticBytesState, HistoryId>(
            StaticBytesState.Idle,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 7, as the microstep reads it.
        val transitionIdleAt7 = EnabledTransition<StaticBytesState, HistoryId>(
            StaticBytesState.Idle,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 8, as the microstep reads it.
        val transitionIdleAt8 = EnabledTransition<StaticBytesState, HistoryId>(
            StaticBytesState.Idle,
            emptyList(),
            8,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticBytesState? = when (stateId) {
        "idle" -> StaticBytesState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticBytesState): String = when (state) {
        is StaticBytesState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticBytesState): Int = when (state) {
        is StaticBytesState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticBytesEvent? = when (name) {
        "check" -> StaticBytesEvent.Check
        "copy" -> StaticBytesEvent.Copy
        "error.execution" -> StaticBytesEvent.Error.Execution
        "fill" -> StaticBytesEvent.Fill
        "measure" -> StaticBytesEvent.Measure
        "other" -> StaticBytesEvent.Other
        "reset" -> StaticBytesEvent.Reset
        "toowide" -> StaticBytesEvent.Toowide
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticBytesEvent): String? = when (event) {
        is StaticBytesEvent.Check -> "check"
        is StaticBytesEvent.Copy -> "copy"
        is StaticBytesEvent.Error.Execution -> "error.execution"
        is StaticBytesEvent.Fill -> "fill"
        is StaticBytesEvent.Measure -> "measure"
        is StaticBytesEvent.Other -> "other"
        is StaticBytesEvent.Reset -> "reset"
        is StaticBytesEvent.Toowide -> "toowide"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticBytesState,
        event: StaticBytesEvent?
    ): EnabledTransition<StaticBytesState, HistoryId>? = when (state) {
        is StaticBytesState.Idle -> when {
            event is StaticBytesEvent.Fill -> transitionIdleAt0
            event is StaticBytesEvent.Toowide -> transitionIdleAt1
            event is StaticBytesEvent.Reset -> transitionIdleAt2
            event is StaticBytesEvent.Other -> transitionIdleAt3
            event is StaticBytesEvent.Copy -> transitionIdleAt4
            event is StaticBytesEvent.Check && frame.contentEquals("ab".toByteArray()) -> transitionIdleAt5
            event is StaticBytesEvent.Check && !frame.contentEquals("ab".toByteArray()) -> transitionIdleAt6
            event is StaticBytesEvent.Measure -> transitionIdleAt7
            event is StaticBytesEvent.Error.Execution -> transitionIdleAt8
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_bytes.scxml:25 :: _machine
    override fun onEntry(state: StaticBytesState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticBytesState.Idle -> {
                // SCE-MAP: static_bytes.scxml:35 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_bytes.scxml:25 :: _machine
    override fun onExit(state: StaticBytesState) {
        when (state) {
            is StaticBytesState.Idle -> {
                // SCE-MAP: static_bytes.scxml:35 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_bytes.scxml:25 :: _machine
    override fun executeTransitionContent(source: StaticBytesState, transitionIndex: Int) {
        when (source) {
        is StaticBytesState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_bytes.scxml:36 :: idle :: _transition_0

            if (try { frame = com.sce.forge.runtime.SceChecked.bounded("abcdefgh".toByteArray(), 8); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBytesEvent.Error.Execution, "<assign location='frame'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_bytes.scxml:39 :: idle :: _transition_1

            if (try { frame = com.sce.forge.runtime.SceChecked.bounded("abcdefghi".toByteArray(), 8); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBytesEvent.Error.Execution, "<assign location='frame'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_bytes.scxml:42 :: idle :: _transition_2

            if (try { frame = com.sce.forge.runtime.SceChecked.bounded("ab".toByteArray(), 8); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBytesEvent.Error.Execution, "<assign location='frame'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_bytes.scxml:45 :: idle :: _transition_3

            if (try { frame = com.sce.forge.runtime.SceChecked.bounded("ba".toByteArray(), 8); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBytesEvent.Error.Execution, "<assign location='frame'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_bytes.scxml:48 :: idle :: _transition_4

            if (try { tail = com.sce.forge.runtime.SceChecked.bounded(frame, 4); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBytesEvent.Error.Execution, "<assign location='tail'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            5 -> {
                // SCE-MAP: static_bytes.scxml:51 :: idle :: _transition_5

            if (try { matches = com.sce.forge.runtime.SceChecked.add(matches, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBytesEvent.Error.Execution, "<assign location='matches'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            6 -> {
                // SCE-MAP: static_bytes.scxml:54 :: idle :: _transition_6

            if (try { misses = com.sce.forge.runtime.SceChecked.add(misses, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBytesEvent.Error.Execution, "<assign location='misses'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            7 -> {
                // SCE-MAP: static_bytes.scxml:57 :: idle :: _transition_7

            size = (frame).size.toUInt()
            }
            8 -> {
                // SCE-MAP: static_bytes.scxml:60 :: idle :: _transition_8

            if (try { errors = com.sce.forge.runtime.SceChecked.add(errors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBytesEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
