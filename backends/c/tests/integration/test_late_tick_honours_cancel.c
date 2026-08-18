// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.3: a <cancel> still lands when the host ticked late —
// C11 AOT path.
//
// The scheduled queue is sorted by fire time and _tick drains it. Draining it
// to exhaustion before running a macrostep is the defect: a host that wakes
// after two fire times have passed holds both entries, and raising both onto
// the external queue makes the second undroppable before the first one's
// transitions have run. The <cancel> then executes against a queue the event
// has already left.
//
// The host below sleeps past BOTH fire times before its first tick, because
// that is the only condition under which the two dispatch orders differ. A
// host that wakes between them passes either way, which is why every existing
// suite was blind to this. Measured 2026-08-19: with the drain-then-macrostep
// order this document reaches cancelLost here as well as on rust, go and
// python.
//
// Fixture: integration_resources/late_tick_honours_cancel/late_tick_honours_cancel.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(late_tick_honours_cancel ...)`
// in `backends/c/tests/CMakeLists.txt`.

// `nanosleep` is POSIX, and this target compiles as `-std=c11 -pedantic`,
// which hides everything outside the C standard. Declared before any include
// so the feature test reaches the first system header.
#define _POSIX_C_SOURCE 200809L

#include <stdint.h>
#include <stdio.h>
#include <time.h>

#include "late_tick_honours_cancel_sm.h"

// Past both <send delay>s in `waiting` (100 ms and 200 ms), with margin for a
// loaded machine.
#define PAST_BOTH_DEADLINES_MS 400L

static void sleep_ms(long ms) {
    struct timespec ts;
    ts.tv_sec = ms / 1000L;
    ts.tv_nsec = (ms % 1000L) * 1000000L;
    nanosleep(&ts, NULL);
}

// Drive to completion with a tick loop, bounded so a machine that stalls fails
// rather than hanging the suite.
static void drive_to_final(late_tick_honours_cancel_t *sm, long budget_ms, long step_ms) {
    long waited = 0;
    while (!late_tick_honours_cancel_is_in_final_state(sm) && waited < budget_ms) {
        sleep_ms(step_ms);
        waited += step_ms;
        late_tick_honours_cancel_tick(sm);
    }
}

int main(void) {
    int rc = 0;

    // The axis: one tick, taken after both deadlines passed.
    {
        late_tick_honours_cancel_t sm;
        late_tick_honours_cancel_init(&sm);

        sleep_ms(PAST_BOTH_DEADLINES_MS);
        late_tick_honours_cancel_tick(&sm);

        if (late_tick_honours_cancel_in_state(&sm, LATE_TICK_HONOURS_CANCEL_STATE_CANCELLOST)) {
            fprintf(stderr, "late_tick_honours_cancel: FAIL - `settle` was delivered even "
                            "though `active`'s <cancel sendid=\"s1\"> ran first. Both "
                            "entries were past due when this tick started, so the "
                            "scheduler drain raised them together and the cancel found "
                            "nothing left to drop. W3C SCXML 6.3 cancels a send that has "
                            "not been dispatched: dispatch is one entry per macrostep, "
                            "not one queue-flush per tick.\n");
            rc = 1;
        } else {
            drive_to_final(&sm, 2000L, 20L);
            if (!late_tick_honours_cancel_in_state(&sm, LATE_TICK_HONOURS_CANCEL_STATE_PASS)) {
                fprintf(stderr, "late_tick_honours_cancel: FAIL - the machine did not "
                                "reach `pass` after the cancel.\n");
                rc = 1;
            }
        }
        late_tick_honours_cancel_destroy(&sm);
    }

    // A host that wakes between the two deadlines is the easy case, and it must
    // keep working — the fix is about the late wake-up.
    {
        late_tick_honours_cancel_t sm;
        late_tick_honours_cancel_init(&sm);
        drive_to_final(&sm, 2000L, 10L);
        if (!late_tick_honours_cancel_in_state(&sm, LATE_TICK_HONOURS_CANCEL_STATE_PASS)) {
            fprintf(stderr, "late_tick_honours_cancel: FAIL - a 10 ms tick loop, which "
                            "wakes between the 100 ms and 200 ms deadlines, must reach "
                            "`pass`.\n");
            rc = 1;
        }
        late_tick_honours_cancel_destroy(&sm);
    }

    // The deadline the host would have to guess is one the machine can state,
    // and driving by it lands every wake-up on a fire time.
    {
        late_tick_honours_cancel_t sm;
        late_tick_honours_cancel_init(&sm);

        int64_t due = late_tick_honours_cancel_time_until_next_scheduled_ms(&sm);
        if (due < 0 || due > 100) {
            fprintf(stderr,
                    "late_tick_honours_cancel: FAIL - the nearer of the two armed sends "
                    "is 100 ms out; the machine answered %lld ms, which would send a "
                    "host past the earlier deadline.\n",
                    (long long)due);
            rc = 1;
        }
        // The lower bound is the half that catches an answer of "due now", which
        // reads as a working query and costs the caller a spin that never
        // sleeps — on an MCU, a core that never idles.
        if (due == 0) {
            fprintf(stderr, "late_tick_honours_cancel: FAIL - the nearer send is 100 ms out "
                            "and nothing is due yet, but the machine answered 0. A host "
                            "sleeping on that answer does not sleep at all.\n");
            rc = 1;
        }

        long budget = 3000L;
        while (!late_tick_honours_cancel_is_in_final_state(&sm) && budget > 0) {
            int64_t wait = late_tick_honours_cancel_time_until_next_scheduled_ms(&sm);
            if (wait < 0) {
                wait = 5;
            }
            if (wait == 0) {
                wait = 1;
            }
            sleep_ms((long)wait);
            budget -= (long)wait;
            late_tick_honours_cancel_tick(&sm);
        }
        if (!late_tick_honours_cancel_in_state(&sm, LATE_TICK_HONOURS_CANCEL_STATE_PASS)) {
            fprintf(stderr, "late_tick_honours_cancel: FAIL - deadline-driven ticking did "
                            "not reach `pass`.\n");
            rc = 1;
        }
        if (late_tick_honours_cancel_time_until_next_scheduled_ms(&sm) != -1) {
            fprintf(stderr, "late_tick_honours_cancel: FAIL - nothing is scheduled once "
                            "the machine is finished, so no wake-up is owed.\n");
            rc = 1;
        }
        late_tick_honours_cancel_destroy(&sm);
    }

    return rc;
}
