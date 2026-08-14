// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4: a region that took an external self-transition still holds an
// atomic state, so it answers the next event too — C11 AOT.
//
// Its sibling `parallel_regions_take_own_transitions` owns the microstep axis.
// This one owns what that axis cannot reach: a region can take its transition
// and run its content exactly as required and still be left holding no leaf,
// and the only thing that tells you so is a later event it fails to answer.
//
// Measured 2026-08-14 on the C++ AOT channel, where the defect lived: with the
// mutation `parallel_microstep_owns_exit_and_entry.cases` restoring it, the
// sibling fixture's driver stayed green and this fixture's went red.
//
// Fixture: integration_resources/parallel_self_transition_keeps_its_leaf/parallel_self_transition_keeps_its_leaf.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(parallel_self_transition_keeps_its_leaf ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "parallel_self_transition_keeps_its_leaf_sm.h"

int main(void) {
    parallel_self_transition_keeps_its_leaf_t sm;
    parallel_self_transition_keeps_its_leaf_init(&sm);
    parallel_self_transition_keeps_its_leaf_run(&sm);

    if (!parallel_self_transition_keeps_its_leaf_in_state(&sm, PARALLEL_SELF_TRANSITION_KEEPS_ITS_LEAF_STATE_WITHIN) ||
        !parallel_self_transition_keeps_its_leaf_in_state(&sm, PARALLEL_SELF_TRANSITION_KEEPS_ITS_LEAF_STATE_WORKING)) {
        fprintf(stderr, "FAIL: the fixture is supposed to start with the self-transitioning "
                        "region in `within` and the deeper one in `working`; it did not, so "
                        "nothing below is testing what it claims\n");
        return 1;
    }

    parallel_self_transition_keeps_its_leaf_event_with_meta_t first = {0};
    first.event = PARALLEL_SELF_TRANSITION_KEEPS_ITS_LEAF_EVENT_E;
    parallel_self_transition_keeps_its_leaf_raise_external(&sm, &first);
    parallel_self_transition_keeps_its_leaf_run(&sm);

    // The symptom, named where it happens. A region holding no atomic state is
    // still "in" the parallel by every ancestor test.
    if (!parallel_self_transition_keeps_its_leaf_in_state(&sm, PARALLEL_SELF_TRANSITION_KEEPS_ITS_LEAF_STATE_WITHIN)) {
        fprintf(stderr, "FAIL: the self-transitioning region lost its leaf on the first event. "
                        "`within` is both the source and the target of its own external "
                        "self-transition, so the microstep exits and re-enters it; anything "
                        "that exits it a second time takes it back out and does not put it "
                        "back.\n");
        return 1;
    }
    if (!parallel_self_transition_keeps_its_leaf_in_state(&sm, PARALLEL_SELF_TRANSITION_KEEPS_ITS_LEAF_STATE_JUDGING)) {
        fprintf(stderr, "FAIL: the deeper region did not take its own transition on `e`.\n");
        return 1;
    }

    // The second event is the one this fixture exists for: `judging` has no `e`
    // transition, so nothing but the self-transitioning region can answer, and
    // it can only answer from a leaf.
    parallel_self_transition_keeps_its_leaf_event_with_meta_t second = {0};
    second.event = PARALLEL_SELF_TRANSITION_KEEPS_ITS_LEAF_EVENT_E;
    parallel_self_transition_keeps_its_leaf_raise_external(&sm, &second);
    parallel_self_transition_keeps_its_leaf_run(&sm);

    if (!parallel_self_transition_keeps_its_leaf_in_state(&sm, PARALLEL_SELF_TRANSITION_KEEPS_ITS_LEAF_STATE_WITHIN)) {
        fprintf(stderr, "FAIL: the self-transitioning region is not in `within` after the "
                        "second `e`.\n");
        return 1;
    }

    parallel_self_transition_keeps_its_leaf_event_with_meta_t check = {0};
    check.event = PARALLEL_SELF_TRANSITION_KEEPS_ITS_LEAF_EVENT_CHECK;
    parallel_self_transition_keeps_its_leaf_raise_external(&sm, &check);
    parallel_self_transition_keeps_its_leaf_run(&sm);

    if (!parallel_self_transition_keeps_its_leaf_in_state(&sm, PARALLEL_SELF_TRANSITION_KEEPS_ITS_LEAF_STATE_SETTLED)) {
        fprintf(stderr, "FAIL: `check` did not carry the machine to `settled`, which the "
                        "document guards on `n == 1 && m == 2`. `m` reaches 2 only if the "
                        "self-transitioning region still had a leaf to transition from when "
                        "the second `e` arrived.\n");
        return 1;
    }

    printf("PASS: the self-transitioned region kept its leaf and answered the next event\n");
    return 0;
}
