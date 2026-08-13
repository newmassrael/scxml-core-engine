// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: every region of a `<parallel>` takes its own enabled
// transition in the same microstep — C11 AOT.
//
// The fixture is asymmetric on purpose. One region's transition on the event
// is an external self-transition, whose domain Appendix D resolves through
// `findLCCA` over the proper ancestors — candidates that never include the
// state itself. Answering with the state left the exit-set walk without a
// stopping point, so it ran to the document root, the exit set named the
// enclosing `<parallel>`, and conflict resolution preempted the deeper
// region's transition on that same event.
//
// The observable is the top-level `<final id="settled">`, which the document
// reaches only when both regions' assignments have run — a configuration check
// alone would still pass for a region that moved without executing its
// transition content.
//
// Fixture: integration_resources/parallel_regions_take_own_transitions/parallel_regions_take_own_transitions.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(parallel_regions_take_own_transitions ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "parallel_regions_take_own_transitions_sm.h"

int main(void) {
    parallel_regions_take_own_transitions_t sm;
    parallel_regions_take_own_transitions_init(&sm);
    parallel_regions_take_own_transitions_run(&sm);

    if (!parallel_regions_take_own_transitions_in_state(&sm, PARALLEL_REGIONS_TAKE_OWN_TRANSITIONS_STATE_WORKING) ||
        !parallel_regions_take_own_transitions_in_state(&sm, PARALLEL_REGIONS_TAKE_OWN_TRANSITIONS_STATE_WITHIN)) {
        fprintf(stderr, "FAIL: the fixture is supposed to start with the deeper region in "
                        "`working` and the shallower one in `within`; it did not, so nothing "
                        "below is testing what it claims\n");
        return 1;
    }

    parallel_regions_take_own_transitions_event_with_meta_t e = {0};
    e.event = PARALLEL_REGIONS_TAKE_OWN_TRANSITIONS_EVENT_E;
    parallel_regions_take_own_transitions_raise_external(&sm, &e);
    parallel_regions_take_own_transitions_run(&sm);

    if (!parallel_regions_take_own_transitions_in_state(&sm, PARALLEL_REGIONS_TAKE_OWN_TRANSITIONS_STATE_JUDGING)) {
        fprintf(stderr, "FAIL: the deeper region lost its leaf. W3C SCXML 3.4 has every region "
                        "take its own enabled transition on `e`; the sibling region's external "
                        "self-transition must not preempt this one.\n");
        return 1;
    }
    if (!parallel_regions_take_own_transitions_in_state(&sm, PARALLEL_REGIONS_TAKE_OWN_TRANSITIONS_STATE_WITHIN)) {
        fprintf(stderr, "FAIL: the shallower region left `within`, which is both the source and "
                        "the target of its own external self-transition.\n");
        return 1;
    }

    parallel_regions_take_own_transitions_event_with_meta_t check = {0};
    check.event = PARALLEL_REGIONS_TAKE_OWN_TRANSITIONS_EVENT_CHECK;
    parallel_regions_take_own_transitions_raise_external(&sm, &check);
    parallel_regions_take_own_transitions_run(&sm);

    if (!parallel_regions_take_own_transitions_in_state(&sm, PARALLEL_REGIONS_TAKE_OWN_TRANSITIONS_STATE_SETTLED)) {
        fprintf(stderr, "FAIL: `check` did not carry the machine to `settled`, which the document "
                        "guards on both regions' assignments having run. Reaching `judging` "
                        "without `n == 1 && m == 1` means a region changed state while its "
                        "transition content was skipped.\n");
        return 1;
    }

    printf("PASS: both regions took their own transition and the verdict was reached\n");
    return 0;
}
