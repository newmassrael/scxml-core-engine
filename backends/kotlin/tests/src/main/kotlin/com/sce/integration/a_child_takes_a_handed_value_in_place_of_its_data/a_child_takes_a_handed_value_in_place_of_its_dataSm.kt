// SCE-GENERATED — DO NOT EDIT
// source-hash: 428dfedc40362bcde893d2a13aaf88295d66fbb3c5dc619cc8fae908cc42df31
// source: a_child_takes_a_handed_value_in_place_of_its_data.scxml

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_child_takes_a_handed_value_in_place_of_its_data/a_child_takes_a_handed_value_in_place_of_its_data.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:69 :: _machine

package com.sce.integration.a_child_takes_a_handed_value_in_place_of_its_data

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AChildTakesAHandedValueInPlaceOfItsDataState : State {
    data object ByNamelist : AChildTakesAHandedValueInPlaceOfItsDataState
    data object ByParam : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailChildRaised : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailChildSharedTheParentsObject : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailEntryReadItsOwnValue : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailHandedValueLost : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailLateStateDataTookTheValue : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailNamelistNameAdded : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailNamelistOwnValueRaised : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailNamelistValueLost : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailObjectNotWhole : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailOwnValueRan : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailParentRaised : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailScriptReadItsOwnValue : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailStateDataTookTheValue : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FailUnmatchedNameAdded : AChildTakesAHandedValueInPlaceOfItsDataState
    data object FirstMacrostep : AChildTakesAHandedValueInPlaceOfItsDataState
    data object JudgeNames : AChildTakesAHandedValueInPlaceOfItsDataState
    data object JudgeParams : AChildTakesAHandedValueInPlaceOfItsDataState
    data object LateBound : AChildTakesAHandedValueInPlaceOfItsDataState
    data object Pass : AChildTakesAHandedValueInPlaceOfItsDataState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AChildTakesAHandedValueInPlaceOfItsDataEvent : Event {
    sealed interface Done : AChildTakesAHandedValueInPlaceOfItsDataEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object FirstMacrostep : Invoke
        }
    }
    sealed interface Error : AChildTakesAHandedValueInPlaceOfItsDataEvent {
        data object Execution : Error
    }
    sealed interface Late : AChildTakesAHandedValueInPlaceOfItsDataEvent {
        data object Kept : Late
        data object Lost : Late
        data object Ok : Late
    }
    sealed interface Names : AChildTakesAHandedValueInPlaceOfItsDataEvent {
        data object Report : Names
    }
    sealed interface Params : AChildTakesAHandedValueInPlaceOfItsDataEvent {
        data object Report : Params
    }
}
// --- State Machine (W3C SCXML) ---

