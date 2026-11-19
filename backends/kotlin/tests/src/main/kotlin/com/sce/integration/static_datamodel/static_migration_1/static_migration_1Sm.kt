// SCE-GENERATED — DO NOT EDIT
// source-hash: 7b9457f46d7c409f542b760731e4138b865f26e4412ea738113cfca71ced024d

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_migration_1.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_migration_1.scxml:14 :: _machine

package com.sce.integration.static_datamodel.static_migration_1

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticMigration1State : State {
    data object Done : StaticMigration1State
    data object Idle : StaticMigration1State
    data object Waiting : StaticMigration1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticMigration1Event : Event {
    data object Finish : StaticMigration1Event
    data object Go : StaticMigration1Event
    data object Tick : StaticMigration1Event
}
// --- State Machine (W3C SCXML) ---

class StaticMigration1StateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticMigration1State, StaticMigration1Event>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `count` datamodel variable, published (`sce:direction="out"`). */
    private var vCount: UInt = 0.toUInt()

    /** What the published `count` holds. */
    val count: UInt get() = vCount
    /** W3C SCXML 5.2: the `total` datamodel variable, published (`sce:direction="out"`). */
    private var vTotal: Int = 0

    /** What the published `total` holds. */
    val total: Int get() = vTotal

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var count: UInt? = null
        var total: Int? = null
    }

    /**
     * §scxml-6.4: give this machine the values [params] carries, each as the
     * initial value of its variable in place of the one its `<data>` gives, so
     * that a variable declared after it whose initial value reads it reads the
     * handed value (docs/adr/0098). Called before [initialize].
     */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    @Suppress("UNUSED_PARAMETER")
    private fun acceptParamsPart0(params: InvokeParams) {
        vCount = params.count ?: (0.toUInt())
        vTotal = params.total ?: (0)
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val count: UInt,
        val total: Int,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticMigration1State>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        count = vCount,
        total = vTotal,
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
        first = "8b4c54d2716b8006fa82cbd2b90c80dfa97d8b1fc503668e9d3d34de4d9d53ca",
        revised = "164972d82811016fd65905b0e4a3fe98ee8d2f86512d10db4df0add5336f926d",
        format2First = "8b4c54d2716b8006fa82cbd2b90c80dfa97d8b1fc503668e9d3d34de4d9d53ca",
        format2Revised = "164972d82811016fd65905b0e4a3fe98ee8d2f86512d10db4df0add5336f926d",
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
    fun save(wallNowMs: Long): SavedState {
        val variables = LinkedHashMap<String, Any?>()
        saveVariablesPart0(variables)
        return savedState(savedShape, variables, wallNowMs)
    }

    private fun saveVariablesPart0(variables: MutableMap<String, Any?>) {
        variables += "count" to SavedValues.of(vCount)
        variables += "total" to SavedValues.of(vTotal)
    }

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
        // Every part is judged before any is written, so a refused value leaves
        // the machine as it was; then each part reads its values again and
        // writes them. A part answers no closure over the values it read: a
        // closure's captures are the arguments of a JVM method, at most 255
        // slots with a `Long` or a `Double` taking two, and a part of
        // `kotlin_method_items` variables passed that, so the class failed to
        // load (`ClassFormatError: Too many arguments in method signature`).
        judgeVariablesPart0(savedGrown)
        restoreVariablesPart0(savedGrown)
        enterSaved(savedGrown, wallNowMs)
    }

    // Writes this part's variables from [savedGrown], which every judge part
    // has already held to its variables' types.
    private fun restoreVariablesPart0(savedGrown: SavedState) {
        vCount = SavedValues.uint32(savedGrown.variable("count"), "count")
        vTotal = SavedValues.int32(savedGrown.variable("total"), "total")
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // docs/adr/0045: this machine as a child session of another's saved state —
    // saved with it, judged with it before anything is built, and restored in
    // place of initialized.
    override fun saveAsSession(wallNowMs: Long): SavedState = save(wallNowMs)

    override fun judgeAsSession(saved: SavedState) {
        val savedGrown = beginRestore(saved, savedShape)
        judgeVariablesPart0(savedGrown)
    }

    private fun judgeVariablesPart0(savedGrown: SavedState) {
        SavedValues.uint32(savedGrown.variable("count"), "count")
        SavedValues.int32(savedGrown.variable("total"), "total")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    override val initialState: StaticMigration1State = StaticMigration1State.Idle

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
    override fun isFinalState(state: StaticMigration1State): Boolean = when (state) {
        is StaticMigration1State.Done -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticMigration1State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticMigration1State, HistoryId>> =
            listOf(StateTarget(StaticMigration1State.Idle))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticMigration1State, HistoryId>(
            StaticMigration1State.Idle,
            listOf(StateTarget(StaticMigration1State.Waiting)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StaticMigration1State, HistoryId>(
            StaticMigration1State.Waiting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<StaticMigration1State, HistoryId>(
            StaticMigration1State.Waiting,
            listOf(StateTarget(StaticMigration1State.Done)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticMigration1State? = when (stateId) {
        "done" -> StaticMigration1State.Done
        "idle" -> StaticMigration1State.Idle
        "waiting" -> StaticMigration1State.Waiting
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticMigration1State): String = when (state) {
        is StaticMigration1State.Done -> "done"
        is StaticMigration1State.Idle -> "idle"
        is StaticMigration1State.Waiting -> "waiting"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticMigration1State): Int = when (state) {
        is StaticMigration1State.Done -> 2
        is StaticMigration1State.Idle -> 0
        is StaticMigration1State.Waiting -> 1
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticMigration1Event? = when (name) {
        "finish" -> StaticMigration1Event.Finish
        "go" -> StaticMigration1Event.Go
        "tick" -> StaticMigration1Event.Tick
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticMigration1Event): String? = when (event) {
        is StaticMigration1Event.Finish -> "finish"
        is StaticMigration1Event.Go -> "go"
        is StaticMigration1Event.Tick -> "tick"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticMigration1State, event: StaticMigration1Event?): EnabledTransition<StaticMigration1State, HistoryId>? = when (state) {
        is StaticMigration1State.Idle -> firstEnabledTransitionOfIdle(state, event)
        is StaticMigration1State.Waiting -> firstEnabledTransitionOfWaiting(state, event)
        else -> null
    }


    // idle's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdle(state: StaticMigration1State, event: StaticMigration1Event?): EnabledTransition<StaticMigration1State, HistoryId>? = when {
        event is StaticMigration1Event.Go -> TransitionsPart0.transitionIdleAt0
        else -> null
    }


    // waiting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWaiting(state: StaticMigration1State, event: StaticMigration1Event?): EnabledTransition<StaticMigration1State, HistoryId>? = when {
        event is StaticMigration1Event.Tick -> TransitionsPart0.transitionWaitingAt0
        event is StaticMigration1Event.Finish -> TransitionsPart0.transitionWaitingAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_migration_1.scxml:14 :: _machine
    override fun onEntry(state: StaticMigration1State, isDefaultEntry: Boolean) {
        when (state) {
            is StaticMigration1State.Done -> onEntryOfDone()
            is StaticMigration1State.Idle -> onEntryOfIdle()
            is StaticMigration1State.Waiting -> onEntryOfWaiting()
        }
    }


    private fun onEntryOfDone() {
                // SCE-MAP: static_migration_1.scxml:30 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfIdle() {
                // SCE-MAP: static_migration_1.scxml:20 :: idle :: _state_body
    }

    private fun onEntryOfWaiting() {
                // SCE-MAP: static_migration_1.scxml:23 :: waiting :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_migration_1.scxml:14 :: _machine
    override fun onExit(state: StaticMigration1State) {
        when (state) {
            is StaticMigration1State.Done -> onExitOfDone()
            is StaticMigration1State.Idle -> onExitOfIdle()
            is StaticMigration1State.Waiting -> onExitOfWaiting()
        }
    }


    private fun onExitOfDone() {
                // SCE-MAP: static_migration_1.scxml:30 :: done :: _state_body
    }

    private fun onExitOfIdle() {
                // SCE-MAP: static_migration_1.scxml:20 :: idle :: _state_body
    }

    private fun onExitOfWaiting() {
                // SCE-MAP: static_migration_1.scxml:23 :: waiting :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_migration_1.scxml:14 :: _machine
    override fun executeTransitionContent(source: StaticMigration1State, transitionIndex: Int) {
        when (source) {
            is StaticMigration1State.Waiting -> transitionContentOfWaiting(transitionIndex)
            else -> {}
        }
    }


    // The content of `waiting`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWaiting(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWaitingAt0()
            else -> {}
        }
    }


    private fun transitionContentOfWaitingAt0() {
                // SCE-MAP: static_migration_1.scxml:24 :: waiting :: _transition_0

            if (try { vCount = com.sce.forge.runtime.SceChecked.add(vCount, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }

            if (try { vTotal = com.sce.forge.runtime.SceChecked.sub(vTotal, 2); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
    }
}
