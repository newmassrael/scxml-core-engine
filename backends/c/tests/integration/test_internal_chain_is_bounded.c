// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 ends a macrostep at a configuration where nothing is enabled
// by NULL AND the internal queue is empty. Appendix D's Principles and
// Constraints then say that end need not exist: "A microstep always
// terminates. A macrostep may not. ... This is currently allowed." C11 AOT
// path.
//
// eventless_macrostep_is_bounded owns the half of that clause built from
// transitions that need no event. This one owns the other half: a <raise>
// answered by a transition that raises again. Measured 2026-08-20 before the
// ceiling reached this branch, process_internal_queue drained
// `while (event_queue_pop(...))` with no budget at all, so the fixture's spin
// document never came back - and the ring buffer that holds the queue never
// filled, because the chain only ever holds one event at a time.
//
// This backend also has the least room of the seven to represent the answer:
// its configuration is a bitmap with no current-state scalar, so the state it
// names is the source of the transition the drain last took. The fixture's
// chains never leave one state, which is what lets all seven channels assert
// the same name.
//
// Fixture: integration_resources/internal_chain_is_bounded/internal_chain_is_bounded.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(internal_chain_is_bounded ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "internal_chain_is_bounded_sm.h"

// The ceiling the engine applies, spelled here rather than read back from it.
// A test that asked the engine for its own limit would agree with any limit,
// including one an edit moved by three orders of magnitude.
#define MAX_MICROSTEPS 1000

// One lap of the alternating chain is two microsteps - one internal event, one
// eventless transition - and only the internal half is counted, so a chain run
// to the shared ceiling records half.
#define ALTERNATING_LAPS_AT_CEILING (MAX_MICROSTEPS / 2)

static void deliver(internal_chain_is_bounded_t *sm, internal_chain_is_bounded_event_t event) {
    internal_chain_is_bounded_event_with_meta_t carrier = {0};
    carrier.event = event;
    internal_chain_is_bounded_raise_external(sm, &carrier);
    internal_chain_is_bounded_step(sm);
}

