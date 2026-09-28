// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.6.2 + 6.2: a <send>'s <content expr> is evaluated when the send
// is, and its value is the event's data — C11 AOT.
//
// Fixture: integration_resources/a_send_content_expr_is_the_payload/a_send_content_expr_is_the_payload.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_send_content_expr_is_the_payload ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "a_send_content_expr_is_the_payload_sm.h"

typedef a_send_content_expr_is_the_payload_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

int main(void) {
    sm_t sm;
    a_send_content_expr_is_the_payload_init(&sm);
    a_send_content_expr_is_the_payload_run(&sm);

    int ok = 1;
    if (!a_send_content_expr_is_the_payload_ended_in(&sm, A_SEND_CONTENT_EXPR_IS_THE_PAYLOAD_STATE_DONE)) {
        fprintf(stderr, "FAIL: the run must end in `done`\n");
        ok = 0;
    }

    const struct {
        const char *name;
        reader_t read;
        int64_t want;
    } observed[] = {
        {"numberOk", a_send_content_expr_is_the_payload_number_ok, 1},
        {"objectOk", a_send_content_expr_is_the_payload_object_ok, 1},
        {"textOk", a_send_content_expr_is_the_payload_text_ok, 1},
        {"errors", a_send_content_expr_is_the_payload_errors, 1},
        {"badArrived", a_send_content_expr_is_the_payload_bad_arrived, 1},
        {"badEmpty", a_send_content_expr_is_the_payload_bad_empty, 1},
        {"afterBad", a_send_content_expr_is_the_payload_after_bad, 0},
    };

    for (size_t i = 0; i < sizeof(observed) / sizeof(observed[0]); ++i) {
        int64_t got = -1;
        if (!observed[i].read(&sm, &got) || got != observed[i].want) {
            fprintf(stderr, "FAIL: %s = %lld, want %lld\n", observed[i].name, (long long)got,
                    (long long)observed[i].want);
            ok = 0;
        }
    }
    a_send_content_expr_is_the_payload_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: each <content expr> was the payload\n");
    return 0;
}
