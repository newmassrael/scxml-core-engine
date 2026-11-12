// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/adr/0096 — C11 AOT path: a generated machine whose script session
// cannot be established does not start, and its host is told why.
//
// The script engine is the build's Lua, reached through the one seam the C11
// machine has for it: the factory of its Lua state, which a build names with
// `SCE_LUA_STATE_FACTORY` (a device's own allocator is what it is for). This
// target names `failing_lua_state` below, which makes every state over an
// allocator that counts what is outstanding, and fails on request one of the
// three steps that establish a session:
//
//   * `create`: it answers NULL, as `luaL_newstate` does when the memory is not
//     there;
//   * `library`: it hands back a state whose call hook refuses the first Lua
//     chunk run on it, which is the first chunk of the shared library;
//   * `system-variables`: it hands back a state whose call hook refuses every
//     call of the binder the shared library defines, `_scxml_bind_system`.
//
// The first `spare` states it is asked for are made whole, so a case can fail
// the session of the child alone: the parent's is the first a start asks for,
// the child's the second.
//
// Fixture:
// integration_resources/a_child_may_send_many_events_in_one_tick/a_child_may_send_many_events_in_one_tick.scxml,
// reused for its shape: a parent in `s0` with `<data ticks>` that invokes an
// inline child, whose `<onentry>` sends `tick` to the parent 110 times. The
// child declares `<data>` of its own, so it has a script session of its own to
// fail. The target raises SCE_MAX_EVENTS above the 110 ticks, as the target
// that holds the fixture's delivery does, so the control runs it whole.
//
// The C++, Rust, Go and Kotlin channels hold the same cases:
// tests/integration/AMachineWhoseScriptSessionFailsDoesNotStartAotTest.cpp and
// its siblings named in SCE_ERROR_CONTRACT.md §11.
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_c_test(a_child_may_send_many_events_in_one_tick ...)`
// in `backends/c/tests/CMakeLists.txt`.

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "a_child_may_send_many_events_in_one_tick_sm.h"

typedef a_child_may_send_many_events_in_one_tick_t sm_t;

#define P(name) A_CHILD_MAY_SEND_MANY_EVENTS_IN_ONE_TICK_##name

static int failures = 0;

#define CHECK(condition, ...)                                                                                          \
    do {                                                                                                               \
        if (!(condition)) {                                                                                            \
            (void)fprintf(stderr, "FAIL %s:%d: ", __FILE__, __LINE__);                                                 \
            (void)fprintf(stderr, __VA_ARGS__);                                                                        \
            (void)fputc('\n', stderr);                                                                                 \
            ++failures;                                                                                                \
        }                                                                                                              \
    } while (0)

// ── The engine: the build's Lua, one step failing on request ──────────────

// Which step of establishing a session the factory fails.
typedef enum {
    FAIL_NEVER,
    FAIL_CREATE,
    FAIL_LIBRARY,
    FAIL_SYSTEM_VARIABLES,
} fail_t;

static struct {
    fail_t fail;
    // States made whole before the factory starts failing.
    unsigned spare;
    // States the factory was asked for.
    unsigned made;
    // Bytes the states it made hold, through `counting_alloc`.
    size_t outstanding;
} engine;

static const char *const LIBRARY_REFUSED = "the test's engine loads no library";
static const char *const BIND_REFUSED = "the test's engine binds no system variable";

static void engine_reset(fail_t fail, unsigned spare) {
    engine.fail = fail;
    engine.spare = spare;
    engine.made = 0u;
}

static const char *name_of(fail_t fail) {
    switch (fail) {
    case FAIL_NEVER:
        return "never";
    case FAIL_CREATE:
        return "create";
    case FAIL_LIBRARY:
        return "library";
    case FAIL_SYSTEM_VARIABLES:
        return "system-variables";
    }
    return "";
}

static void *counting_alloc(void *ud, void *ptr, size_t osize, size_t nsize) {
    (void)ud;
    // With no block, `osize` names the kind of object Lua is making, not a size.
    const size_t held = ptr != NULL ? osize : 0u;
    if (nsize == 0u) {
        free(ptr);
        engine.outstanding -= held;
        return NULL;
    }
    void *grown = realloc(ptr, nsize);
    if (grown != NULL) {
        engine.outstanding = engine.outstanding - held + nsize;
    }
    return grown;
}

// The call hook of a state whose `library` or `system-variables` step fails.
// A hook runs inside the protected call that called the function, so its error
// is that call's failure: the chunk's `luaL_dostring`, or the binder's
// `lua_pcall`, answers it with the words below.
static void refuse_on_call(lua_State *L, lua_Debug *ar) {
    if (engine.fail == FAIL_LIBRARY) {
        // The standard libraries open as C functions; the first Lua chunk run
        // on a state is the first chunk of the shared library.
        if (lua_getinfo(L, "S", ar) != 0 && strcmp(ar->what, "main") == 0) {
            (void)luaL_error(L, "%s", LIBRARY_REFUSED);
        }
        return;
    }
    if (lua_getinfo(L, "f", ar) == 0) {
        return;
    }
    lua_pushglobaltable(L);
    lua_pushliteral(L, "_scxml_bind_system");
    (void)lua_rawget(L, -2);
    const bool binder = lua_rawequal(L, -1, -3) != 0;
    lua_pop(L, 3);
    if (binder) {
        (void)luaL_error(L, "%s", BIND_REFUSED);
    }
}

// The factory this target names (`SCE_LUA_STATE_FACTORY=failing_lua_state`),
// which every machine of the target calls for its state.
lua_State *failing_lua_state(void);

lua_State *failing_lua_state(void) {
    const unsigned ordinal = engine.made++;
    const bool failing = engine.fail != FAIL_NEVER && ordinal >= engine.spare;
    if (failing && engine.fail == FAIL_CREATE) {
        return NULL;
    }
    lua_State *L = lua_newstate(counting_alloc, NULL);
    if (L != NULL && failing) {
        lua_sethook(L, refuse_on_call, LUA_MASKCALL, 0);
    }
    return L;
}

// ── What a refused start leaves ────────────────────────────────────────────

static sce_script_session_stage_t stage_of(fail_t fail) {
    switch (fail) {
    case FAIL_CREATE:
        return SCE_SCRIPT_SESSION_STAGE_CREATE;
    case FAIL_LIBRARY:
        return SCE_SCRIPT_SESSION_STAGE_LIBRARY;
    case FAIL_SYSTEM_VARIABLES:
        return SCE_SCRIPT_SESSION_STAGE_SYSTEM_VARIABLES;
    case FAIL_NEVER:
        break;
    }
    return SCE_SCRIPT_SESSION_STAGE_NONE;
}

// The failure names the step, in the word every backend uses, and carries the
// engine's words.
static void expect_failure(const sm_t *sm, fail_t fail) {
    const sce_script_session_failure_t *failure = a_child_may_send_many_events_in_one_tick_script_session_failure(sm);
    CHECK(failure != NULL, "%s: a refused start says why", name_of(fail));
    if (failure == NULL) {
        return;
    }
    CHECK(failure->stage == stage_of(fail), "%s: the refusal names the step, not %d", name_of(fail),
          (int)failure->stage);
    CHECK(strcmp(sce_script_session_stage_name(failure->stage), name_of(fail)) == 0, "%s: the step is named `%s`",
          name_of(fail), sce_script_session_stage_name(failure->stage));
    switch (fail) {
    case FAIL_CREATE:
        CHECK(strcmp(failure->message, "the Lua state could not be allocated") == 0, "create: the words are `%s`",
              failure->message);
        break;
    case FAIL_LIBRARY:
        CHECK(strstr(failure->message, LIBRARY_REFUSED) != NULL, "library: the engine's words are kept, not `%s`",
              failure->message);
        break;
    case FAIL_SYSTEM_VARIABLES:
        CHECK(strstr(failure->message, BIND_REFUSED) != NULL, "system-variables: the binder's words are kept, not `%s`",
              failure->message);
        break;
    case FAIL_NEVER:
        break;
    }
}

// A refused start left nothing behind: no Lua state kept and none still
// holding memory, no state entered, the machine not running, nothing queued —
// error.execution included, which a failed bind used to raise — and no child
// asked for, since no state was entered to invoke one.
static void expect_nothing_started(const sm_t *sm, fail_t fail, unsigned asked) {
    CHECK(sm->L == NULL, "%s: a refused start keeps no Lua state", name_of(fail));
    CHECK(engine.outstanding == 0u, "%s: what the steps made is released (%zu bytes held)", name_of(fail),
          engine.outstanding);
    CHECK(a_child_may_send_many_events_in_one_tick_active_states(sm) == 0u, "%s: a refused start enters no state",
          name_of(fail));
    CHECK(!sm->is_running, "%s: a refused start leaves the machine not running", name_of(fail));
    CHECK(!a_child_may_send_many_events_in_one_tick_is_in_final_state(sm), "%s: a refused start reaches no final",
          name_of(fail));
    CHECK(sm->internal_queue.count == 0u, "%s: nothing is raised in the machine, error.execution included",
          name_of(fail));
    CHECK(sm->external_queue.count == 0u, "%s: nothing is queued from outside", name_of(fail));
    CHECK(engine.made == asked, "%s: %u state(s) asked for, not %u: no child was started", name_of(fail), asked,
          engine.made);
}

// ── The cases ──────────────────────────────────────────────────────────────

// The control: the factory fails nothing, so the machine is the fixture's, and
// its child's ticks arrive. Without it the cases below could pass over a
// factory that breaks every machine.
static void a_start_over_the_factory_that_fails_nothing_runs_the_fixture(void) {
    static sm_t sm;
    engine_reset(FAIL_NEVER, 0u);
    CHECK(a_child_may_send_many_events_in_one_tick_init(&sm) == P(START_OK),
          "a session that is established starts the machine");
    CHECK(a_child_may_send_many_events_in_one_tick_script_session_failure(&sm) == NULL,
          "a machine that started names no failure");
    a_child_may_send_many_events_in_one_tick_run(&sm);
    a_child_may_send_many_events_in_one_tick_event_with_meta_t carrier = {0};
    carrier.event = P(EVENT_FINISH);
    a_child_may_send_many_events_in_one_tick_raise_external(&sm, &carrier);
    a_child_may_send_many_events_in_one_tick_run(&sm);
    int64_t ticks = -1;
    CHECK(a_child_may_send_many_events_in_one_tick_ended_in(&sm, P(STATE_DONE)), "`finish` ends the run in `done`");
    CHECK(a_child_may_send_many_events_in_one_tick_ticks(&sm, &ticks) && ticks == 110,
          "every tick the child sent arrived: %lld", (long long)ticks);
    CHECK(engine.made == 2u, "the parent and its child asked for a state each, not %u", engine.made);
    CHECK(a_child_may_send_many_events_in_one_tick_unhandled_error_events(&sm) == 0u, "nothing failed");
    a_child_may_send_many_events_in_one_tick_destroy(&sm);
    CHECK(engine.outstanding == 0u, "the states are closed with the machine (%zu bytes held)", engine.outstanding);
}

static void a_start_whose_session_cannot_be_established_is_refused_with_the_step(void) {
    static const fail_t steps[] = {FAIL_CREATE, FAIL_LIBRARY, FAIL_SYSTEM_VARIABLES};
    for (size_t i = 0u; i < sizeof(steps) / sizeof(steps[0]); ++i) {
        static sm_t sm;
        engine_reset(steps[i], 0u);
        CHECK(a_child_may_send_many_events_in_one_tick_init(&sm) == P(START_SCRIPT_SESSION_UNAVAILABLE),
              "%s: a session that cannot be established refuses the start", name_of(steps[i]));
        expect_failure(&sm, steps[i]);
        expect_nothing_started(&sm, steps[i], 1u);
        a_child_may_send_many_events_in_one_tick_destroy(&sm);
    }
}

// A session that failed is not kept as established: the next start asks for
// it again and is refused again, rather than running the machine over no
// session.
static void a_refused_start_is_refused_again_rather_than_run_without_a_session(void) {
    static const fail_t steps[] = {FAIL_CREATE, FAIL_LIBRARY, FAIL_SYSTEM_VARIABLES};
    for (size_t i = 0u; i < sizeof(steps) / sizeof(steps[0]); ++i) {
        static sm_t sm;
        engine_reset(steps[i], 0u);
        CHECK(a_child_may_send_many_events_in_one_tick_init(&sm) == P(START_SCRIPT_SESSION_UNAVAILABLE), "%s",
              name_of(steps[i]));
        CHECK(a_child_may_send_many_events_in_one_tick_init(&sm) == P(START_SCRIPT_SESSION_UNAVAILABLE),
              "%s: a second start over the same failing engine is refused too", name_of(steps[i]));
        expect_failure(&sm, steps[i]);
        expect_nothing_started(&sm, steps[i], 2u);
    }
}

// The machine of a refused start takes no event: what the host raises on it is
// counted as unseen, and nothing runs.
static void a_refused_machine_takes_no_event(void) {
    static sm_t sm;
    engine_reset(FAIL_CREATE, 0u);
    CHECK(a_child_may_send_many_events_in_one_tick_init(&sm) == P(START_SCRIPT_SESSION_UNAVAILABLE),
          "the start is refused");
    a_child_may_send_many_events_in_one_tick_event_with_meta_t carrier = {0};
    carrier.event = P(EVENT_FINISH);
    a_child_may_send_many_events_in_one_tick_raise_external(&sm, &carrier);
    a_child_may_send_many_events_in_one_tick_step(&sm);
    a_child_may_send_many_events_in_one_tick_run(&sm);
    CHECK(a_child_may_send_many_events_in_one_tick_unseen_external_events(&sm) == 1u,
          "the event is counted as one a machine that is not running never looked at");
    CHECK(!a_child_may_send_many_events_in_one_tick_ended_in(&sm, P(STATE_DONE)), "`finish` was not taken");
    expect_nothing_started(&sm, FAIL_CREATE, 1u);
    a_child_may_send_many_events_in_one_tick_destroy(&sm);
}

// The resumed start establishes the session before it stands the machine in
// the configuration, and one that cannot be leaves it in none.
static void a_resumed_start_whose_session_cannot_be_established_is_refused(void) {
    static sm_t sm;
    static const a_child_may_send_many_events_in_one_tick_state_t at[] = {P(STATE_S0)};
    engine_reset(FAIL_CREATE, 0u);
    CHECK(a_child_may_send_many_events_in_one_tick_init_at(&sm, at, 1u, NULL) == P(CONFIG_SCRIPT_SESSION_UNAVAILABLE),
          "a resume whose session cannot be established is refused");
    expect_failure(&sm, FAIL_CREATE);
    expect_nothing_started(&sm, FAIL_CREATE, 1u);
    CHECK(strstr(a_child_may_send_many_events_in_one_tick_configuration_rejection_text(
                     P(CONFIG_SCRIPT_SESSION_UNAVAILABLE)),
                 "script session") != NULL,
          "the rejection says what it was");
}

// A root started under the id its host gives is refused the same way, once the
// id was taken.
static void a_root_start_under_a_given_id_is_refused_for_the_session(void) {
    static sm_t sm;
    engine_reset(FAIL_LIBRARY, 0u);
    const sce_start_refusal_t refused =
        a_child_may_send_many_events_in_one_tick_init_with_session_id(&sm, "root-1", NULL);
    CHECK(refused == SCE_START_REFUSAL_SCRIPT_SESSION_UNAVAILABLE, "the root start is refused for the session: %s",
          sce_start_refusal_text(refused));
    expect_failure(&sm, FAIL_LIBRARY);
    expect_nothing_started(&sm, FAIL_LIBRARY, 1u);
}

// §scxml-6.4: a child whose session fails is a failed invoke. The parent keeps
// no child, raises error.execution, which no transition of `s0` takes, and no
// `done.invoke`; it runs on.
static void a_child_whose_session_fails_leaves_its_parent_with_error_execution_and_no_child(void) {
    static const fail_t steps[] = {FAIL_CREATE, FAIL_LIBRARY, FAIL_SYSTEM_VARIABLES};
    for (size_t i = 0u; i < sizeof(steps) / sizeof(steps[0]); ++i) {
        static sm_t sm;
        engine_reset(steps[i], 1u);
        CHECK(a_child_may_send_many_events_in_one_tick_init(&sm) == P(START_OK),
              "%s: the parent's own session is established", name_of(steps[i]));
        a_child_may_send_many_events_in_one_tick_run(&sm);
        CHECK(engine.made == 2u, "%s: the parent asked for its own state and the child's, not %u", name_of(steps[i]),
              engine.made);
        // The slot of `<invoke id="chatty">` holds no child: nothing drives,
        // cancels or destroys a session that never started.
        CHECK(!sm.child_chatty_active, "%s: the child whose start was refused is not kept", name_of(steps[i]));
        a_child_may_send_many_events_in_one_tick_event_t error = P(EVENT_FINISH);
        CHECK(a_child_may_send_many_events_in_one_tick_unhandled_error_events(&sm) == 1u,
              "%s: the invoke ends in one error event", name_of(steps[i]));
        CHECK(a_child_may_send_many_events_in_one_tick_last_unhandled_error(&sm, &error) &&
                  error == P(EVENT_ERROR_EXECUTION),
              "%s: the error is error.execution", name_of(steps[i]));
        int64_t ticks = -1;
        CHECK(a_child_may_send_many_events_in_one_tick_ticks(&sm, &ticks) && ticks == 0,
              "%s: no child ran, so no tick arrived: %lld", name_of(steps[i]), (long long)ticks);
        CHECK(a_child_may_send_many_events_in_one_tick_discarded_external_events(&sm) == 0u,
              "%s: no done.invoke was raised for the child", name_of(steps[i]));
        CHECK(sm.is_running && a_child_may_send_many_events_in_one_tick_in_state(&sm, P(STATE_S0)),
              "%s: the parent runs on in `s0`", name_of(steps[i]));
        a_child_may_send_many_events_in_one_tick_event_with_meta_t carrier = {0};
        carrier.event = P(EVENT_FINISH);
        a_child_may_send_many_events_in_one_tick_raise_external(&sm, &carrier);
        a_child_may_send_many_events_in_one_tick_run(&sm);
        CHECK(a_child_may_send_many_events_in_one_tick_ended_in(&sm, P(STATE_DONE)), "%s: `finish` ends the parent",
              name_of(steps[i]));
        a_child_may_send_many_events_in_one_tick_destroy(&sm);
        CHECK(engine.outstanding == 0u, "%s: the child's state was closed when it failed (%zu bytes held)",
              name_of(steps[i]), engine.outstanding);
    }
}

int main(void) {
    a_start_over_the_factory_that_fails_nothing_runs_the_fixture();
    a_start_whose_session_cannot_be_established_is_refused_with_the_step();
    a_refused_start_is_refused_again_rather_than_run_without_a_session();
    a_refused_machine_takes_no_event();
    a_resumed_start_whose_session_cannot_be_established_is_refused();
    a_root_start_under_a_given_id_is_refused_for_the_session();
    a_child_whose_session_fails_leaves_its_parent_with_error_execution_and_no_child();
    if (failures != 0) {
        (void)fprintf(stderr, "%d check(s) failed\n", failures);
        return 1;
    }
    (void)puts("PASS: a machine whose script session fails does not start");
    return 0;
}
