// SCE-MAP: crossfile_procedure_filter:10

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using ProcedureStateMachine.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via Event.NONE transitions.

package com.sce.generated.crossfile_procedure_filter
import com.sce.generated.filter_low_pass.*

import com.sce.forge.runtime.procedure.*

// ── State and Event enums ───────────────────────────────────────

enum class State {
    Sample,
    Done
}

enum class Event {
    NONE,
    ErrorExecution,
    Fail,
    Ok
}

// ── Generated procedure state machine ───────────────────────────

class CrossfileProcedureFilter : ProcedureStateMachine<State, Event>() {
    private var rawSample: Double = 0.0
    private var smoothed: Double = 0.0

    // Imported kinds (cross-file composition)
    private val smoother: FilterLowPass = FilterLowPass()

    fun setRawSample(value: Double) {
        this.rawSample = value
    }

    override val noneEvent = Event.NONE

    override fun initialState() = State.Sample

    override fun isFinal(state: State) = state in FINAL_STATES

    override fun finalStateName(state: State) = when (state) {
        State.Done -> "done"
        else -> ""
    }

    override fun executeEntryActions(state: State): Pair<Event, String> {
        when (state) {
            State.Done -> {
                doneData["result"] = "success"
            }
            else -> {}
        }
        return Pair(Event.NONE, "")
    }

    override fun processTransition(state: State, event: Event): Triple<State, Int, Boolean>? {
        when (state) {
            State.Sample -> {
                if (event == Event.NONE) {
                    return Triple(State.Done, 0, true)
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
        if (source == State.Sample) {
            if (trIndex == 0) {
                smoothed = smoother.update(rawSample)
            }
        }
        return null
    }

    companion object {
        private val FINAL_STATES = setOf(State.Done)
    }
}

// ── Convenience wrapper function ────────────────────────────────

fun execute(
    handler: ProcedureServiceHandler,
    rawSample: Double): ProcedureRunResult {
    val sm = CrossfileProcedureFilter()
    sm.setServiceHandler(handler)
    sm.setRawSample(rawSample)
    return sm.runToCompletion()
}
