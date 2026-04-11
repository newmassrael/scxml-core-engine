// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using ProcedureStateMachine.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event.NONE transitions.

package com.sce.generated.procedure_startup_check

import com.sce.forge.runtime.procedure.*

// ── State and Event enums ───────────────────────────────────────

enum class State {
    CheckVoltage,
    CheckTemp,
    Success,
    FailVoltage,
    FailOvertemp
}

enum class Event {
    NONE,
    Fail,
    Ok
}

// ── Generated procedure state machine ───────────────────────────

class ProcedureStartupCheck : ProcedureStateMachine<State, Event>() {
    private var voltage: Float = 0.0f
    private var temperature: Float = 0.0f

    fun setVoltage(value: Float) {
        this.voltage = value
    }

    fun setTemperature(value: Float) {
        this.temperature = value
    }

    override val noneEvent = Event.NONE

    override fun initialState() = State.CheckVoltage

    override fun isFinal(state: State) = state in FINAL_STATES

    override fun finalStateName(state: State) = when (state) {
        State.Success -> "success"
        State.FailVoltage -> "fail_voltage"
        State.FailOvertemp -> "fail_overtemp"
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
            State.CheckVoltage -> {
                if (event == Event.NONE) {
                    if (voltage >= 11.5 && voltage <= 14.5) return Triple(State.CheckTemp, 0, false)
                }
                if (event == Event.NONE) {
                    return Triple(State.FailVoltage, 1, false)
                }
            }
            State.CheckTemp -> {
                if (event == Event.NONE) {
                    if (temperature < 80.0) return Triple(State.Success, 0, false)
                }
                if (event == Event.NONE) {
                    return Triple(State.FailOvertemp, 1, false)
                }
            }
            else -> {}
        }
        return null
    }

    override fun executeTransitionActions(source: State, trIndex: Int) {
    }

    companion object {
        private val FINAL_STATES = setOf(State.Success, State.FailVoltage, State.FailOvertemp)
    }
}

// ── Convenience wrapper function ────────────────────────────────

fun execute(
    handler: ProcedureServiceHandler,
    voltage: Float,
    temperature: Float): ProcedureRunResult {
    val sm = ProcedureStartupCheck()
    sm.setServiceHandler(handler)
    sm.setVoltage(voltage)
    sm.setTemperature(temperature)
    return sm.runToCompletion()
}
