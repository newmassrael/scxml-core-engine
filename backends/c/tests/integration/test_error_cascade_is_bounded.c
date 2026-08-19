// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2 says an error event nothing matches is ignored. It says
// nothing about an error event something DOES match, answered by a handler
// that fails the same way every time: the failure raises error.execution, the
// same transition answers it, and the drain never empties. C11 AOT path.
//
// That is not a hang, which is what makes it worth an accessor. Measured
// 2026-08-19 on the Python engine and a two-line document: 37,000 links a
// second, configuration unmoved, the machine still reporting itself running -
// the reading an unattended supervisor takes as a healthy idle machine while a
// core is pinned. This backend is the one with the least room to survive it:
// the ring buffer that holds the queue is fixed, so a chain also fills it.
//
// The fixture separates a chain that STOPS by itself (settle, three links,
// then its guard stops matching) from one that cannot (spin). Both are runs of
// errors, and only the second is a defect - a ceiling that could not tell them
// apart would report every document that fails often as broken.
//
// This backend answers the membership question differently on purpose: it has
// no event-name table to consult at run time, so the generated `event_is_error`
// compares against the error enum members this document declares. The other
// five ask the same question of a name they already carry.
//
// Fixture: integration_resources/error_cascade_is_bounded/error_cascade_is_bounded.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(error_cascade_is_bounded ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "error_cascade_is_bounded_sm.h"

// The ceiling the engine applies, spelled here rather than read back from it.
// A test that asked the engine for its own limit would agree with any limit,
// including one an edit moved by three orders of magnitude.
#define MAX_LINKS 100

static void deliver(error_cascade_is_bounded_t *sm, error_cascade_is_bounded_event_t event) {
    error_cascade_is_bounded_event_with_meta_t carrier = {0};
    carrier.event = event;
    error_cascade_is_bounded_raise_external(sm, &carrier);
    error_cascade_is_bounded_step(sm);
}

