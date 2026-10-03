// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15) under generated C:
// a variable is a member of the machine's policy struct, every guard and
// statement was lowered to C at build time, and no script engine is built.
//
// The scenarios are the ones Kotlin, Rust, Go, Python, C++ and the Interpreter
// replay — `sce-build/tests/fixtures/static_datamodel/scenarios/<machine>.json`,
// whose expected values are derived from the document, not observed from a
// backend — so the engines are held to one answer. The machines are generated
// from the fixtures beside them by `backends/c/tests/CMakeLists.txt`.
//
// What a failing scenario would say:
//
//   * `static_counter`: a guard reading a variable and `In()`, `<assign>`,
//     `<if>`/`<elseif>` and `<log>` are native, and `count` reaches the host as
//     the `uint32` it is.
//   * `static_overflow`: a checked integer operation that overflows is a
//     failure, not a wrapped value (SCE_FORGE.md §3.4.1) — the variable keeps
//     what it held, `error.execution` is raised, and a guard over the
//     overflowing sum is false.
//   * `static_block_ends`: an error ends the block it stands in (§scxml-4.9) —
//     the statements after a failed `<assign>`, after a failed `<if>` cond, or
//     inside a branch that failed do not run, while the next block does.
//   * `static_payload`: an event's typed payload is read in a guard and in
//     assignments through the channel the machine declares for it, lifted from
//     the `data` the event carries; an operation over it that overflows is a
//     failure like any other, and the statements before it have run.
//
// Linked WITHOUT `sce_c_scripting` and `lua54` on purpose: these machines carry
// no script engine, so the link is the proof. If the emit ever reached for one,
// this target would stop linking, which is a better gate than a comment.

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "static_block_ends_sm.h"
#include "static_counter_sm.h"
#include "static_overflow_sm.h"
#include "static_payload_sm.h"

#include "static_scenario.h"

#ifndef SCE_STATIC_SCENARIO_DIR
#error "SCE_STATIC_SCENARIO_DIR must name the shared scenarios (backends/c/tests/CMakeLists.txt)"
#endif

typedef struct {
    const char *name;
    int value;
} name_value_t;

typedef struct {
    const char *name;
    int64_t (*read)(const void *sm);
} variable_t;

static bool find(const name_value_t *table, size_t count, const char *name, int *out) {
    for (size_t i = 0; i < count; ++i) {
        if (strcmp(table[i].name, name) == 0) {
            *out = table[i].value;
            return true;
        }
    }
    return false;
}

#define COUNT_OF(table) (sizeof(table) / sizeof((table)[0]))

// A published variable's reader, as the int64 a scenario compares it as.
#define VARIABLE_READER(M, VAR)                                                                                        \
    static int64_t M##_read_##VAR(const void *sm) {                                                                    \
        return (int64_t)M##_get_##VAR((const M##_t *)sm);                                                              \
    }

