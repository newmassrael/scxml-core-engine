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
//   * `static_event_arrival`: an event that arrives by name is delivered as the
//     event the machine resolves the name to (§scxml-3.12.1) — the document's
//     own name for it, or the longest of the document's names that is a token
//     prefix of it — and a name no event matches is dropped.
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
//   * `sync_client`: a call of an imported algorithm — a header of `static inline`
//     functions the machine includes — in a guard and in an assignment, over the
//     payload of each answer, and a call that refuses its arguments is a failure
//     like any other.
//   * `static_list`: a list variable is bounded by its `sce:capacity` — an append
//     past the bound appends nothing and raises `error.execution` — emptied by
//     `<sce:clear>`, measured by `len()`, and published as the library's view of
//     its elements.
//   * `static_foreach`: a loop walks a copy of the list as it began, so a body
//     that appends to the list it walks neither lengthens the walk nor reads what
//     it has just written; the item, its position and a loop in a loop are native.
//   * `static_block_ends_list`: an append that fails ends the block it stands in,
//     like any other failed statement.
//   * `static_record_fields`, `static_record`: a record variable is a struct of
//     its schema's fields, built whole from its `<sce:set>`s, read field by field
//     in a guard and an assignment, and updated a field at a time; an assignment
//     that fails leaves the record as it was.
//   * `static_record_list`: a list of records copies the record it is appended
//     to, a loop reads its record item typed and appends it whole to another
//     list, and a whole record is assigned from a record by name.
//   * `static_record_enum`: a record's enum field is held in the machine's type for
//     the enum, compared, assigned a variant and appended with the record, and
//     observed as the name its document declares.
//   * `static_enum`: an enum variable starts at a variant, is compared with `===`
//     and `!==`, takes a conditional of two variants, and is observed as the name
//     its document declares, not as the constant C spells for it.
//
// Linked WITHOUT `sce_c_scripting` and `lua54` on purpose: these machines carry
// no script engine, so the link is the proof. If the emit ever reached for one,
// this target would stop linking, which is a better gate than a comment.

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "static_block_ends_list_sm.h"
#include "static_block_ends_sm.h"
#include "static_counter_sm.h"
#include "static_enum_sm.h"
#include "static_event_arrival_sm.h"
#include "static_foreach_sm.h"
#include "static_list_sm.h"
#include "static_overflow_sm.h"
#include "static_payload_sm.h"
#include "static_record_enum_sm.h"
#include "static_record_fields_sm.h"
#include "static_record_list_sm.h"
#include "static_record_sm.h"
#include "sync_client_sm.h"

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

// A published list's reader: its elements as the int64s a scenario compares, at
// most `cap` of them, and how many it holds. A table of these ends at a NULL name.
typedef struct {
    const char *name;
    size_t (*read)(const void *sm, int64_t *out, size_t cap);
} list_variable_t;

// What a machine with no list passes for its table.
static const list_variable_t no_lists[] = {{NULL, NULL}};

// A published record's reader, or a published list of records': the number a
// field of it holds, or the name an enum field holds — in the record, or in
// element `index` of the list — and, for a list, how many elements it holds. A
// table of these ends at a NULL name.
typedef struct {
    const char *name;
    bool (*field)(const void *sm, size_t index, const char *field, int64_t *out);
    const char *(*text)(const void *sm, size_t index, const char *field);
    size_t (*count)(const void *sm);
} record_variable_t;

// What a machine with no record passes for its table.
static const record_variable_t no_records[] = {{NULL, NULL, NULL, NULL}};

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

// A published list's reader, over the library's borrowed view of its elements.
#define LIST_READER(M, VAR, VIEW)                                                                                      \
    static size_t M##_read_list_##VAR(const void *sm, int64_t *out, size_t cap) {                                      \
        const VIEW view = M##_get_##VAR((const M##_t *)sm);                                                            \
        for (size_t i = 0; i < view.len && i < cap; ++i) {                                                             \
            out[i] = (int64_t)view.data[i];                                                                            \
        }                                                                                                              \
        return view.len;                                                                                               \
    }

