// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0098, §scxml-6.4 with §scxml-5.3 — C11 AOT path: a value an invoke
// hands its child that the child's session cannot hold is the error of the
// <data> it was handed for. The C11 child copies a handed value from its
// parent's Lua state into its own, and a value past the bounds of that copy
// cannot be held: error.execution in the child, the <data> left undefined, its
// own value not tried, and every other handed value still taken.
//
// What every channel holds of docs/adr/0098 is the canonical fixture and its
// driver here, test_a_child_takes_a_handed_value_in_place_of_its_data.c; this
// one asks what only C11 has, the bounds.
//
// Fixture (C-only):
// backends/c/tests/integration_resources/a_handed_value_past_its_bounds_is_its_datas_error/a_handed_value_past_its_bounds_is_its_datas_error.scxml.
// It leans on the bounds this target sets (`SCE_MAX_HANDED_VALUE_DEPTH` 4,
// `SCE_MAX_HANDED_VALUE_ENTRIES` 8, in backends/c/tests/CMakeLists.txt).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_handed_value_past_its_bounds_is_its_datas_error ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "../require_started.h"
#include "a_handed_value_past_its_bounds_is_its_datas_error_sm.h"

typedef a_handed_value_past_its_bounds_is_its_datas_error_t sm_t;

#define P(name) A_HANDED_VALUE_PAST_ITS_BOUNDS_IS_ITS_DATAS_ERROR_##name

// A state of the document and what a run left in it says broke.
typedef struct {
    a_handed_value_past_its_bounds_is_its_datas_error_state_t state;
    const char *meaning;
} verdict_t;

static const verdict_t FAILURES[] = {
    {P(STATE_FAILUNHELDVALUERAISEDNOTHING),
     "a handed value past a bound raised no error.execution in the child, or raised it more than once each"},
    {P(STATE_FAILVALUEPASTITSBOUNDHELD), "a handed value past a bound was declared, or its <data>'s own value was"},
    {P(STATE_FAILHANDEDVALUELOST), "the value handed beside the unheld ones did not arrive"},
    {P(STATE_FAILPARENTRAISED), "the parent raised error.execution while it handed its arguments"},
    {P(STATE_UNHELD), "the child holding values past its bounds never answered"},
    {P(STATE_IDLE), "the machine never took `go`, so its invoke never ran"},
};

int main(void) {
    sm_t sm;
    // The start runs the machine until nothing is left for it to do, so an
    // invoke of the initial state would have handed and dropped its values
    // before this line. The invoke waits for `go` instead: read here, the
    // stack is that of a parent that has handed nothing.
    REQUIRE_STARTED(a_handed_value_past_its_bounds_is_its_datas_error_init(&sm), P(START_OK));
    const int stack_after_start = lua_gettop(sm.L);

    int failures = 0;
    if (!a_handed_value_past_its_bounds_is_its_datas_error_in_state(&sm, P(STATE_IDLE))) {
        ++failures;
        (void)fprintf(stderr, "a_handed_value_past_its_bounds_is_its_datas_error: FAIL - the machine did not wait "
                              "in `idle` for `go`\n");
    }

    a_handed_value_past_its_bounds_is_its_datas_error_event_with_meta_t go = {0};
    go.event = P(EVENT_GO);
    a_handed_value_past_its_bounds_is_its_datas_error_raise_external(&sm, &go);
    a_handed_value_past_its_bounds_is_its_datas_error_run(&sm);

    if (!a_handed_value_past_its_bounds_is_its_datas_error_ended_in(&sm, P(STATE_PASS))) {
        ++failures;
        int named = 0;
        for (size_t i = 0u; i < sizeof(FAILURES) / sizeof(FAILURES[0]); ++i) {
            // A <final> the run ended in, or the phase it was left waiting in.
            if (a_handed_value_past_its_bounds_is_its_datas_error_ended_in(&sm, FAILURES[i].state) ||
                a_handed_value_past_its_bounds_is_its_datas_error_in_state(&sm, FAILURES[i].state)) {
                (void)fprintf(stderr, "a_handed_value_past_its_bounds_is_its_datas_error: FAIL - %s\n",
                              FAILURES[i].meaning);
                named = 1;
            }
        }
        if (!named) {
            (void)fprintf(stderr, "a_handed_value_past_its_bounds_is_its_datas_error: FAIL - the run ended in no "
                                  "state the document names a verdict for\n");
        }
    }

    // The invoke ran in the run: it held its three values on the parent's
    // stack, its child copied the one it could hold and refused the two past a
    // bound, and the values were dropped once the child's start returned, so
    // the stack is as the start left it.
    const int stack_after_run = lua_gettop(sm.L);
    if (stack_after_run != stack_after_start) {
        ++failures;
        (void)fprintf(stderr,
                      "a_handed_value_past_its_bounds_is_its_datas_error: FAIL - the parent's Lua stack held %d "
                      "value(s) after its start and %d after its run: the values its invoke handed were not "
                      "dropped once the child's start returned\n",
                      stack_after_start, stack_after_run);
    }

    a_handed_value_past_its_bounds_is_its_datas_error_destroy(&sm);
    return failures == 0 ? 0 : 1;
}
