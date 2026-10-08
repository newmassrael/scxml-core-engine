// SCE-GENERATED — DO NOT EDIT
// source-hash: 0665514219a2bd1920179bb95b88947aea9f3cb1e16b583d8593e5f3d24c015b

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_child_host_hybrid.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_child_host_hybrid.scxml:19 :: _machine

package com.sce.integration.static_child_host_hybrid

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticChildHostHybridState : State {
    data object Idle : StaticChildHostHybridState
    data object Working : StaticChildHostHybridState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticChildHostHybridEvent : Event {
    data object Again : StaticChildHostHybridEvent
    data object Back : StaticChildHostHybridEvent
    sealed interface Done : StaticChildHostHybridEvent {
        sealed interface Invoke : Done {
            data object Work : Invoke
        }
    }
}
// --- State Machine (W3C SCXML) ---

// ── W3C SCXML G.7: `<sce:action>` host dispatch ───────────────────────
/**
 * W3C SCXML G.7: host operations dispatched by `<sce:action>`.
 * The host supplies the side effects while the statechart keeps each
 * operation symbolic. No runtime script engine is involved.
 */
interface StaticChildHostHybridActions {
    fun actionsForWorkStaticHostedFirst(): StaticHostedFirstActions
    fun actionsForWorkStaticHostedSecond(): StaticHostedSecondActions
}

/**
 * [StaticChildHostHybridActions] that performs nothing and records every call in order —
 * the host a test drives the machine with. Read [calls] after the machine
 * has run; each call is compared by value.
 */
class RecordingStaticChildHostHybridActions(
    /** Answers the host [StaticHostedFirstActions] is asked for; the test keeps what it answers. */
    private val actionsForWorkStaticHostedFirstSource: () -> StaticHostedFirstActions,
    /** Answers the host [StaticHostedSecondActions] is asked for; the test keeps what it answers. */
    private val actionsForWorkStaticHostedSecondSource: () -> StaticHostedSecondActions,
) : StaticChildHostHybridActions {
    /** One recorded host call. */
    sealed interface Call {
        data object ActionsForWorkStaticHostedFirst : Call
        data object ActionsForWorkStaticHostedSecond : Call
    }

    private val recorded = mutableListOf<Call>()

    /** Every call so far, oldest first. */
    val calls: List<Call>
        get() = recorded.toList()

    /** Forget the calls recorded so far. */
    fun clear() {
        recorded.clear()
    }

    override fun actionsForWorkStaticHostedFirst(): StaticHostedFirstActions {
        recorded += Call.ActionsForWorkStaticHostedFirst
        return actionsForWorkStaticHostedFirstSource()
    }
    override fun actionsForWorkStaticHostedSecond(): StaticHostedSecondActions {
        recorded += Call.ActionsForWorkStaticHostedSecond
        return actionsForWorkStaticHostedSecondSource()
    }
}

class StaticChildHostHybridStateMachine(
    /**
     * W3C SCXML G.7: the host implementation every `<sce:action>` in this
     * document calls directly (`actions.<op>(…)`) instead of the script
     * engine.
     *
     * A constructor parameter rather than a setter, because the initial
     * state's `<onentry>` can already perform an act — a host installed
     * afterwards would arrive one act too late. It leads the parameter list
     * so it stays in the same position whether or not this machine also
     * takes a script engine.
     */
    private val actions: StaticChildHostHybridActions,
) : StateMachineEngine<StaticChildHostHybridState, StaticChildHostHybridEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `pick` datamodel variable, the machine's own. */
    private var pick: String = "static_hosted_first.scxml"
    /** W3C SCXML 5.2: the `completed` datamodel variable, published (`sce:direction="out"`). */
    var completed: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var pick: String? = null
        var completed: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.pick?.let { pick = it }
        params.completed?.let { completed = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val completed: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticChildHostHybridState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        completed = completed,
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
    val savedShape: String = "2a6b932ecb9cccbb0f8ab9a2240ee9869d2a0cfc9784a29c0c34179f7233c3a7"

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
            "completed" to SavedValues.of(completed),
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
        val saved2 = SavedValues.uint32(saved.variable("completed"), "completed")
        pick = saved1
        completed = saved2
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // §scxml-6.4: a saved state names the `<invoke>`s whose child is running,
    // and a restore starts each again from its beginning. This document saves
    // only when every invoke is a static child session, a hybrid one among
    // declared candidates, or one a declared host invoker serves.
    override val staticInvokes: List<Pair<String, StaticChildHostHybridState>> = listOf(
        "work" to StaticChildHostHybridState.Working,
    )

    override fun restartInvoke(invokeId: String, state: StaticChildHostHybridState) = deferStaticInvoke(invokeId, state)

    override val initialState: StaticChildHostHybridState = StaticChildHostHybridState.Working

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
    override val documentInitialTargets: List<EntryTarget<StaticChildHostHybridState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticChildHostHybridState, HistoryId>> =
            listOf(StateTarget(StaticChildHostHybridState.Working))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticChildHostHybridState, HistoryId>(
            StaticChildHostHybridState.Idle,
            listOf(StateTarget(StaticChildHostHybridState.Working)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<StaticChildHostHybridState, HistoryId>(
            StaticChildHostHybridState.Working,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 1, as the microstep reads it.
        val transitionWorkingAt1 = EnabledTransition<StaticChildHostHybridState, HistoryId>(
            StaticChildHostHybridState.Working,
            listOf(StateTarget(StaticChildHostHybridState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticChildHostHybridState? = when (stateId) {
        "idle" -> StaticChildHostHybridState.Idle
        "working" -> StaticChildHostHybridState.Working
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticChildHostHybridState): String = when (state) {
        is StaticChildHostHybridState.Idle -> "idle"
        is StaticChildHostHybridState.Working -> "working"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticChildHostHybridState): Int = when (state) {
        is StaticChildHostHybridState.Idle -> 1
        is StaticChildHostHybridState.Working -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticChildHostHybridEvent? = when (name) {
        "again" -> StaticChildHostHybridEvent.Again
        "back" -> StaticChildHostHybridEvent.Back
        "done.invoke.work" -> StaticChildHostHybridEvent.Done.Invoke.Work
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticChildHostHybridEvent): String? = when (event) {
        is StaticChildHostHybridEvent.Again -> "again"
        is StaticChildHostHybridEvent.Back -> "back"
        is StaticChildHostHybridEvent.Done.Invoke.Work -> "done.invoke.work"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticChildHostHybridState,
        event: StaticChildHostHybridEvent?
    ): EnabledTransition<StaticChildHostHybridState, HistoryId>? = when (state) {
        is StaticChildHostHybridState.Idle -> when {
            event is StaticChildHostHybridEvent.Back -> transitionIdleAt0
            else -> null
        }
        is StaticChildHostHybridState.Working -> when {
            event is StaticChildHostHybridEvent.Done.Invoke.Work -> transitionWorkingAt0
            event is StaticChildHostHybridEvent.Again -> transitionWorkingAt1
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_child_host_hybrid.scxml:19 :: _machine
    override fun onEntry(state: StaticChildHostHybridState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticChildHostHybridState.Idle -> {
                // SCE-MAP: static_child_host_hybrid.scxml:35 :: idle :: _state_body
            }
            is StaticChildHostHybridState.Working -> {
                // SCE-MAP: static_child_host_hybrid.scxml:25 :: working :: _state_body
                // W3C SCXML 6.4: Defer the hybrid invoke until macrostep end. `deferStaticInvoke`
                // starts it — for entering the state and for a restore alike, so the two
                // cannot start it differently.
                deferStaticInvoke("work", state)
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
    private fun deferStaticInvoke(invokeId: String, state: StaticChildHostHybridState) {
        when (invokeId) {
            "work" -> {
                // W3C SCXML 6.4: Hybrid invoke — runtime expression evaluation + dynamic child
                // C++ parity: StateMachine::createFromSCXMLString() / FileLoadingHelper::loadScxmlFile()
                run {
                    val generatedInvokeId = "working.${System.identityHashCode(this)}.work"
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
                                "static_hosted_first" -> StaticHostedFirstStateMachine(actions.actionsForWorkStaticHostedFirst())
                                "static_hosted_second" -> StaticHostedSecondStateMachine(actions.actionsForWorkStaticHostedSecond())
                                else -> {
                                    // The failure the Interpreter reports when a
                                    // document will not load: nothing to create.
                                    return@deferInvoke
                                }
                            }

                            startInvoke("work", childSM, false, StaticChildHostHybridEvent.Done.Invoke.Work, "", generatedInvokeId)
                        } catch (_: Exception) {
                            // W3C SCXML 6.4: the child could not be started. Evaluation
                            // failure no longer reaches here — it has its own raise above,
                            // under the one wording every emitter uses for that fact.
                        }
                    }
                }
            }
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_child_host_hybrid.scxml:19 :: _machine
    override fun onExit(state: StaticChildHostHybridState) {
        when (state) {
            is StaticChildHostHybridState.Idle -> {
                // SCE-MAP: static_child_host_hybrid.scxml:35 :: idle :: _state_body
            }
            is StaticChildHostHybridState.Working -> {
                // SCE-MAP: static_child_host_hybrid.scxml:25 :: working :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("work")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_child_host_hybrid.scxml:19 :: _machine
    override fun executeTransitionContent(source: StaticChildHostHybridState, transitionIndex: Int) {
        when (source) {
        is StaticChildHostHybridState.Working -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_child_host_hybrid.scxml:28 :: working :: _transition_0

            if (try { completed = com.sce.forge.runtime.SceChecked.add(completed, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_child_host_hybrid.scxml:31 :: working :: _transition_1

            if (try { pick = com.sce.forge.runtime.SceChecked.bounded("static_hosted_second.scxml", 48); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