// A record's fields as a reader tells them by name — which is how a field is told
// from a typo. FIELDS names every field a scenario can state, as `N(field)` for a
// number or a bool and `E(field, declared_name)` for an enum, `declared_name` being
// the function its document declares for the name of a value.
#define RECORD_NUMBER(FIELD)                                                                                           \
    if (strcmp(field, #FIELD) == 0) {                                                                                  \
        *out = (int64_t)rec->FIELD;                                                                                    \
        return true;                                                                                                   \
    }
#define RECORD_NOT_NUMBER(FIELD, NAME)
#define RECORD_NAME(FIELD, NAME)                                                                                       \
    if (strcmp(field, #FIELD) == 0) {                                                                                  \
        return NAME(rec->FIELD);                                                                                       \
    }
#define RECORD_NOT_NAME(FIELD)

// A record variable's readers, over the record the machine publishes by value.
#define RECORD_READER(M, VAR, RECORD, FIELDS)                                                                          \
    static bool M##_read_record_##VAR(const void *sm, size_t index, const char *field, int64_t *out) {                 \
        const RECORD value = M##_get_##VAR((const M##_t *)sm);                                                         \
        const RECORD *rec = &value;                                                                                    \
        (void)index;                                                                                                   \
        FIELDS(RECORD_NUMBER, RECORD_NOT_NUMBER)                                                                       \
        return false;                                                                                                  \
    }                                                                                                                  \
    static const char *M##_text_record_##VAR(const void *sm, size_t index, const char *field) {                        \
        const RECORD value = M##_get_##VAR((const M##_t *)sm);                                                         \
        const RECORD *rec = &value;                                                                                    \
        (void)index;                                                                                                   \
        (void)rec;                                                                                                     \
        (void)field;                                                                                                   \
        FIELDS(RECORD_NOT_NAME, RECORD_NAME)                                                                           \
        return NULL;                                                                                                   \
    }

// A published list of records' readers, over the borrowed view of its elements.
#define RECORD_LIST_READER(M, VAR, VIEW, RECORD, FIELDS)                                                               \
    static bool M##_read_record_list_##VAR(const void *sm, size_t index, const char *field, int64_t *out) {            \
        const VIEW view = M##_get_##VAR((const M##_t *)sm);                                                            \
        if (index >= view.len) {                                                                                       \
            return false;                                                                                              \
        }                                                                                                              \
        const RECORD *rec = &view.data[index];                                                                         \
        FIELDS(RECORD_NUMBER, RECORD_NOT_NUMBER)                                                                       \
        return false;                                                                                                  \
    }                                                                                                                  \
    static const char *M##_text_record_list_##VAR(const void *sm, size_t index, const char *field) {                   \
        const VIEW view = M##_get_##VAR((const M##_t *)sm);                                                            \
        if (index >= view.len) {                                                                                       \
            return NULL;                                                                                               \
        }                                                                                                              \
        const RECORD *rec = &view.data[index];                                                                         \
        (void)rec;                                                                                                     \
        (void)field;                                                                                                   \
        FIELDS(RECORD_NOT_NAME, RECORD_NAME)                                                                           \
        return NULL;                                                                                                   \
    }                                                                                                                  \
    static size_t M##_count_record_list_##VAR(const void *sm) {                                                        \
        const VIEW view = M##_get_##VAR((const M##_t *)sm);                                                            \
        return view.len;                                                                                               \
    }

