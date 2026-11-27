// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A run of W3C test403c by the C11 lane is the recorded run of its model,
// `docs/formal/W3CTest403c.tla` over `AppendixD.tla` (docs/adr/0115, D4 and D8;
// run_trace_recording.h). Its document takes the optimal enabled transition
// set of a <parallel>, one region preempting another.
//
// This target builds the machine the conformance runner aot_tests/test_403c.c
// builds, with `SCE_TRACE_PORT=1`, so it calls the hook below once each step it
// takes is over. That runner builds it with the port off and defines no hook,
// which links only because nothing of the port is left in it.

#include <stdio.h>

#include "../require_started.h"
#include "run_trace_recording.h"
#include "test403c_sm.h"

#if !TEST403C_TRACE_PORT
#error "this target is built with SCE_TRACE_PORT=1 (backends/c/tests/CMakeLists.txt)"
#endif

static const test403c_t *traced_machine;
static int told_another_machine;

void test403c_trace_step(const test403c_t *sm, const sce_trace_step_t *step) {
    if (sm != traced_machine) {
        told_another_machine = 1;
    }
    run_trace_told(step);
}

int main(void) {
    test403c_t sm;
    traced_machine = &sm;
    run_trace_begin();
    REQUIRE_STARTED(test403c_init(&sm), TEST403C_START_OK);
    test403c_run(&sm);

    int bad = 0;
    if (!test403c_ended_in(&sm, TEST403C_STATE_PASS)) {
        (void)fprintf(stderr, "test403c: FAIL - the traced run did not end in `pass`\n");
        bad = 1;
    }
    test403c_destroy(&sm);
    if (told_another_machine) {
        (void)fprintf(stderr, "test403c: FAIL - the hook was handed another machine than the one run\n");
        bad = 1;
    }
    bad |= run_trace_starts_and_stops_in_pass();
    bad |= run_trace_holds_to_the_recorded_run("W3CTest403c");
    return bad;
}
