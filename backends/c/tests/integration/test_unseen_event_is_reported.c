// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 + Appendix D: an event handed to a machine that has already
// stopped is never looked at, and the host that sent it can find out - C11 AOT.
//
// Appendix D's main event loop exits when the machine reaches a top-level final
// state. Refusing what arrives afterwards is the clause; saying nothing about
// it is not. The silence is expensive because it looks like the two outcomes a
// host can already read:
//
//   dequeued, no transition matched            _discarded_external_events
//   dequeued, matched, guard said no           nothing, correctly
//   never dequeued - the machine had stopped   this
//
// This backend has a second reason to answer it: its queues are fixed-size
// arrays, so a host that keeps feeding a halted machine would otherwise
// silently overrun one.
//
// Fixture: integration_resources/unseen_event_is_reported/unseen_event_is_reported.scxml
// (canonical, shared with the C++ / Rust / Go / Kotlin / Python channels).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(unseen_event_is_reported ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>

#include "unseen_event_is_reported_sm.h"

static void deliver(unseen_event_is_reported_t *sm, unseen_event_is_reported_event_t event) {
    unseen_event_is_reported_event_with_meta_t carrier = {0};
    carrier.event = event;
    unseen_event_is_reported_raise_external(sm, &carrier);
    unseen_event_is_reported_step(sm);
}

