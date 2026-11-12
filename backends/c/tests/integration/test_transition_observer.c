// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The transition observer (docs/adr/0089) — C11 AOT, on a manual clock.
//
// This target builds a_computed_event_name_is_matched_like_any_other with
// `SCE_TRANSITION_OBSERVER=1`, so the machine calls the hook below for each
// transition it takes, by the row of its emitted transition table. The
// document declares the table, and a run that reaches `pass` takes each of its
// six transitions once, in document order: the hook is told exactly those
// rows, each by the `(from, index)` its row carries, and from the first
// macrostep on, which `_init` itself runs.
//
// The same machine built without the switch is
// test_a_computed_event_name_is_matched_like_any_other.c, which reaches `pass`
// with no hook defined: the observer the build did not ask for is not there.
//
// Fixture:
// integration_resources/a_computed_event_name_is_matched_like_any_other/a_computed_event_name_is_matched_like_any_other.scxml

#include <stdint.h>
#include <stdio.h>

#include "../require_started.h"
#include "a_computed_event_name_is_matched_like_any_other_sm.h"

#if !A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_TRANSITION_OBSERVER
#error "this target is built with SCE_TRANSITION_OBSERVER=1 (backends/c/tests/CMakeLists.txt)"
#endif

#define SEEN_MAX 16u

static a_computed_event_name_is_matched_like_any_other_state_t seen_source[SEEN_MAX];
static uint32_t seen_index[SEEN_MAX];
static size_t seen_count;
static const a_computed_event_name_is_matched_like_any_other_t *seen_machine;
static int seen_other_machine;

void a_computed_event_name_is_matched_like_any_other_observe_transition(
    const a_computed_event_name_is_matched_like_any_other_t *sm,
    a_computed_event_name_is_matched_like_any_other_state_t source, uint32_t index) {
    if (sm != seen_machine) {
        seen_other_machine = 1;
    }
    if (seen_count < SEEN_MAX) {
        seen_source[seen_count] = source;
        seen_index[seen_count] = index;
    }
    ++seen_count;
}

int main(void) {
    a_computed_event_name_is_matched_like_any_other_t sm;
    seen_machine = &sm;
    REQUIRE_STARTED(a_computed_event_name_is_matched_like_any_other_init_with_clock(&sm, sce_clock_manual(0u)),
                    A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_START_OK);
    for (int elapsed = 0; elapsed < 1000 && !a_computed_event_name_is_matched_like_any_other_is_in_final_state(&sm);
         elapsed += 10) {
        a_computed_event_name_is_matched_like_any_other_advance_time_ms(&sm, 10u);
    }

    int bad = 0;
    if (!a_computed_event_name_is_matched_like_any_other_ended_in(
            &sm, A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_STATE_PASS)) {
        (void)fprintf(stderr, "transition_observer: FAIL - the observed run did not end in `pass`\n");
        bad = 1;
    }
    if (seen_other_machine) {
        (void)fprintf(stderr, "transition_observer: FAIL - the hook was handed another machine than the one run\n");
        bad = 1;
    }

    // The rows taken, in the order the run took them.
    static const a_computed_event_name_is_matched_like_any_other_state_t expected[] = {
        A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_STATE_S1,
        A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_STATE_S2,
        A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_STATE_S3,
        A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_STATE_S4,
        A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_STATE_S5,
        A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_STATE_S6,
    };
    const size_t expected_count = sizeof(expected) / sizeof(expected[0]);
    if (seen_count != expected_count) {
        (void)fprintf(stderr, "transition_observer: FAIL - the hook was told %zu transitions, not %zu\n", seen_count,
                      expected_count);
        bad = 1;
    } else {
        for (size_t i = 0; i < expected_count; ++i) {
            if (seen_source[i] != expected[i] || seen_index[i] != 0u) {
                (void)fprintf(stderr, "transition_observer: FAIL - transition %zu was told as (%d, %u)\n", i,
                              (int)seen_source[i], (unsigned)seen_index[i]);
                bad = 1;
            }
        }
    }

    // Held to the table: every pair told is a row, and every row was told.
    const a_computed_event_name_is_matched_like_any_other_transition_t *rows =
        a_computed_event_name_is_matched_like_any_other_transitions();
    const size_t row_count = (size_t)A_COMPUTED_EVENT_NAME_IS_MATCHED_LIKE_ANY_OTHER_TRANSITION_COUNT;
    const size_t told = seen_count < SEEN_MAX ? seen_count : SEEN_MAX;
    for (size_t i = 0; i < told; ++i) {
        int is_row = 0;
        for (size_t r = 0; r < row_count; ++r) {
            is_row |= rows[r].from == seen_source[i] && rows[r].index == seen_index[i];
        }
        if (!is_row) {
            (void)fprintf(stderr, "transition_observer: FAIL - (%d, %u) is no row of the table\n", (int)seen_source[i],
                          (unsigned)seen_index[i]);
            bad = 1;
        }
    }
    for (size_t r = 0; r < row_count; ++r) {
        int was_told = 0;
        for (size_t i = 0; i < told; ++i) {
            was_told |= rows[r].from == seen_source[i] && rows[r].index == seen_index[i];
        }
        if (!was_told) {
            (void)fprintf(stderr, "transition_observer: FAIL - row %zu of the table was never taken\n", r);
            bad = 1;
        }
    }

    a_computed_event_name_is_matched_like_any_other_destroy(&sm);
    return bad;
}