int main(void) {
    int rc = 0;

    // The axis: a handler that answers its own failure with the same failure
    // is stopped, and the host is told. This block returning at all is half
    // the assertion - before the ceiling existed, it did not.
    {
        error_cascade_is_bounded_t sm;
        error_cascade_is_bounded_init(&sm);

        if (error_cascade_is_bounded_error_cascade_events(&sm) != 0u) {
            fprintf(stderr, "error_cascade_is_bounded: FAIL - something was refused before the "
                            "machine had done anything.\n");
            rc = 1;
        }

        // The lower bound, and the half a check like this usually forgets: an
        // accessor that always answers reads exactly like a working one from
        // the other side. The enum's ZERO value is a real member, so a
        // memset-clean struct would hand back something that looks deliberate.
        error_cascade_is_bounded_event_t untouched = ERROR_CASCADE_IS_BOUNDED_EVENT_POKE;
        if (error_cascade_is_bounded_last_error_cascade_event(&sm, &untouched)) {
            fprintf(stderr, "error_cascade_is_bounded: FAIL - the machine named a refused error "
                            "before anything had been refused.\n");
            rc = 1;
        }

        deliver(&sm, ERROR_CASCADE_IS_BOUNDED_EVENT_SPIN);

        int64_t runs = -1;
        (void)error_cascade_is_bounded_runs(&sm, &runs);
        if (runs != MAX_LINKS) {
            fprintf(stderr,
                    "error_cascade_is_bounded: FAIL - `runaway`'s handler must run exactly as "
                    "many times as the engine allows links in a chain (want %d, got %lld): fewer "
                    "means the document was cut off early, more means the ceiling moved.\n",
                    MAX_LINKS, (long long)runs);
            rc = 1;
        }

        if (error_cascade_is_bounded_error_cascade_events(&sm) != 1u) {
            fprintf(stderr, "error_cascade_is_bounded: FAIL - the handler's <assign> failed again "
                            "on the last allowed link, and the error it raised is the one the "
                            "engine refused to queue. Without that count the host sees a machine "
                            "that is running, in a plausible state, with nothing to say about the "
                            "core it is burning.\n");
            rc = 1;
        }

        error_cascade_is_bounded_event_t refused = ERROR_CASCADE_IS_BOUNDED_EVENT_POKE;
        if (!error_cascade_is_bounded_last_error_cascade_event(&sm, &refused) ||
            refused != ERROR_CASCADE_IS_BOUNDED_EVENT_ERROR_EXECUTION) {
            fprintf(stderr, "error_cascade_is_bounded: FAIL - a count alone does not name the "
                            "repair: error.execution is a handler whose own content fails, "
                            "error.communication one that answers an unreachable target by "
                            "talking to it again.\n");
            rc = 1;
        }

        if (!sm.is_running) {
            fprintf(stderr, "error_cascade_is_bounded: FAIL - the chain was cut, not the machine.\n");
            rc = 1;
        }

        if (!error_cascade_is_bounded_in_state(&sm, ERROR_CASCADE_IS_BOUNDED_STATE_RUNAWAY)) {
            fprintf(stderr, "error_cascade_is_bounded: FAIL - the handler is targetless, so "
                            "nothing here may move the machine.\n");
            rc = 1;
        }
    }

    // The other half, and the one that makes the count mean something: a chain
    // that ends by itself must pass through untouched.
    {
        error_cascade_is_bounded_t sm;
        error_cascade_is_bounded_init(&sm);

        deliver(&sm, ERROR_CASCADE_IS_BOUNDED_EVENT_SETTLE);

        int64_t repairs = -1;
        (void)error_cascade_is_bounded_repairs(&sm, &repairs);
        if (repairs != 3) {
            fprintf(stderr,
                    "error_cascade_is_bounded: FAIL - `settling`'s handler repairs three times "
                    "and then its `repairs < 3` guard stops matching (got %lld). Three links is "
                    "what a real repair strategy looks like, and the engine must not have "
                    "interrupted it.\n",
                    (long long)repairs);
            rc = 1;
        }

        if (error_cascade_is_bounded_error_cascade_events(&sm) != 0u) {
            fprintf(stderr, "error_cascade_is_bounded: FAIL - nothing was refused: the chain ended "
                            "on the document's own terms. A ceiling that fired here would report "
                            "every document that fails often as one that cannot stop failing.\n");
            rc = 1;
        }

        if (error_cascade_is_bounded_unhandled_error_events(&sm) != 1u) {
            fprintf(stderr, "error_cascade_is_bounded: FAIL - the fourth error found no matching "
                            "transition once the guard closed, which is the ordinary clause. The "
                            "two counts answer different questions and this document produces "
                            "exactly one of each.\n");
            rc = 1;
        }
    }

    // A single failure with nobody to answer it is not a chain. The chain is
    // measured handler-to-handler, not failure-to-failure.
    {
        error_cascade_is_bounded_t sm;
        error_cascade_is_bounded_init(&sm);

        for (int i = 0; i < 5; ++i) {
            deliver(&sm, ERROR_CASCADE_IS_BOUNDED_EVENT_BOOM);
        }

        if (error_cascade_is_bounded_unhandled_error_events(&sm) != 5u) {
            fprintf(stderr, "error_cascade_is_bounded: FAIL - five failures, none of them "
                            "answered, is the clause's own case.\n");
            rc = 1;
        }

        if (error_cascade_is_bounded_error_cascade_events(&sm) != 0u) {
            fprintf(stderr, "error_cascade_is_bounded: FAIL - no handler ran, so no handler raised "
                            "anything: a count keyed off how OFTEN a document fails would already "
                            "be at five here.\n");
            rc = 1;
        }
    }

    // The machine is still a machine afterwards, and a second chain starts
    // from zero: the depth is a property of the chain, not of the machine's
    // whole life.
    {
        error_cascade_is_bounded_t sm;
        error_cascade_is_bounded_init(&sm);

        deliver(&sm, ERROR_CASCADE_IS_BOUNDED_EVENT_SPIN);
        deliver(&sm, ERROR_CASCADE_IS_BOUNDED_EVENT_POKE);

        int64_t pokes = -1;
        (void)error_cascade_is_bounded_pokes(&sm, &pokes);
        if (pokes != 1) {
            fprintf(stderr,
                    "error_cascade_is_bounded: FAIL - `runaway` answers `poke` with a targetless "
                    "transition and it did not run (pokes=%lld). An engine that ended the chain "
                    "by ending the machine would leave the host with a dead document instead of a "
                    "bounded one.\n",
                    (long long)pokes);
            rc = 1;
        }

        deliver(&sm, ERROR_CASCADE_IS_BOUNDED_EVENT_RESET);
        deliver(&sm, ERROR_CASCADE_IS_BOUNDED_EVENT_SPIN);

        int64_t runs = -1;
        (void)error_cascade_is_bounded_runs(&sm, &runs);
        if (runs != 2 * MAX_LINKS) {
            fprintf(stderr,
                    "error_cascade_is_bounded: FAIL - the second entry into `runaway` must buy the "
                    "document a full chain again (want %d, got %lld). A depth carried across the "
                    "drains would stop this one at its first link.\n",
                    2 * MAX_LINKS, (long long)runs);
            rc = 1;
        }

        if (error_cascade_is_bounded_error_cascade_events(&sm) != 2u) {
            fprintf(stderr, "error_cascade_is_bounded: FAIL - two chains, two refusals. A count "
                            "that saturates at one would read as a machine that recovered.\n");
            rc = 1;
        }
    }

    if (rc == 0) {
        printf("error_cascade_is_bounded: PASS\n");
    }
    return rc;
}
