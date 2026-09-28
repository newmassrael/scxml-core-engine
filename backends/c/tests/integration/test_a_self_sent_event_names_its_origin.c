// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML C.1: an event a session sends to itself names its origin, and a
// target expression that evaluates to nothing reaches no one — C11 AOT.
//
// The machine's clock is advanced past the delayed send, so every phase has
// run when the counters are read.
//
// Fixture: integration_resources/a_self_sent_event_names_its_origin/a_self_sent_event_names_its_origin.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_self_sent_event_names_its_origin ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "a_self_sent_event_names_its_origin_sm.h"

typedef a_self_sent_event_names_its_origin_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

int main(void) {
    sm_t sm;
    // The host owns the clock, so the delay passes when the test says so.
    a_self_sent_event_names_its_origin_init_with_clock(&sm, sce_clock_manual(0u));
    a_self_sent_event_names_its_origin_run(&sm);
    a_self_sent_event_names_its_origin_advance_time_ms(&sm, 1000u);
    a_self_sent_event_names_its_origin_run(&sm);

    int ok = 1;
    if (!a_self_sent_event_names_its_origin_ended_in(&sm, A_SELF_SENT_EVENT_NAMES_ITS_ORIGIN_STATE_DONE)) {
        fprintf(stderr, "FAIL: the run must end in `done`\n");
        ok = 0;
    }

    const struct {
        const char *name;
        reader_t read;
        int64_t want;
    } observed[] = {
        {"immediateOk", a_self_sent_event_names_its_origin_immediate_ok, 1},
        {"replied", a_self_sent_event_names_its_origin_replied, 1},
        {"delayedOk", a_self_sent_event_names_its_origin_delayed_ok, 1},
        {"unreachable", a_self_sent_event_names_its_origin_unreachable, 1},
        {"strayed", a_self_sent_event_names_its_origin_strayed, 0},
    };

    for (size_t i = 0; i < sizeof(observed) / sizeof(observed[0]); ++i) {
        int64_t got = -1;
        if (!observed[i].read(&sm, &got) || got != observed[i].want) {
            fprintf(stderr, "FAIL: %s = %lld, want %lld\n", observed[i].name, (long long)got,
                    (long long)observed[i].want);
            ok = 0;
        }
    }
    a_self_sent_event_names_its_origin_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: a self-sent event named its origin\n");
    return 0;
}
