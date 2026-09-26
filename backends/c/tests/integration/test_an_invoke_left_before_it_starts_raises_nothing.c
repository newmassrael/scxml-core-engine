// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4: an invoke whose state leaves within its macrostep is never
// attempted, so the §6.4.1 error.execution for a type no processor runs is
// never raised — C11 AOT.
//
// Measured 2026-09-26, this channel dropped a pending invoke on exit only for
// the SCXML kinds, so a state holding only a refused invoke kept its pending
// entry and raised error.execution at the end of the macrostep.
//
// Fixture:
// integration_resources/an_invoke_left_before_it_starts_raises_nothing/an_invoke_left_before_it_starts_raises_nothing.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(an_invoke_left_before_it_starts_raises_nothing ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "an_invoke_left_before_it_starts_raises_nothing_sm.h"

typedef an_invoke_left_before_it_starts_raises_nothing_t sm_t;

int main(void) {
    sm_t sm;
    an_invoke_left_before_it_starts_raises_nothing_init(&sm);
    an_invoke_left_before_it_starts_raises_nothing_run(&sm);
    if (!an_invoke_left_before_it_starts_raises_nothing_in_state(
            &sm, AN_INVOKE_LEFT_BEFORE_IT_STARTS_RAISES_NOTHING_STATE_S1)) {
        fprintf(stderr, "FAIL: `s0` leaves on an eventless transition within the first macrostep\n");
        return 1;
    }

    an_invoke_left_before_it_starts_raises_nothing_event_with_meta_t carrier = {0};
    carrier.event = AN_INVOKE_LEFT_BEFORE_IT_STARTS_RAISES_NOTHING_EVENT_FINISH;
    an_invoke_left_before_it_starts_raises_nothing_raise_external(&sm, &carrier);
    an_invoke_left_before_it_starts_raises_nothing_run(&sm);

    int ok = 1;
    if (!an_invoke_left_before_it_starts_raises_nothing_ended_in(
            &sm, AN_INVOKE_LEFT_BEFORE_IT_STARTS_RAISES_NOTHING_STATE_DONE)) {
        fprintf(stderr, "FAIL: `finish` must carry the run to `done`\n");
        ok = 0;
    }
    int64_t errors = -1;
    if (!an_invoke_left_before_it_starts_raises_nothing_errors(&sm, &errors) || errors != 0) {
        fprintf(stderr,
                "FAIL: `s0` left before its macrostep ended, yet its invoke was attempted and raised error.execution "
                "(errors=%lld); W3C SCXML 6.4 cancels an invoke whose state has left\n",
                (long long)errors);
        ok = 0;
    }
    an_invoke_left_before_it_starts_raises_nothing_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: an invoke whose state left before its macrostep ended was never attempted\n");
    return 0;
}
