// SCE-GENERATED — DO NOT EDIT
// source-hash: d2a16315bda3b311baf737203d9e9fa84cdbea31907669fbc8b60f205f09fef9

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/host_processor/statechart_static_host_params.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_static_host_params.scxml:80 :: _machine

package com.sce.integration.statechart_static_host_params

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartStaticHostParamsState : State {
    data object Done : StatechartStaticHostParamsState
    data object Idle : StatechartStaticHostParamsState
    data object Working : StatechartStaticHostParamsState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartStaticHostParamsEvent : Event {
    data object After : StatechartStaticHostParamsEvent
    data object Big : StatechartStaticHostParamsEvent
    data object Bloat : StatechartStaticHostParamsEvent
    data object Bump : StatechartStaticHostParamsEvent
    sealed interface Done : StatechartStaticHostParamsEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object H : Invoke
        }
    }
    sealed interface Error : StatechartStaticHostParamsEvent {
        data object Execution : Error
    }
    data object Go : StatechartStaticHostParamsEvent
    data object Lost : StatechartStaticHostParamsEvent
    data object Notify : StatechartStaticHostParamsEvent
    data object Text : StatechartStaticHostParamsEvent
    data object Value : StatechartStaticHostParamsEvent
}
// --- State Machine (W3C SCXML) ---

class StatechartStaticHostParamsStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StatechartStaticHostParamsState, StatechartStaticHostParamsEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `count` datamodel variable, the machine's own. */
    private var vCount: UInt = 3.toUInt()
    /** W3C SCXML 5.2: the `ready` datamodel variable, the machine's own. */
    private var vReady: Boolean = false
    /** W3C SCXML 5.2: the `label` datamodel variable, the machine's own. */
    private var vLabel: String = "idle"
    /** W3C SCXML 5.2: the `delta` datamodel variable, the machine's own. */
    private var vDelta: Short = -5
    /** W3C SCXML 5.2: the `ratio` datamodel variable, the machine's own. */
    private var vRatio: Double = 1.5
    /** W3C SCXML 5.2: the `tag` datamodel variable, the machine's own. */
    private var vTag: String = "job://params"
    /** W3C SCXML 5.2: the `huge` datamodel variable, the machine's own. */
    private var vHuge: UInt = 1.toUInt()
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    private var vErrors: UInt = 0.toUInt()

    /** What the published `errors` holds. */
    val errors: UInt get() = vErrors

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var count: UInt? = null
        var ready: Boolean? = null
        var label: String? = null
        var delta: Short? = null
        var ratio: Double? = null
        var tag: String? = null
        var huge: UInt? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.count?.let { vCount = it }
        params.ready?.let { vReady = it }
        params.label?.let { vLabel = it }
        params.delta?.let { vDelta = it }
        params.ratio?.let { vRatio = it }
        params.tag?.let { vTag = it }
        params.huge?.let { vHuge = it }
        params.errors?.let { vErrors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val errors: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StatechartStaticHostParamsState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        errors = vErrors,
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
        first = "b286785fdef50776d9642d37e26700602e2eff4718258424a77f400ff92c5e20",
        revised = "93ef05f6eb9aadfc384df76112d432c484e071a161d90eb0cbd084451f589d59",
        format2First = "b286785fdef50776d9642d37e26700602e2eff4718258424a77f400ff92c5e20",
        format2Revised = "93ef05f6eb9aadfc384df76112d432c484e071a161d90eb0cbd084451f589d59",
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
            "count" to SavedValues.of(vCount),
            "ready" to SavedValues.of(vReady),
            "label" to SavedValues.of(vLabel),
            "delta" to SavedValues.of(vDelta),
            "ratio" to SavedValues.of(vRatio),
            "tag" to SavedValues.of(vTag),
            "huge" to SavedValues.of(vHuge),
            "errors" to SavedValues.of(vErrors),
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
        val saved1 = SavedValues.uint32(savedGrown.variable("count"), "count")
        val saved2 = SavedValues.bool(savedGrown.variable("ready"), "ready")
        val saved3 = SavedValues.string(savedGrown.variable("label"), "label", 16)
        val saved4 = SavedValues.int16(savedGrown.variable("delta"), "delta")
        val saved5 = SavedValues.float64(savedGrown.variable("ratio"), "ratio")
        val saved6 = SavedValues.string(savedGrown.variable("tag"), "tag", 16)
        val saved7 = SavedValues.uint32(savedGrown.variable("huge"), "huge")
        val saved8 = SavedValues.uint32(savedGrown.variable("errors"), "errors")
        vCount = saved1
        vReady = saved2
        vLabel = saved3
        vDelta = saved4
        vRatio = saved5
        vTag = saved6
        vHuge = saved7
        vErrors = saved8
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
        SavedValues.uint32(savedGrown.variable("count"), "count")
        SavedValues.bool(savedGrown.variable("ready"), "ready")
        SavedValues.string(savedGrown.variable("label"), "label", 16)
        SavedValues.int16(savedGrown.variable("delta"), "delta")
        SavedValues.float64(savedGrown.variable("ratio"), "ratio")
        SavedValues.string(savedGrown.variable("tag"), "tag", 16)
        SavedValues.uint32(savedGrown.variable("huge"), "huge")
        SavedValues.uint32(savedGrown.variable("errors"), "errors")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, StatechartStaticHostParamsState>> = listOf(
        Triple("x-sce-host", "h", StatechartStaticHostParamsState.Working),
    )

    override val initialState: StatechartStaticHostParamsState = StatechartStaticHostParamsState.Idle

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
    override fun isFinalState(state: StatechartStaticHostParamsState): Boolean = when (state) {
        is StatechartStaticHostParamsState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StatechartStaticHostParamsState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StatechartStaticHostParamsState, HistoryId>> =
            listOf(StateTarget(StatechartStaticHostParamsState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StatechartStaticHostParamsState, HistoryId>(
            StatechartStaticHostParamsState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StatechartStaticHostParamsState, HistoryId>(
            StatechartStaticHostParamsState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StatechartStaticHostParamsState, HistoryId>(
            StatechartStaticHostParamsState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StatechartStaticHostParamsState, HistoryId>(
            StatechartStaticHostParamsState.Idle,
            listOf(StateTarget(StatechartStaticHostParamsState.Working)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<StatechartStaticHostParamsState, HistoryId>(
            StatechartStaticHostParamsState.Working,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 1, as the microstep reads it.
        val transitionWorkingAt1 = EnabledTransition<StatechartStaticHostParamsState, HistoryId>(
            StatechartStaticHostParamsState.Working,
            listOf(StateTarget(StatechartStaticHostParamsState.Done)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StatechartStaticHostParamsState? = when (stateId) {
        "done" -> StatechartStaticHostParamsState.Done
        "idle" -> StatechartStaticHostParamsState.Idle
        "working" -> StatechartStaticHostParamsState.Working
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartStaticHostParamsState): String = when (state) {
        is StatechartStaticHostParamsState.Done -> "done"
        is StatechartStaticHostParamsState.Idle -> "idle"
        is StatechartStaticHostParamsState.Working -> "working"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StatechartStaticHostParamsState): Int = when (state) {
        is StatechartStaticHostParamsState.Done -> 2
        is StatechartStaticHostParamsState.Idle -> 0
        is StatechartStaticHostParamsState.Working -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartStaticHostParamsEvent? = when (name) {
        "after" -> StatechartStaticHostParamsEvent.After
        "big" -> StatechartStaticHostParamsEvent.Big
        "bloat" -> StatechartStaticHostParamsEvent.Bloat
        "bump" -> StatechartStaticHostParamsEvent.Bump
        "done.invoke" -> StatechartStaticHostParamsEvent.Done.Invoke.Self
        "done.invoke.h" -> StatechartStaticHostParamsEvent.Done.Invoke.H
        "error.execution" -> StatechartStaticHostParamsEvent.Error.Execution
        "go" -> StatechartStaticHostParamsEvent.Go
        "lost" -> StatechartStaticHostParamsEvent.Lost
        "notify" -> StatechartStaticHostParamsEvent.Notify
        "text" -> StatechartStaticHostParamsEvent.Text
        "value" -> StatechartStaticHostParamsEvent.Value
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartStaticHostParamsEvent): String? = when (event) {
        is StatechartStaticHostParamsEvent.After -> "after"
        is StatechartStaticHostParamsEvent.Big -> "big"
        is StatechartStaticHostParamsEvent.Bloat -> "bloat"
        is StatechartStaticHostParamsEvent.Bump -> "bump"
        is StatechartStaticHostParamsEvent.Done.Invoke.Self -> "done.invoke"
        is StatechartStaticHostParamsEvent.Done.Invoke.H -> "done.invoke.h"
        is StatechartStaticHostParamsEvent.Error.Execution -> "error.execution"
        is StatechartStaticHostParamsEvent.Go -> "go"
        is StatechartStaticHostParamsEvent.Lost -> "lost"
        is StatechartStaticHostParamsEvent.Notify -> "notify"
        is StatechartStaticHostParamsEvent.Text -> "text"
        is StatechartStaticHostParamsEvent.Value -> "value"
    }

    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "h",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StatechartStaticHostParamsState,
        event: StatechartStaticHostParamsEvent?
    ): EnabledTransition<StatechartStaticHostParamsState, HistoryId>? = when (state) {
        is StatechartStaticHostParamsState.Idle -> when {
            event is StatechartStaticHostParamsEvent.Bump -> transitionIdleAt0
            event is StatechartStaticHostParamsEvent.Big -> transitionIdleAt1
            event is StatechartStaticHostParamsEvent.Bloat -> transitionIdleAt2
            event is StatechartStaticHostParamsEvent.Go -> transitionIdleAt3
            else -> null
        }
        is StatechartStaticHostParamsState.Working -> when {
            event is StatechartStaticHostParamsEvent.Error.Execution -> transitionWorkingAt0
            event is StatechartStaticHostParamsEvent.Done.Invoke.H -> transitionWorkingAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_static_host_params.scxml:80 :: _machine
    override fun onEntry(state: StatechartStaticHostParamsState, isDefaultEntry: Boolean) {
        when (state) {
            is StatechartStaticHostParamsState.Done -> {
                // SCE-MAP: statechart_static_host_params.scxml:152 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StatechartStaticHostParamsState.Idle -> {
                // SCE-MAP: statechart_static_host_params.scxml:94 :: idle :: _state_body
            }
            is StatechartStaticHostParamsState.Working -> {
                // SCE-MAP: statechart_static_host_params.scxml:109 :: working :: _state_body
                // W3C SCXML 3.8: Onentry block 1/3
                run {


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "count", (vCount).toLong())

            putParam(sendPayload, "ready", vReady)

            putParam(sendPayload, "label", vLabel)

            try {
                putParam(sendPayload, "twice", (com.sce.forge.runtime.SceChecked.mul(vCount, 2.toUInt())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<send> <param name='twice'> could not be read")
                paramFailed = true
            }

            putParam(sendPayload, "delta", (vDelta).toLong())

            putParam(sendPayload, "ratio", vRatio)

            try {
                putParam(sendPayload, "boom", (com.sce.forge.runtime.SceChecked.mul(vCount, 2000000000.toUInt())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<send> <param name='boom'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "notify",
                target = "somewhere",
                content = "",
                params = sendWireParams,
                sendId = "__send_0",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_0")
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 3.8: Onentry block 2/3
                run {


            if (run send@{
            var paramFailed = false
            // W3C SCXML 5.6.2: the value of <content expr> is the event's data.
            // Under `datamodel="sce-static"` it is one value lowered to native
            // code (`Action::native_content_value`), read from the machine's own
            // fields now and kept, so the event's data and the text a host takes
            // as `content` are renderings of the one reading. "If the evaluation
            // of 'expr' produces an error, the Processor MUST place
            // error.execution in the internal event queue and use the empty
            // string as the value of the <content> element."
            val sendContentValue: Any = try {
                (com.sce.forge.runtime.SceChecked.mul(vCount, 2.toUInt())).toLong()
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<send> contentexpr failed to evaluate", "__send_1")
                paramFailed = true
                ""
            }
            val sendData = valueToJson(sendContentValue)
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "value",
                target = "somewhere",
                content = valueToWireString(sendContentValue),
                params = sendWireParams,
                sendId = "__send_1",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_1")
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            // W3C SCXML 5.6.2: the value of <content expr> is the event's data.
            // Under `datamodel="sce-static"` it is one value lowered to native
            // code (`Action::native_content_value`), read from the machine's own
            // fields now and kept, so the event's data and the text a host takes
            // as `content` are renderings of the one reading. "If the evaluation
            // of 'expr' produces an error, the Processor MUST place
            // error.execution in the internal event queue and use the empty
            // string as the value of the <content> element."
            val sendContentValue: Any = vLabel
            val sendData = valueToJson(sendContentValue)
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "text",
                target = "somewhere",
                content = valueToWireString(sendContentValue),
                params = sendWireParams,
                sendId = "__send_2",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_2")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 3.8: Onentry block 3/3
                run {


            if (run send@{
            var paramFailed = false
            // W3C SCXML 5.6.2: the value of <content expr> is the event's data.
            // Under `datamodel="sce-static"` it is one value lowered to native
            // code (`Action::native_content_value`), read from the machine's own
            // fields now and kept, so the event's data and the text a host takes
            // as `content` are renderings of the one reading. "If the evaluation
            // of 'expr' produces an error, the Processor MUST place
            // error.execution in the internal event queue and use the empty
            // string as the value of the <content> element."
            val sendContentValue: Any = try {
                (com.sce.forge.runtime.SceChecked.mul(vCount, 2000000000.toUInt())).toLong()
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<send> contentexpr failed to evaluate", "__send_3")
                paramFailed = true
                ""
            }
            val sendData = valueToJson(sendContentValue)
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "lost",
                target = "somewhere",
                content = valueToWireString(sendContentValue),
                params = sendWireParams,
                sendId = "__send_3",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_3")
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            // W3C SCXML 5.6.2: the value of <content expr> is the event's data.
            // Under `datamodel="sce-static"` it is one value lowered to native
            // code (`Action::native_content_value`), read from the machine's own
            // fields now and kept, so the event's data and the text a host takes
            // as `content` are renderings of the one reading. "If the evaluation
            // of 'expr' produces an error, the Processor MUST place
            // error.execution in the internal event queue and use the empty
            // string as the value of the <content> element."
            val sendContentValue: Any = (vCount).toLong()
            val sendData = valueToJson(sendContentValue)
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "after",
                target = "somewhere",
                content = valueToWireString(sendContentValue),
                params = sendWireParams,
                sendId = "__send_4",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_4")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
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
                    val generatedInvokeId = "working.${System.identityHashCode(this)}.h"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeSrc: String = try {
                            if (com.sce.forge.runtime.SceChecked.mul(vCount, 2.toUInt()) > 0.toUInt()) vTag else vTag
                        } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                            raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<invoke> srcexpr failed to evaluate")
                            return@deferInvoke
                        }
                        val hostInvokeContent: String = try {
                            if (com.sce.forge.runtime.SceChecked.mul(vHuge, 2.toUInt()) > 0.toUInt()) vLabel else vLabel
                        } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                            raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<invoke> content expr failed to evaluate")
                            return@deferInvoke
                        }
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        run {
                            val v: Any? = (vCount).toLong()
                            hostInvokeParams["count"] =
                                (hostInvokeParams["count"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "count", v)
                        }
                        run {
                            val v: Any? = vReady
                            hostInvokeParams["ready"] =
                                (hostInvokeParams["ready"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "ready", v)
                        }
                        run {
                            val v: Any? = vLabel
                            hostInvokeParams["label"] =
                                (hostInvokeParams["label"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "label", v)
                        }
                        try {
                            val v: Any? = (com.sce.forge.runtime.SceChecked.mul(vCount, 2.toUInt())).toLong()
                            hostInvokeParams["twice"] =
                                (hostInvokeParams["twice"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "twice", v)
                        } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                            // W3C SCXML 5.7.1: report the failure and omit the pair.
                            raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<invoke> <param name='twice'> expr failed to evaluate")
                        }
                        try {
                            val v: Any? = (com.sce.forge.runtime.SceChecked.mul(vCount, 2000000000.toUInt())).toLong()
                            hostInvokeParams["boom"] =
                                (hostInvokeParams["boom"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "boom", v)
                        } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                            // W3C SCXML 5.7.1: report the failure and omit the pair.
                            raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<invoke> <param name='boom'> expr failed to evaluate")
                        }
                        run {
                            val v: Any? = (vDelta).toLong()
                            hostInvokeParams["delta"] =
                                (hostInvokeParams["delta"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "delta", v)
                        }
                        run {
                            val v: Any? = vRatio
                            hostInvokeParams["ratio"] =
                                (hostInvokeParams["ratio"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "ratio", v)
                        }
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "h",
                                src = hostInvokeSrc,
                                params = hostInvokeParams,
                                eventData = if (hostInvokePayload.isEmpty()) "" else buildJsonFromParams(hostInvokePayload),
                                content = hostInvokeContent                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_static_host_params.scxml:80 :: _machine
    override fun onExit(state: StatechartStaticHostParamsState) {
        when (state) {
            is StatechartStaticHostParamsState.Done -> {
                // SCE-MAP: statechart_static_host_params.scxml:152 :: done :: _state_body
            }
            is StatechartStaticHostParamsState.Idle -> {
                // SCE-MAP: statechart_static_host_params.scxml:94 :: idle :: _state_body
            }
            is StatechartStaticHostParamsState.Working -> {
                // SCE-MAP: statechart_static_host_params.scxml:109 :: working :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "h")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: statechart_static_host_params.scxml:80 :: _machine
    override fun executeTransitionContent(source: StatechartStaticHostParamsState, transitionIndex: Int) {
        when (source) {
        is StatechartStaticHostParamsState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_static_host_params.scxml:95 :: idle :: _transition_0

            if (try { vCount = com.sce.forge.runtime.SceChecked.add(vCount, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<assign location='count'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vReady = true

            if (try { vLabel = com.sce.forge.runtime.SceChecked.bounded("busy", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<assign location='label'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: statechart_static_host_params.scxml:100 :: idle :: _transition_1

            vCount = 3000000000.toUInt()
            }
            2 -> {
                // SCE-MAP: statechart_static_host_params.scxml:103 :: idle :: _transition_2

            vHuge = 3000000000.toUInt()
            }
            else -> {}
        }
        is StatechartStaticHostParamsState.Working -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_static_host_params.scxml:146 :: working :: _transition_0

            if (try { vErrors = com.sce.forge.runtime.SceChecked.add(vErrors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticHostParamsEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