int main(void) {
    int rc = 0;

    // The axis: an event the host queued after the machine stopped is counted.
    {
        unseen_event_is_reported_t sm;
        unseen_event_is_reported_init(&sm);

        if (unseen_event_is_reported_unseen_external_events(&sm) != 0u) {
            fprintf(stderr, "unseen_event_is_reported: FAIL - something was refused before "
                            "the first event was delivered.\n");
            rc = 1;
        }

        // The lower bound a check like this usually forgets: an accessor that
        // always answers reads exactly like a working one from the other side.
        // The sentinel is `_EVENT_FINISH` rather than the enum's zero value, so
        // a `memset`-clean struct cannot hand back something that looks
        // deliberate.
        unseen_event_is_reported_event_t untouched = UNSEEN_EVENT_IS_REPORTED_EVENT_FINISH;
        if (unseen_event_is_reported_last_unseen_event(&sm, &untouched)) {
            fprintf(stderr, "unseen_event_is_reported: FAIL - the machine named a refused "
                            "event before anything had been refused.\n");
            rc = 1;
        }
        if (untouched != UNSEEN_EVENT_IS_REPORTED_EVENT_FINISH) {
            fprintf(stderr, "unseen_event_is_reported: FAIL - the accessor wrote to its "
                            "out-parameter while reporting that it had nothing to say.\n");
            rc = 1;
        }

        deliver(&sm, UNSEEN_EVENT_IS_REPORTED_EVENT_POKE);
        int64_t pokes = -1;
        (void)unseen_event_is_reported_pokes(&sm, &pokes);
        if (pokes != 1) {
            fprintf(stderr,
                    "unseen_event_is_reported: FAIL - `poke`'s transition did not run "
                    "(pokes=%lld), so nothing here is measuring a machine that was working "
                    "first.\n",
                    (long long)pokes);
            rc = 1;
        }

        deliver(&sm, UNSEEN_EVENT_IS_REPORTED_EVENT_FINISH);
        if (!unseen_event_is_reported_is_in_final_state(&sm)) {
            fprintf(stderr, "unseen_event_is_reported: FAIL - `finish` did not take the "
                            "machine to its top-level final state.\n");
            rc = 1;
        }
        if (unseen_event_is_reported_unseen_external_events(&sm) != 0u) {
            fprintf(stderr, "unseen_event_is_reported: FAIL - `finish` was itself dequeued "
                            "and handled; the machine stopped BECAUSE of it, which is not "
                            "the same as stopping before it.\n");
            rc = 1;
        }

        deliver(&sm, UNSEEN_EVENT_IS_REPORTED_EVENT_POKE);

        if (unseen_event_is_reported_unseen_external_events(&sm) != 1u) {
            fprintf(stderr, "unseen_event_is_reported: FAIL - the host queued `poke` on a "
                            "machine that had reached its final state. W3C SCXML Appendix D's "
                            "loop had already ended, so the event was never dequeued; before "
                            "this count the host had no way to learn that.\n");
            rc = 1;
        }
        int64_t pokes_after = -1;
        (void)unseen_event_is_reported_pokes(&sm, &pokes_after);
        if (pokes_after != 1) {
            fprintf(stderr,
                    "unseen_event_is_reported: FAIL - the refused delivery ran the document's "
                    "transition anyway (pokes=%lld).\n",
                    (long long)pokes_after);
            rc = 1;
        }

        unseen_event_is_reported_event_t named = UNSEEN_EVENT_IS_REPORTED_EVENT_FINISH;
        if (!unseen_event_is_reported_last_unseen_event(&sm, &named) || named != UNSEEN_EVENT_IS_REPORTED_EVENT_POKE) {
            fprintf(stderr, "unseen_event_is_reported: FAIL - the machine counted a refusal "
                            "but cannot say which event it refused.\n");
            rc = 1;
        }

        unseen_event_is_reported_destroy(&sm);
    }

    // Why the query has to exist: every other accessor answers the same before
    // and after the refused delivery.
    {
        unseen_event_is_reported_t sm;
        unseen_event_is_reported_init(&sm);
        deliver(&sm, UNSEEN_EVENT_IS_REPORTED_EVENT_FINISH);

        const bool before_final = unseen_event_is_reported_is_in_final_state(&sm);
        const uint32_t before_discarded = unseen_event_is_reported_discarded_external_events(&sm);
        int64_t before_pokes = -1;
        (void)unseen_event_is_reported_pokes(&sm, &before_pokes);

        deliver(&sm, UNSEEN_EVENT_IS_REPORTED_EVENT_POKE);

        int64_t after_pokes = -2;
        (void)unseen_event_is_reported_pokes(&sm, &after_pokes);
        if (unseen_event_is_reported_is_in_final_state(&sm) != before_final ||
            unseen_event_is_reported_discarded_external_events(&sm) != before_discarded ||
            after_pokes != before_pokes) {
            fprintf(stderr, "unseen_event_is_reported: FAIL - this fixture exists because a "
                            "refused delivery is indistinguishable through the accessors a "
                            "host had; they differ, so the fixture stopped measuring what it "
                            "claims.\n");
            rc = 1;
        }
        if (unseen_event_is_reported_unseen_external_events(&sm) != 1u) {
            fprintf(stderr, "unseen_event_is_reported: FAIL - the two readings agree on "
                            "everything else, so this count is the only thing that separates "
                            "`the machine never looked` from `it looked and nothing "
                            "matched`.\n");
            rc = 1;
        }

        unseen_event_is_reported_destroy(&sm);
    }

    // A discard and a refusal are different facts, each with its own count, and
    // the count accumulates rather than latching.
    {
        unseen_event_is_reported_t sm;
        unseen_event_is_reported_init(&sm);

        deliver(&sm, UNSEEN_EVENT_IS_REPORTED_EVENT_POKE);
        if (unseen_event_is_reported_discarded_external_events(&sm) != 0u ||
            unseen_event_is_reported_unseen_external_events(&sm) != 0u) {
            fprintf(stderr, "unseen_event_is_reported: FAIL - `poke` matched a targetless "
                            "transition on a running machine; neither count should move.\n");
            rc = 1;
        }

        deliver(&sm, UNSEEN_EVENT_IS_REPORTED_EVENT_FINISH);
        deliver(&sm, UNSEEN_EVENT_IS_REPORTED_EVENT_POKE);
        if (unseen_event_is_reported_discarded_external_events(&sm) != 0u) {
            fprintf(stderr, "unseen_event_is_reported: FAIL - a refusal was reported as a "
                            "discard. The first says the machine looked and nothing matched, "
                            "the second says it never looked.\n");
            rc = 1;
        }

        deliver(&sm, UNSEEN_EVENT_IS_REPORTED_EVENT_FINISH);
        if (unseen_event_is_reported_unseen_external_events(&sm) != 2u) {
            fprintf(stderr, "unseen_event_is_reported: FAIL - the count is a count, not a "
                            "flag.\n");
            rc = 1;
        }
        unseen_event_is_reported_event_t named = UNSEEN_EVENT_IS_REPORTED_EVENT_POKE;
        if (!unseen_event_is_reported_last_unseen_event(&sm, &named) ||
            named != UNSEEN_EVENT_IS_REPORTED_EVENT_FINISH) {
            fprintf(stderr, "unseen_event_is_reported: FAIL - the name did not follow the "
                            "second refusal.\n");
            rc = 1;
        }

        unseen_event_is_reported_destroy(&sm);
    }

    if (rc == 0) {
        printf("unseen_event_is_reported: PASS\n");
    }
    return rc;
}
