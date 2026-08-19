// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2: the processor MUST signal its own failures by raising
// error.* events into the internal queue, and the same paragraph says they
// "are ignored if no transition is found that matches them". Being ignored is
// the clause. Being unable to say it happened is not. C11 AOT path.
//
// discarded_event_is_observable asked this for the EXTERNAL queue and stopped
// at its edge on the stated ground that an unmatched internal event is the
// document's own business with both ends inside the document. That is exactly
// right for an author's <raise> and exactly wrong for an error event, whose
// sender is the ENGINE. The host never wrote the document, cannot see the
// failure in the configuration, and is the only party able to act on it.
//
// Four outcomes the fixture separates, all four leaving the configuration on
// the same state:
//
//   poke              handled, no error            control: proves a run fired
//   whisper           author's <raise>, unmatched  NOT counted
//   boom in idle      error, unmatched             COUNTED - the silent failure
//   boom in guarded   error, HANDLED               not counted
//
// This backend answers the membership question differently on purpose: it has
// no event-name table to consult at run time, so the generated code compares
// against the error enum members this document declares. The other five ask the
// same question of a name they already carry.
//
// Fixture: integration_resources/unhandled_error_is_observable/unhandled_error_is_observable.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(unhandled_error_is_observable ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "unhandled_error_is_observable_sm.h"

static void deliver(unhandled_error_is_observable_t *sm, unhandled_error_is_observable_event_t event) {
    unhandled_error_is_observable_event_with_meta_t carrier = {0};
    carrier.event = event;
    unhandled_error_is_observable_raise_external(sm, &carrier);
    unhandled_error_is_observable_step(sm);
}

