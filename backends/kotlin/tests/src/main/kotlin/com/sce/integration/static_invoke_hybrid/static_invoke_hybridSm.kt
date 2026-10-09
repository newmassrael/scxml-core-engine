// SCE-GENERATED — DO NOT EDIT
// source-hash: ef4ca0a4dd7e55791acb35707aca0211307a64b1955f47f3e92b31829dc36eec

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_invoke_hybrid.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_invoke_hybrid.scxml:33 :: _machine

package com.sce.integration.static_invoke_hybrid

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticInvokeHybridState : State {
    data object First : StaticInvokeHybridState
    data object Lossy : StaticInvokeHybridState
    data object Missing : StaticInvokeHybridState
    data object Over : StaticInvokeHybridState
    data object Run : StaticInvokeHybridState
    data object Second : StaticInvokeHybridState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticInvokeHybridEvent : Event {
    sealed interface Done : StaticInvokeHybridEvent {
        sealed interface Invoke : Done {
            data object FirstRun : Invoke
            data object LossyRun : Invoke
            data object MissingRun : Invoke
            data object SecondRun : Invoke
        }
    }
    sealed interface Error : StaticInvokeHybridEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class StaticInvokeHybridStateMachine(
) : StateMachineEngine<StaticInvokeHybridState, StaticInvokeHybridEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `pick` datamodel variable, the machine's own. */
    private var pick: String = "file:static_hybrid_first.scxml"
    /** W3C SCXML 5.2: the `base` datamodel variable, the machine's own. */
    private var base: UInt = 4.toUInt()
    /** W3C SCXML 5.2: the `enabled` datamodel variable, the machine's own. */
    private var enabled: Boolean = true
    /** W3C SCXML 5.2: the `completed` datamodel variable, published (`sce:direction="out"`). */
    var completed: UInt = 0.toUInt()
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
        var pick: String? = null
        var base: UInt? = null
        var enabled: Boolean? = null
        var completed: UInt? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.pick?.let { pick = it }
        params.base?.let { base = it }
        params.enabled?.let { enabled = it }
        params.completed?.let { completed = it }
        params.errors?.let { errors = it }
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
        val configuration: Set<StaticInvokeHybridState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        completed = completed,
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
    val savedShape: String = "972ab8ed39c25ae07affaee242de1ae666cf4699c734a946880fc52a11084868"

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
            "pick" to SavedValues.of(pick),
            "base" to SavedValues.of(base),
            "enabled" to SavedValues.of(enabled),
            "completed" to SavedValues.of(completed),
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
        val saved1 = SavedValues.string(saved.variable("pick"), "pick", 48)
        val saved2 = SavedValues.uint32(saved.variable("base"), "base")
        val saved3 = SavedValues.bool(saved.variable("enabled"), "enabled")
        val saved4 = SavedValues.uint32(saved.variable("completed"), "completed")
        val saved5 = SavedValues.uint32(saved.variable("errors"), "errors")
        pick = saved1
        base = saved2
        enabled = saved3
        completed = saved4
        errors = saved5
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // §scxml-6.4: a saved state names the `<invoke>`s whose child is running,
    // and a restore starts each again from its beginning. This document saves
    // only when every invoke is a static child session, a hybrid one among
    // declared candidates, or one a declared host invoker serves.
    override val staticInvokes: List<Pair<String, StaticInvokeHybridState>> = listOf(
        "first_run" to StaticInvokeHybridState.First,
        "second_run" to StaticInvokeHybridState.Second,
        "lossy_run" to StaticInvokeHybridState.Lossy,
        "missing_run" to StaticInvokeHybridState.Missing,
    )

    override fun restartInvoke(invokeId: String, state: StaticInvokeHybridState) = deferStaticInvoke(invokeId, state)

    override val initialState: StaticInvokeHybridState = StaticInvokeHybridState.First

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

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: StaticInvokeHybridState): StaticInvokeHybridState? = when (state) {
        is StaticInvokeHybridState.First -> StaticInvokeHybridState.Run
        is StaticInvokeHybridState.Lossy -> StaticInvokeHybridState.Run
        is StaticInvokeHybridState.Missing -> StaticInvokeHybridState.Run
        is StaticInvokeHybridState.Second -> StaticInvokeHybridState.Run
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: StaticInvokeHybridState): Boolean = when (state) {
        is StaticInvokeHybridState.Run -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: StaticInvokeHybridState): Boolean = when (state) {
        is StaticInvokeHybridState.Over -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: StaticInvokeHybridState): List<StaticInvokeHybridState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: StaticInvokeHybridState): List<EntryTarget<StaticInvokeHybridState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticInvokeHybridState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<StaticInvokeHybridState, List<StaticInvokeHybridState>> = mapOf(
            StaticInvokeHybridState.Run to listOf(StaticInvokeHybridState.First, StaticInvokeHybridState.Second, StaticInvokeHybridState.Lossy, StaticInvokeHybridState.Missing),
        )

        val initialTargets: Map<StaticInvokeHybridState, List<EntryTarget<StaticInvokeHybridState, HistoryId>>> = mapOf(
            StaticInvokeHybridState.Run to listOf(StateTarget(StaticInvokeHybridState.First)),
        )

        val documentInitialTargetList: List<EntryTarget<StaticInvokeHybridState, HistoryId>> =
            listOf(StateTarget(StaticInvokeHybridState.Run))

        // W3C SCXML 3.13: first's transition 0, as the microstep reads it.
        val transitionFirstAt0 = EnabledTransition<StaticInvokeHybridState, HistoryId>(
            StaticInvokeHybridState.First,
            listOf(StateTarget(StaticInvokeHybridState.Second)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: lossy's transition 0, as the microstep reads it.
        val transitionLossyAt0 = EnabledTransition<StaticInvokeHybridState, HistoryId>(
            StaticInvokeHybridState.Lossy,
            listOf(StateTarget(StaticInvokeHybridState.Missing)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: missing's transition 0, as the microstep reads it.
        val transitionMissingAt0 = EnabledTransition<StaticInvokeHybridState, HistoryId>(
            StaticInvokeHybridState.Missing,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: missing's transition 1, as the microstep reads it.
        val transitionMissingAt1 = EnabledTransition<StaticInvokeHybridState, HistoryId>(
            StaticInvokeHybridState.Missing,
            listOf(StateTarget(StaticInvokeHybridState.Over)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<StaticInvokeHybridState, HistoryId>(
            StaticInvokeHybridState.Run,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: second's transition 0, as the microstep reads it.
        val transitionSecondAt0 = EnabledTransition<StaticInvokeHybridState, HistoryId>(
            StaticInvokeHybridState.Second,
            listOf(StateTarget(StaticInvokeHybridState.Lossy)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticInvokeHybridState? = when (stateId) {
        "first" -> StaticInvokeHybridState.First
        "lossy" -> StaticInvokeHybridState.Lossy
        "missing" -> StaticInvokeHybridState.Missing
        "over" -> StaticInvokeHybridState.Over
        "run" -> StaticInvokeHybridState.Run
        "second" -> StaticInvokeHybridState.Second
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticInvokeHybridState): String = when (state) {
        is StaticInvokeHybridState.First -> "first"
        is StaticInvokeHybridState.Lossy -> "lossy"
        is StaticInvokeHybridState.Missing -> "missing"
        is StaticInvokeHybridState.Over -> "over"
        is StaticInvokeHybridState.Run -> "run"
        is StaticInvokeHybridState.Second -> "second"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticInvokeHybridState): Int = when (state) {
        is StaticInvokeHybridState.First -> 1
        is StaticInvokeHybridState.Lossy -> 3
        is StaticInvokeHybridState.Missing -> 4
        is StaticInvokeHybridState.Over -> 5
        is StaticInvokeHybridState.Run -> 0
        is StaticInvokeHybridState.Second -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticInvokeHybridEvent? = when (name) {
        "done.invoke.first_run" -> StaticInvokeHybridEvent.Done.Invoke.FirstRun
        "done.invoke.lossy_run" -> StaticInvokeHybridEvent.Done.Invoke.LossyRun
        "done.invoke.missing_run" -> StaticInvokeHybridEvent.Done.Invoke.MissingRun
        "done.invoke.second_run" -> StaticInvokeHybridEvent.Done.Invoke.SecondRun
        "error.execution" -> StaticInvokeHybridEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticInvokeHybridEvent): String? = when (event) {
        is StaticInvokeHybridEvent.Done.Invoke.FirstRun -> "done.invoke.first_run"
        is StaticInvokeHybridEvent.Done.Invoke.LossyRun -> "done.invoke.lossy_run"
        is StaticInvokeHybridEvent.Done.Invoke.MissingRun -> "done.invoke.missing_run"
        is StaticInvokeHybridEvent.Done.Invoke.SecondRun -> "done.invoke.second_run"
        is StaticInvokeHybridEvent.Error.Execution -> "error.execution"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticInvokeHybridState,
        event: StaticInvokeHybridEvent?
    ): EnabledTransition<StaticInvokeHybridState, HistoryId>? = when (state) {
        is StaticInvokeHybridState.First -> when {
            event is StaticInvokeHybridEvent.Done.Invoke.FirstRun -> transitionFirstAt0
            else -> null
        }
        is StaticInvokeHybridState.Lossy -> when {
            event is StaticInvokeHybridEvent.Done.Invoke.LossyRun -> transitionLossyAt0
            else -> null
        }
        is StaticInvokeHybridState.Missing -> when {
            event is StaticInvokeHybridEvent.Done.Invoke.MissingRun -> transitionMissingAt0
            event is StaticInvokeHybridEvent.Error.Execution -> transitionMissingAt1
            else -> null
        }
        is StaticInvokeHybridState.Run -> when {
            event is StaticInvokeHybridEvent.Error.Execution -> transitionRunAt0
            else -> null
        }
        is StaticInvokeHybridState.Second -> when {
            event is StaticInvokeHybridEvent.Done.Invoke.SecondRun -> transitionSecondAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_invoke_hybrid.scxml:33 :: _machine
    override fun onEntry(state: StaticInvokeHybridState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticInvokeHybridState.First -> {
                // SCE-MAP: static_invoke_hybrid.scxml:46 :: first :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            if (try { base = com.sce.forge.runtime.SceChecked.add(base, 3.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<assign location='base'>: an integer operation overflowed or failed"); true }) {
                return@run
            }
                }
                // W3C SCXML 6.4: Defer the hybrid invoke until macrostep end. `deferStaticInvoke`
                // starts it — for entering the state and for a restore alike, so the two
                // cannot start it differently.
                deferStaticInvoke("first_run", state)
            }
            is StaticInvokeHybridState.Lossy -> {
                // SCE-MAP: static_invoke_hybrid.scxml:72 :: lossy :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            if (try { pick = com.sce.forge.runtime.SceChecked.bounded("./static_hybrid_first.scxml", 48); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<assign location='pick'>: an integer operation overflowed or failed"); true }) {
                return@run
            }
                }
                // W3C SCXML 6.4: Defer the hybrid invoke until macrostep end. `deferStaticInvoke`
                // starts it — for entering the state and for a restore alike, so the two
                // cannot start it differently.
                deferStaticInvoke("lossy_run", state)
            }
            is StaticInvokeHybridState.Missing -> {
                // SCE-MAP: static_invoke_hybrid.scxml:85 :: missing :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            if (try { pick = com.sce.forge.runtime.SceChecked.bounded("static_hybrid_missing.scxml", 48); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<assign location='pick'>: an integer operation overflowed or failed"); true }) {
                return@run
            }
                }
                // W3C SCXML 6.4: Defer the hybrid invoke until macrostep end. `deferStaticInvoke`
                // starts it — for entering the state and for a restore alike, so the two
                // cannot start it differently.
                deferStaticInvoke("missing_run", state)
            }
            is StaticInvokeHybridState.Over -> {
                // SCE-MAP: static_invoke_hybrid.scxml:101 :: over :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StaticInvokeHybridState.Run -> {
                // SCE-MAP: static_invoke_hybrid.scxml:42 :: run :: _state_body
            }
            is StaticInvokeHybridState.Second -> {
                // SCE-MAP: static_invoke_hybrid.scxml:59 :: second :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            if (try { pick = com.sce.forge.runtime.SceChecked.bounded("/opt/charts/static_hybrid_second.scxml", 48); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<assign location='pick'>: an integer operation overflowed or failed"); true }) {
                return@run
            }
                }
                // W3C SCXML 6.4: Defer the hybrid invoke until macrostep end. `deferStaticInvoke`
                // starts it — for entering the state and for a restore alike, so the two
                // cannot start it differently.
                deferStaticInvoke("second_run", state)
            }
        }
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
    private fun deferStaticInvoke(invokeId: String, state: StaticInvokeHybridState) {
        when (invokeId) {
            "first_run" -> {
                // W3C SCXML 6.4: Hybrid invoke — runtime expression evaluation + dynamic child
                // C++ parity: StateMachine::createFromSCXMLString() / FileLoadingHelper::loadScxmlFile()
                run {
                    val generatedInvokeId = "first.${System.identityHashCode(this)}.first_run"
                    deferInvoke(state, generatedInvokeId) {
                        try {
                            // §scxml-6.4: the `srcexpr` names the document to start.
                            val filePath: String = pick
                            // §scxml-6.4 + SCE_ACCEPTED_SUBSET.md §2.13: the value
                            // chooses among the declared candidates, matched on the
                            // document stem so `file:x.scxml` and `./x.scxml` name
                            // one child. `startInvoke` takes a star-projected
                            // machine, so only the construction differs here.
                            val __sceSelected = DocumentStem.of(filePath)
                            val childSM = when (__sceSelected) {
                                "static_hybrid_first" -> StaticHybridFirstStateMachine().also { child ->
                                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                                    val childSeed = StaticHybridFirstStateMachine.InvokeParams()
                                    childSeed.start = base
                                    childSeed.enabled = enabled
                                    child.acceptParams(childSeed)
                                    // W3C SCXML 6.4.3: the child declares no `extra`, so the Processor MUST NOT
                                    // add it to its data model — but the argument is still the invoke's,
                                    // evaluated, and a failure of it is reported (W3C SCXML 5.7.1).
                                    try {
                                        (com.sce.forge.runtime.SceChecked.sub(base, 4.toUInt())).toLong()
                                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                                        raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke> <param name='extra'> expr failed to evaluate")
                                    }

                                }
                                "static_hybrid_second" -> StaticHybridSecondStateMachine().also { child ->
                                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                                    val childSeed = StaticHybridSecondStateMachine.InvokeParams()
                                    childSeed.start = base
                                    try {
                                        childSeed.extra = com.sce.forge.runtime.SceChecked.sub(base, 4.toUInt())
                                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                                        // W3C SCXML 5.7.1: report the failure and leave the value out.
                                        raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke> <param name='extra'> expr failed to evaluate")
                                    }
                                    child.acceptParams(childSeed)

                                }
                                else -> {
                                    // The failure the Interpreter reports when a
                                    // document will not load: nothing to create.
                                    raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke srcexpr> evaluated to a document it did not declare")
                                    return@deferInvoke
                                }
                            }

                            startInvoke("first_run", childSM, false, StaticInvokeHybridEvent.Done.Invoke.FirstRun, "", generatedInvokeId)
                        } catch (_: Exception) {
                            // W3C SCXML 6.4: the child could not be started. Evaluation
                            // failure no longer reaches here — it has its own raise above,
                            // under the one wording every emitter uses for that fact.
                            raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke> could not start a child")
                        }
                    }
                }
            }
            "lossy_run" -> {
                // W3C SCXML 6.4: Hybrid invoke — runtime expression evaluation + dynamic child
                // C++ parity: StateMachine::createFromSCXMLString() / FileLoadingHelper::loadScxmlFile()
                run {
                    val generatedInvokeId = "lossy.${System.identityHashCode(this)}.lossy_run"
                    deferInvoke(state, generatedInvokeId) {
                        try {
                            // §scxml-6.4: the `srcexpr` names the document to start.
                            val filePath: String = pick
                            // §scxml-6.4 + SCE_ACCEPTED_SUBSET.md §2.13: the value
                            // chooses among the declared candidates, matched on the
                            // document stem so `file:x.scxml` and `./x.scxml` name
                            // one child. `startInvoke` takes a star-projected
                            // machine, so only the construction differs here.
                            val __sceSelected = DocumentStem.of(filePath)
                            val childSM = when (__sceSelected) {
                                "static_hybrid_first" -> StaticHybridFirstStateMachine().also { child ->
                                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                                    val childSeed = StaticHybridFirstStateMachine.InvokeParams()
                                    childSeed.start = base
                                    childSeed.enabled = enabled
                                    child.acceptParams(childSeed)
                                    // W3C SCXML 6.4.3: the child declares no `extra`, so the Processor MUST NOT
                                    // add it to its data model — but the argument is still the invoke's,
                                    // evaluated, and a failure of it is reported (W3C SCXML 5.7.1).
                                    try {
                                        (com.sce.forge.runtime.SceChecked.mul(base, 2000000000.toUInt())).toLong()
                                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                                        raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke> <param name='extra'> expr failed to evaluate")
                                    }

                                }
                                "static_hybrid_second" -> StaticHybridSecondStateMachine().also { child ->
                                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                                    val childSeed = StaticHybridSecondStateMachine.InvokeParams()
                                    childSeed.start = base
                                    try {
                                        childSeed.extra = com.sce.forge.runtime.SceChecked.mul(base, 2000000000.toUInt())
                                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                                        // W3C SCXML 5.7.1: report the failure and leave the value out.
                                        raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke> <param name='extra'> expr failed to evaluate")
                                    }
                                    child.acceptParams(childSeed)

                                }
                                else -> {
                                    // The failure the Interpreter reports when a
                                    // document will not load: nothing to create.
                                    raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke srcexpr> evaluated to a document it did not declare")
                                    return@deferInvoke
                                }
                            }

                            startInvoke("lossy_run", childSM, false, StaticInvokeHybridEvent.Done.Invoke.LossyRun, "", generatedInvokeId)
                        } catch (_: Exception) {
                            // W3C SCXML 6.4: the child could not be started. Evaluation
                            // failure no longer reaches here — it has its own raise above,
                            // under the one wording every emitter uses for that fact.
                            raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke> could not start a child")
                        }
                    }
                }
            }
            "missing_run" -> {
                // W3C SCXML 6.4: Hybrid invoke — runtime expression evaluation + dynamic child
                // C++ parity: StateMachine::createFromSCXMLString() / FileLoadingHelper::loadScxmlFile()
                run {
                    val generatedInvokeId = "missing.${System.identityHashCode(this)}.missing_run"
                    deferInvoke(state, generatedInvokeId) {
                        try {
                            // §scxml-6.4: the `srcexpr` names the document to start.
                            val filePath: String = pick
                            // §scxml-6.4 + SCE_ACCEPTED_SUBSET.md §2.13: the value
                            // chooses among the declared candidates, matched on the
                            // document stem so `file:x.scxml` and `./x.scxml` name
                            // one child. `startInvoke` takes a star-projected
                            // machine, so only the construction differs here.
                            val __sceSelected = DocumentStem.of(filePath)
                            val childSM = when (__sceSelected) {
                                "static_hybrid_first" -> StaticHybridFirstStateMachine().also { child ->
                                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                                    val childSeed = StaticHybridFirstStateMachine.InvokeParams()
                                    childSeed.start = base
                                    childSeed.enabled = enabled
                                    child.acceptParams(childSeed)

                                }
                                "static_hybrid_second" -> StaticHybridSecondStateMachine().also { child ->
                                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                                    val childSeed = StaticHybridSecondStateMachine.InvokeParams()
                                    childSeed.start = base
                                    child.acceptParams(childSeed)

                                }
                                else -> {
                                    // The failure the Interpreter reports when a
                                    // document will not load: nothing to create.
                                    raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke srcexpr> evaluated to a document it did not declare")
                                    return@deferInvoke
                                }
                            }

                            startInvoke("missing_run", childSM, false, StaticInvokeHybridEvent.Done.Invoke.MissingRun, "", generatedInvokeId)
                        } catch (_: Exception) {
                            // W3C SCXML 6.4: the child could not be started. Evaluation
                            // failure no longer reaches here — it has its own raise above,
                            // under the one wording every emitter uses for that fact.
                            raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke> could not start a child")
                        }
                    }
                }
            }
            "second_run" -> {
                // W3C SCXML 6.4: Hybrid invoke — runtime expression evaluation + dynamic child
                // C++ parity: StateMachine::createFromSCXMLString() / FileLoadingHelper::loadScxmlFile()
                run {
                    val generatedInvokeId = "second.${System.identityHashCode(this)}.second_run"
                    deferInvoke(state, generatedInvokeId) {
                        try {
                            // §scxml-6.4: the `srcexpr` names the document to start.
                            val filePath: String = pick
                            // §scxml-6.4 + SCE_ACCEPTED_SUBSET.md §2.13: the value
                            // chooses among the declared candidates, matched on the
                            // document stem so `file:x.scxml` and `./x.scxml` name
                            // one child. `startInvoke` takes a star-projected
                            // machine, so only the construction differs here.
                            val __sceSelected = DocumentStem.of(filePath)
                            val childSM = when (__sceSelected) {
                                "static_hybrid_first" -> StaticHybridFirstStateMachine().also { child ->
                                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                                    val childSeed = StaticHybridFirstStateMachine.InvokeParams()
                                    childSeed.start = base
                                    childSeed.enabled = enabled
                                    child.acceptParams(childSeed)
                                    // W3C SCXML 6.4.3: the child declares no `extra`, so the Processor MUST NOT
                                    // add it to its data model — but the argument is still the invoke's,
                                    // evaluated, and a failure of it is reported (W3C SCXML 5.7.1).
                                    try {
                                        (com.sce.forge.runtime.SceChecked.sub(base, 4.toUInt())).toLong()
                                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                                        raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke> <param name='extra'> expr failed to evaluate")
                                    }

                                }
                                "static_hybrid_second" -> StaticHybridSecondStateMachine().also { child ->
                                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                                    val childSeed = StaticHybridSecondStateMachine.InvokeParams()
                                    childSeed.start = base
                                    try {
                                        childSeed.extra = com.sce.forge.runtime.SceChecked.sub(base, 4.toUInt())
                                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                                        // W3C SCXML 5.7.1: report the failure and leave the value out.
                                        raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke> <param name='extra'> expr failed to evaluate")
                                    }
                                    child.acceptParams(childSeed)

                                }
                                else -> {
                                    // The failure the Interpreter reports when a
                                    // document will not load: nothing to create.
                                    raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke srcexpr> evaluated to a document it did not declare")
                                    return@deferInvoke
                                }
                            }

                            startInvoke("second_run", childSM, false, StaticInvokeHybridEvent.Done.Invoke.SecondRun, "", generatedInvokeId)
                        } catch (_: Exception) {
                            // W3C SCXML 6.4: the child could not be started. Evaluation
                            // failure no longer reaches here — it has its own raise above,
                            // under the one wording every emitter uses for that fact.
                            raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<invoke> could not start a child")
                        }
                    }
                }
            }
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_invoke_hybrid.scxml:33 :: _machine
    override fun onExit(state: StaticInvokeHybridState) {
        when (state) {
            is StaticInvokeHybridState.First -> {
                // SCE-MAP: static_invoke_hybrid.scxml:46 :: first :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("first_run")
            }
            is StaticInvokeHybridState.Lossy -> {
                // SCE-MAP: static_invoke_hybrid.scxml:72 :: lossy :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("lossy_run")
            }
            is StaticInvokeHybridState.Missing -> {
                // SCE-MAP: static_invoke_hybrid.scxml:85 :: missing :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("missing_run")
            }
            is StaticInvokeHybridState.Over -> {
                // SCE-MAP: static_invoke_hybrid.scxml:101 :: over :: _state_body
            }
            is StaticInvokeHybridState.Run -> {
                // SCE-MAP: static_invoke_hybrid.scxml:42 :: run :: _state_body
            }
            is StaticInvokeHybridState.Second -> {
                // SCE-MAP: static_invoke_hybrid.scxml:59 :: second :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("second_run")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_invoke_hybrid.scxml:33 :: _machine
    override fun executeTransitionContent(source: StaticInvokeHybridState, transitionIndex: Int) {
        when (source) {
        is StaticInvokeHybridState.First -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_invoke_hybrid.scxml:55 :: first :: _transition_0

            if (try { completed = com.sce.forge.runtime.SceChecked.add(completed, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticInvokeHybridState.Lossy -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_invoke_hybrid.scxml:81 :: lossy :: _transition_0

            if (try { completed = com.sce.forge.runtime.SceChecked.add(completed, 100.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticInvokeHybridState.Missing -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_invoke_hybrid.scxml:93 :: missing :: _transition_0

            if (try { completed = com.sce.forge.runtime.SceChecked.add(completed, 1000.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_invoke_hybrid.scxml:96 :: missing :: _transition_1

            if (try { errors = com.sce.forge.runtime.SceChecked.add(errors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticInvokeHybridState.Run -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_invoke_hybrid.scxml:43 :: run :: _transition_0

            if (try { errors = com.sce.forge.runtime.SceChecked.add(errors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticInvokeHybridState.Second -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_invoke_hybrid.scxml:68 :: second :: _transition_0

            if (try { completed = com.sce.forge.runtime.SceChecked.add(completed, 10.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeHybridEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
