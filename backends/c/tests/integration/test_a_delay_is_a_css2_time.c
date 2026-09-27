// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2: a <send> delay is read as one CSS2 time — C11 AOT.
//
// The machine's clock is advanced a full minute: both valid delays fire in the
// order their milliseconds give, and a refused message, had it been scheduled
// under some default wait, would have arrived and moved `bad`.
//
// Fixture: integration_resources/a_delay_is_a_css2_time/a_delay_is_a_css2_time.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_delay_is_a_css2_time ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "a_delay_is_a_css2_time_sm.h"

typedef a_delay_is_a_css2_time_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

int main(void) {
    sm_t sm;
    // The host owns the clock, so a minute passes when the test says so.
    a_delay_is_a_css2_time_init_with_clock(&sm, sce_clock_manual(0u));
    a_delay_is_a_css2_time_run(&sm);
    a_delay_is_a_css2_time_advance_time_ms(&sm, 60000u);
    a_delay_is_a_css2_time_run(&sm);

    int ok = 1;
    if (!a_delay_is_a_css2_time_ended_in(&sm, A_DELAY_IS_A_CSS2_TIME_STATE_DONE)) {
        fprintf(stderr, "FAIL: `a` must carry the run to `done`\n");
        ok = 0;
    }

    const struct {
        const char *name;
        reader_t read;
        int64_t want;
    } observed[] = {
        {"errors", a_delay_is_a_css2_time_errors, 2},
        {"after", a_delay_is_a_css2_time_after, 0},
        {"bad", a_delay_is_a_css2_time_bad, 0},
        {"aAfterB", a_delay_is_a_css2_time_a_after_b, 1},
    };

    for (size_t i = 0; i < sizeof(observed) / sizeof(observed[0]); ++i) {
        int64_t got = -1;
        if (!observed[i].read(&sm, &got) || got != observed[i].want) {
            fprintf(stderr, "FAIL: %s = %lld, want %lld\n", observed[i].name, (long long)got,
                    (long long)observed[i].want);
            ok = 0;
        }
    }
    a_delay_is_a_css2_time_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: each delay was read as one CSS2 time\n");
    return 0;
}
