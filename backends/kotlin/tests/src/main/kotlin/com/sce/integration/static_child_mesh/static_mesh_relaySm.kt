// SCE-GENERATED — DO NOT EDIT
// source-hash: b67ac92c47dabab09bc66be64b0a4b4a6acfabe17f95412102ed640a717fc04f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_mesh_relay.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_mesh_relay.scxml:12 :: _machine

package com.sce.integration.static_child_mesh

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticMeshRelayState : State {
    data object Relaying : StaticMeshRelayState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticMeshRelayEvent : Event {
    sealed interface Done : StaticMeshRelayEvent {
        data object Invoke : Done
    }
    sealed interface Error : StaticMeshRelayEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class StaticMeshRelayStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticMeshRelayState, StaticMeshRelayEvent>() {

    init {
        bindRootSessionId(sessionId)
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
        first = "29091e95d2e4b2df2c5e529bc91941e15de2224bf0bfd73c5823bbd35cb19aac",
        revised = "3bba4cf36f6c537811121f0618bcdf648a685c1fe51ab027cf283e142dd5ab9f",
        format2First = "29091e95d2e4b2df2c5e529bc91941e15de2224bf0bfd73c5823bbd35cb19aac",
        format2Revised = "3bba4cf36f6c537811121f0618bcdf648a685c1fe51ab027cf283e142dd5ab9f",
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
        enterSaved(savedGrown, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // docs/adr/0045: this machine as a child session of another's saved state —
    // saved with it, judged with it before anything is built, and restored in
    // place of initialized.
    override fun saveAsSession(wallNowMs: Long): SavedState = save(wallNowMs)

    override fun judgeAsSession(saved: SavedState) {
        val savedGrown = beginRestore(saved, savedShape)
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4: a saved state names the `<invoke>`s whose child is running,
    // and a restore starts each again from its beginning. This document saves
    // only when every invoke is a static child session, a hybrid one among
    // declared candidates, or one a declared host invoker serves.
    override val staticInvokes: List<Pair<String, StaticMeshRelayState>> = listOf(
        "inner" to StaticMeshRelayState.Relaying,
    )

    override fun restartInvoke(invokeId: String, state: StaticMeshRelayState) = deferStaticInvoke(invokeId, state)

    override val initialState: StaticMeshRelayState = StaticMeshRelayState.Relaying

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
    override val documentInitialTargets: List<EntryTarget<StaticMeshRelayState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticMeshRelayState, HistoryId>> =
            listOf(StateTarget(StaticMeshRelayState.Relaying))
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticMeshRelayState? = when (stateId) {
        "relaying" -> StaticMeshRelayState.Relaying
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticMeshRelayState): String = when (state) {
        is StaticMeshRelayState.Relaying -> "relaying"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticMeshRelayState): Int = when (state) {
        is StaticMeshRelayState.Relaying -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticMeshRelayEvent? = when (name) {
        "done.invoke" -> StaticMeshRelayEvent.Done.Invoke
        "error.execution" -> StaticMeshRelayEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticMeshRelayEvent): String? = when (event) {
        is StaticMeshRelayEvent.Done.Invoke -> "done.invoke"
        is StaticMeshRelayEvent.Error.Execution -> "error.execution"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticMeshRelayState,
        event: StaticMeshRelayEvent?
    ): EnabledTransition<StaticMeshRelayState, HistoryId>? = when (state) {
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_mesh_relay.scxml:12 :: _machine
    override fun onEntry(state: StaticMeshRelayState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticMeshRelayState.Relaying -> {
                // SCE-MAP: static_mesh_relay.scxml:14 :: relaying :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("inner", state)
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
    private fun deferStaticInvoke(invokeId: String, state: StaticMeshRelayState) {
        when (invokeId) {
            "inner" -> run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "relaying.${nextInvokeStart("inner")}.inner"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(StaticMeshRelaySceSynthInvokeInnerStateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("inner", childSM, false, StaticMeshRelayEvent.Done.Invoke, "", generatedInvokeId, "", "static_mesh_relay__sce_synth_invoke__inner", "inner")
                }
            }
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }

    // docs/adr/0045, section 3, steps 1 and 3: the machine of a running child a
    // saved state lists, built as the `<invoke>` that started it builds it and
    // with what a start does with the child once built. Nothing is evaluated here:
    // no `<param>`, no `srcexpr`, no `idlocation` — the child resumes where it
    // stood, and a hybrid child is the candidate its saved `document` names,
    // whatever the `srcexpr` reads now.
    override fun buildSavedChild(child: SavedChild): ResumedChild<StaticMeshRelayEvent> = when (child.id) {
        "inner" -> if (child.document == "static_mesh_relay__sce_synth_invoke__inner") {
            ResumedChild(
                StaticMeshRelaySceSynthInvokeInnerStateMachine(),
                false, StaticMeshRelayEvent.Done.Invoke, "", "", "inner",
            )
        } else {
            throw StateRefusal("invokes names '${child.id}' running '${child.document}', a document that `<invoke>` does not start")
        }
        else -> throw StateRefusal("the document has no child session '${child.id}'")
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_mesh_relay.scxml:12 :: _machine
    override fun onExit(state: StaticMeshRelayState) {
        when (state) {
            is StaticMeshRelayState.Relaying -> {
                // SCE-MAP: static_mesh_relay.scxml:14 :: relaying :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inner")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_mesh_relay.scxml:12 :: _machine
    override fun executeTransitionContent(source: StaticMeshRelayState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
