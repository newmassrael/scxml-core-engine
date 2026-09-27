// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 + 6.4: what each argument of an <invoke> costs when it
// cannot be read, and what a readable one delivers — C11 AOT.
//
// Fixture:
// integration_resources/a_bad_invoke_argument_is_reported_once/a_bad_invoke_argument_is_reported_once.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_bad_invoke_argument_is_reported_once ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "a_bad_invoke_argument_is_reported_once_sm.h"

typedef a_bad_invoke_argument_is_reported_once_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

int main(void) {
    sm_t sm;
    a_bad_invoke_argument_is_reported_once_init(&sm);
    a_bad_invoke_argument_is_reported_once_run(&sm);

    const a_bad_invoke_argument_is_reported_once_event_t events[] = {
        A_BAD_INVOKE_ARGUMENT_IS_REPORTED_ONCE_EVENT_GO,
        A_BAD_INVOKE_ARGUMENT_IS_REPORTED_ONCE_EVENT_FINISH,
    };
    for (size_t i = 0; i < sizeof(events) / sizeof(events[0]); ++i) {
        a_bad_invoke_argument_is_reported_once_event_with_meta_t carrier = {0};
        carrier.event = events[i];
        a_bad_invoke_argument_is_reported_once_raise_external(&sm, &carrier);
        a_bad_invoke_argument_is_reported_once_run(&sm);
    }

    int ok = 1;
    if (!a_bad_invoke_argument_is_reported_once_ended_in(&sm, A_BAD_INVOKE_ARGUMENT_IS_REPORTED_ONCE_STATE_DONE)) {
        fprintf(stderr, "FAIL: `finish` must carry the run to `done`\n");
        ok = 0;
    }

    const struct {
        const char *name;
        reader_t read;
        int64_t want;
    } observed[] = {
        {"errors", a_bad_invoke_argument_is_reported_once_errors, 5},
        {"started", a_bad_invoke_argument_is_reported_once_started, 1},
        {"fromLocOk", a_bad_invoke_argument_is_reported_once_from_loc_ok, 1},
        {"emptyLocLeftOut", a_bad_invoke_argument_is_reported_once_empty_loc_left_out, 1},
        {"brokenLeftOut", a_bad_invoke_argument_is_reported_once_broken_left_out, 1},
    };

    for (size_t i = 0; i < sizeof(observed) / sizeof(observed[0]); ++i) {
        int64_t got = -1;
        if (!observed[i].read(&sm, &got) || got != observed[i].want) {
            fprintf(stderr, "FAIL: %s = %lld, want %lld\n", observed[i].name, (long long)got,
                    (long long)observed[i].want);
            ok = 0;
        }
    }
    a_bad_invoke_argument_is_reported_once_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: each invoke argument cost what its clause says\n");
    return 0;
}
