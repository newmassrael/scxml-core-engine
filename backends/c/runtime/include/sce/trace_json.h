/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * A run a C11 machine told its trace port (`sce/trace_port.h`), written in the
 * one run trace format every lane writes (docs/adr/0115, D4 and D8;
 * `tests/forge/conformance/run_trace.schema.json`, read by sce-build's
 * `run_trace`), which `sce-codegen formal traces` judges against the model a
 * traces manifest names.
 *
 * The text is laid out as the Rust runtime's `trace_port::RunTrace` writes it:
 * the version, the lane and the steps, one step to a line, each with the fields
 * its kind says in the order the format lists them. So two lanes' runs of one
 * document differ, line for line, only where the runs do.
 *
 * It writes through the saved state's JSON writer (`sce/saved_json.h`), whose
 * escapes are the one set every engine writes, into a buffer the host owns, and
 * counts what did not fit: a run written into no buffer is measured. Hosted, as
 * that writer is, so a machine's core never includes it (docs/adr/0110, D2).
 */
#ifndef SCE_TRACE_JSON_H
#define SCE_TRACE_JSON_H

#include <stdbool.h>
#include <stddef.h>

#include "sce/saved_json.h"
#include "sce/trace_port.h"

#ifdef __cplusplus
extern "C" {
#endif

static inline void sce_trace_json_names_(sce_jw_t *w, const char *field, const char *const *names, size_t count) {
    sce_jw_raw(w, ", ");
    sce_jw_string(w, field);
    sce_jw_raw(w, ": [");
    for (size_t at = 0u; at < count; ++at) {
        if (at > 0u) {
            sce_jw_raw(w, ", ");
        }
        sce_jw_string(w, names[at]);
    }
    sce_jw_raw(w, "]");
}

/* `step` as one object of the run trace format: the fields its kind says, in
 * the order the format lists them. False, and nothing written, for a step that
 * lost a name (`lost`), which is not the step the machine took. */
static inline bool sce_trace_json_step(sce_jw_t *w, const sce_trace_step_t *step) {
    if (step->lost) {
        return false;
    }
    sce_jw_raw(w, "{\"kind\": ");
    sce_jw_string(w, sce_trace_kind_name(step->kind));
    switch (step->kind) {
    case SCE_TRACE_START:
        sce_trace_json_names_(w, "entered", step->entered, step->entered_count);
        sce_trace_json_names_(w, "configuration", step->configuration, step->configuration_count);
        break;
    case SCE_TRACE_EVENTLESS:
    case SCE_TRACE_INTERNAL:
    case SCE_TRACE_EXTERNAL:
        if (step->kind != SCE_TRACE_EVENTLESS) {
            sce_jw_raw(w, ", \"event\": ");
            sce_jw_string(w, step->event != NULL ? step->event : "");
        }
        sce_jw_raw(w, ", \"fired\": [");
        for (size_t at = 0u; at < step->fired_count; ++at) {
            if (at > 0u) {
                sce_jw_raw(w, ", ");
            }
            sce_jw_raw(w, "{\"source\": ");
            sce_jw_string(w, step->fired[at].source);
            sce_jw_raw(w, ", \"index\": ");
            sce_jw_uint(w, step->fired[at].index);
            sce_jw_raw(w, "}");
        }
        sce_jw_raw(w, "]");
        sce_trace_json_names_(w, "exited", step->exited, step->exited_count);
        sce_trace_json_names_(w, "entered", step->entered, step->entered_count);
        sce_trace_json_names_(w, "configuration", step->configuration, step->configuration_count);
        break;
    case SCE_TRACE_STOP:
        sce_trace_json_names_(w, "exited", step->exited, step->exited_count);
        sce_jw_raw(w, ", \"outcome\": ");
        if (step->outcome != NULL) {
            sce_jw_string(w, step->outcome);
        } else {
            sce_jw_null(w);
        }
        break;
    }
    sce_jw_raw(w, "}");
    return true;
}

/* A run being written: the text so far, how many steps it holds, and whether a
 * step could not be written. */
typedef struct sce_trace_json_run_s {
    sce_jw_t w;
    size_t steps;
    bool lost;
} sce_trace_json_run_t;

/* Begin a run's text in `buf` (`cap` bytes, or NULL to measure it): its version
 * and its lane, and the steps to follow. */
static inline void sce_trace_json_run_begin(sce_trace_json_run_t *run, char *buf, size_t cap) {
    sce_jw_init(&run->w, buf, cap);
    run->steps = 0u;
    run->lost = false;
    sce_jw_raw(&run->w, "{\n  \"version\": ");
    sce_jw_uint(&run->w, SCE_TRACE_FORMAT_VERSION);
    sce_jw_raw(&run->w, ",\n  \"lane\": ");
    sce_jw_string(&run->w, SCE_TRACE_LANE);
    sce_jw_raw(&run->w, ",\n  \"steps\": [");
}

/* Write `step` after the steps written before it, on a line of its own. A step
 * that lost a name is not written, and the run is then refused by
 * `sce_trace_json_run_end`. */
static inline void sce_trace_json_run_step(sce_trace_json_run_t *run, const sce_trace_step_t *step) {
    if (step->lost) {
        run->lost = true;
        return;
    }
    sce_jw_raw(&run->w, run->steps == 0u ? "\n    " : ",\n    ");
    (void)sce_trace_json_step(&run->w, step);
    ++run->steps;
}

/* End the run's text and terminate it. True when the whole run was written and
 * fit; false when a step was lost, or when the buffer is short, which is then
 * one of `run->w.len + 1` bytes to write it again into. */
static inline bool sce_trace_json_run_end(sce_trace_json_run_t *run) {
    sce_jw_raw(&run->w, run->steps == 0u ? "]\n}\n" : "\n  ]\n}\n");
    return sce_jw_finish(&run->w) && !run->lost;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_TRACE_JSON_H */
