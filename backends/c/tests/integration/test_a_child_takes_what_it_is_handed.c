// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0098, §scxml-6.4 — C11 AOT path: a value an invoke hands its child
// is the initial value of the child's top-level <data> whose id it names.
//
// The child's start takes the values before it declares its datamodel
// (`_init_with_parent` for a child that addresses its parent, `_init_invoked`
// for one that does not), so the <data>'s own value is never evaluated, the
// top-level <script>, the first <onentry> and the first macrostep read what was
// handed, state-local <data> keeps its own value, a name the child does not
// declare is not added, an object arrives whole, and a value the child cannot
// hold is its <data>'s initialization error. Each phase of the document lands
// its failure in a <final> of its own; the driver names the one reached.
//
// The parent keeps the values it hands on its own Lua stack until the child's
// start returns, and drops them then: the driver also holds the parent's stack
// to the height it had once the first invoke had run.
//
// Fixture (C-only):
// backends/c/tests/integration_resources/a_child_takes_what_it_is_handed/a_child_takes_what_it_is_handed.scxml.
// Its last phase leans on the bounds this target sets
// (`SCE_MAX_HANDED_VALUE_DEPTH` 4, `SCE_MAX_HANDED_VALUE_ENTRIES` 8, in
// backends/c/tests/CMakeLists.txt).
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_child_takes_what_it_is_handed ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdio.h>

#include "../require_started.h"
#include "a_child_takes_what_it_is_handed_sm.h"

typedef a_child_takes_what_it_is_handed_t sm_t;

#define P(name) A_CHILD_TAKES_WHAT_IT_IS_HANDED_##name

// A <final> of the document and what reaching it says broke.
typedef struct {
    a_child_takes_what_it_is_handed_state_t state;
    const char *meaning;
} verdict_t;

static const verdict_t FAILURES[] = {
    {P(STATE_FAILOWNEXPRESSIONRAISED),
     "a handed <data>'s own expression was evaluated and raised error.execution in the child"},
    {P(STATE_FAILOWNEXPRESSIONRAN), "a handed <data>'s own expression was evaluated and had its effect"},
    {P(STATE_FAILHANDEDVALUELOST), "a handed value did not reach the <data> it names"},
    {P(STATE_FAILSTATELOCALDATASEEDED), "a state-local <data> took a handed value"},
    {P(STATE_FAILUNMATCHEDNAMEADDED), "a handed name the child does not declare became a variable of the child"},
    {P(STATE_FAILOBJECTNOTWHOLE), "an object value did not arrive whole"},
    {P(STATE_FAILSCRIPTREADITSDEFAULT), "the child's top-level <script> ran before the handed value was declared"},
    {P(STATE_FAILENTRYREADITSDEFAULT), "the child's initial <onentry> ran before the handed value was declared"},
    {P(STATE_FAILUNHELDVALUERAISEDNOTHING),
     "a handed value past a bound raised no error.execution in the child, or raised it more than once each"},
    {P(STATE_FAILVALUEPASTITSBOUNDHELD), "a handed value past a bound was declared, or its <data>'s own value was"},
    {P(STATE_FAILPARENTRAISED), "the parent raised error.execution while it handed its arguments"},
};

// The phases a run that stops early is left in: each one's child never ended.
static const verdict_t STRANDED[] = {
    {P(STATE_INPLACE), "the child started through `_init_with_parent` never answered"},
    {P(STATE_FIRSTMACROSTEP),
     "the child started through `_init_invoked` took its first macrostep on its own value of `handed`"},
    {P(STATE_NAMELISTPHASE), "the object a `namelist` named did not arrive whole in the child's first macrostep"},
    {P(STATE_UNHELD), "the child holding values past its bounds never answered"},
};

int main(void) {
    sm_t sm;
    // The first invoke runs inside the start, so the parent's stack has handed
    // and dropped its values once by now.
    REQUIRE_STARTED(a_child_takes_what_it_is_handed_init(&sm), P(START_OK));
    const int stack_after_start = lua_gettop(sm.L);

    // No `<send delay>`: every child reaches its <final> while its invoke
    // starts it or in the steps the parent drives right after, and the rest
    // is the parent's own queue, all inside `_run`.
    a_child_takes_what_it_is_handed_run(&sm);

    int failures = 0;
    const int passed = a_child_takes_what_it_is_handed_ended_in(&sm, P(STATE_PASS));
    if (!passed) {
        ++failures;
        int named = 0;
        for (size_t i = 0u; i < sizeof(FAILURES) / sizeof(FAILURES[0]); ++i) {
            if (a_child_takes_what_it_is_handed_ended_in(&sm, FAILURES[i].state)) {
                (void)fprintf(stderr, "a_child_takes_what_it_is_handed: FAIL - %s\n", FAILURES[i].meaning);
                named = 1;
            }
        }
        for (size_t i = 0u; i < sizeof(STRANDED) / sizeof(STRANDED[0]); ++i) {
            if (a_child_takes_what_it_is_handed_in_state(&sm, STRANDED[i].state)) {
                (void)fprintf(stderr, "a_child_takes_what_it_is_handed: FAIL - %s\n", STRANDED[i].meaning);
                named = 1;
            }
        }
        if (!named) {
            (void)fprintf(stderr, "a_child_takes_what_it_is_handed: FAIL - the run ended in no state the "
                                  "document names a verdict for\n");
        }
    }

    // Three more invokes handed their values during the run; each start
    // returned and each drop put the stack back.
    const int stack_after_run = lua_gettop(sm.L);
    if (stack_after_run != stack_after_start) {
        ++failures;
        (void)fprintf(stderr,
                      "a_child_takes_what_it_is_handed: FAIL - the parent's Lua stack held %d value(s) after "
                      "its start and %d after its run: the values an invoke handed were not dropped once its "
                      "child's start returned\n",
                      stack_after_start, stack_after_run);
    }

    a_child_takes_what_it_is_handed_destroy(&sm);
    return failures == 0 ? 0 : 1;
}
