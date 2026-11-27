// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A run of W3C test570 by the C11 lane is the recorded run of its model,
// `docs/formal/W3CTest570.tla` over `AppendixD.tla` (docs/adr/0115, D4 and D8;
// run_trace_recording.h). Its document completes a <parallel> when its last
// region does, and raises the done.state events in order.
//
// This target builds the machine the conformance runner aot_tests/test_570.c
// builds, with `SCE_TRACE_PORT=1`, so it calls the hook below once each step it
// takes is over. That runner builds it with the port off and defines no hook,
// which links only because nothing of the port is left in it.

#include <stdio.h>

#include "../require_started.h"
#include "run_trace_recording.h"
#include "test570_sm.h"

#if !TEST570_TRACE_PORT
#error "this target is built with SCE_TRACE_PORT=1 (backends/c/tests/CMakeLists.txt)"
#endif

static const test570_t *traced_machine;
static int told_another_machine;

void test570_trace_step(const test570_t *sm, const sce_trace_step_t *step) {
    if (sm != traced_machine) {
        told_another_machine = 1;
    }
    run_trace_told(step);
}

int main(void) {
    test570_t sm;
    traced_machine = &sm;
    run_trace_begin();
    REQUIRE_STARTED(test570_init(&sm), TEST570_START_OK);
    test570_run(&sm);

    int bad = 0;
    if (!test570_ended_in(&sm, TEST570_STATE_PASS)) {
        (void)fprintf(stderr, "test570: FAIL - the traced run did not end in `pass`\n");
        bad = 1;
    }
    test570_destroy(&sm);
    if (told_another_machine) {
        (void)fprintf(stderr, "test570: FAIL - the hook was handed another machine than the one run\n");
        bad = 1;
    }
    bad |= run_trace_starts_and_stops_in_pass();
    bad |= run_trace_holds_to_the_recorded_run("W3CTest570");
    return bad;
}
