// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_donedata.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_donedata.scxml:16 :: _machine

package com.sce.integration.static_donedata

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticDonedataState : State {
    data object Counting : StaticDonedataState
    data object Done : StaticDonedataState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticDonedataEvent : Event {
    sealed interface Error : StaticDonedataEvent {
        data object Execution : Error
    }
    data object Finish : StaticDonedataEvent
    data object Tick : StaticDonedataEvent
    data object Widen : StaticDonedataEvent
}
// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:ViewMode` datamodel value. */
enum class StaticDonedataViewModeEnum(val declaredName: String) {
    MONTH("month"),
    WEEK("week"),
    DAY("day"),
    AGENDA_LIST("agenda_list");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        MONTH -> 0u.toUByte()
        WEEK -> 1u.toUByte()
        DAY -> 2u.toUByte()
        AGENDA_LIST -> 3u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticDonedataViewModeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of ViewMode")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticDonedataViewModeEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
// --- State Machine (W3C SCXML) ---

class StaticDonedataStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticDonedataState, StaticDonedataEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `count` datamodel variable, published (`sce:direction="out"`). */
    private var vCount: UInt = 0.toUInt()

    /** What the published `count` holds. */
    val count: UInt get() = vCount
    /** W3C SCXML 5.2: the `small` datamodel variable, the machine's own. */
    private var vSmall: UByte = 250.toUByte()
    /** W3C SCXML 5.2: the `label` datamodel variable, the machine's own. */
    private var vLabel: String = "tally"
    /** W3C SCXML 5.2: the `layout` datamodel variable, the machine's own. */
    private var vLayout: StaticDonedataViewModeEnum = StaticDonedataViewModeEnum.MONTH

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var count: UInt? = null
        var small: UByte? = null
        var label: String? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.count?.let { vCount = it }
        params.small?.let { vSmall = it }
        params.label?.let { vLabel = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val count: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticDonedataState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        count = vCount,
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
        first = "0d0759aeddae60c9eb915b9c754c292935f6f65a9f8c69a23d950ddd6d8adc6d",
        revised = "8dd92a183235e2d9c1a44309a93537d8e17e06426fd3354f2621ba63e309c2d1",
        format2First = "35e6bdb9f8bdfe06bc168111a18d43678c65e4ae45b4e7780d761df46ff34de2",
        format2Revised = "aa75f4a4d744b247fc26347060f6bca356dfffde8e6be6b3b4d57799f7483b93",
        records = listOf(
        ),
        enums = listOf(
            EnumLineage(
                key = "enum:enum_view_mode.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("layout", null, false),
                ),
            ),
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
            "count" to SavedValues.of(vCount),
            "small" to SavedValues.of(vSmall),
            "label" to SavedValues.of(vLabel),
            "layout" to vLayout.toSaved(),
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
        val saved1 = SavedValues.uint32(savedGrown.variable("count"), "count")
        val saved2 = SavedValues.uint8(savedGrown.variable("small"), "small")
        val saved3 = SavedValues.string(savedGrown.variable("label"), "label", 16)
        val saved4 = StaticDonedataViewModeEnum.fromSaved(savedGrown.variable("layout"), "layout")
        vCount = saved1
        vSmall = saved2
        vLabel = saved3
        vLayout = saved4
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
        SavedValues.uint32(savedGrown.variable("count"), "count")
        SavedValues.uint8(savedGrown.variable("small"), "small")
        SavedValues.string(savedGrown.variable("label"), "label", 16)
        StaticDonedataViewModeEnum.fromSaved(savedGrown.variable("layout"), "layout")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    override val initialState: StaticDonedataState = StaticDonedataState.Counting

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
    override fun isFinalState(state: StaticDonedataState): Boolean = when (state) {
        is StaticDonedataState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticDonedataState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticDonedataState, HistoryId>> =
            listOf(StateTarget(StaticDonedataState.Counting))

        // W3C SCXML 3.13: counting's transition 0, as the microstep reads it.
        val transitionCountingAt0 = EnabledTransition<StaticDonedataState, HistoryId>(
            StaticDonedataState.Counting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: counting's transition 1, as the microstep reads it.
        val transitionCountingAt1 = EnabledTransition<StaticDonedataState, HistoryId>(
            StaticDonedataState.Counting,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: counting's transition 2, as the microstep reads it.
        val transitionCountingAt2 = EnabledTransition<StaticDonedataState, HistoryId>(
            StaticDonedataState.Counting,
            listOf(StateTarget(StaticDonedataState.Done)),
            2,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticDonedataState? = when (stateId) {
        "counting" -> StaticDonedataState.Counting
        "done" -> StaticDonedataState.Done
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticDonedataState): String = when (state) {
        is StaticDonedataState.Counting -> "counting"
        is StaticDonedataState.Done -> "done"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticDonedataState): Int = when (state) {
        is StaticDonedataState.Counting -> 0
        is StaticDonedataState.Done -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticDonedataEvent? = when (name) {
        "error.execution" -> StaticDonedataEvent.Error.Execution
        "finish" -> StaticDonedataEvent.Finish
        "tick" -> StaticDonedataEvent.Tick
        "widen" -> StaticDonedataEvent.Widen
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticDonedataEvent): String? = when (event) {
        is StaticDonedataEvent.Error.Execution -> "error.execution"
        is StaticDonedataEvent.Finish -> "finish"
        is StaticDonedataEvent.Tick -> "tick"
        is StaticDonedataEvent.Widen -> "widen"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticDonedataState,
        event: StaticDonedataEvent?
    ): EnabledTransition<StaticDonedataState, HistoryId>? = when (state) {
        is StaticDonedataState.Counting -> when {
            event is StaticDonedataEvent.Tick -> transitionCountingAt0
            event is StaticDonedataEvent.Widen -> transitionCountingAt1
            event is StaticDonedataEvent.Finish -> transitionCountingAt2
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_donedata.scxml:16 :: _machine
    override fun onEntry(state: StaticDonedataState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticDonedataState.Counting -> {
                // SCE-MAP: static_donedata.scxml:25 :: counting :: _state_body
            }
            is StaticDonedataState.Done -> {
                // SCE-MAP: static_donedata.scxml:34 :: done :: _state_body
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    var doneEventData = ""
                    // W3C SCXML 5.5: Evaluate <param> elements (C++ DoneDataHelper::evaluateParams pattern)
                    val doneParams = mutableMapOf<String, Any?>()
                    try {
                        doneParams["total"] = (com.sce.forge.runtime.SceChecked.mul(vCount, 2.toUInt())).toLong()
                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                        raisePlatformError(StaticDonedataEvent.Error.Execution, "<donedata> <param name='total'> failed to evaluate")
                    }
                    doneParams["many"] = vCount > 2.toUInt()
                    doneParams["name"] = vLabel
                    doneParams["layout"] = (vLayout).declaredName
                    try {
                        doneParams["overflow"] = (com.sce.forge.runtime.SceChecked.add(vSmall, vSmall)).toLong()
                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                        raisePlatformError(StaticDonedataEvent.Error.Execution, "<donedata> <param name='overflow'> failed to evaluate")
                    }
                    // §scxml-5.5: the pairs that survived, `{}` when none did
                    // (C++ DoneDataHelper::evaluateParams). Not left to
                    // buildJsonFromParams, whose empty answer is a <send>'s.
                    doneEventData = if (doneParams.isEmpty()) "{}" else buildJsonFromParams(doneParams)
                    // W3C SCXML 5.5 + 6.3.1: stash onto the engine so the invoking parent's
                    // startInvoke completion callback can lift the payload onto
                    // done.invoke.<id>._event.data. Mirrors C++ AOT stashDonedataAtFinal.
                    stashDonedataAtFinal(doneEventData)
                }
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_donedata.scxml:16 :: _machine
    override fun onExit(state: StaticDonedataState) {
        when (state) {
            is StaticDonedataState.Counting -> {
                // SCE-MAP: static_donedata.scxml:25 :: counting :: _state_body
            }
            is StaticDonedataState.Done -> {
                // SCE-MAP: static_donedata.scxml:34 :: done :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_donedata.scxml:16 :: _machine
    override fun executeTransitionContent(source: StaticDonedataState, transitionIndex: Int) {
        when (source) {
        is StaticDonedataState.Counting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_donedata.scxml:26 :: counting :: _transition_0

            if (try { vCount = com.sce.forge.runtime.SceChecked.add(vCount, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticDonedataEvent.Error.Execution, "<assign location='count'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_donedata.scxml:29 :: counting :: _transition_1

            vLayout = StaticDonedataViewModeEnum.AGENDA_LIST
            }
            else -> {}
        }
        else -> {}
        }
    }
}
