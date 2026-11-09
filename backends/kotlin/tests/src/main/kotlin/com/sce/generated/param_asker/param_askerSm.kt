// SCE-GENERATED — DO NOT EDIT
// source-hash: 16d76c0f9ed42e7e13706b7e291287d0861ab6ac1014d873b4e04ff755d59e01

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/test_profile/runtime/param_asker.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: param_asker.scxml:6 :: _machine

package com.sce.generated.param_asker

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ParamAskerState : State {
    data object Asking : ParamAskerState
    data object Checking : ParamAskerState
    data object Failed : ParamAskerState
    data object Passed : ParamAskerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ParamAskerEvent : Event {
    sealed interface Error : ParamAskerEvent {
        data object Execution : Error
    }
    sealed interface Job : ParamAskerEvent {
        data object Reply : Job
    }
    sealed interface Sce : ParamAskerEvent {
        sealed interface Test : Sce {
            data object Snapshot : Test
        }
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `ParamAskerPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// ParamAskerJobReplyPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `job.reply`. Consumers inject it via the `raiseJobReply` seam
// on the machine — they never name this class directly.
data class ParamAskerJobReplyPayload(val job: UInt, val code: UInt)


// --- State Machine (W3C SCXML) ---

// ======================================================================
// The test component the profile's runtime drives (docs/SCE_TEST_PROFILE.md §12.3)
// ======================================================================

/**
 * The test component `param_asker` as the profile's runtime drives it
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
class ParamAskerComponent : com.sce.runtime.test.ComponentMachine {
    private var sceMachine: ParamAskerStateMachine? = null

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
        val machine = ParamAskerStateMachine(sceTestContext = context, sessionId = context.name)
        // §11.5, §11.8: the module parameters take the suite's values before the
        // machine starts. Without one, or with one not of its type, the run ends
        // in error (P13), and the machine is not started on a value made up in
        // its place.
        if (!context.readModuleParameters { given -> machine.acceptModuleParameters(given) }) {
            return
        }
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
        machine.send(ParamAskerEvent.Sce.Test.Snapshot)
        machine.tick()
    }

    // P10: the machine entered a top-level `<final>`.
    override fun terminated(): Boolean = sceMachine?.isInFinalState == true
}

class ParamAskerStateMachine(
    /**
     * docs/SCE_TEST_PROFILE.md §12.3: the runtime context of the test
     * component this machine is, which its branches' guards read the round's
     * snapshot through and its statements call. Given by the component the
     * runtime drives (ParamAskerComponent) when it builds the machine,
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
) : StateMachineEngine<ParamAskerState, ParamAskerEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /**
     * docs/SCE_TEST_PROFILE.md §11.5: the module parameter `job`, which takes the
     * suite's value before the machine starts.
     */
    private var vJob: UInt = 0u
    /** W3C SCXML 5.2: the `answered` datamodel variable, the machine's own. */
    private var vAnswered: UInt = 0.toUInt()

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var answered: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.answered?.let { vAnswered = it }
    }

    /**
     * docs/SCE_TEST_PROFILE.md §11.5, §11.8: each module parameter takes the
     * value the suite gives it, read as a saved value of its declared type is
     * read back, before the machine starts. Called by the component the
     * profile's runtime drives (ParamAskerComponent), before [initialize].
     *
     * @throws StateRefusal naming the module parameter whose value is not given,
     *   not of its type or past its bound; nothing is converted.
     */
    internal fun acceptModuleParameters(given: com.sce.runtime.test.ModuleParameters) {
        acceptModuleParametersPart0(given)
    }

    private fun acceptModuleParametersPart0(given: com.sce.runtime.test.ModuleParameters) {
        vJob = SavedValues.uint32(given.of("job"), "job")
    }

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<ParamAskerState>,
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
    private var pendingJobReplyPayload: ParamAskerJobReplyPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: ParamAskerEvent, metadata: EventMetadata) {
        pendingJobReplyPayload = null
        when (val tp = metadata.typedPayload) {
            is ParamAskerJobReplyPayload -> pendingJobReplyPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == ParamAskerEvent.Job.Reply) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingJobReplyPayload = ParamAskerJobReplyPayload(fields.uint32("job"), fields.uint32("code"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `job.reply` — binds the event name and the payload field values in one call.
    fun raiseJobReply(job: UInt, code: UInt) {
        send(
            ParamAskerEvent.Job.Reply,
            EventMetadata(
                type = "external",
                typedPayload = ParamAskerJobReplyPayload(job, code),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("job" to job, "code" to code))
            )
        )
    }


    override val initialState: ParamAskerState = ParamAskerState.Asking

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
    override fun isFinalState(state: ParamAskerState): Boolean = when (state) {
        is ParamAskerState.Failed -> true
        is ParamAskerState.Passed -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ParamAskerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<ParamAskerState, HistoryId>> =
            listOf(StateTarget(ParamAskerState.Asking))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: asking's transition 0, as the microstep reads it.
        val transitionAskingAt0 = EnabledTransition<ParamAskerState, HistoryId>(
            ParamAskerState.Asking,
            listOf(StateTarget(ParamAskerState.Checking)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: asking's transition 1, as the microstep reads it.
        val transitionAskingAt1 = EnabledTransition<ParamAskerState, HistoryId>(
            ParamAskerState.Asking,
            listOf(StateTarget(ParamAskerState.Failed)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: checking's transition 0, as the microstep reads it.
        val transitionCheckingAt0 = EnabledTransition<ParamAskerState, HistoryId>(
            ParamAskerState.Checking,
            listOf(StateTarget(ParamAskerState.Passed)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: checking's transition 1, as the microstep reads it.
        val transitionCheckingAt1 = EnabledTransition<ParamAskerState, HistoryId>(
            ParamAskerState.Checking,
            listOf(StateTarget(ParamAskerState.Failed)),
            1,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ParamAskerState? = when (stateId) {
        "asking" -> ParamAskerState.Asking
        "checking" -> ParamAskerState.Checking
        "failed" -> ParamAskerState.Failed
        "passed" -> ParamAskerState.Passed
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ParamAskerState): String = when (state) {
        is ParamAskerState.Asking -> "asking"
        is ParamAskerState.Checking -> "checking"
        is ParamAskerState.Failed -> "failed"
        is ParamAskerState.Passed -> "passed"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ParamAskerState): Int = when (state) {
        is ParamAskerState.Asking -> 0
        is ParamAskerState.Checking -> 1
        is ParamAskerState.Failed -> 3
        is ParamAskerState.Passed -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ParamAskerEvent? = when (name) {
        "error.execution" -> ParamAskerEvent.Error.Execution
        "job.reply" -> ParamAskerEvent.Job.Reply
        "sce.test.snapshot" -> ParamAskerEvent.Sce.Test.Snapshot
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ParamAskerEvent): String? = when (event) {
        is ParamAskerEvent.Error.Execution -> "error.execution"
        is ParamAskerEvent.Job.Reply -> "job.reply"
        is ParamAskerEvent.Sce.Test.Snapshot -> "sce.test.snapshot"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: ParamAskerState, event: ParamAskerEvent?): EnabledTransition<ParamAskerState, HistoryId>? = when (state) {
        is ParamAskerState.Asking -> firstEnabledTransitionOfAsking(state, event)
        is ParamAskerState.Checking -> firstEnabledTransitionOfChecking(state, event)
        else -> null
    }


    // asking's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAsking(state: ParamAskerState, event: ParamAskerEvent?): EnabledTransition<ParamAskerState, HistoryId>? = when {
        event is ParamAskerEvent.Sce.Test.Snapshot && com.sce.runtime.test.receives<com.sce.generated.reply_ok.Value>(sceTestContext, "sut", "job.reply", { sceMessage -> com.sce.generated.reply_ok.matches(sceMessage) }) -> TransitionsPart0.transitionAskingAt0
        event is ParamAskerEvent.Sce.Test.Snapshot && sceTestContext.timedOut("deadline") -> TransitionsPart0.transitionAskingAt1
        else -> null
    }


    // checking's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfChecking(state: ParamAskerState, event: ParamAskerEvent?): EnabledTransition<ParamAskerState, HistoryId>? = when {
        event == null && vAnswered == vJob -> TransitionsPart0.transitionCheckingAt0
        event == null -> TransitionsPart0.transitionCheckingAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: param_asker.scxml:6 :: _machine
    override fun onEntry(state: ParamAskerState, isDefaultEntry: Boolean) {
        when (state) {
            is ParamAskerState.Asking -> onEntryOfAsking()
            is ParamAskerState.Checking -> onEntryOfChecking()
            is ParamAskerState.Failed -> onEntryOfFailed()
            is ParamAskerState.Passed -> onEntryOfPassed()
        }
    }


    private fun onEntryOfAsking() {
                // SCE-MAP: param_asker.scxml:20 :: asking :: _state_body
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
                raisePlatformError(ParamAskerEvent.Error.Execution, "<send type='http://sce.dev/ext/test-port'> names a processor the host declared but never registered", "__send_0")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)

            sceTestContext.startTimer("deadline", 1000000L)
                }
    }

    private fun onEntryOfChecking() {
                // SCE-MAP: param_asker.scxml:34 :: checking :: _state_body
    }

    private fun onEntryOfFailed() {
                // SCE-MAP: param_asker.scxml:43 :: failed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPassed() {
                // SCE-MAP: param_asker.scxml:42 :: passed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: param_asker.scxml:6 :: _machine
    override fun onExit(state: ParamAskerState) {
        when (state) {
            is ParamAskerState.Asking -> onExitOfAsking()
            is ParamAskerState.Checking -> onExitOfChecking()
            is ParamAskerState.Failed -> onExitOfFailed()
            is ParamAskerState.Passed -> onExitOfPassed()
        }
    }


    private fun onExitOfAsking() {
                // SCE-MAP: param_asker.scxml:20 :: asking :: _state_body
    }

    private fun onExitOfChecking() {
                // SCE-MAP: param_asker.scxml:34 :: checking :: _state_body
    }

    private fun onExitOfFailed() {
                // SCE-MAP: param_asker.scxml:43 :: failed :: _state_body
    }

    private fun onExitOfPassed() {
                // SCE-MAP: param_asker.scxml:42 :: passed :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: param_asker.scxml:6 :: _machine
    override fun executeTransitionContent(source: ParamAskerState, transitionIndex: Int) {
        when (source) {
            is ParamAskerState.Asking -> transitionContentOfAsking(transitionIndex)
            is ParamAskerState.Checking -> transitionContentOfChecking(transitionIndex)
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
                // SCE-MAP: param_asker.scxml:26 :: asking :: _transition_0
                // docs/SCE_TEST_PROFILE.md §12.3: taking the branch (P4 step 1).
                val sceMessage = sceTestContext.receive("sut")
                val sceValue = checkNotNull(sceMessage.valueAs<com.sce.generated.reply_ok.Value>()) { "a receive branch takes the message its guard matched" }
                pendingJobReplyPayload = ParamAskerJobReplyPayload(job = sceValue.job, code = sceValue.code)

            sceTestContext.stopTimer("deadline")

            vAnswered = pendingJobReplyPayload!!.job
    }

    private fun transitionContentOfAskingAt1() {
                // SCE-MAP: param_asker.scxml:30 :: asking :: _transition_1
                // docs/SCE_TEST_PROFILE.md §12.3: taking the branch (P4 step 1).
                sceTestContext.timeout("deadline")

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
                // SCE-MAP: param_asker.scxml:35 :: checking :: _transition_0

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.PASS, "")
    }

    private fun transitionContentOfCheckingAt1() {
                // SCE-MAP: param_asker.scxml:38 :: checking :: _transition_1

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.FAIL, "the reply answers another job")
    }
}
