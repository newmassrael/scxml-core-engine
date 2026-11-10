// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_render.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_render.scxml:14 :: _machine

package com.sce.integration.static_render

import com.sce.runtime.*
import com.sce.generated.text_notice.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticRenderState : State {
    data object Idle : StaticRenderState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticRenderEvent : Event {
    sealed interface Error : StaticRenderEvent {
        data object Execution : Error
    }
    data object Longest : StaticRenderEvent
    data object Markers : StaticRenderEvent
    data object Overflow : StaticRenderEvent
    data object Rebuilt : StaticRenderEvent
    data object Render : StaticRenderEvent
    data object Wide : StaticRenderEvent
}
// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:ViewMode` datamodel value. */
enum class StaticRenderViewModeEnum(val declaredName: String) {
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
        fun fromSaved(value: Any?, what: String): StaticRenderViewModeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of ViewMode")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticRenderViewModeEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
// --- State Machine (W3C SCXML) ---

class StaticRenderStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticRenderState, StaticRenderEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `note` datamodel variable, published (`sce:direction="out"`). */
    private var vNote: String = ""

    /** What the published `note` holds. */
    val note: String get() = vNote
    /** W3C SCXML 5.2: the `who` datamodel variable, published (`sce:direction="out"`). */
    private var vWho: String = "ann"

    /** What the published `who` holds. */
    val who: String get() = vWho
    /** W3C SCXML 5.2: the `count` datamodel variable, published (`sce:direction="out"`). */
    private var vCount: Short = 0

    /** What the published `count` holds. */
    val count: Short get() = vCount
    /** W3C SCXML 5.2: the `view` datamodel variable, published (`sce:direction="out"`). */
    private var vView: StaticRenderViewModeEnum = StaticRenderViewModeEnum.MONTH

    /** What the published `view` holds. */
    val view: StaticRenderViewModeEnum get() = vView
    /** W3C SCXML 5.2: the `said` datamodel variable, published (`sce:direction="out"`). */
    private var vSaid: String = "hi"

    /** What the published `said` holds. */
    val said: String get() = vSaid
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
        var note: String? = null
        var who: String? = null
        var count: Short? = null
        var said: String? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.note?.let { vNote = it }
        params.who?.let { vWho = it }
        params.count?.let { vCount = it }
        params.said?.let { vSaid = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val note: String,
        val who: String,
        val count: Short,
        val view: StaticRenderViewModeEnum,
        val said: String,
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
        val configuration: Set<StaticRenderState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        note = vNote,
        who = vWho,
        count = vCount,
        view = vView,
        said = vSaid,
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
        first = "f69330de1791fac4fea5443a4edb47d0b9a30c16b89d6dbcba340f782e4e6aa2",
        revised = "3b0435a93c7b5401f210480d9b9ab31575bf6b795c919383efe5380c8760991b",
        format2First = "522b69050aa237fcced921551aef5849aa167a4155596f8d416507e26d97f826",
        format2Revised = "fb1a6efe0b66163a599c2f55f653c13992b856df2aef90c049830b22bbc184d1",
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
                    EnumHolder("view", null, false),
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
    fun save(wallNowMs: Long): SavedState {
        val variables = LinkedHashMap<String, Any?>()
        saveVariablesPart0(variables)
        return savedState(savedShape, variables, wallNowMs)
    }

    private fun saveVariablesPart0(variables: MutableMap<String, Any?>) {
        variables += "note" to SavedValues.of(vNote)
        variables += "who" to SavedValues.of(vWho)
        variables += "count" to SavedValues.of(vCount)
        variables += "view" to vView.toSaved()
        variables += "said" to SavedValues.of(vSaid)
        variables += "refusals" to SavedValues.of(vRefusals)
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
        val saved1 = SavedValues.string(savedGrown.variable("note"), "note", 95)
        val saved2 = SavedValues.string(savedGrown.variable("who"), "who", 8)
        val saved3 = SavedValues.int16(savedGrown.variable("count"), "count")
        val saved4 = StaticRenderViewModeEnum.fromSaved(savedGrown.variable("view"), "view")
        val saved5 = SavedValues.string(savedGrown.variable("said"), "said", 16)
        val saved6 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        return {
            vNote = saved1
            vWho = saved2
            vCount = saved3
            vView = saved4
            vSaid = saved5
            vRefusals = saved6
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
        SavedValues.string(savedGrown.variable("note"), "note", 95)
        SavedValues.string(savedGrown.variable("who"), "who", 8)
        SavedValues.int16(savedGrown.variable("count"), "count")
        StaticRenderViewModeEnum.fromSaved(savedGrown.variable("view"), "view")
        SavedValues.string(savedGrown.variable("said"), "said", 16)
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    override val initialState: StaticRenderState = StaticRenderState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticRenderState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticRenderState, HistoryId>> =
            listOf(StateTarget(StaticRenderState.Idle))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticRenderState, HistoryId>(
            StaticRenderState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticRenderState, HistoryId>(
            StaticRenderState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticRenderState, HistoryId>(
            StaticRenderState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticRenderState, HistoryId>(
            StaticRenderState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticRenderState, HistoryId>(
            StaticRenderState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<StaticRenderState, HistoryId>(
            StaticRenderState.Idle,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 6, as the microstep reads it.
        val transitionIdleAt6 = EnabledTransition<StaticRenderState, HistoryId>(
            StaticRenderState.Idle,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticRenderState? = when (stateId) {
        "idle" -> StaticRenderState.Idle
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticRenderState): String = when (state) {
        is StaticRenderState.Idle -> "idle"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticRenderState): Int = when (state) {
        is StaticRenderState.Idle -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticRenderEvent? = when (name) {
        "error.execution" -> StaticRenderEvent.Error.Execution
        "longest" -> StaticRenderEvent.Longest
        "markers" -> StaticRenderEvent.Markers
        "overflow" -> StaticRenderEvent.Overflow
        "rebuilt" -> StaticRenderEvent.Rebuilt
        "render" -> StaticRenderEvent.Render
        "wide" -> StaticRenderEvent.Wide
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticRenderEvent): String? = when (event) {
        is StaticRenderEvent.Error.Execution -> "error.execution"
        is StaticRenderEvent.Longest -> "longest"
        is StaticRenderEvent.Markers -> "markers"
        is StaticRenderEvent.Overflow -> "overflow"
        is StaticRenderEvent.Rebuilt -> "rebuilt"
        is StaticRenderEvent.Render -> "render"
        is StaticRenderEvent.Wide -> "wide"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticRenderState, event: StaticRenderEvent?): EnabledTransition<StaticRenderState, HistoryId>? = when (state) {
        is StaticRenderState.Idle -> firstEnabledTransitionOfIdle(state, event)
    }


    // idle's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdle(state: StaticRenderState, event: StaticRenderEvent?): EnabledTransition<StaticRenderState, HistoryId>? = when {
        event is StaticRenderEvent.Render -> TransitionsPart0.transitionIdleAt0
        event is StaticRenderEvent.Longest -> TransitionsPart0.transitionIdleAt1
        event is StaticRenderEvent.Markers -> TransitionsPart0.transitionIdleAt2
        event is StaticRenderEvent.Rebuilt -> TransitionsPart0.transitionIdleAt3
        event is StaticRenderEvent.Wide -> TransitionsPart0.transitionIdleAt4
        event is StaticRenderEvent.Overflow -> TransitionsPart0.transitionIdleAt5
        event is StaticRenderEvent.Error.Execution -> TransitionsPart0.transitionIdleAt6
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_render.scxml:14 :: _machine
    override fun onEntry(state: StaticRenderState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticRenderState.Idle -> onEntryOfIdle()
        }
    }


    private fun onEntryOfIdle() {
                // SCE-MAP: static_render.scxml:26 :: idle :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_render.scxml:14 :: _machine
    override fun onExit(state: StaticRenderState) {
        when (state) {
            is StaticRenderState.Idle -> onExitOfIdle()
        }
    }


    private fun onExitOfIdle() {
                // SCE-MAP: static_render.scxml:26 :: idle :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_render.scxml:14 :: _machine
    override fun executeTransitionContent(source: StaticRenderState, transitionIndex: Int) {
        when (source) {
            is StaticRenderState.Idle -> transitionContentOfIdle(transitionIndex)
        }
    }


    // The content of `idle`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfIdle(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfIdleAt0()
            1 -> transitionContentOfIdleAt1()
            2 -> transitionContentOfIdleAt2()
            3 -> transitionContentOfIdleAt3()
            4 -> transitionContentOfIdleAt4()
            5 -> transitionContentOfIdleAt5()
            6 -> transitionContentOfIdleAt6()
            else -> {}
        }
    }


    private fun transitionContentOfIdleAt0() {
                // SCE-MAP: static_render.scxml:27 :: idle :: _transition_0

            if (try { vNote = com.sce.forge.runtime.SceChecked.bounded(TextNotice.render(vWho, vCount, (vView).declaredName, vSaid), 95); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRenderEvent.Error.Execution, "<sce:render template='Notice'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt1() {
                // SCE-MAP: static_render.scxml:36 :: idle :: _transition_1

            if (try { vWho = com.sce.forge.runtime.SceChecked.bounded("abcdefgh", 8); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRenderEvent.Error.Execution, "<assign location='who'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vCount = -32768

            vView = StaticRenderViewModeEnum.AGENDA_LIST

            if (try { vSaid = com.sce.forge.runtime.SceChecked.bounded("0123456789abcdef", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRenderEvent.Error.Execution, "<assign location='said'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt2() {
                // SCE-MAP: static_render.scxml:43 :: idle :: _transition_2

            if (try { vSaid = com.sce.forge.runtime.SceChecked.bounded("a[[TEXTbTEXT]]c", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRenderEvent.Error.Execution, "<assign location='said'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt3() {
                // SCE-MAP: static_render.scxml:47 :: idle :: _transition_3

            if (try { vSaid = com.sce.forge.runtime.SceChecked.bounded("[[TE[[TEXTXT", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRenderEvent.Error.Execution, "<assign location='said'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt4() {
                // SCE-MAP: static_render.scxml:51 :: idle :: _transition_4

            if (try { vSaid = com.sce.forge.runtime.SceChecked.bounded("é[[TEXTü", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRenderEvent.Error.Execution, "<assign location='said'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt5() {
                // SCE-MAP: static_render.scxml:55 :: idle :: _transition_5

            if (try { vNote = com.sce.forge.runtime.SceChecked.bounded(TextNotice.render(vWho, com.sce.forge.runtime.SceChecked.sub(vCount, 1), (vView).declaredName, vSaid), 95); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRenderEvent.Error.Execution, "<sce:render template='Notice'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 100.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRenderEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt6() {
                // SCE-MAP: static_render.scxml:64 :: idle :: _transition_6

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRenderEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
