// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: a `<parallel>` completing raises `done.state.<id>` —
// C11 AOT.
//
// A `<parallel>` owns no `<final>` of its own; its finals sit one level down,
// inside the regions. The rule that registers the completion event walked from
// a `<final>` to its direct parent and so never reached the parallel, while
// this backend's emitter raises it from the grandparent — which produced
// generated code naming an enumerator the model never declared. That defect is
// a *compile* failure, so building this driver at all is most of the gate;
// what follows is the behavioural half.
//
// Fixture: integration_resources/parallel_completion_raises_done_state/parallel_completion_raises_done_state.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(parallel_completion_raises_done_state ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "parallel_completion_raises_done_state_sm.h"

int main(void) {
    parallel_completion_raises_done_state_t sm;
    parallel_completion_raises_done_state_init(&sm);
    parallel_completion_raises_done_state_run(&sm);

    if (!parallel_completion_raises_done_state_in_state(&sm, PARALLEL_COMPLETION_RAISES_DONE_STATE_STATE_A1) ||
        !parallel_completion_raises_done_state_in_state(&sm, PARALLEL_COMPLETION_RAISES_DONE_STATE_STATE_B1)) {
        fprintf(stderr, "FAIL: the fixture is supposed to start with both regions inside the "
                        "<parallel>; it did not, so nothing below is testing what it claims\n");
        return 1;
    }

    // The by-name entry point is emitted only for machines that host an
    // invoke; this fixture has none, so the event goes on by enum.
    parallel_completion_raises_done_state_event_with_meta_t go = {0};
    go.event = PARALLEL_COMPLETION_RAISES_DONE_STATE_EVENT_GO;
    parallel_completion_raises_done_state_raise_external(&sm, &go);
    parallel_completion_raises_done_state_run(&sm);

    const int a_final =
        parallel_completion_raises_done_state_in_state(&sm, PARALLEL_COMPLETION_RAISES_DONE_STATE_STATE_A2);
    const int b_final =
        parallel_completion_raises_done_state_in_state(&sm, PARALLEL_COMPLETION_RAISES_DONE_STATE_STATE_B2);

    if (!a_final || !b_final) {
        fprintf(stderr,
                "FAIL: a region did not reach its <final> on `go` (a2=%d b2=%d). "
                "W3C SCXML 3.4 has every region take its own enabled transition in "
                "the same microstep.\n",
                a_final, b_final);
        return 1;
    }

    printf("PASS: both regions reached their <final>, completing the parallel\n");
    return 0;
}
