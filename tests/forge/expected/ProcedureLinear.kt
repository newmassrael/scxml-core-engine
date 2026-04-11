// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using ProcedureStateMachine.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event.NONE transitions.

package com.sce.generated.procedure_linear

import com.sce.forge.runtime.procedure.*

// ── State and Event enums ───────────────────────────────────────

enum class State {
    StageA,
    StageB,
    StageC,
    Done
}

enum class Event {
    NONE,
    Fail,
    Ok
}

// ── Generated procedure state machine ───────────────────────────

class ProcedureLinear : ProcedureStateMachine<State, Event>() {
    private var value: Int = 0

    fun setValue(value: Int) {
        this.value = value
    }

    override val noneEvent = Event.NONE

    override fun initialState() = State.StageA

    override fun isFinal(state: State) = state in FINAL_STATES

    override fun finalStateName(state: State) = when (state) {
        State.Done -> "done"
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
            State.StageA -> {
                if (event == Event.NONE) {
                    return Triple(State.StageB, 0, false)
                }
            }
            State.StageB -> {
                if (event == Event.NONE) {
                    return Triple(State.StageC, 0, false)
                }
            }
            State.StageC -> {
                if (event == Event.NONE) {
                    return Triple(State.Done, 0, false)
                }
            }
            else -> {}
        }
        return null
    }

    override fun executeTransitionActions(source: State, trIndex: Int) {
    }

    companion object {
        private val FINAL_STATES = setOf(State.Done)
    }
}

// ── Convenience wrapper function ────────────────────────────────

fun execute(
    handler: ProcedureServiceHandler,
    value: Int): ProcedureRunResult {
    val sm = ProcedureLinear()
    sm.setServiceHandler(handler)
    sm.setValue(value)
    return sm.runToCompletion()
}
