// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 says a macrostep is a chain of microsteps ending in a
// configuration where nothing is enabled by NULL. Appendix D's Principles and
// Constraints then say the chain need not exist: "A microstep always
// terminates. A macrostep may not. ... This is currently allowed." C11 AOT
// path.
//
// So a cyclic eventless document is not malformed, and an engine that runs it
// to the letter never returns. This one did not run it to the letter and said
// nothing either way: measured 2026-08-20, the loop's bare `while (iter < 100)`
// cut the macrostep and left no log line, no counter and no return value to
// read it from. Bounded and silent is the same signal as unbounded to the host
// reading it - and this backend has the least room of the seven to survive the
// alternative, since the ring buffer that holds the queue is fixed.
//
// error_cascade_is_bounded owns the chain built from errors; this one owns the
// chain built from transitions that need no event at all. The fixture
// separates a chain that stops on its own - a HUNDRED microsteps, exactly the
// ceiling, which is where an off-by-one lands - from one that cannot stop.
//
// Fixture: integration_resources/eventless_macrostep_is_bounded/eventless_macrostep_is_bounded.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(eventless_macrostep_is_bounded ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "eventless_macrostep_is_bounded_sm.h"

// The ceiling the engine applies, spelled here rather than read back from it.
// A test that asked the engine for its own limit would agree with any limit,
// including one an edit moved by three orders of magnitude.
#define MAX_MICROSTEPS 100

// One lap of either chain is two microsteps (_a to _b, then back) and only the
// _a edge counts, so a chain run to the ceiling records half.
#define LAPS_AT_CEILING (MAX_MICROSTEPS / 2)

static void deliver(eventless_macrostep_is_bounded_t *sm, eventless_macrostep_is_bounded_event_t event) {
    eventless_macrostep_is_bounded_event_with_meta_t carrier = {0};
    carrier.event = event;
    eventless_macrostep_is_bounded_raise_external(sm, &carrier);
    eventless_macrostep_is_bounded_step(sm);
}

