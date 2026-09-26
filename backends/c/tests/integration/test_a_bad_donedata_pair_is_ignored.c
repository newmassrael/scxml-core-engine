// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7: a <donedata> pair that cannot be evaluated raises
// error.execution and is ignored; the done events are raised all the same —
// C11 AOT.
//
// Measured 2026-09-27, this channel checked every pair at build time and, on
// finding an empty `location`, raised one error.execution for the whole
// <donedata>, evaluated no pair and raised neither done event.
//
// Fixture:
// integration_resources/a_bad_donedata_pair_is_ignored/a_bad_donedata_pair_is_ignored.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_bad_donedata_pair_is_ignored ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "a_bad_donedata_pair_is_ignored_sm.h"

typedef a_bad_donedata_pair_is_ignored_t sm_t;

static void report(const sm_t *sm) {
    int64_t errors = -1, shape = -1;
    (void)a_bad_donedata_pair_is_ignored_errors(sm, &errors);
    (void)a_bad_donedata_pair_is_ignored_shape(sm, &shape);
    fprintf(stderr, "       errors=%lld shape=%lld (wanted 2 / 1)\n", (long long)errors, (long long)shape);
}

static int expect_record(const sm_t *sm, const char *name, bool (*reader)(const sm_t *, int64_t *), int64_t want,
                         const char *why) {
    int64_t value = -1;
    if (!reader(sm, &value) || value != want) {
        fprintf(stderr, "FAIL: %s = %lld, want %lld: %s\n", name, (long long)value, (long long)want, why);
        report(sm);
        return 0;
    }
    return 1;
}

int main(void) {
    sm_t sm;
    a_bad_donedata_pair_is_ignored_init(&sm);
    // The run needs nothing from the host.
    a_bad_donedata_pair_is_ignored_run(&sm);

    int ok = 1;
    if (!a_bad_donedata_pair_is_ignored_ended_in(&sm, A_BAD_DONEDATA_PAIR_IS_IGNORED_STATE_DONE)) {
        fprintf(stderr, "FAIL: done.state.p must still arrive and carry the run to `done`\n");
        report(&sm);
        ok = 0;
    }
    ok &= expect_record(&sm, "errors", a_bad_donedata_pair_is_ignored_errors, 2,
                        "each ignored pair raises its own error.execution");
    ok &= expect_record(&sm, "shape", a_bad_donedata_pair_is_ignored_shape, 1,
                        "done.state.r1 must carry the surviving pair and neither ignored one");
    a_bad_donedata_pair_is_ignored_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: the bad donedata pairs were ignored and both done events arrived\n");
    return 0;
}
