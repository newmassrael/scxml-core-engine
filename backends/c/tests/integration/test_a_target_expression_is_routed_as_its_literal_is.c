// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2.4 + C.1: a `targetexpr` is routed as the same value written in
// `target` is, at once or after a delay — C11 AOT, on a manual clock.
//
// Fixture:
// integration_resources/a_target_expression_is_routed_as_its_literal_is/a_target_expression_is_routed_as_its_literal_is.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_target_expression_is_routed_as_its_literal_is ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "a_target_expression_is_routed_as_its_literal_is_sm.h"

typedef a_target_expression_is_routed_as_its_literal_is_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

int main(void) {
    sm_t sm;
    a_target_expression_is_routed_as_its_literal_is_init_with_clock(&sm, sce_clock_manual(0u));
    for (int elapsed = 0; elapsed < 1000 && !a_target_expression_is_routed_as_its_literal_is_is_in_final_state(&sm);
         elapsed += 10) {
        a_target_expression_is_routed_as_its_literal_is_advance_time_ms(&sm, 10u);
    }

    int ok = 1;
    if (!a_target_expression_is_routed_as_its_literal_is_ended_in(
            &sm, A_TARGET_EXPRESSION_IS_ROUTED_AS_ITS_LITERAL_IS_STATE_DONE)) {
        fprintf(stderr, "FAIL: the run must end in `done`\n");
        ok = 0;
    }

    const struct {
        const char *name;
        reader_t read;
        int64_t want;
    } observed[] = {
        {"internalNow", a_target_expression_is_routed_as_its_literal_is_internal_now, 1},
        {"internalLater", a_target_expression_is_routed_as_its_literal_is_internal_later, 1},
        {"kidNow", a_target_expression_is_routed_as_its_literal_is_kid_now, 1},
        {"kidLater", a_target_expression_is_routed_as_its_literal_is_kid_later, 1},
        {"sessNow", a_target_expression_is_routed_as_its_literal_is_sess_now, 1},
        {"sessLater", a_target_expression_is_routed_as_its_literal_is_sess_later, 1},
        {"commErrors", a_target_expression_is_routed_as_its_literal_is_comm_errors, 4},
        {"execErrors", a_target_expression_is_routed_as_its_literal_is_exec_errors, 2},
        {"afterStranger", a_target_expression_is_routed_as_its_literal_is_after_stranger, 0},
        {"afterStrangerLater", a_target_expression_is_routed_as_its_literal_is_after_stranger_later, 0},
        {"afterOrphan", a_target_expression_is_routed_as_its_literal_is_after_orphan, 0},
        {"afterOrphanLater", a_target_expression_is_routed_as_its_literal_is_after_orphan_later, 0},
        {"afterBogus", a_target_expression_is_routed_as_its_literal_is_after_bogus, 0},
        {"afterBogusLater", a_target_expression_is_routed_as_its_literal_is_after_bogus_later, 0},
    };

    for (size_t i = 0; i < sizeof(observed) / sizeof(observed[0]); ++i) {
        int64_t got = -1;
        if (!observed[i].read(&sm, &got) || got != observed[i].want) {
            fprintf(stderr, "FAIL: %s = %lld, want %lld\n", observed[i].name, (long long)got,
                    (long long)observed[i].want);
            ok = 0;
        }
    }
    a_target_expression_is_routed_as_its_literal_is_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: each <send targetexpr> was routed as its literal is\n");
    return 0;
}