int main(void) {
    int rc = 0;

    // The axis: an error the engine raised that no active state answers.
    {
        unhandled_error_is_observable_t sm;
        unhandled_error_is_observable_init(&sm);

        if (unhandled_error_is_observable_unhandled_error_events(&sm) != 0u) {
            fprintf(stderr, "unhandled_error_is_observable: FAIL - an error was counted "
                            "before the first event was delivered.\n");
            rc = 1;
        }

        // The lower bound, and the half a check like this usually forgets: an
        // accessor that always answers reads exactly like a working one from
        // the other side. `_EVENT_GO` is the sentinel because the enum's ZERO
        // value is a real member, so a `memset`-clean struct would hand back
        // something that looks deliberate.
        unhandled_error_is_observable_event_t untouched = UNHANDLED_ERROR_IS_OBSERVABLE_EVENT_GO;
        if (unhandled_error_is_observable_last_unhandled_error(&sm, &untouched)) {
            fprintf(stderr, "unhandled_error_is_observable: FAIL - the machine named an "
                            "unhandled error before anything had gone unhandled.\n");
            rc = 1;
        }

        deliver(&sm, UNHANDLED_ERROR_IS_OBSERVABLE_EVENT_BOOM);

        int64_t booms = -1;
        (void)unhandled_error_is_observable_booms(&sm, &booms);
        if (booms != 1) {
            fprintf(stderr,
                    "unhandled_error_is_observable: FAIL - `boom`'s transition did not "
                    "run (booms=%lld), so nothing here is measuring an error raised "
                    "inside a transition that fired.\n",
                    (long long)booms);
            rc = 1;
        }

        if (unhandled_error_is_observable_unhandled_error_events(&sm) != 1u) {
            fprintf(stderr, "unhandled_error_is_observable: FAIL - `boom`'s second <assign> has "
                            "W3C 5.3's invalid empty location, so the engine raised "
                            "error.execution, and `idle` declares no transition for it. The host "
                            "driving this machine has no other way to learn its <assign> failed.\n");
            rc = 1;
        }

        unhandled_error_is_observable_event_t named = UNHANDLED_ERROR_IS_OBSERVABLE_EVENT_GO;
        if (!unhandled_error_is_observable_last_unhandled_error(&sm, &named) ||
            named != UNHANDLED_ERROR_IS_OBSERVABLE_EVENT_ERROR_EXECUTION) {
            fprintf(stderr, "unhandled_error_is_observable: FAIL - the engine counted an unhandled "
                            "error but cannot say which one. error.execution is the document's own "
                            "executable content failing; error.communication would be a <send> that "
                            "could not reach its target - two different repairs.\n");
            rc = 1;
        }

        if (!unhandled_error_is_observable_in_state(&sm, UNHANDLED_ERROR_IS_OBSERVABLE_STATE_IDLE)) {
            fprintf(stderr, "unhandled_error_is_observable: FAIL - the error moved the machine "
                            "on its own.\n");
            rc = 1;
        }
    }

    // The other half: an error the DOCUMENT answered must not be counted. An
    // always-true count is as useless as an always-false one.
    {
        unhandled_error_is_observable_t sm;
        unhandled_error_is_observable_init(&sm);

        deliver(&sm, UNHANDLED_ERROR_IS_OBSERVABLE_EVENT_GO);
        if (!unhandled_error_is_observable_in_state(&sm, UNHANDLED_ERROR_IS_OBSERVABLE_STATE_GUARDED)) {
            fprintf(stderr, "unhandled_error_is_observable: FAIL - `go` did not reach the state "
                            "that answers errors.\n");
            rc = 1;
        }

        deliver(&sm, UNHANDLED_ERROR_IS_OBSERVABLE_EVENT_BOOM);

        int64_t caught = -1;
        (void)unhandled_error_is_observable_caught(&sm, &caught);
        if (caught != 1) {
            fprintf(stderr,
                    "unhandled_error_is_observable: FAIL - `guarded`'s error.execution "
                    "transition did not run (caught=%lld), so this block is not "
                    "measuring a HANDLED error.\n",
                    (long long)caught);
            rc = 1;
        }

        if (unhandled_error_is_observable_unhandled_error_events(&sm) != 0u) {
            fprintf(stderr, "unhandled_error_is_observable: FAIL - the same <assign> failed in "
                            "`guarded`, where the document does declare a transition for "
                            "error.execution. The document dealt with it, and counting that would "
                            "report the author's own error handling as a silent failure.\n");
            rc = 1;
        }
    }

    // The boundary the count is drawn at: an author's own unmatched <raise> is
    // not an unhandled error. Both ends of that event are inside the document.
    {
        unhandled_error_is_observable_t sm;
        unhandled_error_is_observable_init(&sm);

        deliver(&sm, UNHANDLED_ERROR_IS_OBSERVABLE_EVENT_WHISPER);

        if (unhandled_error_is_observable_unhandled_error_events(&sm) != 0u) {
            fprintf(stderr, "unhandled_error_is_observable: FAIL - `whisper` raises `unheard` and "
                            "`retry.error.execution`, neither of which any state answers. Both are "
                            "discarded exactly as an unmatched error is, and neither is one. "
                            "`retry.error.execution` CONTAINS `error.` without starting with it, "
                            "and W3C 3.12.2 reserves the prefix, not the substring.\n");
            rc = 1;
        }

        int64_t heards = -1;
        (void)unhandled_error_is_observable_heards(&sm, &heards);
        if (heards != 1) {
            fprintf(stderr,
                    "unhandled_error_is_observable: FAIL - `whisper`'s third raise, `heard`, "
                    "does match, and the transition it matches did not run (heards=%lld). The "
                    "count above is a byproduct of the internal drain, never its job.\n",
                    (long long)heards);
            rc = 1;
        }

        if (unhandled_error_is_observable_discarded_external_events(&sm) != 0u) {
            fprintf(stderr, "unhandled_error_is_observable: FAIL - `whisper` itself was handled, "
                            "so the external-queue count must stay put; the internal events it "
                            "raised are not on that queue at all.\n");
            rc = 1;
        }
    }

    // Why the query has to exist: every pre-existing accessor answers the same
    // for a run that failed silently and one that did not fail at all.
    {
        unhandled_error_is_observable_t sm;
        unhandled_error_is_observable_init(&sm);

        deliver(&sm, UNHANDLED_ERROR_IS_OBSERVABLE_EVENT_POKE);
        const bool clean_in_idle =
            unhandled_error_is_observable_in_state(&sm, UNHANDLED_ERROR_IS_OBSERVABLE_STATE_IDLE);
        const bool clean_final = unhandled_error_is_observable_is_in_final_state(&sm);
        const uint32_t clean_discarded = unhandled_error_is_observable_discarded_external_events(&sm);

        deliver(&sm, UNHANDLED_ERROR_IS_OBSERVABLE_EVENT_BOOM);

        if (unhandled_error_is_observable_in_state(&sm, UNHANDLED_ERROR_IS_OBSERVABLE_STATE_IDLE) != clean_in_idle ||
            unhandled_error_is_observable_is_in_final_state(&sm) != clean_final ||
            unhandled_error_is_observable_discarded_external_events(&sm) != clean_discarded) {
            fprintf(stderr, "unhandled_error_is_observable: FAIL - this fixture exists because "
                            "these two runs are indistinguishable through every accessor a host "
                            "had, including the external-queue discard count, which never sees "
                            "the internal queue. If they differ, the fixture stopped measuring "
                            "what it claims.\n");
            rc = 1;
        }

        if (unhandled_error_is_observable_unhandled_error_events(&sm) != 1u) {
            fprintf(stderr, "unhandled_error_is_observable: FAIL - the two runs are "
                            "indistinguishable through every other accessor, so this count is the "
                            "only thing that separates a silent failure from a clean run.\n");
            rc = 1;
        }
    }

    // The supervisor's actual failure mode: every round fails the same way and
    // nothing in the configuration ever changes.
    {
        unhandled_error_is_observable_t sm;
        unhandled_error_is_observable_init(&sm);

        for (uint32_t round = 1u; round <= 3u; ++round) {
            deliver(&sm, UNHANDLED_ERROR_IS_OBSERVABLE_EVENT_BOOM);
            if (unhandled_error_is_observable_unhandled_error_events(&sm) != round) {
                fprintf(stderr,
                        "unhandled_error_is_observable: FAIL - round %u did not add to the "
                        "count; a supervisor polling this number is exactly who learns the "
                        "loop is not making progress.\n",
                        round);
                rc = 1;
            }
        }

        int64_t booms = -1;
        (void)unhandled_error_is_observable_booms(&sm, &booms);
        if (booms != 3) {
            fprintf(stderr,
                    "unhandled_error_is_observable: FAIL - not every round ran its "
                    "transition (booms=%lld).\n",
                    (long long)booms);
            rc = 1;
        }
    }

    if (rc == 0) {
        printf("unhandled_error_is_observable: PASS\n");
    }
    return rc;
}
