// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D's main event loop returns to
// selectEventlessTransitions() after every microstep, and drains the internal
// queue in the same inner loop. It never asks whether the microstep it just
// took moved the machine - it cannot, because W3C SCXML 3.13 defines a
// transition with no target as one that exits and enters nothing and runs its
// content in place. C11 AOT path.
//
// Measured 2026-08-20, the two C++ engines end the macrostep at such a
// transition: whatever its content enabled is never walked, and the host is
// handed a configuration the clause says is not stable. This channel is the
// side of that comparison that was already right, and it is here so the
// contract is stated for every backend rather than only for the ones that
// broke it.
//
// eventless_macrostep_is_bounded owns how FAR a chain may run; this one owns
// whether the chain is entered at all.
//
// Fixture:
// integration_resources/targetless_transition_completes_macrostep/targetless_transition_completes_macrostep.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(targetless_transition_completes_macrostep ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "targetless_transition_completes_macrostep_sm.h"

static void deliver(targetless_transition_completes_macrostep_t *sm,
                    targetless_transition_completes_macrostep_event_t event) {
    targetless_transition_completes_macrostep_event_with_meta_t carrier = {0};
    carrier.event = event;
    targetless_transition_completes_macrostep_raise_external(sm, &carrier);
    targetless_transition_completes_macrostep_step(sm);
}

