// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Kotlin Runtime — the trace port: each step of Appendix D a run takes

package com.sce.runtime

/**
 * Which step of Appendix D a [TraceStep] is: the actions
 * `docs/formal/AppendixD.tla` gives the interpreter (docs/adr/0115, D4 and D8).
 *
 * @param wireName The step's name in the run trace format
 *   (`tests/forge/conformance/run_trace.schema.json`)
 */
enum class TraceStepKind(val wireName: String) {
    /** AppendixD's `Start`: the entry into the initial configuration. */
    START("start"),

    /** AppendixD's `Eventless`: a microstep on eventless transitions. */
    EVENTLESS("eventless"),

    /**
     * AppendixD's `Internal`: an event taken off the internal queue, and the
     * microstep it selects, if it selects one.
     */
    INTERNAL("internal"),

    /**
     * AppendixD's `External`: an event taken off the external queue, and the
     * microstep it selects, if it selects one.
     */
    EXTERNAL("external"),

    /** AppendixD's `Stop`: every state still active exited. */
    STOP("stop"),
}

/**
 * A transition a step took, by the pair its row of the emitted transition table
 * is named by (docs/adr/0089), as [TransitionTaken] names it to the transition
 * observer.
 *
 * @param source The id of the state the transition is written in
 * @param index Its position among [source]'s own transitions in document order,
 *   from 0
 */
data class TraceFired(
    val source: String,
    val index: Int,
)

/**
 * One step a run took, as the trace port is told it.
 *
 * Each kind says what the run trace format asks of it and no more: a start what
 * it entered and the configuration; a microstep, eventless or on an event, what
 * it took, exited and entered and the configuration; an event step the event
 * too; a stop what it exited and the top-level final state the run ended in.
 * What a kind does not say is empty.
 *
 * @param kind Which step
 * @param event The event an internal or external step took, by the name
 *   `_event.name` holds while it is taken: the whole name it was
 *   raised or sent under, which is not the name of the event of the document it
 *   matched when a door resolved a longer name, or one the document never
 *   writes, to it. `null` for any other step.
 * @param fired The transitions the step's microstep took, in the order they were
 *   selected
 * @param exited The states exited, in the order they were exited
 * @param entered The states entered, in the order they were entered
 * @param configuration The configuration the step left, in document order;
 *   empty for a stop
 * @param outcome The top-level `<final>` the run ended in, of a stop that ends
 *   one
 */
data class TraceStep(
    val kind: TraceStepKind,
    val event: String? = null,
    val fired: List<TraceFired> = emptyList(),
    val exited: List<String> = emptyList(),
    val entered: List<String> = emptyList(),
    val configuration: List<String> = emptyList(),
    val outcome: String? = null,
) {
    /**
     * The step as one object of the run trace format: the fields its kind says,
     * in the order the format lists them, as every lane writes them.
     */
    fun toJson(): String {
        val out = StringBuilder()
        out.append("{\"kind\": ").append(Json.quote(kind.wireName))
        when (kind) {
            TraceStepKind.START -> {
                appendNames(out, "entered", entered)
                appendNames(out, "configuration", configuration)
            }
            TraceStepKind.EVENTLESS, TraceStepKind.INTERNAL, TraceStepKind.EXTERNAL -> {
                if (kind != TraceStepKind.EVENTLESS) {
                    out.append(", \"event\": ").append(Json.quote(event ?: ""))
                }
                out.append(", \"fired\": [")
                fired.forEachIndexed { at, taken ->
                    if (at > 0) out.append(", ")
                    out.append("{\"source\": ").append(Json.quote(taken.source))
                    out.append(", \"index\": ").append(taken.index.toString()).append('}')
                }
                out.append(']')
                appendNames(out, "exited", exited)
                appendNames(out, "entered", entered)
                appendNames(out, "configuration", configuration)
            }
            TraceStepKind.STOP -> {
                appendNames(out, "exited", exited)
                out.append(", \"outcome\": ").append(outcome?.let { Json.quote(it) } ?: "null")
            }
        }
        out.append('}')
        return out.toString()
    }

    private fun appendNames(out: StringBuilder, field: String, names: List<String>) {
        out.append(", ").append(Json.quote(field)).append(": [")
        names.forEachIndexed { at, name ->
            if (at > 0) out.append(", ")
            out.append(Json.quote(name))
        }
        out.append(']')
    }
}

