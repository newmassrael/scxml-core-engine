// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 5.7.1 under 6.4 — C11 AOT path.
//
// A `<param>` of an `<invoke>` whose expression will not evaluate is the one
// place two clauses meet. §scxml-6.4.2 terminates the element when "the
// evaluation of its arguments produces an error", and the sentence after it —
// "Otherwise the Processor MUST start a new logical instance" — makes the
// alternative explicit. §scxml-5.7.1 says a failing `<param>` costs
// `error.execution` on the internal queue and "MUST ignore the name and
// value", then delegates only the SUCCESSFUL name and value to the context:
// "See 5.5 <donedata>, 6.2 <send> and 6.4 <invoke> for details."
//
// 5.7.1 governs. This backend's arm popped the Lua error object and moved on,
// so the child came up with a `<data>` nothing explained and the document had
// no event to act on. W3C test343 settles the same clause from the
// `<donedata>` side; no IRP document asks it of `<invoke>`.
//
// Fixture: integration_resources/invoke_param_error_starts_the_child/invoke_param_error_starts_the_child.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(invoke_param_error_starts_the_child ...)`
// in `backends/c/tests/CMakeLists.txt`.

// `nanosleep` is POSIX, and the target is built with C_EXTENSIONS OFF — so
// the feature-test macro has to precede every include or `<time.h>` hides it.
#define _POSIX_C_SOURCE 199309L

#include <stdio.h>
#include <time.h>

#include "invoke_param_error_starts_the_child_sm.h"

// The fixture's `timeout` is a 3 s delayed `<send>`, so the host drives a real
// clock: a channel that terminated the element leaves the machine waiting on a
// session that was never created, and only the clock turns that into a verdict.
static bool run_to_final(invoke_param_error_starts_the_child_t *sm, unsigned budget_ms) {
    const struct timespec nap = {0, 10 * 1000 * 1000};  // 10 ms
    unsigned waited = 0u;
    while (waited < budget_ms) {
        invoke_param_error_starts_the_child_tick(sm);
        invoke_param_error_starts_the_child_step(sm);
        if (invoke_param_error_starts_the_child_is_in_final_state(sm)) {
            return true;
        }
        nanosleep(&nap, NULL);
        waited += 10u;
    }
    return false;
}

int main(void) {
    int rc = 0;

    invoke_param_error_starts_the_child_t sm;
    invoke_param_error_starts_the_child_init(&sm);

    if (!run_to_final(&sm, 10000u)) {
        printf("FAIL: invoke_param_error_starts_the_child never reached a final "
               "state — neither the child's `childUp` nor the delayed `timeout` "
               "that judges a never-started child arrived\n");
        invoke_param_error_starts_the_child_destroy(&sm);
        return 1;
    }

    if (invoke_param_error_starts_the_child_in_state(&sm, INVOKE_PARAM_ERROR_STARTS_THE_CHILD_STATE_PASS)) {
        printf("PASS: the failed param cost its own pair and nothing else\n");
    } else if (invoke_param_error_starts_the_child_in_state(
                   &sm, INVOKE_PARAM_ERROR_STARTS_THE_CHILD_STATE_FAILNOPARAMERROR)) {
        printf("FAIL: `childUp` arrived with no `error.execution` before it — W3C "
               "SCXML 5.7.1 puts that error on the internal queue while the "
               "<invoke> is being evaluated\n");
        rc = 1;
    } else if (invoke_param_error_starts_the_child_in_state(
                   &sm, INVOKE_PARAM_ERROR_STARTS_THE_CHILD_STATE_FAILINVOKENOTSTARTED)) {
        printf("FAIL: the child never started — this backend read W3C SCXML "
               "6.4.2's \"terminate the processing of the element\" over 5.7.1's "
               "per-item rule\n");
        rc = 1;
    } else if (invoke_param_error_starts_the_child_in_state(
                   &sm, INVOKE_PARAM_ERROR_STARTS_THE_CHILD_STATE_FAILGOODPARAMLOST)) {
        printf("FAIL: the child's `kept` did not arrive as 'here' — one sibling "
               "that failed does not cost the others (W3C SCXML 6.4.3)\n");
        rc = 1;
    } else if (invoke_param_error_starts_the_child_in_state(
                   &sm, INVOKE_PARAM_ERROR_STARTS_THE_CHILD_STATE_FAILBROKENPARAMSEEDED)) {
        printf("FAIL: the child found the empty string under `broken` — 5.7.1 says "
               "ignore the name AND the value\n");
        rc = 1;
    } else {
        printf("FAIL: settled in a state that is not a verdict state (active "
               "bitmap 0x%08x)\n",
               invoke_param_error_starts_the_child_active_states(&sm));
        rc = 1;
    }

    invoke_param_error_starts_the_child_destroy(&sm);
    return rc;
}
