// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Each scenario of the properties judge's turnstile, replayed by the C11 lane
// (docs/adr/0115, D15 and the converse of D8). A scenario is the run TLC gave
// of a violation or a witness, with the events its environment put on the
// external queue; this program gives the machine those events, on a manual
// clock, with its trace port on, and holds the run the port is told to the
// recorded run `<name>.c11.json` beside the scenario, byte for byte
// (run_trace_recording.h). `sce_build::formal::replay` judges each recorded
// run to be the scenario's, step for step, as it judges the Rust lane's.
//
// The harness gives an input once the machine has taken every step it can,
// which is after the step the input names unless the model's environment put
// it on the queue in the middle of a macrostep. A macrostep that also sends to
// its own external queue after that point would take the two in another order
// here; the judge then finds the runs part, which is a defect of this harness
// to settle, and not a verdict about the document. A delayed send comes due by
// the clock moving to the next deadline, which is the earliest: a scenario in
// which the model's time makes a later one due first is one no clock replays,
// and the judge finds that too.
//
// The scenarios are the arguments, as the build globs them
// (backends/c/tests/CMakeLists.txt). Recording again, after a change that moves
// a run on purpose: `UPDATE_EXPECT=1 ctest -R c11_a_scenario_of_the_turnstile`.
//
// Fixture: sce-build/tests/fixtures/formal_properties/turnstile.scxml

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "../require_started.h"
#include "run_trace_recording.h"
#include "scenario_inputs.h"
#include "turnstile_sm.h"

#if !TURNSTILE_TRACE_PORT
#error "this target is built with SCE_TRACE_PORT=1 (backends/c/tests/CMakeLists.txt)"
#endif

#define SCENARIO_SUFFIX ".scenario.json"

static const turnstile_t *traced_machine;
static int told_another_machine;
static size_t told_count;

void turnstile_trace_step(const turnstile_t *sm, const sce_trace_step_t *step) {
    if (sm != traced_machine) {
        told_another_machine = 1;
    }
    ++told_count;
    run_trace_told(step);
}

// The name of the scenario at `path`: its file's name without its suffix.
static int scenario_name(const char *path, char *out, size_t cap) {
    const char *slash = strrchr(path, '/');
    const char *base = slash != NULL ? slash + 1 : path;
    const size_t len = strlen(base);
    const size_t suffix = strlen(SCENARIO_SUFFIX);
    if (len <= suffix || strcmp(base + len - suffix, SCENARIO_SUFFIX) != 0 || len - suffix >= cap) {
        return 0;
    }
    memcpy(out, base, len - suffix);
    out[len - suffix] = '\0';
    return 1;
}

// Give the machine each input of the scenario at `path`, and hold the run to
// its recorded one. 0 when it holds, or was recorded.
static int replay(const char *path) {
    static scenario_t scenario;
    char name[256];
    if (!scenario_name(path, name, sizeof(name))) {
        (void)fprintf(stderr, "scenario: FAIL - %s is not named <name>%s\n", path, SCENARIO_SUFFIX);
        return 1;
    }
    if (!scenario_read(path, &scenario)) {
        (void)fprintf(stderr, "scenario: FAIL - the inputs of %s are not read\n", path);
        return 1;
    }
    turnstile_t sm;
    traced_machine = &sm;
    told_count = 0u;
    run_trace_begin();
    REQUIRE_STARTED(turnstile_init_with_clock(&sm, sce_clock_manual(0u)), TURNSTILE_START_OK);
    int bad = 0;
    for (size_t i = 0u; i < scenario.count && !bad; ++i) {
        const scenario_input_t *input = &scenario.inputs[i];
        if (told_count < input->after) {
            (void)fprintf(stderr,
                          "scenario: FAIL - %s: the machine took %zu steps, and the model %zu before `%s` "
                          "came\n",
                          name, told_count, input->after, input->event);
            bad = 1;
            break;
        }
        if (input->kind == SCENARIO_ARRIVE) {
            turnstile_event_t event;
            if (!turnstile_resolve_event_by_name(input->event, &event)) {
                (void)fprintf(stderr, "scenario: FAIL - %s: `%s` is no event of the turnstile\n", name, input->event);
                bad = 1;
                break;
            }
            turnstile_event_with_meta_t meta;
            memset(&meta, 0, sizeof(meta));
            meta.event = event;
            turnstile_raise_external(&sm, &meta);
            turnstile_step(&sm);
        } else {
            const int64_t wait = turnstile_time_until_next_scheduled_ms(&sm);
            if (wait < 0) {
                (void)fprintf(stderr, "scenario: FAIL - %s: `%s` comes due, and the machine holds no delayed send\n",
                              name, input->event);
                bad = 1;
                break;
            }
            turnstile_advance_time_ms(&sm, (uint64_t)wait);
        }
    }
    turnstile_destroy(&sm);
    if (told_another_machine) {
        (void)fprintf(stderr, "scenario: FAIL - the hook was handed another machine than the one run\n");
        bad = 1;
    }
    if (!bad) {
        bad = run_trace_holds_to_the_recorded_run(name);
    }
    return bad;
}

int main(int argc, char **argv) {
    if (argc < 2) {
        (void)fprintf(stderr,
                      "scenario: FAIL - no scenario was given: write them with 'sce-codegen formal properties "
                      "--scenarios sce-build/tests/fixtures/formal_properties/scenarios' and configure again\n");
        return 1;
    }
    int bad = 0;
    for (int i = 1; i < argc; ++i) {
        bad |= replay(argv[i]);
    }
    return bad;
}
