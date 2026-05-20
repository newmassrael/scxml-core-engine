// SCE-MAP: crossfile_procedure_codec_mutate:3

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

package com.sce.generated.crossfile_procedure_codec_mutate
import com.sce.generated.codec_simple_frame.*

import com.sce.forge.runtime.procedure.*

// ── State and Event enums ───────────────────────────────────────

enum class State {
    Init,
    Send,
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

class CrossfileProcedureCodecMutate : ProcedureStateMachine<State, Event>() {
    private var msgId: UByte = 0.toUByte()

    // Imported kinds (cross-file composition)
    private val frame: CodecSimpleFrame = CodecSimpleFrame()

    fun setMsgId(value: UByte) {
        this.msgId = value
    }

    override val noneEvent = Event.NONE

    override fun initialState() = State.Init

    override fun isFinal(state: State) = state in FINAL_STATES

    override fun finalStateName(state: State) = when (state) {
        State.Done -> "done"
        State.Error -> "error"
        else -> ""
    }

    override fun executeEntryActions(state: State): Pair<Event, String> {
        when (state) {
            State.Send -> {
                serviceHandler?.let { handler ->
                    val req = ProcedureServiceRequest(
                        service = "transport",
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
            State.Init -> {
                if (event == Event.NONE) {
                    return Triple(State.Send, 0, true)
                }
            }
            State.Send -> {
                if (event == Event.Ok) {
                    return Triple(State.Done, 0, false)
                }
                if (event == Event.Fail) {
                    return Triple(State.Error, 1, false)
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
        if (source == State.Init) {
            if (trIndex == 0) {
                frame.msgId = msgId
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
    msgId: UByte): ProcedureRunResult {
    val sm = CrossfileProcedureCodecMutate()
    sm.setServiceHandler(handler)
    sm.setMsgId(msgId)
    return sm.runToCompletion()
}