// A table row for each.
#define RECORD_ROW(M, VAR) {#VAR, M##_read_record_##VAR, M##_text_record_##VAR, NULL}
#define RECORD_LIST_ROW(M, VAR)                                                                                        \
    {#VAR, M##_read_record_list_##VAR, M##_text_record_list_##VAR, M##_count_record_list_##VAR}

// One machine as `sce_scenario_driver_t` asks for it: the event by the name the
// machine itself resolves (§scxml-3.12.1), the state by its document name, the
// variable by its, the lists and records by theirs, and the replay of its
// scenario.
#define STATIC_SCENARIO(M, STATES, VARIABLES, TEXT, LISTS, RECORDS)                                                    \
    static bool M##_read_record_field(void *sm, const char *name, size_t index, const char *field, int64_t *out) {     \
        for (size_t i = 0; RECORDS[i].name != NULL; ++i) {                                                             \
            if (strcmp(RECORDS[i].name, name) == 0) {                                                                  \
                return RECORDS[i].field(sm, index, field, out);                                                        \
            }                                                                                                          \
        }                                                                                                              \
        return false;                                                                                                  \
    }                                                                                                                  \
    static const char *M##_read_record_text(void *sm, const char *name, size_t index, const char *field) {             \
        for (size_t i = 0; RECORDS[i].name != NULL; ++i) {                                                             \
            if (strcmp(RECORDS[i].name, name) == 0) {                                                                  \
                return RECORDS[i].text(sm, index, field);                                                              \
            }                                                                                                          \
        }                                                                                                              \
        return NULL;                                                                                                   \
    }                                                                                                                  \
    static bool M##_read_record_count(void *sm, const char *name, size_t *len) {                                       \
        for (size_t i = 0; RECORDS[i].name != NULL; ++i) {                                                             \
            if (strcmp(RECORDS[i].name, name) == 0 && RECORDS[i].count != NULL) {                                      \
                *len = RECORDS[i].count(sm);                                                                           \
                return true;                                                                                           \
            }                                                                                                          \
        }                                                                                                              \
        return false;                                                                                                  \
    }                                                                                                                  \
    static bool M##_read_lists(void *sm, const char *name, int64_t *out, size_t cap, size_t *len) {                    \
        for (size_t i = 0; LISTS[i].name != NULL; ++i) {                                                               \
            if (strcmp(LISTS[i].name, name) == 0) {                                                                    \
                *len = LISTS[i].read(sm, out, cap);                                                                    \
                return *len <= cap;                                                                                    \
            }                                                                                                          \
        }                                                                                                              \
        return false;                                                                                                  \
    }                                                                                                                  \
    static bool M##_raise_event(void *sm, const char *name, const char *data) {                                        \
        M##_event_t event;                                                                                             \
        if (!M##_resolve_event_by_name(name, &event)) {                                                                \
            return false;                                                                                              \
        }                                                                                                              \
        M##_event_with_meta_t meta;                                                                                    \
        memset(&meta, 0, sizeof(meta));                                                                                \
        meta.event = event;                                                                                            \
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
        const sce_scenario_driver_t driver = {#M,                                                                      \
                                              &sm,                                                                     \
                                              M##_raise_event,                                                         \
                                              M##_state_active,                                                        \
                                              M##_run_ended,                                                           \
                                              M##_read_variable,                                                       \
                                              TEXT,                                                                    \
                                              M##_read_lists,                                                          \
                                              M##_read_record_field,                                                   \
                                              M##_read_record_text,                                                    \
                                              M##_read_record_count};                                                  \
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
static const name_value_t counter_states[] = {
    {"counting", STATIC_COUNTER_STATE_COUNTING},
    {"done", STATIC_COUNTER_STATE_DONE},
};
static const variable_t counter_variables[] = {
    {"count", static_counter_read_count},
    {"ready", static_counter_read_ready},
};
STATIC_SCENARIO(static_counter, counter_states, counter_variables, NULL, no_lists, no_records)

// static_event_arrival: an event arrives by name from outside the document
// (§scxml-3.12.1), so the machine delivers it as the event it resolves the name
// to, and drops a name it resolves to none.
VARIABLE_READER(static_event_arrival, requests)
VARIABLE_READER(static_event_arrival, specials)
static const name_value_t event_arrival_states[] = {
    {"listening", STATIC_EVENT_ARRIVAL_STATE_LISTENING},
};
static const variable_t event_arrival_variables[] = {
    {"requests", static_event_arrival_read_requests},
    {"specials", static_event_arrival_read_specials},
};
STATIC_SCENARIO(static_event_arrival, event_arrival_states, event_arrival_variables, NULL, no_lists, no_records)

// static_overflow
VARIABLE_READER(static_overflow, level)
VARIABLE_READER(static_overflow, refusals)
static const name_value_t overflow_states[] = {
    {"waiting", STATIC_OVERFLOW_STATE_WAITING},
    {"probed", STATIC_OVERFLOW_STATE_PROBED},
};
static const variable_t overflow_variables[] = {
    {"level", static_overflow_read_level},
    {"refusals", static_overflow_read_refusals},
};
STATIC_SCENARIO(static_overflow, overflow_states, overflow_variables, NULL, no_lists, no_records)

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
STATIC_SCENARIO(static_block_ends, block_ends_states, block_ends_variables, NULL, no_lists, no_records)

// static_payload
VARIABLE_READER(static_payload, day)
VARIABLE_READER(static_payload, late)
VARIABLE_READER(static_payload, sinceEpoch)
VARIABLE_READER(static_payload, refusals)
static const name_value_t payload_states[] = {
    {"waiting", STATIC_PAYLOAD_STATE_WAITING},
};
static const variable_t payload_variables[] = {
    {"day", static_payload_read_day},
    {"late", static_payload_read_late},
    {"sinceEpoch", static_payload_read_sinceEpoch},
    {"refusals", static_payload_read_refusals},
};
STATIC_SCENARIO(static_payload, payload_states, payload_variables, NULL, no_lists, no_records)

// static_enum: the layout a calendar screen shows its days in. `layout` is
// published; `previous` is the machine's own, so it is read from the policy the
// struct holds, as the C++ suite reads it from the policy member. A value is
// stated as the name its document declares.
VARIABLE_READER(static_enum, changes)

static int64_t static_enum_read_layout(const void *sm) {
    return (int64_t)static_enum_get_layout((const static_enum_t *)sm);
}

static int64_t static_enum_read_previous(const void *sm) {
    return (int64_t)((const static_enum_t *)sm)->policy.v_previous;
}

static const char *static_enum_text(void *sm, const char *name) {
    const static_enum_t *machine = (const static_enum_t *)sm;
    // The function is named for the enum document's own name, which the
    // generator reads from its file: `enum_view_mode.scxml`.
    if (strcmp(name, "layout") == 0) {
        return enum_view_mode_declared_name(static_enum_get_layout(machine));
    }
    if (strcmp(name, "previous") == 0) {
        return enum_view_mode_declared_name(machine->policy.v_previous);
    }
    return NULL;
}

static const name_value_t enum_states[] = {
    {"browsing", STATIC_ENUM_STATE_BROWSING},
};
static const variable_t enum_variables[] = {
    {"changes", static_enum_read_changes},
    {"layout", static_enum_read_layout},
    {"previous", static_enum_read_previous},
};
STATIC_SCENARIO(static_enum, enum_states, enum_variables, static_enum_text, no_lists, no_records)

// sync_client: one collection's sync run, which calls the standard sync rules —
// algorithms the machine includes — over the payload of each answer the host
// reports. `retryAt` is an int64, and a rule it calls refuses a status outside
// its domain, which the document counts in `refusals`.
VARIABLE_READER(sync_client, byToken)
VARIABLE_READER(sync_client, fullListing)
VARIABLE_READER(sync_client, outcome)
VARIABLE_READER(sync_client, retryAt)
VARIABLE_READER(sync_client, deleted)
VARIABLE_READER(sync_client, uploaded)
VARIABLE_READER(sync_client, discarded)
VARIABLE_READER(sync_client, pages)
VARIABLE_READER(sync_client, refusals)
static const name_value_t sync_states[] = {
    {"idle", SYNC_CLIENT_STATE_IDLE},
    {"deleting", SYNC_CLIENT_STATE_DELETING},
    {"uploading", SYNC_CLIENT_STATE_UPLOADING},
    {"listing", SYNC_CLIENT_STATE_LISTING},
};
static const variable_t sync_variables[] = {
    {"byToken", sync_client_read_byToken},     {"fullListing", sync_client_read_fullListing},
    {"outcome", sync_client_read_outcome},     {"retryAt", sync_client_read_retryAt},
    {"deleted", sync_client_read_deleted},     {"uploaded", sync_client_read_uploaded},
    {"discarded", sync_client_read_discarded}, {"pages", sync_client_read_pages},
    {"refusals", sync_client_read_refusals},
};
STATIC_SCENARIO(sync_client, sync_states, sync_variables, NULL, no_lists, no_records)

// static_list: a list filled to its bound by <sce:append> from a typed payload,
// measured by len(), emptied by <sce:clear>. A published list is read through the
// library's view of its elements.
VARIABLE_READER(static_list, refusals)
VARIABLE_READER(static_list, count)
LIST_READER(static_list, picked, sce_forge_uint8_view_t)
static const name_value_t list_states[] = {
    {"collecting", STATIC_LIST_STATE_COLLECTING},
    {"done", STATIC_LIST_STATE_DONE},
};
static const variable_t list_variables[] = {
    {"refusals", static_list_read_refusals},
    {"count", static_list_read_count},
};
static const list_variable_t list_lists[] = {{"picked", static_list_read_list_picked}, {NULL, NULL}};
STATIC_SCENARIO(static_list, list_states, list_variables, NULL, list_lists, no_records)

// static_foreach: a loop over a copy of the list as it began — the item alone, the
// item and its position, a loop in a loop, and a body that appends to the list it
// walks.
VARIABLE_READER(static_foreach, total)
VARIABLE_READER(static_foreach, weighted)
VARIABLE_READER(static_foreach, small)
VARIABLE_READER(static_foreach, crossings)
VARIABLE_READER(static_foreach, visited)
VARIABLE_READER(static_foreach, finished)
VARIABLE_READER(static_foreach, errors)
LIST_READER(static_foreach, picked, sce_forge_uint8_view_t)
static const name_value_t foreach_states[] = {
    {"working", STATIC_FOREACH_STATE_WORKING},
};
static const variable_t foreach_variables[] = {
    {"total", static_foreach_read_total},     {"weighted", static_foreach_read_weighted},
    {"small", static_foreach_read_small},     {"crossings", static_foreach_read_crossings},
    {"visited", static_foreach_read_visited}, {"finished", static_foreach_read_finished},
    {"errors", static_foreach_read_errors},
};
static const list_variable_t foreach_lists[] = {{"picked", static_foreach_read_list_picked}, {NULL, NULL}};
STATIC_SCENARIO(static_foreach, foreach_states, foreach_variables, NULL, foreach_lists, no_records)

// static_block_ends_list: a full list ends the block it is appended to.
VARIABLE_READER(static_block_ends_list, afterAppend)
VARIABLE_READER(static_block_ends_list, errors)
LIST_READER(static_block_ends_list, picked, sce_forge_uint8_view_t)
static const name_value_t block_ends_list_states[] = {
    {"waiting", STATIC_BLOCK_ENDS_LIST_STATE_WAITING},
};
static const variable_t block_ends_list_variables[] = {
    {"afterAppend", static_block_ends_list_read_afterAppend},
    {"errors", static_block_ends_list_read_errors},
};
static const list_variable_t block_ends_list_lists[] = {{"picked", static_block_ends_list_read_list_picked},
                                                        {NULL, NULL}};
STATIC_SCENARIO(static_block_ends_list, block_ends_list_states, block_ends_list_variables, NULL, block_ends_list_lists,
                no_records)

// static_record_fields: a record variable built whole from its <sce:set>s, read
// field by field in a guard and an assignment, and updated a field at a time —
// from its own value and from a typed event payload. It is published by value,
// as the struct the machine's header declares.
#define DAY_FIELDS(N, E) N(year) N(month) N(dayOfMonth)
VARIABLE_READER(static_record_fields, refusals)
RECORD_READER(static_record_fields, shown, static_record_fields_record_day_t, DAY_FIELDS)
static const name_value_t record_fields_states[] = {
    {"showing", STATIC_RECORD_FIELDS_STATE_SHOWING},
};
static const variable_t record_fields_variables[] = {
    {"refusals", static_record_fields_read_refusals},
};
static const record_variable_t record_fields_records[] = {RECORD_ROW(static_record_fields, shown),
                                                          {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_record_fields, record_fields_states, record_fields_variables, NULL, no_lists,
                record_fields_records)

// static_record: the same record, with a guard that calls an imported algorithm
// over two of its fields.
VARIABLE_READER(static_record, refusals)
RECORD_READER(static_record, shown, static_record_record_day_t, DAY_FIELDS)
static const name_value_t record_states[] = {
    {"showing", STATIC_RECORD_STATE_SHOWING},
};
static const variable_t record_variables[] = {
    {"refusals", static_record_read_refusals},
};
static const record_variable_t record_records[] = {RECORD_ROW(static_record, shown), {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_record, record_states, record_variables, NULL, no_lists, record_records)

// static_record_list: lists of records — each append takes a copy of the record
// as it stands, a loop reads its record item typed and appends it whole to
// another list, and a whole record is assigned from a record by name.
VARIABLE_READER(static_record_list, total)
VARIABLE_READER(static_record_list, errors)
RECORD_READER(static_record_list, draft, static_record_list_record_day_t, DAY_FIELDS)
RECORD_READER(static_record_list, last, static_record_list_record_day_t, DAY_FIELDS)
RECORD_LIST_READER(static_record_list, days, static_record_list_record_day_view_t, static_record_list_record_day_t,
                   DAY_FIELDS)
RECORD_LIST_READER(static_record_list, copies, static_record_list_record_day_view_t, static_record_list_record_day_t,
                   DAY_FIELDS)
static const name_value_t record_list_states[] = {
    {"collecting", STATIC_RECORD_LIST_STATE_COLLECTING},
};
static const variable_t record_list_variables[] = {
    {"total", static_record_list_read_total},
    {"errors", static_record_list_read_errors},
};
static const record_variable_t record_list_records[] = {RECORD_ROW(static_record_list, draft),
                                                        RECORD_ROW(static_record_list, last),
                                                        RECORD_LIST_ROW(static_record_list, days),
                                                        RECORD_LIST_ROW(static_record_list, copies),
                                                        {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_record_list, record_list_states, record_list_variables, NULL, no_lists, record_list_records)

// static_record_enum: a record with an enum field is held in the machine's own
// type for the enum — compared with `===`, assigned a variant, appended whole and
// read through a loop's record item — and its field is observed as the name its
// document declares, not the constant C spells for it.
#define VIEW_FIELDS(N, E) E(layout, enum_view_mode_declared_name) N(zoom)
VARIABLE_READER(static_record_enum, weeks)
VARIABLE_READER(static_record_enum, flips)
RECORD_READER(static_record_enum, shown, static_record_enum_record_view_t, VIEW_FIELDS)
RECORD_LIST_READER(static_record_enum, seen, static_record_enum_record_view_view_t, static_record_enum_record_view_t,
                   VIEW_FIELDS)
static const name_value_t record_enum_states[] = {
    {"viewing", STATIC_RECORD_ENUM_STATE_VIEWING},
};
static const variable_t record_enum_variables[] = {
    {"weeks", static_record_enum_read_weeks},
    {"flips", static_record_enum_read_flips},
};
static const record_variable_t record_enum_records[] = {
    RECORD_ROW(static_record_enum, shown), RECORD_LIST_ROW(static_record_enum, seen), {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_record_enum, record_enum_states, record_enum_variables, NULL, no_lists, record_enum_records)

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
    bad |= static_event_arrival_scenario("static_event_arrival", 8);
    bad |= static_overflow_scenario("static_overflow", 5);
    bad |= static_block_ends_scenario("static_block_ends", 5);
    bad |= static_list_scenario("static_list", 11);
    bad |= static_foreach_scenario("static_foreach", 13);
    bad |= static_block_ends_list_scenario("static_block_ends_list", 4);
    bad |= static_record_fields_scenario("static_record_fields", 9);
    bad |= static_record_scenario("static_record", 16);
    bad |= static_record_list_scenario("static_record_list", 14);
    bad |= static_record_enum_scenario("static_record_enum", 13);
    bad |= static_payload_scenario("static_payload", 5);
    bad |= static_enum_scenario("static_enum", 11);
    bad |= sync_client_scenario("sync_client", 30);
    bad |= content_that_reads_a_payload_does_not_run_for_a_delivery_without_one();
    if (bad != 0) {
        return 1;
    }
    (void)printf("static scalars: ok\n");
    return 0;
}
