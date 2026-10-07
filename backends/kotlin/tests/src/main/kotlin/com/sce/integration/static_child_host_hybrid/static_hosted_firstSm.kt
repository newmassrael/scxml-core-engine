// SCE-GENERATED — DO NOT EDIT
// source-hash: d321654623235a43e67203edd8f804817a9d25b08581dd212fe8c5386f0fb495

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_hosted_first.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_hosted_first.scxml:8 :: _machine

package com.sce.integration.static_child_host_hybrid

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticHostedFirstState : State {
    data object Busy : StaticHostedFirstState
    data object Leaf : StaticHostedFirstState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticHostedFirstEvent : Event {

}
// --- State Machine (W3C SCXML) ---

// ── W3C SCXML G.7: `<sce:action>` host dispatch ───────────────────────
/**
 * W3C SCXML G.7: host operations dispatched by `<sce:action>`.
 * The host supplies the side effects while the statechart keeps each
 * operation symbolic. No runtime script engine is involved.
 */
interface StaticHostedFirstActions {
    fun firstRan()
}

/**
 * [StaticHostedFirstActions] that performs nothing and records every call in order —
 * the host a test drives the machine with. Read [calls] after the machine
 * has run; each call is compared by value.
 */
class RecordingStaticHostedFirstActions : StaticHostedFirstActions {
    /** One recorded host call. */
    sealed interface Call {
        data object FirstRan : Call
    }

    private val recorded = mutableListOf<Call>()

    /** Every call so far, oldest first. */
    val calls: List<Call>
        get() = recorded.toList()

    /** Forget the calls recorded so far. */
    fun clear() {
        recorded.clear()
    }

    override fun firstRan() {
        recorded += Call.FirstRan
    }
}

class StaticHostedFirstStateMachine(
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
    private val actions: StaticHostedFirstActions,
) : StateMachineEngine<StaticHostedFirstState, StaticHostedFirstEvent>() {

    // ── SCE Accepted Subset §2.15: saving this machine, restoring it ─────────

    /**
     * The shape a saved state of this document is bound to: a state saved
     * from a document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not.
     */
    val savedShape: String = "fd7b03b2e4562af6acf12bd49174d042b940a5a511449688a641080b2f84f2d0"

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
        beginRestore(saved, savedShape)
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticHostedFirstState = StaticHostedFirstState.Busy

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
    override fun isFinalState(state: StaticHostedFirstState): Boolean = when (state) {
        is StaticHostedFirstState.Leaf -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticHostedFirstState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticHostedFirstState, HistoryId>> =
            listOf(StateTarget(StaticHostedFirstState.Busy))

        // W3C SCXML 3.13: busy's transition 0, as the microstep reads it.
        val transitionBusyAt0 = EnabledTransition<StaticHostedFirstState, HistoryId>(
            StaticHostedFirstState.Busy,
            listOf(StateTarget(StaticHostedFirstState.Leaf)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticHostedFirstState? = when (stateId) {
        "busy" -> StaticHostedFirstState.Busy
        "leaf" -> StaticHostedFirstState.Leaf
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticHostedFirstState): String = when (state) {
        is StaticHostedFirstState.Busy -> "busy"
        is StaticHostedFirstState.Leaf -> "leaf"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticHostedFirstState): Int = when (state) {
        is StaticHostedFirstState.Busy -> 0
        is StaticHostedFirstState.Leaf -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticHostedFirstEvent? = when (name) {
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    // A child SM that inherits the has_parent_communication override while
    // declaring no events of its own leaves the sealed hierarchy with zero
    // implementors, so `StaticHostedFirstEvent` is uninhabited: no caller can
    // construct an argument and the body is unreachable. A `when` over an
    // uninhabited sealed subject is vacuously exhaustive, so any branch —
    // `else` included — is dead code the compiler rejects under -Werror.
    // Returning the null directly is the honest expression of "unreachable".
    override fun eventNameOf(event: StaticHostedFirstEvent): String? = null





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticHostedFirstState,
        event: StaticHostedFirstEvent?
    ): EnabledTransition<StaticHostedFirstState, HistoryId>? = when (state) {
        is StaticHostedFirstState.Busy -> when {
            event == null -> transitionBusyAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_hosted_first.scxml:8 :: _machine
    override fun onEntry(state: StaticHostedFirstState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticHostedFirstState.Busy -> {
                // SCE-MAP: static_hosted_first.scxml:10 :: busy :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            // W3C SCXML G.7: <sce:action name="first_ran">
            actions.firstRan()
                }
            }
            is StaticHostedFirstState.Leaf -> {
                // SCE-MAP: static_hosted_first.scxml:16 :: leaf :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_hosted_first.scxml:8 :: _machine
    override fun onExit(state: StaticHostedFirstState) {
        when (state) {
            is StaticHostedFirstState.Busy -> {
                // SCE-MAP: static_hosted_first.scxml:10 :: busy :: _state_body
            }
            is StaticHostedFirstState.Leaf -> {
                // SCE-MAP: static_hosted_first.scxml:16 :: leaf :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_hosted_first.scxml:8 :: _machine
    override fun executeTransitionContent(source: StaticHostedFirstState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
