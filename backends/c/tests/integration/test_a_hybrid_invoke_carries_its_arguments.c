// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4.1 + 6.4.3: an <invoke> whose child is named by an expression
// carries its arguments as one whose child is fixed does — C11 AOT.
//
// The value always names `keeper`, and `bare` declares the one name `keeper`
// does not, so a pair seeded by the wrong candidate's declarations is a leak
// `keeper` reports rather than an absence the test has to infer.
//
// Fixture: integration_resources/a_hybrid_invoke_carries_its_arguments/a_hybrid_invoke_carries_its_arguments.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_hybrid_invoke_carries_its_arguments ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "a_hybrid_invoke_carries_its_arguments_sm.h"

typedef a_hybrid_invoke_carries_its_arguments_t sm_t;

typedef bool (*reader_t)(const sm_t *, int64_t *);

int main(void) {
    sm_t sm;
    a_hybrid_invoke_carries_its_arguments_init(&sm);
    a_hybrid_invoke_carries_its_arguments_run(&sm);

    int ok = 1;
    if (!a_hybrid_invoke_carries_its_arguments_ended_in(&sm, A_HYBRID_INVOKE_CARRIES_ITS_ARGUMENTS_STATE_DONE)) {
        fprintf(stderr, "FAIL: the run must end in `done` (`failWrongChild` means `bare` ran; "
                        "`failRefusedChildStarted` means an unreadable namelist still started a child)\n");
        ok = 0;
    }

    const struct {
        const char *name;
        reader_t read;
        int64_t want;
    } observed[] = {
        {"errors", a_hybrid_invoke_carries_its_arguments_errors, 2},
        {"started", a_hybrid_invoke_carries_its_arguments_started, 2},
        {"paramsOk", a_hybrid_invoke_carries_its_arguments_params_ok, 1},
        {"namelistOk", a_hybrid_invoke_carries_its_arguments_namelist_ok, 1},
    };

    for (size_t i = 0; i < sizeof(observed) / sizeof(observed[0]); ++i) {
        int64_t got = -1;
        if (!observed[i].read(&sm, &got) || got != observed[i].want) {
            fprintf(stderr, "FAIL: %s = %lld, want %lld\n", observed[i].name, (long long)got,
                    (long long)observed[i].want);
            ok = 0;
        }
    }
    a_hybrid_invoke_carries_its_arguments_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: each argument reached only the child that declares it\n");
    return 0;
}
