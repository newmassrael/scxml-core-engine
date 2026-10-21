// SCE-GENERATED — DO NOT EDIT
// source-hash: 9fab540da93829eaa9159cbc7132fde0b98312062867e02e6e631d43aef759bc

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_forge_calls.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_forge_calls.scxml:15 :: _machine

package com.sce.integration.static_forge_calls

import com.sce.runtime.*
import com.sce.generated.forge_call_code.*
import com.sce.generated.forge_call_high.*
import com.sce.generated.forge_call_limit.*
import com.sce.generated.forge_call_offset.*
import com.sce.generated.forge_call_range.*
import com.sce.generated.forge_call_ratio.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticForgeCallsState : State {
    data object Run : StaticForgeCallsState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticForgeCallsEvent : Event {
    sealed interface Error : StaticForgeCallsEvent {
        data object Execution : Error
    }
    data object Probe : StaticForgeCallsEvent
    data object Rise : StaticForgeCallsEvent
    data object Shift : StaticForgeCallsEvent
    data object Spin : StaticForgeCallsEvent
}
// --- State Machine (W3C SCXML) ---

class StaticForgeCallsStateMachine(
) : StateMachineEngine<StaticForgeCallsState, StaticForgeCallsEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `gear` datamodel variable, published (`sce:direction="out"`). */
    private var vGear: UByte = 0.toUByte()

    /** What the published `gear` holds. */
    val gear: UByte get() = vGear
    /** W3C SCXML 5.2: the `ratio` datamodel variable, published (`sce:direction="out"`). */
    private var vRatio: Int = 0

    /** What the published `ratio` holds. */
    val ratio: Int get() = vRatio
    /** W3C SCXML 5.2: the `code` datamodel variable, published (`sce:direction="out"`). */
    private var vCode: Int = 0

    /** What the published `code` holds. */
    val code: Int get() = vCode
    /** W3C SCXML 5.2: the `misses` datamodel variable, published (`sce:direction="out"`). */
    private var vMisses: UInt = 0.toUInt()

    /** What the published `misses` holds. */
    val misses: UInt get() = vMisses
    /** W3C SCXML 5.2: the `level` datamodel variable, published (`sce:direction="out"`). */
    private var vLevel: Int = 0

    /** What the published `level` holds. */
    val level: Int get() = vLevel
    /** W3C SCXML 5.2: the `offset` datamodel variable, published (`sce:direction="out"`). */
    private var vOffset: Int = 0

    /** What the published `offset` holds. */
    val offset: Int get() = vOffset
    /** W3C SCXML 5.2: the `accepted` datamodel variable, published (`sce:direction="out"`). */
    private var vAccepted: Boolean = false

    /** What the published `accepted` holds. */
    val accepted: Boolean get() = vAccepted
    /** W3C SCXML 5.2: the `rpm` datamodel variable, published (`sce:direction="out"`). */
    private var vRpm: UShort = 0.toUShort()

    /** What the published `rpm` holds. */
    val rpm: UShort get() = vRpm
    /** W3C SCXML 5.2: the `limit` datamodel variable, published (`sce:direction="out"`). */
    private var vLimit: Double = 0.0

    /** What the published `limit` holds. */
    val limit: Double get() = vLimit
    /** W3C SCXML 5.2: the `highs` datamodel variable, published (`sce:direction="out"`). */
    private var vHighs: UInt = 0.toUInt()

    /** What the published `highs` holds. */
    val highs: UInt get() = vHighs

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var gear: UByte? = null
        var ratio: Int? = null
        var code: Int? = null
        var misses: UInt? = null
        var level: Int? = null
        var offset: Int? = null
        var accepted: Boolean? = null
        var rpm: UShort? = null
        var limit: Double? = null
        var highs: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.gear?.let { vGear = it }
        params.ratio?.let { vRatio = it }
        params.code?.let { vCode = it }
        params.misses?.let { vMisses = it }
        params.level?.let { vLevel = it }
        params.offset?.let { vOffset = it }
        params.accepted?.let { vAccepted = it }
        params.rpm?.let { vRpm = it }
        params.limit?.let { vLimit = it }
        params.highs?.let { vHighs = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val gear: UByte,
        val ratio: Int,
        val code: Int,
        val misses: UInt,
        val level: Int,
        val offset: Int,
        val accepted: Boolean,
        val rpm: UShort,
        val limit: Double,
        val highs: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticForgeCallsState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        gear = vGear,
        ratio = vRatio,
        code = vCode,
        misses = vMisses,
        level = vLevel,
        offset = vOffset,
        accepted = vAccepted,
        rpm = vRpm,
        limit = vLimit,
        highs = vHighs,
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
     * What a saved state of this document is bound to: a state saved from a
     * document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not, and one
     * saved at an earlier revision of a record schema is read with each field a
     * later revision added at its default.
     */
    val savedShape: SavedShape = SavedShape(
        first = "894ce6ad7b5cf850e09034ba3c29f741ef9f52312ca7fd0c01d84819c4390e69",
        revised = "08d66e661a0cdc8c30dcda21efc297b43b2b3f930eef12e2da27da58b6deb180",
        format2First = "894ce6ad7b5cf850e09034ba3c29f741ef9f52312ca7fd0c01d84819c4390e69",
        format2Revised = "08d66e661a0cdc8c30dcda21efc297b43b2b3f930eef12e2da27da58b6deb180",
        records = listOf(
        ),
        enums = listOf(
        ),
    )

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
            "gear" to SavedValues.of(vGear),
            "ratio" to SavedValues.of(vRatio),
            "code" to SavedValues.of(vCode),
            "misses" to SavedValues.of(vMisses),
            "level" to SavedValues.of(vLevel),
            "offset" to SavedValues.of(vOffset),
            "accepted" to SavedValues.of(vAccepted),
            "rpm" to SavedValues.of(vRpm),
            "limit" to SavedValues.of(vLimit),
            "highs" to SavedValues.of(vHighs),
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
        // The state as one of this machine: each record saved at an earlier
        // revision of its schema holds the defaults of the fields added since.
        val savedGrown = beginRestore(saved, savedShape)
        val saved1 = SavedValues.uint8(savedGrown.variable("gear"), "gear")
        val saved2 = SavedValues.int32(savedGrown.variable("ratio"), "ratio")
        val saved3 = SavedValues.int32(savedGrown.variable("code"), "code")
        val saved4 = SavedValues.uint32(savedGrown.variable("misses"), "misses")
        val saved5 = SavedValues.int32(savedGrown.variable("level"), "level")
        val saved6 = SavedValues.int32(savedGrown.variable("offset"), "offset")
        val saved7 = SavedValues.bool(savedGrown.variable("accepted"), "accepted")
        val saved8 = SavedValues.uint16(savedGrown.variable("rpm"), "rpm")
        val saved9 = SavedValues.float64(savedGrown.variable("limit"), "limit")
        val saved10 = SavedValues.uint32(savedGrown.variable("highs"), "highs")
        vGear = saved1
        vRatio = saved2
        vCode = saved3
        vMisses = saved4
        vLevel = saved5
        vOffset = saved6
        vAccepted = saved7
        vRpm = saved8
        vLimit = saved9
        vHighs = saved10
        enterSaved(savedGrown, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticForgeCallsState = StaticForgeCallsState.Run

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
    override val documentInitialTargets: List<EntryTarget<StaticForgeCallsState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticForgeCallsState, HistoryId>> =
            listOf(StateTarget(StaticForgeCallsState.Run))

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<StaticForgeCallsState, HistoryId>(
            StaticForgeCallsState.Run,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: run's transition 1, as the microstep reads it.
        val transitionRunAt1 = EnabledTransition<StaticForgeCallsState, HistoryId>(
            StaticForgeCallsState.Run,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: run's transition 2, as the microstep reads it.
        val transitionRunAt2 = EnabledTransition<StaticForgeCallsState, HistoryId>(
            StaticForgeCallsState.Run,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: run's transition 3, as the microstep reads it.
        val transitionRunAt3 = EnabledTransition<StaticForgeCallsState, HistoryId>(
            StaticForgeCallsState.Run,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: run's transition 4, as the microstep reads it.
        val transitionRunAt4 = EnabledTransition<StaticForgeCallsState, HistoryId>(
            StaticForgeCallsState.Run,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticForgeCallsState? = when (stateId) {
        "run" -> StaticForgeCallsState.Run
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticForgeCallsState): String = when (state) {
        is StaticForgeCallsState.Run -> "run"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticForgeCallsState): Int = when (state) {
        is StaticForgeCallsState.Run -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticForgeCallsEvent? = when (name) {
        "error.execution" -> StaticForgeCallsEvent.Error.Execution
        "probe" -> StaticForgeCallsEvent.Probe
        "rise" -> StaticForgeCallsEvent.Rise
        "shift" -> StaticForgeCallsEvent.Shift
        "spin" -> StaticForgeCallsEvent.Spin
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticForgeCallsEvent): String? = when (event) {
        is StaticForgeCallsEvent.Error.Execution -> "error.execution"
        is StaticForgeCallsEvent.Probe -> "probe"
        is StaticForgeCallsEvent.Rise -> "rise"
        is StaticForgeCallsEvent.Shift -> "shift"
        is StaticForgeCallsEvent.Spin -> "spin"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticForgeCallsState,
        event: StaticForgeCallsEvent?
    ): EnabledTransition<StaticForgeCallsState, HistoryId>? = when (state) {
        is StaticForgeCallsState.Run -> when {
            event is StaticForgeCallsEvent.Shift -> transitionRunAt0
            event is StaticForgeCallsEvent.Rise -> transitionRunAt1
            event is StaticForgeCallsEvent.Spin -> transitionRunAt2
            event is StaticForgeCallsEvent.Probe && forgeCallHigh(vLevel) -> transitionRunAt3
            event is StaticForgeCallsEvent.Error.Execution -> transitionRunAt4
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_forge_calls.scxml:15 :: _machine
    override fun onEntry(state: StaticForgeCallsState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticForgeCallsState.Run -> {
                // SCE-MAP: static_forge_calls.scxml:35 :: run :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_forge_calls.scxml:15 :: _machine
    override fun onExit(state: StaticForgeCallsState) {
        when (state) {
            is StaticForgeCallsState.Run -> {
                // SCE-MAP: static_forge_calls.scxml:35 :: run :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_forge_calls.scxml:15 :: _machine
    override fun executeTransitionContent(source: StaticForgeCallsState, transitionIndex: Int) {
        when (source) {
        is StaticForgeCallsState.Run -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_forge_calls.scxml:36 :: run :: _transition_0

            if (try { vGear = com.sce.forge.runtime.SceChecked.add(vGear, 1.toUByte()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForgeCallsEvent.Error.Execution, "<assign location='gear'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vRatio = lookupRatio(vGear)

            if (try { vCode = com.sce.forge.runtime.SceChecked.take(com.sce.forge.runtime.SceChecked.found(lookupCode(vGear))); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForgeCallsEvent.Error.Execution, "<assign location='code'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_forge_calls.scxml:41 :: run :: _transition_1

            if (try { vLevel = com.sce.forge.runtime.SceChecked.add(vLevel, 7); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForgeCallsEvent.Error.Execution, "<assign location='level'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vOffset = computeOffset(vLevel)

            vAccepted = ForgeCallRange().validate(vLevel, vOffset).valid
            }
            2 -> {
                // SCE-MAP: static_forge_calls.scxml:46 :: run :: _transition_2

            if (try { vRpm = com.sce.forge.runtime.SceChecked.add(vRpm, 500.toUShort()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForgeCallsEvent.Error.Execution, "<assign location='rpm'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vLimit = ForgeCallLimit.lookup(vRpm)
            }
            3 -> {
                // SCE-MAP: static_forge_calls.scxml:50 :: run :: _transition_3

            if (try { vHighs = com.sce.forge.runtime.SceChecked.add(vHighs, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForgeCallsEvent.Error.Execution, "<assign location='highs'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_forge_calls.scxml:53 :: run :: _transition_4

            if (try { vMisses = com.sce.forge.runtime.SceChecked.add(vMisses, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForgeCallsEvent.Error.Execution, "<assign location='misses'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
