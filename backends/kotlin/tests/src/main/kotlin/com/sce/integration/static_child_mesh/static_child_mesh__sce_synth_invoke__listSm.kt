// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_child_mesh__sce_synth_invoke__list.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_child_mesh__sce_synth_invoke__list.scxml:3 :: _machine

package com.sce.integration.static_child_mesh

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticChildMeshSceSynthInvokeListState : State {
    data object Open : StaticChildMeshSceSynthInvokeListState
    data object Refused : StaticChildMeshSceSynthInvokeListState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticChildMeshSceSynthInvokeListEvent : Event {
    sealed interface Error : StaticChildMeshSceSynthInvokeListEvent {
        data object Execution : Error
    }
    sealed interface List : StaticChildMeshSceSynthInvokeListEvent {
        data object Hello : List
        data object Ping : List
        data object Pong : List
    }
}
// --- State Machine (W3C SCXML) ---

class StaticChildMeshSceSynthInvokeListStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticChildMeshSceSynthInvokeListState, StaticChildMeshSceSynthInvokeListEvent>() {

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
        first = "122742c57881dae3ec3f8b89ce662c07f3fbb4e3b790bf678535d302495d35f5",
        revised = "d99f34c48133b948c9fb435a8a811deca997a01968f8e13825f23343e2f4614f",
        format2First = "122742c57881dae3ec3f8b89ce662c07f3fbb4e3b790bf678535d302495d35f5",
        format2Revised = "d99f34c48133b948c9fb435a8a811deca997a01968f8e13825f23343e2f4614f",
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

    override val initialState: StaticChildMeshSceSynthInvokeListState = StaticChildMeshSceSynthInvokeListState.Open

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
    override fun isFinalState(state: StaticChildMeshSceSynthInvokeListState): Boolean = when (state) {
        is StaticChildMeshSceSynthInvokeListState.Refused -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticChildMeshSceSynthInvokeListState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticChildMeshSceSynthInvokeListState, HistoryId>> =
            listOf(StateTarget(StaticChildMeshSceSynthInvokeListState.Open))

        // W3C SCXML 3.13: open's transition 0, as the microstep reads it.
        val transitionOpenAt0 = EnabledTransition<StaticChildMeshSceSynthInvokeListState, HistoryId>(
            StaticChildMeshSceSynthInvokeListState.Open,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: open's transition 1, as the microstep reads it.
        val transitionOpenAt1 = EnabledTransition<StaticChildMeshSceSynthInvokeListState, HistoryId>(
            StaticChildMeshSceSynthInvokeListState.Open,
            listOf(StateTarget(StaticChildMeshSceSynthInvokeListState.Refused)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticChildMeshSceSynthInvokeListState? = when (stateId) {
        "open" -> StaticChildMeshSceSynthInvokeListState.Open
        "refused" -> StaticChildMeshSceSynthInvokeListState.Refused
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticChildMeshSceSynthInvokeListState): String = when (state) {
        is StaticChildMeshSceSynthInvokeListState.Open -> "open"
        is StaticChildMeshSceSynthInvokeListState.Refused -> "refused"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticChildMeshSceSynthInvokeListState): Int = when (state) {
        is StaticChildMeshSceSynthInvokeListState.Open -> 0
        is StaticChildMeshSceSynthInvokeListState.Refused -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticChildMeshSceSynthInvokeListEvent? = when (name) {
        "error.execution" -> StaticChildMeshSceSynthInvokeListEvent.Error.Execution
        "list.hello" -> StaticChildMeshSceSynthInvokeListEvent.List.Hello
        "list.ping" -> StaticChildMeshSceSynthInvokeListEvent.List.Ping
        "list.pong" -> StaticChildMeshSceSynthInvokeListEvent.List.Pong
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticChildMeshSceSynthInvokeListEvent): String? = when (event) {
        is StaticChildMeshSceSynthInvokeListEvent.Error.Execution -> "error.execution"
        is StaticChildMeshSceSynthInvokeListEvent.List.Hello -> "list.hello"
        is StaticChildMeshSceSynthInvokeListEvent.List.Ping -> "list.ping"
        is StaticChildMeshSceSynthInvokeListEvent.List.Pong -> "list.pong"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticChildMeshSceSynthInvokeListState,
        event: StaticChildMeshSceSynthInvokeListEvent?
    ): EnabledTransition<StaticChildMeshSceSynthInvokeListState, HistoryId>? = when (state) {
        is StaticChildMeshSceSynthInvokeListState.Open -> when {
            event is StaticChildMeshSceSynthInvokeListEvent.List.Ping -> transitionOpenAt0
            event is StaticChildMeshSceSynthInvokeListEvent.Error.Execution -> transitionOpenAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_child_mesh__sce_synth_invoke__list.scxml:3 :: _machine
    override fun onEntry(state: StaticChildMeshSceSynthInvokeListState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticChildMeshSceSynthInvokeListState.Open -> {
                // SCE-MAP: static_child_mesh__sce_synth_invoke__list.scxml:5 :: open :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "list.hello",
                target = "#server",
                content = "",
                params = sendWireParams,
                sendId = "__send_1",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(StaticChildMeshSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_1")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is StaticChildMeshSceSynthInvokeListState.Refused -> {
                // SCE-MAP: static_child_mesh__sce_synth_invoke__list.scxml:15 :: refused :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_child_mesh__sce_synth_invoke__list.scxml:3 :: _machine
    override fun onExit(state: StaticChildMeshSceSynthInvokeListState) {
        when (state) {
            is StaticChildMeshSceSynthInvokeListState.Open -> {
                // SCE-MAP: static_child_mesh__sce_synth_invoke__list.scxml:5 :: open :: _state_body
            }
            is StaticChildMeshSceSynthInvokeListState.Refused -> {
                // SCE-MAP: static_child_mesh__sce_synth_invoke__list.scxml:15 :: refused :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_child_mesh__sce_synth_invoke__list.scxml:3 :: _machine
    override fun executeTransitionContent(source: StaticChildMeshSceSynthInvokeListState, transitionIndex: Int) {
        when (source) {
        is StaticChildMeshSceSynthInvokeListState.Open -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_child_mesh__sce_synth_invoke__list.scxml:9 :: open :: _transition_0


            if (run send@{
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "list.pong",
                target = "#server",
                content = "",
                params = sendWireParams,
                sendId = "__send_0",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(StaticChildMeshSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_0")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