class AChildTakesAHandedValueInPlaceOfItsDataStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<AChildTakesAHandedValueInPlaceOfItsDataState, AChildTakesAHandedValueInPlaceOfItsDataEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `record` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `record` was assigned a value of another type, or the engine refused.
     *
     * The value as JSON text, serialised by the engine's own `JSON.stringify`
     * (§scxml-B-2) so the key order is the document's.
     */
    fun record(): String? =
        com.sce.runtime.DatamodelRead.readJson(scriptEngine, scriptSessionId, "record")

    /**
     * §scxml-5.3: what the `listed` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `listed` was assigned a value of another type, or the engine refused.
     */
    fun listed(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "listed")

    /**
     * §scxml-5.3: what the `stray` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `stray` was assigned a value of another type, or the engine refused.
     */
    fun stray(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "stray")

    /**
     * §scxml-5.3: what the `raising_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `raising_seen` was assigned a value of another type, or the engine refused.
     */
    fun raisingSeen(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "raising_seen")

    /**
     * §scxml-5.3: what the `effectful_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `effectful_seen` was assigned a value of another type, or the engine refused.
     */
    fun effectfulSeen(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "effectful_seen")

    /**
     * §scxml-5.3: what the `effects_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `effects_seen` was assigned a value of another type, or the engine refused.
     */
    fun effectsSeen(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "effects_seen")

    /**
     * §scxml-5.3: what the `refusable_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `refusable_seen` was assigned a value of another type, or the engine refused.
     */
    fun refusableSeen(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "refusable_seen")

    /**
     * §scxml-5.3: what the `kept_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `kept_seen` was assigned a value of another type, or the engine refused.
     */
    fun keptSeen(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "kept_seen")

    /**
     * §scxml-5.3: what the `stranger_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `stranger_seen` was assigned a value of another type, or the engine refused.
     */
    fun strangerSeen(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "stranger_seen")

    /**
     * §scxml-5.3: what the `payload_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `payload_seen` was assigned a value of another type, or the engine refused.
     */
    fun payloadSeen(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "payload_seen")

    /**
     * §scxml-5.3: what the `script_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `script_seen` was assigned a value of another type, or the engine refused.
     */
    fun scriptSeen(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "script_seen")

    /**
     * §scxml-5.3: what the `entry_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `entry_seen` was assigned a value of another type, or the engine refused.
     */
    fun entrySeen(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "entry_seen")

    /**
     * §scxml-5.3: what the `errors_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `errors_seen` was assigned a value of another type, or the engine refused.
     */
    fun errorsSeen(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "errors_seen")

    /**
     * §scxml-5.3: what the `listed_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `listed_seen` was assigned a value of another type, or the engine refused.
     */
    fun listedSeen(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "listed_seen")

    /**
     * §scxml-5.3: what the `stray_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `stray_seen` was assigned a value of another type, or the engine refused.
     */
    fun straySeen(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "stray_seen")

    /**
     * §scxml-5.3: what the `record_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `record_seen` was assigned a value of another type, or the engine refused.
     */
    fun recordSeen(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "record_seen")

    /**
     * §scxml-5.3: what the `list_errors_seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `list_errors_seen` was assigned a value of another type, or the engine refused.
     */
    fun listErrorsSeen(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "list_errors_seen")

    override val initialState: AChildTakesAHandedValueInPlaceOfItsDataState = AChildTakesAHandedValueInPlaceOfItsDataState.ByParam

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

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: AChildTakesAHandedValueInPlaceOfItsDataState): Boolean = when (state) {
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailChildRaised -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailChildSharedTheParentsObject -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailEntryReadItsOwnValue -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailHandedValueLost -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailLateStateDataTookTheValue -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistNameAdded -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistOwnValueRaised -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistValueLost -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailObjectNotWhole -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailOwnValueRan -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailParentRaised -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailScriptReadItsOwnValue -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailStateDataTookTheValue -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailUnmatchedNameAdded -> true
        is AChildTakesAHandedValueInPlaceOfItsDataState.Pass -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>> =
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.ByParam))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: byNamelist's transition 0, as the microstep reads it.
        val transitionByNamelistAt0 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.ByNamelist,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.JudgeNames)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: byNamelist's transition 1, as the microstep reads it.
        val transitionByNamelistAt1 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.ByNamelist,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailParentRaised)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: byParam's transition 0, as the microstep reads it.
        val transitionByParamAt0 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.ByParam,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: byParam's transition 1, as the microstep reads it.
        val transitionByParamAt1 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.ByParam,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailParentRaised)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: firstMacrostep's transition 0, as the microstep reads it.
        val transitionFirstMacrostepAt0 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.FirstMacrostep,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: firstMacrostep's transition 1, as the microstep reads it.
        val transitionFirstMacrostepAt1 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.FirstMacrostep,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailParentRaised)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeNames's transition 0, as the microstep reads it.
        val transitionJudgeNamesAt0 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeNames,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistOwnValueRaised)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeNames's transition 1, as the microstep reads it.
        val transitionJudgeNamesAt1 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeNames,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistValueLost)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeNames's transition 2, as the microstep reads it.
        val transitionJudgeNamesAt2 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeNames,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistNameAdded)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeNames's transition 3, as the microstep reads it.
        val transitionJudgeNamesAt3 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeNames,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailObjectNotWhole)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeNames's transition 4, as the microstep reads it.
        val transitionJudgeNamesAt4 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeNames,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.LateBound)),
            4,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeParams's transition 0, as the microstep reads it.
        val transitionJudgeParamsAt0 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailOwnValueRan)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeParams's transition 1, as the microstep reads it.
        val transitionJudgeParamsAt1 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailHandedValueLost)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeParams's transition 2, as the microstep reads it.
        val transitionJudgeParamsAt2 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailStateDataTookTheValue)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeParams's transition 3, as the microstep reads it.
        val transitionJudgeParamsAt3 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailUnmatchedNameAdded)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeParams's transition 4, as the microstep reads it.
        val transitionJudgeParamsAt4 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailObjectNotWhole)),
            4,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeParams's transition 5, as the microstep reads it.
        val transitionJudgeParamsAt5 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailChildSharedTheParentsObject)),
            5,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeParams's transition 6, as the microstep reads it.
        val transitionJudgeParamsAt6 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailScriptReadItsOwnValue)),
            6,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeParams's transition 7, as the microstep reads it.
        val transitionJudgeParamsAt7 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailEntryReadItsOwnValue)),
            7,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeParams's transition 8, as the microstep reads it.
        val transitionJudgeParamsAt8 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailChildRaised)),
            8,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeParams's transition 9, as the microstep reads it.
        val transitionJudgeParamsAt9 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailHandedValueLost)),
            9,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: judgeParams's transition 10, as the microstep reads it.
        val transitionJudgeParamsAt10 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.ByNamelist)),
            10,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: lateBound's transition 0, as the microstep reads it.
        val transitionLateBoundAt0 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.LateBound,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FirstMacrostep)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: lateBound's transition 1, as the microstep reads it.
        val transitionLateBoundAt1 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.LateBound,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailLateStateDataTookTheValue)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: lateBound's transition 2, as the microstep reads it.
        val transitionLateBoundAt2 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.LateBound,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailHandedValueLost)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: lateBound's transition 3, as the microstep reads it.
        val transitionLateBoundAt3 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataState.LateBound,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataState.FailParentRaised)),
            3,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AChildTakesAHandedValueInPlaceOfItsDataState? = when (stateId) {
        "byNamelist" -> AChildTakesAHandedValueInPlaceOfItsDataState.ByNamelist
        "byParam" -> AChildTakesAHandedValueInPlaceOfItsDataState.ByParam
        "failChildRaised" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailChildRaised
        "failChildSharedTheParentsObject" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailChildSharedTheParentsObject
        "failEntryReadItsOwnValue" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailEntryReadItsOwnValue
        "failHandedValueLost" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailHandedValueLost
        "failLateStateDataTookTheValue" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailLateStateDataTookTheValue
        "failNamelistNameAdded" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistNameAdded
        "failNamelistOwnValueRaised" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistOwnValueRaised
        "failNamelistValueLost" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistValueLost
        "failObjectNotWhole" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailObjectNotWhole
        "failOwnValueRan" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailOwnValueRan
        "failParentRaised" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailParentRaised
        "failScriptReadItsOwnValue" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailScriptReadItsOwnValue
        "failStateDataTookTheValue" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailStateDataTookTheValue
        "failUnmatchedNameAdded" -> AChildTakesAHandedValueInPlaceOfItsDataState.FailUnmatchedNameAdded
        "firstMacrostep" -> AChildTakesAHandedValueInPlaceOfItsDataState.FirstMacrostep
        "judgeNames" -> AChildTakesAHandedValueInPlaceOfItsDataState.JudgeNames
        "judgeParams" -> AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams
        "lateBound" -> AChildTakesAHandedValueInPlaceOfItsDataState.LateBound
        "pass" -> AChildTakesAHandedValueInPlaceOfItsDataState.Pass
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AChildTakesAHandedValueInPlaceOfItsDataState): String = when (state) {
        is AChildTakesAHandedValueInPlaceOfItsDataState.ByNamelist -> "byNamelist"
        is AChildTakesAHandedValueInPlaceOfItsDataState.ByParam -> "byParam"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailChildRaised -> "failChildRaised"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailChildSharedTheParentsObject -> "failChildSharedTheParentsObject"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailEntryReadItsOwnValue -> "failEntryReadItsOwnValue"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailHandedValueLost -> "failHandedValueLost"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailLateStateDataTookTheValue -> "failLateStateDataTookTheValue"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistNameAdded -> "failNamelistNameAdded"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistOwnValueRaised -> "failNamelistOwnValueRaised"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistValueLost -> "failNamelistValueLost"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailObjectNotWhole -> "failObjectNotWhole"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailOwnValueRan -> "failOwnValueRan"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailParentRaised -> "failParentRaised"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailScriptReadItsOwnValue -> "failScriptReadItsOwnValue"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailStateDataTookTheValue -> "failStateDataTookTheValue"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailUnmatchedNameAdded -> "failUnmatchedNameAdded"
        is AChildTakesAHandedValueInPlaceOfItsDataState.FirstMacrostep -> "firstMacrostep"
        is AChildTakesAHandedValueInPlaceOfItsDataState.JudgeNames -> "judgeNames"
        is AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams -> "judgeParams"
        is AChildTakesAHandedValueInPlaceOfItsDataState.LateBound -> "lateBound"
        is AChildTakesAHandedValueInPlaceOfItsDataState.Pass -> "pass"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AChildTakesAHandedValueInPlaceOfItsDataState): Int = when (state) {
        is AChildTakesAHandedValueInPlaceOfItsDataState.ByNamelist -> 2
        is AChildTakesAHandedValueInPlaceOfItsDataState.ByParam -> 0
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailChildRaised -> 15
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailChildSharedTheParentsObject -> 12
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailEntryReadItsOwnValue -> 14
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailHandedValueLost -> 8
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailLateStateDataTookTheValue -> 19
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistNameAdded -> 18
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistOwnValueRaised -> 16
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistValueLost -> 17
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailObjectNotWhole -> 11
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailOwnValueRan -> 7
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailParentRaised -> 20
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailScriptReadItsOwnValue -> 13
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailStateDataTookTheValue -> 9
        is AChildTakesAHandedValueInPlaceOfItsDataState.FailUnmatchedNameAdded -> 10
        is AChildTakesAHandedValueInPlaceOfItsDataState.FirstMacrostep -> 5
        is AChildTakesAHandedValueInPlaceOfItsDataState.JudgeNames -> 3
        is AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams -> 1
        is AChildTakesAHandedValueInPlaceOfItsDataState.LateBound -> 4
        is AChildTakesAHandedValueInPlaceOfItsDataState.Pass -> 6
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AChildTakesAHandedValueInPlaceOfItsDataEvent? = when (name) {
        "done.invoke" -> AChildTakesAHandedValueInPlaceOfItsDataEvent.Done.Invoke.Self
        "done.invoke.first_macrostep" -> AChildTakesAHandedValueInPlaceOfItsDataEvent.Done.Invoke.FirstMacrostep
        "error.execution" -> AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution
        "late.kept" -> AChildTakesAHandedValueInPlaceOfItsDataEvent.Late.Kept
        "late.lost" -> AChildTakesAHandedValueInPlaceOfItsDataEvent.Late.Lost
        "late.ok" -> AChildTakesAHandedValueInPlaceOfItsDataEvent.Late.Ok
        "names.report" -> AChildTakesAHandedValueInPlaceOfItsDataEvent.Names.Report
        "params.report" -> AChildTakesAHandedValueInPlaceOfItsDataEvent.Params.Report
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AChildTakesAHandedValueInPlaceOfItsDataEvent): String? = when (event) {
        is AChildTakesAHandedValueInPlaceOfItsDataEvent.Done.Invoke.Self -> "done.invoke"
        is AChildTakesAHandedValueInPlaceOfItsDataEvent.Done.Invoke.FirstMacrostep -> "done.invoke.first_macrostep"
        is AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution -> "error.execution"
        is AChildTakesAHandedValueInPlaceOfItsDataEvent.Late.Kept -> "late.kept"
        is AChildTakesAHandedValueInPlaceOfItsDataEvent.Late.Lost -> "late.lost"
        is AChildTakesAHandedValueInPlaceOfItsDataEvent.Late.Ok -> "late.ok"
        is AChildTakesAHandedValueInPlaceOfItsDataEvent.Names.Report -> "names.report"
        is AChildTakesAHandedValueInPlaceOfItsDataEvent.Params.Report -> "params.report"
    }




    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // docs/adr/0096: make this machine's script session, the first step of
    // every start (`initialize`, `start`, `enterAt`, and the start of a child
    // this machine is). The runtime's `makeScriptSession` takes the three steps
    // (create the session, which loads the shared library into it, then bind
    // the system variables), releases what a failed step made and answers the
    // failure, which refuses the start with nothing entered; a later start
    // asks for the session again.
    override fun establishScriptSession(): com.sce.runtime.ScriptSessionFailure? {
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        makeScriptSession(
            engine,
            "a_child_takes_a_handed_value_in_place_of_its_data",
        )?.let { return it }
        return null
    }

    // W3C SCXML 5.3: initialize the datamodel in the session the start
    // established, before any state is entered: the document's <data>, its
    // states' early-bound <data>, then the top-level <script>s, the order of
    // interpret in Appendix D. §scxml-6.4: a value the invoke that started
    // this session handed it under the id of a top-level <data> is that
    // <data>'s initial value, declared in place of its own, so the scripts,
    // the <onentry>s and the conditions of the first macrostep read it; a
    // state's <data> never takes one, and a name no top-level <data> has is
    // not added (docs/adr/0098).
    override fun declareDatamodel() {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")

        declareVariablesPart0(engine, sid)

    }

    @Suppress("UNUSED_PARAMETER")
    private fun declareVariablesPart0(engine: ScxmlScriptEngine, sid: String) {
        // §scxml-6.4: 'record' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "record", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'record' with expr
            try {
                engine.setVariable(sid, "record", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{[\"name\"] = \"whole\", [\"inner\"] = {[\"label\"] = \"inner\", [\"depth\"] = 2}, [\"list\"] = {1, 2, 3}}", "({name: 'whole', inner: {label: 'inner', depth: 2}, list: [1, 2, 3]})")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='record'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'listed' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "listed", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'listed' with expr
            try {
                engine.setVariable(sid, "listed", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"handed listed\"", "'handed listed'")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='listed'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'stray' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "stray", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'stray' with expr
            try {
                engine.setVariable(sid, "stray", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"handed stray\"", "'handed stray'")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='stray'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'raising_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "raising_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'raising_seen' with expr
            try {
                engine.setVariable(sid, "raising_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='raising_seen'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'effectful_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "effectful_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'effectful_seen' with expr
            try {
                engine.setVariable(sid, "effectful_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='effectful_seen'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'effects_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "effects_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'effects_seen' with expr
            try {
                engine.setVariable(sid, "effects_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("(-1)", "-1")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='effects_seen'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'refusable_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "refusable_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'refusable_seen' with expr
            try {
                engine.setVariable(sid, "refusable_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='refusable_seen'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'kept_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "kept_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'kept_seen' with expr
            try {
                engine.setVariable(sid, "kept_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='kept_seen'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'stranger_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "stranger_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'stranger_seen' with expr
            try {
                engine.setVariable(sid, "stranger_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='stranger_seen'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'payload_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "payload_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'payload_seen' with expr
            try {
                engine.setVariable(sid, "payload_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='payload_seen'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'script_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "script_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'script_seen' with expr
            try {
                engine.setVariable(sid, "script_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='script_seen'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'entry_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "entry_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'entry_seen' with expr
            try {
                engine.setVariable(sid, "entry_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='entry_seen'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'errors_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "errors_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'errors_seen' with expr
            try {
                engine.setVariable(sid, "errors_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("(-1)", "-1")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='errors_seen'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'listed_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "listed_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'listed_seen' with expr
            try {
                engine.setVariable(sid, "listed_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='listed_seen'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'stray_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "stray_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'stray_seen' with expr
            try {
                engine.setVariable(sid, "stray_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='stray_seen'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'record_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "record_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'record_seen' with expr
            try {
                engine.setVariable(sid, "record_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"\"", "''")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='record_seen'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'list_errors_seen' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "list_errors_seen", AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'list_errors_seen' with expr
            try {
                engine.setVariable(sid, "list_errors_seen", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("(-1)", "-1")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='list_errors_seen'> expr failed to evaluate")
            }
        }
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
    private fun safeEvaluateGuard(guardExpr: com.sce.runtime.ScriptSource): Boolean =
        evaluateGuardRaising(guardExpr, "a <transition> cond failed to evaluate") ?: false

    // W3C SCXML 5.9.1 + 4.9: an <if> or <elseif> cond, evaluated and reported
    // as a transition guard is. A failure also runs [onFailure]: the <if>
    // still selects on `false`, and is then the element whose processing
    // raised, so its block ends after it.
    @Suppress("unused")
    private inline fun evaluateIfCond(guardExpr: com.sce.runtime.ScriptSource, onFailure: () -> Unit): Boolean {
        val result = evaluateGuardRaising(guardExpr, "an <if> cond failed to evaluate")
        if (result == null) onFailure()
        return result ?: false
    }

    // W3C SCXML 5.9.1: a cond that cannot be evaluated raises error.execution;
    // `null` says so, where a bare `false` could not.
    private fun evaluateGuardRaising(guardExpr: com.sce.runtime.ScriptSource, detail: String): Boolean? {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, detail)
            null
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
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
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
            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "an expression could not be serialised to JSON")
            ""
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    //
    // Both halves carry a language: this engine's Lua arm splices the location
    // in front of `=` and runs the result, so a write target written in
    // ECMAScript has to have been lowered too. Same split as
    // `ScxmlScriptEngine.assign`.
    // Returns whether the assignment took place; on failure error.execution is
    // already raised, and the caller ends its block (W3C SCXML 4.9).
    private fun executeAssign(location: com.sce.runtime.ScriptSource, expr: com.sce.runtime.ScriptSource): Boolean {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        return try {
            engine.assign(sid, location, expr)
            true
        } catch (e: Exception) {
            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.ASSIGNMENT_FAILED, "<assign> failed")
            false
        }
    }

    // W3C SCXML 5.8: Script block execution
    // Returns whether the script ran; on failure error.execution is already
    // raised, and the caller ends its block (W3C SCXML 4.9).
    private fun executeScriptBlock(script: com.sce.runtime.ScriptSource): Boolean {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        return try {
            engine.executeScript(sid, script)
            true
        } catch (e: Exception) {
            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.SCRIPT_FAILED, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AChildTakesAHandedValueInPlaceOfItsDataEvent) {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        val meta = currentEventMetadata
        // §scxml-5.10: the name the event ARRIVED under, which is longer than its
        // member's when a name the document does not write was matched through a
        // prefix (§scxml-3.12.1).
        val eventName = meta.name.ifEmpty { eventNameOf(event) ?: return }
        // W3C SCXML 5.10.1: C++ classifyEventType — platform events override type
        val effectiveType = when {
            eventName.startsWith("done.") || eventName.startsWith("error.") -> "platform"
            else -> meta.type
        }
        // W3C SCXML 5.10.1 + C.1: `_event.origin` and `_event.origintype` are the
        // reply address the processor that delivered the event wrote where it
        // built it, as one value (docs/adr/0090): the SCXML processor's type and
        // the sender's published `_ioprocessors` location for an event a session
        // sent, a host's own pair for what a host named, the Mesh transport's
        // for a peer. Read here as written, with no conversion. An event no
        // processor stamped — an internal or platform event, a host-posted event
        // or reply that names no party, a BasicHTTP request — has none and is
        // handed over as the empty string, which the binding surfaces as blank
        // (undefined, §scxml-B-2-8): nothing is filled in for it, neither this
        // session's own address nor the SCXML processor's type.
        val eventOrigin = meta.origin
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
                origin = eventOrigin?.address ?: "",
                originType = eventOrigin?.processor ?: "",
                invokeId = meta.invokeId
            )
        )
        notePayloadReading(event, payloadReading)
    }



    // W3C SCXML 5.10: bind the event as the `_event` its transitions' guards
    // read — once, before the first guard runs, and not for an eventless
    // selection, which has no event of its own.
    override fun bindCurrentEvent(event: AChildTakesAHandedValueInPlaceOfItsDataEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: AChildTakesAHandedValueInPlaceOfItsDataState, event: AChildTakesAHandedValueInPlaceOfItsDataEvent?): EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>? = when (state) {
        is AChildTakesAHandedValueInPlaceOfItsDataState.ByNamelist -> firstEnabledTransitionOfByNamelist(state, event)
        is AChildTakesAHandedValueInPlaceOfItsDataState.ByParam -> firstEnabledTransitionOfByParam(state, event)
        is AChildTakesAHandedValueInPlaceOfItsDataState.FirstMacrostep -> firstEnabledTransitionOfFirstMacrostep(state, event)
        is AChildTakesAHandedValueInPlaceOfItsDataState.JudgeNames -> firstEnabledTransitionOfJudgeNames(state, event)
        is AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams -> firstEnabledTransitionOfJudgeParams(state, event)
        is AChildTakesAHandedValueInPlaceOfItsDataState.LateBound -> firstEnabledTransitionOfLateBound(state, event)
        else -> null
    }


    // byNamelist's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfByNamelist(state: AChildTakesAHandedValueInPlaceOfItsDataState, event: AChildTakesAHandedValueInPlaceOfItsDataEvent?): EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>? = when {
        event is AChildTakesAHandedValueInPlaceOfItsDataEvent.Names.Report -> TransitionsPart0.transitionByNamelistAt0
        event is AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution -> TransitionsPart0.transitionByNamelistAt1
        else -> null
    }


    // byParam's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfByParam(state: AChildTakesAHandedValueInPlaceOfItsDataState, event: AChildTakesAHandedValueInPlaceOfItsDataEvent?): EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>? = when {
        event is AChildTakesAHandedValueInPlaceOfItsDataEvent.Params.Report -> TransitionsPart0.transitionByParamAt0
        event is AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution -> TransitionsPart0.transitionByParamAt1
        else -> null
    }


    // firstMacrostep's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfFirstMacrostep(state: AChildTakesAHandedValueInPlaceOfItsDataState, event: AChildTakesAHandedValueInPlaceOfItsDataEvent?): EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>? = when {
        event is AChildTakesAHandedValueInPlaceOfItsDataEvent.Done.Invoke.FirstMacrostep -> TransitionsPart0.transitionFirstMacrostepAt0
        event is AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution -> TransitionsPart0.transitionFirstMacrostepAt1
        else -> null
    }


    // judgeNames's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfJudgeNames(state: AChildTakesAHandedValueInPlaceOfItsDataState, event: AChildTakesAHandedValueInPlaceOfItsDataEvent?): EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(list_errors_seen ~= 0)", "list_errors_seen !== 0")) -> TransitionsPart0.transitionJudgeNamesAt0
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(listed_seen ~= \"handed listed\")", "listed_seen !== 'handed listed'")) -> TransitionsPart0.transitionJudgeNamesAt1
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(stray_seen ~= \"undefined\")", "stray_seen !== 'undefined'")) -> TransitionsPart0.transitionJudgeNamesAt2
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(record_seen ~= \"whole\")", "record_seen !== 'whole'")) -> TransitionsPart0.transitionJudgeNamesAt3
        event == null -> TransitionsPart0.transitionJudgeNamesAt4
        else -> null
    }


    // judgeParams's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfJudgeParams(state: AChildTakesAHandedValueInPlaceOfItsDataState, event: AChildTakesAHandedValueInPlaceOfItsDataEvent?): EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(effects_seen ~= 0)", "effects_seen !== 0")) -> TransitionsPart0.transitionJudgeParamsAt0
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("((raising_seen ~= \"handed raising\") or (effectful_seen ~= \"handed effectful\"))", "raising_seen !== 'handed raising' || effectful_seen !== 'handed effectful'")) -> TransitionsPart0.transitionJudgeParamsAt1
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(kept_seen ~= \"own kept\")", "kept_seen !== 'own kept'")) -> TransitionsPart0.transitionJudgeParamsAt2
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(stranger_seen ~= \"undefined\")", "stranger_seen !== 'undefined'")) -> TransitionsPart0.transitionJudgeParamsAt3
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(payload_seen ~= \"whole\")", "payload_seen !== 'whole'")) -> TransitionsPart0.transitionJudgeParamsAt4
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(record.name ~= \"whole\")", "record.name !== 'whole'")) -> TransitionsPart0.transitionJudgeParamsAt5
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(script_seen ~= \"handed raising|handed effectful|whole\")", "script_seen !== 'handed raising|handed effectful|whole'")) -> TransitionsPart0.transitionJudgeParamsAt6
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(entry_seen ~= \"handed raising|handed effectful|inner\")", "entry_seen !== 'handed raising|handed effectful|inner'")) -> TransitionsPart0.transitionJudgeParamsAt7
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(errors_seen ~= 0)", "errors_seen !== 0")) -> TransitionsPart0.transitionJudgeParamsAt8
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(refusable_seen ~= \"handed refusable\")", "refusable_seen !== 'handed refusable'")) -> TransitionsPart0.transitionJudgeParamsAt9
        event == null -> TransitionsPart0.transitionJudgeParamsAt10
        else -> null
    }


    // lateBound's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfLateBound(state: AChildTakesAHandedValueInPlaceOfItsDataState, event: AChildTakesAHandedValueInPlaceOfItsDataEvent?): EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataState, HistoryId>? = when {
        event is AChildTakesAHandedValueInPlaceOfItsDataEvent.Late.Ok -> TransitionsPart0.transitionLateBoundAt0
        event is AChildTakesAHandedValueInPlaceOfItsDataEvent.Late.Kept -> TransitionsPart0.transitionLateBoundAt1
        event is AChildTakesAHandedValueInPlaceOfItsDataEvent.Late.Lost -> TransitionsPart0.transitionLateBoundAt2
        event is AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution -> TransitionsPart0.transitionLateBoundAt3
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:69 :: _machine
    override fun onEntry(state: AChildTakesAHandedValueInPlaceOfItsDataState, isDefaultEntry: Boolean) {
        when (state) {
            is AChildTakesAHandedValueInPlaceOfItsDataState.ByNamelist -> onEntryOfByNamelist()
            is AChildTakesAHandedValueInPlaceOfItsDataState.ByParam -> onEntryOfByParam()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailChildRaised -> onEntryOfFailChildRaised()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailChildSharedTheParentsObject -> onEntryOfFailChildSharedTheParentsObject()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailEntryReadItsOwnValue -> onEntryOfFailEntryReadItsOwnValue()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailHandedValueLost -> onEntryOfFailHandedValueLost()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailLateStateDataTookTheValue -> onEntryOfFailLateStateDataTookTheValue()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistNameAdded -> onEntryOfFailNamelistNameAdded()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistOwnValueRaised -> onEntryOfFailNamelistOwnValueRaised()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistValueLost -> onEntryOfFailNamelistValueLost()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailObjectNotWhole -> onEntryOfFailObjectNotWhole()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailOwnValueRan -> onEntryOfFailOwnValueRan()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailParentRaised -> onEntryOfFailParentRaised()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailScriptReadItsOwnValue -> onEntryOfFailScriptReadItsOwnValue()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailStateDataTookTheValue -> onEntryOfFailStateDataTookTheValue()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailUnmatchedNameAdded -> onEntryOfFailUnmatchedNameAdded()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FirstMacrostep -> onEntryOfFirstMacrostep()
            is AChildTakesAHandedValueInPlaceOfItsDataState.JudgeNames -> onEntryOfJudgeNames()
            is AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams -> onEntryOfJudgeParams()
            is AChildTakesAHandedValueInPlaceOfItsDataState.LateBound -> onEntryOfLateBound()
            is AChildTakesAHandedValueInPlaceOfItsDataState.Pass -> onEntryOfPass()
        }
    }


    private fun onEntryOfByNamelist() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:181 :: byNamelist :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("by_namelist", AChildTakesAHandedValueInPlaceOfItsDataState.ByNamelist)
    }

    private fun onEntryOfByParam() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:92 :: byParam :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("by_param", AChildTakesAHandedValueInPlaceOfItsDataState.ByParam)
    }

    private fun onEntryOfFailChildRaised() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:296 :: failChildRaised :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailChildSharedTheParentsObject() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:293 :: failChildSharedTheParentsObject :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailEntryReadItsOwnValue() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:295 :: failEntryReadItsOwnValue :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailHandedValueLost() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:289 :: failHandedValueLost :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailLateStateDataTookTheValue() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:300 :: failLateStateDataTookTheValue :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailNamelistNameAdded() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:299 :: failNamelistNameAdded :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailNamelistOwnValueRaised() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:297 :: failNamelistOwnValueRaised :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailNamelistValueLost() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:298 :: failNamelistValueLost :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailObjectNotWhole() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:292 :: failObjectNotWhole :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailOwnValueRan() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:288 :: failOwnValueRan :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailParentRaised() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:301 :: failParentRaised :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailScriptReadItsOwnValue() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:294 :: failScriptReadItsOwnValue :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailStateDataTookTheValue() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:290 :: failStateDataTookTheValue :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailUnmatchedNameAdded() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:291 :: failUnmatchedNameAdded :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFirstMacrostep() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:265 :: firstMacrostep :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("first_macrostep", AChildTakesAHandedValueInPlaceOfItsDataState.FirstMacrostep)
    }

    private fun onEntryOfJudgeNames() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:220 :: judgeNames :: _state_body
    }

    private fun onEntryOfJudgeParams() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:164 :: judgeParams :: _state_body
    }

    private fun onEntryOfLateBound() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:228 :: lateBound :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("late_bound", AChildTakesAHandedValueInPlaceOfItsDataState.LateBound)
    }

    private fun onEntryOfPass() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:287 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
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
    private fun deferStaticInvoke(invokeId: String, state: AChildTakesAHandedValueInPlaceOfItsDataState) {
        when (invokeId) {
            "by_namelist" -> startInvokeOfByNamelistAt0(state)
            "by_param" -> startInvokeOfByParamAt0(state)
            "first_macrostep" -> startInvokeOfFirstMacrostepAt0(state)
            "late_bound" -> startInvokeOfLateBoundAt0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfByNamelistAt0(state: AChildTakesAHandedValueInPlaceOfItsDataState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "byNamelist.${nextInvokeStart("by_namelist")}.by_namelist"
                deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        assertScriptSession()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        if (!(engineInv.hasVariable(sidInv, "listed") && engineInv.hasVariable(sidInv, "stray") && engineInv.hasVariable(sidInv, "record"))) {
                            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<invoke> namelist names a location that cannot be read")
                            return@deferInvoke
                        }
                        invokeParams["listed"] = engineInv.getVariable(sidInv, "listed")
                        invokeParams["stray"] = engineInv.getVariable(sidInv, "stray")
                        invokeParams["record"] = engineInv.getVariable(sidInv, "record")

                    val childSM = adoptChild(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)")), generatedInvokeId)
                    setInvokeParams(childSM, invokeParams)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("by_namelist", childSM, false, AChildTakesAHandedValueInPlaceOfItsDataEvent.Done.Invoke.Self, "", generatedInvokeId, "", "a_child_takes_a_handed_value_in_place_of_its_data__sce_synth_invoke__by_namelist", "by_namelist")
                }
            }
    }

    private fun startInvokeOfByParamAt0(state: AChildTakesAHandedValueInPlaceOfItsDataState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "byParam.${nextInvokeStart("by_param")}.by_param"
                deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        assertScriptSession()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["raising"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("\"handed raising\"", "'handed raising'"))
                        } catch (_: Exception) {
                            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<invoke> <param name='raising'> could not be read")
                        }
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["effectful"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("\"handed effectful\"", "'handed effectful'"))
                        } catch (_: Exception) {
                            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<invoke> <param name='effectful'> could not be read")
                        }
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["refusable"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("\"handed refusable\"", "'handed refusable'"))
                        } catch (_: Exception) {
                            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<invoke> <param name='refusable'> could not be read")
                        }
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["kept"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("\"handed kept\"", "'handed kept'"))
                        } catch (_: Exception) {
                            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<invoke> <param name='kept'> could not be read")
                        }
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["stranger"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("\"handed stranger\"", "'handed stranger'"))
                        } catch (_: Exception) {
                            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<invoke> <param name='stranger'> could not be read")
                        }
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["payload"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("record", "record"))
                        } catch (_: Exception) {
                            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<invoke> <param name='payload'> could not be read")
                        }

                    val childSM = adoptChild(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByParamStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)")), generatedInvokeId)
                    setInvokeParams(childSM, invokeParams)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("by_param", childSM, false, AChildTakesAHandedValueInPlaceOfItsDataEvent.Done.Invoke.Self, "", generatedInvokeId, "", "a_child_takes_a_handed_value_in_place_of_its_data__sce_synth_invoke__by_param", "by_param")
                }
            }
    }

    private fun startInvokeOfFirstMacrostepAt0(state: AChildTakesAHandedValueInPlaceOfItsDataState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "firstMacrostep.${nextInvokeStart("first_macrostep")}.first_macrostep"
                deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        assertScriptSession()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["handed"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("\"handed first\"", "'handed first'"))
                        } catch (_: Exception) {
                            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<invoke> <param name='handed'> could not be read")
                        }

                    val childSM = adoptChild(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeFirstMacrostepStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)")), generatedInvokeId)
                    setInvokeParams(childSM, invokeParams)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("first_macrostep", childSM, false, AChildTakesAHandedValueInPlaceOfItsDataEvent.Done.Invoke.FirstMacrostep, "", generatedInvokeId, "", "a_child_takes_a_handed_value_in_place_of_its_data__sce_synth_invoke__first_macrostep", "first_macrostep")
                }
            }
    }

    private fun startInvokeOfLateBoundAt0(state: AChildTakesAHandedValueInPlaceOfItsDataState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "lateBound.${nextInvokeStart("late_bound")}.late_bound"
                deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        assertScriptSession()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["handed"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("\"handed late\"", "'handed late'"))
                        } catch (_: Exception) {
                            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<invoke> <param name='handed'> could not be read")
                        }
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["kept"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("\"handed kept\"", "'handed kept'"))
                        } catch (_: Exception) {
                            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<invoke> <param name='kept'> could not be read")
                        }

                    val childSM = adoptChild(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeLateBoundStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)")), generatedInvokeId)
                    setInvokeParams(childSM, invokeParams)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("late_bound", childSM, false, AChildTakesAHandedValueInPlaceOfItsDataEvent.Done.Invoke.Self, "", generatedInvokeId, "", "a_child_takes_a_handed_value_in_place_of_its_data__sce_synth_invoke__late_bound", "late_bound")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:69 :: _machine
    override fun onExit(state: AChildTakesAHandedValueInPlaceOfItsDataState) {
        when (state) {
            is AChildTakesAHandedValueInPlaceOfItsDataState.ByNamelist -> onExitOfByNamelist()
            is AChildTakesAHandedValueInPlaceOfItsDataState.ByParam -> onExitOfByParam()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailChildRaised -> onExitOfFailChildRaised()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailChildSharedTheParentsObject -> onExitOfFailChildSharedTheParentsObject()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailEntryReadItsOwnValue -> onExitOfFailEntryReadItsOwnValue()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailHandedValueLost -> onExitOfFailHandedValueLost()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailLateStateDataTookTheValue -> onExitOfFailLateStateDataTookTheValue()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistNameAdded -> onExitOfFailNamelistNameAdded()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistOwnValueRaised -> onExitOfFailNamelistOwnValueRaised()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailNamelistValueLost -> onExitOfFailNamelistValueLost()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailObjectNotWhole -> onExitOfFailObjectNotWhole()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailOwnValueRan -> onExitOfFailOwnValueRan()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailParentRaised -> onExitOfFailParentRaised()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailScriptReadItsOwnValue -> onExitOfFailScriptReadItsOwnValue()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailStateDataTookTheValue -> onExitOfFailStateDataTookTheValue()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FailUnmatchedNameAdded -> onExitOfFailUnmatchedNameAdded()
            is AChildTakesAHandedValueInPlaceOfItsDataState.FirstMacrostep -> onExitOfFirstMacrostep()
            is AChildTakesAHandedValueInPlaceOfItsDataState.JudgeNames -> onExitOfJudgeNames()
            is AChildTakesAHandedValueInPlaceOfItsDataState.JudgeParams -> onExitOfJudgeParams()
            is AChildTakesAHandedValueInPlaceOfItsDataState.LateBound -> onExitOfLateBound()
            is AChildTakesAHandedValueInPlaceOfItsDataState.Pass -> onExitOfPass()
        }
    }


    private fun onExitOfByNamelist() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:181 :: byNamelist :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(AChildTakesAHandedValueInPlaceOfItsDataState.ByNamelist)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("by_namelist")
    }

    private fun onExitOfByParam() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:92 :: byParam :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(AChildTakesAHandedValueInPlaceOfItsDataState.ByParam)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("by_param")
    }

    private fun onExitOfFailChildRaised() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:296 :: failChildRaised :: _state_body
    }

    private fun onExitOfFailChildSharedTheParentsObject() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:293 :: failChildSharedTheParentsObject :: _state_body
    }

    private fun onExitOfFailEntryReadItsOwnValue() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:295 :: failEntryReadItsOwnValue :: _state_body
    }

    private fun onExitOfFailHandedValueLost() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:289 :: failHandedValueLost :: _state_body
    }

    private fun onExitOfFailLateStateDataTookTheValue() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:300 :: failLateStateDataTookTheValue :: _state_body
    }

    private fun onExitOfFailNamelistNameAdded() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:299 :: failNamelistNameAdded :: _state_body
    }

    private fun onExitOfFailNamelistOwnValueRaised() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:297 :: failNamelistOwnValueRaised :: _state_body
    }

    private fun onExitOfFailNamelistValueLost() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:298 :: failNamelistValueLost :: _state_body
    }

    private fun onExitOfFailObjectNotWhole() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:292 :: failObjectNotWhole :: _state_body
    }

    private fun onExitOfFailOwnValueRan() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:288 :: failOwnValueRan :: _state_body
    }

    private fun onExitOfFailParentRaised() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:301 :: failParentRaised :: _state_body
    }

    private fun onExitOfFailScriptReadItsOwnValue() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:294 :: failScriptReadItsOwnValue :: _state_body
    }

    private fun onExitOfFailStateDataTookTheValue() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:290 :: failStateDataTookTheValue :: _state_body
    }

    private fun onExitOfFailUnmatchedNameAdded() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:291 :: failUnmatchedNameAdded :: _state_body
    }

    private fun onExitOfFirstMacrostep() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:265 :: firstMacrostep :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(AChildTakesAHandedValueInPlaceOfItsDataState.FirstMacrostep)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("first_macrostep")
    }

    private fun onExitOfJudgeNames() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:220 :: judgeNames :: _state_body
    }

    private fun onExitOfJudgeParams() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:164 :: judgeParams :: _state_body
    }

    private fun onExitOfLateBound() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:228 :: lateBound :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(AChildTakesAHandedValueInPlaceOfItsDataState.LateBound)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("late_bound")
    }

    private fun onExitOfPass() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:287 :: pass :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:69 :: _machine
    override fun executeTransitionContent(source: AChildTakesAHandedValueInPlaceOfItsDataState, transitionIndex: Int) {
        when (source) {
            is AChildTakesAHandedValueInPlaceOfItsDataState.ByNamelist -> transitionContentOfByNamelist(transitionIndex)
            is AChildTakesAHandedValueInPlaceOfItsDataState.ByParam -> transitionContentOfByParam(transitionIndex)
            else -> {}
        }
    }


    // The content of `byNamelist`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfByNamelist(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfByNamelistAt0()
            else -> {}
        }
    }


    private fun transitionContentOfByNamelistAt0() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:211 :: byNamelist :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("listed_seen", "listed_seen"), com.sce.runtime.ScriptSource.lua("_event.data.listed", "_event.data.listed"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("stray_seen", "stray_seen"), com.sce.runtime.ScriptSource.lua("_event.data.stray", "_event.data.stray"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("record_seen", "record_seen"), com.sce.runtime.ScriptSource.lua("_event.data.record", "_event.data.record"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("list_errors_seen", "list_errors_seen"), com.sce.runtime.ScriptSource.lua("_event.data.errors", "_event.data.errors"))) {
                return
            }
    }

    // The content of `byParam`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfByParam(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfByParamAt0()
            else -> {}
        }
    }


    private fun transitionContentOfByParamAt0() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:145 :: byParam :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("raising_seen", "raising_seen"), com.sce.runtime.ScriptSource.lua("_event.data.raising", "_event.data.raising"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("effectful_seen", "effectful_seen"), com.sce.runtime.ScriptSource.lua("_event.data.effectful", "_event.data.effectful"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("effects_seen", "effects_seen"), com.sce.runtime.ScriptSource.lua("_event.data.effects", "_event.data.effects"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("refusable_seen", "refusable_seen"), com.sce.runtime.ScriptSource.lua("_event.data.refusable", "_event.data.refusable"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("kept_seen", "kept_seen"), com.sce.runtime.ScriptSource.lua("_event.data.kept", "_event.data.kept"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("stranger_seen", "stranger_seen"), com.sce.runtime.ScriptSource.lua("_event.data.stranger", "_event.data.stranger"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("payload_seen", "payload_seen"), com.sce.runtime.ScriptSource.lua("_event.data.payload", "_event.data.payload"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("script_seen", "script_seen"), com.sce.runtime.ScriptSource.lua("_event.data.script", "_event.data.script"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("entry_seen", "entry_seen"), com.sce.runtime.ScriptSource.lua("_event.data.entry", "_event.data.entry"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("errors_seen", "errors_seen"), com.sce.runtime.ScriptSource.lua("_event.data.errors", "_event.data.errors"))) {
                return
            }
    }
}
