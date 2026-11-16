// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0098, §scxml-6.4 — C11 AOT path: a value an invoke hands its child
// is the initial value of the child's top-level <data> whose id it names.
//
// The child's start takes the values before it declares its datamodel
// (`_init_with_parent` for a child that addresses its parent, `_init_invoked`
// for one that does not), so the <data>'s own value is never evaluated, the
// top-level <script>, the first <onentry> and the first macrostep read what was
// handed, state-local <data>, early- or late-bound, keeps its own value, a name
// the child does not declare is not added, and an object arrives whole, as the
// child's own copy. The parent judges what each child reported and lands a
// failure in a <final> of its own; the driver names the one reached.
//
// The parent keeps the values it hands on its own Lua stack until the child's
// start returns, and drops them then: the driver also holds the parent's stack
// to the height it had once the first invoke had run.
//
// Fixture:
// integration_resources/a_child_takes_a_handed_value_in_place_of_its_data/a_child_takes_a_handed_value_in_place_of_its_data.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_child_takes_a_handed_value_in_place_of_its_data ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "../require_started.h"
#include "a_child_takes_a_handed_value_in_place_of_its_data_sm.h"

typedef a_child_takes_a_handed_value_in_place_of_its_data_t sm_t;

#define P(name) A_CHILD_TAKES_A_HANDED_VALUE_IN_PLACE_OF_ITS_DATA_##name

// A state of the document and what a run left in it says broke.
typedef struct {
    a_child_takes_a_handed_value_in_place_of_its_data_state_t state;
    const char *meaning;
} verdict_t;

static const verdict_t FAILURES[] = {
    {P(STATE_FAILOWNVALUERAN), "a handed <data>'s own expression was evaluated and had its effect"},
    {P(STATE_FAILHANDEDVALUELOST), "a handed value did not reach the top-level <data> it names"},
    {P(STATE_FAILSTATEDATATOOKTHEVALUE), "an early-bound state <data> took a handed value"},
    {P(STATE_FAILUNMATCHEDNAMEADDED), "a handed name the child does not declare became a variable of the child"},
    {P(STATE_FAILOBJECTNOTWHOLE), "an object value did not arrive whole"},
    {P(STATE_FAILCHILDSHAREDTHEPARENTSOBJECT), "the child's write into the object it was handed reached the parent"},
    {P(STATE_FAILSCRIPTREADITSOWNVALUE), "the child's top-level <script> ran before the handed values were declared"},
    {P(STATE_FAILENTRYREADITSOWNVALUE), "the child's initial <onentry> ran before the handed values were declared"},
    {P(STATE_FAILCHILDRAISED), "a handed <data>'s own expression was evaluated and raised error.execution"},
    {P(STATE_FAILNAMELISTOWNVALUERAISED), "the own expression of a <data> a namelist value was handed for raised"},
    {P(STATE_FAILNAMELISTVALUELOST), "a namelist value did not reach the top-level <data> it names"},
    {P(STATE_FAILNAMELISTNAMEADDED), "a namelist name the child does not declare became a variable of the child"},
    {P(STATE_FAILLATESTATEDATATOOKTHEVALUE), "a late-bound state <data> took a handed value"},
    {P(STATE_FAILPARENTRAISED), "the parent raised error.execution while it handed its arguments"},
};

// The phases a run that stops early is left in: each one's child never ended.
static const verdict_t STRANDED[] = {
    {P(STATE_BYPARAM), "the child handed <param>s, started through `_init_with_parent`, never answered"},
    {P(STATE_BYNAMELIST), "the child handed `namelist` values never answered"},
    {P(STATE_LATEBOUND), "the late-bound child never answered"},
    {P(STATE_FIRSTMACROSTEP),
     "the child started through `_init_invoked` took its first macrostep on its own value of `handed`"},
};

int main(void) {
    sm_t sm;
    // The first invoke runs inside the start, so the parent's stack has handed
    // and dropped its values once by now.
    REQUIRE_STARTED(a_child_takes_a_handed_value_in_place_of_its_data_init(&sm), P(START_OK));
    const int stack_after_start = lua_gettop(sm.L);

    // No `<send delay>`: every child reaches its <final> while its invoke
    // starts it or in the steps the parent drives right after, and the rest
    // is the parent's own queue, all inside `_run`.
    a_child_takes_a_handed_value_in_place_of_its_data_run(&sm);

    int failures = 0;
    const int passed = a_child_takes_a_handed_value_in_place_of_its_data_ended_in(&sm, P(STATE_PASS));
    if (!passed) {
        ++failures;
        int named = 0;
        for (size_t i = 0u; i < sizeof(FAILURES) / sizeof(FAILURES[0]); ++i) {
            if (a_child_takes_a_handed_value_in_place_of_its_data_ended_in(&sm, FAILURES[i].state)) {
                (void)fprintf(stderr, "a_child_takes_a_handed_value_in_place_of_its_data: FAIL - %s\n",
                              FAILURES[i].meaning);
                named = 1;
            }
        }
        for (size_t i = 0u; i < sizeof(STRANDED) / sizeof(STRANDED[0]); ++i) {
            if (a_child_takes_a_handed_value_in_place_of_its_data_in_state(&sm, STRANDED[i].state)) {
                (void)fprintf(stderr, "a_child_takes_a_handed_value_in_place_of_its_data: FAIL - %s\n",
                              STRANDED[i].meaning);
                named = 1;
            }
        }
        if (!named) {
            (void)fprintf(stderr, "a_child_takes_a_handed_value_in_place_of_its_data: FAIL - the run ended in no "
                                  "state the document names a verdict for\n");
        }
    }

    // Three more invokes handed their values during the run; each start
    // returned and each drop put the stack back.
    const int stack_after_run = lua_gettop(sm.L);
    if (stack_after_run != stack_after_start) {
        ++failures;
        (void)fprintf(stderr,
                      "a_child_takes_a_handed_value_in_place_of_its_data: FAIL - the parent's Lua stack held %d "
                      "value(s) after its start and %d after its run: the values an invoke handed were not dropped "
                      "once its child's start returned\n",
                      stack_after_start, stack_after_run);
    }

    a_child_takes_a_handed_value_in_place_of_its_data_destroy(&sm);
    return failures == 0 ? 0 : 1;
}
