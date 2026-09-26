// SCE-GENERATED — DO NOT EDIT
// source-hash: fcc36ca5ace7ff619d1d0a3cef283e3a12d9eaf17570ba300dc27c1de6383a2f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_host_invoker.scxml:106 :: _machine

package com.sce.integration.statechart_host_invoker

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartHostInvokerState : State {
    data object Done : StatechartHostInvokerState
    data object Evaluating : StatechartHostInvokerState
    data object Invoking : StatechartHostInvokerState
    data object Locating : StatechartHostInvokerState
    data object Timed : StatechartHostInvokerState
    data object Typed : StatechartHostInvokerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartHostInvokerEvent : Event {
    data object Again : StatechartHostInvokerEvent
    sealed interface Done : StatechartHostInvokerEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Perm : Invoke
            data object Probe : Invoke
            data object Probe2 : Invoke
            data object Slow : Invoke
        }
    }
    sealed interface Error : StatechartHostInvokerEvent {
        data object Execution : Error
        sealed interface Invoke : Error {
            data object Self : Invoke
            data object Slow : Invoke
        }
    }
    data object Evaluate : StatechartHostInvokerEvent
    data object Forget : StatechartHostInvokerEvent
    data object Leak : StatechartHostInvokerEvent
    data object Leave : StatechartHostInvokerEvent
    data object Locate : StatechartHostInvokerEvent
    data object Ping : StatechartHostInvokerEvent
    data object Retype : StatechartHostInvokerEvent
    data object Time : StatechartHostInvokerEvent
    data object Type : StatechartHostInvokerEvent
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StatechartHostInvokerPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StatechartHostInvokerDoneInvokePermPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.perm`. Consumers inject it via the `raiseDoneInvokePerm` seam
// on the machine — they never name this class directly.
data class StatechartHostInvokerDoneInvokePermPayload(val granted: Boolean)


// --- State Machine (W3C SCXML) ---

class StatechartHostInvokerStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<StatechartHostInvokerState, StatechartHostInvokerEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `started` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `started` was assigned a value of another type, or the engine refused.
     */
    fun started(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "started")

    /**
     * §scxml-5.3: what the `started2` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `started2` was assigned a value of another type, or the engine refused.
     */
    fun started2(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "started2")

    /**
     * §scxml-5.3: what the `refused` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `refused` was assigned a value of another type, or the engine refused.
     */
    fun refused(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "refused")

    /**
     * §scxml-5.3: what the `ended` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `ended` was assigned a value of another type, or the engine refused.
     */
    fun ended(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "ended")

    /**
     * §scxml-5.3: what the `entered` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `entered` was assigned a value of another type, or the engine refused.
     */
    fun entered(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "entered")

    /**
     * §scxml-5.3: what the `destination` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `destination` was assigned a value of another type, or the engine refused.
     */
    fun destination(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "destination")

    /**
     * §scxml-5.3: what the `n` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `n` was assigned a value of another type, or the engine refused.
     */
    fun n(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "n")

    /**
     * §scxml-5.3: what the `dropped` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `dropped` was assigned a value of another type, or the engine refused.
     */
    fun dropped(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "dropped")

    /**
     * §scxml-5.3: what the `doneId` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `doneId` was assigned a value of another type, or the engine refused.
     */
    fun doneId(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "doneId")

    /**
     * §scxml-5.3: what the `matched` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `matched` was assigned a value of another type, or the engine refused.
     */
    fun matched(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "matched")

    /**
     * §scxml-5.3: what the `slot` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `slot` was assigned a value of another type, or the engine refused.
     *
     * The value as JSON text, serialised by the engine's own `JSON.stringify`
     * (§scxml-B-2) so the key order is the document's.
     */
    fun slot(): String? =
        com.sce.runtime.DatamodelRead.readJson(scriptEngine, scriptSessionId, "slot")

    /**
     * §scxml-5.3: what the `slotted` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `slotted` was assigned a value of another type, or the engine refused.
     */
    fun slotted(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "slotted")

    /**
     * §scxml-5.3: what the `pinged` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `pinged` was assigned a value of another type, or the engine refused.
     */
    fun pinged(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "pinged")

    /**
     * §scxml-5.3: what the `leaked` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `leaked` was assigned a value of another type, or the engine refused.
     */
    fun leaked(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "leaked")

    /**
     * §scxml-5.3: what the `lost` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `lost` was assigned a value of another type, or the engine refused.
     */
    fun lost(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "lost")

    /**
     * §scxml-5.3: what the `expired` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `expired` was assigned a value of another type, or the engine refused.
     */
    fun expired(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "expired")

    /**
     * §scxml-5.3: what the `finished` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `finished` was assigned a value of another type, or the engine refused.
     */
    fun finished(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "finished")

    /**
     * §scxml-5.3: what the `misdated` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `misdated` was assigned a value of another type, or the engine refused.
     */
    fun misdated(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "misdated")

    /**
     * §scxml-5.3: what the `granted` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `granted` was assigned a value of another type, or the engine refused.
     */
    fun granted(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "granted")

    /**
     * §scxml-5.3: what the `denied` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `denied` was assigned a value of another type, or the engine refused.
     */
    fun denied(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "denied")

    /**
     * §scxml-5.3: what the `unreadable` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `unreadable` was assigned a value of another type, or the engine refused.
     */
    fun unreadable(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "unreadable")

    /**
     * §scxml-5.3: what the `scope` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `scope` was assigned a value of another type, or the engine refused.
     */
    fun scope(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "scope")

    /**
     * §scxml-5.3: what the `level` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `level` was assigned a value of another type, or the engine refused.
     */
    fun level(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "level")

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDoneInvokePermPayload: StatechartHostInvokerDoneInvokePermPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StatechartHostInvokerEvent, metadata: EventMetadata) {
        pendingDoneInvokePermPayload = null
        when (val tp = metadata.typedPayload) {
            is StatechartHostInvokerDoneInvokePermPayload -> pendingDoneInvokePermPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StatechartHostInvokerEvent.Done.Invoke.Perm) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokePermPayload = StatechartHostInvokerDoneInvokePermPayload(fields.boolean("granted"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.


    override val initialState: StatechartHostInvokerState = StatechartHostInvokerState.Invoking

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StatechartHostInvokerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StatechartHostInvokerState, HistoryId>> =
            listOf(StateTarget(StatechartHostInvokerState.Invoking))

        // W3C SCXML 3.13: done's transition 0, as the microstep reads it.
        val transitionDoneAt0 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Done,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: done's transition 1, as the microstep reads it.
        val transitionDoneAt1 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Done,
            listOf(StateTarget(StatechartHostInvokerState.Invoking)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: evaluating's transition 0, as the microstep reads it.
        val transitionEvaluatingAt0 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Evaluating,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: invoking's transition 0, as the microstep reads it.
        val transitionInvokingAt0 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Invoking,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: invoking's transition 1, as the microstep reads it.
        val transitionInvokingAt1 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Invoking,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: invoking's transition 2, as the microstep reads it.
        val transitionInvokingAt2 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Invoking,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: invoking's transition 3, as the microstep reads it.
        val transitionInvokingAt3 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Invoking,
            listOf(StateTarget(StatechartHostInvokerState.Done)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: invoking's transition 4, as the microstep reads it.
        val transitionInvokingAt4 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Invoking,
            listOf(StateTarget(StatechartHostInvokerState.Evaluating)),
            4,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: invoking's transition 5, as the microstep reads it.
        val transitionInvokingAt5 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Invoking,
            listOf(StateTarget(StatechartHostInvokerState.Locating)),
            5,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: invoking's transition 6, as the microstep reads it.
        val transitionInvokingAt6 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Invoking,
            listOf(StateTarget(StatechartHostInvokerState.Timed)),
            6,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: invoking's transition 7, as the microstep reads it.
        val transitionInvokingAt7 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Invoking,
            listOf(StateTarget(StatechartHostInvokerState.Typed)),
            7,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: locating's transition 0, as the microstep reads it.
        val transitionLocatingAt0 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Locating,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: locating's transition 1, as the microstep reads it.
        val transitionLocatingAt1 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Locating,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: locating's transition 2, as the microstep reads it.
        val transitionLocatingAt2 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Locating,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: locating's transition 3, as the microstep reads it.
        val transitionLocatingAt3 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Locating,
            emptyList(),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: timed's transition 0, as the microstep reads it.
        val transitionTimedAt0 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Timed,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: timed's transition 1, as the microstep reads it.
        val transitionTimedAt1 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Timed,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: timed's transition 2, as the microstep reads it.
        val transitionTimedAt2 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Timed,
            listOf(StateTarget(StatechartHostInvokerState.Done)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: timed's transition 3, as the microstep reads it.
        val transitionTimedAt3 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Timed,
            emptyList(),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: timed's transition 4, as the microstep reads it.
        val transitionTimedAt4 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Timed,
            emptyList(),
            4,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: typed's transition 0, as the microstep reads it.
        val transitionTypedAt0 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Typed,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: typed's transition 1, as the microstep reads it.
        val transitionTypedAt1 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Typed,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: typed's transition 2, as the microstep reads it.
        val transitionTypedAt2 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Typed,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: typed's transition 3, as the microstep reads it.
        val transitionTypedAt3 = EnabledTransition<StatechartHostInvokerState, HistoryId>(
            StatechartHostInvokerState.Typed,
            listOf(StateTarget(StatechartHostInvokerState.Typed)),
            3,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StatechartHostInvokerState? = when (stateId) {
        "done" -> StatechartHostInvokerState.Done
        "evaluating" -> StatechartHostInvokerState.Evaluating
        "invoking" -> StatechartHostInvokerState.Invoking
        "locating" -> StatechartHostInvokerState.Locating
        "timed" -> StatechartHostInvokerState.Timed
        "typed" -> StatechartHostInvokerState.Typed
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartHostInvokerState): String = when (state) {
        is StatechartHostInvokerState.Done -> "done"
        is StatechartHostInvokerState.Evaluating -> "evaluating"
        is StatechartHostInvokerState.Invoking -> "invoking"
        is StatechartHostInvokerState.Locating -> "locating"
        is StatechartHostInvokerState.Timed -> "timed"
        is StatechartHostInvokerState.Typed -> "typed"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StatechartHostInvokerState): Int = when (state) {
        is StatechartHostInvokerState.Done -> 2
        is StatechartHostInvokerState.Evaluating -> 1
        is StatechartHostInvokerState.Invoking -> 0
        is StatechartHostInvokerState.Locating -> 3
        is StatechartHostInvokerState.Timed -> 4
        is StatechartHostInvokerState.Typed -> 5
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartHostInvokerEvent? = when (name) {
        "again" -> StatechartHostInvokerEvent.Again
        "done.invoke" -> StatechartHostInvokerEvent.Done.Invoke.Self
        "done.invoke.perm" -> StatechartHostInvokerEvent.Done.Invoke.Perm
        "done.invoke.probe" -> StatechartHostInvokerEvent.Done.Invoke.Probe
        "done.invoke.probe2" -> StatechartHostInvokerEvent.Done.Invoke.Probe2
        "done.invoke.slow" -> StatechartHostInvokerEvent.Done.Invoke.Slow
        "error.execution" -> StatechartHostInvokerEvent.Error.Execution
        "error.invoke" -> StatechartHostInvokerEvent.Error.Invoke.Self
        "error.invoke.slow" -> StatechartHostInvokerEvent.Error.Invoke.Slow
        "evaluate" -> StatechartHostInvokerEvent.Evaluate
        "forget" -> StatechartHostInvokerEvent.Forget
        "leak" -> StatechartHostInvokerEvent.Leak
        "leave" -> StatechartHostInvokerEvent.Leave
        "locate" -> StatechartHostInvokerEvent.Locate
        "ping" -> StatechartHostInvokerEvent.Ping
        "retype" -> StatechartHostInvokerEvent.Retype
        "time" -> StatechartHostInvokerEvent.Time
        "type" -> StatechartHostInvokerEvent.Type
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartHostInvokerEvent): String? = when (event) {
        is StatechartHostInvokerEvent.Again -> "again"
        is StatechartHostInvokerEvent.Done.Invoke.Self -> "done.invoke"
        is StatechartHostInvokerEvent.Done.Invoke.Perm -> "done.invoke.perm"
        is StatechartHostInvokerEvent.Done.Invoke.Probe -> "done.invoke.probe"
        is StatechartHostInvokerEvent.Done.Invoke.Probe2 -> "done.invoke.probe2"
        is StatechartHostInvokerEvent.Done.Invoke.Slow -> "done.invoke.slow"
        is StatechartHostInvokerEvent.Error.Execution -> "error.execution"
        is StatechartHostInvokerEvent.Error.Invoke.Self -> "error.invoke"
        is StatechartHostInvokerEvent.Error.Invoke.Slow -> "error.invoke.slow"
        is StatechartHostInvokerEvent.Evaluate -> "evaluate"
        is StatechartHostInvokerEvent.Forget -> "forget"
        is StatechartHostInvokerEvent.Leak -> "leak"
        is StatechartHostInvokerEvent.Leave -> "leave"
        is StatechartHostInvokerEvent.Locate -> "locate"
        is StatechartHostInvokerEvent.Ping -> "ping"
        is StatechartHostInvokerEvent.Retype -> "retype"
        is StatechartHostInvokerEvent.Time -> "time"
        is StatechartHostInvokerEvent.Type -> "type"
    }

    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "probe",
        "probe2",
        "req",
        "req2",
        "req3",
        "done._invoke_0",
        "locating._invoke_1",
        "locating._invoke_2",
        "slow",
        "undated",
        "perm",
    )



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML 5.3: the declaration hook `enterAt` reaches. Every other caller
    // arrives through a guard, an assign or a script block, all of which run
    // `ensureScriptEngine()` on their own way in; a resume runs none of them,
    // and a host putting saved values back needs the variables to exist first.
    override fun declareDatamodel() {
        ensureScriptEngine()
    }

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // §scxml-C-1-1 / §scxml-C-2-3: the `_ioprocessors` entries come from the
        // same helper every other backend uses, so a machine reads the same
        // entry names and the same addresses whichever one runs it.
        engine.setupSystemVariables(
            sid,
            "statechart_host_invoker",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'started' with expr
        try {
            val initResult_started = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "started", initResult_started)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='started'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'started2' with expr
        try {
            val initResult_started2 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "started2", initResult_started2)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='started2'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'refused' with expr
        try {
            val initResult_refused = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "refused", initResult_refused)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='refused'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'ended' with expr
        try {
            val initResult_ended = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "ended", initResult_ended)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='ended'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'entered' with expr
        try {
            val initResult_entered = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "entered", initResult_entered)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='entered'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'destination' with expr
        try {
            val initResult_destination = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"pane://dyn\"", "'pane://dyn'"))
            engine.setVariable(sid, "destination", initResult_destination)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='destination'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'n' with expr
        try {
            val initResult_n = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("7", "7"))
            engine.setVariable(sid, "n", initResult_n)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='n'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'dropped' with expr
        try {
            val initResult_dropped = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "dropped", initResult_dropped)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='dropped'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'doneId' with expr
        try {
            val initResult_doneId = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''"))
            engine.setVariable(sid, "doneId", initResult_doneId)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='doneId'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'matched' with expr
        try {
            val initResult_matched = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "matched", initResult_matched)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='matched'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'slot' with expr
        try {
            val initResult_slot = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{[\"id\"] = \"\", [\"sid\"] = \"\"}", "({ id: '', sid: '' })"))
            engine.setVariable(sid, "slot", initResult_slot)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='slot'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'slotted' with expr
        try {
            val initResult_slotted = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "slotted", initResult_slotted)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='slotted'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'pinged' with expr
        try {
            val initResult_pinged = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "pinged", initResult_pinged)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='pinged'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'leaked' with expr
        try {
            val initResult_leaked = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "leaked", initResult_leaked)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='leaked'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'lost' with expr
        try {
            val initResult_lost = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "lost", initResult_lost)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='lost'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'expired' with expr
        try {
            val initResult_expired = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "expired", initResult_expired)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='expired'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'finished' with expr
        try {
            val initResult_finished = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "finished", initResult_finished)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='finished'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'misdated' with expr
        try {
            val initResult_misdated = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "misdated", initResult_misdated)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='misdated'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'granted' with expr
        try {
            val initResult_granted = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "granted", initResult_granted)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='granted'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'denied' with expr
        try {
            val initResult_denied = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "denied", initResult_denied)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='denied'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'unreadable' with expr
        try {
            val initResult_unreadable = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "unreadable", initResult_unreadable)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='unreadable'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'scope' with expr
        try {
            val initResult_scope = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"calendar\"", "'calendar'"))
            engine.setVariable(sid, "scope", initResult_scope)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='scope'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'level' with expr
        try {
            val initResult_level = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("2", "2"))
            engine.setVariable(sid, "level", initResult_level)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<data id='level'> expr failed to evaluate")
        }




        // W3C SCXML 6.4: Apply pending invoke params from parent
        // Only set params matching child's declared datamodel variables (C++ DatamodelValidationHelper)
        if (pendingInvokeParams.isNotEmpty()) {
            for ((pName, pValue) in pendingInvokeParams) {
                if (engine.hasVariable(sid, pName)) {
                    try { engine.setVariable(sid, pName, pValue) } catch (_: Exception) {}
                }
            }
            pendingInvokeParams = emptyMap()
        }

        scriptEngineInitialized = true
    }

    // W3C SCXML 5.9: Guard evaluation with error.execution on failure
    //
    // The guard arrives as a `ScriptSource`, not a `String`: it carries the
    // language its text is in, so a machine generated for a Lua engine hands
    // over Lua the build-time frontend produced and one generated for an
    // ECMAScript engine hands over the author's own text — and the engine is
    // never left to guess which it got. The C++ sibling
    // (`process_transition.jinja2`) takes the same argument for the same
    // reason.
    private fun safeEvaluateGuard(guardExpr: com.sce.runtime.ScriptSource): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "a <transition> cond failed to evaluate")
            false
        }
    }

    // W3C SCXML B.2: the value of an inline `<content>` body, serialized
    // for transport.
    //
    // The reading is decided at build time — `source` is already the
    // expression or string literal the clause's ordered readings give —
    // and this evaluates it *here*, at send time, rather than handing the
    // expression to whatever reads `_event.data` later. That distinction
    // is not academic: the two engines this backend runs on disagree
    // about what a data string is. QuickJS tries a JS evaluation before
    // falling back; Rhino goes straight from JSON to the normalized
    // string, so an expression handed to it arrives as its own source
    // text. `JSON.stringify` is what both of them can read back, and it
    // is the same shape the C++ backend transports.
    //
    // The serialization wraps BOTH halves, in each half's own language. A
    // wrapper composed around one of them only would build a `ScriptSource`
    // whose two strings no longer say the same thing, and the diagnostic that
    // reads `source` would name an expression the engine never ran. `JSON` is
    // a §scxml-B-2-9 name both engines carry, so the wrapper is the same eight
    // characters on either arm — what differs is what it wraps.
    private fun evaluateSendContent(source: com.sce.runtime.ScriptSource): String {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        val serialized = when (source.language) {
            com.sce.runtime.ScriptLanguage.ECMAScript ->
                com.sce.runtime.ScriptSource.ecmascript("JSON.stringify((" + source.source + "))")
            com.sce.runtime.ScriptLanguage.Lua ->
                com.sce.runtime.ScriptSource.lua(
                    "JSON.stringify((" + source.text + "))",
                    "JSON.stringify((" + source.source + "))",
                )
        }
        return try {
            engine.evaluateExpr(sid, serialized)?.toString() ?: ""
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "an expression could not be serialised to JSON")
            ""
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    //
    // Both halves carry a language: this engine's Lua arm splices the location
    // in front of `=` and runs the result, so a write target written in
    // ECMAScript has to have been lowered too. Same split as
    // `ScxmlScriptEngine.assign`.
    private fun executeAssign(location: com.sce.runtime.ScriptSource, expr: com.sce.runtime.ScriptSource) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<assign> failed")
        }
    }

    // W3C SCXML 6.2.4 / 6.4.1: write a generated id to an `idlocation`.
    //
    // The location is a location expression, so this is the assignment
    // `executeAssign` makes: lowered, so a member path lands, and one that
    // cannot take the id raises error.execution (W3C SCXML 5.9.2) and answers
    // false for the caller to abandon the element. The id is quoted here, in
    // the location's own language, because an invoke's is a run-time value.
    private fun storeIdInLocation(location: com.sce.runtime.ScriptSource, id: String, element: String): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.assign(sid, location, com.sce.runtime.ScriptSource.stringLiteral(id, location.language))
            true
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "$element idlocation could not take the id")
            false
        }
    }

    // W3C SCXML 5.8: Script block execution
    private fun executeScriptBlock(script: com.sce.runtime.ScriptSource) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: StatechartHostInvokerEvent) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        val eventName = eventNameOf(event) ?: return
        val meta = currentEventMetadata
        // W3C SCXML 5.10.1: C++ classifyEventType — platform events override type
        val effectiveType = when {
            eventName.startsWith("done.") || eventName.startsWith("error.") -> "platform"
            else -> meta.type
        }
        // W3C SCXML 5.10.1: C++ pattern — origin/origintype only for external events
        // Internal events (<raise>) have empty origin; external events (<send>) have session ID
        // W3C SCXML C.1: `_event.origin` is the sender's published
        // `_ioprocessors` location, not its bare session id — and this is the
        // one place that publishes `_event` to the document, so this is where
        // the id becomes a location. The engine keeps the bare id in
        // `EventMetadata.origin` because its session-keyed lookups (`<finalize>`
        // dispatch, cancelled-invoke filtering) match on it; converting at the
        // raise would make one value serve two consumers that need different
        // spellings. The conversion itself lives in
        // `com.sce.runtime.IoProcessors.publishedOrigin`, the port of the
        // `IOProcessorHelper::publishedOrigin` the C++ engines share: a second
        // spelling of the rule is how the backends would stop agreeing.
        val effectiveOrigin = com.sce.runtime.IoProcessors.publishedOrigin(
            if (meta.type == "external") meta.origin.ifEmpty { scriptSessionId ?: "" } else meta.origin
        )
        val effectiveOriginType = if (meta.type == "external") meta.originType.ifEmpty { "http://www.w3.org/TR/scxml/#SCXMLEventProcessor" } else meta.originType
        // §scxml-B-2-8-1: the binding answers which rung the payload got, and
        // that answer used to end here. The ladder decided between a DOM, a
        // value and a space-normalized string, and the decision was dropped —
        // so a payload that announced structure and would not parse reached
        // the document as raw characters, every `_event.data.<field>` read
        // empty, and nothing anywhere could say so.
        //
        // Recorded on the spot rather than returned up: this class extends
        // `StateMachineEngine`, so the frame that binds already holds both the
        // reading and the event it belongs to — which is the pairing the count
        // needs.
        val payloadReading = engine.setCurrentEvent(
            sid,
            com.sce.runtime.SetCurrentEventArgs(
                name = eventName,
                data = meta.data,
                type = effectiveType,
                sendId = meta.sendId,
                origin = effectiveOrigin,
                originType = effectiveOriginType,
                invokeId = meta.invokeId
            )
        )
        notePayloadReading(event, payloadReading)
    }



    // W3C SCXML 5.10: bind the event as the `_event` its transitions' guards
    // read — once, before the first guard runs, and not for an eventless
    // selection, which has no event of its own.
    override fun bindCurrentEvent(event: StatechartHostInvokerEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StatechartHostInvokerState,
        event: StatechartHostInvokerEvent?
    ): EnabledTransition<StatechartHostInvokerState, HistoryId>? = when (state) {
        is StatechartHostInvokerState.Done -> when {
            (event is StatechartHostInvokerEvent.Done.Invoke || event is StatechartHostInvokerEvent.Done.Invoke.Perm || event is StatechartHostInvokerEvent.Done.Invoke.Probe || event is StatechartHostInvokerEvent.Done.Invoke.Probe2 || event is StatechartHostInvokerEvent.Done.Invoke.Slow) && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.invokeid == doneId)", "_event.invokeid === doneId")) -> transitionDoneAt0
            event is StatechartHostInvokerEvent.Again -> transitionDoneAt1
            else -> null
        }
        is StatechartHostInvokerState.Evaluating -> when {
            event is StatechartHostInvokerEvent.Error.Execution -> transitionEvaluatingAt0
            else -> null
        }
        is StatechartHostInvokerState.Invoking -> when {
            event is StatechartHostInvokerEvent.Done.Invoke.Probe && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.invokeid == \"probe\")", "_event.invokeid === 'probe'")) -> transitionInvokingAt0
            event is StatechartHostInvokerEvent.Done.Invoke.Probe2 && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.invokeid == \"probe2\")", "_event.invokeid === 'probe2'")) -> transitionInvokingAt1
            event is StatechartHostInvokerEvent.Error.Execution -> transitionInvokingAt2
            event is StatechartHostInvokerEvent.Leave -> transitionInvokingAt3
            event is StatechartHostInvokerEvent.Evaluate -> transitionInvokingAt4
            event is StatechartHostInvokerEvent.Locate -> transitionInvokingAt5
            event is StatechartHostInvokerEvent.Time -> transitionInvokingAt6
            event is StatechartHostInvokerEvent.Type -> transitionInvokingAt7
            else -> null
        }
        is StatechartHostInvokerState.Locating -> when {
            (event is StatechartHostInvokerEvent.Done.Invoke || event is StatechartHostInvokerEvent.Done.Invoke.Perm || event is StatechartHostInvokerEvent.Done.Invoke.Probe || event is StatechartHostInvokerEvent.Done.Invoke.Probe2 || event is StatechartHostInvokerEvent.Done.Invoke.Slow) && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.invokeid == slot.id)", "_event.invokeid === slot.id")) -> transitionLocatingAt0
            event is StatechartHostInvokerEvent.Ping -> transitionLocatingAt1
            event is StatechartHostInvokerEvent.Leak -> transitionLocatingAt2
            event is StatechartHostInvokerEvent.Error.Execution -> transitionLocatingAt3
            else -> null
        }
        is StatechartHostInvokerState.Timed -> when {
            event is StatechartHostInvokerEvent.Error.Execution -> transitionTimedAt0
            event is StatechartHostInvokerEvent.Forget -> transitionTimedAt1
            event is StatechartHostInvokerEvent.Leave -> transitionTimedAt2
            event is StatechartHostInvokerEvent.Error.Invoke.Slow && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("((_event.invokeid == \"slow\") and (_event.data == \"deadline\"))", "_event.invokeid === 'slow' && _event.data === 'deadline'")) -> transitionTimedAt3
            event is StatechartHostInvokerEvent.Done.Invoke.Slow && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.invokeid == \"slow\")", "_event.invokeid === 'slow'")) -> transitionTimedAt4
            else -> null
        }
        is StatechartHostInvokerState.Typed -> when {
            event is StatechartHostInvokerEvent.Done.Invoke.Perm && pendingDoneInvokePermPayload != null && (pendingDoneInvokePermPayload!!.granted == true) -> transitionTypedAt0
            event is StatechartHostInvokerEvent.Done.Invoke.Perm && pendingDoneInvokePermPayload != null && (pendingDoneInvokePermPayload!!.granted == false) -> transitionTypedAt1
            event is StatechartHostInvokerEvent.Error.Execution -> transitionTypedAt2
            event is StatechartHostInvokerEvent.Retype -> transitionTypedAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_host_invoker.scxml:106 :: _machine
    override fun onEntry(state: StatechartHostInvokerState, isDefaultEntry: Boolean) {
        when (state) {
            is StatechartHostInvokerState.Done -> {
                // SCE-MAP: statechart_host_invoker.scxml:187 :: done :: _state_body


            executeAssign(com.sce.runtime.ScriptSource.lua("ended", "ended"), com.sce.runtime.ScriptSource.lua("_scxml_add(ended, 1)", "ended + 1"))
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "done.${System.identityHashCode(this)}.done._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4.1: what the request says is evaluated
                        // now, when the invocation starts. An attribute that
                        // cannot be evaluated raises error.execution and starts
                        // nothing; a `<param>` that cannot is reported and
                        // dropped (W3C SCXML 5.7.1) while the invocation starts.
                        ensureScriptEngine()
                        val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        // W3C SCXML 6.4.1: the invocation's generated id goes
                        // to `idlocation` when the element is evaluated. It is
                        // the id the host is handed and `done.invoke.<id>`
                        // names, so the document can match on what it stored.
                        // The write is the assignment `<assign>` makes, so a
                        // location that cannot take the id starts nothing.
                        if (!storeIdInLocation(com.sce.runtime.ScriptSource.lua("doneId", "doneId"), "done._invoke_0", "<invoke>")) return@deferInvoke
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "done._invoke_0",
                                src = "",
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StatechartHostInvokerState.Evaluating -> {
                // SCE-MAP: statechart_host_invoker.scxml:171 :: evaluating :: _state_body
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "evaluating.${System.identityHashCode(this)}.req"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4.1: what the request says is evaluated
                        // now, when the invocation starts. An attribute that
                        // cannot be evaluated raises error.execution and starts
                        // nothing; a `<param>` that cannot is reported and
                        // dropped (W3C SCXML 5.7.1) while the invocation starts.
                        ensureScriptEngine()
                        val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val hostInvokeSrc = try {
                            valueToWireString(hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("destination", "destination")))
                        } catch (_: Exception) {
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> srcexpr failed to evaluate")
                            return@deferInvoke
                        }
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        if (!hostEngine.hasVariable(hostSid, "n")) {
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> namelist names 'n', which is not declared")
                            return@deferInvoke
                        }
                        try {
                            hostInvokeParams["n"] =
                                (hostInvokeParams["n"] ?: emptyList()) + valueToWireString(hostEngine.getVariable(hostSid, "n"))
                        } catch (_: Exception) {
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> namelist entry 'n' failed to evaluate")
                            return@deferInvoke
                        }
                        hostInvokeParams["twice"] =
                            (hostInvokeParams["twice"] ?: emptyList()) + "a"
                        try {
                            // The param crosses as text, and `toString()` is the platform's
                            // spelling of the value; this is the document's.
                            val v = hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("_scxml_add(n, 1)", "n + 1"))
                            hostInvokeParams["twice"] =
                                (hostInvokeParams["twice"] ?: emptyList()) + valueToWireString(v)
                        } catch (_: Exception) {
                            // W3C SCXML 5.7.1: report the failure and omit the name and the
                            // value — the act still happens, without a field the document
                            // could not produce.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> <param name='twice'> expr failed to evaluate")
                        }
                        try {
                            // The param crosses as text, and `toString()` is the platform's
                            // spelling of the value; this is the document's.
                            val v = hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("n.nope.deeper", "n.nope.deeper"))
                            hostInvokeParams["bad"] =
                                (hostInvokeParams["bad"] ?: emptyList()) + valueToWireString(v)
                        } catch (_: Exception) {
                            // W3C SCXML 5.7.1: report the failure and omit the name and the
                            // value — the act still happens, without a field the document
                            // could not produce.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> <param name='bad'> expr failed to evaluate")
                        }
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "req",
                                src = hostInvokeSrc,
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "evaluating.${System.identityHashCode(this)}.req2"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4.1: what the request says is evaluated
                        // now, when the invocation starts. An attribute that
                        // cannot be evaluated raises error.execution and starts
                        // nothing; a `<param>` that cannot is reported and
                        // dropped (W3C SCXML 5.7.1) while the invocation starts.
                        ensureScriptEngine()
                        val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val hostInvokeContent = try {
                            valueToWireString(hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("(\"body:\" .. _scxml_tostring(n))", "'body:' + n")))
                        } catch (_: Exception) {
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> contentexpr failed to evaluate")
                            return@deferInvoke
                        }
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "req2",
                                src = "",
                                params = hostInvokeParams,
                                content = hostInvokeContent                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "evaluating.${System.identityHashCode(this)}.req3"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4.1: what the request says is evaluated
                        // now, when the invocation starts. An attribute that
                        // cannot be evaluated raises error.execution and starts
                        // nothing; a `<param>` that cannot is reported and
                        // dropped (W3C SCXML 5.7.1) while the invocation starts.
                        ensureScriptEngine()
                        val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val hostInvokeSrc = try {
                            valueToWireString(hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("n.nope.deeper", "n.nope.deeper")))
                        } catch (_: Exception) {
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> srcexpr failed to evaluate")
                            return@deferInvoke
                        }
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "req3",
                                src = hostInvokeSrc,
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StatechartHostInvokerState.Invoking -> {
                // SCE-MAP: statechart_host_invoker.scxml:143 :: invoking :: _state_body


            executeAssign(com.sce.runtime.ScriptSource.lua("entered", "entered"), com.sce.runtime.ScriptSource.lua("_scxml_add(entered, 1)", "entered + 1"))
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "invoking.${System.identityHashCode(this)}.probe"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        hostInvokeParams["within"] =
                            (hostInvokeParams["within"] ?: emptyList()) + "2500"
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "probe",
                                src = "pane://turn",
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "invoking.${System.identityHashCode(this)}.probe2"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "probe2",
                                src = "pane://other",
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StatechartHostInvokerState.Locating -> {
                // SCE-MAP: statechart_host_invoker.scxml:198 :: locating :: _state_body


            // W3C SCXML 6.2.4: Store sendid in idlocation (test183, test332),
            // through the assignment `<assign>` makes — the location is lowered,
            // so a member path lands. A location that cannot take the id is an
            // argument that cannot be evaluated, so the message is discarded
            // (W3C SCXML 6.2, 5.9.2); the whole send is the labelled block
            // returned from.
            run send@{
            if (!storeIdInLocation(com.sce.runtime.ScriptSource.lua("slot.sid", "slot.sid"), "__send_0", "<send>")) return@send
            send(StatechartHostInvokerEvent.Ping, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            // W3C SCXML 6.2.4: Store sendid in idlocation (test183, test332),
            // through the assignment `<assign>` makes — the location is lowered,
            // so a member path lands. A location that cannot take the id is an
            // argument that cannot be evaluated, so the message is discarded
            // (W3C SCXML 6.2, 5.9.2); the whole send is the labelled block
            // returned from.
            run send@{
            if (!storeIdInLocation(com.sce.runtime.ScriptSource.lua("n.nope.deeper", "n.nope.deeper"), "__send_1", "<send>")) return@send
            send(StatechartHostInvokerEvent.Leak, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "locating.${System.identityHashCode(this)}.locating._invoke_1"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4.1: what the request says is evaluated
                        // now, when the invocation starts. An attribute that
                        // cannot be evaluated raises error.execution and starts
                        // nothing; a `<param>` that cannot is reported and
                        // dropped (W3C SCXML 5.7.1) while the invocation starts.
                        ensureScriptEngine()
                        val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        // W3C SCXML 6.4.1: the invocation's generated id goes
                        // to `idlocation` when the element is evaluated. It is
                        // the id the host is handed and `done.invoke.<id>`
                        // names, so the document can match on what it stored.
                        // The write is the assignment `<assign>` makes, so a
                        // location that cannot take the id starts nothing.
                        if (!storeIdInLocation(com.sce.runtime.ScriptSource.lua("slot.id", "slot.id"), "locating._invoke_1", "<invoke>")) return@deferInvoke
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "locating._invoke_1",
                                src = "",
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "locating.${System.identityHashCode(this)}.locating._invoke_2"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4.1: what the request says is evaluated
                        // now, when the invocation starts. An attribute that
                        // cannot be evaluated raises error.execution and starts
                        // nothing; a `<param>` that cannot is reported and
                        // dropped (W3C SCXML 5.7.1) while the invocation starts.
                        ensureScriptEngine()
                        val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        // W3C SCXML 6.4.1: the invocation's generated id goes
                        // to `idlocation` when the element is evaluated. It is
                        // the id the host is handed and `done.invoke.<id>`
                        // names, so the document can match on what it stored.
                        // The write is the assignment `<assign>` makes, so a
                        // location that cannot take the id starts nothing.
                        if (!storeIdInLocation(com.sce.runtime.ScriptSource.lua("n.nope.deeper", "n.nope.deeper"), "locating._invoke_2", "<invoke>")) return@deferInvoke
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "locating._invoke_2",
                                src = "",
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StatechartHostInvokerState.Timed -> {
                // SCE-MAP: statechart_host_invoker.scxml:220 :: timed :: _state_body
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "timed.${System.identityHashCode(this)}.slow"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4.1: what the request says is evaluated
                        // now, when the invocation starts. An attribute that
                        // cannot be evaluated raises error.execution and starts
                        // nothing; a `<param>` that cannot is reported and
                        // dropped (W3C SCXML 5.7.1) while the invocation starts.
                        ensureScriptEngine()
                        val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        try {
                            // The param crosses as text, and `toString()` is the platform's
                            // spelling of the value; this is the document's.
                            val v = hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("50", "50"))
                            hostInvokeParams["_sce_deadline_ms"] =
                                (hostInvokeParams["_sce_deadline_ms"] ?: emptyList()) + valueToWireString(v)
                        } catch (_: Exception) {
                            // W3C SCXML 5.7.1: report the failure and omit the name and the
                            // value — the act still happens, without a field the document
                            // could not produce.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> <param name='_sce_deadline_ms'> expr failed to evaluate")
                        }
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "slow",
                                src = "",
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "timed.${System.identityHashCode(this)}.undated"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        hostInvokeParams["_sce_deadline_ms"] =
                            (hostInvokeParams["_sce_deadline_ms"] ?: emptyList()) + "soon"
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "undated",
                                src = "",
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StatechartHostInvokerState.Typed -> {
                // SCE-MAP: statechart_host_invoker.scxml:254 :: typed :: _state_body
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "typed.${System.identityHashCode(this)}.perm"
                    deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4.1: what the request says is evaluated
                        // now, when the invocation starts. An attribute that
                        // cannot be evaluated raises error.execution and starts
                        // nothing; a `<param>` that cannot is reported and
                        // dropped (W3C SCXML 5.7.1) while the invocation starts.
                        ensureScriptEngine()
                        val hostEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val hostSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        try {
                            // The param crosses as text, and `toString()` is the platform's
                            // spelling of the value; this is the document's.
                            val v = hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("scope", "scope"))
                            hostInvokeParams["scope"] =
                                (hostInvokeParams["scope"] ?: emptyList()) + valueToWireString(v)
                        } catch (_: Exception) {
                            // W3C SCXML 5.7.1: report the failure and omit the name and the
                            // value — the act still happens, without a field the document
                            // could not produce.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> <param name='scope'> expr failed to evaluate")
                        }
                        try {
                            // The param crosses as text, and `toString()` is the platform's
                            // spelling of the value; this is the document's.
                            val v = hostEngine.evaluateExpr(hostSid, com.sce.runtime.ScriptSource.lua("level", "level"))
                            hostInvokeParams["level"] =
                                (hostInvokeParams["level"] ?: emptyList()) + valueToWireString(v)
                        } catch (_: Exception) {
                            // W3C SCXML 5.7.1: report the failure and omit the name and the
                            // value — the act still happens, without a field the document
                            // could not produce.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> <param name='level'> expr failed to evaluate")
                        }
val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "perm",
                                src = "",
                                params = hostInvokeParams,
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartHostInvokerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_host_invoker.scxml:106 :: _machine
    override fun onExit(state: StatechartHostInvokerState) {
        when (state) {
            is StatechartHostInvokerState.Done -> {
                // SCE-MAP: statechart_host_invoker.scxml:187 :: done :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "done._invoke_0")
            }
            is StatechartHostInvokerState.Evaluating -> {
                // SCE-MAP: statechart_host_invoker.scxml:171 :: evaluating :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "req")
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "req2")
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "req3")
            }
            is StatechartHostInvokerState.Invoking -> {
                // SCE-MAP: statechart_host_invoker.scxml:143 :: invoking :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "probe")
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "probe2")
            }
            is StatechartHostInvokerState.Locating -> {
                // SCE-MAP: statechart_host_invoker.scxml:198 :: locating :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "locating._invoke_1")
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "locating._invoke_2")
            }
            is StatechartHostInvokerState.Timed -> {
                // SCE-MAP: statechart_host_invoker.scxml:220 :: timed :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "slow")
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "undated")
            }
            is StatechartHostInvokerState.Typed -> {
                // SCE-MAP: statechart_host_invoker.scxml:254 :: typed :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "perm")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: statechart_host_invoker.scxml:106 :: _machine
    override fun executeTransitionContent(source: StatechartHostInvokerState, transitionIndex: Int) {
        when (source) {
        is StatechartHostInvokerState.Done -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_host_invoker.scxml:192 :: done :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("matched", "matched"), com.sce.runtime.ScriptSource.lua("_scxml_add(matched, 1)", "matched + 1"))
            }
            else -> {}
        }
        is StatechartHostInvokerState.Evaluating -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_host_invoker.scxml:182 :: evaluating :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("dropped", "dropped"), com.sce.runtime.ScriptSource.lua("_scxml_add(dropped, 1)", "dropped + 1"))
            }
            else -> {}
        }
        is StatechartHostInvokerState.Invoking -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_host_invoker.scxml:153 :: invoking :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("started", "started"), com.sce.runtime.ScriptSource.lua("_scxml_add(started, 1)", "started + 1"))
            }
            1 -> {
                // SCE-MAP: statechart_host_invoker.scxml:156 :: invoking :: _transition_1


            executeAssign(com.sce.runtime.ScriptSource.lua("started2", "started2"), com.sce.runtime.ScriptSource.lua("_scxml_add(started2, 1)", "started2 + 1"))
            }
            2 -> {
                // SCE-MAP: statechart_host_invoker.scxml:159 :: invoking :: _transition_2


            executeAssign(com.sce.runtime.ScriptSource.lua("refused", "refused"), com.sce.runtime.ScriptSource.lua("_scxml_add(refused, 1)", "refused + 1"))
            }
            else -> {}
        }
        is StatechartHostInvokerState.Locating -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_host_invoker.scxml:206 :: locating :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("slotted", "slotted"), com.sce.runtime.ScriptSource.lua("_scxml_add(slotted, 1)", "slotted + 1"))
            }
            1 -> {
                // SCE-MAP: statechart_host_invoker.scxml:209 :: locating :: _transition_1


            executeAssign(com.sce.runtime.ScriptSource.lua("pinged", "pinged"), com.sce.runtime.ScriptSource.lua("_scxml_add(pinged, 1)", "pinged + 1"))
            }
            2 -> {
                // SCE-MAP: statechart_host_invoker.scxml:212 :: locating :: _transition_2


            executeAssign(com.sce.runtime.ScriptSource.lua("leaked", "leaked"), com.sce.runtime.ScriptSource.lua("_scxml_add(leaked, 1)", "leaked + 1"))
            }
            3 -> {
                // SCE-MAP: statechart_host_invoker.scxml:215 :: locating :: _transition_3


            executeAssign(com.sce.runtime.ScriptSource.lua("lost", "lost"), com.sce.runtime.ScriptSource.lua("_scxml_add(lost, 1)", "lost + 1"))
            }
            else -> {}
        }
        is StatechartHostInvokerState.Timed -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_host_invoker.scxml:227 :: timed :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("misdated", "misdated"), com.sce.runtime.ScriptSource.lua("_scxml_add(misdated, 1)", "misdated + 1"))
            }
            1 -> {
                // SCE-MAP: statechart_host_invoker.scxml:233 :: timed :: _transition_1


            // W3C SCXML 6.3: Dynamic sendid evaluation (test210)
            run {
                ensureScriptEngine()
                val engineCancel = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidCancel = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    val v = engineCancel.evaluateExpr(sidCancel, com.sce.runtime.ScriptSource.lua("\"\"", "''"))
                    val sendidToCancel = v?.toString() ?: ""
                    if (sendidToCancel.isNotEmpty()) cancelSend(sendidToCancel)
                } catch (_: Exception) {}
            }
            }
            3 -> {
                // SCE-MAP: statechart_host_invoker.scxml:240 :: timed :: _transition_3


            executeAssign(com.sce.runtime.ScriptSource.lua("expired", "expired"), com.sce.runtime.ScriptSource.lua("_scxml_add(expired, 1)", "expired + 1"))
            }
            4 -> {
                // SCE-MAP: statechart_host_invoker.scxml:244 :: timed :: _transition_4


            executeAssign(com.sce.runtime.ScriptSource.lua("finished", "finished"), com.sce.runtime.ScriptSource.lua("_scxml_add(finished, 1)", "finished + 1"))
            }
            else -> {}
        }
        is StatechartHostInvokerState.Typed -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_host_invoker.scxml:259 :: typed :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("granted", "granted"), com.sce.runtime.ScriptSource.lua("_scxml_add(granted, 1)", "granted + 1"))
            }
            1 -> {
                // SCE-MAP: statechart_host_invoker.scxml:262 :: typed :: _transition_1


            executeAssign(com.sce.runtime.ScriptSource.lua("denied", "denied"), com.sce.runtime.ScriptSource.lua("_scxml_add(denied, 1)", "denied + 1"))
            }
            2 -> {
                // SCE-MAP: statechart_host_invoker.scxml:265 :: typed :: _transition_2


            executeAssign(com.sce.runtime.ScriptSource.lua("unreadable", "unreadable"), com.sce.runtime.ScriptSource.lua("_scxml_add(unreadable, 1)", "unreadable + 1"))
            }
            3 -> {
                // SCE-MAP: statechart_host_invoker.scxml:268 :: typed :: _transition_3


            executeAssign(com.sce.runtime.ScriptSource.lua("level", "level"), com.sce.runtime.ScriptSource.lua("\"high\"", "'high'"))
            }
            else -> {}
        }
        }
    }
}
