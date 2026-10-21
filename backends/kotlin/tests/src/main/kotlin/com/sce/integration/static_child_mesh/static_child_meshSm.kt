// SCE-GENERATED — DO NOT EDIT
// source-hash: 048fe3b13b4ba344e3f60c0d3a93bf8f8568ab7bd254b4d4763d7e631895e2f6

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_child_mesh.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_child_mesh.scxml:29 :: _machine

package com.sce.integration.static_child_mesh

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticChildMeshState : State {
    data object Alone : StaticChildMeshState
    data object Idle : StaticChildMeshState
    data object Joined : StaticChildMeshState
    data object Working : StaticChildMeshState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticChildMeshEvent : Event {
    data object Again : StaticChildMeshEvent
    data object Back : StaticChildMeshEvent
    sealed interface Done : StaticChildMeshEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object List : Invoke
        }
    }
    sealed interface Error : StaticChildMeshEvent {
        data object Execution : Error
    }
    data object Open : StaticChildMeshEvent
}
// --- State Machine (W3C SCXML) ---

class StaticChildMeshStateMachine(
) : StateMachineEngine<StaticChildMeshState, StaticChildMeshEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `pick` datamodel variable, the machine's own. */
    private var vPick: String = "static_mesh_relay.scxml"
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    private var vRefusals: UInt = 0.toUInt()

    /** What the published `refusals` holds. */
    val refusals: UInt get() = vRefusals

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var pick: String? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.pick?.let { vPick = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
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
        val configuration: Set<StaticChildMeshState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        refusals = vRefusals,
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
        first = "da543ed16b83c929a0920b9a9aac50cd56e51ff437dd739e049e2f87b699a9fe",
        revised = "b428739571d63d80edb7874f3bddb4b4c6c3d1f4c245f5a2cc0f5c72da4fd05e",
        format2First = "da543ed16b83c929a0920b9a9aac50cd56e51ff437dd739e049e2f87b699a9fe",
        format2Revised = "b428739571d63d80edb7874f3bddb4b4c6c3d1f4c245f5a2cc0f5c72da4fd05e",
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
            "pick" to SavedValues.of(vPick),
            "refusals" to SavedValues.of(vRefusals),
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
        val saved1 = SavedValues.string(savedGrown.variable("pick"), "pick", 48)
        val saved2 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        vPick = saved1
        vRefusals = saved2
        enterSaved(savedGrown, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // §scxml-6.4: a saved state names the `<invoke>`s whose child is running,
    // and a restore starts each again from its beginning. This document saves
    // only when every invoke is a static child session, a hybrid one among
    // declared candidates, or one a declared host invoker serves.
    override val staticInvokes: List<Pair<String, StaticChildMeshState>> = listOf(
        "list" to StaticChildMeshState.Working,
        "peer" to StaticChildMeshState.Joined,
    )

    override fun restartInvoke(invokeId: String, state: StaticChildMeshState) = deferStaticInvoke(invokeId, state)

    override val initialState: StaticChildMeshState = StaticChildMeshState.Alone

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
    override fun parentOf(state: StaticChildMeshState): StaticChildMeshState? = when (state) {
        is StaticChildMeshState.Alone -> StaticChildMeshState.Working
        is StaticChildMeshState.Joined -> StaticChildMeshState.Working
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: StaticChildMeshState): Boolean = when (state) {
        is StaticChildMeshState.Working -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: StaticChildMeshState): List<StaticChildMeshState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: StaticChildMeshState): List<EntryTarget<StaticChildMeshState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticChildMeshState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<StaticChildMeshState, List<StaticChildMeshState>> = mapOf(
            StaticChildMeshState.Working to listOf(StaticChildMeshState.Alone, StaticChildMeshState.Joined),
        )

        val initialTargets: Map<StaticChildMeshState, List<EntryTarget<StaticChildMeshState, HistoryId>>> = mapOf(
            StaticChildMeshState.Working to listOf(StateTarget(StaticChildMeshState.Alone)),
        )

        val documentInitialTargetList: List<EntryTarget<StaticChildMeshState, HistoryId>> =
            listOf(StateTarget(StaticChildMeshState.Working))

        // W3C SCXML 3.13: alone's transition 0, as the microstep reads it.
        val transitionAloneAt0 = EnabledTransition<StaticChildMeshState, HistoryId>(
            StaticChildMeshState.Alone,
            listOf(StateTarget(StaticChildMeshState.Joined)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticChildMeshState, HistoryId>(
            StaticChildMeshState.Idle,
            listOf(StateTarget(StaticChildMeshState.Working)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<StaticChildMeshState, HistoryId>(
            StaticChildMeshState.Working,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 1, as the microstep reads it.
        val transitionWorkingAt1 = EnabledTransition<StaticChildMeshState, HistoryId>(
            StaticChildMeshState.Working,
            listOf(StateTarget(StaticChildMeshState.Idle)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticChildMeshState? = when (stateId) {
        "alone" -> StaticChildMeshState.Alone
        "idle" -> StaticChildMeshState.Idle
        "joined" -> StaticChildMeshState.Joined
        "working" -> StaticChildMeshState.Working
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticChildMeshState): String = when (state) {
        is StaticChildMeshState.Alone -> "alone"
        is StaticChildMeshState.Idle -> "idle"
        is StaticChildMeshState.Joined -> "joined"
        is StaticChildMeshState.Working -> "working"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticChildMeshState): Int = when (state) {
        is StaticChildMeshState.Alone -> 1
        is StaticChildMeshState.Idle -> 3
        is StaticChildMeshState.Joined -> 2
        is StaticChildMeshState.Working -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticChildMeshEvent? = when (name) {
        "again" -> StaticChildMeshEvent.Again
        "back" -> StaticChildMeshEvent.Back
        "done.invoke" -> StaticChildMeshEvent.Done.Invoke.Self
        "done.invoke.list" -> StaticChildMeshEvent.Done.Invoke.List
        "error.execution" -> StaticChildMeshEvent.Error.Execution
        "open" -> StaticChildMeshEvent.Open
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticChildMeshEvent): String? = when (event) {
        is StaticChildMeshEvent.Again -> "again"
        is StaticChildMeshEvent.Back -> "back"
        is StaticChildMeshEvent.Done.Invoke.Self -> "done.invoke"
        is StaticChildMeshEvent.Done.Invoke.List -> "done.invoke.list"
        is StaticChildMeshEvent.Error.Execution -> "error.execution"
        is StaticChildMeshEvent.Open -> "open"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticChildMeshState,
        event: StaticChildMeshEvent?
    ): EnabledTransition<StaticChildMeshState, HistoryId>? = when (state) {
        is StaticChildMeshState.Alone -> when {
            event is StaticChildMeshEvent.Open -> transitionAloneAt0
            else -> null
        }
        is StaticChildMeshState.Idle -> when {
            event is StaticChildMeshEvent.Back -> transitionIdleAt0
            else -> null
        }
        is StaticChildMeshState.Working -> when {
            event is StaticChildMeshEvent.Done.Invoke.List -> transitionWorkingAt0
            event is StaticChildMeshEvent.Again -> transitionWorkingAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_child_mesh.scxml:29 :: _machine
    override fun onEntry(state: StaticChildMeshState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticChildMeshState.Alone -> {
                // SCE-MAP: static_child_mesh.scxml:58 :: alone :: _state_body
            }
            is StaticChildMeshState.Idle -> {
                // SCE-MAP: static_child_mesh.scxml:65 :: idle :: _state_body
            }
            is StaticChildMeshState.Joined -> {
                // SCE-MAP: static_child_mesh.scxml:61 :: joined :: _state_body
                // W3C SCXML 6.4: Defer the hybrid invoke until macrostep end. `deferStaticInvoke`
                // starts it — for entering the state and for a restore alike, so the two
                // cannot start it differently.
                deferStaticInvoke("peer", state)
            }
            is StaticChildMeshState.Working -> {
                // SCE-MAP: static_child_mesh.scxml:35 :: working :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("list", state)
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
    private fun deferStaticInvoke(invokeId: String, state: StaticChildMeshState) {
        when (invokeId) {
            "peer" -> {
                // W3C SCXML 6.4: Hybrid invoke — runtime expression evaluation + dynamic child
                // C++ parity: StateMachine::createFromSCXMLString() / FileLoadingHelper::loadScxmlFile()
                run {
                    // docs/adr/0045: the platform part of the id is the number of
                    // this start, so the id comes from the tree alone.
                    val generatedInvokeId = "joined.${nextInvokeStart("peer")}.peer"
                    deferInvoke(state, generatedInvokeId) {
                        try {
                            // §scxml-6.4: the `srcexpr` names the document to start.
                            val filePath: String = vPick
                            // §scxml-6.4 + SCE_ACCEPTED_SUBSET.md §2.13: the value
                            // chooses among the declared candidates, matched on the
                            // document stem so `file:x.scxml` and `./x.scxml` name
                            // one child. `startInvoke` takes a star-projected
                            // machine, so only the construction differs here.
                            // docs/adr/0045: each candidate takes its id where it
                            // is built, before it is handed an argument.
                            val __sceSelected = DocumentStem.of(filePath)
                            val childSM = when (__sceSelected) {
                                "static_mesh_relay" -> adoptChild(StaticMeshRelayStateMachine(), generatedInvokeId)
                                else -> {
                                    // The failure the Interpreter reports when a
                                    // document will not load: nothing to create.
                                    raisePlatformError(StaticChildMeshEvent.Error.Execution, "<invoke srcexpr> evaluated to a document it did not declare")
                                    return@deferInvoke
                                }
                            }

                            startInvoke("peer", childSM, false, StaticChildMeshEvent.Done.Invoke.Self, "", generatedInvokeId, __sceSelected)
                        } catch (_: Exception) {
                            // W3C SCXML 6.4: the child could not be started. Evaluation
                            // failure no longer reaches here — it has its own raise above,
                            // under the one wording every emitter uses for that fact.
                            raisePlatformError(StaticChildMeshEvent.Error.Execution, "<invoke> could not start a child")
                        }
                    }
                }
            }
            "list" -> run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "working.${nextInvokeStart("list")}.list"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(StaticChildMeshSceSynthInvokeListStateMachine(), generatedInvokeId)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("list", childSM, false, StaticChildMeshEvent.Done.Invoke.List, "", generatedInvokeId, "")
                }
            }
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_child_mesh.scxml:29 :: _machine
    override fun onExit(state: StaticChildMeshState) {
        when (state) {
            is StaticChildMeshState.Alone -> {
                // SCE-MAP: static_child_mesh.scxml:58 :: alone :: _state_body
            }
            is StaticChildMeshState.Idle -> {
                // SCE-MAP: static_child_mesh.scxml:65 :: idle :: _state_body
            }
            is StaticChildMeshState.Joined -> {
                // SCE-MAP: static_child_mesh.scxml:61 :: joined :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("peer")
            }
            is StaticChildMeshState.Working -> {
                // SCE-MAP: static_child_mesh.scxml:35 :: working :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("list")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_child_mesh.scxml:29 :: _machine
    override fun executeTransitionContent(source: StaticChildMeshState, transitionIndex: Int) {
        when (source) {
        is StaticChildMeshState.Working -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_child_mesh.scxml:54 :: working :: _transition_0

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticChildMeshEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
