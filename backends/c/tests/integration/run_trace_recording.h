// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A run of the C11 lane, recorded as the trace `sce-codegen formal traces`
// judges a behaviour of the document's model (docs/adr/0115, D4 and D8). The
// twin of the Rust lane's `a_run_is_a_behaviour_of_its_model`
// (backends/rust/tests).
//
// A driver builds one W3C document that `docs/formal` has a model of with its
// machine's trace port on (`SCE_TRACE_PORT=1`), hands each step its hook is
// told to `run_trace_told`, runs the machine to its end, and holds what was
// told to the recorded run `docs/formal/traces.json` declares of that model,
// byte for byte. The judge reads the recorded file, so a run that moves is
// seen here first, and a recorded run that no longer is the lane's is never
// judged in its place. A C11 run of a document is judged against the same
// generated model as the Rust run of it, and these documents leave a run's
// steps no choice, so the two recorded runs differ only in their lane, or the
// judge refuses one.
//
// Recording again, after a change that moves a run on purpose:
//   UPDATE_EXPECT=1 ctest -R c11_a_run_of_test_
// then `sce-codegen formal traces --manifest docs/formal/traces.json`, and read
// the diff of the recorded runs.

#ifndef SCE_C_TESTS_RUN_TRACE_RECORDING_H
#define SCE_C_TESTS_RUN_TRACE_RECORDING_H

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <sce/trace_json.h>
#include <sce/trace_port.h>

#ifndef SCE_FORMAL_TRACES_DIR
#error "SCE_FORMAL_TRACES_DIR names docs/formal/traces; the build defines it"
#endif

// Room for a whole recorded run: the longest of the Rust lane's is under 3 KiB.
#define RUN_TRACE_CAP 65536u

static char run_trace_text[RUN_TRACE_CAP];
static sce_trace_json_run_t run_trace;

// What the first and the last step told said, which every recorded run must
// hold: a start first, and a stop last that exits the state the run ended in.
static size_t run_trace_count;
static sce_trace_kind_t run_trace_first_kind;
static sce_trace_kind_t run_trace_last_kind;
static const char *run_trace_last_outcome;
static size_t run_trace_last_exited_count;
static const char *run_trace_last_exited;

// Begin recording, before the machine is started: `_init` takes the first steps.
static void run_trace_begin(void) {
    sce_trace_json_run_begin(&run_trace, run_trace_text, sizeof(run_trace_text));
    run_trace_count = 0u;
}

// One step the machine's hook was told, written as it is told: what it names is
// valid only for the length of the hook's call. The names a stop says are the
// machine's own state names, which live as long as the program.
static void run_trace_told(const sce_trace_step_t *step) {
    sce_trace_json_run_step(&run_trace, step);
    if (run_trace_count == 0u) {
        run_trace_first_kind = step->kind;
    }
    ++run_trace_count;
    run_trace_last_kind = step->kind;
    run_trace_last_outcome = step->outcome;
    run_trace_last_exited_count = step->exited_count;
    run_trace_last_exited = step->exited_count > 0u ? step->exited[0] : NULL;
}

// The run starts, ends in `pass`, and the interpreter's exit leaves that state
// alone: the first and the last step a recorded run must hold. 0 when it does.
static int run_trace_starts_and_stops_in_pass(void) {
    int bad = 0;
    if (run_trace_count == 0u || run_trace_first_kind != SCE_TRACE_START) {
        (void)fprintf(stderr, "run trace: FAIL - the run does not begin with a start step\n");
        bad = 1;
    }
    if (run_trace_count == 0u || run_trace_last_kind != SCE_TRACE_STOP) {
        (void)fprintf(stderr, "run trace: FAIL - the run does not end with a stop step\n");
        bad = 1;
    }
    if (run_trace_last_outcome == NULL || strcmp(run_trace_last_outcome, "pass") != 0) {
        (void)fprintf(stderr, "run trace: FAIL - the stop names no outcome 'pass'\n");
        bad = 1;
    }
    if (run_trace_last_exited_count != 1u || run_trace_last_exited == NULL ||
        strcmp(run_trace_last_exited, "pass") != 0) {
        (void)fprintf(stderr, "run trace: FAIL - the stop does not exit 'pass' alone\n");
        bad = 1;
    }
    return bad;
}

// Hold the run told to the recorded run of `model`, or record it again when
// `UPDATE_EXPECT` is set. 0 when it holds, or was recorded.
static int run_trace_holds_to_the_recorded_run(const char *model) {
    if (!sce_trace_json_run_end(&run_trace)) {
        (void)fprintf(stderr,
                      "run trace: FAIL - the run of %s was not written whole (a step lost a name, or it is past "
                      "%u bytes: %zu)\n",
                      model, (unsigned)RUN_TRACE_CAP, run_trace.w.len);
        return 1;
    }
    char path[1024];
    const int path_len = snprintf(path, sizeof(path), "%s/%s.c11.json", SCE_FORMAL_TRACES_DIR, model);
    if (path_len < 0 || (size_t)path_len >= sizeof(path)) {
        (void)fprintf(stderr, "run trace: FAIL - the path of the recorded run of %s does not fit\n", model);
        return 1;
    }
    const size_t len = run_trace.w.len;
    if (getenv("UPDATE_EXPECT") != NULL) {
        FILE *out = fopen(path, "wb");
        if (out == NULL) {
            (void)fprintf(stderr, "run trace: FAIL - %s is not writable\n", path);
            return 1;
        }
        const size_t wrote = fwrite(run_trace_text, 1u, len, out);
        const int closed = fclose(out);
        if (wrote != len || closed != 0) {
            (void)fprintf(stderr, "run trace: FAIL - %s was not written whole\n", path);
            return 1;
        }
        return 0;
    }
    FILE *in = fopen(path, "rb");
    if (in == NULL) {
        (void)fprintf(stderr,
                      "run trace: FAIL - %s cannot be read: record it with 'UPDATE_EXPECT=1 ctest -R "
                      "c11_a_run_of_test_'\n",
                      path);
        return 1;
    }
    static char recorded[RUN_TRACE_CAP + 1u];
    const size_t got = fread(recorded, 1u, sizeof(recorded), in);
    (void)fclose(in);
    if (got != len || memcmp(recorded, run_trace_text, len) != 0) {
        size_t at = 0u;
        while (at < len && at < got && recorded[at] == run_trace_text[at]) {
            ++at;
        }
        (void)fprintf(stderr,
                      "run trace: FAIL - the run of %s is not the recorded one that 'sce-codegen formal traces' "
                      "judges: they part at byte %zu.\nThe run:\n%s\n",
                      model, at, run_trace_text);
        return 1;
    }
    return 0;
}

#endif
