// SCE-GENERATED — DO NOT EDIT
// source-hash: 011d804f887126b888ea6ba5b4213d56e0f5c8740555a154c83e9b8d0105c4b6

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/test_profile/runtime/asker.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: asker.scxml:7 :: _machine

package com.sce.generated.asker

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AskerState : State {
    data object Asking : AskerState
    data object Checking : AskerState
    data object Failed : AskerState
    data object Passed : AskerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AskerEvent : Event {
    sealed interface Error : AskerEvent {
        data object Execution : Error
    }
    sealed interface Job : AskerEvent {
        data object Reply : Job
    }
    sealed interface Sce : AskerEvent {
        sealed interface Test : Sce {
            data object Snapshot : Test
        }
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `AskerPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// AskerJobReplyPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `job.reply`. Consumers inject it via the `raiseJobReply` seam
// on the machine — they never name this class directly.
data class AskerJobReplyPayload(val job: UInt, val code: UInt)


// --- State Machine (W3C SCXML) ---

// ======================================================================
// The test component the profile's runtime drives (docs/SCE_TEST_PROFILE.md §12.3)
// ======================================================================

/**
 * The test component `asker` as the profile's runtime drives it
 * (docs/SCE_TEST_PROFILE.md §12.3).
 *
 * It owns the machine below, unchanged, and reaches it only through the doors
 * the engine already has: the machine's sends on a port leave through the
 * test-port processor and its PTCs through the test-component invoker, both
 * registered here over the component's runtime context (docs/adr/0084), and
 * each round's snapshot is the one external event every branch is a
 * transition on. The runtime never reports a PTC's completion to the engine:
 * a done branch reads it from the snapshot, so the engine cancels every PTC it
 * started when the invoking state is exited, which is P9's kill. One thread;
 * the engine's scheduler takes no part, since a component sends nothing
 * delayed.
 *
 * The machine is built when the component starts, constructed with the
 * context, and there is none before.
 */
class AskerComponent : com.sce.runtime.test.ComponentMachine {
    private var sceMachine: AskerStateMachine? = null

    override fun start(context: com.sce.runtime.test.ComponentContext) {
        // The session is named by the component, and named rather than drawn.
        // The name is `mtc` or the id of the `<invoke>` that creates the PTC
        // (`ComponentContext.create`, which creates nothing under a name its
        // configuration has no PTC of), and every `<invoke id>` is refused
        // where its document is parsed unless it is an XML Name, which is an
        // Nmtoken (docs/adr/0045). A component's machine always has a saved
        // shape, and so takes its session id: what would leave it none, a
        // delayed send or an invoke a restore cannot start, the profile
        // refuses.
        val machine = AskerStateMachine(sceTestContext = context, sessionId = context.name)
        // P6: a send on a port is appended to the queue of the port it is
        // connected to, or handed to the system adapter where it is mapped. A
        // test-port send always carries its template's value
        // (`forge::static_lowering`, `StaticTarget::test_message`).
        machine.registerEventProcessor("http://sce.dev/ext/test-port") { request ->
            context.send(
                request.target,
                com.sce.runtime.test.Message(
                    request.eventName,
                    checkNotNull(request.value) { "a test-port send carries its template's value" },
                ),
            )
            emptyList()
        }
        // P9: the start of a PTC's invocation creates and starts it, and its
        // cancel removes it, whether it is still running or already done.
        machine.registerInvoker("http://sce.dev/ext/test-component") { event ->
            event.start?.let { context.create(it.invokeId) }
            event.cancel?.let { context.kill(it.invokeId) }
            null
        }
        machine.initialize()
        sceMachine = machine
    }

    // §3.2 step 3: the round's snapshot, on which the machine takes the first
    // branch whose guard holds (P3) and completes its macrostep (P4), or takes
    // none and discards the event (§7). The owner's tick runs the macrostep
    // the event starts.
    override fun deliverSnapshot() {
        val machine = sceMachine ?: return
        machine.send(AskerEvent.Sce.Test.Snapshot)
        machine.tick()
    }

    // P10: the machine entered a top-level `<final>`.
    override fun terminated(): Boolean = sceMachine?.isInFinalState == true
}

class AskerStateMachine(
    /**
     * docs/SCE_TEST_PROFILE.md §12.3: the runtime context of the test
     * component this machine is, which its branches' guards read the round's
     * snapshot through and its statements call. Given by the component the
     * runtime drives (AskerComponent) when it builds the machine,
     * since the initial state's `<onentry>` may already start a timer or set a
     * verdict.
     */
    private val sceTestContext: com.sce.runtime.test.ComponentContext,
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<AskerState, AskerEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `job` datamodel variable, the machine's own. */
    private var vJob: UInt = 7.toUInt()
    /** W3C SCXML 5.2: the `answered` datamodel variable, the machine's own. */
    private var vAnswered: UInt = 0.toUInt()

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var job: UInt? = null
        var answered: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.job?.let { vJob = it }
        params.answered?.let { vAnswered = it }
    }

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<AskerState>,
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
        first = "a583b6e14d4fdbed507f361cb327e011d9c68c6d502d3bada074c55fd230d7f0",
        revised = "7566aa63b5407468bf2a9e5afee6994658d2b470ea6e1e4abd01cd92c6c1aa88",
        format2First = "a583b6e14d4fdbed507f361cb327e011d9c68c6d502d3bada074c55fd230d7f0",
        format2Revised = "7566aa63b5407468bf2a9e5afee6994658d2b470ea6e1e4abd01cd92c6c1aa88",
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
        variables += "job" to SavedValues.of(vJob)
        variables += "answered" to SavedValues.of(vAnswered)
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
        val saved1 = SavedValues.uint32(savedGrown.variable("job"), "job")
        val saved2 = SavedValues.uint32(savedGrown.variable("answered"), "answered")
        return {
            vJob = saved1
            vAnswered = saved2
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
        SavedValues.uint32(savedGrown.variable("job"), "job")
        SavedValues.uint32(savedGrown.variable("answered"), "answered")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingJobReplyPayload: AskerJobReplyPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: AskerEvent, metadata: EventMetadata) {
        pendingJobReplyPayload = null
        when (val tp = metadata.typedPayload) {
            is AskerJobReplyPayload -> pendingJobReplyPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == AskerEvent.Job.Reply) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingJobReplyPayload = AskerJobReplyPayload(fields.uint32("job"), fields.uint32("code"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `job.reply` — binds the event name and the payload field values in one call.
    fun raiseJobReply(job: UInt, code: UInt) {
        send(
            AskerEvent.Job.Reply,
            EventMetadata(
                type = "external",
                typedPayload = AskerJobReplyPayload(job, code),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("job" to job, "code" to code))
            )
        )
    }


    override val initialState: AskerState = AskerState.Asking

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
    override fun isFinalState(state: AskerState): Boolean = when (state) {
        is AskerState.Failed -> true
        is AskerState.Passed -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AskerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AskerState, HistoryId>> =
            listOf(StateTarget(AskerState.Asking))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: asking's transition 0, as the microstep reads it.
        val transitionAskingAt0 = EnabledTransition<AskerState, HistoryId>(
            AskerState.Asking,
            listOf(StateTarget(AskerState.Checking)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: asking's transition 1, as the microstep reads it.
        val transitionAskingAt1 = EnabledTransition<AskerState, HistoryId>(
            AskerState.Asking,
            listOf(StateTarget(AskerState.Failed)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: checking's transition 0, as the microstep reads it.
        val transitionCheckingAt0 = EnabledTransition<AskerState, HistoryId>(
            AskerState.Checking,
            listOf(StateTarget(AskerState.Passed)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: checking's transition 1, as the microstep reads it.
        val transitionCheckingAt1 = EnabledTransition<AskerState, HistoryId>(
            AskerState.Checking,
            listOf(StateTarget(AskerState.Failed)),
            1,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AskerState? = when (stateId) {
        "asking" -> AskerState.Asking
        "checking" -> AskerState.Checking
        "failed" -> AskerState.Failed
        "passed" -> AskerState.Passed
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AskerState): String = when (state) {
        is AskerState.Asking -> "asking"
        is AskerState.Checking -> "checking"
        is AskerState.Failed -> "failed"
        is AskerState.Passed -> "passed"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AskerState): Int = when (state) {
        is AskerState.Asking -> 0
        is AskerState.Checking -> 1
        is AskerState.Failed -> 3
        is AskerState.Passed -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AskerEvent? = when (name) {
        "error.execution" -> AskerEvent.Error.Execution
        "job.reply" -> AskerEvent.Job.Reply
        "sce.test.snapshot" -> AskerEvent.Sce.Test.Snapshot
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AskerEvent): String? = when (event) {
        is AskerEvent.Error.Execution -> "error.execution"
        is AskerEvent.Job.Reply -> "job.reply"
        is AskerEvent.Sce.Test.Snapshot -> "sce.test.snapshot"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: AskerState, event: AskerEvent?): EnabledTransition<AskerState, HistoryId>? = when (state) {
        is AskerState.Asking -> firstEnabledTransitionOfAsking(state, event)
        is AskerState.Checking -> firstEnabledTransitionOfChecking(state, event)
        else -> null
    }


    // asking's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAsking(state: AskerState, event: AskerEvent?): EnabledTransition<AskerState, HistoryId>? = when {
        event is AskerEvent.Sce.Test.Snapshot && com.sce.runtime.test.receives<com.sce.generated.reply_ok.Value>(sceTestContext, "sut", "job.reply", { sceMessage -> com.sce.generated.reply_ok.matches(sceMessage) }) -> TransitionsPart0.transitionAskingAt0
        event is AskerEvent.Sce.Test.Snapshot && sceTestContext.timedOut("reply") -> TransitionsPart0.transitionAskingAt1
        else -> null
    }


    // checking's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfChecking(state: AskerState, event: AskerEvent?): EnabledTransition<AskerState, HistoryId>? = when {
        event == null && vAnswered == vJob -> TransitionsPart0.transitionCheckingAt0
        event == null -> TransitionsPart0.transitionCheckingAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: asker.scxml:7 :: _machine
    override fun onEntry(state: AskerState, isDefaultEntry: Boolean) {
        when (state) {
            is AskerState.Asking -> onEntryOfAsking()
            is AskerState.Checking -> onEntryOfChecking()
            is AskerState.Failed -> onEntryOfFailed()
            is AskerState.Passed -> onEntryOfPassed()
        }
    }


    private fun onEntryOfAsking() {
                // SCE-MAP: asker.scxml:21 :: asking :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 6.2.5: "http://sce.dev/ext/test-port" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "http://sce.dev/ext/test-port",
                eventName = "job.request",
                target = "sut",
                content = "",
                params = sendWireParams,
                sendId = "__send_0",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId,
                // docs/SCE_TEST_PROFILE.md §12.3: a test component's message is
                // the value of its specific template, handed to the test-port
                // processor as a value and never as an encoding (docs/adr/0091,
                // C2). Every other send leaves it null and carries `eventData`.
                value = com.sce.generated.request_of.value(vJob)
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("http://sce.dev/ext/test-port")) {
                raisePlatformError(AskerEvent.Error.Execution, "<send type='http://sce.dev/ext/test-port'> names a processor the host declared but never registered", "__send_0")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)

            sceTestContext.startTimer("reply", 1000000L)
                }
    }

    private fun onEntryOfChecking() {
                // SCE-MAP: asker.scxml:36 :: checking :: _state_body
    }

    private fun onEntryOfFailed() {
                // SCE-MAP: asker.scxml:45 :: failed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPassed() {
                // SCE-MAP: asker.scxml:44 :: passed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: asker.scxml:7 :: _machine
    override fun onExit(state: AskerState) {
        when (state) {
            is AskerState.Asking -> onExitOfAsking()
            is AskerState.Checking -> onExitOfChecking()
            is AskerState.Failed -> onExitOfFailed()
            is AskerState.Passed -> onExitOfPassed()
        }
    }


    private fun onExitOfAsking() {
                // SCE-MAP: asker.scxml:21 :: asking :: _state_body
    }

    private fun onExitOfChecking() {
                // SCE-MAP: asker.scxml:36 :: checking :: _state_body
    }

    private fun onExitOfFailed() {
                // SCE-MAP: asker.scxml:45 :: failed :: _state_body
    }

    private fun onExitOfPassed() {
                // SCE-MAP: asker.scxml:44 :: passed :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: asker.scxml:7 :: _machine
    override fun executeTransitionContent(source: AskerState, transitionIndex: Int) {
        when (source) {
            is AskerState.Asking -> transitionContentOfAsking(transitionIndex)
            is AskerState.Checking -> transitionContentOfChecking(transitionIndex)
            else -> {}
        }
    }


    // The content of `asking`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfAsking(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfAskingAt0()
            1 -> transitionContentOfAskingAt1()
            else -> {}
        }
    }


    private fun transitionContentOfAskingAt0() {
                // SCE-MAP: asker.scxml:27 :: asking :: _transition_0
                // docs/SCE_TEST_PROFILE.md §12.3: taking the branch (P4 step 1).
                val sceMessage = sceTestContext.receive("sut")
                val sceValue = checkNotNull(sceMessage.valueAs<com.sce.generated.reply_ok.Value>()) { "a receive branch takes the message its guard matched" }
                pendingJobReplyPayload = AskerJobReplyPayload(job = sceValue.job, code = sceValue.code)

            sceTestContext.stopTimer("reply")

            vAnswered = pendingJobReplyPayload!!.job

            sceTestContext.log("answered: " + (pendingJobReplyPayload!!.job).toString())
    }

    private fun transitionContentOfAskingAt1() {
                // SCE-MAP: asker.scxml:32 :: asking :: _transition_1
                // docs/SCE_TEST_PROFILE.md §12.3: taking the branch (P4 step 1).
                sceTestContext.timeout("reply")

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.FAIL, "no matching reply in time")
    }

    // The content of `checking`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfChecking(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfCheckingAt0()
            1 -> transitionContentOfCheckingAt1()
            else -> {}
        }
    }


    private fun transitionContentOfCheckingAt0() {
                // SCE-MAP: asker.scxml:37 :: checking :: _transition_0

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.PASS, "")
    }

    private fun transitionContentOfCheckingAt1() {
                // SCE-MAP: asker.scxml:40 :: checking :: _transition_1

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.FAIL, "the reply answers another job")
    }
}
