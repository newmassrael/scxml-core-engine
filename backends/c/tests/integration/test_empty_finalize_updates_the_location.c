// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.5.2 — what an EMPTY `<finalize>` does, and what an absent one
// does not. C11 AOT path.
//
// With no executable content the Processor "MUST update the data model each
// time an event is received from the child process ... for each item in the
// 'namelist' attribute and each such <param> element ... as if by <assign>
// with any return value that has a name that matches", and then: "Note that
// the automatic update does not take place if the <finalize> element is
// absent as opposed to empty."
//
// The corpus holds two <finalize> documents (W3C 233/234) and zero empty
// ones. Measured 2026-08-22, no channel implemented the automatic update: the
// AOT model carried `finalize_content` as one string, so an empty element and
// a missing one were the same value. This backend already lowered the body it
// did run (`to_lua_script`), which Rust, Go and cpp-AOT did not.
//
// Fixture: integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(empty_finalize_updates_the_location ...)`
// in `backends/c/tests/CMakeLists.txt`.

// `nanosleep` is POSIX, and the target is built with C_EXTENSIONS OFF — so
// the feature-test macro has to precede every include or `<time.h>` hides it.
#define _POSIX_C_SOURCE 199309L

#include <stdio.h>
#include <time.h>

#include "empty_finalize_updates_the_location_sm.h"

// Each phase is settled by a 3 s delayed `<send>`, so the host drives a real
// clock: a child that never answers must reach its own verdict state rather
// than leave the machine waiting.
static bool run_to_final(empty_finalize_updates_the_location_t *sm, unsigned budget_ms) {
    const struct timespec nap = {0, 10 * 1000 * 1000};  // 10 ms
    unsigned waited = 0u;
    while (waited < budget_ms) {
        empty_finalize_updates_the_location_tick(sm);
        empty_finalize_updates_the_location_step(sm);
        if (empty_finalize_updates_the_location_is_in_final_state(sm)) {
            return true;
        }
        nanosleep(&nap, NULL);
        waited += 10u;
    }
    return false;
}

int main(void) {
    int rc = 0;

    empty_finalize_updates_the_location_t sm;
    empty_finalize_updates_the_location_init(&sm);

    if (!run_to_final(&sm, 20000u)) {
        printf("FAIL: empty_finalize_updates_the_location never reached a final "
               "state — neither child answered and neither delayed timeout fired\n");
        empty_finalize_updates_the_location_destroy(&sm);
        return 1;
    }

    if (empty_finalize_updates_the_location_in_state(&sm, EMPTY_FINALIZE_UPDATES_THE_LOCATION_STATE_PASS)) {
        printf("PASS: the empty <finalize/> updated the location and the absent one "
               "did not\n");
    } else if (empty_finalize_updates_the_location_in_state(&sm,
                                                            EMPTY_FINALIZE_UPDATES_THE_LOCATION_STATE_FAILNOTUPDATED)) {
        printf("FAIL: the empty <finalize/> left `tally` at its old value — W3C "
               "SCXML 6.5.2 makes an empty element mean the automatic update\n");
        rc = 1;
    } else if (empty_finalize_updates_the_location_in_state(
                   &sm, EMPTY_FINALIZE_UPDATES_THE_LOCATION_STATE_FAILUPDATEDWITHOUTFINALIZE)) {
        printf("FAIL: `guard` moved with no <finalize> element at all — the clause's "
               "note is a prohibition, not an omission\n");
        rc = 1;
    } else if (empty_finalize_updates_the_location_in_state(
                   &sm, EMPTY_FINALIZE_UPDATES_THE_LOCATION_STATE_FAILUNMATCHEDNAMEWROTE)) {
        printf("FAIL: an event carrying no matching name still wrote `keeper` — W3C "
               "SCXML 6.5.2 says \"with ANY return value that has a name that "
               "matches\", so the write has to be guarded\n");
        rc = 1;
    } else if (empty_finalize_updates_the_location_in_state(
                   &sm, EMPTY_FINALIZE_UPDATES_THE_LOCATION_STATE_FAILUNMATCHEDCHILDSILENT)) {
        printf("FAIL: the third child never answered, so the guarded-write half was "
               "never exercised\n");
        rc = 1;
    } else if (empty_finalize_updates_the_location_in_state(
                   &sm, EMPTY_FINALIZE_UPDATES_THE_LOCATION_STATE_FAILEMPTYCHILDSILENT)) {
        printf("FAIL: the first child never answered, so the empty-<finalize> half "
               "was never exercised\n");
        rc = 1;
    } else if (empty_finalize_updates_the_location_in_state(
                   &sm, EMPTY_FINALIZE_UPDATES_THE_LOCATION_STATE_FAILABSENTCHILDSILENT)) {
        printf("FAIL: the second child never answered, so the absent-<finalize> half "
               "was never exercised\n");
        rc = 1;
    } else {
        printf("FAIL: settled in a state that is not a verdict state (active bitmap "
               "0x%08x)\n",
               empty_finalize_updates_the_location_active_states(&sm));
        rc = 1;
    }

    empty_finalize_updates_the_location_destroy(&sm);
    return rc;
}