// One machine as `sce_scenario_driver_t` asks for it: the event by its document
// name, the state by its, the variable by its, and the replay of its scenario.
#define STATIC_SCENARIO(M, EVENTS, STATES, VARIABLES)                                                                  \
    static bool M##_raise_event(void *sm, const char *name, const char *data) {                                        \
        int event = 0;                                                                                                 \
        if (!find(EVENTS, COUNT_OF(EVENTS), name, &event)) {                                                           \
            return false;                                                                                              \
        }                                                                                                              \
        M##_event_with_meta_t meta;                                                                                    \
        memset(&meta, 0, sizeof(meta));                                                                                \
        meta.event = (M##_event_t)event;                                                                               \
        if (data != NULL) {                                                                                            \
            if (strlen(data) >= sizeof(meta.data)) {                                                                   \
                return false;                                                                                          \
            }                                                                                                          \
            memcpy(meta.data, data, strlen(data) + 1u);                                                                \
        }                                                                                                              \
        M##_raise_external((M##_t *)sm, &meta);                                                                        \
        M##_step((M##_t *)sm);                                                                                         \
        return true;                                                                                                   \
    }                                                                                                                  \
    static int M##_state_active(void *sm, const char *name) {                                                          \
        int state = 0;                                                                                                 \
        if (!find(STATES, COUNT_OF(STATES), name, &state)) {                                                           \
            return -1;                                                                                                 \
        }                                                                                                              \
        return M##_in_state((const M##_t *)sm, (M##_state_t)state) ? 1 : 0;                                            \
    }                                                                                                                  \
    static bool M##_run_ended(void *sm) {                                                                              \
        return M##_is_in_final_state((const M##_t *)sm);                                                               \
    }                                                                                                                  \
    static bool M##_read_variable(void *sm, const char *name, int64_t *out) {                                          \
        for (size_t i = 0; i < COUNT_OF(VARIABLES); ++i) {                                                             \
            if (strcmp(VARIABLES[i].name, name) == 0) {                                                                \
                *out = VARIABLES[i].read(sm);                                                                          \
                return true;                                                                                           \
            }                                                                                                          \
        }                                                                                                              \
        return false;                                                                                                  \
    }                                                                                                                  \
    static int M##_scenario(const char *scenario, int min_steps) {                                                     \
        M##_t sm;                                                                                                      \
        M##_init(&sm);                                                                                                 \
        const sce_scenario_driver_t driver = {                                                                         \
            #M, &sm, M##_raise_event, M##_state_active, M##_run_ended, M##_read_variable};                             \
        char path[512];                                                                                                \
        (void)snprintf(path, sizeof(path), "%s/%s.json", SCE_STATIC_SCENARIO_DIR, scenario);                           \
        int replayed = 0;                                                                                              \
        int bad = sce_scenario_replay(path, &driver, &replayed);                                                       \
        /* A floor: a scenario that lost its steps would pass every one it has left. */                                \
        if (replayed < min_steps) {                                                                                    \
            (void)fprintf(stderr, "%s: FAIL - the scenario %s lost steps: %d of at least %d\n", #M, scenario,          \
                          replayed, min_steps);                                                                        \
            bad = 1;                                                                                                   \
        }                                                                                                              \
        return bad;                                                                                                    \
    }

// static_counter
VARIABLE_READER(static_counter, count)
VARIABLE_READER(static_counter, ready)
static const name_value_t counter_events[] = {
    {"tick", STATIC_COUNTER_EVENT_TICK},
    {"go", STATIC_COUNTER_EVENT_GO},
};
static const name_value_t counter_states[] = {
    {"counting", STATIC_COUNTER_STATE_COUNTING},
    {"done", STATIC_COUNTER_STATE_DONE},
};
static const variable_t counter_variables[] = {
    {"count", static_counter_read_count},
    {"ready", static_counter_read_ready},
};
STATIC_SCENARIO(static_counter, counter_events, counter_states, counter_variables)

// static_overflow
VARIABLE_READER(static_overflow, level)
VARIABLE_READER(static_overflow, refusals)
static const name_value_t overflow_events[] = {
    {"up", STATIC_OVERFLOW_EVENT_UP},
    {"probe", STATIC_OVERFLOW_EVENT_PROBE},
};
static const name_value_t overflow_states[] = {
    {"waiting", STATIC_OVERFLOW_STATE_WAITING},
    {"probed", STATIC_OVERFLOW_STATE_PROBED},
};
static const variable_t overflow_variables[] = {
    {"level", static_overflow_read_level},
    {"refusals", static_overflow_read_refusals},
};
STATIC_SCENARIO(static_overflow, overflow_events, overflow_states, overflow_variables)

