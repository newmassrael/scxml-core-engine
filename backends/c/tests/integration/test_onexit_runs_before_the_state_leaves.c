// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D exitStates: a state leaves the configuration AFTER its
// own `<onexit>` has run — C11 AOT.
//
// The procedure is onexit, then cancelInvoke, then configuration.delete(s),
// for each state in exitOrder. So `In(s)` inside s's own `<onexit>` is true,
// the parent is still active inside the child's `<onexit>`, and the child is
// already gone inside the parent's. The configuration after the microstep is
// the same whatever order an engine used, so the handlers record what they saw
// and those records are the verdict.
//
// Fixture: integration_resources/onexit_runs_before_the_state_leaves/onexit_runs_before_the_state_leaves.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(onexit_runs_before_the_state_leaves ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "onexit_runs_before_the_state_leaves_sm.h"

/// What the handlers recorded (W3C SCXML 5.3 accessors).
static void report_records(const onexit_runs_before_the_state_leaves_t *sm) {
    int64_t self_inner = -1, parent_inner = -1, self_outer = -1, child_outer = -1, exits = -1;
    (void)onexit_runs_before_the_state_leaves_self_in_inner(sm, &self_inner);
    (void)onexit_runs_before_the_state_leaves_parent_in_inner(sm, &parent_inner);
    (void)onexit_runs_before_the_state_leaves_self_in_outer(sm, &self_outer);
    (void)onexit_runs_before_the_state_leaves_child_in_outer(sm, &child_outer);
    (void)onexit_runs_before_the_state_leaves_exits(sm, &exits);
    fprintf(stderr,
            "       records: selfInInner=%lld parentInInner=%lld selfInOuter=%lld childInOuter=%lld "
            "exits=%lld (wanted 1 / 1 / 1 / 0 / 2)\n",
            (long long)self_inner, (long long)parent_inner, (long long)self_outer, (long long)child_outer,
            (long long)exits);
}

int main(void) {
    onexit_runs_before_the_state_leaves_t sm;
    onexit_runs_before_the_state_leaves_init(&sm);
    onexit_runs_before_the_state_leaves_run(&sm);

    if (!onexit_runs_before_the_state_leaves_in_state(&sm, ONEXIT_RUNS_BEFORE_THE_STATE_LEAVES_STATE_INNER)) {
        fprintf(stderr, "FAIL: the run has to start inside `inner`\n");
        return 1;
    }

    onexit_runs_before_the_state_leaves_event_with_meta_t carrier = {0};
    carrier.event = ONEXIT_RUNS_BEFORE_THE_STATE_LEAVES_EVENT_LEAVE;
    onexit_runs_before_the_state_leaves_raise_external(&sm, &carrier);
    onexit_runs_before_the_state_leaves_run(&sm);

    if (!onexit_runs_before_the_state_leaves_ended_in(&sm, ONEXIT_RUNS_BEFORE_THE_STATE_LEAVES_STATE_SETTLED)) {
        // The document checks its clauses in document order and lands each in a
        // `<final>` of its own, so which one it stopped at names the defect.
        const char *stopped = "no final at all — `leave` was not answered";
        if (onexit_runs_before_the_state_leaves_ended_in(&sm, ONEXIT_RUNS_BEFORE_THE_STATE_LEAVES_STATE_FAILEXITS)) {
            stopped = "failExits — a handler did not run";
        } else if (onexit_runs_before_the_state_leaves_ended_in(
                       &sm, ONEXIT_RUNS_BEFORE_THE_STATE_LEAVES_STATE_FAILSELFININNER)) {
            stopped = "failSelfInInner — `inner` was out of the configuration during its own <onexit>";
        } else if (onexit_runs_before_the_state_leaves_ended_in(
                       &sm, ONEXIT_RUNS_BEFORE_THE_STATE_LEAVES_STATE_FAILPARENTININNER)) {
            stopped = "failParentInInner — `outer` left before `inner`'s <onexit> ran";
        } else if (onexit_runs_before_the_state_leaves_ended_in(
                       &sm, ONEXIT_RUNS_BEFORE_THE_STATE_LEAVES_STATE_FAILSELFINOUTER)) {
            stopped = "failSelfInOuter — `outer` was out of the configuration during its own <onexit>";
        } else if (onexit_runs_before_the_state_leaves_ended_in(
                       &sm, ONEXIT_RUNS_BEFORE_THE_STATE_LEAVES_STATE_FAILCHILDINOUTER)) {
            stopped = "failChildInOuter — `inner` was still active during `outer`'s <onexit>";
        }
        fprintf(stderr, "FAIL: `leave` did not carry the machine to `settled`. It stopped at %s\n", stopped);
        report_records(&sm);
        return 1;
    }

    printf("PASS: every state was still active while its own <onexit> ran\n");
    return 0;
}
