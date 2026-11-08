// SCE-GENERATED — DO NOT EDIT
// source-hash: 011d804f887126b888ea6ba5b4213d56e0f5c8740555a154c83e9b8d0105c4b6

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/test_profile/runtime/coordinator.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: coordinator.scxml:6 :: _machine

package com.sce.generated.coordinator

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface CoordinatorState : State {
    data object Passed : CoordinatorState
    data object Served : CoordinatorState
    data object Serving : CoordinatorState
    data object Working : CoordinatorState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface CoordinatorEvent : Event {
    sealed interface Done : CoordinatorEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Answerer : Invoke
        }
    }
    sealed interface Error : CoordinatorEvent {
        data object Execution : Error
    }
    sealed interface Job : CoordinatorEvent {
        data object Request : Job
    }
    sealed interface Sce : CoordinatorEvent {
        sealed interface Test : Sce {
            data object Snapshot : Test
        }
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `CoordinatorPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// CoordinatorJobRequestPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `job.request`. Consumers inject it via the `raiseJobRequest` seam
// on the machine — they never name this class directly.
data class CoordinatorJobRequestPayload(val job: UInt)


// --- State Machine (W3C SCXML) ---

// ======================================================================
// The test component the profile's runtime drives (docs/SCE_TEST_PROFILE.md §12.3)
// ======================================================================

/**
 * The test component `coordinator` as the profile's runtime drives it
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
class CoordinatorComponent : com.sce.runtime.test.ComponentMachine {
    private var sceMachine: CoordinatorStateMachine? = null

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
        val machine = CoordinatorStateMachine(sceTestContext = context, sessionId = context.name)
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
        machine.send(CoordinatorEvent.Sce.Test.Snapshot)
        machine.tick()
    }

    // P10: the machine entered a top-level `<final>`.
    override fun terminated(): Boolean = sceMachine?.isInFinalState == true
}

class CoordinatorStateMachine(
    /**
     * docs/SCE_TEST_PROFILE.md §12.3: the runtime context of the test
     * component this machine is, which its branches' guards read the round's
     * snapshot through and its statements call. Given by the component the
     * runtime drives (CoordinatorComponent) when it builds the machine,
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
) : StateMachineEngine<CoordinatorState, CoordinatorEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `job` datamodel variable, the machine's own. */
    private var vJob: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `ok` datamodel variable, the machine's own. */
    private var vOk: UInt = 0.toUInt()

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var job: UInt? = null
        var ok: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.job?.let { vJob = it }
        params.ok?.let { vOk = it }
    }

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<CoordinatorState>,
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
        first = "734e400812e9cc397a661eba66e879e6c39d82f80c7bf1464405daaf18c30178",
        revised = "98b0941af6c46fbc364590822bc4e5940fdeff475cb833ebc71f32641045e1de",
        format2First = "734e400812e9cc397a661eba66e879e6c39d82f80c7bf1464405daaf18c30178",
        format2Revised = "98b0941af6c46fbc364590822bc4e5940fdeff475cb833ebc71f32641045e1de",
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
        variables += "ok" to SavedValues.of(vOk)
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
        val saved2 = SavedValues.uint32(savedGrown.variable("ok"), "ok")
        return {
            vJob = saved1
            vOk = saved2
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
        SavedValues.uint32(savedGrown.variable("ok"), "ok")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, CoordinatorState>> = listOf(
        Triple("http://sce.dev/ext/test-component", "answerer", CoordinatorState.Working),
    )

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingJobRequestPayload: CoordinatorJobRequestPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: CoordinatorEvent, metadata: EventMetadata) {
        pendingJobRequestPayload = null
        when (val tp = metadata.typedPayload) {
            is CoordinatorJobRequestPayload -> pendingJobRequestPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == CoordinatorEvent.Job.Request) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingJobRequestPayload = CoordinatorJobRequestPayload(fields.uint32("job"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `job.request` — binds the event name and the payload field values in one call.
    fun raiseJobRequest(job: UInt) {
        send(
            CoordinatorEvent.Job.Request,
            EventMetadata(
                type = "external",
                typedPayload = CoordinatorJobRequestPayload(job),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("job" to job))
            )
        )
    }


    override val initialState: CoordinatorState = CoordinatorState.Serving

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

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: CoordinatorState): CoordinatorState? = when (state) {
        is CoordinatorState.Served -> CoordinatorState.Working
        is CoordinatorState.Serving -> CoordinatorState.Working
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: CoordinatorState): Boolean = when (state) {
        is CoordinatorState.Working -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: CoordinatorState): Boolean = when (state) {
        is CoordinatorState.Passed -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: CoordinatorState): List<CoordinatorState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: CoordinatorState): List<EntryTarget<CoordinatorState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<CoordinatorState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<CoordinatorState, List<CoordinatorState>> = LinkedHashMap<CoordinatorState, List<CoordinatorState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<CoordinatorState, List<CoordinatorState>>) {
            table[CoordinatorState.Working] = listOf(CoordinatorState.Serving, CoordinatorState.Served)
        }


        val initialTargets: Map<CoordinatorState, List<EntryTarget<CoordinatorState, HistoryId>>> = LinkedHashMap<CoordinatorState, List<EntryTarget<CoordinatorState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<CoordinatorState, List<EntryTarget<CoordinatorState, HistoryId>>>) {
            table[CoordinatorState.Working] = listOf(StateTarget(CoordinatorState.Serving))
        }


        val documentInitialTargetList: List<EntryTarget<CoordinatorState, HistoryId>> =
            listOf(StateTarget(CoordinatorState.Working))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: serving's transition 0, as the microstep reads it.
        val transitionServingAt0 = EnabledTransition<CoordinatorState, HistoryId>(
            CoordinatorState.Serving,
            listOf(StateTarget(CoordinatorState.Served)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<CoordinatorState, HistoryId>(
            CoordinatorState.Working,
            listOf(StateTarget(CoordinatorState.Passed)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): CoordinatorState? = when (stateId) {
        "passed" -> CoordinatorState.Passed
        "served" -> CoordinatorState.Served
        "serving" -> CoordinatorState.Serving
        "working" -> CoordinatorState.Working
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: CoordinatorState): String = when (state) {
        is CoordinatorState.Passed -> "passed"
        is CoordinatorState.Served -> "served"
        is CoordinatorState.Serving -> "serving"
        is CoordinatorState.Working -> "working"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: CoordinatorState): Int = when (state) {
        is CoordinatorState.Passed -> 3
        is CoordinatorState.Served -> 2
        is CoordinatorState.Serving -> 1
        is CoordinatorState.Working -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): CoordinatorEvent? = when (name) {
        "done.invoke" -> CoordinatorEvent.Done.Invoke.Self
        "done.invoke.answerer" -> CoordinatorEvent.Done.Invoke.Answerer
        "error.execution" -> CoordinatorEvent.Error.Execution
        "job.request" -> CoordinatorEvent.Job.Request
        "sce.test.snapshot" -> CoordinatorEvent.Sce.Test.Snapshot
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: CoordinatorEvent): String? = when (event) {
        is CoordinatorEvent.Done.Invoke.Self -> "done.invoke"
        is CoordinatorEvent.Done.Invoke.Answerer -> "done.invoke.answerer"
        is CoordinatorEvent.Error.Execution -> "error.execution"
        is CoordinatorEvent.Job.Request -> "job.request"
        is CoordinatorEvent.Sce.Test.Snapshot -> "sce.test.snapshot"
    }


    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "answerer",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: CoordinatorState, event: CoordinatorEvent?): EnabledTransition<CoordinatorState, HistoryId>? = when (state) {
        is CoordinatorState.Serving -> firstEnabledTransitionOfServing(state, event)
        is CoordinatorState.Working -> firstEnabledTransitionOfWorking(state, event)
        else -> null
    }


    // serving's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfServing(state: CoordinatorState, event: CoordinatorEvent?): EnabledTransition<CoordinatorState, HistoryId>? = when {
        event is CoordinatorEvent.Sce.Test.Snapshot && com.sce.runtime.test.receives<com.sce.generated.any_request.Value>(sceTestContext, "peer", "job.request", { sceMessage -> com.sce.generated.any_request.matches(sceMessage) }) -> TransitionsPart0.transitionServingAt0
        else -> null
    }


    // working's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWorking(state: CoordinatorState, event: CoordinatorEvent?): EnabledTransition<CoordinatorState, HistoryId>? = when {
        event is CoordinatorEvent.Sce.Test.Snapshot && sceTestContext.done("answerer") -> TransitionsPart0.transitionWorkingAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: coordinator.scxml:6 :: _machine
    override fun onEntry(state: CoordinatorState, isDefaultEntry: Boolean) {
        when (state) {
            is CoordinatorState.Passed -> onEntryOfPassed()
            is CoordinatorState.Served -> onEntryOfServed()
            is CoordinatorState.Serving -> onEntryOfServing()
            is CoordinatorState.Working -> onEntryOfWorking()
        }
    }


    private fun onEntryOfPassed() {
                // SCE-MAP: coordinator.scxml:33 :: passed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfServed() {
                // SCE-MAP: coordinator.scxml:31 :: served :: _state_body
    }

    private fun onEntryOfServing() {
                // SCE-MAP: coordinator.scxml:24 :: serving :: _state_body
    }

    private fun onEntryOfWorking() {
                // SCE-MAP: coordinator.scxml:19 :: working :: _state_body
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "working.${System.identityHashCode(this)}.answerer"
                    deferInvoke(CoordinatorState.Working, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "http://sce.dev/ext/test-component",
                                invokeId = "answerer",
                                src = "answerer.scxml",
                                params = hostInvokeParams,
                                eventData = if (hostInvokePayload.isEmpty()) "" else buildJsonFromParams(hostInvokePayload),
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(CoordinatorEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: coordinator.scxml:6 :: _machine
    override fun onExit(state: CoordinatorState) {
        when (state) {
            is CoordinatorState.Passed -> onExitOfPassed()
            is CoordinatorState.Served -> onExitOfServed()
            is CoordinatorState.Serving -> onExitOfServing()
            is CoordinatorState.Working -> onExitOfWorking()
        }
    }


    private fun onExitOfPassed() {
                // SCE-MAP: coordinator.scxml:33 :: passed :: _state_body
    }

    private fun onExitOfServed() {
                // SCE-MAP: coordinator.scxml:31 :: served :: _state_body
    }

    private fun onExitOfServing() {
                // SCE-MAP: coordinator.scxml:24 :: serving :: _state_body
    }

    private fun onExitOfWorking() {
                // SCE-MAP: coordinator.scxml:19 :: working :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(CoordinatorState.Working)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("http://sce.dev/ext/test-component", "answerer")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: coordinator.scxml:6 :: _machine
    override fun executeTransitionContent(source: CoordinatorState, transitionIndex: Int) {
        when (source) {
            is CoordinatorState.Serving -> transitionContentOfServing(transitionIndex)
            is CoordinatorState.Working -> transitionContentOfWorking(transitionIndex)
            else -> {}
        }
    }


    // The content of `serving`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfServing(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfServingAt0()
            else -> {}
        }
    }


    private fun transitionContentOfServingAt0() {
                // SCE-MAP: coordinator.scxml:25 :: serving :: _transition_0
                // docs/SCE_TEST_PROFILE.md §12.3: taking the branch (P4 step 1).
                val sceMessage = sceTestContext.receive("peer")
                val sceValue = checkNotNull(sceMessage.valueAs<com.sce.generated.any_request.Value>()) { "a receive branch takes the message its guard matched" }
                pendingJobRequestPayload = CoordinatorJobRequestPayload(job = sceValue.job)

            vJob = pendingJobRequestPayload!!.job


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
                eventName = "job.reply",
                target = "peer",
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
                value = com.sce.generated.reply_of.value(vJob, vOk)
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("http://sce.dev/ext/test-port")) {
                raisePlatformError(CoordinatorEvent.Error.Execution, "<send type='http://sce.dev/ext/test-port'> names a processor the host declared but never registered", "__send_0")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    // The content of `working`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWorking(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWorkingAt0()
            else -> {}
        }
    }


    private fun transitionContentOfWorkingAt0() {
                // SCE-MAP: coordinator.scxml:21 :: working :: _transition_0
                // docs/SCE_TEST_PROFILE.md §12.3: taking the branch (P4 step 1).
                sceTestContext.doneTaken("answerer")

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.PASS, "")
    }
}
