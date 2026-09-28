// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a delay postpones a <send>, it does not change
// where the send goes — C11 AOT, on a manual clock so the order of two events
// due at one instant is a verdict about the engine, not about the host.
//
// Fixture:
// integration_resources/a_delayed_send_reaches_what_its_target_names/a_delayed_send_reaches_what_its_target_names.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_delayed_send_reaches_what_its_target_names ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "a_delayed_send_reaches_what_its_target_names_sm.h"

typedef a_delayed_send_reaches_what_its_target_names_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

int main(void) {
    sm_t sm;
    a_delayed_send_reaches_what_its_target_names_init_with_clock(&sm, sce_clock_manual(0u));
    for (int elapsed = 0; elapsed < 1000 && !a_delayed_send_reaches_what_its_target_names_is_in_final_state(&sm);
         elapsed += 10) {
        a_delayed_send_reaches_what_its_target_names_advance_time_ms(&sm, 10u);
    }

    int ok = 1;
    if (!a_delayed_send_reaches_what_its_target_names_ended_in(
            &sm, A_DELAYED_SEND_REACHES_WHAT_ITS_TARGET_NAMES_STATE_DONE)) {
        fprintf(stderr, "FAIL: the run must end in `done`\n");
        ok = 0;
    }

    const struct {
        const char *name;
        reader_t read;
        int64_t want;
    } observed[] = {
        {"order", a_delayed_send_reaches_what_its_target_names_order, 31},
        {"innerInternal", a_delayed_send_reaches_what_its_target_names_inner_internal, 1},
        {"lateOk", a_delayed_send_reaches_what_its_target_names_late_ok, 1},
        {"lateCount", a_delayed_send_reaches_what_its_target_names_late_count, 1},
        {"pongOk", a_delayed_send_reaches_what_its_target_names_pong_ok, 1},
        {"commErrors", a_delayed_send_reaches_what_its_target_names_comm_errors, 2},
        {"lostArrived", a_delayed_send_reaches_what_its_target_names_lost_arrived, 0},
        {"afterStranger", a_delayed_send_reaches_what_its_target_names_after_stranger, 0},
    };

    for (size_t i = 0; i < sizeof(observed) / sizeof(observed[0]); ++i) {
        int64_t got = -1;
        if (!observed[i].read(&sm, &got) || got != observed[i].want) {
            fprintf(stderr, "FAIL: %s = %lld, want %lld\n", observed[i].name, (long long)got,
                    (long long)observed[i].want);
            ok = 0;
        }
    }
    a_delayed_send_reaches_what_its_target_names_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: each delayed <send> reached what its target named\n");
    return 0;
}