int main(void) {
    int rc = 0;

    // The axis: a macrostep whose eventless chain cannot end is stopped, and
    // the host is told. This block returning at all is half the assertion.
    {
        eventless_macrostep_is_bounded_t sm;
        eventless_macrostep_is_bounded_init(&sm);

        if (eventless_macrostep_is_bounded_truncated_macrosteps(&sm) != 0u) {
            fprintf(stderr, "eventless_macrostep_is_bounded: FAIL - something was refused before "
                            "the machine had done anything.\n");
            rc = 1;
        }

        // The lower bound, and the half a check like this usually forgets: an
        // accessor that always answers reads exactly like a working one from
        // the other side. The state enum's ZERO value is a real member, so a
        // memset-clean struct would hand back something that looks deliberate.
        eventless_macrostep_is_bounded_state_t untouched = EVENTLESS_MACROSTEP_IS_BOUNDED_STATE_IDLE;
        if (eventless_macrostep_is_bounded_last_truncated_macrostep_state(&sm, &untouched)) {
            fprintf(stderr, "eventless_macrostep_is_bounded: FAIL - the machine named a stopped "
                            "macrostep before anything had been stopped.\n");
            rc = 1;
        }

        deliver(&sm, EVENTLESS_MACROSTEP_IS_BOUNDED_EVENT_SPIN);

        int64_t spins = -1;
        (void)eventless_macrostep_is_bounded_spins(&sm, &spins);
        if (spins != LAPS_AT_CEILING) {
            fprintf(stderr,
                    "eventless_macrostep_is_bounded: FAIL - the chain must run exactly as far as "
                    "the engine allows (want %d, got %lld): fewer means the document was cut off "
                    "early, more means the ceiling moved.\n",
                    LAPS_AT_CEILING, (long long)spins);
            rc = 1;
        }

        if (eventless_macrostep_is_bounded_truncated_macrosteps(&sm) != 1u) {
            fprintf(stderr, "eventless_macrostep_is_bounded: FAIL - the hundred-and-first "
                            "microstep was enabled and was not taken. Without that count the host "
                            "sees a machine that is running, in a state the document names, "
                            "having returned at once, with no way to learn that the configuration "
                            "it is reading is not a stable one.\n");
            rc = 1;
        }

        eventless_macrostep_is_bounded_state_t stopped_in = EVENTLESS_MACROSTEP_IS_BOUNDED_STATE_IDLE;
        if (!eventless_macrostep_is_bounded_last_truncated_macrostep_state(&sm, &stopped_in) ||
            stopped_in != EVENTLESS_MACROSTEP_IS_BOUNDED_STATE_SPIN_A) {
            fprintf(stderr, "eventless_macrostep_is_bounded: FAIL - an eventless cycle is a closed "
                            "walk through the state graph, and the count alone does not say which "
                            "walk. This names a state on it, which is where an author looks.\n");
            rc = 1;
        }

        if (!sm.is_running) {
            fprintf(stderr, "eventless_macrostep_is_bounded: FAIL - the chain was cut, not the "
                            "machine.\n");
            rc = 1;
        }
    }

    // The other half, and the one that makes the count mean something: a chain
    // that ends on its own is not refused, however long it is. The fixture's
    // bounded chain is exactly MAX_MICROSTEPS microsteps for that reason - a
    // ceiling that counted loop turns rather than microsteps taken, or that
    // tested >= where it meant >, reports this ordinary document as a runaway.
    {
        eventless_macrostep_is_bounded_t sm;
        eventless_macrostep_is_bounded_init(&sm);

        deliver(&sm, EVENTLESS_MACROSTEP_IS_BOUNDED_EVENT_BOUNDED);

        int64_t laps = -1;
        (void)eventless_macrostep_is_bounded_laps(&sm, &laps);
        if (laps != LAPS_AT_CEILING) {
            fprintf(stderr,
                    "eventless_macrostep_is_bounded: FAIL - the guard `laps < 50` closes after "
                    "fifty laps, so the chain is a hundred microsteps long and then stops by "
                    "itself (want %d, got %lld).\n",
                    LAPS_AT_CEILING, (long long)laps);
            rc = 1;
        }

        if (eventless_macrostep_is_bounded_truncated_macrosteps(&sm) != 0u) {
            fprintf(stderr, "eventless_macrostep_is_bounded: FAIL - nothing was refused: the "
                            "macrostep reached the stable configuration the clause describes, "
                            "using every microstep it was allowed. A long chain is not a "
                            "runaway.\n");
            rc = 1;
        }

        if (!sm.is_running) {
            fprintf(stderr, "eventless_macrostep_is_bounded: FAIL - a document that settles on "
                            "its own must not be reported dead by an engine that just finished "
                            "running it correctly.\n");
            rc = 1;
        }

        if (!eventless_macrostep_is_bounded_in_state(&sm, EVENTLESS_MACROSTEP_IS_BOUNDED_STATE_BOUNDED_A)) {
            fprintf(stderr, "eventless_macrostep_is_bounded: FAIL - the chain rests where its "
                            "guard closed.\n");
            rc = 1;
        }
    }

    // A count, not a flag: a second unbounded macrostep is refused the same way
    // the first was.
    {
        eventless_macrostep_is_bounded_t sm;
        eventless_macrostep_is_bounded_init(&sm);

        deliver(&sm, EVENTLESS_MACROSTEP_IS_BOUNDED_EVENT_SPIN);
        // reset is the fixture's way back out of the cycle, and it moves the
        // machine on purpose: the two C++ engines complete a macrostep only
        // after a transition that does.
        deliver(&sm, EVENTLESS_MACROSTEP_IS_BOUNDED_EVENT_RESET);
        deliver(&sm, EVENTLESS_MACROSTEP_IS_BOUNDED_EVENT_SPIN);

        int64_t spins = -1;
        (void)eventless_macrostep_is_bounded_spins(&sm, &spins);
        if (spins != 2 * LAPS_AT_CEILING) {
            fprintf(stderr,
                    "eventless_macrostep_is_bounded: FAIL - the second macrostep must buy the "
                    "document a full budget again (want %d, got %lld). The ceiling bounds a "
                    "macrostep, it does not condemn a machine.\n",
                    2 * LAPS_AT_CEILING, (long long)spins);
            rc = 1;
        }

        if (eventless_macrostep_is_bounded_truncated_macrosteps(&sm) != 2u) {
            fprintf(stderr, "eventless_macrostep_is_bounded: FAIL - two macrosteps, two refusals. "
                            "A count that saturates at one would read as a machine that "
                            "recovered.\n");
            rc = 1;
        }
    }

    // The control: an ordinary document is untouched by any of this. Without
    // it, an engine that refused every macrostep would pass the blocks above
    // and fail nothing.
    {
        eventless_macrostep_is_bounded_t sm;
        eventless_macrostep_is_bounded_init(&sm);

        deliver(&sm, EVENTLESS_MACROSTEP_IS_BOUNDED_EVENT_POKE);

        int64_t pokes = -1;
        (void)eventless_macrostep_is_bounded_pokes(&sm, &pokes);
        if (pokes != 1) {
            fprintf(stderr, "eventless_macrostep_is_bounded: FAIL - the run did not fire (got %lld).\n",
                    (long long)pokes);
            rc = 1;
        }

        if (eventless_macrostep_is_bounded_truncated_macrosteps(&sm) != 0u) {
            fprintf(stderr, "eventless_macrostep_is_bounded: FAIL - a macrostep of one microstep "
                            "ends the way the clause says it does.\n");
            rc = 1;
        }
    }

    if (rc == 0) {
        printf("eventless_macrostep_is_bounded: PASS\n");
    }
    return rc;
}
