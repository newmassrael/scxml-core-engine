// SCE-GENERATED — DO NOT EDIT
// source-hash: 048fe3b13b4ba344e3f60c0d3a93bf8f8568ab7bd254b4d4763d7e631895e2f6

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_mesh_relay__sce_synth_invoke__inner.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_mesh_relay__sce_synth_invoke__inner.scxml:3 :: _machine

package com.sce.integration.static_child_mesh

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticMeshRelaySceSynthInvokeInnerState : State {
    data object Waiting : StaticMeshRelaySceSynthInvokeInnerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticMeshRelaySceSynthInvokeInnerEvent : Event {
    sealed interface Error : StaticMeshRelaySceSynthInvokeInnerEvent {
        data object Execution : Error
    }
    sealed interface Inner : StaticMeshRelaySceSynthInvokeInnerEvent {
        data object Ping : Inner
        data object Pong : Inner
    }
}
// --- State Machine (W3C SCXML) ---

class StaticMeshRelaySceSynthInvokeInnerStateMachine(
) : StateMachineEngine<StaticMeshRelaySceSynthInvokeInnerState, StaticMeshRelaySceSynthInvokeInnerEvent>() {

    // ── SCE Accepted Subset §2.15: saving this machine, restoring it ─────────

    /**
     * What a saved state of this document is bound to: a state saved from a
     * document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not, and one
     * saved at an earlier revision of a record schema is read with each field a
     * later revision added at its default.
     */
    val savedShape: SavedShape = SavedShape(
        first = "fcde5ccc7bbe96d580c215a7dd7a771f26a24d604aa198c1a6ac1f7b88663205",
        revised = "4ea0e0e30338d81d9fbe2eabbbf391c38461f625b495ef1c6480c3c246ff5130",
        format2First = "fcde5ccc7bbe96d580c215a7dd7a771f26a24d604aa198c1a6ac1f7b88663205",
        format2Revised = "4ea0e0e30338d81d9fbe2eabbbf391c38461f625b495ef1c6480c3c246ff5130",
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

    override val initialState: StaticMeshRelaySceSynthInvokeInnerState = StaticMeshRelaySceSynthInvokeInnerState.Waiting

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
    override val documentInitialTargets: List<EntryTarget<StaticMeshRelaySceSynthInvokeInnerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticMeshRelaySceSynthInvokeInnerState, HistoryId>> =
            listOf(StateTarget(StaticMeshRelaySceSynthInvokeInnerState.Waiting))

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StaticMeshRelaySceSynthInvokeInnerState, HistoryId>(
            StaticMeshRelaySceSynthInvokeInnerState.Waiting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticMeshRelaySceSynthInvokeInnerState? = when (stateId) {
        "waiting" -> StaticMeshRelaySceSynthInvokeInnerState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticMeshRelaySceSynthInvokeInnerState): String = when (state) {
        is StaticMeshRelaySceSynthInvokeInnerState.Waiting -> "waiting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticMeshRelaySceSynthInvokeInnerState): Int = when (state) {
        is StaticMeshRelaySceSynthInvokeInnerState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticMeshRelaySceSynthInvokeInnerEvent? = when (name) {
        "error.execution" -> StaticMeshRelaySceSynthInvokeInnerEvent.Error.Execution
        "inner.ping" -> StaticMeshRelaySceSynthInvokeInnerEvent.Inner.Ping
        "inner.pong" -> StaticMeshRelaySceSynthInvokeInnerEvent.Inner.Pong
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticMeshRelaySceSynthInvokeInnerEvent): String? = when (event) {
        is StaticMeshRelaySceSynthInvokeInnerEvent.Error.Execution -> "error.execution"
        is StaticMeshRelaySceSynthInvokeInnerEvent.Inner.Ping -> "inner.ping"
        is StaticMeshRelaySceSynthInvokeInnerEvent.Inner.Pong -> "inner.pong"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticMeshRelaySceSynthInvokeInnerState,
        event: StaticMeshRelaySceSynthInvokeInnerEvent?
    ): EnabledTransition<StaticMeshRelaySceSynthInvokeInnerState, HistoryId>? = when (state) {
        is StaticMeshRelaySceSynthInvokeInnerState.Waiting -> when {
            event is StaticMeshRelaySceSynthInvokeInnerEvent.Inner.Ping -> transitionWaitingAt0
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_mesh_relay__sce_synth_invoke__inner.scxml:3 :: _machine
    override fun onEntry(state: StaticMeshRelaySceSynthInvokeInnerState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticMeshRelaySceSynthInvokeInnerState.Waiting -> {
                // SCE-MAP: static_mesh_relay__sce_synth_invoke__inner.scxml:5 :: waiting :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_mesh_relay__sce_synth_invoke__inner.scxml:3 :: _machine
    override fun onExit(state: StaticMeshRelaySceSynthInvokeInnerState) {
        when (state) {
            is StaticMeshRelaySceSynthInvokeInnerState.Waiting -> {
                // SCE-MAP: static_mesh_relay__sce_synth_invoke__inner.scxml:5 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_mesh_relay__sce_synth_invoke__inner.scxml:3 :: _machine
    override fun executeTransitionContent(source: StaticMeshRelaySceSynthInvokeInnerState, transitionIndex: Int) {
        when (source) {
        is StaticMeshRelaySceSynthInvokeInnerState.Waiting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_mesh_relay__sce_synth_invoke__inner.scxml:6 :: waiting :: _transition_0


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
                eventName = "inner.pong",
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
                raisePlatformError(StaticMeshRelaySceSynthInvokeInnerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_0")
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
        }
    }
}
