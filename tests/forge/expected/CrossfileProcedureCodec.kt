// SCE-MAP: crossfile_procedure_codec:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using ProcedureStateMachine.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event.NONE transitions.
//
// External dependencies (from sce:payload expressions — must be in scope):
//   frame.encode()

package com.sce.generated.crossfile_procedure_codec
import com.sce.generated.codec_simple_frame.*

import com.sce.forge.runtime.procedure.*

// ── State and Event enums ───────────────────────────────────────

enum class State {
    SendRequest,
    Decode,
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

class CrossfileProcedureCodec : ProcedureStateMachine<State, Event>() {
    private var ecuAddr: UInt = 0u
    private var response: ByteArray = byteArrayOf()

    // Imported kinds (cross-file composition)
    private val frame: CodecSimpleFrame = CodecSimpleFrame()

    fun setEcuAddr(value: UInt) {
        this.ecuAddr = value
    }

    override val noneEvent = Event.NONE

    override fun initialState() = State.SendRequest

    override fun isFinal(state: State) = state in FINAL_STATES

    override fun finalStateName(state: State) = when (state) {
        State.Done -> "done"
        State.Error -> "error"
        else -> ""
    }

    override fun executeEntryActions(state: State): Pair<Event, String> {
        when (state) {
            State.SendRequest -> {
                serviceHandler?.let { handler ->
                    val req = ProcedureServiceRequest(
                        service = "Diag",
                        addr = (ecuAddr).toString(),
                        payload = frame.encodeToByteArray(),
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
            State.SendRequest -> {
                if (event == Event.Ok) {
                    return Triple(State.Decode, 0, true)
                }
                if (event == Event.Fail) {
                    return Triple(State.Error, 1, false)
                }
            }
            State.Decode -> {
                if (event == Event.NONE) {
                    return Triple(State.Done, 0, false)
                }
            }
            else -> {}
        }
        return null
    }

    // Returns null for normal flow; a non-null Event signals that an
    // assign-time bytes-cap check raised an internal event that the
    // shared runToCompletion loop re-pumps through processTransition.
    override fun executeTransitionActions(source: State, trIndex: Int): Event? {
        if (source == State.SendRequest) {
            if (trIndex == 0) {
                run {
                    val scopeTmp = pendingEventData.toByteArray()
                    if (scopeTmp.size > 256) {
                        return Event.ErrorExecution
                    }
                    response = scopeTmp
                }
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
    ecuAddr: UInt): ProcedureRunResult {
    val sm = CrossfileProcedureCodec()
    sm.setServiceHandler(handler)
    sm.setEcuAddr(ecuAddr)
    return sm.runToCompletion()
}
