// SCE-GENERATED — DO NOT EDIT
// source-hash: ea764ab5524bc7a7e7d4553640b3fb26547d13f0d2053c0f5e7214161efbc6d0

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_invoke_ending_order.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_invoke_ending_order.scxml:35 :: _machine

package com.sce.integration.static_invoke_ending_order

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticInvokeEndingOrderState : State {
    data object Working : StaticInvokeEndingOrderState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticInvokeEndingOrderEvent : Event {
    data object Answer : StaticInvokeEndingOrderEvent
    data object Ask : StaticInvokeEndingOrderEvent
    sealed interface Done : StaticInvokeEndingOrderEvent {
        data object Invoke : Done
    }
    sealed interface Error : StaticInvokeEndingOrderEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Question : StaticInvokeEndingOrderEvent
    data object Stamped : StaticInvokeEndingOrderEvent
}
// --- State Machine (W3C SCXML) ---

// ── W3C SCXML G.7: `<sce:action>` host dispatch ───────────────────────
/**
 * W3C SCXML G.7: host operations dispatched by `<sce:action>`.
 * The host supplies the side effects while the statechart keeps each
 * operation symbolic. No runtime script engine is involved.
 */
interface StaticInvokeEndingOrderActions {
    fun beforeQuestion()
}

/**
 * [StaticInvokeEndingOrderActions] that performs nothing and records every call in order —
 * the host a test drives the machine with. Read [calls] after the machine
 * has run; each call is compared by value.
 */
class RecordingStaticInvokeEndingOrderActions : StaticInvokeEndingOrderActions {
    /** One recorded host call. */
    sealed interface Call {
        data object BeforeQuestion : Call
    }

    private val recorded = mutableListOf<Call>()

    /** Every call so far, oldest first. */
    val calls: List<Call>
        get() = recorded.toList()

    /** Forget the calls recorded so far. */
    fun clear() {
        recorded.clear()
    }

    override fun beforeQuestion() {
        recorded += Call.BeforeQuestion
    }
}

class StaticInvokeEndingOrderStateMachine(
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
    private val actions: StaticInvokeEndingOrderActions,
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticInvokeEndingOrderState, StaticInvokeEndingOrderEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `heard` datamodel variable, the machine's own. */
    private var vHeard: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `stamped_at` datamodel variable, published (`sce:direction="out"`). */
    private var vStampedAt: UInt = 0.toUInt()

    /** What the published `stamped_at` holds. */
    val stampedAt: UInt get() = vStampedAt
    /** W3C SCXML 5.2: the `answer_at` datamodel variable, published (`sce:direction="out"`). */
    private var vAnswerAt: UInt = 0.toUInt()

    /** What the published `answer_at` holds. */
    val answerAt: UInt get() = vAnswerAt

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var heard: UInt? = null
        var stampedAt: UInt? = null
        var answerAt: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.heard?.let { vHeard = it }
        params.stampedAt?.let { vStampedAt = it }
        params.answerAt?.let { vAnswerAt = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val stampedAt: UInt,
        val answerAt: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticInvokeEndingOrderState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        stampedAt = vStampedAt,
        answerAt = vAnswerAt,
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
        first = "31450909625aa512a1cb2f3f27960c2d9d2bb6286f5db49571debcdeb181bfc9",
        revised = "08606b08031f08f6e52f9ab9e8ede8fd208fbf6c6ef0756685d0883a6998aaf3",
        format2First = "31450909625aa512a1cb2f3f27960c2d9d2bb6286f5db49571debcdeb181bfc9",
        format2Revised = "08606b08031f08f6e52f9ab9e8ede8fd208fbf6c6ef0756685d0883a6998aaf3",
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
        variables += "heard" to SavedValues.of(vHeard)
        variables += "stamped_at" to SavedValues.of(vStampedAt)
        variables += "answer_at" to SavedValues.of(vAnswerAt)
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
        val saved1 = SavedValues.uint32(savedGrown.variable("heard"), "heard")
        val saved2 = SavedValues.uint32(savedGrown.variable("stamped_at"), "stamped_at")
        val saved3 = SavedValues.uint32(savedGrown.variable("answer_at"), "answer_at")
        return {
            vHeard = saved1
            vStampedAt = saved2
            vAnswerAt = saved3
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
        SavedValues.uint32(savedGrown.variable("heard"), "heard")
        SavedValues.uint32(savedGrown.variable("stamped_at"), "stamped_at")
        SavedValues.uint32(savedGrown.variable("answer_at"), "answer_at")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4: a saved state names the `<invoke>`s whose child is running,
    // and a restore starts each again from its beginning. This document saves
    // only when every invoke is a static child session, a hybrid one among
    // declared candidates, or one a declared host invoker serves.
    override val staticInvokes: List<Pair<String, StaticInvokeEndingOrderState>> = listOf(
        "worker" to StaticInvokeEndingOrderState.Working,
    )

    override fun restartInvoke(invokeId: String, state: StaticInvokeEndingOrderState) = deferStaticInvoke(invokeId, state)

    override val initialState: StaticInvokeEndingOrderState = StaticInvokeEndingOrderState.Working

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
    override val documentInitialTargets: List<EntryTarget<StaticInvokeEndingOrderState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticInvokeEndingOrderState, HistoryId>> =
            listOf(StateTarget(StaticInvokeEndingOrderState.Working))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<StaticInvokeEndingOrderState, HistoryId>(
            StaticInvokeEndingOrderState.Working,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 1, as the microstep reads it.
        val transitionWorkingAt1 = EnabledTransition<StaticInvokeEndingOrderState, HistoryId>(
            StaticInvokeEndingOrderState.Working,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 2, as the microstep reads it.
        val transitionWorkingAt2 = EnabledTransition<StaticInvokeEndingOrderState, HistoryId>(
            StaticInvokeEndingOrderState.Working,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticInvokeEndingOrderState? = when (stateId) {
        "working" -> StaticInvokeEndingOrderState.Working
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticInvokeEndingOrderState): String = when (state) {
        is StaticInvokeEndingOrderState.Working -> "working"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticInvokeEndingOrderState): Int = when (state) {
        is StaticInvokeEndingOrderState.Working -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticInvokeEndingOrderEvent? = when (name) {
        "answer" -> StaticInvokeEndingOrderEvent.Answer
        "ask" -> StaticInvokeEndingOrderEvent.Ask
        "done.invoke" -> StaticInvokeEndingOrderEvent.Done.Invoke
        "error.communication" -> StaticInvokeEndingOrderEvent.Error.Communication
        "error.execution" -> StaticInvokeEndingOrderEvent.Error.Execution
        "question" -> StaticInvokeEndingOrderEvent.Question
        "stamped" -> StaticInvokeEndingOrderEvent.Stamped
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticInvokeEndingOrderEvent): String? = when (event) {
        is StaticInvokeEndingOrderEvent.Answer -> "answer"
        is StaticInvokeEndingOrderEvent.Ask -> "ask"
        is StaticInvokeEndingOrderEvent.Done.Invoke -> "done.invoke"
        is StaticInvokeEndingOrderEvent.Error.Communication -> "error.communication"
        is StaticInvokeEndingOrderEvent.Error.Execution -> "error.execution"
        is StaticInvokeEndingOrderEvent.Question -> "question"
        is StaticInvokeEndingOrderEvent.Stamped -> "stamped"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticInvokeEndingOrderState, event: StaticInvokeEndingOrderEvent?): EnabledTransition<StaticInvokeEndingOrderState, HistoryId>? = when (state) {
        is StaticInvokeEndingOrderState.Working -> firstEnabledTransitionOfWorking(state, event)
    }


    // working's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWorking(state: StaticInvokeEndingOrderState, event: StaticInvokeEndingOrderEvent?): EnabledTransition<StaticInvokeEndingOrderState, HistoryId>? = when {
        event is StaticInvokeEndingOrderEvent.Ask -> TransitionsPart0.transitionWorkingAt0
        event is StaticInvokeEndingOrderEvent.Stamped -> TransitionsPart0.transitionWorkingAt1
        event is StaticInvokeEndingOrderEvent.Answer -> TransitionsPart0.transitionWorkingAt2
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_invoke_ending_order.scxml:35 :: _machine
    override fun onEntry(state: StaticInvokeEndingOrderState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticInvokeEndingOrderState.Working -> onEntryOfWorking()
        }
    }


    private fun onEntryOfWorking() {
                // SCE-MAP: static_invoke_ending_order.scxml:42 :: working :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("worker", StaticInvokeEndingOrderState.Working)
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
    private fun deferStaticInvoke(invokeId: String, state: StaticInvokeEndingOrderState) {
        when (invokeId) {
            "worker" -> startInvokeOfWorkingAt0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfWorkingAt0(state: StaticInvokeEndingOrderState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "working.${nextInvokeStart("worker")}.worker"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(StaticInvokeEndingOrderSceSynthInvokeWorkerStateMachine(), generatedInvokeId)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("worker", childSM, false, StaticInvokeEndingOrderEvent.Done.Invoke, "", generatedInvokeId, "", "static_invoke_ending_order__sce_synth_invoke__worker", "worker")
                }
            }
    }

    // docs/adr/0045, section 3, steps 1 and 3: the machine of a running child a
    // saved state lists, built as the `<invoke>` that started it builds it and
    // with what a start does with the child once built. Nothing is evaluated here:
    // no `<param>`, no `srcexpr`, no `idlocation` — the child resumes where it
    // stood, and a hybrid child is the candidate its saved `document` names,
    // whatever the `srcexpr` reads now.
    //
    // Each invoke's child is built by a method of its own
    // (`savedChildOf<State>At<n>`), and this dispatches to it with one
    // call an arm, for the reason `onEntry` does.
    override fun buildSavedChild(child: SavedChild): ResumedChild<StaticInvokeEndingOrderEvent> = when (child.id) {
        "worker" -> savedChildOfWorkingAt0(child)
        else -> throw StateRefusal("the document has no child session '${child.id}'")
    }


    private fun savedChildOfWorkingAt0(child: SavedChild): ResumedChild<StaticInvokeEndingOrderEvent> =
        if (child.document == "static_invoke_ending_order__sce_synth_invoke__worker") {
            ResumedChild(
                StaticInvokeEndingOrderSceSynthInvokeWorkerStateMachine(),
                false, StaticInvokeEndingOrderEvent.Done.Invoke, "", "", "worker",
            )
        } else {
            throw StateRefusal("invokes names '${child.id}' running '${child.document}', a document that `<invoke>` does not start")
        }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_invoke_ending_order.scxml:35 :: _machine
    override fun onExit(state: StaticInvokeEndingOrderState) {
        when (state) {
            is StaticInvokeEndingOrderState.Working -> onExitOfWorking()
        }
    }


    private fun onExitOfWorking() {
                // SCE-MAP: static_invoke_ending_order.scxml:42 :: working :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(StaticInvokeEndingOrderState.Working)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("worker")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_invoke_ending_order.scxml:35 :: _machine
    override fun executeTransitionContent(source: StaticInvokeEndingOrderState, transitionIndex: Int) {
        when (source) {
            is StaticInvokeEndingOrderState.Working -> transitionContentOfWorking(transitionIndex)
        }
    }


    // The content of `working`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWorking(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWorkingAt0()
            1 -> transitionContentOfWorkingAt1()
            2 -> transitionContentOfWorkingAt2()
            else -> {}
        }
    }


    private fun transitionContentOfWorkingAt0() {
                // SCE-MAP: static_invoke_ending_order.scxml:60 :: working :: _transition_0

            // W3C SCXML G.7: <sce:action name="beforeQuestion">
            actions.beforeQuestion()


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test192): Send event to invoked child. The build
            // refused a target naming no invocation above; one that is not
            // running is not there to reach (W3C SCXML C.1).
            if (!sendToChild("worker", "question", sendData)) {
                raisePlatformError(StaticInvokeEndingOrderEvent.Error.Communication, "<send target='#_worker'> names an invocation that is not running", "__send_0")
                return@send true
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfWorkingAt1() {
                // SCE-MAP: static_invoke_ending_order.scxml:64 :: working :: _transition_1

            if (try { vHeard = com.sce.forge.runtime.SceChecked.add(vHeard, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeEndingOrderEvent.Error.Execution, "<assign location='heard'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vStampedAt = vHeard
    }

    private fun transitionContentOfWorkingAt2() {
                // SCE-MAP: static_invoke_ending_order.scxml:68 :: working :: _transition_2

            if (try { vHeard = com.sce.forge.runtime.SceChecked.add(vHeard, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeEndingOrderEvent.Error.Execution, "<assign location='heard'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vAnswerAt = vHeard
    }
}
