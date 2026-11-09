// SCE-GENERATED — DO NOT EDIT
// source-hash: 39c188440acd2a30e1c96c90e825a582fc7011a2c558c89e70ea674bb5bda40e

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_invoke_tree.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_invoke_tree.scxml:23 :: _machine

package com.sce.integration.static_invoke_tree

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticInvokeTreeState : State {
    data object Resting : StaticInvokeTreeState
    data object Working : StaticInvokeTreeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticInvokeTreeEvent : Event {
    data object Answer : StaticInvokeTreeEvent
    data object Arm : StaticInvokeTreeEvent
    data object Ask : StaticInvokeTreeEvent
    data object Back : StaticInvokeTreeEvent
    sealed interface Done : StaticInvokeTreeEvent {
        data object Invoke : Done
    }
    sealed interface Error : StaticInvokeTreeEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Go : StaticInvokeTreeEvent
    data object Nudge : StaticInvokeTreeEvent
    data object Question : StaticInvokeTreeEvent
    data object Report : StaticInvokeTreeEvent
    data object Rest : StaticInvokeTreeEvent
    data object Stamped : StaticInvokeTreeEvent
    data object Swap : StaticInvokeTreeEvent
}
// --- State Machine (W3C SCXML) ---

class StaticInvokeTreeStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticInvokeTreeState, StaticInvokeTreeEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `answers` datamodel variable, published (`sce:direction="out"`). */
    private var vAnswers: UInt = 0.toUInt()

    /** What the published `answers` holds. */
    val answers: UInt get() = vAnswers
    /** W3C SCXML 5.2: the `reports` datamodel variable, published (`sce:direction="out"`). */
    private var vReports: UInt = 0.toUInt()

    /** What the published `reports` holds. */
    val reports: UInt get() = vReports
    /** W3C SCXML 5.2: the `stamps` datamodel variable, published (`sce:direction="out"`). */
    private var vStamps: UInt = 0.toUInt()

    /** What the published `stamps` holds. */
    val stamps: UInt get() = vStamps

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var answers: UInt? = null
        var reports: UInt? = null
        var stamps: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.answers?.let { vAnswers = it }
        params.reports?.let { vReports = it }
        params.stamps?.let { vStamps = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val answers: UInt,
        val reports: UInt,
        val stamps: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticInvokeTreeState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        answers = vAnswers,
        reports = vReports,
        stamps = vStamps,
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
        first = "a28d6fa9db66327ab1f11b0ea924c1623dee60d8aee767949439f0eb5aeb990f",
        revised = "bab0ea6d6e2cb56b20d3028d098f17045fe20a45f1abd9eb838ead2d8c701998",
        format2First = "a28d6fa9db66327ab1f11b0ea924c1623dee60d8aee767949439f0eb5aeb990f",
        format2Revised = "bab0ea6d6e2cb56b20d3028d098f17045fe20a45f1abd9eb838ead2d8c701998",
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
        variables += "answers" to SavedValues.of(vAnswers)
        variables += "reports" to SavedValues.of(vReports)
        variables += "stamps" to SavedValues.of(vStamps)
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
        val saved1 = SavedValues.uint32(savedGrown.variable("answers"), "answers")
        val saved2 = SavedValues.uint32(savedGrown.variable("reports"), "reports")
        val saved3 = SavedValues.uint32(savedGrown.variable("stamps"), "stamps")
        return {
            vAnswers = saved1
            vReports = saved2
            vStamps = saved3
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
        SavedValues.uint32(savedGrown.variable("answers"), "answers")
        SavedValues.uint32(savedGrown.variable("reports"), "reports")
        SavedValues.uint32(savedGrown.variable("stamps"), "stamps")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4: a saved state names the `<invoke>`s whose child is running,
    // and a restore starts each again from its beginning. This document saves
    // only when every invoke is a static child session, a hybrid one among
    // declared candidates, or one a declared host invoker serves.
    override val staticInvokes: List<Pair<String, StaticInvokeTreeState>> = listOf(
        "worker" to StaticInvokeTreeState.Working,
    )

    override fun restartInvoke(invokeId: String, state: StaticInvokeTreeState) = deferStaticInvoke(invokeId, state)

    override val initialState: StaticInvokeTreeState = StaticInvokeTreeState.Working

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
    override val documentInitialTargets: List<EntryTarget<StaticInvokeTreeState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticInvokeTreeState, HistoryId>> =
            listOf(StateTarget(StaticInvokeTreeState.Working))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: resting's transition 0, as the microstep reads it.
        val transitionRestingAt0 = EnabledTransition<StaticInvokeTreeState, HistoryId>(
            StaticInvokeTreeState.Resting,
            listOf(StateTarget(StaticInvokeTreeState.Working)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<StaticInvokeTreeState, HistoryId>(
            StaticInvokeTreeState.Working,
            emptyList(),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 1, as the microstep reads it.
        val transitionWorkingAt1 = EnabledTransition<StaticInvokeTreeState, HistoryId>(
            StaticInvokeTreeState.Working,
            emptyList(),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 2, as the microstep reads it.
        val transitionWorkingAt2 = EnabledTransition<StaticInvokeTreeState, HistoryId>(
            StaticInvokeTreeState.Working,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 3, as the microstep reads it.
        val transitionWorkingAt3 = EnabledTransition<StaticInvokeTreeState, HistoryId>(
            StaticInvokeTreeState.Working,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 4, as the microstep reads it.
        val transitionWorkingAt4 = EnabledTransition<StaticInvokeTreeState, HistoryId>(
            StaticInvokeTreeState.Working,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 5, as the microstep reads it.
        val transitionWorkingAt5 = EnabledTransition<StaticInvokeTreeState, HistoryId>(
            StaticInvokeTreeState.Working,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 6, as the microstep reads it.
        val transitionWorkingAt6 = EnabledTransition<StaticInvokeTreeState, HistoryId>(
            StaticInvokeTreeState.Working,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 7, as the microstep reads it.
        val transitionWorkingAt7 = EnabledTransition<StaticInvokeTreeState, HistoryId>(
            StaticInvokeTreeState.Working,
            listOf(StateTarget(StaticInvokeTreeState.Resting)),
            7,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticInvokeTreeState? = when (stateId) {
        "resting" -> StaticInvokeTreeState.Resting
        "working" -> StaticInvokeTreeState.Working
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticInvokeTreeState): String = when (state) {
        is StaticInvokeTreeState.Resting -> "resting"
        is StaticInvokeTreeState.Working -> "working"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticInvokeTreeState): Int = when (state) {
        is StaticInvokeTreeState.Resting -> 1
        is StaticInvokeTreeState.Working -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticInvokeTreeEvent? = when (name) {
        "answer" -> StaticInvokeTreeEvent.Answer
        "arm" -> StaticInvokeTreeEvent.Arm
        "ask" -> StaticInvokeTreeEvent.Ask
        "back" -> StaticInvokeTreeEvent.Back
        "done.invoke" -> StaticInvokeTreeEvent.Done.Invoke
        "error.communication" -> StaticInvokeTreeEvent.Error.Communication
        "error.execution" -> StaticInvokeTreeEvent.Error.Execution
        "go" -> StaticInvokeTreeEvent.Go
        "nudge" -> StaticInvokeTreeEvent.Nudge
        "question" -> StaticInvokeTreeEvent.Question
        "report" -> StaticInvokeTreeEvent.Report
        "rest" -> StaticInvokeTreeEvent.Rest
        "stamped" -> StaticInvokeTreeEvent.Stamped
        "swap" -> StaticInvokeTreeEvent.Swap
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticInvokeTreeEvent): String? = when (event) {
        is StaticInvokeTreeEvent.Answer -> "answer"
        is StaticInvokeTreeEvent.Arm -> "arm"
        is StaticInvokeTreeEvent.Ask -> "ask"
        is StaticInvokeTreeEvent.Back -> "back"
        is StaticInvokeTreeEvent.Done.Invoke -> "done.invoke"
        is StaticInvokeTreeEvent.Error.Communication -> "error.communication"
        is StaticInvokeTreeEvent.Error.Execution -> "error.execution"
        is StaticInvokeTreeEvent.Go -> "go"
        is StaticInvokeTreeEvent.Nudge -> "nudge"
        is StaticInvokeTreeEvent.Question -> "question"
        is StaticInvokeTreeEvent.Report -> "report"
        is StaticInvokeTreeEvent.Rest -> "rest"
        is StaticInvokeTreeEvent.Stamped -> "stamped"
        is StaticInvokeTreeEvent.Swap -> "swap"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticInvokeTreeState, event: StaticInvokeTreeEvent?): EnabledTransition<StaticInvokeTreeState, HistoryId>? = when (state) {
        is StaticInvokeTreeState.Resting -> firstEnabledTransitionOfResting(state, event)
        is StaticInvokeTreeState.Working -> firstEnabledTransitionOfWorking(state, event)
    }


    // resting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfResting(state: StaticInvokeTreeState, event: StaticInvokeTreeEvent?): EnabledTransition<StaticInvokeTreeState, HistoryId>? = when {
        event is StaticInvokeTreeEvent.Back -> TransitionsPart0.transitionRestingAt0
        else -> null
    }


    // working's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWorking(state: StaticInvokeTreeState, event: StaticInvokeTreeEvent?): EnabledTransition<StaticInvokeTreeState, HistoryId>? = when {
        event is StaticInvokeTreeEvent.Go -> TransitionsPart0.transitionWorkingAt0
        event is StaticInvokeTreeEvent.Swap -> TransitionsPart0.transitionWorkingAt1
        event is StaticInvokeTreeEvent.Ask -> TransitionsPart0.transitionWorkingAt2
        event is StaticInvokeTreeEvent.Arm -> TransitionsPart0.transitionWorkingAt3
        event is StaticInvokeTreeEvent.Answer -> TransitionsPart0.transitionWorkingAt4
        event is StaticInvokeTreeEvent.Report -> TransitionsPart0.transitionWorkingAt5
        event is StaticInvokeTreeEvent.Stamped -> TransitionsPart0.transitionWorkingAt6
        event is StaticInvokeTreeEvent.Rest -> TransitionsPart0.transitionWorkingAt7
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_invoke_tree.scxml:23 :: _machine
    override fun onEntry(state: StaticInvokeTreeState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticInvokeTreeState.Resting -> onEntryOfResting()
            is StaticInvokeTreeState.Working -> onEntryOfWorking()
        }
    }


    private fun onEntryOfResting() {
                // SCE-MAP: static_invoke_tree.scxml:102 :: resting :: _state_body
    }

    private fun onEntryOfWorking() {
                // SCE-MAP: static_invoke_tree.scxml:30 :: working :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("worker", StaticInvokeTreeState.Working)
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
    private fun deferStaticInvoke(invokeId: String, state: StaticInvokeTreeState) {
        when (invokeId) {
            "worker" -> startInvokeOfWorkingAt0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfWorkingAt0(state: StaticInvokeTreeState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "working.${nextInvokeStart("worker")}.worker"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(StaticInvokeTreeSceSynthInvokeWorkerStateMachine(), generatedInvokeId)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("worker", childSM, true, StaticInvokeTreeEvent.Done.Invoke, "", generatedInvokeId, "", "static_invoke_tree__sce_synth_invoke__worker", "worker")
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
    override fun buildSavedChild(child: SavedChild): ResumedChild<StaticInvokeTreeEvent> = when (child.id) {
        "worker" -> savedChildOfWorkingAt0(child)
        else -> throw StateRefusal("the document has no child session '${child.id}'")
    }


    private fun savedChildOfWorkingAt0(child: SavedChild): ResumedChild<StaticInvokeTreeEvent> =
        if (child.document == "static_invoke_tree__sce_synth_invoke__worker") {
            ResumedChild(
                StaticInvokeTreeSceSynthInvokeWorkerStateMachine(),
                true, StaticInvokeTreeEvent.Done.Invoke, "", "", "worker",
            )
        } else {
            throw StateRefusal("invokes names '${child.id}' running '${child.document}', a document that `<invoke>` does not start")
        }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_invoke_tree.scxml:23 :: _machine
    override fun onExit(state: StaticInvokeTreeState) {
        when (state) {
            is StaticInvokeTreeState.Resting -> onExitOfResting()
            is StaticInvokeTreeState.Working -> onExitOfWorking()
        }
    }


    private fun onExitOfResting() {
                // SCE-MAP: static_invoke_tree.scxml:102 :: resting :: _state_body
    }

    private fun onExitOfWorking() {
                // SCE-MAP: static_invoke_tree.scxml:30 :: working :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(StaticInvokeTreeState.Working)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("worker")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_invoke_tree.scxml:23 :: _machine
    override fun executeTransitionContent(source: StaticInvokeTreeState, transitionIndex: Int) {
        when (source) {
            is StaticInvokeTreeState.Working -> transitionContentOfWorking(transitionIndex)
            else -> {}
        }
    }


    // The content of `working`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWorking(transitionIndex: Int) {
        when (transitionIndex) {
            2 -> transitionContentOfWorkingAt2()
            3 -> transitionContentOfWorkingAt3()
            4 -> transitionContentOfWorkingAt4()
            5 -> transitionContentOfWorkingAt5()
            6 -> transitionContentOfWorkingAt6()
            else -> {}
        }
    }


    private fun transitionContentOfWorkingAt2() {
                // SCE-MAP: static_invoke_tree.scxml:85 :: working :: _transition_2


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test192): Send event to invoked child. The build
            // refused a target naming no invocation above; one that is not
            // running is not there to reach (W3C SCXML C.1).
            if (!sendToChild("worker", "question", sendData)) {
                raisePlatformError(StaticInvokeTreeEvent.Error.Communication, "<send target='#_worker'> names an invocation that is not running", "__send_0")
                return@send true
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfWorkingAt3() {
                // SCE-MAP: static_invoke_tree.scxml:88 :: working :: _transition_3


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2.4 + 6.4: a delayed send to an invoked child. Whether
            // it is still running is judged when the send comes due; if not,
            // the sender hears error.communication then (W3C SCXML C.1).
            scheduleChildSend("poke", 5000L, "worker", "nudge", sendData, StaticInvokeTreeEvent.Error.Communication)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfWorkingAt4() {
                // SCE-MAP: static_invoke_tree.scxml:91 :: working :: _transition_4

            if (try { vAnswers = com.sce.forge.runtime.SceChecked.add(vAnswers, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeTreeEvent.Error.Execution, "<assign location='answers'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfWorkingAt5() {
                // SCE-MAP: static_invoke_tree.scxml:94 :: working :: _transition_5

            if (try { vReports = com.sce.forge.runtime.SceChecked.add(vReports, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeTreeEvent.Error.Execution, "<assign location='reports'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfWorkingAt6() {
                // SCE-MAP: static_invoke_tree.scxml:97 :: working :: _transition_6

            if (try { vStamps = com.sce.forge.runtime.SceChecked.add(vStamps, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeTreeEvent.Error.Execution, "<assign location='stamps'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
