// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 4.9 + 4.6: an error raised inside a <foreach> body ends the
// block that contains the <foreach>, and it is the only error raised —
// C11 AOT.
//
// Fixture:
// integration_resources/an_error_inside_a_foreach_ends_its_block/an_error_inside_a_foreach_ends_its_block.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(an_error_inside_a_foreach_ends_its_block ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "an_error_inside_a_foreach_ends_its_block_sm.h"

typedef an_error_inside_a_foreach_ends_its_block_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

int main(void) {
    sm_t sm;
    an_error_inside_a_foreach_ends_its_block_init(&sm);
    an_error_inside_a_foreach_ends_its_block_run(&sm);

    const an_error_inside_a_foreach_ends_its_block_event_t events[] = {
        AN_ERROR_INSIDE_A_FOREACH_ENDS_ITS_BLOCK_EVENT_GO,
        AN_ERROR_INSIDE_A_FOREACH_ENDS_ITS_BLOCK_EVENT_T,
        AN_ERROR_INSIDE_A_FOREACH_ENDS_ITS_BLOCK_EVENT_FINISH,
    };
    for (size_t i = 0; i < sizeof(events) / sizeof(events[0]); ++i) {
        an_error_inside_a_foreach_ends_its_block_event_with_meta_t carrier = {0};
        carrier.event = events[i];
        an_error_inside_a_foreach_ends_its_block_raise_external(&sm, &carrier);
        an_error_inside_a_foreach_ends_its_block_run(&sm);
    }

    int ok = 1;
    if (!an_error_inside_a_foreach_ends_its_block_ended_in(&sm, AN_ERROR_INSIDE_A_FOREACH_ENDS_ITS_BLOCK_STATE_DONE)) {
        fprintf(stderr, "FAIL: `finish` must carry the run to `done`\n");
        ok = 0;
    }

    const struct {
        const char *name;
        reader_t read;
        int64_t want;
    } observed[] = {
        {"iters1", an_error_inside_a_foreach_ends_its_block_iters1, 1},
        {"after1", an_error_inside_a_foreach_ends_its_block_after1, 0},
        {"iters2", an_error_inside_a_foreach_ends_its_block_iters2, 1},
        {"after2", an_error_inside_a_foreach_ends_its_block_after2, 0},
        {"iters3", an_error_inside_a_foreach_ends_its_block_iters3, 1},
        {"after3", an_error_inside_a_foreach_ends_its_block_after3, 0},
        {"errors", an_error_inside_a_foreach_ends_its_block_errors, 3},
        {"sent", an_error_inside_a_foreach_ends_its_block_sent, 2},
    };

    for (size_t i = 0; i < sizeof(observed) / sizeof(observed[0]); ++i) {
        int64_t got = -1;
        if (!observed[i].read(&sm, &got) || got != observed[i].want) {
            fprintf(stderr, "FAIL: %s = %lld, want %lld\n", observed[i].name, (long long)got,
                    (long long)observed[i].want);
            ok = 0;
        }
    }
    an_error_inside_a_foreach_ends_its_block_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: each failed <foreach> ended its block with one error\n");
    return 0;
}