int main(void) {
    int rc = 0;

    // The axis: a macrostep whose <raise> chain cannot end is stopped, and the
    // host is told. This block returning at all is half the assertion.
    {
        internal_chain_is_bounded_t sm;
        internal_chain_is_bounded_init(&sm);

        if (internal_chain_is_bounded_truncated_macrosteps(&sm) != 0u) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - something was refused before the "
                            "machine had done anything.\n");
            rc = 1;
        }

        // The lower bound, and the half a check like this usually forgets: an
        // accessor that always answers reads exactly like a working one from
        // the other side. The state enum's ZERO value is a real member, so a
        // memset-clean struct would hand back something that looks deliberate.
        internal_chain_is_bounded_state_t untouched = INTERNAL_CHAIN_IS_BOUNDED_STATE_IDLE;
        if (internal_chain_is_bounded_last_truncated_macrostep_state(&sm, &untouched)) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - the machine named a stopped "
                            "macrostep before anything had been stopped.\n");
            rc = 1;
        }

        deliver(&sm, INTERNAL_CHAIN_IS_BOUNDED_EVENT_SPIN);

        int64_t links = -1;
        (void)internal_chain_is_bounded_links(&sm, &links);
        if (links != MAX_MICROSTEPS) {
            fprintf(stderr,
                    "internal_chain_is_bounded: FAIL - the chain must run exactly as far as the "
                    "engine allows (want %d, got %lld): fewer means the document was cut off "
                    "early, more means the ceiling moved.\n",
                    MAX_MICROSTEPS, (long long)links);
            rc = 1;
        }

        if (internal_chain_is_bounded_truncated_macrosteps(&sm) != 1u) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - the microstep past the budget "
                            "was queued and was not taken. Without that count the host sees a "
                            "machine that is running, in a state the document names, having "
                            "returned at once, with no way to learn that the configuration it is "
                            "reading is not a stable one.\n");
            rc = 1;
        }

        internal_chain_is_bounded_state_t stopped_in = INTERNAL_CHAIN_IS_BOUNDED_STATE_IDLE;
        if (!internal_chain_is_bounded_last_truncated_macrostep_state(&sm, &stopped_in) ||
            stopped_in != INTERNAL_CHAIN_IS_BOUNDED_STATE_SPIN) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - the count alone says a document "
                            "somewhere cannot settle; this says where to look.\n");
            rc = 1;
        }

        if (!sm.is_running) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - the chain was cut, not the "
                            "machine.\n");
            rc = 1;
        }
    }

    // The other half, and the one that makes the count mean something: a chain
    // that ends on its own is not refused, however long it is. The fixture's
    // bounded chain is exactly MAX_MICROSTEPS links for that reason - a ceiling
    // that counted loop turns rather than microsteps taken, or that tested >=
    // where it meant >, reports this ordinary document as a runaway.
    {
        internal_chain_is_bounded_t sm;
        internal_chain_is_bounded_init(&sm);

        deliver(&sm, INTERNAL_CHAIN_IS_BOUNDED_EVENT_BOUNDED);

        int64_t laps = -1;
        (void)internal_chain_is_bounded_laps(&sm, &laps);
        if (laps != MAX_MICROSTEPS) {
            fprintf(stderr,
                    "internal_chain_is_bounded: FAIL - the guard `laps < 999` stops matching at "
                    "the thousandth link, which raises nothing, so the queue empties and the "
                    "chain stops by itself (want %d, got %lld).\n",
                    MAX_MICROSTEPS, (long long)laps);
            rc = 1;
        }

        if (internal_chain_is_bounded_truncated_macrosteps(&sm) != 0u) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - nothing was refused: the macrostep "
                            "reached the stable configuration the clause describes, using every "
                            "microstep it was allowed. A long chain is not a runaway.\n");
            rc = 1;
        }

        if (!sm.is_running) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - a document that settles on its own "
                            "must not be reported dead by an engine that just finished running it "
                            "correctly.\n");
            rc = 1;
        }

        if (!internal_chain_is_bounded_in_state(&sm, INTERNAL_CHAIN_IS_BOUNDED_STATE_BOUNDED)) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - the chain rests where it ended.\n");
            rc = 1;
        }
    }

    // A dequeue that selected nothing is not a microstep, so it spends no
    // budget. Appendix D takes a microstep for a transition that was SELECTED;
    // a dequeue that matched none is the loop turn the clause does not count.
    // The fixture's unanswered chain is `bounded` with one unmatched event
    // added per link, so the two differ in exactly that and must cost the same.
    //
    // Measured 2026-08-21: this claim had no witness in any channel. The
    // mutation that spends the budget on every dequeue SURVIVED all five
    // outcomes, because every other chain here answers every event it raises.
    {
        internal_chain_is_bounded_t sm;
        internal_chain_is_bounded_init(&sm);

        deliver(&sm, INTERNAL_CHAIN_IS_BOUNDED_EVENT_UNANSWERED);

        int64_t ignores = -1;
        (void)internal_chain_is_bounded_ignores(&sm, &ignores);
        if (ignores != MAX_MICROSTEPS) {
            fprintf(stderr,
                    "internal_chain_is_bounded: FAIL - the chain is the same length as `bounded`; "
                    "the unmatched events between its links are dequeues that selected nothing, "
                    "and those are not microsteps (want %d, got %lld).\n",
                    MAX_MICROSTEPS, (long long)ignores);
            rc = 1;
        }

        if (internal_chain_is_bounded_truncated_macrosteps(&sm) != 0u) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - a thousand microsteps and a "
                            "thousand discards is a thousand microsteps: an engine that counted "
                            "the discards refuses this document at link five hundred and reports "
                            "a runaway that is not one.\n");
            rc = 1;
        }

        if (!sm.is_running) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - the document settled on its own.\n");
            rc = 1;
        }

        if (!internal_chain_is_bounded_in_state(&sm, INTERNAL_CHAIN_IS_BOUNDED_STATE_IGNORING)) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - the chain rests where it ended.\n");
            rc = 1;
        }
    }

    // The case a per-branch budget lets through: a chain that alternates one
    // <raise> with one eventless transition. Neither branch of Appendix D's
    // inner loop reaches the ceiling on its own here, so an engine that gives
    // each branch a counter of its own runs this document forever with both
    // ceilings half spent.
    {
        internal_chain_is_bounded_t sm;
        internal_chain_is_bounded_init(&sm);

        deliver(&sm, INTERNAL_CHAIN_IS_BOUNDED_EVENT_ALTERNATE);

        int64_t alts = -1;
        (void)internal_chain_is_bounded_alts(&sm, &alts);
        if (alts != ALTERNATING_LAPS_AT_CEILING) {
            fprintf(stderr,
                    "internal_chain_is_bounded: FAIL - the two branches share one budget, so a "
                    "chain that alternates them gets five hundred laps out of a thousand "
                    "microsteps (want %d, got %lld). A thousand here would mean the internal "
                    "branch had a ceiling of its own.\n",
                    ALTERNATING_LAPS_AT_CEILING, (long long)alts);
            rc = 1;
        }

        if (internal_chain_is_bounded_truncated_macrosteps(&sm) != 1u) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - the refusal is reported once, "
                            "whichever branch was holding the budget when it ran out.\n");
            rc = 1;
        }

        internal_chain_is_bounded_state_t stopped_in = INTERNAL_CHAIN_IS_BOUNDED_STATE_IDLE;
        if (!internal_chain_is_bounded_last_truncated_macrostep_state(&sm, &stopped_in) ||
            stopped_in != INTERNAL_CHAIN_IS_BOUNDED_STATE_ALT) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - named the same way as any other "
                            "chain that could not settle.\n");
            rc = 1;
        }
    }

    // What the refusal did with the links it would not run: it left them
    // queued. The fixture's resume chain is half again as long as the ceiling,
    // so the first macrostep is refused with five hundred links still to go and
    // the second one finishes them. An engine that dropped the queue stops at a
    // thousand and never finishes; one that ran the chain anyway finishes it in
    // the first macrostep.
    //
    // The event driving the second macrostep is poke, and what it does is
    // deliberately not asserted: internal events outrank it here, while the C++
    // AOT engine's processEvent takes the host's event first. That divergence
    // is its own debt - the counters below are the same on both.
    {
        internal_chain_is_bounded_t sm;
        internal_chain_is_bounded_init(&sm);

        deliver(&sm, INTERNAL_CHAIN_IS_BOUNDED_EVENT_RESUME);

        int64_t beats = -1;
        (void)internal_chain_is_bounded_beats(&sm, &beats);
        if (beats != MAX_MICROSTEPS) {
            fprintf(stderr,
                    "internal_chain_is_bounded: FAIL - the first macrostep spends the whole "
                    "budget on the chain (want %d, got %lld).\n",
                    MAX_MICROSTEPS, (long long)beats);
            rc = 1;
        }

        deliver(&sm, INTERNAL_CHAIN_IS_BOUNDED_EVENT_POKE);

        (void)internal_chain_is_bounded_beats(&sm, &beats);
        if (beats != MAX_MICROSTEPS + MAX_MICROSTEPS / 2) {
            fprintf(stderr,
                    "internal_chain_is_bounded: FAIL - the second macrostep must pick the chain "
                    "up where the first was cut and run it to its end (want %d, got %lld): the "
                    "refused links were left on the queue, not dropped.\n",
                    MAX_MICROSTEPS + MAX_MICROSTEPS / 2, (long long)beats);
            rc = 1;
        }

        if (internal_chain_is_bounded_truncated_macrosteps(&sm) != 1u) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - nothing was refused the second "
                            "time: the chain ended on its own inside the budget, which is an "
                            "ordinary macrostep however long the document took to get there.\n");
            rc = 1;
        }
    }

    // The control: an ordinary document is untouched by any of this. Without
    // it, an engine that refused every macrostep would pass the blocks above
    // and fail nothing.
    {
        internal_chain_is_bounded_t sm;
        internal_chain_is_bounded_init(&sm);

        deliver(&sm, INTERNAL_CHAIN_IS_BOUNDED_EVENT_POKE);

        int64_t pokes = -1;
        (void)internal_chain_is_bounded_pokes(&sm, &pokes);
        if (pokes != 1) {
            fprintf(stderr,
                    "internal_chain_is_bounded: FAIL - the run happened: a counter of zero cannot "
                    "tell an engine that did nothing from one that was never asked (got %lld).\n",
                    (long long)pokes);
            rc = 1;
        }

        if (internal_chain_is_bounded_truncated_macrosteps(&sm) != 0u) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - one transition is not a chain that "
                            "cannot end.\n");
            rc = 1;
        }

        if (!internal_chain_is_bounded_in_state(&sm, INTERNAL_CHAIN_IS_BOUNDED_STATE_IDLE)) {
            fprintf(stderr, "internal_chain_is_bounded: FAIL - the control transition returns to "
                            "idle.\n");
            rc = 1;
        }
    }

    if (rc == 0) {
        printf("internal_chain_is_bounded: PASS\n");
    }
    return rc;
}