// static_block_ends
VARIABLE_READER(static_block_ends, a)
VARIABLE_READER(static_block_ends, b)
VARIABLE_READER(static_block_ends, afterAssign)
VARIABLE_READER(static_block_ends, thenRan)
VARIABLE_READER(static_block_ends, elseRan)
VARIABLE_READER(static_block_ends, afterIf)
VARIABLE_READER(static_block_ends, inBranch)
VARIABLE_READER(static_block_ends, afterBranch)
VARIABLE_READER(static_block_ends, afterOk)
VARIABLE_READER(static_block_ends, errors)
static const name_value_t block_ends_events[] = {
    {"fail.assign", STATIC_BLOCK_ENDS_EVENT_FAIL_ASSIGN},
    {"fail.cond", STATIC_BLOCK_ENDS_EVENT_FAIL_COND},
    {"fail.branch", STATIC_BLOCK_ENDS_EVENT_FAIL_BRANCH},
    {"ok", STATIC_BLOCK_ENDS_EVENT_OK},
};
static const name_value_t block_ends_states[] = {
    {"waiting", STATIC_BLOCK_ENDS_STATE_WAITING},
};
static const variable_t block_ends_variables[] = {
    {"a", static_block_ends_read_a},
    {"b", static_block_ends_read_b},
    {"afterAssign", static_block_ends_read_afterAssign},
    {"thenRan", static_block_ends_read_thenRan},
    {"elseRan", static_block_ends_read_elseRan},
    {"afterIf", static_block_ends_read_afterIf},
    {"inBranch", static_block_ends_read_inBranch},
    {"afterBranch", static_block_ends_read_afterBranch},
    {"afterOk", static_block_ends_read_afterOk},
    {"errors", static_block_ends_read_errors},
};
STATIC_SCENARIO(static_block_ends, block_ends_events, block_ends_states, block_ends_variables)

// static_payload
VARIABLE_READER(static_payload, day)
VARIABLE_READER(static_payload, late)
VARIABLE_READER(static_payload, sinceEpoch)
VARIABLE_READER(static_payload, refusals)
static const name_value_t payload_events[] = {
    {"day.picked", STATIC_PAYLOAD_EVENT_DAY_PICKED},
};
static const name_value_t payload_states[] = {
    {"waiting", STATIC_PAYLOAD_STATE_WAITING},
};
static const variable_t payload_variables[] = {
    {"day", static_payload_read_day},
    {"late", static_payload_read_late},
    {"sinceEpoch", static_payload_read_sinceEpoch},
    {"refusals", static_payload_read_refusals},
};
STATIC_SCENARIO(static_payload, payload_events, payload_states, payload_variables)

// What no scenario can state, because a scenario's event carries its data or is
// a different event: a delivery that carried no payload. Content that reads one
// runs for a payload and for nothing else — against the zeroed buffer of a
// delivery with none it would assign `day = 0` and say the thirtieth never
// happened.
static int content_that_reads_a_payload_does_not_run_for_a_delivery_without_one(void) {
    static_payload_t sm;
    static_payload_init(&sm);
    int bad = 0;
    int64_t day = 0;
    if (!static_payload_raise_event(&sm, "day.picked", "{\"year\":2026,\"month\":9,\"dayOfMonth\":30}") ||
        !static_payload_raise_event(&sm, "day.picked", NULL)) {
        (void)fprintf(stderr, "static_payload: FAIL - the machine has no day.picked\n");
        return 1;
    }
    if (!static_payload_read_variable(&sm, "day", &day) || day != 30) {
        (void)fprintf(
            stderr,
            "static_payload: FAIL - a delivery with no payload ran content that reads one: day is %lld, want 30\n",
            (long long)day);
        bad = 1;
    }
    return bad;
}

int main(void) {
    int bad = 0;
    // Each scenario file by its name, and the steps it has at the least.
    bad |= static_counter_scenario("static_counter", 7);
    bad |= static_counter_scenario("static_counter_bound", 12);
    bad |= static_overflow_scenario("static_overflow", 5);
    bad |= static_block_ends_scenario("static_block_ends", 5);
    bad |= static_payload_scenario("static_payload", 5);
    bad |= content_that_reads_a_payload_does_not_run_for_a_delivery_without_one();
    if (bad != 0) {
        return 1;
    }
    (void)printf("static scalars: ok\n");
    return 0;
}
