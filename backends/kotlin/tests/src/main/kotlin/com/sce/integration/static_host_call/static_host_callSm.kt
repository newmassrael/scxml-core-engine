// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_host_call.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_host_call.scxml:19 :: _machine

package com.sce.integration.static_host_call

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticHostCallState : State {
    data object Idle : StaticHostCallState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticHostCallEvent : Event {
    data object Escalate : StaticHostCallEvent
    data object Plan : StaticHostCallEvent
    data object Retry : StaticHostCallEvent
    data object Unplan : StaticHostCallEvent
}
// --- State Machine (W3C SCXML) ---

// ── W3C SCXML G.7: `<sce:action>` host dispatch ───────────────────────
/**
 * W3C SCXML G.7: host operations dispatched by `<sce:action>`.
 * The host supplies the side effects while the statechart keeps each
 * operation symbolic. No runtime script engine is involved.
 */
interface StaticHostCallActions {
    fun showAttempts(count: UInt, exhausted: Boolean)
    fun showDue(day: Int?, urgent: Boolean?)
}

/**
 * [StaticHostCallActions] that performs nothing and records every call in order —
 * the host a test drives the machine with. Read [calls] after the machine
 * has run; each call is compared by value.
 */
class RecordingStaticHostCallActions : StaticHostCallActions {
    /** One recorded host call. */
    sealed interface Call {
        data class ShowAttempts(val count: UInt, val exhausted: Boolean) : Call
        data class ShowDue(val day: Int?, val urgent: Boolean?) : Call
    }

    private val recorded = mutableListOf<Call>()

    /** Every call so far, oldest first. */
    val calls: List<Call>
        get() = recorded.toList()

    /** Forget the calls recorded so far. */
    fun clear() {
        recorded.clear()
    }

    override fun showAttempts(count: UInt, exhausted: Boolean) {
        recorded += Call.ShowAttempts(count, exhausted)
    }
    override fun showDue(day: Int?, urgent: Boolean?) {
        recorded += Call.ShowDue(day, urgent)
    }
}

class StaticHostCallStateMachine(
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
    private val actions: StaticHostCallActions,
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticHostCallState, StaticHostCallEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `attempts` datamodel variable, the machine's own. */
    private var vAttempts: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `due` datamodel variable, the machine's own. */
    private var vDue: Int? = null
    /** W3C SCXML 5.2: the `urgent` datamodel variable, the machine's own. */
    private var vUrgent: Boolean? = null

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var attempts: UInt? = null
        var due: Int? = null
        var urgent: Boolean? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.attempts?.let { vAttempts = it }
        params.due?.let { vDue = it }
        params.urgent?.let { vUrgent = it }
    }

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticHostCallState>,
        val truncated: Boolean,
    )

    private val _snapshot = kotlinx.coroutines.flow.MutableStateFlow(
        Snapshot(emptySet(), false)
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
        _snapshot.value = Snapshot(activeConfiguration, truncated)
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
        first = "0e54b09a3098bee9e28951082f40f18020c63b5d620a4120bb2c5c584a6ee23a",
        revised = "d096c40e0051f0b1ca2156f87dfc2449eb12128020a11a693b3ed2d3bd1e3f48",
        format2First = "0e54b09a3098bee9e28951082f40f18020c63b5d620a4120bb2c5c584a6ee23a",
        format2Revised = "d096c40e0051f0b1ca2156f87dfc2449eb12128020a11a693b3ed2d3bd1e3f48",
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
            "attempts" to SavedValues.of(vAttempts),
            "due" to SavedValues.optional(vDue) { SavedValues.of(it) },
            "urgent" to SavedValues.optional(vUrgent) { SavedValues.of(it) },
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
        val saved1 = SavedValues.uint32(savedGrown.variable("attempts"), "attempts")
        val saved2 = SavedValues.optional(savedGrown.variable("due"), "due") { e, w -> SavedValues.int32(e, w) }
        val saved3 = SavedValues.optional(savedGrown.variable("urgent"), "urgent") { e, w -> SavedValues.bool(e, w) }
        vAttempts = saved1
        vDue = saved2
        vUrgent = saved3
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
        SavedValues.uint32(savedGrown.variable("attempts"), "attempts")
        SavedValues.optional(savedGrown.variable("due"), "due") { e, w -> SavedValues.int32(e, w) }
        SavedValues.optional(savedGrown.variable("urgent"), "urgent") { e, w -> SavedValues.bool(e, w) }
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    override val initialState: StaticHostCallState = StaticHostCallState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticHostCallState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticHostCallState, HistoryId>> =
            listOf(StateTarget(StaticHostCallState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticHostCallState, HistoryId>(
            StaticHostCallState.Idle,
            listOf(StateTarget(StaticHostCallState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticHostCallState, HistoryId>(
            StaticHostCallState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticHostCallState, HistoryId>(
            StaticHostCallState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticHostCallState, HistoryId>(
            StaticHostCallState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticHostCallState? = when (stateId) {
        "idle" -> StaticHostCallState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticHostCallState): String = when (state) {
        is StaticHostCallState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticHostCallState): Int = when (state) {
        is StaticHostCallState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticHostCallEvent? = when (name) {
        "escalate" -> StaticHostCallEvent.Escalate
        "plan" -> StaticHostCallEvent.Plan
        "retry" -> StaticHostCallEvent.Retry
        "unplan" -> StaticHostCallEvent.Unplan
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticHostCallEvent): String? = when (event) {
        is StaticHostCallEvent.Escalate -> "escalate"
        is StaticHostCallEvent.Plan -> "plan"
        is StaticHostCallEvent.Retry -> "retry"
        is StaticHostCallEvent.Unplan -> "unplan"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticHostCallState,
        event: StaticHostCallEvent?
    ): EnabledTransition<StaticHostCallState, HistoryId>? = when (state) {
        is StaticHostCallState.Idle -> when {
            event is StaticHostCallEvent.Retry && vAttempts < 3.toUInt() -> transitionIdleAt0
            event is StaticHostCallEvent.Plan -> transitionIdleAt1
            event is StaticHostCallEvent.Escalate -> transitionIdleAt2
            event is StaticHostCallEvent.Unplan -> transitionIdleAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_host_call.scxml:19 :: _machine
    override fun onEntry(state: StaticHostCallState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticHostCallState.Idle -> {
                // SCE-MAP: static_host_call.scxml:26 :: idle :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            // W3C SCXML G.7: <sce:action name="showAttempts">
            actions.showAttempts(vAttempts, vAttempts >= 3.toUInt())
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_host_call.scxml:19 :: _machine
    override fun onExit(state: StaticHostCallState) {
        when (state) {
            is StaticHostCallState.Idle -> {
                // SCE-MAP: static_host_call.scxml:26 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_host_call.scxml:19 :: _machine
    override fun executeTransitionContent(source: StaticHostCallState, transitionIndex: Int) {
        when (source) {
        is StaticHostCallState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_call.scxml:33 :: idle :: _transition_0

            if (try { vAttempts = com.sce.forge.runtime.SceChecked.add(vAttempts, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_call.scxml:36 :: idle :: _transition_1

            vDue = 7

            // W3C SCXML G.7: <sce:action name="showDue">
            actions.showDue(vDue, vUrgent)
            }
            2 -> {
                // SCE-MAP: static_host_call.scxml:43 :: idle :: _transition_2

            vUrgent = true

            // W3C SCXML G.7: <sce:action name="showDue">
            actions.showDue(vDue, vUrgent)
            }
            3 -> {
                // SCE-MAP: static_host_call.scxml:50 :: idle :: _transition_3

            vDue = null

            vUrgent = false

            // W3C SCXML G.7: <sce:action name="showDue">
            actions.showDue(vDue, vUrgent)
            }
            else -> {}
        }
        }
    }
}
