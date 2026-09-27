// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 + 4.9: a <send> whose <param> cannot be read still sends
// its message without that pair, and the error ends its block; a valid
// location param is sent — C11 AOT.
//
// Measured 2026-09-27, this channel let the rest of the block run, its
// external arm stored '' under a failed name and raised nothing, and a
// location-only param was read as `()`.
//
// Fixture:
// integration_resources/a_bad_send_param_ends_its_block/a_bad_send_param_ends_its_block.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_bad_send_param_ends_its_block ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "a_bad_send_param_ends_its_block_sm.h"

typedef a_bad_send_param_ends_its_block_t sm_t;

static void report(const sm_t *sm) {
    int64_t errors = -1, partials = -1, bares = -1, after = -1, carried = -1;
    (void)a_bad_send_param_ends_its_block_errors(sm, &errors);
    (void)a_bad_send_param_ends_its_block_partials(sm, &partials);
    (void)a_bad_send_param_ends_its_block_bares(sm, &bares);
    (void)a_bad_send_param_ends_its_block_after(sm, &after);
    (void)a_bad_send_param_ends_its_block_carried(sm, &carried);
    fprintf(stderr, "       errors=%lld partials=%lld bares=%lld after=%lld carried=%lld (wanted 3 / 2 / 1 / 0 / 1)\n",
            (long long)errors, (long long)partials, (long long)bares, (long long)after, (long long)carried);
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
    a_bad_send_param_ends_its_block_init(&sm);
    a_bad_send_param_ends_its_block_run(&sm);

    a_bad_send_param_ends_its_block_event_with_meta_t carrier = {0};
    carrier.event = A_BAD_SEND_PARAM_ENDS_ITS_BLOCK_EVENT_FINISH;
    a_bad_send_param_ends_its_block_raise_external(&sm, &carrier);
    a_bad_send_param_ends_its_block_run(&sm);

    int ok = 1;
    if (!a_bad_send_param_ends_its_block_ended_in(&sm, A_BAD_SEND_PARAM_ENDS_ITS_BLOCK_STATE_DONE)) {
        fprintf(stderr, "FAIL: `finish` must carry the run to `done`\n");
        report(&sm);
        ok = 0;
    }
    ok &= expect_record(&sm, "errors", a_bad_send_param_ends_its_block_errors, 3,
                        "each unreadable <param> raises one error.execution");
    ok &= expect_record(&sm, "partials", a_bad_send_param_ends_its_block_partials, 2,
                        "both internal sends go, carrying the good pair without the bad one");
    ok &= expect_record(&sm, "bares", a_bad_send_param_ends_its_block_bares, 1,
                        "the external send goes with its empty pair left out");
    ok &= expect_record(&sm, "after", a_bad_send_param_ends_its_block_after, 0,
                        "the <param> error ends the block, so nothing after the <send> runs");
    ok &= expect_record(&sm, "carried", a_bad_send_param_ends_its_block_carried, 1,
                        "a valid location param is sent with its value");
    a_bad_send_param_ends_its_block_destroy(&sm);
    if (!ok) {
        return 1;
    }
    printf("PASS: a <param> that could not be read was dropped, its message sent, and its block ended\n");
    return 0;
}
