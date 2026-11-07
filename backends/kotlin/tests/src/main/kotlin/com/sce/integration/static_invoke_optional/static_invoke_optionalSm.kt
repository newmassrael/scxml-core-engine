// SCE-GENERATED — DO NOT EDIT
// source-hash: 22249655389e33dabe1467cad0f6465e75f1c83f40cd0207380a1fc15324f922

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_invoke_optional.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_invoke_optional.scxml:34 :: _machine

package com.sce.integration.static_invoke_optional

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticInvokeOptionalState : State {
    data object Running : StaticInvokeOptionalState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticInvokeOptionalEvent : Event {
    sealed interface Done : StaticInvokeOptionalEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Absent : Invoke
            data object Fits : Invoke
            data object Given : Invoke
            data object Lifted : Invoke
            data object Over : Invoke
            data object Replaced : Invoke
        }
    }
    sealed interface Error : StaticInvokeOptionalEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class StaticInvokeOptionalStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticInvokeOptionalState, StaticInvokeOptionalEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `size` datamodel variable, the machine's own. */
    private var vSize: Int? = 7
    /** W3C SCXML 5.2: the `label` datamodel variable, the machine's own. */
    private var vLabel: String? = "abc"
    /** W3C SCXML 5.2: the `nosize` datamodel variable, the machine's own. */
    private var vNosize: Int? = null
    /** W3C SCXML 5.2: the `lengthy` datamodel variable, the machine's own. */
    private var vLengthy: String = "abcdefgh"
    /** W3C SCXML 5.2: the `completed` datamodel variable, published (`sce:direction="out"`). */
    private var vCompleted: UInt = 0.toUInt()

    /** What the published `completed` holds. */
    val completed: UInt get() = vCompleted
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    private var vErrors: UInt = 0.toUInt()

    /** What the published `errors` holds. */
    val errors: UInt get() = vErrors

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var size: Int? = null
        var label: String? = null
        var nosize: Int? = null
        var lengthy: String? = null
        var completed: UInt? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.size?.let { vSize = it }
        params.label?.let { vLabel = it }
        params.nosize?.let { vNosize = it }
        params.lengthy?.let { vLengthy = it }
        params.completed?.let { vCompleted = it }
        params.errors?.let { vErrors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val completed: UInt,
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
        val configuration: Set<StaticInvokeOptionalState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        completed = vCompleted,
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
        first = "5d7741d7e9d3bf0f39a4d38ceac4dd1ee3516ad684e8ded44134bf4c814efb8c",
        revised = "0c3699e9d5792b59d2b8867ecd6d350d058df394138b0605552a7c5350b7e8f0",
        format2First = "5d7741d7e9d3bf0f39a4d38ceac4dd1ee3516ad684e8ded44134bf4c814efb8c",
        format2Revised = "0c3699e9d5792b59d2b8867ecd6d350d058df394138b0605552a7c5350b7e8f0",
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
        variables += "size" to SavedValues.optional(vSize) { SavedValues.of(it) }
        variables += "label" to SavedValues.optional(vLabel) { SavedValues.of(it) }
        variables += "nosize" to SavedValues.optional(vNosize) { SavedValues.of(it) }
        variables += "lengthy" to SavedValues.of(vLengthy)
        variables += "completed" to SavedValues.of(vCompleted)
        variables += "errors" to SavedValues.of(vErrors)
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
        val writeVariablesPart0 = restoreVariablesPart0(savedGrown)
        writeVariablesPart0()
        enterSaved(savedGrown, wallNowMs)
    }

    // Reads this part's variables from [savedGrown], refusing a value its
    // variable's type cannot hold, and answers what writes them: nothing is
    // written until every part has read.
    private fun restoreVariablesPart0(savedGrown: SavedState): () -> Unit {
        val saved1 = SavedValues.optional(savedGrown.variable("size"), "size") { e, w -> SavedValues.int32(e, w) }
        val saved2 = SavedValues.optional(savedGrown.variable("label"), "label") { e, w -> SavedValues.string(e, w, 8) }
        val saved3 = SavedValues.optional(savedGrown.variable("nosize"), "nosize") { e, w -> SavedValues.int32(e, w) }
        val saved4 = SavedValues.string(savedGrown.variable("lengthy"), "lengthy", 8)
        val saved5 = SavedValues.uint32(savedGrown.variable("completed"), "completed")
        val saved6 = SavedValues.uint32(savedGrown.variable("errors"), "errors")
        return {
            vSize = saved1
            vLabel = saved2
            vNosize = saved3
            vLengthy = saved4
            vCompleted = saved5
            vErrors = saved6
        }
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
        SavedValues.optional(savedGrown.variable("size"), "size") { e, w -> SavedValues.int32(e, w) }
        SavedValues.optional(savedGrown.variable("label"), "label") { e, w -> SavedValues.string(e, w, 8) }
        SavedValues.optional(savedGrown.variable("nosize"), "nosize") { e, w -> SavedValues.int32(e, w) }
        SavedValues.string(savedGrown.variable("lengthy"), "lengthy", 8)
        SavedValues.uint32(savedGrown.variable("completed"), "completed")
        SavedValues.uint32(savedGrown.variable("errors"), "errors")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4: a saved state names the `<invoke>`s whose child is running,
    // and a restore starts each again from its beginning. This document saves
    // only when every invoke is a static child session, a hybrid one among
    // declared candidates, or one a declared host invoker serves.
    override val staticInvokes: List<Pair<String, StaticInvokeOptionalState>> = listOf(
        "given" to StaticInvokeOptionalState.Running,
        "lifted" to StaticInvokeOptionalState.Running,
        "absent" to StaticInvokeOptionalState.Running,
        "replaced" to StaticInvokeOptionalState.Running,
        "over" to StaticInvokeOptionalState.Running,
        "fits" to StaticInvokeOptionalState.Running,
    )

    override fun restartInvoke(invokeId: String, state: StaticInvokeOptionalState) = deferStaticInvoke(invokeId, state)

    override val initialState: StaticInvokeOptionalState = StaticInvokeOptionalState.Running

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
    override val documentInitialTargets: List<EntryTarget<StaticInvokeOptionalState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticInvokeOptionalState, HistoryId>> =
            listOf(StateTarget(StaticInvokeOptionalState.Running))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: running's transition 0, as the microstep reads it.
        val transitionRunningAt0 = EnabledTransition<StaticInvokeOptionalState, HistoryId>(
            StaticInvokeOptionalState.Running,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 1, as the microstep reads it.
        val transitionRunningAt1 = EnabledTransition<StaticInvokeOptionalState, HistoryId>(
            StaticInvokeOptionalState.Running,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 2, as the microstep reads it.
        val transitionRunningAt2 = EnabledTransition<StaticInvokeOptionalState, HistoryId>(
            StaticInvokeOptionalState.Running,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 3, as the microstep reads it.
        val transitionRunningAt3 = EnabledTransition<StaticInvokeOptionalState, HistoryId>(
            StaticInvokeOptionalState.Running,
            emptyList(),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 4, as the microstep reads it.
        val transitionRunningAt4 = EnabledTransition<StaticInvokeOptionalState, HistoryId>(
            StaticInvokeOptionalState.Running,
            emptyList(),
            4,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 5, as the microstep reads it.
        val transitionRunningAt5 = EnabledTransition<StaticInvokeOptionalState, HistoryId>(
            StaticInvokeOptionalState.Running,
            emptyList(),
            5,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 6, as the microstep reads it.
        val transitionRunningAt6 = EnabledTransition<StaticInvokeOptionalState, HistoryId>(
            StaticInvokeOptionalState.Running,
            emptyList(),
            6,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticInvokeOptionalState? = when (stateId) {
        "running" -> StaticInvokeOptionalState.Running
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticInvokeOptionalState): String = when (state) {
        is StaticInvokeOptionalState.Running -> "running"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticInvokeOptionalState): Int = when (state) {
        is StaticInvokeOptionalState.Running -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticInvokeOptionalEvent? = when (name) {
        "done.invoke" -> StaticInvokeOptionalEvent.Done.Invoke.Self
        "done.invoke.absent" -> StaticInvokeOptionalEvent.Done.Invoke.Absent
        "done.invoke.fits" -> StaticInvokeOptionalEvent.Done.Invoke.Fits
        "done.invoke.given" -> StaticInvokeOptionalEvent.Done.Invoke.Given
        "done.invoke.lifted" -> StaticInvokeOptionalEvent.Done.Invoke.Lifted
        "done.invoke.over" -> StaticInvokeOptionalEvent.Done.Invoke.Over
        "done.invoke.replaced" -> StaticInvokeOptionalEvent.Done.Invoke.Replaced
        "error.execution" -> StaticInvokeOptionalEvent.Error.Execution
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticInvokeOptionalEvent): String? = when (event) {
        is StaticInvokeOptionalEvent.Done.Invoke.Self -> "done.invoke"
        is StaticInvokeOptionalEvent.Done.Invoke.Absent -> "done.invoke.absent"
        is StaticInvokeOptionalEvent.Done.Invoke.Fits -> "done.invoke.fits"
        is StaticInvokeOptionalEvent.Done.Invoke.Given -> "done.invoke.given"
        is StaticInvokeOptionalEvent.Done.Invoke.Lifted -> "done.invoke.lifted"
        is StaticInvokeOptionalEvent.Done.Invoke.Over -> "done.invoke.over"
        is StaticInvokeOptionalEvent.Done.Invoke.Replaced -> "done.invoke.replaced"
        is StaticInvokeOptionalEvent.Error.Execution -> "error.execution"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticInvokeOptionalState, event: StaticInvokeOptionalEvent?): EnabledTransition<StaticInvokeOptionalState, HistoryId>? = when (state) {
        is StaticInvokeOptionalState.Running -> firstEnabledTransitionOfRunning(state, event)
    }


    // running's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRunning(state: StaticInvokeOptionalState, event: StaticInvokeOptionalEvent?): EnabledTransition<StaticInvokeOptionalState, HistoryId>? = when {
        event is StaticInvokeOptionalEvent.Done.Invoke.Given -> TransitionsPart0.transitionRunningAt0
        event is StaticInvokeOptionalEvent.Done.Invoke.Lifted -> TransitionsPart0.transitionRunningAt1
        event is StaticInvokeOptionalEvent.Done.Invoke.Absent -> TransitionsPart0.transitionRunningAt2
        event is StaticInvokeOptionalEvent.Done.Invoke.Replaced -> TransitionsPart0.transitionRunningAt3
        event is StaticInvokeOptionalEvent.Done.Invoke.Over -> TransitionsPart0.transitionRunningAt4
        event is StaticInvokeOptionalEvent.Done.Invoke.Fits -> TransitionsPart0.transitionRunningAt5
        event is StaticInvokeOptionalEvent.Error.Execution -> TransitionsPart0.transitionRunningAt6
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_invoke_optional.scxml:34 :: _machine
    override fun onEntry(state: StaticInvokeOptionalState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticInvokeOptionalState.Running -> onEntryOfRunning()
        }
    }


    private fun onEntryOfRunning() {
                // SCE-MAP: static_invoke_optional.scxml:44 :: running :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("given", StaticInvokeOptionalState.Running)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("lifted", StaticInvokeOptionalState.Running)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("absent", StaticInvokeOptionalState.Running)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("replaced", StaticInvokeOptionalState.Running)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("over", StaticInvokeOptionalState.Running)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("fits", StaticInvokeOptionalState.Running)
    }

    // W3C SCXML 6.4: defer the start of the child session of the
    // `<invoke type="scxml">` (a static child, or a hybrid one that names its
    // child by `srcexpr`) `invokeId`, held by `state`, to the macrostep's
    // end. One body for the two things that start a child: entering the state
    // (`onEntry` above) and a restore, which starts again each running
    // invocation a saved state lists (`restartInvoke`). The child is not saved,
    // so what a restore needs is exactly what entering the state does, and two
    // spellings of it would be two places a change to one is forgotten in the
    // other. A state that exits before the macrostep ends cancels the entry
    // (`cancelPendingInvokesForState`) in either case.
    //
    // Each invoke's start is a method of its own (`startInvokeOf<State>At<n>`,
    // `n` its place among its state's invokes), and this dispatches to it with
    // one call an arm, for the reason `onEntry` does.
    private fun deferStaticInvoke(invokeId: String, state: StaticInvokeOptionalState) {
        when (invokeId) {
            "given" -> startInvokeOfRunningAt0(state)
            "lifted" -> startInvokeOfRunningAt1(state)
            "absent" -> startInvokeOfRunningAt2(state)
            "replaced" -> startInvokeOfRunningAt3(state)
            "over" -> startInvokeOfRunningAt4(state)
            "fits" -> startInvokeOfRunningAt5(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfRunningAt0(state: StaticInvokeOptionalState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "running.${nextInvokeStart("given")}.given"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(StaticInvokeOptionalSceSynthInvokeGivenStateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                    val childSeed = StaticInvokeOptionalSceSynthInvokeGivenStateMachine.InvokeParams()
                    childSeed.size = vSize
                    try {
                        childSeed.label = (vLabel)?.let { scePresent -> com.sce.forge.runtime.SceChecked.bounded(scePresent, 8) }
                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                        // W3C SCXML 5.7.1: report the failure and leave the value out.
                        raisePlatformError(StaticInvokeOptionalEvent.Error.Execution, "<invoke> <param name='label'> expr failed to evaluate")
                    }
                    childSM.acceptParams(childSeed)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("given", childSM, false, StaticInvokeOptionalEvent.Done.Invoke.Given, "", generatedInvokeId, "", "static_invoke_optional__sce_synth_invoke__given", "given")
                }
            }
    }

    private fun startInvokeOfRunningAt1(state: StaticInvokeOptionalState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "running.${nextInvokeStart("lifted")}.lifted"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(StaticInvokeOptionalSceSynthInvokeLiftedStateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                    val childSeed = StaticInvokeOptionalSceSynthInvokeLiftedStateMachine.InvokeParams()
                    childSeed.size = 5
                    try {
                        childSeed.label = com.sce.forge.runtime.SceChecked.bounded("xy", 8)
                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                        // W3C SCXML 5.7.1: report the failure and leave the value out.
                        raisePlatformError(StaticInvokeOptionalEvent.Error.Execution, "<invoke> <param name='label'> expr failed to evaluate")
                    }
                    childSM.acceptParams(childSeed)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("lifted", childSM, false, StaticInvokeOptionalEvent.Done.Invoke.Lifted, "", generatedInvokeId, "", "static_invoke_optional__sce_synth_invoke__lifted", "lifted")
                }
            }
    }

    private fun startInvokeOfRunningAt2(state: StaticInvokeOptionalState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "running.${nextInvokeStart("absent")}.absent"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(StaticInvokeOptionalSceSynthInvokeAbsentStateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                    val childSeed = StaticInvokeOptionalSceSynthInvokeAbsentStateMachine.InvokeParams()
                    childSeed.size = vNosize
                    childSeed.label = null
                    childSM.acceptParams(childSeed)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("absent", childSM, false, StaticInvokeOptionalEvent.Done.Invoke.Absent, "", generatedInvokeId, "", "static_invoke_optional__sce_synth_invoke__absent", "absent")
                }
            }
    }

    private fun startInvokeOfRunningAt3(state: StaticInvokeOptionalState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "running.${nextInvokeStart("replaced")}.replaced"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(StaticInvokeOptionalSceSynthInvokeReplacedStateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                    val childSeed = StaticInvokeOptionalSceSynthInvokeReplacedStateMachine.InvokeParams()
                    childSeed.size = vSize ?: 0
                    childSM.acceptParams(childSeed)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("replaced", childSM, false, StaticInvokeOptionalEvent.Done.Invoke.Replaced, "", generatedInvokeId, "", "static_invoke_optional__sce_synth_invoke__replaced", "replaced")
                }
            }
    }

    private fun startInvokeOfRunningAt4(state: StaticInvokeOptionalState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "running.${nextInvokeStart("over")}.over"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(StaticInvokeOptionalSceSynthInvokeOverStateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                    val childSeed = StaticInvokeOptionalSceSynthInvokeOverStateMachine.InvokeParams()
                    try {
                        childSeed.label = com.sce.forge.runtime.SceChecked.bounded(vLengthy, 4)
                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                        // W3C SCXML 5.7.1: report the failure and leave the value out.
                        raisePlatformError(StaticInvokeOptionalEvent.Error.Execution, "<invoke> <param name='label'> expr failed to evaluate")
                    }
                    childSM.acceptParams(childSeed)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("over", childSM, false, StaticInvokeOptionalEvent.Done.Invoke.Over, "", generatedInvokeId, "", "static_invoke_optional__sce_synth_invoke__over", "over")
                }
            }
    }

    private fun startInvokeOfRunningAt5(state: StaticInvokeOptionalState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "running.${nextInvokeStart("fits")}.fits"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(StaticInvokeOptionalSceSynthInvokeFitsStateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                    val childSeed = StaticInvokeOptionalSceSynthInvokeFitsStateMachine.InvokeParams()
                    try {
                        childSeed.label = com.sce.forge.runtime.SceChecked.bounded("wxyz", 4)
                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                        // W3C SCXML 5.7.1: report the failure and leave the value out.
                        raisePlatformError(StaticInvokeOptionalEvent.Error.Execution, "<invoke> <param name='label'> expr failed to evaluate")
                    }
                    childSM.acceptParams(childSeed)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("fits", childSM, false, StaticInvokeOptionalEvent.Done.Invoke.Fits, "", generatedInvokeId, "", "static_invoke_optional__sce_synth_invoke__fits", "fits")
                }
            }
    }

    // docs/adr/0045, section 3, steps 1 and 3: the machine of a running child a
    // saved state lists, built as the `<invoke>` that started it builds it and
    // with what a start does with the child once built. Nothing is evaluated here:
    // no `<param>`, no `srcexpr`, no `idlocation` — the child resumes where it
    // stood, and a hybrid child is the candidate its saved `document` names,
    // whatever the `srcexpr` reads now.
    //
    // Each invoke's child is built by a method of its own
    // (`savedChildOf<State>At<n>`), and this dispatches to it with one
    // call an arm, for the reason `onEntry` does.
    override fun buildSavedChild(child: SavedChild): ResumedChild<StaticInvokeOptionalEvent> = when (child.id) {
        "given" -> savedChildOfRunningAt0(child)
        "lifted" -> savedChildOfRunningAt1(child)
        "absent" -> savedChildOfRunningAt2(child)
        "replaced" -> savedChildOfRunningAt3(child)
        "over" -> savedChildOfRunningAt4(child)
        "fits" -> savedChildOfRunningAt5(child)
        else -> throw StateRefusal("the document has no child session '${child.id}'")
    }


    private fun savedChildOfRunningAt0(child: SavedChild): ResumedChild<StaticInvokeOptionalEvent> =
        if (child.document == "static_invoke_optional__sce_synth_invoke__given") {
            ResumedChild(
                StaticInvokeOptionalSceSynthInvokeGivenStateMachine(),
                false, StaticInvokeOptionalEvent.Done.Invoke.Given, "", "", "given",
            )
        } else {
            throw StateRefusal("invokes names '${child.id}' running '${child.document}', a document that `<invoke>` does not start")
        }

    private fun savedChildOfRunningAt1(child: SavedChild): ResumedChild<StaticInvokeOptionalEvent> =
        if (child.document == "static_invoke_optional__sce_synth_invoke__lifted") {
            ResumedChild(
                StaticInvokeOptionalSceSynthInvokeLiftedStateMachine(),
                false, StaticInvokeOptionalEvent.Done.Invoke.Lifted, "", "", "lifted",
            )
        } else {
            throw StateRefusal("invokes names '${child.id}' running '${child.document}', a document that `<invoke>` does not start")
        }

    private fun savedChildOfRunningAt2(child: SavedChild): ResumedChild<StaticInvokeOptionalEvent> =
        if (child.document == "static_invoke_optional__sce_synth_invoke__absent") {
            ResumedChild(
                StaticInvokeOptionalSceSynthInvokeAbsentStateMachine(),
                false, StaticInvokeOptionalEvent.Done.Invoke.Absent, "", "", "absent",
            )
        } else {
            throw StateRefusal("invokes names '${child.id}' running '${child.document}', a document that `<invoke>` does not start")
        }

    private fun savedChildOfRunningAt3(child: SavedChild): ResumedChild<StaticInvokeOptionalEvent> =
        if (child.document == "static_invoke_optional__sce_synth_invoke__replaced") {
            ResumedChild(
                StaticInvokeOptionalSceSynthInvokeReplacedStateMachine(),
                false, StaticInvokeOptionalEvent.Done.Invoke.Replaced, "", "", "replaced",
            )
        } else {
            throw StateRefusal("invokes names '${child.id}' running '${child.document}', a document that `<invoke>` does not start")
        }

    private fun savedChildOfRunningAt4(child: SavedChild): ResumedChild<StaticInvokeOptionalEvent> =
        if (child.document == "static_invoke_optional__sce_synth_invoke__over") {
            ResumedChild(
                StaticInvokeOptionalSceSynthInvokeOverStateMachine(),
                false, StaticInvokeOptionalEvent.Done.Invoke.Over, "", "", "over",
            )
        } else {
            throw StateRefusal("invokes names '${child.id}' running '${child.document}', a document that `<invoke>` does not start")
        }

    private fun savedChildOfRunningAt5(child: SavedChild): ResumedChild<StaticInvokeOptionalEvent> =
        if (child.document == "static_invoke_optional__sce_synth_invoke__fits") {
            ResumedChild(
                StaticInvokeOptionalSceSynthInvokeFitsStateMachine(),
                false, StaticInvokeOptionalEvent.Done.Invoke.Fits, "", "", "fits",
            )
        } else {
            throw StateRefusal("invokes names '${child.id}' running '${child.document}', a document that `<invoke>` does not start")
        }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_invoke_optional.scxml:34 :: _machine
    override fun onExit(state: StaticInvokeOptionalState) {
        when (state) {
            is StaticInvokeOptionalState.Running -> onExitOfRunning()
        }
    }


    private fun onExitOfRunning() {
                // SCE-MAP: static_invoke_optional.scxml:44 :: running :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(StaticInvokeOptionalState.Running)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("given")
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("lifted")
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("absent")
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("replaced")
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("over")
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("fits")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_invoke_optional.scxml:34 :: _machine
    override fun executeTransitionContent(source: StaticInvokeOptionalState, transitionIndex: Int) {
        when (source) {
            is StaticInvokeOptionalState.Running -> transitionContentOfRunning(transitionIndex)
        }
    }


    // The content of `running`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfRunning(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfRunningAt0()
            1 -> transitionContentOfRunningAt1()
            2 -> transitionContentOfRunningAt2()
            3 -> transitionContentOfRunningAt3()
            4 -> transitionContentOfRunningAt4()
            5 -> transitionContentOfRunningAt5()
            6 -> transitionContentOfRunningAt6()
            else -> {}
        }
    }


    private fun transitionContentOfRunningAt0() {
                // SCE-MAP: static_invoke_optional.scxml:141 :: running :: _transition_0

            if (try { vCompleted = com.sce.forge.runtime.SceChecked.add(vCompleted, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeOptionalEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfRunningAt1() {
                // SCE-MAP: static_invoke_optional.scxml:144 :: running :: _transition_1

            if (try { vCompleted = com.sce.forge.runtime.SceChecked.add(vCompleted, 10.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeOptionalEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfRunningAt2() {
                // SCE-MAP: static_invoke_optional.scxml:147 :: running :: _transition_2

            if (try { vCompleted = com.sce.forge.runtime.SceChecked.add(vCompleted, 100.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeOptionalEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfRunningAt3() {
                // SCE-MAP: static_invoke_optional.scxml:150 :: running :: _transition_3

            if (try { vCompleted = com.sce.forge.runtime.SceChecked.add(vCompleted, 1000.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeOptionalEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfRunningAt4() {
                // SCE-MAP: static_invoke_optional.scxml:153 :: running :: _transition_4

            if (try { vCompleted = com.sce.forge.runtime.SceChecked.add(vCompleted, 10000.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeOptionalEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfRunningAt5() {
                // SCE-MAP: static_invoke_optional.scxml:156 :: running :: _transition_5

            if (try { vCompleted = com.sce.forge.runtime.SceChecked.add(vCompleted, 100000.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeOptionalEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfRunningAt6() {
                // SCE-MAP: static_invoke_optional.scxml:159 :: running :: _transition_6

            if (try { vErrors = com.sce.forge.runtime.SceChecked.add(vErrors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeOptionalEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
