// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 4.9: an error ends the block it was raised in — whichever element
// raised it — and no other block — C11 AOT.
//
// Fixture:
// integration_resources/an_error_ends_the_block_it_was_raised_in/an_error_ends_the_block_it_was_raised_in.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(an_error_ends_the_block_it_was_raised_in ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "an_error_ends_the_block_it_was_raised_in_sm.h"

typedef an_error_ends_the_block_it_was_raised_in_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

int main(void) {
    sm_t sm;
    an_error_ends_the_block_it_was_raised_in_init(&sm);
    an_error_ends_the_block_it_was_raised_in_run(&sm);

    const an_error_ends_the_block_it_was_raised_in_event_t events[] = {
        AN_ERROR_ENDS_THE_BLOCK_IT_WAS_RAISED_IN_EVENT_T,
        AN_ERROR_ENDS_THE_BLOCK_IT_WAS_RAISED_IN_EVENT_FINISH,
    };
    for (size_t i = 0; i < sizeof(events) / sizeof(events[0]); ++i) {
        an_error_ends_the_block_it_was_raised_in_event_with_meta_t carrier = {0};
        carrier.event = events[i];
        an_error_ends_the_block_it_was_raised_in_raise_external(&sm, &carrier);
        an_error_ends_the_block_it_was_raised_in_run(&sm);
    }

    int ok = 1;
    if (!an_error_ends_the_block_it_was_raised_in_ended_in(&sm, AN_ERROR_ENDS_THE_BLOCK_IT_WAS_RAISED_IN_STATE_DONE)) {
        fprintf(stderr, "FAIL: `finish` must carry the run to `done`\n");
        ok = 0;
    }

    const struct {
        const char *name;
        reader_t read;
        int64_t want;
    } observed[] = {
        {"errors", an_error_ends_the_block_it_was_raised_in_errors, 7},
        {"afterAssign", an_error_ends_the_block_it_was_raised_in_after_assign, 0},
        {"afterScript", an_error_ends_the_block_it_was_raised_in_after_script, 0},
        {"afterLog", an_error_ends_the_block_it_was_raised_in_after_log, 0},
        {"afterCancel", an_error_ends_the_block_it_was_raised_in_after_cancel, 0},
        {"afterIfInner", an_error_ends_the_block_it_was_raised_in_after_if_inner, 0},
        {"afterIf", an_error_ends_the_block_it_was_raised_in_after_if, 0},
        {"afterSingle", an_error_ends_the_block_it_was_raised_in_after_single, 0},
        {"afterTrans", an_error_ends_the_block_it_was_raised_in_after_trans, 0},
        {"initRan", an_error_ends_the_block_it_was_raised_in_init_ran, 1},
        {"pairs", an_error_ends_the_block_it_was_raised_in_pairs, 4},
        {"sum", an_error_ends_the_block_it_was_raised_in_sum, 90},
    };

    for (size_t i = 0; i < sizeof(observed) / sizeof(observed[0]); ++i) {
        int64_t got = -1;
        if (!observed[i].read(&sm, &got) || got != observed[i].want) {
            fprintf(stderr, "FAIL: %s = %lld, want %lld\n", observed[i].name, (long long)got,
                    (long long)observed[i].want);
            ok = 0;
        }
    }
    an_error_ends_the_block_it_was_raised_in_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: each error ended only the block it was raised in\n");
    return 0;
}
