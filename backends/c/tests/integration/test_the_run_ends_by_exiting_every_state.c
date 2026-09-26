// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D exitInterpreter: a run ends by exiting every state it
// is still in, the way exitStates exits one — C11 AOT.
//
// Reached two ways, and both are driven here: the run enters a top-level
// `<final>` after a step, or the host stops it (`_stop`). Measured 2026-09-26,
// this channel ran a final's `<onexit>` only for an invoked child whose parent
// walked it (`_finalize_session`), left the final in the configuration, had no
// way for a host to stop a run, and `_run` cleared `is_running` whenever the
// machine went quiet.
//
// Fixture: integration_resources/the_run_ends_by_exiting_every_state/the_run_ends_by_exiting_every_state.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(the_run_ends_by_exiting_every_state ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "the_run_ends_by_exiting_every_state_sm.h"

typedef the_run_ends_by_exiting_every_state_t sm_t;

/// What the run left behind (W3C SCXML 5.3 accessors).
static void report(const sm_t *sm) {
    int64_t order = -1, final_exits = -1, self_in_final = -1;
    (void)the_run_ends_by_exiting_every_state_order(sm, &order);
    (void)the_run_ends_by_exiting_every_state_final_exits(sm, &final_exits);
    (void)the_run_ends_by_exiting_every_state_self_in_final(sm, &self_in_final);
    fprintf(stderr, "       active=0x%08x ended=%s order=%lld finalExits=%lld selfInFinal=%lld\n",
            (unsigned)the_run_ends_by_exiting_every_state_active_states(sm),
            the_run_ends_by_exiting_every_state_is_in_final_state(sm) ? "yes" : "no", (long long)order,
            (long long)final_exits, (long long)self_in_final);
}

static int started(sm_t *sm) {
    the_run_ends_by_exiting_every_state_init(sm);
    the_run_ends_by_exiting_every_state_run(sm);
    if (!the_run_ends_by_exiting_every_state_in_state(sm, THE_RUN_ENDS_BY_EXITING_EVERY_STATE_STATE_INNER)) {
        fprintf(stderr, "FAIL: the run has to start inside `inner`\n");
        return 0;
    }
    return 1;
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

/// The run ends in a top-level `<final>` after a step: the final is exited too.
static int a_run_that_reaches_its_final_exits_the_final(void) {
    sm_t sm;
    if (!started(&sm)) {
        return 0;
    }
    the_run_ends_by_exiting_every_state_event_with_meta_t carrier = {0};
    carrier.event = THE_RUN_ENDS_BY_EXITING_EVERY_STATE_EVENT_FINISH;
    the_run_ends_by_exiting_every_state_raise_external(&sm, &carrier);
    the_run_ends_by_exiting_every_state_run(&sm);

    int ok = 1;
    if (!the_run_ends_by_exiting_every_state_ended_in(&sm, THE_RUN_ENDS_BY_EXITING_EVERY_STATE_STATE_DONE)) {
        fprintf(stderr, "FAIL: the run did not end in `done`\n");
        report(&sm);
        ok = 0;
    }
    if (the_run_ends_by_exiting_every_state_active_states(&sm) != 0u) {
        fprintf(stderr, "FAIL: exitInterpreter deletes every state it exits, the final included\n");
        report(&sm);
        ok = 0;
    }
    ok &= expect_record(&sm, "finalExits", the_run_ends_by_exiting_every_state_final_exits, 1,
                        "the final's own <onexit> must run exactly once as the run ends");
    ok &= expect_record(&sm, "selfInFinal", the_run_ends_by_exiting_every_state_self_in_final, 1,
                        "`done` must still be in the configuration during its own <onexit>");
    ok &= expect_record(&sm, "order", the_run_ends_by_exiting_every_state_order, 12,
                        "`finish` exits `inner` then `outer`");
    the_run_ends_by_exiting_every_state_destroy(&sm);
    return ok;
}

/// The host stops a run that has not ended: every state is exited, innermost first.
static int a_stopped_run_exits_every_state_innermost_first(void) {
    sm_t sm;
    if (!started(&sm)) {
        return 0;
    }
    the_run_ends_by_exiting_every_state_stop(&sm);

    int ok = 1;
    if (the_run_ends_by_exiting_every_state_is_in_final_state(&sm)) {
        fprintf(stderr, "FAIL: a stopped run did not end in a final\n");
        report(&sm);
        ok = 0;
    }
    if (the_run_ends_by_exiting_every_state_active_states(&sm) != 0u) {
        fprintf(stderr, "FAIL: _stop must leave the configuration empty\n");
        report(&sm);
        ok = 0;
    }
    ok &= expect_record(&sm, "order", the_run_ends_by_exiting_every_state_order, 12,
                        "_stop must run `inner`'s <onexit> and then `outer`'s: 0 is a stop that exited "
                        "nothing, 21 one that exited in document order instead of exit order");
    ok &= expect_record(&sm, "finalExits", the_run_ends_by_exiting_every_state_final_exits, 0,
                        "the run never entered `done`");
    the_run_ends_by_exiting_every_state_destroy(&sm);
    return ok;
}

int main(void) {
    int ok = a_run_that_reaches_its_final_exits_the_final();
    ok &= a_stopped_run_exits_every_state_innermost_first();
    if (!ok) {
        return 1;
    }
    printf("PASS: a run ends by exiting every state it is still in, at its final and on stop\n");
    return 0;
}
