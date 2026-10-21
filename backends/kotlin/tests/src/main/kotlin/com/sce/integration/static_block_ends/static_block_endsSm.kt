// SCE-GENERATED — DO NOT EDIT
// source-hash: 0fafd5dc7cb78db436777a56fff2cdefea18c3275c9e740c945e368e87cb13ce

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_block_ends.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_block_ends.scxml:24 :: _machine

package com.sce.integration.static_block_ends

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticBlockEndsState : State {
    data object Waiting : StaticBlockEndsState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticBlockEndsEvent : Event {
    sealed interface Error : StaticBlockEndsEvent {
        data object Execution : Error
    }
    sealed interface Fail : StaticBlockEndsEvent {
        data object Assign : Fail
        data object Branch : Fail
        data object Cond : Fail
    }
    data object Ok : StaticBlockEndsEvent
}
// --- State Machine (W3C SCXML) ---

class StaticBlockEndsStateMachine(
) : StateMachineEngine<StaticBlockEndsState, StaticBlockEndsEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `a` datamodel variable, published (`sce:direction="out"`). */
    private var vA: UByte = 250.toUByte()

    /** What the published `a` holds. */
    val a: UByte get() = vA
    /** W3C SCXML 5.2: the `b` datamodel variable, published (`sce:direction="out"`). */
    private var vB: UByte = 0.toUByte()

    /** What the published `b` holds. */
    val b: UByte get() = vB
    /** W3C SCXML 5.2: the `afterAssign` datamodel variable, published (`sce:direction="out"`). */
    private var vAfterAssign: UByte = 0.toUByte()

    /** What the published `afterAssign` holds. */
    val afterAssign: UByte get() = vAfterAssign
    /** W3C SCXML 5.2: the `thenRan` datamodel variable, published (`sce:direction="out"`). */
    private var vThenRan: UByte = 0.toUByte()

    /** What the published `thenRan` holds. */
    val thenRan: UByte get() = vThenRan
    /** W3C SCXML 5.2: the `elseRan` datamodel variable, published (`sce:direction="out"`). */
    private var vElseRan: UByte = 0.toUByte()

    /** What the published `elseRan` holds. */
    val elseRan: UByte get() = vElseRan
    /** W3C SCXML 5.2: the `afterIf` datamodel variable, published (`sce:direction="out"`). */
    private var vAfterIf: UByte = 0.toUByte()

    /** What the published `afterIf` holds. */
    val afterIf: UByte get() = vAfterIf
    /** W3C SCXML 5.2: the `inBranch` datamodel variable, published (`sce:direction="out"`). */
    private var vInBranch: UByte = 0.toUByte()

    /** What the published `inBranch` holds. */
    val inBranch: UByte get() = vInBranch
    /** W3C SCXML 5.2: the `afterBranch` datamodel variable, published (`sce:direction="out"`). */
    private var vAfterBranch: UByte = 0.toUByte()

    /** What the published `afterBranch` holds. */
    val afterBranch: UByte get() = vAfterBranch
    /** W3C SCXML 5.2: the `afterOk` datamodel variable, published (`sce:direction="out"`). */
    private var vAfterOk: UByte = 0.toUByte()

    /** What the published `afterOk` holds. */
    val afterOk: UByte get() = vAfterOk
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    private var vErrors: UByte = 0.toUByte()

    /** What the published `errors` holds. */
    val errors: UByte get() = vErrors

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var a: UByte? = null
        var b: UByte? = null
        var afterAssign: UByte? = null
        var thenRan: UByte? = null
        var elseRan: UByte? = null
        var afterIf: UByte? = null
        var inBranch: UByte? = null
        var afterBranch: UByte? = null
        var afterOk: UByte? = null
        var errors: UByte? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.a?.let { vA = it }
        params.b?.let { vB = it }
        params.afterAssign?.let { vAfterAssign = it }
        params.thenRan?.let { vThenRan = it }
        params.elseRan?.let { vElseRan = it }
        params.afterIf?.let { vAfterIf = it }
        params.inBranch?.let { vInBranch = it }
        params.afterBranch?.let { vAfterBranch = it }
        params.afterOk?.let { vAfterOk = it }
        params.errors?.let { vErrors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val a: UByte,
        val b: UByte,
        val afterAssign: UByte,
        val thenRan: UByte,
        val elseRan: UByte,
        val afterIf: UByte,
        val inBranch: UByte,
        val afterBranch: UByte,
        val afterOk: UByte,
        val errors: UByte,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticBlockEndsState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        a = vA,
        b = vB,
        afterAssign = vAfterAssign,
        thenRan = vThenRan,
        elseRan = vElseRan,
        afterIf = vAfterIf,
        inBranch = vInBranch,
        afterBranch = vAfterBranch,
        afterOk = vAfterOk,
        errors = vErrors,
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
        first = "3392e4c39beb56ba9cc0d257d4b7e2fe3e78865fd9251747ebba45ab05882c51",
        revised = "49722041863d35d2b2ef736d0a7663d2462f845efe740a017d404a4897ec0292",
        format2First = "3392e4c39beb56ba9cc0d257d4b7e2fe3e78865fd9251747ebba45ab05882c51",
        format2Revised = "49722041863d35d2b2ef736d0a7663d2462f845efe740a017d404a4897ec0292",
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
            "a" to SavedValues.of(vA),
            "b" to SavedValues.of(vB),
            "afterAssign" to SavedValues.of(vAfterAssign),
            "thenRan" to SavedValues.of(vThenRan),
            "elseRan" to SavedValues.of(vElseRan),
            "afterIf" to SavedValues.of(vAfterIf),
            "inBranch" to SavedValues.of(vInBranch),
            "afterBranch" to SavedValues.of(vAfterBranch),
            "afterOk" to SavedValues.of(vAfterOk),
            "errors" to SavedValues.of(vErrors),
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
        val saved1 = SavedValues.uint8(savedGrown.variable("a"), "a")
        val saved2 = SavedValues.uint8(savedGrown.variable("b"), "b")
        val saved3 = SavedValues.uint8(savedGrown.variable("afterAssign"), "afterAssign")
        val saved4 = SavedValues.uint8(savedGrown.variable("thenRan"), "thenRan")
        val saved5 = SavedValues.uint8(savedGrown.variable("elseRan"), "elseRan")
        val saved6 = SavedValues.uint8(savedGrown.variable("afterIf"), "afterIf")
        val saved7 = SavedValues.uint8(savedGrown.variable("inBranch"), "inBranch")
        val saved8 = SavedValues.uint8(savedGrown.variable("afterBranch"), "afterBranch")
        val saved9 = SavedValues.uint8(savedGrown.variable("afterOk"), "afterOk")
        val saved10 = SavedValues.uint8(savedGrown.variable("errors"), "errors")
        vA = saved1
        vB = saved2
        vAfterAssign = saved3
        vThenRan = saved4
        vElseRan = saved5
        vAfterIf = saved6
        vInBranch = saved7
        vAfterBranch = saved8
        vAfterOk = saved9
        vErrors = saved10
        enterSaved(savedGrown, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticBlockEndsState = StaticBlockEndsState.Waiting

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
    override val documentInitialTargets: List<EntryTarget<StaticBlockEndsState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticBlockEndsState, HistoryId>> =
            listOf(StateTarget(StaticBlockEndsState.Waiting))

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StaticBlockEndsState, HistoryId>(
            StaticBlockEndsState.Waiting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<StaticBlockEndsState, HistoryId>(
            StaticBlockEndsState.Waiting,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: waiting's transition 2, as the microstep reads it.
        val transitionWaitingAt2 = EnabledTransition<StaticBlockEndsState, HistoryId>(
            StaticBlockEndsState.Waiting,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: waiting's transition 3, as the microstep reads it.
        val transitionWaitingAt3 = EnabledTransition<StaticBlockEndsState, HistoryId>(
            StaticBlockEndsState.Waiting,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: waiting's transition 4, as the microstep reads it.
        val transitionWaitingAt4 = EnabledTransition<StaticBlockEndsState, HistoryId>(
            StaticBlockEndsState.Waiting,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticBlockEndsState? = when (stateId) {
        "waiting" -> StaticBlockEndsState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticBlockEndsState): String = when (state) {
        is StaticBlockEndsState.Waiting -> "waiting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticBlockEndsState): Int = when (state) {
        is StaticBlockEndsState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticBlockEndsEvent? = when (name) {
        "error.execution" -> StaticBlockEndsEvent.Error.Execution
        "fail.assign" -> StaticBlockEndsEvent.Fail.Assign
        "fail.branch" -> StaticBlockEndsEvent.Fail.Branch
        "fail.cond" -> StaticBlockEndsEvent.Fail.Cond
        "ok" -> StaticBlockEndsEvent.Ok
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticBlockEndsEvent): String? = when (event) {
        is StaticBlockEndsEvent.Error.Execution -> "error.execution"
        is StaticBlockEndsEvent.Fail.Assign -> "fail.assign"
        is StaticBlockEndsEvent.Fail.Branch -> "fail.branch"
        is StaticBlockEndsEvent.Fail.Cond -> "fail.cond"
        is StaticBlockEndsEvent.Ok -> "ok"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticBlockEndsState,
        event: StaticBlockEndsEvent?
    ): EnabledTransition<StaticBlockEndsState, HistoryId>? = when (state) {
        is StaticBlockEndsState.Waiting -> when {
            event is StaticBlockEndsEvent.Fail.Assign -> transitionWaitingAt0
            event is StaticBlockEndsEvent.Fail.Cond -> transitionWaitingAt1
            event is StaticBlockEndsEvent.Fail.Branch -> transitionWaitingAt2
            event is StaticBlockEndsEvent.Ok -> transitionWaitingAt3
            event is StaticBlockEndsEvent.Error.Execution -> transitionWaitingAt4
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_block_ends.scxml:24 :: _machine
    override fun onEntry(state: StaticBlockEndsState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticBlockEndsState.Waiting -> {
                // SCE-MAP: static_block_ends.scxml:38 :: waiting :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_block_ends.scxml:24 :: _machine
    override fun onExit(state: StaticBlockEndsState) {
        when (state) {
            is StaticBlockEndsState.Waiting -> {
                // SCE-MAP: static_block_ends.scxml:38 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_block_ends.scxml:24 :: _machine
    override fun executeTransitionContent(source: StaticBlockEndsState, transitionIndex: Int) {
        when (source) {
        is StaticBlockEndsState.Waiting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_block_ends.scxml:39 :: waiting :: _transition_0

            if (try { vA = com.sce.forge.runtime.SceChecked.add(vA, 10.toUByte()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBlockEndsEvent.Error.Execution, "<assign location='a'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vAfterAssign = 1.toUByte()
            }
            1 -> {
                // SCE-MAP: static_block_ends.scxml:43 :: waiting :: _transition_1


            var ifCondFailed1 = false
            if ((try { com.sce.forge.runtime.SceChecked.add(vA, 10.toUByte()) > 0.toUByte() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBlockEndsEvent.Error.Execution, "<if cond='a + 10 > 0'>: an integer operation overflowed or failed"); ifCondFailed1 = true; false })) {

            vThenRan = 1.toUByte()
            } else {

            vElseRan = 1.toUByte()
            }
            if (ifCondFailed1) {
                return
            }

            vAfterIf = 1.toUByte()
            }
            2 -> {
                // SCE-MAP: static_block_ends.scxml:51 :: waiting :: _transition_2


            if (vA > 0.toUByte()) {

            if (try { vA = com.sce.forge.runtime.SceChecked.add(vA, 10.toUByte()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBlockEndsEvent.Error.Execution, "<assign location='a'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vInBranch = 1.toUByte()
            }

            vAfterBranch = 1.toUByte()
            }
            3 -> {
                // SCE-MAP: static_block_ends.scxml:58 :: waiting :: _transition_3

            if (try { vB = com.sce.forge.runtime.SceChecked.add(vB, 1.toUByte()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBlockEndsEvent.Error.Execution, "<assign location='b'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vAfterOk = 1.toUByte()
            }
            4 -> {
                // SCE-MAP: static_block_ends.scxml:62 :: waiting :: _transition_4

            if (try { vErrors = com.sce.forge.runtime.SceChecked.add(vErrors, 1.toUByte()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBlockEndsEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
