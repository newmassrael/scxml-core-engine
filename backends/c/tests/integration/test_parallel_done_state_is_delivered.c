// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.4 + 3.7: `done.state.<parallel>` is delivered, not merely
// declared — C11 AOT.
//
// The sibling fixture `parallel_completion_raises_done_state` proves this
// backend DECLARES the completion enumerator, and proves it by compiling. It
// cannot prove delivery: it carries no listener, deliberately, because a
// transition's `event` attribute is itself a registration site and a listener
// there would leave that fixture unable to fail for the defect it exists to
// catch.
//
// Declared is not delivered. An emitter that names the enumerator and never
// raises it — or raises it where nothing selects from — compiles clean and
// passes there. This document listens, and reaching `settled`, a TOP-LEVEL
// `<final>`, is possible by no other route.
//
// Fixture: integration_resources/parallel_done_state_is_delivered/parallel_done_state_is_delivered.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(parallel_done_state_is_delivered ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "parallel_done_state_is_delivered_sm.h"

int main(void) {
    parallel_done_state_is_delivered_t sm;
    parallel_done_state_is_delivered_init(&sm);
    parallel_done_state_is_delivered_run(&sm);

    if (!parallel_done_state_is_delivered_in_state(&sm, PARALLEL_DONE_STATE_IS_DELIVERED_STATE_A1) ||
        !parallel_done_state_is_delivered_in_state(&sm, PARALLEL_DONE_STATE_IS_DELIVERED_STATE_B1)) {
        fprintf(stderr, "FAIL: the fixture is supposed to start with both regions inside the "
                        "<parallel>; it did not, so nothing below is testing what it claims\n");
        return 1;
    }

    // The by-name entry point is emitted only for machines that host an
    // invoke; this fixture has none, so the event goes on by enum.
    parallel_done_state_is_delivered_event_with_meta_t go = {0};
    go.event = PARALLEL_DONE_STATE_IS_DELIVERED_EVENT_GO;
    parallel_done_state_is_delivered_raise_external(&sm, &go);
    parallel_done_state_is_delivered_run(&sm);

    // One check, with the configuration in its message, because the two ways
    // this can fail are not separately observable: completion is selected in
    // the SAME macrostep as the regions' finals, so once `run` returns the
    // parallel has been exited and a2/b2 are gone. Measured — checking them
    // as a precondition failed against a backend that had already done the
    // right thing.
    //
    // The remaining states tell the two apart: a1/b1 means `go` moved
    // nothing; a2/b2 means the parallel completed and the event went nowhere.
    if (!parallel_done_state_is_delivered_in_state(&sm, PARALLEL_DONE_STATE_IS_DELIVERED_STATE_SETTLED)) {
        fprintf(stderr,
                "FAIL: every region reaching its <final> completes the parallel, so "
                "done.state.run had to be raised AND selected — `settled` is reachable by "
                "nothing else. Still inside: a1=%d a2=%d b1=%d b2=%d\n",
                parallel_done_state_is_delivered_in_state(&sm, PARALLEL_DONE_STATE_IS_DELIVERED_STATE_A1),
                parallel_done_state_is_delivered_in_state(&sm, PARALLEL_DONE_STATE_IS_DELIVERED_STATE_A2),
                parallel_done_state_is_delivered_in_state(&sm, PARALLEL_DONE_STATE_IS_DELIVERED_STATE_B1),
                parallel_done_state_is_delivered_in_state(&sm, PARALLEL_DONE_STATE_IS_DELIVERED_STATE_B2));
        return 1;
    }

    printf("PASS: the parallel's completion event was raised and selected\n");
    return 0;
}
