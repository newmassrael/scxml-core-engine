// SCE-GENERATED — DO NOT EDIT
// source-hash: 9a457df8c2ad1e85f1a27bced7a3bd97e3f8334eac4c5c367983c13b36d0f81f
// source: job_tester.scxml

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/test_profile/job_tester.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: job_tester.scxml:7 :: _machine

package com.sce.generated.job_tester

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface JobTesterState : State {
    data object Failed : JobTesterState
    data object Passed : JobTesterState
    data object Requesting : JobTesterState
    data object Running : JobTesterState
    data object Settling : JobTesterState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface JobTesterEvent : Event {
    sealed interface Done : JobTesterEvent {
        data object Invoke : Done
    }
    sealed interface Error : JobTesterEvent {
        data object Execution : Error
    }
    sealed interface Job : JobTesterEvent {
        data object Status : Job
    }
    sealed interface Sce : JobTesterEvent {
        sealed interface Test : Sce {
            data object Snapshot : Test
        }
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `JobTesterPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// JobTesterJobStatusPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `job.status`. Consumers inject it via the `raiseJobStatus` seam
// on the machine — they never name this class directly.
data class JobTesterJobStatusPayload(val elapsed_ms: UInt, val note: String?, val codes: List<UByte>, val tag: UByte?, val retry: UByte?, val count: UInt) {
    init {
        com.sce.runtime.ValueBounds.optional("note", note) { scePath0, sceValue0 -> com.sce.runtime.ValueBounds.text(scePath0, sceValue0, 32) }
        com.sce.runtime.ValueBounds.list("codes", codes, 4) { _, _ -> }
    }
}


// --- State Machine (W3C SCXML) ---

// ======================================================================
// The test component the profile's runtime drives (docs/SCE_TEST_PROFILE.md §12.3)
// ======================================================================

/**
 * The test component `job_tester` as the profile's runtime drives it
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
class JobTesterComponent : com.sce.runtime.test.ComponentMachine {
    private var sceMachine: JobTesterStateMachine? = null

    override fun start(context: com.sce.runtime.test.ComponentContext) {
        // The session is named by the component, and named rather than drawn.
        // The name is `mtc` or the id of the `<invoke>` that creates the PTC
        // (`ComponentContext.create`, which creates nothing under a name its
        // configuration has no PTC of), and every `<invoke id>` is refused
        // where its document is parsed unless it is an XML Name, which is an
        // Nmtoken (docs/adr/0045). Every `sce-static` root takes the session id
        // its host gives, whether or not it saves (docs/adr/0045, answer 2), so
        // the component's machine is named by the component.
        val machine = JobTesterStateMachine(sceTestContext = context, sessionId = context.name)
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
        machine.send(JobTesterEvent.Sce.Test.Snapshot)
        machine.tick()
    }

    // P10: the machine entered a top-level `<final>`.
    override fun terminated(): Boolean = sceMachine?.isInFinalState == true
}

class JobTesterStateMachine(
    /**
     * docs/SCE_TEST_PROFILE.md §12.3: the runtime context of the test
     * component this machine is, which its branches' guards read the round's
     * snapshot through and its statements call. Given by the component the
     * runtime drives (JobTesterComponent) when it builds the machine,
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
) : StateMachineEngine<JobTesterState, JobTesterEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /**
     * docs/SCE_TEST_PROFILE.md §11.5: the module parameter `elapsed_limit`, which takes the
     * suite's value before the machine starts.
     */
    private var vElapsedLimit: UInt = 0u
    /** W3C SCXML 5.2: the `elapsed` datamodel variable, the machine's own. */
    private var vElapsed: UInt = 0.toUInt()

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var elapsed: UInt? = null
    }

    /**
     * §scxml-6.4: give this machine the values [params] carries, each as the
     * initial value of its variable in place of the one its `<data>` gives, so
     * that a variable declared after it whose initial value reads it reads the
     * handed value (docs/adr/0098). Called before [initialize].
     */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    @Suppress("UNUSED_PARAMETER")
    private fun acceptParamsPart0(params: InvokeParams) {
        vElapsed = params.elapsed ?: (0.toUInt())
    }

    /**
     * docs/SCE_TEST_PROFILE.md §11.5, §11.8: each module parameter takes the
     * value the suite gives it, read as a saved value of its declared type is
     * read back, before the machine starts. Called by the component the
     * profile's runtime drives (JobTesterComponent), before [initialize].
     *
     * @throws StateRefusal naming the module parameter whose value is not given,
     *   not of its type or past its bound; nothing is converted.
     */
    internal fun acceptModuleParameters(given: com.sce.runtime.test.ModuleParameters) {
        acceptModuleParametersPart0(given)
    }

    private fun acceptModuleParametersPart0(given: com.sce.runtime.test.ModuleParameters) {
        vElapsedLimit = SavedValues.uint32(given.of("elapsed_limit"), "elapsed_limit")
    }

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<JobTesterState>,
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
        first = "689ce8cee02eb78271b7c9143d396966a18b74c4702ca715d4e2d8228d45343c",
        revised = "18ae334a0a37446d1a771fdde3dae61652c94ca824c2e4f820d7f81c4e659242",
        format2First = "689ce8cee02eb78271b7c9143d396966a18b74c4702ca715d4e2d8228d45343c",
        format2Revised = "18ae334a0a37446d1a771fdde3dae61652c94ca824c2e4f820d7f81c4e659242",
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
        variables += "elapsed_limit" to SavedValues.of(vElapsedLimit)
        variables += "elapsed" to SavedValues.of(vElapsed)
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
        // Every part is judged before any is written, so a refused value leaves
        // the machine as it was; then each part reads its values again and
        // writes them. A part answers no closure over the values it read: a
        // closure's captures are the arguments of a JVM method, at most 255
        // slots with a `Long` or a `Double` taking two, and a part of
        // `kotlin_method_items` variables passed that, so the class failed to
        // load (`ClassFormatError: Too many arguments in method signature`).
        judgeVariablesPart0(savedGrown)
        restoreVariablesPart0(savedGrown)
        enterSaved(savedGrown, wallNowMs)
    }

    // Writes this part's variables from [savedGrown], which every judge part
    // has already held to its variables' types.
    private fun restoreVariablesPart0(savedGrown: SavedState) {
        vElapsedLimit = SavedValues.uint32(savedGrown.variable("elapsed_limit"), "elapsed_limit")
        vElapsed = SavedValues.uint32(savedGrown.variable("elapsed"), "elapsed")
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
        SavedValues.uint32(savedGrown.variable("elapsed_limit"), "elapsed_limit")
        SavedValues.uint32(savedGrown.variable("elapsed"), "elapsed")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, JobTesterState>> = listOf(
        Triple("http://sce.dev/ext/test-component", "helper", JobTesterState.Running),
    )

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingJobStatusPayload: JobTesterJobStatusPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, on which the engine refuses the delivery whole and
    // takes an error.execution in its place (docs/adr/0086, section 4.2). Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: JobTesterEvent, metadata: EventMetadata) {
        pendingJobStatusPayload = null
        when (val tp = metadata.typedPayload) {
            is JobTesterJobStatusPayload -> pendingJobStatusPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == JobTesterEvent.Job.Status) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingJobStatusPayload = JobTesterJobStatusPayload(fields.field("elapsed_ms").let { it.uint32() }, fields.field("note").let { it.optional { it.string(32) } }, fields.field("codes").let { it.list(4) { it.uint8() } }, fields.field("tag").let { it.optional { it.uint8() } }, fields.field("retry").let { it.optional { it.uint8() } }, fields.field("count").let { it.uint32() })
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `job.status` — binds the event name and the payload field values in one call.
    fun raiseJobStatus(elapsed_ms: UInt, note: String?, codes: List<UByte>, tag: UByte?, retry: UByte?, count: UInt) {
        val scePayload = JobTesterJobStatusPayload(elapsed_ms, note, codes, tag, retry, count)
        send(
            JobTesterEvent.Job.Status,
            EventMetadata(
                type = "external",
                typedPayload = scePayload,
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("elapsed_ms" to com.sce.runtime.EventPayload.valueJson(scePayload.elapsed_ms), "note" to (scePayload.note?.let { com.sce.runtime.EventPayload.valueJson(it) } ?: com.sce.runtime.EventPayload.valueJson(null)), "codes" to com.sce.runtime.EventPayload.listJson(scePayload.codes) { com.sce.runtime.EventPayload.valueJson(it) }, "tag" to (scePayload.tag?.let { com.sce.runtime.EventPayload.valueJson(it) } ?: com.sce.runtime.EventPayload.valueJson(null)), "retry" to (scePayload.retry?.let { com.sce.runtime.EventPayload.valueJson(it) } ?: com.sce.runtime.EventPayload.valueJson(null)), "count" to com.sce.runtime.EventPayload.valueJson(scePayload.count))).json
            )
        )
    }


    override val initialState: JobTesterState = JobTesterState.Requesting

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
    override fun parentOf(state: JobTesterState): JobTesterState? = when (state) {
        is JobTesterState.Requesting -> JobTesterState.Running
        is JobTesterState.Settling -> JobTesterState.Running
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: JobTesterState): Boolean = when (state) {
        is JobTesterState.Running -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: JobTesterState): Boolean = when (state) {
        is JobTesterState.Failed -> true
        is JobTesterState.Passed -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: JobTesterState): List<JobTesterState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: JobTesterState): List<EntryTarget<JobTesterState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<JobTesterState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<JobTesterState, List<JobTesterState>> = LinkedHashMap<JobTesterState, List<JobTesterState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<JobTesterState, List<JobTesterState>>) {
            table[JobTesterState.Running] = listOf(JobTesterState.Requesting, JobTesterState.Settling)
        }


        val initialTargets: Map<JobTesterState, List<EntryTarget<JobTesterState, HistoryId>>> = LinkedHashMap<JobTesterState, List<EntryTarget<JobTesterState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<JobTesterState, List<EntryTarget<JobTesterState, HistoryId>>>) {
            table[JobTesterState.Running] = listOf(StateTarget(JobTesterState.Requesting))
        }


        val documentInitialTargetList: List<EntryTarget<JobTesterState, HistoryId>> =
            listOf(StateTarget(JobTesterState.Running))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: requesting's transition 0, as the microstep reads it.
        val transitionRequestingAt0 = EnabledTransition<JobTesterState, HistoryId>(
            JobTesterState.Requesting,
            listOf(StateTarget(JobTesterState.Settling)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: requesting's transition 1, as the microstep reads it.
        val transitionRequestingAt1 = EnabledTransition<JobTesterState, HistoryId>(
            JobTesterState.Requesting,
            listOf(StateTarget(JobTesterState.Failed)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: settling's transition 0, as the microstep reads it.
        val transitionSettlingAt0 = EnabledTransition<JobTesterState, HistoryId>(
            JobTesterState.Settling,
            listOf(StateTarget(JobTesterState.Passed)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): JobTesterState? = when (stateId) {
        "failed" -> JobTesterState.Failed
        "passed" -> JobTesterState.Passed
        "requesting" -> JobTesterState.Requesting
        "running" -> JobTesterState.Running
        "settling" -> JobTesterState.Settling
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: JobTesterState): String = when (state) {
        is JobTesterState.Failed -> "failed"
        is JobTesterState.Passed -> "passed"
        is JobTesterState.Requesting -> "requesting"
        is JobTesterState.Running -> "running"
        is JobTesterState.Settling -> "settling"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: JobTesterState): Int = when (state) {
        is JobTesterState.Failed -> 4
        is JobTesterState.Passed -> 3
        is JobTesterState.Requesting -> 1
        is JobTesterState.Running -> 0
        is JobTesterState.Settling -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): JobTesterEvent? = when (name) {
        "done.invoke" -> JobTesterEvent.Done.Invoke
        "error.execution" -> JobTesterEvent.Error.Execution
        "job.status" -> JobTesterEvent.Job.Status
        "sce.test.snapshot" -> JobTesterEvent.Sce.Test.Snapshot
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: JobTesterEvent): String? = when (event) {
        is JobTesterEvent.Done.Invoke -> "done.invoke"
        is JobTesterEvent.Error.Execution -> "error.execution"
        is JobTesterEvent.Job.Status -> "job.status"
        is JobTesterEvent.Sce.Test.Snapshot -> "sce.test.snapshot"
    }


    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "helper",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: JobTesterState, event: JobTesterEvent?): EnabledTransition<JobTesterState, HistoryId>? = when (state) {
        is JobTesterState.Requesting -> firstEnabledTransitionOfRequesting(state, event)
        is JobTesterState.Settling -> firstEnabledTransitionOfSettling(state, event)
        else -> null
    }


    // requesting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRequesting(state: JobTesterState, event: JobTesterEvent?): EnabledTransition<JobTesterState, HistoryId>? = when {
        event is JobTesterEvent.Sce.Test.Snapshot && com.sce.runtime.test.receives<com.sce.generated.every_matcher.Value>(sceTestContext, "sut", "job.status", { sceMessage -> com.sce.generated.every_matcher.matches(sceMessage, vElapsedLimit) }) -> TransitionsPart0.transitionRequestingAt0
        event is JobTesterEvent.Sce.Test.Snapshot && sceTestContext.timedOut("deadline") -> TransitionsPart0.transitionRequestingAt1
        else -> null
    }


    // settling's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSettling(state: JobTesterState, event: JobTesterEvent?): EnabledTransition<JobTesterState, HistoryId>? = when {
        event is JobTesterEvent.Sce.Test.Snapshot && sceTestContext.timedOut("settle") -> TransitionsPart0.transitionSettlingAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: job_tester.scxml:7 :: _machine
    override fun onEntry(state: JobTesterState, isDefaultEntry: Boolean) {
        when (state) {
            is JobTesterState.Failed -> onEntryOfFailed()
            is JobTesterState.Passed -> onEntryOfPassed()
            is JobTesterState.Requesting -> onEntryOfRequesting()
            is JobTesterState.Running -> onEntryOfRunning()
            is JobTesterState.Settling -> onEntryOfSettling()
        }
    }


    private fun onEntryOfFailed() {
                // SCE-MAP: job_tester.scxml:50 :: failed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPassed() {
                // SCE-MAP: job_tester.scxml:49 :: passed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfRequesting() {
                // SCE-MAP: job_tester.scxml:25 :: requesting :: _state_body
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
                eventName = "job.status",
                target = "sut",
                content = "",
                params = sendWireParams,
                sendId = "__send_1",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId,
                // docs/SCE_TEST_PROFILE.md §12.3: a test component's message is
                // the value of its specific template, handed to the test-port
                // processor as a value and never as an encoding (docs/adr/0091,
                // C2), with the writer of its canonical form in the test log
                // (§12.5). Every other send leaves it null and carries `eventData`.
                value = com.sce.runtime.test.messageValue(com.sce.generated.job_report.value(), com.sce.generated.job_report.TYPE) { sceSent -> com.sce.generated.job_report.canonical(sceSent) }
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("http://sce.dev/ext/test-port")) {
                raisePlatformError(JobTesterEvent.Error.Execution, ErrorReason.PROCESSOR_UNREGISTERED, "<send type='http://sce.dev/ext/test-port'> names a processor the host declared but never registered", "__send_1")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)

            sceTestContext.startTimer("deadline", 2000000L)
                }
    }

    private fun onEntryOfRunning() {
                // SCE-MAP: job_tester.scxml:23 :: running :: _state_body
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
                    val generatedInvokeId = "running.${System.identityHashCode(this)}.helper"
                    deferInvoke(JobTesterState.Running, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "http://sce.dev/ext/test-component",
                                invokeId = "helper",
                                src = "helper.scxml",
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
                            raisePlatformError(JobTesterEvent.Error.Execution, ErrorReason.PROCESSOR_UNREGISTERED, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
    }

    private fun onEntryOfSettling() {
                // SCE-MAP: job_tester.scxml:40 :: settling :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            sceTestContext.startTimer("settle", 500000L)
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: job_tester.scxml:7 :: _machine
    override fun onExit(state: JobTesterState) {
        when (state) {
            is JobTesterState.Failed -> onExitOfFailed()
            is JobTesterState.Passed -> onExitOfPassed()
            is JobTesterState.Requesting -> onExitOfRequesting()
            is JobTesterState.Running -> onExitOfRunning()
            is JobTesterState.Settling -> onExitOfSettling()
        }
    }


    private fun onExitOfFailed() {
                // SCE-MAP: job_tester.scxml:50 :: failed :: _state_body
    }

    private fun onExitOfPassed() {
                // SCE-MAP: job_tester.scxml:49 :: passed :: _state_body
    }

    private fun onExitOfRequesting() {
                // SCE-MAP: job_tester.scxml:25 :: requesting :: _state_body
    }

    private fun onExitOfRunning() {
                // SCE-MAP: job_tester.scxml:23 :: running :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(JobTesterState.Running)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("http://sce.dev/ext/test-component", "helper")
    }

    private fun onExitOfSettling() {
                // SCE-MAP: job_tester.scxml:40 :: settling :: _state_body
    }



    // docs/SCE_TEST_PROFILE.md §4.4 (P4 step 1), §12.3: a branch of the test
    // component is taken before the microstep exits any state: it removes what
    // it matched from the round's snapshot, writes its record in the test log,
    // and holds a message its content reads in the payload member of its
    // record's event (`Transition::native_take`).
    override fun takeTransition(source: JobTesterState, transitionIndex: Int) {
        when {
            source is JobTesterState.Requesting && transitionIndex == 0 -> {
                val sceMessage = sceTestContext.receive("sut")
                val sceValue = checkNotNull(sceMessage.valueAs<com.sce.generated.every_matcher.Value>()) { "a receive branch takes the message its guard matched" }
                pendingJobStatusPayload = JobTesterJobStatusPayload(elapsed_ms = sceValue.elapsed_ms, note = sceValue.note, codes = sceValue.codes, tag = sceValue.tag, retry = sceValue.retry, count = sceValue.count)
            }
            source is JobTesterState.Requesting && transitionIndex == 1 -> {
                sceTestContext.timeout("deadline")
            }
            source is JobTesterState.Settling && transitionIndex == 0 -> {
                sceTestContext.timeout("settle")
            }
        }
    }

    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: job_tester.scxml:7 :: _machine
    override fun executeTransitionContent(source: JobTesterState, transitionIndex: Int) {
        when (source) {
            is JobTesterState.Requesting -> transitionContentOfRequesting(transitionIndex)
            is JobTesterState.Settling -> transitionContentOfSettling(transitionIndex)
            else -> {}
        }
    }


    // The content of `requesting`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfRequesting(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfRequestingAt0()
            1 -> transitionContentOfRequestingAt1()
            else -> {}
        }
    }


    private fun transitionContentOfRequestingAt0() {
                // SCE-MAP: job_tester.scxml:30 :: requesting :: _transition_0

            sceTestContext.stopTimer("deadline")

            vElapsed = pendingJobStatusPayload!!.elapsed_ms


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
                eventName = "job.status",
                target = "ctl",
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
                // C2), with the writer of its canonical form in the test log
                // (§12.5). Every other send leaves it null and carries `eventData`.
                value = com.sce.runtime.test.messageValue(com.sce.generated.job_report.value(), com.sce.generated.job_report.TYPE) { sceSent -> com.sce.generated.job_report.canonical(sceSent) }
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("http://sce.dev/ext/test-port")) {
                raisePlatformError(JobTesterEvent.Error.Execution, ErrorReason.PROCESSOR_UNREGISTERED, "<send type='http://sce.dev/ext/test-port'> names a processor the host declared but never registered", "__send_0")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfRequestingAt1() {
                // SCE-MAP: job_tester.scxml:36 :: requesting :: _transition_1

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.FAIL, "no reply in time")
    }

    // The content of `settling`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfSettling(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfSettlingAt0()
            else -> {}
        }
    }


    private fun transitionContentOfSettlingAt0() {
                // SCE-MAP: job_tester.scxml:44 :: settling :: _transition_0

            sceTestContext.setverdict(com.sce.runtime.test.Verdict.PASS, "")
    }
}