/**
 * The steps of one run, kept as the trace port is told them, and written in the
 * one run trace format every lane writes (docs/adr/0115, D4 and D8):
 * `tests/forge/conformance/run_trace.schema.json`, read by `sce-build`'s
 * `run_trace`, which `sce-codegen formal traces` judges against the model the
 * traces manifest names.
 *
 * Not thread-safe: the port is called on the thread that drives the engine, and
 * the trace is read once the run is over.
 */
class RunTrace {
    private val kept = mutableListOf<TraceStep>()

    /** Keep [step], after the steps kept before it. */
    fun push(step: TraceStep) {
        kept.add(step)
    }

    /** The steps kept, in the order the run took them. */
    val steps: List<TraceStep> get() = kept.toList()

    /**
     * The run in the run trace format: its version, its lane and its steps, one
     * step to a line, byte for byte as every lane lays it out.
     */
    fun toJson(): String {
        val out = StringBuilder()
        out.append("{\n  \"version\": ").append(FORMAT_VERSION.toString())
        out.append(",\n  \"lane\": ").append(Json.quote(LANE))
        out.append(",\n  \"steps\": [")
        kept.forEachIndexed { at, step ->
            out.append(if (at == 0) "\n    " else ",\n    ")
            out.append(step.toJson())
        }
        out.append(if (kept.isEmpty()) "]\n}\n" else "\n  ]\n}\n")
        return out.toString()
    }

    companion object {
        /**
         * The version of the run trace format [toJson] writes, the
         * `TRACE_FORMAT_VERSION` its reader in `sce-build` checks.
         */
        const val FORMAT_VERSION: Int = 1

        /** The lane this runtime is, as a run trace names it. */
        const val LANE: String = "kotlin"
    }
}

/**
 * The port a host set, if any, and the step being told.
 *
 * A step is opened where Appendix D begins it and closed where it ends; the
 * transitions taken and the states exited and entered between are added to it
 * as the engine takes, exits and enters each. With no port set nothing is
 * opened, so nothing is kept, and each place the engine tells it asks one
 * `null` check.
 */
internal class TraceRecorder {
    /** What [StateMachineEngine.tracePort] set: told each step once it is over. */
    var port: ((TraceStep) -> Unit)? = null
        private set

    private var open: OpenStep? = null

    /** The step being told, as the engine fills it. */
    private class OpenStep(val kind: TraceStepKind, val event: String?) {
        val fired = mutableListOf<TraceFired>()
        val exited = mutableListOf<String>()
        val entered = mutableListOf<String>()
    }

    /** Tell [port] each step from now on, or no port anything when it is `null`. */
    fun set(port: ((TraceStep) -> Unit)?) {
        this.port = port
        open = null
    }

    /** Whether a port is set, so a step would be told. */
    val isSet: Boolean get() = port != null

    /** Whether a step is being told. */
    val isOpen: Boolean get() = open != null

    /** Whether the step being told is a stop, which leaves no configuration. */
    val isStop: Boolean get() = open?.kind == TraceStepKind.STOP

    /**
     * Begin a step of [kind], which took the event named [event] when it is an
     * event step. Nothing when no port is set.
     */
    fun begin(kind: TraceStepKind, event: String?) {
        if (port != null) open = OpenStep(kind, event)
    }

    /** The step's microstep takes the transition written [index]-th in [source]. */
    fun fired(source: String, index: Int) {
        open?.fired?.add(TraceFired(source, index))
    }

    /** The step exits [state]. */
    fun exited(state: String) {
        open?.exited?.add(state)
    }

    /** The step enters [state]. */
    fun entered(state: String) {
        open?.entered?.add(state)
    }

    /**
     * The step is over: tell the port it, with the [configuration] it left, or,
     * for a stop, the top-level `<final>` the run ended in, [outcome].
     */
    fun end(configuration: List<String>, outcome: String?) {
        val told = port ?: return
        val step = open ?: return
        open = null
        val stop = step.kind == TraceStepKind.STOP
        told(
            TraceStep(
                kind = step.kind,
                event = step.event,
                fired = step.fired.toList(),
                exited = step.exited.toList(),
                entered = step.entered.toList(),
                configuration = if (stop) emptyList() else configuration,
                outcome = if (stop) outcome else null,
            )
        )
    }
}
