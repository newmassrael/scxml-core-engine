// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using ProcedureStateMachine.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event.NONE transitions.
//
// External dependencies (from sce:payload expressions — must be in scope):
//   computeKey(seed)

package com.sce.generated.procedure_security_access

import com.sce.forge.runtime.procedure.*

// ── State and Event enums ───────────────────────────────────────

enum class State {
    SendTesterPresent,
    RequestSeed,
    SendKey,
    Retry,
    Done,
    Error
}

enum class Event {
    NONE,
    ErrorExecution,
    Fail,
    Ok
}

// ── Generated procedure state machine ───────────────────────────

class ProcedureSecurityAccess : ProcedureStateMachine<State, Event>() {
    private var ecuAddr: UInt = 0u
    private var seed: ByteArray = byteArrayOf()
    private var maxRetries: Int = 3
    private var retryCount: Int = 0

    // <sce:helper> DI closures
    private var computeKey: (ByteArray) -> ByteArray = { _arg0 -> error("helper 'computeKey' not set — call setComputeKey() before runToCompletion()") }

    fun setComputeKey(fn: (ByteArray) -> ByteArray) {
        this.computeKey = fn
    }

    fun setEcuAddr(value: UInt) {
        this.ecuAddr = value
    }

    override val noneEvent = Event.NONE

    override fun initialState() = State.SendTesterPresent

    override fun isFinal(state: State) = state in FINAL_STATES

    override fun finalStateName(state: State) = when (state) {
        State.Done -> "done"
        State.Error -> "error"
        else -> ""
    }

    override fun executeEntryActions(state: State): Pair<Event, String> {
        when (state) {
            State.SendTesterPresent -> {
                serviceHandler?.let { handler ->
                    val req = ProcedureServiceRequest(
                        service = "TesterPresent",
                        addr = (ecuAddr).toString(),
                    )
                    val resp = handler(req)
                    return Pair(if (resp.success) Event.Ok else Event.Fail, resp.data)
                }
            }
            State.RequestSeed -> {
                serviceHandler?.let { handler ->
                    val req = ProcedureServiceRequest(
                        service = "SecurityAccess",
                        subfunc = "0x01",
                    )
                    val resp = handler(req)
                    return Pair(if (resp.success) Event.Ok else Event.Fail, resp.data)
                }
            }
            State.SendKey -> {
                serviceHandler?.let { handler ->
                    val req = ProcedureServiceRequest(
                        service = "SecurityAccess",
                        subfunc = "0x02",
                        payload = computeKey(seed),
                    )
                    val resp = handler(req)
                    return Pair(if (resp.success) Event.Ok else Event.Fail, resp.data)
                }
            }
            State.Done -> {
                doneData["result"] = "success"
            }
            State.Error -> {
                doneData["result"] = "failure"
            }
            else -> {}
        }
        return Pair(Event.NONE, "")
    }

    override fun processTransition(state: State, event: Event): Triple<State, Int, Boolean>? {
        when (state) {
            State.SendTesterPresent -> {
                if (event == Event.Ok) {
                    return Triple(State.RequestSeed, 0, false)
                }
                if (event == Event.Fail) {
                    return Triple(State.Error, 1, false)
                }
            }
            State.RequestSeed -> {
                if (event == Event.Ok) {
                    return Triple(State.SendKey, 0, true)
                }
                if (event == Event.Fail) {
                    return Triple(State.Retry, 1, false)
                }
            }
            State.SendKey -> {
                if (event == Event.Ok) {
                    return Triple(State.Done, 0, false)
                }
                if (event == Event.Fail) {
                    return Triple(State.Retry, 1, false)
                }
            }
            State.Retry -> {
                if (event == Event.NONE) {
                    if (retryCount < maxRetries) return Triple(State.RequestSeed, 0, true)
                }
                if (event == Event.NONE) {
                    if (retryCount >= maxRetries) return Triple(State.Error, 1, false)
                }
            }
            else -> {}
        }
        return null
    }

    // Returns null for normal flow; a non-null Event signals that an
    // assign-time check (RFC `claudedocs/rfc-forge-bytes-bounded.md`
    // §3 B4 bytes cap violation) raised an internal event that the
    // shared runToCompletion loop re-pumps through processTransition.
    override fun executeTransitionActions(source: State, trIndex: Int): Event? {
        if (source == State.RequestSeed) {
            if (trIndex == 0) {
                run {
                    val scopeTmp = pendingEventData.toByteArray()
                    if (scopeTmp.size > 64) {
                        return Event.ErrorExecution
                    }
                    seed = scopeTmp
                }
            }
        }
        if (source == State.Retry) {
            if (trIndex == 0) {
                retryCount = retryCount + 1
            }
        }
        return null
    }

    companion object {
        private val FINAL_STATES = setOf(State.Done, State.Error)
    }
}

// ── Convenience wrapper function ────────────────────────────────

fun execute(
    handler: ProcedureServiceHandler,
    computeKey: (ByteArray) -> ByteArray,
    ecuAddr: UInt): ProcedureRunResult {
    val sm = ProcedureSecurityAccess()
    sm.setServiceHandler(handler)
    sm.setComputeKey(computeKey)
    sm.setEcuAddr(ecuAddr)
    return sm.runToCompletion()
}