int main(void) {
    int rc = 0;

    // The axis: a transition that moves nothing still ends a microstep, so the
    // macrostep continues into whatever its content enabled.
    //
    // chained == 1 with polished == 0 is the signature of an engine that
    // resumes the chain only after a transition that MOVED the machine: it
    // takes the link that moves and stops before the link that does not.
    // chained == 0 is the signature of one that never entered the chain at all.
    {
        targetless_transition_completes_macrostep_t sm;
        targetless_transition_completes_macrostep_init(&sm);

        deliver(&sm, TARGETLESS_TRANSITION_COMPLETES_MACROSTEP_EVENT_ARM);

        int64_t armed = -1;
        (void)targetless_transition_completes_macrostep_armed(&sm, &armed);
        if (armed != 1) {
            fprintf(stderr,
                    "targetless_transition_completes_macrostep: FAIL - the targetless transition "
                    "did not run its content (armed=%lld); the rest of this block would be "
                    "measuring a lost event rather than a stopped macrostep.\n",
                    (long long)armed);
            rc = 1;
        }

        int64_t chained = -1;
        (void)targetless_transition_completes_macrostep_chained(&sm, &chained);
        if (chained != 1) {
            fprintf(stderr,
                    "targetless_transition_completes_macrostep: FAIL - the eventless transition "
                    "that content enabled must be taken in the SAME macrostep (want 1, got %lld), "
                    "which is the whole of what Appendix D's inner loop promises a host.\n",
                    (long long)chained);
            rc = 1;
        }

        int64_t polished = -1;
        (void)targetless_transition_completes_macrostep_polished(&sm, &polished);
        if (polished != 1) {
            fprintf(stderr,
                    "targetless_transition_completes_macrostep: FAIL - the chain's last link is "
                    "targetless itself (want 1, got %lld), and an engine that walks the chain only "
                    "while the machine keeps moving stops exactly here.\n",
                    (long long)polished);
            rc = 1;
        }

        if (!targetless_transition_completes_macrostep_in_state(
                &sm, TARGETLESS_TRANSITION_COMPLETES_MACROSTEP_STATE_SETTLED)) {
            fprintf(stderr, "targetless_transition_completes_macrostep: FAIL - the host must be "
                            "handed the stable configuration, not the one the machine was passing "
                            "through.\n");
            rc = 1;
        }
    }

    // The other side of the same inner loop: what a targetless transition
    // raises is answered before the host gets control back.
    {
        targetless_transition_completes_macrostep_t sm;
        targetless_transition_completes_macrostep_init(&sm);

        deliver(&sm, TARGETLESS_TRANSITION_COMPLETES_MACROSTEP_EVENT_PING);

        int64_t answered = -1;
        (void)targetless_transition_completes_macrostep_answered(&sm, &answered);
        if (answered != 1) {
            fprintf(stderr,
                    "targetless_transition_completes_macrostep: FAIL - the internal event the "
                    "targetless transition raised must be dequeued and matched inside this "
                    "macrostep (want 1, got %lld).\n",
                    (long long)answered);
            rc = 1;
        }

        if (!targetless_transition_completes_macrostep_in_state(&sm,
                                                                TARGETLESS_TRANSITION_COMPLETES_MACROSTEP_STATE_IDLE)) {
            fprintf(stderr, "targetless_transition_completes_macrostep: FAIL - neither transition "
                            "moves the machine, which is the point: the macrostep has to continue "
                            "anyway.\n");
            rc = 1;
        }
    }

    // The control, and the reason a zero above means anything: a targetless
    // transition that enables nothing leaves the machine exactly where it was,
    // and having run is still observable. Without this block, an engine that
    // dropped every targetless transition on the floor would fail the two
    // above with the same numbers as one that took them and stopped early.
    {
        targetless_transition_completes_macrostep_t sm;
        targetless_transition_completes_macrostep_init(&sm);

        deliver(&sm, TARGETLESS_TRANSITION_COMPLETES_MACROSTEP_EVENT_QUIET);

        int64_t quiet = -1;
        (void)targetless_transition_completes_macrostep_quiet(&sm, &quiet);
        if (quiet != 1) {
            fprintf(stderr,
                    "targetless_transition_completes_macrostep: FAIL - the transition did not fire "
                    "(quiet=%lld).\n",
                    (long long)quiet);
            rc = 1;
        }

        int64_t chained = -1;
        int64_t polished = -1;
        int64_t answered = -1;
        (void)targetless_transition_completes_macrostep_chained(&sm, &chained);
        (void)targetless_transition_completes_macrostep_polished(&sm, &polished);
        (void)targetless_transition_completes_macrostep_answered(&sm, &answered);
        if (chained != 0 || polished != 0 || answered != 0) {
            fprintf(stderr,
                    "targetless_transition_completes_macrostep: FAIL - nothing else may run: the "
                    "eventless transition's guard is still closed, so an engine that walked the "
                    "chain here would be firing a transition the document did not enable "
                    "(chained=%lld, polished=%lld, answered=%lld).\n",
                    (long long)chained, (long long)polished, (long long)answered);
            rc = 1;
        }

        if (!targetless_transition_completes_macrostep_in_state(&sm,
                                                                TARGETLESS_TRANSITION_COMPLETES_MACROSTEP_STATE_IDLE) ||
            !sm.is_running) {
            fprintf(stderr, "targetless_transition_completes_macrostep: FAIL - the machine must be "
                            "left running, exactly where it was.\n");
            rc = 1;
        }
    }

    // The other microstep that ends where it began: a transition whose target
    // is its own source. It is not targetless - W3C SCXML 3.13 gives it an
    // exit and an entry - but the loop that dropped the targetless one dropped
    // this too, in the same line of code and for the same reason.
    {
        targetless_transition_completes_macrostep_t sm;
        targetless_transition_completes_macrostep_init(&sm);

        deliver(&sm, TARGETLESS_TRANSITION_COMPLETES_MACROSTEP_EVENT_RECYCLE);

        int64_t entries = -1;
        (void)targetless_transition_completes_macrostep_entries(&sm, &entries);
        if (entries != 2) {
            fprintf(stderr,
                    "targetless_transition_completes_macrostep: FAIL - the state is entered once by "
                    "`recycle` and once more by the eventless self transition its entry enabled "
                    "(want 2, got %lld): a self transition exits and re-enters, so <onentry> runs "
                    "again.\n",
                    (long long)entries);
            rc = 1;
        }

        if (!targetless_transition_completes_macrostep_in_state(
                &sm, TARGETLESS_TRANSITION_COMPLETES_MACROSTEP_STATE_RECYCLED)) {
            fprintf(stderr, "targetless_transition_completes_macrostep: FAIL - the guard closes "
                            "behind it, so the machine must rest here rather than spin.\n");
            rc = 1;
        }
    }

    // A macrostep, not a one-shot: the second targetless transition is
    // followed the same way the first was. An engine that ran the inner loop
    // once per machine passes the blocks above and fails this one.
    {
        targetless_transition_completes_macrostep_t sm;
        targetless_transition_completes_macrostep_init(&sm);

        deliver(&sm, TARGETLESS_TRANSITION_COMPLETES_MACROSTEP_EVENT_QUIET);
        deliver(&sm, TARGETLESS_TRANSITION_COMPLETES_MACROSTEP_EVENT_PING);
        deliver(&sm, TARGETLESS_TRANSITION_COMPLETES_MACROSTEP_EVENT_PING);

        int64_t answered = -1;
        (void)targetless_transition_completes_macrostep_answered(&sm, &answered);
        if (answered != 2) {
            fprintf(stderr,
                    "targetless_transition_completes_macrostep: FAIL - the raise in the third "
                    "macrostep must be answered like the one in the second (want 2, got %lld); the "
                    "inner loop belongs to every macrostep, not to the first.\n",
                    (long long)answered);
            rc = 1;
        }

        int64_t quiet = -1;
        (void)targetless_transition_completes_macrostep_quiet(&sm, &quiet);
        if (quiet != 1) {
            fprintf(stderr,
                    "targetless_transition_completes_macrostep: FAIL - the control transition ran "
                    "once, not once per macrostep (quiet=%lld).\n",
                    (long long)quiet);
            rc = 1;
        }
    }

    if (rc == 0) {
        printf("targetless_transition_completes_macrostep: PASS\n");
    }
    return rc;
}
