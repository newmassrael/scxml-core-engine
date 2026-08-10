// SCE-MAP: procedure_diamond:2 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using ProcedureStateMachine.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event.NONE transitions.

package com.sce.generated.procedure_diamond

import com.sce.forge.runtime.procedure.*

// ── State and Event enums ───────────────────────────────────────

enum class State {
    Classify,
    HighPath,
    MidPath,
    LowPath,
    Accept,
    Reject
}

enum class Event {
    NONE,
    ErrorExecution,
    Fail,
    Ok
}

// ── Generated procedure state machine ───────────────────────────

class ProcedureDiamond : ProcedureStateMachine<State, Event>() {
    private var sensorValue: UShort = 0.toUShort()
    private var mode: String = ""

    fun setSensorValue(value: UShort) {
        this.sensorValue = value
    }

    fun setMode(value: String) {
        this.mode = value
    }

    override val noneEvent = Event.NONE

    override fun initialState() = State.Classify

    override fun isFinal(state: State) = state in FINAL_STATES

    override fun finalStateName(state: State) = when (state) {
        State.Accept -> "accept"
        State.Reject -> "reject"
        else -> ""
    }

    override fun executeEntryActions(state: State): Pair<Event, String> {
        when (state) {
            else -> {}
        }
        return Pair(Event.NONE, "")
    }

    override fun processTransition(state: State, event: Event): Triple<State, Int, Boolean>? {
        when (state) {
            State.Classify -> {
                if (event == Event.NONE) {
                    if (sensorValue > 1000.toUShort()) return Triple(State.HighPath, 0, false)
                }
                if (event == Event.NONE) {
                    if (sensorValue > 500.toUShort()) return Triple(State.MidPath, 1, false)
                }
                if (event == Event.NONE) {
                    return Triple(State.LowPath, 2, false)
                }
            }
            State.HighPath -> {
                if (event == Event.NONE) {
                    if (mode == "strict") return Triple(State.Reject, 0, false)
                }
                if (event == Event.NONE) {
                    return Triple(State.Accept, 1, false)
                }
            }
            State.MidPath -> {
                if (event == Event.NONE) {
                    return Triple(State.Accept, 0, false)
                }
            }
            State.LowPath -> {
                if (event == Event.NONE) {
                    return Triple(State.Accept, 0, false)
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
        return null
    }

    companion object {
        private val FINAL_STATES = setOf(State.Accept, State.Reject)
    }
}

// ── Convenience wrapper function ────────────────────────────────

fun execute(
    handler: ProcedureServiceHandler,
    sensorValue: UShort,
    mode: String): ProcedureRunResult {
    val sm = ProcedureDiamond()
    sm.setServiceHandler(handler)
    sm.setSensorValue(sensorValue)
    sm.setMode(mode)
    return sm.runToCompletion()
}
