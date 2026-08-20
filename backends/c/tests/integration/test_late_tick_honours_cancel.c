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

// A clock that jumps forward on every reading.
//
// This is what a descheduled host looks like from inside the machine: two
// readings taken for what the document calls one instant come back different. A
// real one does it unpredictably and only under load, which is why the defect it
// exposes reached a push before it reached a test; this one does it on every
// reading, so the cases below are a verdict about the engine rather than about
// the machine the suite runs on.
static uint64_t stepping_now_ms;
static uint64_t stepping_step_ms;
static int stepping_readings;

static uint64_t stepping_now(void *user_data) {
    (void)user_data;
    ++stepping_readings;
    stepping_now_ms += stepping_step_ms;
    return stepping_now_ms;
}

static void stepping_reset(uint64_t step_ms) {
    stepping_now_ms = 0u;
    stepping_step_ms = step_ms;
    stepping_readings = 0;
}

// One host-owned run, recorded configuration by configuration. `out` holds the
// entry bitmap plus one reading after each of six 100 ms advances. The bitmap
// rather than a single state, because this backend's configuration IS the
// bitmap and a machine that took a different path through it would otherwise
// compare equal.
static void manual_trace(uint32_t *out) {
    late_tick_honours_cancel_t sm;
    late_tick_honours_cancel_init_with_clock(&sm, sce_clock_manual(0u));
    out[0] = late_tick_honours_cancel_active_states(&sm);
    for (int i = 0; i < 6; ++i) {
        late_tick_honours_cancel_advance_time_ms(&sm, 100u);
        out[i + 1] = late_tick_honours_cancel_active_states(&sm);
    }
    late_tick_honours_cancel_destroy(&sm);
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

    // ═══════════════════════════════════════════════════════════════════════
    // §scxml-6.2.2 — the clock the deadlines are measured from
    //
    // Everything above drives the machine on the link-time monotonic clock,
    // which is what a production host does and what the push runs. It is also
    // why this document reached a push before it reached a test: the two
    // <send delay>s in `waiting` were armed against two separate readings, so
    // a host descheduled between them by more than the 100 ms separating their
    // delays got the later send's deadline first. The cases below take the
    // clock away from the machine the suite runs on and hand it to the test,
    // so the verdict is about the engine.
    // ═══════════════════════════════════════════════════════════════════════

    // The axis of this round, swept rather than pinned to one value. The
    // threshold is arithmetic — the stall has to reach the 100 ms separating
    // the two delays before the later deadline can overtake the earlier one —
    // and a case pinned at one stall would pass for a fix that moved the
    // threshold instead of removing it. Measured on the pre-latch engine: 1, 50
    // and 99 pass, and 100 is the first failure.
    {
        static const uint64_t stalls[] = {1u, 50u, 99u, 100u, 101u, 150u, 1000u};
        for (size_t i = 0; i < sizeof(stalls) / sizeof(stalls[0]); ++i) {
            late_tick_honours_cancel_t sm;
            stepping_reset(stalls[i]);
            late_tick_honours_cancel_init_with_clock(&sm, sce_clock_source(stepping_now, NULL));

            if (late_tick_honours_cancel_in_state(&sm, LATE_TICK_HONOURS_CANCEL_STATE_CANCELLOST)) {
                fprintf(stderr,
                        "late_tick_honours_cancel: FAIL - a host stalled %llu ms between "
                        "the two <send delay>s of one <onentry> reordered them: `settle` "
                        "(200 ms) came due before `poke` (100 ms) because each send took "
                        "its own reading. W3C SCXML 6.2.2 makes a delay the wait the "
                        "DOCUMENT asks for, and the time the host spent descheduled is "
                        "not part of it.\n",
                        (unsigned long long)stalls[i]);
                rc = 1;
            }

            // One tick is one reading, so time moves `stall` per tick and the
            // smallest stall in the sweep needs a few hundred of them to cross
            // the document's 200 ms of deadlines.
            for (int n = 0; n < 4096 && !late_tick_honours_cancel_is_in_final_state(&sm); ++n) {
                late_tick_honours_cancel_tick(&sm);
            }
            if (!late_tick_honours_cancel_in_state(&sm, LATE_TICK_HONOURS_CANCEL_STATE_PASS)) {
                fprintf(stderr,
                        "late_tick_honours_cancel: FAIL - with a %llu ms stall per clock "
                        "reading the machine did not reach `pass`; the document's "
                        "<cancel sendid=\"s1\"> must still drop `settle`.\n",
                        (unsigned long long)stalls[i]);
                rc = 1;
            }
            late_tick_honours_cancel_destroy(&sm);
        }
    }

    // A tick dispatches what was due when the host called it — not what its own
    // slowness made due while it ran. Counted rather than inferred: the stall
    // here (150 ms) is larger than every delay in the document, so a machine
    // re-reading per pass would run the whole document inside one tick.
    {
        late_tick_honours_cancel_t sm;
        stepping_reset(150u);
        late_tick_honours_cancel_init_with_clock(&sm, sce_clock_source(stepping_now, NULL));
        if (stepping_readings != 1) {
            fprintf(stderr,
                    "late_tick_honours_cancel: FAIL - _init is one turn and must take one "
                    "clock reading; it took %d.\n",
                    stepping_readings);
            rc = 1;
        }
        late_tick_honours_cancel_tick(&sm);
        if (stepping_readings != 2) {
            fprintf(stderr,
                    "late_tick_honours_cancel: FAIL - _tick is one turn and must take one "
                    "clock reading; the run has taken %d in total. A tick that re-reads "
                    "the clock while it works extends its own window and dispatches "
                    "entries the host has not yet reached.\n",
                    stepping_readings);
            rc = 1;
        }
        late_tick_honours_cancel_destroy(&sm);
    }

    // The host-owned clock: the same generated machine, driven by
    // _advance_time_ms, reaches its verdict on the test's schedule. This is the
    // contract the Python channel has had all along (advance_time / now_ms).
    {
        late_tick_honours_cancel_t sm;
        late_tick_honours_cancel_init_with_clock(&sm, sce_clock_manual(0u));

        if (!late_tick_honours_cancel_in_state(&sm, LATE_TICK_HONOURS_CANCEL_STATE_WAITING)) {
            fprintf(stderr, "late_tick_honours_cancel: FAIL - nothing is due at t=0, so "
                            "the machine waits on its two delayed sends.\n");
            rc = 1;
        }

        // Past both deadlines in one move — the late wake-up this file is about.
        late_tick_honours_cancel_advance_time_ms(&sm, 400u);
        if (late_tick_honours_cancel_in_state(&sm, LATE_TICK_HONOURS_CANCEL_STATE_CANCELLOST)) {
            fprintf(stderr, "late_tick_honours_cancel: FAIL - a single 400 ms advance "
                            "stepped over both deadlines; `poke` must still be dispatched "
                            "first so `active`'s <cancel sendid=\"s1\"> can drop "
                            "`settle`.\n");
            rc = 1;
        }

        late_tick_honours_cancel_advance_time_ms(&sm, 100u);
        if (!late_tick_honours_cancel_in_state(&sm, LATE_TICK_HONOURS_CANCEL_STATE_PASS)) {
            fprintf(stderr, "late_tick_honours_cancel: FAIL - `finish` is armed for 100 ms "
                            "after `active` is entered, so the machine should be done.\n");
            rc = 1;
        }
        if (late_tick_honours_cancel_now_ms(&sm) != 500u) {
            fprintf(stderr,
                    "late_tick_honours_cancel: FAIL - the host moved this clock 400 + 100 "
                    "ms and nothing else may move it; it reads %llu.\n",
                    (unsigned long long)late_tick_honours_cancel_now_ms(&sm));
            rc = 1;
        }
        late_tick_honours_cancel_destroy(&sm);
    }

    // Determinism is the point, so it is asserted as such: the same call
    // sequence twice, and the intermediate states compared rather than only the
    // verdict. The wall-clock cases above cannot make this assertion — they
    // would be re-measuring the load on the build machine, which is exactly the
    // dependency this seam removes.
    {
        uint32_t first[7];
        uint32_t second[7];
        manual_trace(first);
        manual_trace(second);
        for (int i = 0; i < 7; ++i) {
            if (first[i] != second[i]) {
                fprintf(stderr,
                        "late_tick_honours_cancel: FAIL - two identical sequences of "
                        "_advance_time_ms produced different traces at step %d (%u vs %u). "
                        "A host-owned clock that is not reproducible is not host-owned.\n",
                        i, (unsigned)first[i], (unsigned)second[i]);
                rc = 1;
            }
        }
        if (first[6] != (1u << (unsigned)LATE_TICK_HONOURS_CANCEL_STATE_PASS)) {
            fprintf(stderr,
                    "late_tick_honours_cancel: FAIL - the host-owned trace did not end in "
                    "`pass`; its last configuration is %u.\n",
                    (unsigned)first[6]);
            rc = 1;
        }
    }

    // _advance_time_ms on a clock the host does not own must not move it: the
    // caller believes it owns time and does not, so the events it is waiting
    // for would arrive on a schedule it did not choose.
    {
        late_tick_honours_cancel_t sm;
        stepping_reset(0u);
        late_tick_honours_cancel_init_with_clock(&sm, sce_clock_source(stepping_now, NULL));
        uint64_t before = late_tick_honours_cancel_now_ms(&sm);
        late_tick_honours_cancel_advance_time_ms(&sm, 5000u);
        if (late_tick_honours_cancel_now_ms(&sm) != before) {
            fprintf(stderr, "late_tick_honours_cancel: FAIL - _advance_time_ms moved a "
                            "clock the host does not own.\n");
            rc = 1;
        }
        late_tick_honours_cancel_destroy(&sm);
    }

    return rc;
}
