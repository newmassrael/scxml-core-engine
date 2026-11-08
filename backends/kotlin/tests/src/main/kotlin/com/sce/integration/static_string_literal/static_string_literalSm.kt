// SCE-GENERATED — DO NOT EDIT
// source-hash: ea764ab5524bc7a7e7d4553640b3fb26547d13f0d2053c0f5e7214161efbc6d0

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_string_literal.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_string_literal.scxml:19 :: _machine

package com.sce.integration.static_string_literal

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticStringLiteralState : State {
    data object Idle : StaticStringLiteralState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticStringLiteralEvent : Event {
    data object Astral : StaticStringLiteralEvent
    data object Backslash : StaticStringLiteralEvent
    data object Controls : StaticStringLiteralEvent
    data object Hex : StaticStringLiteralEvent
    data object Interpolation : StaticStringLiteralEvent
    data object Measure : StaticStringLiteralEvent
    data object Quote : StaticStringLiteralEvent
    data object Template : StaticStringLiteralEvent
    data object Trigraph : StaticStringLiteralEvent
}
// --- State Machine (W3C SCXML) ---

class StaticStringLiteralStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticStringLiteralState, StaticStringLiteralEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `held` datamodel variable, published (`sce:direction="out"`). */
    private var vHeld: String = "a\"b"

    /** What the published `held` holds. */
    val held: String get() = vHeld
    /** W3C SCXML 5.2: the `measured` datamodel variable, published (`sce:direction="out"`). */
    private var vMeasured: UInt = 0.toUInt()

    /** What the published `measured` holds. */
    val measured: UInt get() = vMeasured

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var held: String? = null
        var measured: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.held?.let { vHeld = it }
        params.measured?.let { vMeasured = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val held: String,
        val measured: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticStringLiteralState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        held = vHeld,
        measured = vMeasured,
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
        first = "6f1e40ef39b70a13447300ffce9bec90cb1663b9376ffb51ffcd2d1ac65a680d",
        revised = "870a742563c7601d377427a83417f4c9b169d2cdb70d191393454c03a9ec502e",
        format2First = "6f1e40ef39b70a13447300ffce9bec90cb1663b9376ffb51ffcd2d1ac65a680d",
        format2Revised = "870a742563c7601d377427a83417f4c9b169d2cdb70d191393454c03a9ec502e",
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
    fun save(wallNowMs: Long): SavedState {
        val variables = LinkedHashMap<String, Any?>()
        saveVariablesPart0(variables)
        return savedState(savedShape, variables, wallNowMs)
    }

    private fun saveVariablesPart0(variables: MutableMap<String, Any?>) {
        variables += "held" to SavedValues.of(vHeld)
        variables += "measured" to SavedValues.of(vMeasured)
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
        val saved1 = SavedValues.string(savedGrown.variable("held"), "held", 32)
        val saved2 = SavedValues.uint32(savedGrown.variable("measured"), "measured")
        return {
            vHeld = saved1
            vMeasured = saved2
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
        SavedValues.string(savedGrown.variable("held"), "held", 32)
        SavedValues.uint32(savedGrown.variable("measured"), "measured")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    override val initialState: StaticStringLiteralState = StaticStringLiteralState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticStringLiteralState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticStringLiteralState, HistoryId>> =
            listOf(StateTarget(StaticStringLiteralState.Idle))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticStringLiteralState, HistoryId>(
            StaticStringLiteralState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticStringLiteralState, HistoryId>(
            StaticStringLiteralState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticStringLiteralState, HistoryId>(
            StaticStringLiteralState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticStringLiteralState, HistoryId>(
            StaticStringLiteralState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticStringLiteralState, HistoryId>(
            StaticStringLiteralState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<StaticStringLiteralState, HistoryId>(
            StaticStringLiteralState.Idle,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 6, as the microstep reads it.
        val transitionIdleAt6 = EnabledTransition<StaticStringLiteralState, HistoryId>(
            StaticStringLiteralState.Idle,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 7, as the microstep reads it.
        val transitionIdleAt7 = EnabledTransition<StaticStringLiteralState, HistoryId>(
            StaticStringLiteralState.Idle,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 8, as the microstep reads it.
        val transitionIdleAt8 = EnabledTransition<StaticStringLiteralState, HistoryId>(
            StaticStringLiteralState.Idle,
            emptyList(),
            8,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticStringLiteralState? = when (stateId) {
        "idle" -> StaticStringLiteralState.Idle
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticStringLiteralState): String = when (state) {
        is StaticStringLiteralState.Idle -> "idle"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticStringLiteralState): Int = when (state) {
        is StaticStringLiteralState.Idle -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticStringLiteralEvent? = when (name) {
        "astral" -> StaticStringLiteralEvent.Astral
        "backslash" -> StaticStringLiteralEvent.Backslash
        "controls" -> StaticStringLiteralEvent.Controls
        "hex" -> StaticStringLiteralEvent.Hex
        "interpolation" -> StaticStringLiteralEvent.Interpolation
        "measure" -> StaticStringLiteralEvent.Measure
        "quote" -> StaticStringLiteralEvent.Quote
        "template" -> StaticStringLiteralEvent.Template
        "trigraph" -> StaticStringLiteralEvent.Trigraph
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticStringLiteralEvent): String? = when (event) {
        is StaticStringLiteralEvent.Astral -> "astral"
        is StaticStringLiteralEvent.Backslash -> "backslash"
        is StaticStringLiteralEvent.Controls -> "controls"
        is StaticStringLiteralEvent.Hex -> "hex"
        is StaticStringLiteralEvent.Interpolation -> "interpolation"
        is StaticStringLiteralEvent.Measure -> "measure"
        is StaticStringLiteralEvent.Quote -> "quote"
        is StaticStringLiteralEvent.Template -> "template"
        is StaticStringLiteralEvent.Trigraph -> "trigraph"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticStringLiteralState, event: StaticStringLiteralEvent?): EnabledTransition<StaticStringLiteralState, HistoryId>? = when (state) {
        is StaticStringLiteralState.Idle -> firstEnabledTransitionOfIdle(state, event)
    }


    // idle's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdle(state: StaticStringLiteralState, event: StaticStringLiteralEvent?): EnabledTransition<StaticStringLiteralState, HistoryId>? = when {
        event is StaticStringLiteralEvent.Quote -> TransitionsPart0.transitionIdleAt0
        event is StaticStringLiteralEvent.Backslash -> TransitionsPart0.transitionIdleAt1
        event is StaticStringLiteralEvent.Template -> TransitionsPart0.transitionIdleAt2
        event is StaticStringLiteralEvent.Interpolation -> TransitionsPart0.transitionIdleAt3
        event is StaticStringLiteralEvent.Controls -> TransitionsPart0.transitionIdleAt4
        event is StaticStringLiteralEvent.Hex -> TransitionsPart0.transitionIdleAt5
        event is StaticStringLiteralEvent.Astral -> TransitionsPart0.transitionIdleAt6
        event is StaticStringLiteralEvent.Trigraph -> TransitionsPart0.transitionIdleAt7
        event is StaticStringLiteralEvent.Measure -> TransitionsPart0.transitionIdleAt8
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_string_literal.scxml:19 :: _machine
    override fun onEntry(state: StaticStringLiteralState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticStringLiteralState.Idle -> onEntryOfIdle()
        }
    }


    private fun onEntryOfIdle() {
                // SCE-MAP: static_string_literal.scxml:25 :: idle :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_string_literal.scxml:19 :: _machine
    override fun onExit(state: StaticStringLiteralState) {
        when (state) {
            is StaticStringLiteralState.Idle -> onExitOfIdle()
        }
    }


    private fun onExitOfIdle() {
                // SCE-MAP: static_string_literal.scxml:25 :: idle :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_string_literal.scxml:19 :: _machine
    override fun executeTransitionContent(source: StaticStringLiteralState, transitionIndex: Int) {
        when (source) {
            is StaticStringLiteralState.Idle -> transitionContentOfIdle(transitionIndex)
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
            7 -> transitionContentOfIdleAt7()
            8 -> transitionContentOfIdleAt8()
            else -> {}
        }
    }


    private fun transitionContentOfIdleAt0() {
                // SCE-MAP: static_string_literal.scxml:26 :: idle :: _transition_0

            if (try { vHeld = com.sce.forge.runtime.SceChecked.bounded("it's", 32); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt1() {
                // SCE-MAP: static_string_literal.scxml:29 :: idle :: _transition_1

            if (try { vHeld = com.sce.forge.runtime.SceChecked.bounded("a\\b", 32); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt2() {
                // SCE-MAP: static_string_literal.scxml:32 :: idle :: _transition_2

            if (try { vHeld = com.sce.forge.runtime.SceChecked.bounded("\$name \${x}", 32); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt3() {
                // SCE-MAP: static_string_literal.scxml:35 :: idle :: _transition_3

            if (try { vHeld = com.sce.forge.runtime.SceChecked.bounded("\\(x)", 32); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt4() {
                // SCE-MAP: static_string_literal.scxml:38 :: idle :: _transition_4

            if (try { vHeld = com.sce.forge.runtime.SceChecked.bounded("a\nb\tc", 32); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt5() {
                // SCE-MAP: static_string_literal.scxml:42 :: idle :: _transition_5

            if (try { vHeld = com.sce.forge.runtime.SceChecked.bounded("Aé€", 32); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt6() {
                // SCE-MAP: static_string_literal.scxml:46 :: idle :: _transition_6

            if (try { vHeld = com.sce.forge.runtime.SceChecked.bounded("𐍈𐍈", 32); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt7() {
                // SCE-MAP: static_string_literal.scxml:49 :: idle :: _transition_7

            if (try { vHeld = com.sce.forge.runtime.SceChecked.bounded("a??=b", 32); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt8() {
                // SCE-MAP: static_string_literal.scxml:52 :: idle :: _transition_8

            vMeasured = (vHeld).encodeToByteArray().size.toUInt()
    }
}
