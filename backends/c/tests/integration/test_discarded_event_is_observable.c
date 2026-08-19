// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.1.2: "If no transition matches in any state, the event is
// discarded" — and the host that fed it in can find out. C11 AOT path.
//
// Three outcomes leave the configuration identical, so no accessor that existed
// before this fixture separates them:
//
//   poke    self transition       handled (exits and re-enters idle)
//   nudge   targetless internal   handled (actions only, no exit/entry)
//   settle  no matching           DISCARDED - the host's event went nowhere
//
// The C++ Interpreter answers all three (processEvent's TransitionResult and
// getStatistics().failedTransitions); the generated engines computed the same
// fact at the same point of Appendix D's mainEventLoop and dropped it. Here the
// fact was the `else` branch that `process_transition` never had.
//
// Fixture: integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(discarded_event_is_observable ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdint.h>
#include <stdio.h>

#include "discarded_event_is_observable_sm.h"

static void deliver(discarded_event_is_observable_t *sm, discarded_event_is_observable_event_t event) {
    discarded_event_is_observable_event_with_meta_t carrier = {0};
    carrier.event = event;
    discarded_event_is_observable_raise_external(sm, &carrier);
    discarded_event_is_observable_step(sm);
}

int main(void) {
    int rc = 0;

    // The axis: an event the machine knows but no active state answers.
    {
        discarded_event_is_observable_t sm;
        discarded_event_is_observable_init(&sm);

        if (discarded_event_is_observable_discarded_external_events(&sm) != 0u) {
            fprintf(stderr, "discarded_event_is_observable: FAIL - something was counted "
                            "before the first event was delivered.\n");
            rc = 1;
        }

        // The lower bound, and the half a check like this usually forgets: an
        // accessor that always answers reads exactly like a working one from
        // the other side. `_EVENT_GO` is the sentinel here because the enum's
        // ZERO value is a real member (`_EVENT_NONE`), so a `memset`-clean
        // struct would hand back something that looks deliberate.
        discarded_event_is_observable_event_t untouched = DISCARDED_EVENT_IS_OBSERVABLE_EVENT_GO;
        if (discarded_event_is_observable_last_discarded_event(&sm, &untouched)) {
            fprintf(stderr, "discarded_event_is_observable: FAIL - the machine named a "
                            "discarded event before anything had been discarded.\n");
            rc = 1;
        }
        if (untouched != DISCARDED_EVENT_IS_OBSERVABLE_EVENT_GO) {
            fprintf(stderr, "discarded_event_is_observable: FAIL - the accessor wrote to its "
                            "out-parameter while reporting that it had nothing to say.\n");
            rc = 1;
        }

        // `settle` is declared in `busy`, so it is in the machine's vocabulary
        // and the host can name it - it just matches nothing in `idle`.
        deliver(&sm, DISCARDED_EVENT_IS_OBSERVABLE_EVENT_SETTLE);

        if (discarded_event_is_observable_discarded_external_events(&sm) != 1u) {
            fprintf(stderr, "discarded_event_is_observable: FAIL - `settle` came off the "
                            "external queue in `idle`, where no transition matches it. "
                            "W3C SCXML 3.1.2 discards it; the host that queued it has no "
                            "other way to learn its event went nowhere.\n");
            rc = 1;
        }
        if (!discarded_event_is_observable_in_state(&sm, DISCARDED_EVENT_IS_OBSERVABLE_STATE_IDLE)) {
            fprintf(stderr, "discarded_event_is_observable: FAIL - a discarded event moved "
                            "the machine.\n");
            rc = 1;
        }

        discarded_event_is_observable_event_t named = DISCARDED_EVENT_IS_OBSERVABLE_EVENT_NONE;
        if (!discarded_event_is_observable_last_discarded_event(&sm, &named) ||
            named != DISCARDED_EVENT_IS_OBSERVABLE_EVENT_SETTLE) {
            fprintf(stderr, "discarded_event_is_observable: FAIL - the machine counted a "
                            "discard but cannot say which event it was.\n");
            rc = 1;
        }

        discarded_event_is_observable_destroy(&sm);
    }

    // The other half: a handled event must NOT be counted, including the one
    // that changes nothing. An always-true count is as useless as an
    // always-false one.
    {
        discarded_event_is_observable_t sm;
        discarded_event_is_observable_init(&sm);

        deliver(&sm, DISCARDED_EVENT_IS_OBSERVABLE_EVENT_POKE);
        int64_t pokes = -1;
        (void)discarded_event_is_observable_pokes(&sm, &pokes);
        if (pokes != 1) {
            fprintf(stderr,
                    "discarded_event_is_observable: FAIL - `poke`'s self transition "
                    "did not run (pokes=%lld), so nothing here is measuring a "
                    "handled event.\n",
                    (long long)pokes);
            rc = 1;
        }

        deliver(&sm, DISCARDED_EVENT_IS_OBSERVABLE_EVENT_NUDGE);
        int64_t nudges = -1;
        (void)discarded_event_is_observable_nudges(&sm, &nudges);
        if (nudges != 1) {
            fprintf(stderr,
                    "discarded_event_is_observable: FAIL - `nudge`'s targetless "
                    "transition did not run (nudges=%lld).\n",
                    (long long)nudges);
            rc = 1;
        }

        if (discarded_event_is_observable_discarded_external_events(&sm) != 0u) {
            fprintf(stderr, "discarded_event_is_observable: FAIL - a handled event was "
                            "counted as discarded. `poke` returns to its own source and "
                            "`nudge` exits nothing at all, so neither changes the "
                            "configuration - which is exactly why the count cannot be "
                            "keyed off whether it changed.\n");
            rc = 1;
        }

        discarded_event_is_observable_destroy(&sm);
    }

    // Why the query has to exist: the configuration answers the same for a
    // handled event and a discarded one.
    {
        discarded_event_is_observable_t sm;
        discarded_event_is_observable_init(&sm);

        deliver(&sm, DISCARDED_EVENT_IS_OBSERVABLE_EVENT_POKE);
        uint32_t handled_active = discarded_event_is_observable_active_states(&sm);

        deliver(&sm, DISCARDED_EVENT_IS_OBSERVABLE_EVENT_SETTLE);
        uint32_t discarded_active = discarded_event_is_observable_active_states(&sm);

        if (handled_active != discarded_active) {
            fprintf(stderr, "discarded_event_is_observable: FAIL - this fixture exists "
                            "because a handled event and a discarded one leave the same "
                            "configuration; if they differ, it stopped measuring what it "
                            "claims.\n");
            rc = 1;
        }
        if (discarded_event_is_observable_discarded_external_events(&sm) != 1u) {
            fprintf(stderr, "discarded_event_is_observable: FAIL - the two are "
                            "indistinguishable through every other accessor, so the count "
                            "is the only thing that separates them.\n");
            rc = 1;
        }

        discarded_event_is_observable_destroy(&sm);
    }

    // The supervisor's actual failure mode: the machine moved on and the events
    // the host keeps sending no longer match anything.
    {
        discarded_event_is_observable_t sm;
        discarded_event_is_observable_init(&sm);

        deliver(&sm, DISCARDED_EVENT_IS_OBSERVABLE_EVENT_GO);
        if (!discarded_event_is_observable_in_state(&sm, DISCARDED_EVENT_IS_OBSERVABLE_STATE_BUSY)) {
            fprintf(stderr, "discarded_event_is_observable: FAIL - `go` should have moved "
                            "the machine out of `idle`.\n");
            rc = 1;
        }

        deliver(&sm, DISCARDED_EVENT_IS_OBSERVABLE_EVENT_POKE);
        if (discarded_event_is_observable_discarded_external_events(&sm) != 1u) {
            fprintf(stderr, "discarded_event_is_observable: FAIL - the machine left `idle`, "
                            "so `poke` no longer matches; the host that kept sending it is "
                            "exactly who the count is for.\n");
            rc = 1;
        }

        discarded_event_is_observable_destroy(&sm);
    }

    if (rc == 0) {
        printf("discarded_event_is_observable: PASS\n");
    }
    return rc;
}
