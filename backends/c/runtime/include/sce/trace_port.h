/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * The trace port of a C11 machine: what a run tells a host of each step
 * Appendix D takes, so the run can be judged a behaviour of the document's
 * model (docs/adr/0115, D4 and D8).
 *
 * A step is the start of the interpreter, a microstep on eventless transitions,
 * the taking of an internal or an external event (with the microstep it
 * selects, or none), or the exit of the interpreter: the steps
 * `docs/formal/AppendixD.tla` names `Start`, `Eventless`, `Internal`,
 * `External` and `Stop`. Each is told once it is over, with what its record
 * `told` holds there: the transitions it took by their rows, the states it
 * exited in the order it exited them, the states it entered in the order it
 * entered them, and the configuration it left, or, for a stop, the top-level
 * `<final>` the run ended in.
 *
 * A machine tells its steps only when its build asks for it
 * (`<PREFIX><NAME>_TRACE_PORT`, whose default is `SCE_TRACE_PORT` in
 * `sce/types.h`, 0 unless set). On, the machine holds a recorder below, opens a
 * step where Appendix D begins it, adds each transition, exit and entry as it
 * takes it, and hands the step to `<prefix><name>_trace_step`, which the host
 * defines, when it is over. Off, the preprocessor leaves nothing of it in the
 * machine. The port reads and is never asked anything, so a machine that tells
 * its steps takes the steps one that does not takes.
 *
 * What a step names is the run trace format's (`tests/forge/conformance/
 * run_trace.schema.json`, read by sce-build's `run_trace`), and
 * `sce/trace_json.h` writes it in that format, which `sce-codegen formal
 * traces` judges. An event from outside, and a delayed send that comes due, are
 * steps of the environment and not of the interpreter (AppendixD's `Arrive` and
 * `Due`): the machine cannot tell where an event on its external queue came
 * from, and the external step that takes it names it.
 *
 * The cross-language sibling is the Rust runtime's `trace_port` module (feature
 * `trace-port`). Header-only and allocation-free, and freestanding: a machine's
 * core includes it (docs/adr/0110, D1), and the writer, which is hosted, is
 * `sce/trace_json.h`.
 */
#ifndef SCE_TRACE_PORT_H
#define SCE_TRACE_PORT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* The version of the run trace format this port's steps are written in, the
 * `TRACE_FORMAT_VERSION` its reader in sce-build checks. */
#define SCE_TRACE_FORMAT_VERSION 1u

/* The lane this runtime is, as a run trace names it. */
#define SCE_TRACE_LANE "c11"

/* The most names one list of a step holds. A configuration is a bitmap of 32
 * states on this backend (the generated header refuses a document of more), a
 * step exits and enters each state at most once, and its microstep takes at
 * most one transition per active atomic state, so no list of a step of any
 * machine of this backend names more. A machine that tells its steps holds
 * itself to the bound at compile time. */
#define SCE_TRACE_MAX_NAMES 32u

/* Which step of Appendix D a step is. */
typedef enum sce_trace_kind_e {
    /* AppendixD's `Start`: the entry into the initial configuration. */
    SCE_TRACE_START = 0,
    /* AppendixD's `Eventless`: a microstep on eventless transitions. */
    SCE_TRACE_EVENTLESS,
    /* AppendixD's `Internal`: an event taken off the internal queue, and the
     * microstep it selects, if it selects one. */
    SCE_TRACE_INTERNAL,
    /* AppendixD's `External`: an event taken off the external queue, and the
     * microstep it selects, if it selects one. */
    SCE_TRACE_EXTERNAL,
    /* AppendixD's `Stop`: every state still active exited. */
    SCE_TRACE_STOP
} sce_trace_kind_t;

/* The step's name in the run trace format. */
static inline const char *sce_trace_kind_name(sce_trace_kind_t kind) {
    switch (kind) {
    case SCE_TRACE_START:
        return "start";
    case SCE_TRACE_EVENTLESS:
        return "eventless";
    case SCE_TRACE_INTERNAL:
        return "internal";
    case SCE_TRACE_EXTERNAL:
        return "external";
    case SCE_TRACE_STOP:
        return "stop";
    }
    return "";
}

/* A transition a step took, by the pair its row of the emitted transition table
 * is named by (docs/adr/0089): the state it is written in, by its id, and its
 * position among that state's own transitions in document order, from 0. */
typedef struct sce_trace_fired_s {
    const char *source;
    uint32_t index;
} sce_trace_fired_t;

/* One step a run took, as the host is told it.
 *
 * Each kind says what the run trace format asks of it and no more: a start what
 * it entered and the configuration; a microstep, eventless or on an event, what
 * it took, exited and entered and the configuration; an event step the event
 * too; a stop what it exited and the top-level final state the run ended in,
 * or NULL. What a kind does not say is empty. Every text a step names is the
 * machine's and valid for the length of the call that hands the step over: a
 * host that keeps a step past it keeps its own copy. */
typedef struct sce_trace_step_s {
    sce_trace_kind_t kind;
    /* The name of the event an internal or external step took, as `_event.name`
     * holds it while it is taken: the whole name it was raised or sent under,
     * not the event of the document a door resolved it to; NULL otherwise. */
    const char *event;
    /* The transitions the step's microstep took, in the order they were selected. */
    sce_trace_fired_t fired[SCE_TRACE_MAX_NAMES];
    size_t fired_count;
    /* The states exited, in the order they were exited. */
    const char *exited[SCE_TRACE_MAX_NAMES];
    size_t exited_count;
    /* The states entered, in the order they were entered. */
    const char *entered[SCE_TRACE_MAX_NAMES];
    size_t entered_count;
    /* The configuration the step left, in document order; empty for a stop. */
    const char *configuration[SCE_TRACE_MAX_NAMES];
    size_t configuration_count;
    /* The top-level `<final>` a stop ended the run in, or NULL. */
    const char *outcome;
    /* A list was told more names than it holds, so the step is not whole and is
     * written by no writer. A machine's compile-time bound keeps this false. */
    bool lost;
} sce_trace_step_t;

/* The step being told. A step is opened where Appendix D begins it and closed
 * where it ends; the transitions, exits and entries between are added to it as
 * the machine takes each. Nothing is added while no step is open, so an exit or
 * an entry that is no part of a step (a restore setting a configuration) is
 * told to no one. Zeroed, it holds no open step. */
typedef struct sce_trace_recorder_s {
    sce_trace_step_t step;
    bool open;
} sce_trace_recorder_t;

/* Open a step of `kind`, which took the event named `event` when it is an event
 * step (NULL otherwise), in place of any step left open. */
static inline void sce_trace_begin(sce_trace_recorder_t *recorder, sce_trace_kind_t kind, const char *event) {
    sce_trace_step_t *step = &recorder->step;
    step->kind = kind;
    step->event = event;
    step->fired_count = 0u;
    step->exited_count = 0u;
    step->entered_count = 0u;
    step->configuration_count = 0u;
    step->outcome = NULL;
    step->lost = false;
    recorder->open = true;
}

/* Whether a step is being told. */
static inline bool sce_trace_is_open(const sce_trace_recorder_t *recorder) {
    return recorder->open;
}

/* Add `name` at the end of one list of the open step, or mark the step lost
 * when the list is full. */
static inline void sce_trace_add_name_(sce_trace_recorder_t *recorder, const char **list, size_t *count,
                                       const char *name) {
    if (!recorder->open) {
        return;
    }
    if (*count >= (size_t)SCE_TRACE_MAX_NAMES) {
        recorder->step.lost = true;
        return;
    }
    list[*count] = name;
    ++*count;
}

/* The open step's microstep takes the transition written `index`-th in the
 * state named `source`. */
static inline void sce_trace_fired(sce_trace_recorder_t *recorder, const char *source, uint32_t index) {
    sce_trace_step_t *step = &recorder->step;
    if (!recorder->open) {
        return;
    }
    if (step->fired_count >= (size_t)SCE_TRACE_MAX_NAMES) {
        step->lost = true;
        return;
    }
    step->fired[step->fired_count].source = source;
    step->fired[step->fired_count].index = index;
    ++step->fired_count;
}

/* The open step exits the state named `state`. */
static inline void sce_trace_exited(sce_trace_recorder_t *recorder, const char *state) {
    sce_trace_add_name_(recorder, recorder->step.exited, &recorder->step.exited_count, state);
}

/* The open step enters the state named `state`. */
static inline void sce_trace_entered(sce_trace_recorder_t *recorder, const char *state) {
    sce_trace_add_name_(recorder, recorder->step.entered, &recorder->step.entered_count, state);
}

/* The state named `state` is active once the open step is over. */
static inline void sce_trace_configuration(sce_trace_recorder_t *recorder, const char *state) {
    sce_trace_add_name_(recorder, recorder->step.configuration, &recorder->step.configuration_count, state);
}

/* The open stop ended the run in the top-level `<final>` named `outcome`, or in
 * none when it is NULL. */
static inline void sce_trace_outcome(sce_trace_recorder_t *recorder, const char *outcome) {
    if (recorder->open) {
        recorder->step.outcome = outcome;
    }
}

/* Close the open step and hand it back, to be told to the host; NULL when no
 * step was open. */
static inline const sce_trace_step_t *sce_trace_close(sce_trace_recorder_t *recorder) {
    if (!recorder->open) {
        return NULL;
    }
    recorder->open = false;
    return &recorder->step;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_TRACE_PORT_H */
