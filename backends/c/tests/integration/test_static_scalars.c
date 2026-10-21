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
//   * `static_event_wildcard`: where the document listens with `event="*"`, a
//     name no event matches is delivered as the wildcard event, not dropped.
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
//   * `static_payload_enum`: a payload's enum field is held in the machine's own
//     enum, lifted from the variant's declared name the data carries, compared and
//     assigned as a variable of the enum is; a name the enum does not declare, or
//     a value that is no text, is a payload that does not fit, and writes nothing.
//   * `static_payload_relay`: the payload a transition's event carried is read
//     where a `<send>` runs and carried on as its `<param>`s — an enum field as
//     the name its enum declares, an integer as its number — and a value that
//     does not fit its type is left out with an `error.execution`.
//   * `sync_client`: a call of an imported algorithm — a header of `static inline`
//     functions the machine includes — in a guard and in an assignment, over the
//     payload of each answer, and a call that refuses its arguments is a failure
//     like any other.
//   * `static_list`: a list variable is bounded by its `sce:capacity` — an append
//     past the bound appends nothing and raises `error.execution` — emptied by
//     `<sce:clear>`, measured by `len()`, and published as the library's view of
//     its elements.
//   * `static_list_index`: an element is read by its index in an assignment, an
//     operand and a guard; an index below zero or not below the length reads
//     nothing and raises `error.execution`, as a checked integer operation does.
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
//   * `static_donedata`: a top-level final writes the pairs of its `<donedata>`, each
//     read from the machine's own fields, as the JSON object of the done event's
//     data; a pair whose value failed to compute is left out and the others cross.
//   * `static_donedata_content`: a top-level final whose `<donedata>` is inline
//     `<content>` writes the text as the JSON string it spells (`42` is the string
//     "42"), with no script engine to read it as a number.
//   * `static_string_capacity`: a string is a buffer of the UTF-8 bytes its variable
//     declares, and an assignment past it is refused by bytes, not characters.
//   * `static_bytes`: a byte string is a buffer of the bytes its variable declares and
//     the length it holds, assigned from a literal or another byte string, compared,
//     measured, and refused past its bound; a host is lent a view of it.
//   * `static_record_bytes`: a record's byte-string field is the same buffer, bounded
//     by the schema's `sce:max-size`, written a field at a time, copied into a list of
//     records, compared and measured; a host reads the record by value.
//   * `static_enum`: an enum variable starts at a variant, is compared with `===`
//     and `!==`, takes a conditional of two variants, and is observed as the name
//     its document declares, not as the constant C spells for it.
//   * `static_enum_number`: a value of an enum is read as the number its variant
//     declares and a number back as its variant, and a number no variant declares
//     is error.execution (docs/adr/0047).
//   * `static_forge_calls`: a condition, a transform, two lookups, an interpolation
//     and a validator called as `Alias(args)`, and a lookup whose miss is an error
//     failing into error.execution (docs/adr/0048).
//   * `presence_peer`: a machine whose entries expire by its own delayed send, so
//     it runs on a manual clock the scenario moves on, and sends its Mesh peers
//     through the same router.
//   * `todo_peer`, `sheet_peer`: a machine made of the standard merge rules
//     (sce:std/merge) holds, after every event of a run it took part in, the cells,
//     texts, rows and clock the scenario states — a 64-bit field as the text of its
//     digits — and sent its Mesh peers the sends the scenario states, in their
//     order, through the router a host installs before the machine starts.
//   * `record_list`: the standard shared record list (sce:std/shared/record_list), a
//     scenario's machine named by its library name, as a person's copy and as the
//     hub, started by a host, when it tells no parent anything, and started with no
//     Mesh router, when it keeps every change it makes; and
//     `record_list_parent`, which
//     invokes it by its library name and publishes what it was told of each
//     register, and of each change the list refused, its reason by the name the
//     standard enum record_refusal declares.
//
// Linked WITHOUT `sce_c_scripting` and `lua54` on purpose: these machines carry
// no script engine, so the link is the proof. If the emit ever reached for one,
// this target would stop linking, which is a better gate than a comment.

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "presence_peer_sm.h"
#include "record_list_parent_sm.h"
#include "record_list_sm.h"
#include "sheet_peer_sm.h"
#include "static_block_ends_list_sm.h"
#include "static_block_ends_sm.h"
#include "static_bytes_sm.h"
#include "static_bytes_wire_sm.h"
#include "static_cancel_expr_sm.h"
#include "static_child_host_hybrid_sm.h"
#include "static_child_host_sm.h"
#include "static_child_mesh_sm.h"
#include "static_counter_sm.h"
#include "static_donedata_content_sm.h"
#include "static_donedata_record_sm.h"
#include "static_donedata_sm.h"
#include "static_enum_number_sm.h"
#include "static_enum_sm.h"
#include "static_event_arrival_sm.h"
#include "static_event_wildcard_sm.h"
#include "static_foreach_sm.h"
#include "static_forge_calls_sm.h"
#include "static_history_sm.h"
#include "static_host_call_sm.h"
#include "static_host_duty_sm.h"
#include "static_invoke_params_sm.h"
#include "static_invoke_sm.h"
#include "static_invoke_string_sm.h"
#include "static_list_assign_sm.h"
#include "static_list_index_sm.h"
#include "static_list_sm.h"
#include "static_overflow_sm.h"
#include "static_payload_bytes_sm.h"
#include "static_payload_enum_sm.h"
#include "static_payload_relay_sm.h"
#include "static_payload_sm.h"
#include "static_real32_sm.h"
#include "static_real_sm.h"
#include "static_record_bytes_sm.h"
#include "static_record_enum_sm.h"
#include "static_record_fields_sm.h"
#include "static_record_list_index_sm.h"
#include "static_record_list_sm.h"
#include "static_record_real32_sm.h"
#include "static_record_real_sm.h"
#include "static_record_sm.h"
#include "static_record_string_sm.h"
#include "static_schema_revision_sm.h"
#include "static_send_content_sm.h"
#include "static_send_delay_sm.h"
#include "static_send_event_sm.h"
#include "static_send_idlocation_sm.h"
#include "static_send_namelist_sm.h"
#include "static_send_params_sm.h"
#include "static_send_target_sm.h"
#include "static_send_type_sm.h"
#include "static_state_algorithm_enum_sm.h"
#include "static_state_algorithm_sm.h"
#include "static_string_capacity_sm.h"
#include "static_text_record_sm.h"
#include "static_timers_sm.h"
#include "static_whole_payload_sm.h"
#include "static_wire_enum_sm.h"
#include "static_wire_wide_sm.h"
#include "sync_client_sm.h"
#include "todo_peer_sm.h"

#include <sce/forge/wire.h>

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
// the function its document declares for the name of a value. A field a scenario
// states as any other string is an `E` too, with the function that answers its
// text: a string's, or the digits of a 64-bit integer. A record may have no field
// of the one kind, which is what the reader's unused names are voided for.
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
        (void)rec;                                                                                                     \
        (void)field;                                                                                                   \
        (void)out;                                                                                                     \
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
        (void)rec;                                                                                                     \
        (void)field;                                                                                                   \
        (void)out;                                                                                                     \
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
// scenario. DONE answers the data its final's `<donedata>` wrote, for a machine
// that has one. INIT starts the machine in `sm`, and ADVANCE moves the clock of a
// machine that waits on one, or is NULL for a machine that does not. HOST_CALLS
// answers what the machine asked of its host, SENT what it sent its Mesh peers
// and CHILD_HOSTS the child hosts its host was asked for and what the children
// asked of them, each since it was last asked, or is NULL for a machine with none.
// DELIVER hands a step's `invoke` event to the running child of that path through
// the Mesh router the driver gave it, or is NULL for a machine whose children are
// given none. No machine here holds a value that may be absent, so none is asked
// whether one is present.
#define STATIC_SCENARIO_CORE(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, DONE, REAL, REAL_LIST, RECORD_REAL, INIT,     \
                             ADVANCE, HOST_CALLS, SENT, CHILD_HOSTS, DELIVER)                                          \
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
    static size_t M##_active_count(void *sm) {                                                                         \
        /* The bitmap of the active configuration holds a bit for each active state. */                                \
        uint32_t bits = M##_active_states((const M##_t *)sm);                                                          \
        size_t count = 0;                                                                                              \
        for (; bits != 0u; bits >>= 1u) {                                                                              \
            count += bits & 1u;                                                                                        \
        }                                                                                                              \
        return count;                                                                                                  \
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
    static int M##_replay(M##_t *started, const char *(*sent)(void *), const char *scenario, int min_steps) {          \
        const sce_scenario_driver_t driver = {#M,                                                                      \
                                              started,                                                                 \
                                              M##_raise_event,                                                         \
                                              M##_state_active,                                                        \
                                              M##_run_ended,                                                           \
                                              M##_read_variable,                                                       \
                                              TEXT,                                                                    \
                                              M##_read_lists,                                                          \
                                              M##_read_record_field,                                                   \
                                              M##_read_record_text,                                                    \
                                              M##_read_record_count,                                                   \
                                              DONE,                                                                    \
                                              REAL,                                                                    \
                                              REAL_LIST,                                                               \
                                              RECORD_REAL,                                                             \
                                              ADVANCE,                                                                 \
                                              M##_active_count,                                                        \
                                              HOST_CALLS,                                                              \
                                              sent,                                                                    \
                                              CHILD_HOSTS,                                                             \
                                              DELIVER,                                                                 \
                                              NULL};                                                                   \
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
    }                                                                                                                  \
    static int M##_scenario(const char *scenario, int min_steps) {                                                     \
        M##_t sm;                                                                                                      \
        INIT;                                                                                                          \
        return M##_replay(&sm, SENT, scenario, min_steps);                                                             \
    }

// A machine that waits on no clock, started as `_init` starts it.
#define STATIC_SCENARIO_FULL(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, DONE, REAL, REAL_LIST, RECORD_REAL)           \
    STATIC_SCENARIO_CORE(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, DONE, REAL, REAL_LIST, RECORD_REAL,               \
                         M##_init(&sm), NULL, NULL, NULL, NULL, NULL)

// A machine whose `<sce:action>`s go to a host that records them: INIT starts it
// with that host installed, which it must be before the first entry runs, and
// HOST_CALLS answers what the host heard since it was last asked.
#define STATIC_SCENARIO_HOSTED(M, STATES, VARIABLES, INIT, HOST_CALLS)                                                 \
    STATIC_SCENARIO_CORE(M, STATES, VARIABLES, NULL, no_lists, no_records, NULL, NULL, NULL, NULL, INIT, NULL,         \
                         HOST_CALLS, NULL, NULL, NULL)

// A machine that declares no act of its own and whose children are given hosts
// that record: INIT starts it with the table of the host that answers each child's
// installed, which it must be before the first entry runs, and CHILD_HOSTS answers
// what those hosts were asked since it was last asked.
#define STATIC_SCENARIO_CHILD_HOSTED(M, STATES, VARIABLES, INIT, CHILD_HOSTS)                                          \
    STATIC_SCENARIO_CORE(M, STATES, VARIABLES, NULL, no_lists, no_records, NULL, NULL, NULL, NULL, INIT, NULL, NULL,   \
                         NULL, CHILD_HOSTS, NULL)

// What the machines below sent their Mesh peers, kept by the router installed
// here as the compact JSON a scenario's `sent` states: each send its target, its
// event and — where the event has any — its data, which is the engine's own
// serialisation and is put in as it came. One record serves every machine, since
// one scenario is replayed at a time and a start empties it.
//
// A request's pointers are the sender's frame, valid for the call alone, so the
// send is copied where it is heard. One send is at most its target and its event
// name, each an id of the document, and the data of an event, with the names and
// the punctuation around them; a step of these machines sends two at the most —
// the hub, to the two copies — and the record is sized for four times that. What
// does not fit is not cut short: the record is marked as having lost a send, and
// the step it was heard in fails.
enum {
    PEER_SEND_SIZE = 2 * SCE_MAX_ID_LEN + SCE_MAX_DATA_LEN + 48,
    PEER_SENDS_KEPT = 8,
};

static struct {
    char text[PEER_SENDS_KEPT * PEER_SEND_SIZE];
    size_t len;
    bool lost;
} peers_heard;

/* Write `request` down as a scenario's `sent` states it. `members` are written
   after the send's own, already spelled with their leading comma: a child
   session's path and start (SCE_MESH.md, Child Sessions in Mesh), NULL for the
   machine's own send. */
static void peers_record(const sce_host_send_request_t *request, const char *members) {
    char send[PEER_SEND_SIZE];
    sce_forge_wire_t wire;
    sce_forge_wire_begin(&wire, send, sizeof(send));
    sce_forge_wire_pair(&wire, "\"target\"", sce_forge_wire_string(request->target));
    sce_forge_wire_pair(&wire, "\"event\"", sce_forge_wire_string(request->event_name));
    if (request->event_data[0] != '\0') {
        static const char data_key[] = ",\"data\":";
        sce_forge_wire_put(&wire, data_key, sizeof(data_key) - 1u);
        sce_forge_wire_put(&wire, request->event_data, strlen(request->event_data));
    }
    if (members != NULL) {
        sce_forge_wire_put(&wire, members, strlen(members));
    }
    if (!sce_forge_wire_end(&wire)) {
        peers_heard.lost = true;
        return;
    }
    const size_t size = strlen(send);
    if (peers_heard.len + size + 2u >= sizeof(peers_heard.text)) {
        peers_heard.lost = true;
        return;
    }
    if (peers_heard.len > 0u) {
        peers_heard.text[peers_heard.len++] = ',';
    }
    memcpy(peers_heard.text + peers_heard.len, send, size + 1u);
    peers_heard.len += size;
}

static void peers_router(void *user_data, const sce_host_send_request_t *request, sce_host_send_response_list_t *out) {
    (void)user_data;
    (void)out;
    peers_record(request, NULL);
}

static void peers_forget(void) {
    peers_heard.len = 0u;
    peers_heard.text[0] = '\0';
    peers_heard.lost = false;
}

/* The sends since it was last asked, or NULL when one of them was lost. */
static const char *peers_sent(void *sm) {
    static char taken[sizeof(peers_heard.text) + 2u];
    (void)sm;
    const bool lost = peers_heard.lost;
    (void)snprintf(taken, sizeof(taken), "[%s]", peers_heard.text);
    peers_forget();
    return lost ? NULL : taken;
}

// A machine that sends to Mesh peers (`<send target="#name">`), which a machine
// with no router fails as `error.execution`: it is started with the router above
// installed, which it must be before its first entry runs, so that what its start
// sends is heard as the first step's.
#define STATIC_SCENARIO_PEERED(M, STATES, VARIABLES, TEXT, LISTS, RECORDS)                                             \
    static void M##_start_with_router(M##_t *sm) {                                                                     \
        sce_host_processor_registry_t wiring;                                                                          \
        memset(&wiring, 0, sizeof(wiring));                                                                            \
        peers_forget();                                                                                                \
        if (!sce_host_registry_register_mesh_router(&wiring, peers_router, NULL)) {                                    \
            (void)fprintf(stderr, "%s: FAIL - the router door refused the router\n", #M);                              \
        }                                                                                                              \
        M##_init_with_host_processors(sm, &wiring);                                                                    \
    }                                                                                                                  \
    STATIC_SCENARIO_CORE(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, NULL, NULL, NULL, NULL,                           \
                         M##_start_with_router(&sm), NULL, NULL, peers_sent, NULL, NULL)

// The same machine, started as a host that attached no link starts it (docs/adr/0046,
// section 4): with no Mesh router installed, so every send to a peer is refused where
// it is made, and no record of what it sent. It replays a scenario that says
// `"router": false`, which the reader refuses for a driver that records sends.
#define STATIC_SCENARIO_UNROUTED(M)                                                                                    \
    static int M##_unrouted_scenario(const char *scenario, int min_steps) {                                            \
        M##_t sm;                                                                                                      \
        M##_init(&sm);                                                                                                 \
        return M##_replay(&sm, NULL, scenario, min_steps);                                                             \
    }

// A machine whose delayed sends wait on a clock the scenario owns: it is started
// on a manual one, and an `advance_ms` step moves it on and runs what came due.
#define STATIC_SCENARIO_TIMED(M, STATES, VARIABLES, TEXT, LISTS, RECORDS)                                              \
    static bool M##_advance_clock(void *sm, int64_t ms) {                                                              \
        M##_advance_time_ms((M##_t *)sm, (uint64_t)ms);                                                                \
        M##_run((M##_t *)sm);                                                                                          \
        return true;                                                                                                   \
    }                                                                                                                  \
    STATIC_SCENARIO_CORE(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, NULL, NULL, NULL, NULL,                           \
                         M##_init_with_clock(&sm, sce_clock_manual(0u)), M##_advance_clock, NULL, NULL, NULL, NULL)

// A machine that does both: it sends to Mesh peers and its delayed sends wait on a
// clock the scenario owns. It is started with the router installed and on a manual
// clock in one call, because neither single-seam init installs the other: its first
// entry can already send, and a send that arms a delayed one reads the clock.
#define STATIC_SCENARIO_PEERED_TIMED(M, STATES, VARIABLES, TEXT, LISTS, RECORDS)                                       \
    static void M##_start_with_router_and_clock(M##_t *sm) {                                                           \
        sce_host_processor_registry_t wiring;                                                                          \
        memset(&wiring, 0, sizeof(wiring));                                                                            \
        peers_forget();                                                                                                \
        if (!sce_host_registry_register_mesh_router(&wiring, peers_router, NULL)) {                                    \
            (void)fprintf(stderr, "%s: FAIL - the router door refused the router\n", #M);                              \
        }                                                                                                              \
        M##_init_with_clock_and_host_processors(sm, sce_clock_manual(0u), &wiring);                                    \
    }                                                                                                                  \
    static bool M##_advance_clock(void *sm, int64_t ms) {                                                              \
        M##_advance_time_ms((M##_t *)sm, (uint64_t)ms);                                                                \
        M##_run((M##_t *)sm);                                                                                          \
        return true;                                                                                                   \
    }                                                                                                                  \
    STATIC_SCENARIO_CORE(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, NULL, NULL, NULL, NULL,                           \
                         M##_start_with_router_and_clock(&sm), M##_advance_clock, NULL, peers_sent, NULL, NULL)

// The routers the driver gives the child sessions of a machine (SCE_MESH.md, Child
// Sessions in Mesh): the factory answers one for every child, which writes what
// the child sends into the record above with the path of invoke ids that started
// it and the start of that path, holds what a step's `invoke` hands the child
// until its parent applies it, and answers for the child's own children under
// `<path>/<invoke>`. A table of fixed size, since the freestanding profile the
// machines are written for has no allocator; a start past it is no router, which
// the step it was made in fails on as a child whose send was refused.
enum {
    CHILD_ROUTERS_KEPT = 16,
    CHILD_CALLS_KEPT = 4,
    CHILD_PATH_LEN = 128,
};

typedef struct {
    char path[CHILD_PATH_LEN];
    int start;
    bool running;
    sce_mesh_call_t calls[CHILD_CALLS_KEPT];
    size_t queued;
} child_router_t;

static struct {
    child_router_t routers[CHILD_ROUTERS_KEPT];
    size_t count;
} child_routers;

static void child_routers_forget(void) {
    memset(&child_routers, 0, sizeof(child_routers));
}

static bool child_router_open(const char *path, sce_child_mesh_router_t *out);

static void child_router_send(void *user_data, const sce_host_send_request_t *request,
                              sce_host_send_response_list_t *out) {
    const child_router_t *router = (const child_router_t *)user_data;
    char members[CHILD_PATH_LEN + 48];
    (void)snprintf(members, sizeof(members), ",\"invoke\":\"%s\",\"start\":%d", router->path, router->start);
    peers_record(request, members);
    (void)out;
}

static bool child_router_take_call(void *user_data, sce_mesh_call_t *out) {
    child_router_t *router = (child_router_t *)user_data;
    if (router->queued == 0u) {
        return false;
    }
    *out = router->calls[0];
    memmove(&router->calls[0], &router->calls[1], (router->queued - 1u) * sizeof(router->calls[0]));
    router->queued--;
    return true;
}

static void child_router_detach(void *user_data) {
    ((child_router_t *)user_data)->running = false;
}

static bool child_router_child(void *user_data, const char *invoke_id, const char *document,
                               sce_child_mesh_router_t *out) {
    const child_router_t *router = (const child_router_t *)user_data;
    char path[CHILD_PATH_LEN];
    (void)document;
    const size_t base = strlen(router->path);
    const size_t tail = strlen(invoke_id);
    if (base + 1u + tail >= sizeof(path)) {
        return false;
    }
    memcpy(path, router->path, base);
    path[base] = '/';
    memcpy(path + base + 1u, invoke_id, tail + 1u);
    return child_router_open(path, out);
}

static bool child_router_open(const char *path, sce_child_mesh_router_t *out) {
    if (child_routers.count >= CHILD_ROUTERS_KEPT || strlen(path) >= CHILD_PATH_LEN) {
        return false;
    }
    int start = 1;
    for (size_t i = 0; i < child_routers.count; ++i) {
        if (strcmp(child_routers.routers[i].path, path) == 0) {
            ++start;
        }
    }
    child_router_t *router = &child_routers.routers[child_routers.count++];
    memset(router, 0, sizeof(*router));
    memcpy(router->path, path, strlen(path) + 1u);
    router->start = start;
    router->running = true;
    out->user_data = router;
    out->send = child_router_send;
    out->invoke = NULL;
    out->take_call = child_router_take_call;
    out->detach = child_router_detach;
    out->child_router = child_router_child;
    return true;
}

static bool child_routers_ask(void *user_data, const char *invoke_id, const char *document,
                              sce_child_mesh_router_t *out) {
    (void)user_data;
    (void)document;
    return child_router_open(invoke_id, out);
}

/* Hand `event` to the running child of `path`, the latest start of it. */
static bool child_routers_deliver(const char *path, const char *event, const char *data) {
    for (size_t i = child_routers.count; i > 0u; --i) {
        child_router_t *router = &child_routers.routers[i - 1u];
        if (!router->running || strcmp(router->path, path) != 0) {
            continue;
        }
        if (router->queued >= CHILD_CALLS_KEPT || strlen(event) >= SCE_MAX_ID_LEN ||
            (data != NULL && strlen(data) >= SCE_MAX_DATA_LEN)) {
            return false;
        }
        sce_mesh_call_t *call = &router->calls[router->queued++];
        memset(call, 0, sizeof(*call));
        call->kind = SCE_MESH_CALL_RAISE;
        memcpy(call->name, event, strlen(event) + 1u);
        if (data != NULL) {
            memcpy(call->data, data, strlen(data) + 1u);
        }
        return true;
    }
    return false;
}

// A machine whose child sessions talk to Mesh peers: it is started with the
// router that records its own sends and the factory of its children's routers in
// the registry it is started with, which the children of its initial
// configuration, started inside `_init`, are asked of. A step's `invoke` reaches a
// running child through its router, and the rounds after it let the parent apply
// it as it drives the child.
#define STATIC_SCENARIO_CHILD_MESH(M, STATES, VARIABLES)                                                               \
    static void M##_start_with_routers(M##_t *sm) {                                                                    \
        sce_host_processor_registry_t wiring;                                                                          \
        memset(&wiring, 0, sizeof(wiring));                                                                            \
        peers_forget();                                                                                                \
        child_routers_forget();                                                                                        \
        if (!sce_host_registry_register_mesh_router(&wiring, peers_router, NULL) ||                                    \
            !sce_host_registry_register_child_mesh_routers(&wiring, child_routers_ask, NULL)) {                        \
            (void)fprintf(stderr, "%s: FAIL - the router doors refused the routers\n", #M);                            \
        }                                                                                                              \
        M##_init_with_host_processors(sm, &wiring);                                                                    \
    }                                                                                                                  \
    static bool M##_deliver(void *sm, const char *path, const char *event, const char *data) {                         \
        if (!child_routers_deliver(path, event, data)) {                                                               \
            return false;                                                                                              \
        }                                                                                                              \
        for (int round = 0; round < 5; ++round) {                                                                      \
            M##_step((M##_t *)sm);                                                                                     \
        }                                                                                                              \
        return true;                                                                                                   \
    }                                                                                                                  \
    STATIC_SCENARIO_CORE(M, STATES, VARIABLES, NULL, no_lists, no_records, NULL, NULL, NULL, NULL,                     \
                         M##_start_with_routers(&sm), NULL, NULL, peers_sent, NULL, M##_deliver)

// A machine that publishes no real: DONE as above, and REAL, REAL_LIST and
// RECORD_REAL — the readers of a real variable, of a list of reals and of a real
// field of a record — left unset.
#define STATIC_SCENARIO_DONE(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, DONE)                                         \
    STATIC_SCENARIO_FULL(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, DONE, NULL, NULL, NULL)

// A machine with no `<donedata>`.
#define STATIC_SCENARIO(M, STATES, VARIABLES, TEXT, LISTS, RECORDS)                                                    \
    STATIC_SCENARIO_DONE(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, NULL)

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

// static_event_wildcard: where the document listens with `event="*"`, a name
// that no event of the machine's matches is delivered as the wildcard event
// (§scxml-3.12.1) rather than dropped.
VARIABLE_READER(static_event_wildcard, requests)
VARIABLE_READER(static_event_wildcard, strays)
static const name_value_t event_wildcard_states[] = {
    {"listening", STATIC_EVENT_WILDCARD_STATE_LISTENING},
};
static const variable_t event_wildcard_variables[] = {
    {"requests", static_event_wildcard_read_requests},
    {"strays", static_event_wildcard_read_strays},
};
STATIC_SCENARIO(static_event_wildcard, event_wildcard_states, event_wildcard_variables, NULL, no_lists, no_records)

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

// static_enum_number (docs/adr/0047): a value of an enum is read as the number its
// variant declares and a number back as the variant that declares it; a number no
// variant declares is error.execution, and the variable keeps what it held. A value
// is stated as the name its document declares.
VARIABLE_READER(static_enum_number, checks)
VARIABLE_READER(static_enum_number, number)
VARIABLE_READER(static_enum_number, refusals)

static int64_t static_enum_number_read_status(const void *sm) {
    return (int64_t)static_enum_number_get_status((const static_enum_number_t *)sm);
}

static const char *static_enum_number_text(void *sm, const char *name) {
    const static_enum_number_t *machine = (const static_enum_number_t *)sm;
    // The function is named for the enum document's own name: `enum_item_status.scxml`.
    if (strcmp(name, "status") == 0) {
        return enum_item_status_declared_name(static_enum_number_get_status(machine));
    }
    return NULL;
}

static const name_value_t enum_number_states[] = {
    {"board", STATIC_ENUM_NUMBER_STATE_BOARD},
};
static const variable_t enum_number_variables[] = {
    {"checks", static_enum_number_read_checks},
    {"number", static_enum_number_read_number},
    {"refusals", static_enum_number_read_refusals},
    {"status", static_enum_number_read_status},
};
STATIC_SCENARIO(static_enum_number, enum_number_states, enum_number_variables, static_enum_number_text, no_lists,
                no_records)

// static_forge_calls (docs/adr/0048): a condition, a transform, two lookups, an
// interpolation and a validator called as `Alias(args)`; a lookup whose miss is an
// error fails into error.execution, and the variable keeps what it held. The limit
// an interpolation answers is a `double` field, read as the double itself.
VARIABLE_READER(static_forge_calls, gear)
VARIABLE_READER(static_forge_calls, ratio)
VARIABLE_READER(static_forge_calls, code)
VARIABLE_READER(static_forge_calls, misses)
VARIABLE_READER(static_forge_calls, level)
VARIABLE_READER(static_forge_calls, offset)
VARIABLE_READER(static_forge_calls, accepted)
VARIABLE_READER(static_forge_calls, rpm)
VARIABLE_READER(static_forge_calls, highs)
VARIABLE_READER(static_forge_calls, known)

static bool static_forge_calls_read_real(void *sm, const char *name, double *out) {
    if (strcmp(name, "limit") == 0) {
        *out = static_forge_calls_get_limit((const static_forge_calls_t *)sm);
        return true;
    }
    return false;
}

// A value of an enum handed to a function of another kind and taken back, read by the
// name its document declares, and the text a validator is handed (docs/adr/0051).
static const char *static_forge_calls_text(void *sm, const char *name) {
    const static_forge_calls_t *machine = (const static_forge_calls_t *)sm;
    if (strcmp(name, "phase") == 0) {
        return forge_call_status_declared_name(static_forge_calls_get_phase(machine));
    }
    if (strcmp(name, "weight") == 0) {
        return forge_call_level_declared_name(static_forge_calls_get_weight(machine));
    }
    if (strcmp(name, "customer") == 0) {
        return static_forge_calls_get_customer(machine);
    }
    return NULL;
}

static const name_value_t forge_calls_states[] = {
    {"run", STATIC_FORGE_CALLS_STATE_RUN},
};
static const variable_t forge_calls_variables[] = {
    {"gear", static_forge_calls_read_gear},         {"ratio", static_forge_calls_read_ratio},
    {"code", static_forge_calls_read_code},         {"misses", static_forge_calls_read_misses},
    {"level", static_forge_calls_read_level},       {"offset", static_forge_calls_read_offset},
    {"accepted", static_forge_calls_read_accepted}, {"rpm", static_forge_calls_read_rpm},
    {"highs", static_forge_calls_read_highs},       {"known", static_forge_calls_read_known},
};
STATIC_SCENARIO_FULL(static_forge_calls, forge_calls_states, forge_calls_variables, static_forge_calls_text, no_lists,
                     no_records, NULL, static_forge_calls_read_real, NULL, NULL)

// static_payload_enum: an event's payload carries an enum field, the variant's
// declared name, held in the machine's own enum and lifted from the data it
// arrives in. A name the enum does not declare, or a value that is no text, does
// not fit the schema: the delivery is refused and writes nothing.
VARIABLE_READER(static_payload_enum, zoom)
VARIABLE_READER(static_payload_enum, agenda)
VARIABLE_READER(static_payload_enum, shown)

static int64_t static_payload_enum_read_layout(const void *sm) {
    return (int64_t)static_payload_enum_get_layout((const static_payload_enum_t *)sm);
}

static const char *static_payload_enum_text(void *sm, const char *name) {
    if (strcmp(name, "layout") == 0) {
        return enum_view_mode_declared_name(static_payload_enum_get_layout((const static_payload_enum_t *)sm));
    }
    return NULL;
}

static const name_value_t payload_enum_states[] = {
    {"browsing", STATIC_PAYLOAD_ENUM_STATE_BROWSING},
};
static const variable_t payload_enum_variables[] = {
    {"layout", static_payload_enum_read_layout},
    {"zoom", static_payload_enum_read_zoom},
    {"agenda", static_payload_enum_read_agenda},
    {"shown", static_payload_enum_read_shown},
};
STATIC_SCENARIO(static_payload_enum, payload_enum_states, payload_enum_variables, static_payload_enum_text, no_lists,
                no_records)

// static_payload_relay: the payload of the event a transition is on is carried
// on as the `<param>`s of a `<send>` — an enum field as the name its enum
// declares and an integer — read where the send runs, from the payload the
// delivery carried.
VARIABLE_READER(static_payload_relay, zoom)
VARIABLE_READER(static_payload_relay, relays)
VARIABLE_READER(static_payload_relay, refusals)

static int64_t static_payload_relay_read_layout(const void *sm) {
    return (int64_t)static_payload_relay_get_layout((const static_payload_relay_t *)sm);
}

static const char *static_payload_relay_text(void *sm, const char *name) {
    if (strcmp(name, "layout") == 0) {
        return enum_view_mode_declared_name(static_payload_relay_get_layout((const static_payload_relay_t *)sm));
    }
    return NULL;
}

static const name_value_t payload_relay_states[] = {
    {"relaying", STATIC_PAYLOAD_RELAY_STATE_RELAYING},
};
static const variable_t payload_relay_variables[] = {
    {"layout", static_payload_relay_read_layout},
    {"zoom", static_payload_relay_read_zoom},
    {"relays", static_payload_relay_read_relays},
    {"refusals", static_payload_relay_read_refusals},
};
STATIC_SCENARIO(static_payload_relay, payload_relay_states, payload_relay_variables, static_payload_relay_text,
                no_lists, no_records)

// static_string_capacity: a string is a buffer of the UTF-8 bytes its variable
// declares, assigned from a literal or another string, and refused past its
// bound — by bytes, not characters. A published string is read as the text of its
// buffer.
VARIABLE_READER(static_string_capacity, copied)
VARIABLE_READER(static_string_capacity, measured)
VARIABLE_READER(static_string_capacity, refusals)

static const char *static_string_capacity_text(void *sm, const char *name) {
    const static_string_capacity_t *machine = (const static_string_capacity_t *)sm;
    if (strcmp(name, "title") == 0) {
        return static_string_capacity_get_title(machine);
    }
    if (strcmp(name, "body") == 0) {
        return static_string_capacity_get_body(machine);
    }
    return NULL;
}

static const name_value_t string_capacity_states[] = {
    {"idle", STATIC_STRING_CAPACITY_STATE_IDLE},
};
static const variable_t string_capacity_variables[] = {
    {"copied", static_string_capacity_read_copied},
    {"measured", static_string_capacity_read_measured},
    {"refusals", static_string_capacity_read_refusals},
};
STATIC_SCENARIO(static_string_capacity, string_capacity_states, string_capacity_variables, static_string_capacity_text,
                no_lists, no_records)

// static_bytes: a byte string is a buffer of the bytes its variable declares and the
// length it holds, assigned from a literal or from another byte string, and refused
// past its bound. A host is lent the view of it (`sce_forge_bytes_view_t`), which a
// scenario states as the byte-exact Latin-1 text of the bytes: each byte is the
// character of that code point, written as the UTF-8 a JSON string is held in.
VARIABLE_READER(static_bytes, size)
VARIABLE_READER(static_bytes, matches)
VARIABLE_READER(static_bytes, misses)
VARIABLE_READER(static_bytes, errors)

// The text of `view`, in the `size` bytes of `text`, which holds a view of up to
// `(size - 1) / 2` bytes: each byte is two UTF-8 bytes at most.
static const char *bytes_latin1_text(char *text, size_t size, sce_forge_bytes_view_t view) {
    size_t used = 0;
    for (size_t i = 0; i < view.len && used + 2u < size; ++i) {
        const uint8_t byte = view.data[i];
        if (byte < 0x80u) {
            text[used++] = (char)byte;
        } else {
            text[used++] = (char)(0xC0u | (byte >> 6));
            text[used++] = (char)(0x80u | (byte & 0x3Fu));
        }
    }
    text[used] = '\0';
    return text;
}

// The text of `view`, in a buffer that holds a view of up to eight bytes.
static const char *static_bytes_latin1_text(sce_forge_bytes_view_t view) {
    static char text[2 * 8 + 1];
    return bytes_latin1_text(text, sizeof(text), view);
}

static const char *static_bytes_text(void *sm, const char *name) {
    const static_bytes_t *machine = (const static_bytes_t *)sm;
    if (strcmp(name, "frame") == 0) {
        return static_bytes_latin1_text(static_bytes_get_frame(machine));
    }
    if (strcmp(name, "tail") == 0) {
        return static_bytes_latin1_text(static_bytes_get_tail(machine));
    }
    return NULL;
}

static const name_value_t bytes_states[] = {
    {"idle", STATIC_BYTES_STATE_IDLE},
};
static const variable_t bytes_variables[] = {
    {"size", static_bytes_read_size},
    {"matches", static_bytes_read_matches},
    {"misses", static_bytes_read_misses},
    {"errors", static_bytes_read_errors},
};
STATIC_SCENARIO(static_bytes, bytes_states, bytes_variables, static_bytes_text, no_lists, no_records)

// static_record_bytes: a record's byte-string field is a buffer of the bound its
// schema declares and the length it holds, written from a literal and from a byte
// string variable, compared, measured and copied into a list of records. A host reads
// the record by value, and the field's bytes as the Latin-1 text a scenario states,
// copied where it outlives the record the reader holds.
#define FRAMED_FIELDS(N, E) N(sensor) E(frame, static_record_bytes_frame_text)
VARIABLE_READER(static_record_bytes, size)
VARIABLE_READER(static_record_bytes, matches)
VARIABLE_READER(static_record_bytes, misses)
VARIABLE_READER(static_record_bytes, leads)
VARIABLE_READER(static_record_bytes, errors)

static const char *static_record_bytes_frame_text(sce_static_bytes_8_t frame) {
    static char text[2 * 8 + 1];
    return bytes_latin1_text(text, sizeof(text), (sce_forge_bytes_view_t){frame.data, frame.len});
}

RECORD_READER(static_record_bytes, last, static_record_bytes_record_framed_t, FRAMED_FIELDS)
RECORD_LIST_READER(static_record_bytes, frames, static_record_bytes_record_framed_view_t,
                   static_record_bytes_record_framed_t, FRAMED_FIELDS)

static const char *static_record_bytes_text(void *sm, const char *name) {
    static char text[2 * 16 + 1];
    if (strcmp(name, "spare") == 0) {
        return bytes_latin1_text(text, sizeof(text), static_record_bytes_get_spare((const static_record_bytes_t *)sm));
    }
    return NULL;
}

static const name_value_t record_bytes_states[] = {
    {"idle", STATIC_RECORD_BYTES_STATE_IDLE},
};
static const variable_t record_bytes_variables[] = {
    {"size", static_record_bytes_read_size},     {"matches", static_record_bytes_read_matches},
    {"misses", static_record_bytes_read_misses}, {"leads", static_record_bytes_read_leads},
    {"errors", static_record_bytes_read_errors},
};
static const record_variable_t record_bytes_records[] = {
    RECORD_ROW(static_record_bytes, last), RECORD_LIST_ROW(static_record_bytes, frames), {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_record_bytes, record_bytes_states, record_bytes_variables, static_record_bytes_text, no_lists,
                record_bytes_records)

// static_payload_bytes: the bytes a typed payload carries — an array of the schema's
// bound and the length beside it — are read as the view a byte string held by the
// machine is, into a byte string variable, into a record's field and into a whole
// record, each held to its own bound. The record is the one `static_record_bytes`
// reads, of the same bound, so its field is the same type and is read the same way.
VARIABLE_READER(static_payload_bytes, size)
VARIABLE_READER(static_payload_bytes, matches)
VARIABLE_READER(static_payload_bytes, misses)
VARIABLE_READER(static_payload_bytes, errors)

RECORD_READER(static_payload_bytes, last, static_payload_bytes_record_framed_t, FRAMED_FIELDS)
RECORD_LIST_READER(static_payload_bytes, frames, static_payload_bytes_record_framed_view_t,
                   static_payload_bytes_record_framed_t, FRAMED_FIELDS)

static const char *static_payload_bytes_text(void *sm, const char *name) {
    static char text[2 * 4 + 1];
    if (strcmp(name, "held") == 0) {
        return bytes_latin1_text(text, sizeof(text), static_payload_bytes_get_held((const static_payload_bytes_t *)sm));
    }
    return NULL;
}

static const name_value_t payload_bytes_states[] = {
    {"idle", STATIC_PAYLOAD_BYTES_STATE_IDLE},
};
static const variable_t payload_bytes_variables[] = {
    {"size", static_payload_bytes_read_size},
    {"matches", static_payload_bytes_read_matches},
    {"misses", static_payload_bytes_read_misses},
    {"errors", static_payload_bytes_read_errors},
};
static const record_variable_t payload_bytes_records[] = {
    RECORD_ROW(static_payload_bytes, last), RECORD_LIST_ROW(static_payload_bytes, frames), {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_payload_bytes, payload_bytes_states, payload_bytes_variables, static_payload_bytes_text,
                no_lists, payload_bytes_records)

// static_bytes_wire: a byte string held by the machine crosses a `<param>` and a
// `<donedata>` as its byte-exact Latin-1 text, sent to itself and read back through
// the typed payload. The runtime's wire value holds it with its length, since a C
// string cannot hold a 0x00 byte.
VARIABLE_READER(static_bytes_wire, relays)
VARIABLE_READER(static_bytes_wire, errors)

static const char *static_bytes_wire_text(void *sm, const char *name) {
    static char text[2 * 8 + 1];
    const static_bytes_wire_t *machine = (const static_bytes_wire_t *)sm;
    if (strcmp(name, "held") == 0) {
        return bytes_latin1_text(text, sizeof(text), static_bytes_wire_get_held(machine));
    }
    if (strcmp(name, "echo") == 0) {
        return bytes_latin1_text(text, sizeof(text), static_bytes_wire_get_echo(machine));
    }
    return NULL;
}

static const char *static_bytes_wire_done(void *sm) {
    return static_bytes_wire_done_data((const static_bytes_wire_t *)sm);
}

static const name_value_t bytes_wire_states[] = {
    {"idle", STATIC_BYTES_WIRE_STATE_IDLE},
    {"done", STATIC_BYTES_WIRE_STATE_DONE},
};
static const variable_t bytes_wire_variables[] = {
    {"relays", static_bytes_wire_read_relays},
    {"errors", static_bytes_wire_read_errors},
};
STATIC_SCENARIO_DONE(static_bytes_wire, bytes_wire_states, bytes_wire_variables, static_bytes_wire_text, no_lists,
                     no_records, static_bytes_wire_done)

// static_donedata: a top-level final hands its done event the pairs of its
// `<donedata>`, each read from the machine's own fields when the final is entered
// and written as the JSON object of the event's data; a pair whose value failed
// to compute (`small + small` over a uint8) is left out and the others cross.
VARIABLE_READER(static_donedata, count)

static const char *static_donedata_done(void *sm) {
    return static_donedata_done_data((const static_donedata_t *)sm);
}

static const name_value_t donedata_states[] = {
    {"counting", STATIC_DONEDATA_STATE_COUNTING},
    {"done", STATIC_DONEDATA_STATE_DONE},
};
static const variable_t donedata_variables[] = {
    {"count", static_donedata_read_count},
};
STATIC_SCENARIO_DONE(static_donedata, donedata_states, donedata_variables, NULL, no_lists, no_records,
                     static_donedata_done)

// static_donedata_content: a top-level final whose `<donedata>` is inline
// `<content>` hands its done event the text as the JSON string it spells, finished
// when the machine was generated; `42` is the string "42", not the number a script
// engine would read it as. The same machine has finals whose `<content expr>` names
// one value, which is the done event's whole data as the JSON it is: a number as its
// digits, a string quoted, and one that cannot be computed as the empty string.
VARIABLE_READER(static_donedata_content, count)

static const char *static_donedata_content_done(void *sm) {
    return static_donedata_content_done_data((const static_donedata_content_t *)sm);
}

static const name_value_t donedata_content_states[] = {
    {"counting", STATIC_DONEDATA_CONTENT_STATE_COUNTING}, {"done", STATIC_DONEDATA_CONTENT_STATE_DONE},
    {"valued", STATIC_DONEDATA_CONTENT_STATE_VALUED},     {"named", STATIC_DONEDATA_CONTENT_STATE_NAMED},
    {"lost", STATIC_DONEDATA_CONTENT_STATE_LOST},
};
static const variable_t donedata_content_variables[] = {
    {"count", static_donedata_content_read_count},
};
STATIC_SCENARIO_DONE(static_donedata_content, donedata_content_states, donedata_content_variables, NULL, no_lists,
                     no_records, static_donedata_content_done)

// static_send_params: a `<send>` hands its event the pairs of its `<param>`s, read
// from the machine's own fields when the send runs; the machine sends itself the
// event and reads the pairs back through the schemas it imports. A pair whose value
// failed (`small + small` over a uint8) is left out, the message still goes, and
// the receiver, finding a field missing, raises an `error.execution` of its own.
VARIABLE_READER(static_send_params, total)
VARIABLE_READER(static_send_params, ok)
VARIABLE_READER(static_send_params, partialTotal)
VARIABLE_READER(static_send_params, refusals)

static const char *static_send_params_text(void *sm, const char *name) {
    if (strcmp(name, "tag") == 0) {
        return static_send_params_get_tag((const static_send_params_t *)sm);
    }
    return NULL;
}

static const name_value_t send_params_states[] = {
    {"idle", STATIC_SEND_PARAMS_STATE_IDLE},
};
static const variable_t send_params_variables[] = {
    {"total", static_send_params_read_total},
    {"ok", static_send_params_read_ok},
    {"partialTotal", static_send_params_read_partialTotal},
    {"refusals", static_send_params_read_refusals},
};
STATIC_SCENARIO(static_send_params, send_params_states, send_params_variables, static_send_params_text, no_lists,
                no_records)

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

// static_list_index: an element read by its index — the first and the last, one
// under a cursor that moves both ways, and two compared in a guard. An index below
// zero or not below the length reads nothing and raises `error.execution`, and a
// length is a `uint32`, so one taken from an empty list fails as well.
VARIABLE_READER(static_list_index, cursor)
VARIABLE_READER(static_list_index, under)
VARIABLE_READER(static_list_index, first)
VARIABLE_READER(static_list_index, last)
VARIABLE_READER(static_list_index, ordered)
VARIABLE_READER(static_list_index, room)
VARIABLE_READER(static_list_index, count)
VARIABLE_READER(static_list_index, refusals)
LIST_READER(static_list_index, picked, sce_forge_uint8_view_t)
static const name_value_t list_index_states[] = {
    {"reading", STATIC_LIST_INDEX_STATE_READING},
};
static const variable_t list_index_variables[] = {
    {"cursor", static_list_index_read_cursor},   {"under", static_list_index_read_under},
    {"first", static_list_index_read_first},     {"last", static_list_index_read_last},
    {"ordered", static_list_index_read_ordered}, {"room", static_list_index_read_room},
    {"count", static_list_index_read_count},     {"refusals", static_list_index_read_refusals},
};
static const list_variable_t list_index_lists[] = {{"picked", static_list_index_read_list_picked}, {NULL, NULL}};
STATIC_SCENARIO(static_list_index, list_index_states, list_index_variables, NULL, list_index_lists, no_records)

// static_list_assign: a list variable takes whole what an algorithm returns as a
// list — longer, shorter, empty — and a call that fails leaves it as it was and
// raises `error.execution`.
VARIABLE_READER(static_list_assign, first)
VARIABLE_READER(static_list_assign, count)
VARIABLE_READER(static_list_assign, size)
VARIABLE_READER(static_list_assign, refusals)
LIST_READER(static_list_assign, shown, sce_forge_uint8_view_t)
LIST_READER(static_list_assign, few, sce_forge_uint8_view_t)
static const name_value_t list_assign_states[] = {
    {"showing", STATIC_LIST_ASSIGN_STATE_SHOWING},
};
static const variable_t list_assign_variables[] = {
    {"first", static_list_assign_read_first},
    {"count", static_list_assign_read_count},
    {"size", static_list_assign_read_size},
    {"refusals", static_list_assign_read_refusals},
};
static const list_variable_t list_assign_lists[] = {
    {"shown", static_list_assign_read_list_shown}, {"few", static_list_assign_read_list_few}, {NULL, NULL}};
STATIC_SCENARIO(static_list_assign, list_assign_states, list_assign_variables, NULL, list_assign_lists, no_records)

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

// static_real: a 64-bit real is a `double` field of the policy, and a list of
// reals a bounded buffer of them. Their readers answer the double itself, which
// the scenario compares as the 64 bits it is, and `errors` is the integer the
// full list's refused append is counted in.
VARIABLE_READER(static_real, errors)

static bool static_real_read_real(void *sm, const char *name, double *out) {
    const static_real_t *machine = (const static_real_t *)sm;
    if (strcmp(name, "level") == 0) {
        *out = static_real_get_level(machine);
        return true;
    }
    if (strcmp(name, "total") == 0) {
        *out = static_real_get_total(machine);
        return true;
    }
    if (strcmp(name, "drift") == 0) {
        *out = static_real_get_drift(machine);
        return true;
    }
    return false;
}

static bool static_real_read_real_list(void *sm, const char *name, double *out, size_t cap, size_t *len) {
    if (strcmp(name, "samples") != 0) {
        return false;
    }
    const sce_forge_double_view_t view = static_real_get_samples((const static_real_t *)sm);
    for (size_t i = 0; i < view.len && i < cap; ++i) {
        out[i] = view.data[i];
    }
    *len = view.len;
    return view.len <= cap;
}

static const name_value_t real_states[] = {
    {"low", STATIC_REAL_STATE_LOW},
    {"high", STATIC_REAL_STATE_HIGH},
};
static const variable_t real_variables[] = {
    {"errors", static_real_read_errors},
};
STATIC_SCENARIO_FULL(static_real, real_states, real_variables, NULL, no_lists, no_records, NULL, static_real_read_real,
                     static_real_read_real_list, NULL)

// static_real32: a 32-bit real is a `float` field of the policy. Its reader
// widens it to the double it is exactly, which the scenario compares as the 64
// bits it is; the readers of a `float` and of a `double` are the same.
static bool static_real32_read_real(void *sm, const char *name, double *out) {
    const static_real32_t *machine = (const static_real32_t *)sm;
    if (strcmp(name, "level") == 0) {
        *out = (double)static_real32_get_level(machine);
        return true;
    }
    if (strcmp(name, "drift") == 0) {
        *out = (double)static_real32_get_drift(machine);
        return true;
    }
    if (strcmp(name, "wide") == 0) {
        *out = static_real32_get_wide(machine);
        return true;
    }
    if (strcmp(name, "total") == 0) {
        *out = (double)static_real32_get_total(machine);
        return true;
    }
    return false;
}

// A list of `float`: each element is widened to the double it is exactly.
static bool static_real32_read_real_list(void *sm, const char *name, double *out, size_t cap, size_t *len) {
    if (strcmp(name, "samples") != 0) {
        return false;
    }
    const sce_forge_float_view_t view = static_real32_get_samples((const static_real32_t *)sm);
    for (size_t i = 0; i < view.len && i < cap; ++i) {
        out[i] = (double)view.data[i];
    }
    *len = view.len;
    return view.len <= cap;
}

static const name_value_t real32_states[] = {
    {"idle", STATIC_REAL32_STATE_IDLE},
};
// The machine publishes no integer variable: the one row has a name no scenario
// states, so a lookup finds nothing rather than dereferencing a null name.
static const variable_t real32_variables[] = {{"", NULL}};
STATIC_SCENARIO_FULL(static_real32, real32_states, real32_variables, NULL, no_lists, no_records, NULL,
                     static_real32_read_real, static_real32_read_real_list, NULL)

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

// static_record_real: a record with a 64-bit real field. Its integer field is read
// as every record's is, and its real through `record_real`, which answers the
// double itself for the scenario to compare as the 64 bits it is; `sum` is the
// real variable the record's value is added to.
#define READING_FIELDS(N, E) N(sensor)
RECORD_READER(static_record_real, last, static_record_real_record_reading_t, READING_FIELDS)

static bool static_record_real_read_real(void *sm, const char *name, double *out) {
    if (strcmp(name, "sum") != 0) {
        return false;
    }
    *out = static_record_real_get_sum((const static_record_real_t *)sm);
    return true;
}

static bool static_record_real_read_record_real(void *sm, const char *name, size_t index, const char *field,
                                                double *out) {
    if (index != SCE_SCENARIO_WHOLE || strcmp(name, "last") != 0 || strcmp(field, "value") != 0) {
        return false;
    }
    *out = static_record_real_get_last((const static_record_real_t *)sm).value;
    return true;
}

static const name_value_t record_real_states[] = {
    {"idle", STATIC_RECORD_REAL_STATE_IDLE},
};
// The machine publishes no integer variable: the one row has a name no scenario
// states, so a lookup finds nothing rather than dereferencing a null name.
static const variable_t record_real_variables[] = {{"", NULL}};
static const record_variable_t record_real_records[] = {RECORD_ROW(static_record_real, last), {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO_FULL(static_record_real, record_real_states, record_real_variables, NULL, no_lists, record_real_records,
                     NULL, static_record_real_read_real, NULL, static_record_real_read_record_real)

// static_record_real32: the same record with a 32-bit real field. The field is a
// `float`, widened to the double it is exactly for the scenario to compare.
#define READING32_FIELDS(N, E) N(sensor)
VARIABLE_READER(static_record_real32, errors)
RECORD_READER(static_record_real32, last, static_record_real32_record_reading32_t, READING32_FIELDS)

static bool static_record_real32_read_real(void *sm, const char *name, double *out) {
    if (strcmp(name, "sum") != 0) {
        return false;
    }
    *out = (double)static_record_real32_get_sum((const static_record_real32_t *)sm);
    return true;
}

static bool static_record_real32_read_record_real(void *sm, const char *name, size_t index, const char *field,
                                                  double *out) {
    if (index != SCE_SCENARIO_WHOLE || strcmp(name, "last") != 0 || strcmp(field, "value") != 0) {
        return false;
    }
    *out = (double)static_record_real32_get_last((const static_record_real32_t *)sm).value;
    return true;
}

static const name_value_t record_real32_states[] = {
    {"idle", STATIC_RECORD_REAL32_STATE_IDLE},
};
static const variable_t record_real32_variables[] = {
    {"errors", static_record_real32_read_errors},
};
static const record_variable_t record_real32_records[] = {RECORD_ROW(static_record_real32, last),
                                                          {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO_FULL(static_record_real32, record_real32_states, record_real32_variables, NULL, no_lists,
                     record_real32_records, NULL, static_record_real32_read_real, NULL,
                     static_record_real32_read_record_real)

// static_record_string: a record's string field is a buffer of the UTF-8 bytes its
// schema declares, written from a literal, a string variable and a payload — a
// field at a time and the payload whole — held in a record and in a list of
// them. The buffer's text is what a scenario compares, copied where it outlives
// the record the reader holds by value.
#define LABELLED_FIELDS(N, E) N(sensor) E(label, static_record_string_label_text)
VARIABLE_READER(static_record_string, errors)

static const char *static_record_string_label_text(sce_static_string_8_t label) {
    static char text[sizeof(label.data)];
    memcpy(text, label.data, sizeof(text));
    return text;
}

RECORD_READER(static_record_string, last, static_record_string_record_labelled_t, LABELLED_FIELDS)
RECORD_LIST_READER(static_record_string, labels, static_record_string_record_labelled_view_t,
                   static_record_string_record_labelled_t, LABELLED_FIELDS)

static const char *static_record_string_text(void *sm, const char *name) {
    if (strcmp(name, "note") == 0) {
        return static_record_string_get_note((const static_record_string_t *)sm);
    }
    return NULL;
}

static const name_value_t record_string_states[] = {
    {"idle", STATIC_RECORD_STRING_STATE_IDLE},
};
static const variable_t record_string_variables[] = {
    {"errors", static_record_string_read_errors},
};
static const record_variable_t record_string_records[] = {
    RECORD_ROW(static_record_string, last), RECORD_LIST_ROW(static_record_string, labels), {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_record_string, record_string_states, record_string_variables, static_record_string_text,
                no_lists, record_string_records)

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

// static_state_algorithm: the algorithms are handed the records and lists the
// machine holds and the event's own payload, and what they return is written only
// when the call succeeded and held to the bound of the variable it is written to.
#define ENTRY_FIELDS(N, E) N(id) N(amount)
#define TOTAL_FIELDS(N, E) N(count) N(sum)
VARIABLE_READER(static_state_algorithm, idsum)
VARIABLE_READER(static_state_algorithm, large)
VARIABLE_READER(static_state_algorithm, refusals)
LIST_READER(static_state_algorithm, ids, sce_forge_uint32_view_t)
RECORD_READER(static_state_algorithm, total, static_state_algorithm_record_total_t, TOTAL_FIELDS)
RECORD_READER(static_state_algorithm, last, static_state_algorithm_record_entry_t, ENTRY_FIELDS)
RECORD_LIST_READER(static_state_algorithm, entries, static_state_algorithm_record_entry_view_t,
                   static_state_algorithm_record_entry_t, ENTRY_FIELDS)
static const name_value_t state_algorithm_states[] = {
    {"open", STATIC_STATE_ALGORITHM_STATE_OPEN},
};
static const variable_t state_algorithm_variables[] = {
    {"idsum", static_state_algorithm_read_idsum},
    {"large", static_state_algorithm_read_large},
    {"refusals", static_state_algorithm_read_refusals},
};
static const list_variable_t state_algorithm_lists[] = {{"ids", static_state_algorithm_read_list_ids}, {NULL, NULL}};
static const record_variable_t state_algorithm_records[] = {RECORD_ROW(static_state_algorithm, total),
                                                            RECORD_ROW(static_state_algorithm, last),
                                                            RECORD_LIST_ROW(static_state_algorithm, entries),
                                                            {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_state_algorithm, state_algorithm_states, state_algorithm_variables, NULL, state_algorithm_lists,
                state_algorithm_records)

// static_state_algorithm_enum: the algorithms are handed records whose field is a
// value of an enum, and a value of the enum itself, and give back a list of those
// records and a value of the enum. The machine holds the enum in the type the
// enum document's header declares, which the algorithms hold it in too, and each
// value is observed as the name its document declares — the function is named
// for the document's own name, read from its file: `enum_item_status.scxml`.
#define ITEM_FIELDS(N, E) N(id) E(status, enum_item_status_declared_name)
VARIABLE_READER(static_state_algorithm_enum, last)
VARIABLE_READER(static_state_algorithm_enum, openCount)
VARIABLE_READER(static_state_algorithm_enum, startedCount)
VARIABLE_READER(static_state_algorithm_enum, doneCount)
VARIABLE_READER(static_state_algorithm_enum, checks)
VARIABLE_READER(static_state_algorithm_enum, refusals)
RECORD_LIST_READER(static_state_algorithm_enum, items, static_state_algorithm_enum_record_item_view_t,
                   static_state_algorithm_enum_record_item_t, ITEM_FIELDS)

static const char *static_state_algorithm_enum_text(void *sm, const char *name) {
    if (strcmp(name, "last") == 0) {
        return enum_item_status_declared_name(
            static_state_algorithm_enum_get_last((const static_state_algorithm_enum_t *)sm));
    }
    return NULL;
}

static const name_value_t state_algorithm_enum_states[] = {
    {"board", STATIC_STATE_ALGORITHM_ENUM_STATE_BOARD},
};
static const variable_t state_algorithm_enum_variables[] = {
    {"last", static_state_algorithm_enum_read_last},
    {"openCount", static_state_algorithm_enum_read_openCount},
    {"startedCount", static_state_algorithm_enum_read_startedCount},
    {"doneCount", static_state_algorithm_enum_read_doneCount},
    {"checks", static_state_algorithm_enum_read_checks},
    {"refusals", static_state_algorithm_enum_read_refusals},
};
static const record_variable_t state_algorithm_enum_records[] = {RECORD_LIST_ROW(static_state_algorithm_enum, items),
                                                                 {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_state_algorithm_enum, state_algorithm_enum_states, state_algorithm_enum_variables,
                static_state_algorithm_enum_text, no_lists, state_algorithm_enum_records)

// static_record_list_index: a field of the element a list of records is indexed
// at, read in an assignment, an operand and a guard; an index outside the list
// reads nothing and raises `error.execution`.
VARIABLE_READER(static_record_list_index, cursor)
VARIABLE_READER(static_record_list_index, day)
VARIABLE_READER(static_record_list_index, year)
VARIABLE_READER(static_record_list_index, sum)
VARIABLE_READER(static_record_list_index, ordered)
VARIABLE_READER(static_record_list_index, refusals)
static const name_value_t record_list_index_states[] = {
    {"reading", STATIC_RECORD_LIST_INDEX_STATE_READING},
};
static const variable_t record_list_index_variables[] = {
    {"cursor", static_record_list_index_read_cursor},   {"day", static_record_list_index_read_day},
    {"year", static_record_list_index_read_year},       {"sum", static_record_list_index_read_sum},
    {"ordered", static_record_list_index_read_ordered}, {"refusals", static_record_list_index_read_refusals},
};
RECORD_READER(static_record_list_index, picked, static_record_list_index_record_day_t, DAY_FIELDS)
static const record_variable_t record_list_index_records[] = {RECORD_ROW(static_record_list_index, picked),
                                                              {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_record_list_index, record_list_index_states, record_list_index_variables, NULL, no_lists,
                record_list_index_records)

// static_record_enum: a record with an enum field is held in the machine's own
// type for the enum — compared with `===`, assigned a variant, appended whole and
// read through a loop's record item — and its field is observed as the name its
// document declares, not the constant C spells for it.
#define VIEW_FIELDS(N, E) E(layout, enum_view_mode_declared_name) N(zoom)
VARIABLE_READER(static_record_enum, weeks)
VARIABLE_READER(static_record_enum, flips)
VARIABLE_READER(static_record_enum, leads)
RECORD_READER(static_record_enum, shown, static_record_enum_record_view_t, VIEW_FIELDS)
RECORD_LIST_READER(static_record_enum, seen, static_record_enum_record_view_view_t, static_record_enum_record_view_t,
                   VIEW_FIELDS)
static const name_value_t record_enum_states[] = {
    {"viewing", STATIC_RECORD_ENUM_STATE_VIEWING},
};
static const variable_t record_enum_variables[] = {
    {"weeks", static_record_enum_read_weeks},
    {"flips", static_record_enum_read_flips},
    {"leads", static_record_enum_read_leads},
};
static const record_variable_t record_enum_records[] = {
    RECORD_ROW(static_record_enum, shown), RECORD_LIST_ROW(static_record_enum, seen), {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_record_enum, record_enum_states, record_enum_variables, NULL, no_lists, record_enum_records)

// static_whole_payload: the payload of an event is a record of its schema taken
// whole — it replaces a record variable in one assignment and is appended whole to
// a list — and a payload that does not fit its schema is refused as a whole.
VARIABLE_READER(static_whole_payload, updates)
VARIABLE_READER(static_whole_payload, agendas)
VARIABLE_READER(static_whole_payload, others)
RECORD_READER(static_whole_payload, shown, static_whole_payload_record_view_t, VIEW_FIELDS)
RECORD_LIST_READER(static_whole_payload, seen, static_whole_payload_record_view_view_t,
                   static_whole_payload_record_view_t, VIEW_FIELDS)
static const name_value_t whole_payload_states[] = {
    {"viewing", STATIC_WHOLE_PAYLOAD_STATE_VIEWING},
};
static const variable_t whole_payload_variables[] = {
    {"updates", static_whole_payload_read_updates},
    {"agendas", static_whole_payload_read_agendas},
    {"others", static_whole_payload_read_others},
};
static const record_variable_t whole_payload_records[] = {
    RECORD_ROW(static_whole_payload, shown), RECORD_LIST_ROW(static_whole_payload, seen), {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_whole_payload, whole_payload_states, whole_payload_variables, NULL, no_lists,
                whole_payload_records)

// static_schema_revision: a delivery written against an earlier revision of a
// schema that grew lacks the fields a later revision added, and takes their
// defaults; a delivery that lacks a field of the first revision is refused, and a
// field the schema does not name is not read.
#define REVISED_FIELDS(N, E) N(level) E(layout, enum_view_mode_declared_name) N(urgent)
VARIABLE_READER(static_schema_revision, taken)
VARIABLE_READER(static_schema_revision, refusals)
RECORD_READER(static_schema_revision, last, static_schema_revision_record_revised_t, REVISED_FIELDS)
static const name_value_t schema_revision_states[] = {
    {"reading", STATIC_SCHEMA_REVISION_STATE_READING},
};
static const variable_t schema_revision_variables[] = {
    {"taken", static_schema_revision_read_taken},
    {"refusals", static_schema_revision_read_refusals},
};
static const record_variable_t schema_revision_records[] = {RECORD_ROW(static_schema_revision, last),
                                                            {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_schema_revision, schema_revision_states, schema_revision_variables, NULL, no_lists,
                schema_revision_records)

// static_wire_wide: a 64-bit integer crosses a `<param>` as the text of its digits and
// is read back from it; the machine keeps the remainders by 1000, which a value that was
// rounded on the way would get wrong.
VARIABLE_READER(static_wire_wide, low)
VARIABLE_READER(static_wire_wide, smallLow)
VARIABLE_READER(static_wire_wide, large)
VARIABLE_READER(static_wire_wide, negative)
VARIABLE_READER(static_wire_wide, relays)
VARIABLE_READER(static_wire_wide, refusals)
static const name_value_t wire_wide_states[] = {
    {"relaying", STATIC_WIRE_WIDE_STATE_RELAYING},
};
static const variable_t wire_wide_variables[] = {
    {"low", static_wire_wide_read_low},       {"smallLow", static_wire_wide_read_smallLow},
    {"large", static_wire_wide_read_large},   {"negative", static_wire_wide_read_negative},
    {"relays", static_wire_wide_read_relays}, {"refusals", static_wire_wide_read_refusals},
};
STATIC_SCENARIO(static_wire_wide, wire_wide_states, wire_wide_variables, NULL, no_lists, no_records)

// static_wire_enum: an enum value as a `<param>` crosses as the name its enum
// declares for it — a variable, a field of a record variable and a conditional,
// sent and read back through the schema. A value is stated as the name its
// document declares.
VARIABLE_READER(static_wire_enum, deliveries)
RECORD_READER(static_wire_enum, shown, static_wire_enum_record_view_t, VIEW_FIELDS)

static int64_t static_wire_enum_read_layout(const void *sm) {
    return (int64_t)static_wire_enum_get_layout((const static_wire_enum_t *)sm);
}

static int64_t static_wire_enum_read_received(const void *sm) {
    return (int64_t)static_wire_enum_get_received((const static_wire_enum_t *)sm);
}

static const char *static_wire_enum_text(void *sm, const char *name) {
    const static_wire_enum_t *machine = (const static_wire_enum_t *)sm;
    if (strcmp(name, "layout") == 0) {
        return enum_view_mode_declared_name(static_wire_enum_get_layout(machine));
    }
    if (strcmp(name, "received") == 0) {
        return enum_view_mode_declared_name(static_wire_enum_get_received(machine));
    }
    return NULL;
}

static const name_value_t wire_enum_states[] = {
    {"viewing", STATIC_WIRE_ENUM_STATE_VIEWING},
};
static const variable_t wire_enum_variables[] = {
    {"layout", static_wire_enum_read_layout},
    {"received", static_wire_enum_read_received},
    {"deliveries", static_wire_enum_read_deliveries},
};
static const record_variable_t wire_enum_records[] = {RECORD_ROW(static_wire_enum, shown), {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_wire_enum, wire_enum_states, wire_enum_variables, static_wire_enum_text, no_lists,
                wire_enum_records)

// static_send_namelist: the `namelist` of a `<send>` names variables the machine
// holds, each carried as the pair `<param name="x" expr="x"/>` it abbreviates —
// an enum value as the name its enum declares, an integer as its number — and
// read back through the schema.
VARIABLE_READER(static_send_namelist, zoom)
VARIABLE_READER(static_send_namelist, level)
VARIABLE_READER(static_send_namelist, deliveries)

static int64_t static_send_namelist_read_layout(const void *sm) {
    return (int64_t)static_send_namelist_get_layout((const static_send_namelist_t *)sm);
}

static int64_t static_send_namelist_read_received(const void *sm) {
    return (int64_t)static_send_namelist_get_received((const static_send_namelist_t *)sm);
}

static const char *static_send_namelist_text(void *sm, const char *name) {
    const static_send_namelist_t *machine = (const static_send_namelist_t *)sm;
    if (strcmp(name, "layout") == 0) {
        return enum_view_mode_declared_name(static_send_namelist_get_layout(machine));
    }
    if (strcmp(name, "received") == 0) {
        return enum_view_mode_declared_name(static_send_namelist_get_received(machine));
    }
    return NULL;
}

static const name_value_t send_namelist_states[] = {
    {"viewing", STATIC_SEND_NAMELIST_STATE_VIEWING},
};
static const variable_t send_namelist_variables[] = {
    {"layout", static_send_namelist_read_layout},         {"zoom", static_send_namelist_read_zoom},
    {"received", static_send_namelist_read_received},     {"level", static_send_namelist_read_level},
    {"deliveries", static_send_namelist_read_deliveries},
};
STATIC_SCENARIO(static_send_namelist, send_namelist_states, send_namelist_variables, static_send_namelist_text,
                no_lists, no_records)

// static_send_content: the `<content expr>` of a `<send>` names a record, which
// crosses as the pairs of its fields — a record variable and the payload of the
// event the transition is on, taken whole — and is read back through the schema.
VARIABLE_READER(static_send_content, level)
VARIABLE_READER(static_send_content, relays)
RECORD_READER(static_send_content, shown, static_send_content_record_view_t, VIEW_FIELDS)

static int64_t static_send_content_read_received(const void *sm) {
    return (int64_t)static_send_content_get_received((const static_send_content_t *)sm);
}

static const char *static_send_content_text(void *sm, const char *name) {
    if (strcmp(name, "received") == 0) {
        return enum_view_mode_declared_name(static_send_content_get_received((const static_send_content_t *)sm));
    }
    return NULL;
}

static const name_value_t send_content_states[] = {
    {"viewing", STATIC_SEND_CONTENT_STATE_VIEWING},
};
static const variable_t send_content_variables[] = {
    {"received", static_send_content_read_received},
    {"level", static_send_content_read_level},
    {"relays", static_send_content_read_relays},
};
static const record_variable_t send_content_records[] = {RECORD_ROW(static_send_content, shown),
                                                         {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_send_content, send_content_states, send_content_variables, static_send_content_text, no_lists,
                send_content_records)

// static_send_event: the `eventexpr` of a `<send>` is a string the machine holds,
// the name of the event the send delivers, read when the send runs; a name that is
// empty names no event, which is an `error.execution` and nothing sent.
VARIABLE_READER(static_send_event, pings)
VARIABLE_READER(static_send_event, pongs)
VARIABLE_READER(static_send_event, refusals)

static const name_value_t send_event_states[] = {
    {"idle", STATIC_SEND_EVENT_STATE_IDLE},
};
static const variable_t send_event_variables[] = {
    {"pings", static_send_event_read_pings},
    {"pongs", static_send_event_read_pongs},
    {"refusals", static_send_event_read_refusals},
};
STATIC_SCENARIO(static_send_event, send_event_states, send_event_variables, NULL, no_lists, no_records)

// static_send_target: the `targetexpr` of a `<send>` is a string the machine holds,
// the route the send goes by, read when the send runs and held to the routes the
// document declares as `sce:targets`; a value in none of them is an
// `error.communication` and nothing sent, whether or not the machine could send by it.
VARIABLE_READER(static_send_target, landed)
VARIABLE_READER(static_send_target, refused)

static const name_value_t send_target_states[] = {
    {"idle", STATIC_SEND_TARGET_STATE_IDLE},
};
static const variable_t send_target_variables[] = {
    {"landed", static_send_target_read_landed},
    {"refused", static_send_target_read_refused},
};
STATIC_SCENARIO(static_send_target, send_target_states, send_target_variables, NULL, no_lists, no_records)

// static_send_type: the `typeexpr` of a `<send>` is a string the machine holds, the
// processor the send goes through, read when the send runs and held to the
// processors the document declares as `sce:types`; a value in none of them is an
// `error.execution` and nothing sent, whether or not this build serves it.
VARIABLE_READER(static_send_type, landed)
VARIABLE_READER(static_send_type, refused)

static const name_value_t send_type_states[] = {
    {"idle", STATIC_SEND_TYPE_STATE_IDLE},
};
static const variable_t send_type_variables[] = {
    {"landed", static_send_type_read_landed},
    {"refused", static_send_type_read_refused},
};
STATIC_SCENARIO(static_send_type, send_type_states, send_type_variables, NULL, no_lists, no_records)

// static_send_delay: the `delayexpr` of a `<send>` is the string `wait + 'ms'`, a
// number joined to its unit, computed when the send runs and read as the CSS2 time
// it must be; a value that is no time, and an operation that fails, send nothing.
// The join is written into a buffer sized from the bounds the data model declares
// (`SCE_FORGE_CONCAT`), and the machine's sends wait on a clock the scenario owns.
VARIABLE_READER(static_send_delay, wait)
VARIABLE_READER(static_send_delay, beats)
VARIABLE_READER(static_send_delay, refusals)

static const name_value_t send_delay_states[] = {
    {"idle", STATIC_SEND_DELAY_STATE_IDLE},
};
static const variable_t send_delay_variables[] = {
    {"wait", static_send_delay_read_wait},
    {"beats", static_send_delay_read_beats},
    {"refusals", static_send_delay_read_refusals},
};
STATIC_SCENARIO_TIMED(static_send_delay, send_delay_states, send_delay_variables, NULL, no_lists, no_records)

// static_cancel_expr: the `sendidexpr` of a `<cancel>` is a string the machine
// holds, the id of the delayed send it removes, read when the cancel runs; an id
// no send holds cancels nothing, and one that cannot be computed is an
// `error.execution` that removes none. The machine's sends wait on a clock the
// scenario owns.
VARIABLE_READER(static_cancel_expr, a_fired)
VARIABLE_READER(static_cancel_expr, b_fired)
VARIABLE_READER(static_cancel_expr, refusals)

static const name_value_t cancel_expr_states[] = {
    {"idle", STATIC_CANCEL_EXPR_STATE_IDLE},
};
static const variable_t cancel_expr_variables[] = {
    {"a_fired", static_cancel_expr_read_a_fired},
    {"b_fired", static_cancel_expr_read_b_fired},
    {"refusals", static_cancel_expr_read_refusals},
};
STATIC_SCENARIO_TIMED(static_cancel_expr, cancel_expr_states, cancel_expr_variables, NULL, no_lists, no_records)

// static_send_idlocation: the `idlocation` of a `<send>` names a string variable
// the machine writes the id it generates for the send to, which a later `<cancel
// sendidexpr>` names. The id is one execution's, not the element's, and is
// written before the send reads any other argument. The machine's sends wait on
// a clock the scenario owns.
VARIABLE_READER(static_send_idlocation, first_fired)
VARIABLE_READER(static_send_idlocation, second_fired)
VARIABLE_READER(static_send_idlocation, refusals)

static const name_value_t idlocation_states[] = {
    {"idle", STATIC_SEND_IDLOCATION_STATE_IDLE},
};
static const variable_t idlocation_variables[] = {
    {"first_fired", static_send_idlocation_read_first_fired},
    {"second_fired", static_send_idlocation_read_second_fired},
    {"refusals", static_send_idlocation_read_refusals},
};
STATIC_SCENARIO_TIMED(static_send_idlocation, idlocation_states, idlocation_variables, NULL, no_lists, no_records)

// static_history: a `<history>` remembers what its parent held when it was left,
// and entering it brings that back -- the shallow one the child that was active,
// the deep one the atomic states below, and a deep one of a `<parallel>` both
// regions at once. The scenario states the whole active configuration of each
// step, so every state of the machine is named here, a compound or a parallel one
// with the atomic ones.
VARIABLE_READER(static_history, resumed)

static const name_value_t history_states[] = {
    {"running", STATIC_HISTORY_STATE_RUNNING}, {"slow", STATIC_HISTORY_STATE_SLOW},
    {"fast", STATIC_HISTORY_STATE_FAST},       {"cruise", STATIC_HISTORY_STATE_CRUISE},
    {"burst", STATIC_HISTORY_STATE_BURST},     {"working", STATIC_HISTORY_STATE_WORKING},
    {"crew", STATIC_HISTORY_STATE_CREW},       {"left", STATIC_HISTORY_STATE_LEFT},
    {"l1", STATIC_HISTORY_STATE_L1},           {"l2", STATIC_HISTORY_STATE_L2},
    {"right", STATIC_HISTORY_STATE_RIGHT},     {"r1", STATIC_HISTORY_STATE_R1},
    {"r2", STATIC_HISTORY_STATE_R2},           {"paused", STATIC_HISTORY_STATE_PAUSED},
};
static const variable_t history_variables[] = {
    {"resumed", static_history_read_resumed},
};
STATIC_SCENARIO(static_history, history_states, history_variables, NULL, no_lists, no_records)

// static_host_call: a native host action takes the machine's variables as typed
// arguments, each read when the call is made. The machine publishes no variable,
// so what it holds is read from what it told its host: the host installed here
// writes each call as the compact JSON a scenario's `host_calls` states, `action`
// first and then `args`, and hands back what it heard since it was last asked.
static struct {
    char text[1024];
    size_t len;
} host_heard;

static void host_hears(const char *call) {
    const size_t size = strlen(call);
    /* A call that does not fit is left out, which the comparison then reports. */
    if (host_heard.len + size + 2u >= sizeof(host_heard.text)) {
        return;
    }
    if (host_heard.len > 0u) {
        host_heard.text[host_heard.len++] = ',';
    }
    memcpy(host_heard.text + host_heard.len, call, size + 1u);
    host_heard.len += size;
}

static void host_hears_show_attempts(void *user_data, uint32_t count, bool exhausted) {
    (void)user_data;
    char call[96];
    (void)snprintf(call, sizeof(call), "{\"action\":\"showAttempts\",\"args\":[%u,%s]}", (unsigned)count,
                   exhausted ? "true" : "false");
    host_hears(call);
}

static const char *host_call_heard(void *sm) {
    static char taken[sizeof(host_heard.text) + 2u];
    (void)sm;
    (void)snprintf(taken, sizeof(taken), "[%s]", host_heard.text);
    host_heard.len = 0u;
    host_heard.text[0] = '\0';
    return taken;
}

/* The host is installed with the machine, before its first entry calls it. */
static void host_call_start(static_host_call_t *sm) {
    static_host_call_actions_t host;
    memset(&host, 0, sizeof(host));
    host.show_attempts = host_hears_show_attempts;
    host_heard.len = 0u;
    host_heard.text[0] = '\0';
    if (!static_host_call_init_with_actions(sm, &host)) {
        (void)fprintf(stderr, "static_host_call: FAIL - a complete vtable was refused\n");
    }
}

static const name_value_t host_call_states[] = {
    {"idle", STATIC_HOST_CALL_STATE_IDLE},
};
/* No variable is published; the empty name matches no scenario's. */
static const variable_t host_call_variables[] = {
    {"", NULL},
};
STATIC_SCENARIO_HOSTED(static_host_call, host_call_states, host_call_variables, host_call_start(&sm), host_call_heard)

// static_host_duty (docs/adr/0038): a duty taken on entering `engaged` beside a
// notice, and given back on its <onexit>. The host installed here only records, so
// it says it simulates, and the machine is started through the entry that names a
// simulated host. What it heard is written as the compact JSON a scenario's
// `host_calls` states, as the host of `static_host_call` above does, and handed back
// through the same `host_call_heard`.
static void host_hears_start_keepalive(void *user_data) {
    (void)user_data;
    host_hears("{\"action\":\"start_keepalive\",\"args\":[]}");
}

static void host_hears_note_engaged(void *user_data) {
    (void)user_data;
    host_hears("{\"action\":\"note_engaged\",\"args\":[]}");
}

static void host_hears_stop_keepalive(void *user_data) {
    (void)user_data;
    host_hears("{\"action\":\"stop_keepalive\",\"args\":[]}");
}

static void host_duty_start(static_host_duty_t *sm) {
    static_host_duty_actions_t host;
    memset(&host, 0, sizeof(host));
    host.realization = SCE_ACTION_SIMULATES;
    host.start_keepalive = host_hears_start_keepalive;
    host.note_engaged = host_hears_note_engaged;
    host.stop_keepalive = host_hears_stop_keepalive;
    host_heard.len = 0u;
    host_heard.text[0] = '\0';
    if (!static_host_duty_init_with_simulated_actions(sm, &host)) {
        (void)fprintf(stderr, "static_host_duty: FAIL - a complete simulating vtable was refused\n");
    }
}

static const name_value_t host_duty_states[] = {
    {"idle", STATIC_HOST_DUTY_STATE_IDLE},
    {"engaged", STATIC_HOST_DUTY_STATE_ENGAGED},
};
/* No variable is published; the empty name matches no scenario's. */
static const variable_t host_duty_variables[] = {
    {"", NULL},
};
STATIC_SCENARIO_HOSTED(static_host_duty, host_duty_states, host_duty_variables, host_duty_start(&sm), host_call_heard)

// static_child_host and static_child_host_hybrid: a child that declares
// `<sce:action>`s is begun with the table of the host that performs them, which its
// parent's table answers each time the invocation starts, and the parent declares no
// act of its own. What the machine did is what each child asked of the table it was
// given, so each table the parent answers is opened in the book below, and what its
// child calls is written down in it as the compact JSON a scenario's `host_calls`
// states. The tables and the book are static, as the parent's table is, because a
// child keeps the pointer it was handed.
enum {
    CHILD_HOSTS_KEPT = 4,
    CHILD_CALLS_SIZE = 384,
};

typedef struct {
    const char *invoke;
    const char *candidate; /* the document stem of a candidate, NULL for an inline child */
    int start;
    char calls[CHILD_CALLS_SIZE]; /* each call as JSON, each followed by a comma */
    size_t len;
    size_t taken; /* the bytes of `calls` an earlier step took */
    bool announced;
} child_record_t;

static struct {
    child_record_t children[CHILD_HOSTS_KEPT];
    size_t count;
    bool lost;
} child_book;

static void child_book_forget(void) {
    memset(&child_book, 0, sizeof(child_book));
}

/* The parent's table was asked for the table of a child its `invoke` started. NULL
 * for a child the book has no room for, which fails the step it was asked in. */
static child_record_t *child_open(const char *invoke, const char *candidate) {
    if (child_book.count >= CHILD_HOSTS_KEPT) {
        child_book.lost = true;
        return NULL;
    }
    int start = 1;
    for (size_t i = 0; i < child_book.count; ++i) {
        if (strcmp(child_book.children[i].invoke, invoke) == 0) {
            ++start;
        }
    }
    child_record_t *child = &child_book.children[child_book.count++];
    memset(child, 0, sizeof(*child));
    child->invoke = invoke;
    child->candidate = candidate;
    child->start = start;
    return child;
}

/* A call a child made of its table. One that does not fit is not cut short: the book
 * is marked as having lost it, and the step it was heard in fails. */
static void child_hears(void *user_data, const char *call) {
    child_record_t *child = (child_record_t *)user_data;
    const size_t size = strlen(call);
    if (child->len + size + 2u >= sizeof(child->calls)) {
        child_book.lost = true;
        return;
    }
    memcpy(child->calls + child->len, call, size);
    child->len += size;
    child->calls[child->len++] = ',';
    child->calls[child->len] = '\0';
}

/* The children asked for or called since this was last asked, oldest asked first, each
 * as a scenario's `child_hosts` writes it, or NULL when a call or a child was lost. */
static const char *child_hosts_heard(void *sm) {
    static char taken[CHILD_HOSTS_KEPT * (CHILD_CALLS_SIZE + 128) + 4];
    (void)sm;
    size_t used = 0;
    taken[used++] = '[';
    for (size_t i = 0; i < child_book.count; ++i) {
        child_record_t *child = &child_book.children[i];
        /* What it called in this step, without the comma that follows its last. */
        const size_t heard = child->len - child->taken;
        if (child->announced && heard == 0u) {
            continue;
        }
        char head[128];
        int wrote = snprintf(head, sizeof(head), "%s{\"invoke\":\"%s\",\"start\":%d,", used > 1u ? "," : "",
                             child->invoke, child->start);
        if (child->candidate != NULL) {
            wrote += snprintf(head + wrote, sizeof(head) - (size_t)wrote, "\"candidate\":\"%s\",", child->candidate);
        }
        wrote += snprintf(head + wrote, sizeof(head) - (size_t)wrote, "\"host_calls\":[");
        memcpy(taken + used, head, (size_t)wrote);
        used += (size_t)wrote;
        if (heard > 0u) {
            memcpy(taken + used, child->calls + child->taken, heard - 1u);
            used += heard - 1u;
        }
        taken[used++] = ']';
        taken[used++] = '}';
        child->taken = child->len;
        child->announced = true;
    }
    taken[used++] = ']';
    taken[used] = '\0';
    const bool lost = child_book.lost;
    child_book.lost = false;
    return lost ? NULL : taken;
}

static void on_worker_started(void *user_data) {
    child_hears(user_data, "{\"action\":\"started\",\"args\":[]}");
}

static void on_worker_finished(void *user_data, uint32_t steps) {
    char call[64];
    (void)snprintf(call, sizeof(call), "{\"action\":\"finished\",\"args\":[%u]}", (unsigned)steps);
    child_hears(user_data, call);
}

static void on_first_ran(void *user_data) {
    child_hears(user_data, "{\"action\":\"first_ran\",\"args\":[]}");
}

static void on_second_ran(void *user_data) {
    child_hears(user_data, "{\"action\":\"second_ran\",\"args\":[]}");
}

static static_child_host__sce_synth_invoke__worker_actions_t worker_tables[CHILD_HOSTS_KEPT];
static static_hosted_first_actions_t first_tables[CHILD_HOSTS_KEPT];
static static_hosted_second_actions_t second_tables[CHILD_HOSTS_KEPT];

/* The tables are indexed as the book's children are: the child opened n-th is
 * `children[n]`, so its table is the n-th of its kind's pool. */
static const static_child_host__sce_synth_invoke__worker_actions_t *child_host_for_worker(void *user_data) {
    (void)user_data;
    child_record_t *child = child_open("worker", NULL);
    if (child == NULL) {
        return NULL;
    }
    static_child_host__sce_synth_invoke__worker_actions_t *table = &worker_tables[child - child_book.children];
    memset(table, 0, sizeof(*table));
    table->started = on_worker_started;
    table->finished = on_worker_finished;
    table->user_data = child;
    return table;
}

static const static_hosted_first_actions_t *child_host_for_first(void *user_data) {
    (void)user_data;
    child_record_t *child = child_open("work", "static_hosted_first");
    if (child == NULL) {
        return NULL;
    }
    static_hosted_first_actions_t *table = &first_tables[child - child_book.children];
    memset(table, 0, sizeof(*table));
    table->first_ran = on_first_ran;
    table->user_data = child;
    return table;
}

static const static_hosted_second_actions_t *child_host_for_second(void *user_data) {
    (void)user_data;
    child_record_t *child = child_open("work", "static_hosted_second");
    if (child == NULL) {
        return NULL;
    }
    static_hosted_second_actions_t *table = &second_tables[child - child_book.children];
    memset(table, 0, sizeof(*table));
    table->second_ran = on_second_ran;
    table->user_data = child;
    return table;
}

/* The parent's table is installed with the machine, before its first entry runs. */
static void child_host_start(static_child_host_t *sm) {
    static static_child_host_actions_t parent;
    memset(&parent, 0, sizeof(parent));
    parent.actions_for_worker = child_host_for_worker;
    child_book_forget();
    if (!static_child_host_init_with_actions(sm, &parent)) {
        (void)fprintf(stderr, "static_child_host: FAIL - a complete table was refused\n");
    }
}

static void child_host_hybrid_start(static_child_host_hybrid_t *sm) {
    static static_child_host_hybrid_actions_t parent;
    memset(&parent, 0, sizeof(parent));
    parent.actions_for_work_0j_static_hosted_first = child_host_for_first;
    parent.actions_for_work_0j_static_hosted_second = child_host_for_second;
    child_book_forget();
    if (!static_child_host_hybrid_init_with_actions(sm, &parent)) {
        (void)fprintf(stderr, "static_child_host_hybrid: FAIL - a complete table was refused\n");
    }
}

VARIABLE_READER(static_child_host, completed)

static const name_value_t child_host_states[] = {
    {"working", STATIC_CHILD_HOST_STATE_WORKING},
    {"idle", STATIC_CHILD_HOST_STATE_IDLE},
};
static const variable_t child_host_variables[] = {
    {"completed", static_child_host_read_completed},
};
STATIC_SCENARIO_CHILD_HOSTED(static_child_host, child_host_states, child_host_variables, child_host_start(&sm),
                             child_hosts_heard)

VARIABLE_READER(static_child_host_hybrid, completed)

static const name_value_t child_host_hybrid_states[] = {
    {"working", STATIC_CHILD_HOST_HYBRID_STATE_WORKING},
    {"idle", STATIC_CHILD_HOST_HYBRID_STATE_IDLE},
};
static const variable_t child_host_hybrid_variables[] = {
    {"completed", static_child_host_hybrid_read_completed},
};
STATIC_SCENARIO_CHILD_HOSTED(static_child_host_hybrid, child_host_hybrid_states, child_host_hybrid_variables,
                             child_host_hybrid_start(&sm), child_hosts_heard)

// static_child_mesh: a child session that talks to a Mesh peer is given a router of
// its own by the factory the driver registers (SCE_MESH.md, Child Sessions in
// Mesh). What `list` and `inner` send is heard under their path and start, and a
// step's `invoke` reaches them through their routers, `inner`'s answered by the
// router of the hybrid candidate `static_mesh_relay`.
VARIABLE_READER(static_child_mesh, refusals)
static const name_value_t child_mesh_states[] = {
    {"working", STATIC_CHILD_MESH_STATE_WORKING},
    {"alone", STATIC_CHILD_MESH_STATE_ALONE},
    {"joined", STATIC_CHILD_MESH_STATE_JOINED},
    {"idle", STATIC_CHILD_MESH_STATE_IDLE},
};
static const variable_t child_mesh_variables[] = {
    {"refusals", static_child_mesh_read_refusals},
};
STATIC_SCENARIO_CHILD_MESH(static_child_mesh, child_mesh_states, child_mesh_variables)

// static_invoke: a child session an `<invoke>` started is driven through its
// parent by autoforward, takes the events it waits for in order, and its end
// reaches the parent as `done.invoke`, which the parent counts. Leaving the state
// that holds the invoke cancels the child.
VARIABLE_READER(static_invoke, completed)

static const name_value_t invoke_states[] = {
    {"working", STATIC_INVOKE_STATE_WORKING},
    {"idle", STATIC_INVOKE_STATE_IDLE},
};
static const variable_t invoke_variables[] = {
    {"completed", static_invoke_read_completed},
};
STATIC_SCENARIO(static_invoke, invoke_states, invoke_variables, NULL, no_lists, no_records)

// static_invoke_params: an `<invoke>` hands its child the values its `<param>`s and
// `namelist` name, as they stand when the invoke executes, after the entry
// actions, and once.
VARIABLE_READER(static_invoke_params, completed)

static const name_value_t invoke_params_states[] = {
    {"working", STATIC_INVOKE_PARAMS_STATE_WORKING},
    {"plain", STATIC_INVOKE_PARAMS_STATE_PLAIN},
};
static const variable_t invoke_params_variables[] = {
    {"completed", static_invoke_params_read_completed},
};
STATIC_SCENARIO(static_invoke_params, invoke_params_states, invoke_params_variables, NULL, no_lists, no_records)

// static_invoke_string: a string an `<invoke>` hands its child is held to the bound
// the child declared, in bytes: a value past it is left out and raises
// `error.execution`, and the child still starts.
VARIABLE_READER(static_invoke_string, completed)
VARIABLE_READER(static_invoke_string, errors)

static const name_value_t invoke_string_states[] = {
    {"running", STATIC_INVOKE_STRING_STATE_RUNNING},
};
static const variable_t invoke_string_variables[] = {
    {"completed", static_invoke_string_read_completed},
    {"errors", static_invoke_string_read_errors},
};
STATIC_SCENARIO(static_invoke_string, invoke_string_states, invoke_string_variables, NULL, no_lists, no_records)

// static_timers: four delayed sends armed on entering a state are delivered when
// each is due, two due the same moment in the order they were sent, and the last
// takes the machine to its final state; a `<cancel>` by the id of the longest
// removes that one and no other. The machine's sends wait on a clock the scenario
// owns.
VARIABLE_READER(static_timers, trace)

static const name_value_t timers_states[] = {
    {"waiting", STATIC_TIMERS_STATE_WAITING},
};
static const variable_t timers_variables[] = {
    {"trace", static_timers_read_trace},
};
STATIC_SCENARIO_TIMED(static_timers, timers_states, timers_variables, NULL, no_lists, no_records)

// static_donedata_record: a top-level final whose `<donedata>` names a record in
// its `<content expr>` hands its done event the pairs of the record's fields, read
// when the state is entered — an enum field as the name its enum declares.
RECORD_READER(static_donedata_record, shown, static_donedata_record_record_view_t, VIEW_FIELDS)

static int64_t static_donedata_record_read_zoom(const void *sm) {
    return static_donedata_record_get_shown((const static_donedata_record_t *)sm).zoom;
}

static const char *static_donedata_record_done(void *sm) {
    return static_donedata_record_done_data((const static_donedata_record_t *)sm);
}

static const name_value_t donedata_record_states[] = {
    {"counting", STATIC_DONEDATA_RECORD_STATE_COUNTING},
    {"done", STATIC_DONEDATA_RECORD_STATE_DONE},
};
static const variable_t donedata_record_variables[] = {
    {"zoom", static_donedata_record_read_zoom},
};
static const record_variable_t donedata_record_records[] = {RECORD_ROW(static_donedata_record, shown),
                                                            {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO_DONE(static_donedata_record, donedata_record_states, donedata_record_variables, NULL, no_lists,
                     donedata_record_records, static_donedata_record_done)

// A 64-bit integer as a scenario states it, the text of its digits: a JSON number
// past 2^53 is not the integer it spells in every reader, so a variable or a
// record's field of that width is stated, and answered, as a string. The text is
// compared before the next is asked for.
static const char *wide_digits(uint64_t value) {
    static char text[24];
    (void)snprintf(text, sizeof(text), "%llu", (unsigned long long)value);
    return text;
}

static const char *wide_signed_digits(int64_t value) {
    static char text[24];
    (void)snprintf(text, sizeof(text), "%lld", (long long)value);
    return text;
}

// The records of the standard merge rules (sce:std/merge), as the two machines
// below hold them: a hybrid logical clock's stamp, a last-writer-wins cell, an
// element of a replicated growable array and the mark of one taken out.
#define HLC_FIELDS(N, E) E(wallTime, wide_signed_digits) N(counter) E(nodeId, wide_digits)
#define LWW_CELL_FIELDS(N, E)                                                                                          \
    E(row, wide_digits)                                                                                                \
    E(col, wide_digits)                                                                                                \
    E(value, wide_signed_digits)                                                                                       \
    E(wallTime, wide_signed_digits)                                                                                    \
    N(counter)                                                                                                         \
    E(nodeId, wide_digits)                                                                                             \
    N(op)                                                                                                              \
    N(aRef)                                                                                                            \
    E(aRow, wide_digits)                                                                                               \
    E(aCol, wide_digits)                                                                                               \
    E(aVal, wide_signed_digits)                                                                                        \
    N(bRef)                                                                                                            \
    E(bRow, wide_digits)                                                                                               \
    E(bCol, wide_digits)                                                                                               \
    E(bVal, wide_signed_digits)
// What a cell of the sheet shows: a place, the number and the fault when it shows none.
#define SHEET_SHOWN_FIELDS(N, E) E(row, wide_digits) E(col, wide_digits) E(value, wide_signed_digits) N(fault)
#define ORSET_ENTRY_FIELDS(N, E)                                                                                       \
    E(element, wide_digits)                                                                                            \
    E(wallTime, wide_signed_digits)                                                                                    \
    N(counter)                                                                                                         \
    E(nodeId, wide_digits)
#define MERKLE_NODE_FIELDS(N, E) N(depth) E(prefix, wide_digits) N(hash)
#define OCCURRENCE_FIELDS(N, E)                                                                                        \
    E(id, wide_digits) E(start, wide_signed_digits) E(end, wide_signed_digits) N(nth) N(conflict)
#define RGA_NODE_FIELDS(N, E) E(key, wide_digits) E(after, wide_digits)
#define RGA_GONE_FIELDS(N, E) E(key, wide_digits) E(by, wide_digits)

// static_text_record: the algorithms are handed the record and the list of records
// the machine holds, whose text is a buffer of the bound its schema declares, and
// what they return is written back with its text. The buffer's text is what a
// scenario compares, copied where it outlives the record the reader holds by value;
// the version is a 64-bit integer, stated as the text of its digits.
#define SLOT_FIELDS(N, E) N(slot) E(version, wide_signed_digits) E(text, static_text_record_slot_text)
VARIABLE_READER(static_text_record, known)
VARIABLE_READER(static_text_record, refusals)

static const char *static_text_record_slot_text(sce_static_string_32_t text) {
    static char copy[sizeof(text.data)];
    memcpy(copy, text.data, sizeof(copy));
    return copy;
}

RECORD_READER(static_text_record, held, static_text_record_record_slot_t, SLOT_FIELDS)
RECORD_READER(static_text_record, incoming, static_text_record_record_slot_t, SLOT_FIELDS)
RECORD_LIST_READER(static_text_record, slots, static_text_record_record_slot_view_t, static_text_record_record_slot_t,
                   SLOT_FIELDS)
static const name_value_t text_record_states[] = {
    {"open", STATIC_TEXT_RECORD_STATE_OPEN},
};
static const variable_t text_record_variables[] = {
    {"known", static_text_record_read_known},
    {"refusals", static_text_record_read_refusals},
};
static const record_variable_t text_record_records[] = {RECORD_ROW(static_text_record, held),
                                                        RECORD_ROW(static_text_record, incoming),
                                                        RECORD_LIST_ROW(static_text_record, slots),
                                                        {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(static_text_record, text_record_states, text_record_variables, NULL, no_lists, text_record_records)

// todo_peer: one copy of a shared to-do list, replayed through the afternoon the
// world simulation ran it through, as each of the three that took part saw it.
// A to-do's fields are values the copy keeps, each stamped; an observed-remove set
// of entries (`adds`, `tombstones`) decides which stamp of a field is live, merged
// by the standard `orset_union`, `orset_live` and `orset_observed`. Stamps are
// 64-bit and cross as their digits; a field's text travels with its value; and
// what a copy sends the hub, and the hub the other copies, is held send by send.
#define TODO_VALUE_FIELDS(N, E)                                                                                        \
    E(id, wide_digits)                                                                                                 \
    E(field, wide_digits)                                                                                              \
    E(value, wide_signed_digits)                                                                                       \
    E(text, todo_peer_text_text)                                                                                       \
    E(wallTime, wide_signed_digits)                                                                                    \
    N(counter)                                                                                                         \
    E(nodeId, wide_digits)
VARIABLE_READER(todo_peer, hub)
VARIABLE_READER(todo_peer, refusals)
VARIABLE_READER(todo_peer, agendaNext)
VARIABLE_READER(todo_peer, stale)
VARIABLE_READER(todo_peer, late)
VARIABLE_READER(todo_peer, full)
VARIABLE_READER(todo_peer, denied)

static const char *todo_peer_text_text(sce_static_string_96_t text) {
    static char copy[sizeof(text.data)];
    memcpy(copy, text.data, sizeof(copy));
    return copy;
}

RECORD_READER(todo_peer, clock, todo_peer_record_hlc_t, HLC_FIELDS)
RECORD_LIST_READER(todo_peer, adds, todo_peer_record_entry_view_t, todo_peer_record_entry_t, ORSET_ENTRY_FIELDS)
RECORD_LIST_READER(todo_peer, tombstones, todo_peer_record_entry_view_t, todo_peer_record_entry_t, ORSET_ENTRY_FIELDS)
RECORD_LIST_READER(todo_peer, values, todo_peer_record_value_view_t, todo_peer_record_value_t, TODO_VALUE_FIELDS)
RECORD_LIST_READER(todo_peer, digest, todo_peer_record_node_view_t, todo_peer_record_node_t, MERKLE_NODE_FIELDS)
RECORD_LIST_READER(todo_peer, agenda, todo_peer_record_occurrence_view_t, todo_peer_record_occurrence_t,
                   OCCURRENCE_FIELDS)

static const char *todo_peer_text(void *sm, const char *name) {
    if (strcmp(name, "node") == 0) {
        return wide_digits(todo_peer_get_node((const todo_peer_t *)sm));
    }
    if (strcmp(name, "floor") == 0) {
        return wide_signed_digits(todo_peer_get_floor((const todo_peer_t *)sm));
    }
    return NULL;
}

static const name_value_t todo_peer_states[] = {
    {"open", TODO_PEER_STATE_OPEN},
};
static const variable_t todo_peer_variables[] = {
    {"hub", todo_peer_read_hub},       {"refusals", todo_peer_read_refusals}, {"agendaNext", todo_peer_read_agendaNext},
    {"stale", todo_peer_read_stale},   {"late", todo_peer_read_late},         {"full", todo_peer_read_full},
    {"denied", todo_peer_read_denied},
};
static const record_variable_t todo_peer_records[] = {RECORD_ROW(todo_peer, clock),
                                                      RECORD_LIST_ROW(todo_peer, adds),
                                                      RECORD_LIST_ROW(todo_peer, tombstones),
                                                      RECORD_LIST_ROW(todo_peer, values),
                                                      RECORD_LIST_ROW(todo_peer, digest),
                                                      RECORD_LIST_ROW(todo_peer, agenda),
                                                      {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO_PEERED(todo_peer, todo_peer_states, todo_peer_variables, todo_peer_text, no_lists, todo_peer_records)

// record_list: the standard shared record list (sce:std/shared/record_list), the
// generic part of todo_peer, read the way todo_peer is, as a person's copy and as the
// hub, and started by a host, so that it tells no parent anything and
// `announced` stays 0.
#define RECORD_LIST_VALUE_FIELDS(N, E)                                                                                 \
    E(id, wide_digits)                                                                                                 \
    E(field, wide_digits)                                                                                              \
    E(value, wide_signed_digits)                                                                                       \
    E(text, record_list_text_text)                                                                                     \
    E(wallTime, wide_signed_digits)                                                                                    \
    N(counter)                                                                                                         \
    E(nodeId, wide_digits)
VARIABLE_READER(record_list, hub)
VARIABLE_READER(record_list, refusals)
VARIABLE_READER(record_list, stale)
VARIABLE_READER(record_list, late)
VARIABLE_READER(record_list, full)
VARIABLE_READER(record_list, denied)
VARIABLE_READER(record_list, announced)

static const char *record_list_text_text(sce_static_string_96_t text) {
    static char copy[sizeof(text.data)];
    memcpy(copy, text.data, sizeof(copy));
    return copy;
}

RECORD_READER(record_list, clock, record_list_record_hlc_t, HLC_FIELDS)
RECORD_LIST_READER(record_list, adds, record_list_record_entry_view_t, record_list_record_entry_t, ORSET_ENTRY_FIELDS)
RECORD_LIST_READER(record_list, tombstones, record_list_record_entry_view_t, record_list_record_entry_t,
                   ORSET_ENTRY_FIELDS)
RECORD_LIST_READER(record_list, values, record_list_record_value_view_t, record_list_record_value_t,
                   RECORD_LIST_VALUE_FIELDS)
RECORD_LIST_READER(record_list, digest, record_list_record_node_view_t, record_list_record_node_t, MERKLE_NODE_FIELDS)

static const char *record_list_text(void *sm, const char *name) {
    if (strcmp(name, "node") == 0) {
        return wide_digits(record_list_get_node((const record_list_t *)sm));
    }
    if (strcmp(name, "floor") == 0) {
        return wide_signed_digits(record_list_get_floor((const record_list_t *)sm));
    }
    return NULL;
}

static const name_value_t record_list_states[] = {
    {"open", RECORD_LIST_STATE_OPEN},
};
static const variable_t record_list_variables[] = {
    {"hub", record_list_read_hub},
    {"refusals", record_list_read_refusals},
    {"stale", record_list_read_stale},
    {"late", record_list_read_late},
    {"full", record_list_read_full},
    {"denied", record_list_read_denied},
    {"announced", record_list_read_announced},
};
static const record_variable_t record_list_records[] = {
    RECORD_ROW(record_list, clock),           RECORD_LIST_ROW(record_list, adds),
    RECORD_LIST_ROW(record_list, tombstones), RECORD_LIST_ROW(record_list, values),
    RECORD_LIST_ROW(record_list, digest),     {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO_PEERED(record_list, record_list_states, record_list_variables, record_list_text, no_lists,
                       record_list_records)
STATIC_SCENARIO_UNROUTED(record_list)

// record_list_parent: a statechart that invokes the standard record list by its
// library name, passes it the host's writes and supersedes, and publishes what it
// was told of each register: the last register with its count and first value, and
// the further values told after it. A 64-bit field is stated as the text of its
// digits, and the list's text as the buffer of the bound its schema declares.
#define RECORD_LIST_REGISTER_FIELDS(N, E)                                                                              \
    N(id)                                                                                                              \
    N(field)                                                                                                           \
    N(count)                                                                                                           \
    E(value, wide_signed_digits)                                                                                       \
    E(text, record_list_parent_text_text)                                                                              \
    E(wallTime, wide_signed_digits)                                                                                    \
    N(counter)                                                                                                         \
    E(nodeId, wide_digits)
#define RECORD_LIST_REGISTER_VALUE_FIELDS(N, E)                                                                        \
    N(id)                                                                                                              \
    N(field)                                                                                                           \
    N(ordinal)                                                                                                         \
    E(value, wide_signed_digits)                                                                                       \
    E(text, record_list_parent_text_text)                                                                              \
    E(wallTime, wide_signed_digits)                                                                                    \
    N(counter)                                                                                                         \
    E(nodeId, wide_digits)
// The last change the list refused: the command at fault and the reason, each by the
// name its enum declares, record_command and record_refusal (docs/adr/0046, docs/adr/0052).
#define RECORD_LIST_REFUSED_FIELDS(N, E)                                                                               \
    N(id)                                                                                                              \
    N(field)                                                                                                           \
    E(kind, record_command_declared_name)                                                                              \
    E(reason, record_refusal_declared_name)
VARIABLE_READER(record_list_parent, told)
VARIABLE_READER(record_list_parent, refusedCount)

static const char *record_list_parent_text_text(sce_static_string_96_t text) {
    static char copy[sizeof(text.data)];
    memcpy(copy, text.data, sizeof(copy));
    return copy;
}

RECORD_READER(record_list_parent, shown, record_list_parent_record_told_t, RECORD_LIST_REGISTER_FIELDS)
RECORD_LIST_READER(record_list_parent, others, record_list_parent_record_told_value_view_t,
                   record_list_parent_record_told_value_t, RECORD_LIST_REGISTER_VALUE_FIELDS)
RECORD_READER(record_list_parent, refused, record_list_parent_record_refused_t, RECORD_LIST_REFUSED_FIELDS)
static const name_value_t record_list_parent_states[] = {
    {"holding", RECORD_LIST_PARENT_STATE_HOLDING},
};
static const variable_t record_list_parent_variables[] = {
    {"told", record_list_parent_read_told},
    {"refusedCount", record_list_parent_read_refusedCount},
};
static const record_variable_t record_list_parent_records[] = {RECORD_ROW(record_list_parent, shown),
                                                               RECORD_LIST_ROW(record_list_parent, others),
                                                               RECORD_ROW(record_list_parent, refused),
                                                               {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO(record_list_parent, record_list_parent_states, record_list_parent_variables, NULL, no_lists,
                record_list_parent_records)

// sheet_peer: one copy of a shared sheet of numbers, replayed the same way: its
// cells merged by `lww_write`, its rows and columns placed by `rga_insert` and
// taken out by `rga_remove`, and its clock moved by `hlc_send` and `hlc_receive`.
VARIABLE_READER(sheet_peer, hub)
VARIABLE_READER(sheet_peer, orphans)
VARIABLE_READER(sheet_peer, refusals)
RECORD_READER(sheet_peer, clock, sheet_peer_record_hlc_t, HLC_FIELDS)
RECORD_LIST_READER(sheet_peer, cells, sheet_peer_record_lww_cell_view_t, sheet_peer_record_lww_cell_t, LWW_CELL_FIELDS)
RECORD_LIST_READER(sheet_peer, rows, sheet_peer_record_rga_node_view_t, sheet_peer_record_rga_node_t, RGA_NODE_FIELDS)
RECORD_LIST_READER(sheet_peer, gone, sheet_peer_record_rga_gone_view_t, sheet_peer_record_rga_gone_t, RGA_GONE_FIELDS)
RECORD_LIST_READER(sheet_peer, cols, sheet_peer_record_rga_node_view_t, sheet_peer_record_rga_node_t, RGA_NODE_FIELDS)
RECORD_LIST_READER(sheet_peer, goneCols, sheet_peer_record_rga_gone_view_t, sheet_peer_record_rga_gone_t,
                   RGA_GONE_FIELDS)
RECORD_LIST_READER(sheet_peer, shown, sheet_peer_record_shown_view_t, sheet_peer_record_shown_t, SHEET_SHOWN_FIELDS)

static const char *sheet_peer_text(void *sm, const char *name) {
    if (strcmp(name, "node") == 0) {
        return wide_digits(sheet_peer_get_node((const sheet_peer_t *)sm));
    }
    if (strcmp(name, "lamport") == 0) {
        return wide_digits(sheet_peer_get_lamport((const sheet_peer_t *)sm));
    }
    return NULL;
}

static const name_value_t sheet_peer_states[] = {
    {"open", SHEET_PEER_STATE_OPEN},
};
static const variable_t sheet_peer_variables[] = {
    {"hub", sheet_peer_read_hub},
    {"orphans", sheet_peer_read_orphans},
    {"refusals", sheet_peer_read_refusals},
};
static const record_variable_t sheet_peer_records[] = {
    RECORD_ROW(sheet_peer, clock),      RECORD_LIST_ROW(sheet_peer, cells),
    RECORD_LIST_ROW(sheet_peer, rows),  RECORD_LIST_ROW(sheet_peer, gone),
    RECORD_LIST_ROW(sheet_peer, cols),  RECORD_LIST_ROW(sheet_peer, goneCols),
    RECORD_LIST_ROW(sheet_peer, shown), {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO_PEERED(sheet_peer, sheet_peer_states, sheet_peer_variables, sheet_peer_text, no_lists,
                       sheet_peer_records)

// presence_peer: one copy of the list of who is online, replayed through the
// afternoon the world simulation ran it through, as each of the three that took
// part saw it. An entry is kept for a lease, by the machine's own delayed send,
// so the copy runs on a manual clock the scenario's `advance_ms` steps move on,
// and what it sends the hub and the hub the others is held send by send. The
// last time an entry was heard from is 64-bit and crosses as its digits.
#define PRESENCE_SEEN_FIELDS(N, E) N(client) E(last, wide_signed_digits) N(gen)
VARIABLE_READER(presence_peer, node)
VARIABLE_READER(presence_peer, hub)
VARIABLE_READER(presence_peer, ttl)
VARIABLE_READER(presence_peer, lease)
VARIABLE_READER(presence_peer, serial)
VARIABLE_READER(presence_peer, lost)
VARIABLE_READER(presence_peer, refusals)
RECORD_LIST_READER(presence_peer, online, presence_peer_record_seen_view_t, presence_peer_record_seen_t,
                   PRESENCE_SEEN_FIELDS)

static const name_value_t presence_peer_states[] = {
    {"open", PRESENCE_PEER_STATE_OPEN},
};
static const variable_t presence_peer_variables[] = {
    {"node", presence_peer_read_node},         {"hub", presence_peer_read_hub},       {"ttl", presence_peer_read_ttl},
    {"lease", presence_peer_read_lease},       {"serial", presence_peer_read_serial}, {"lost", presence_peer_read_lost},
    {"refusals", presence_peer_read_refusals},
};
static const record_variable_t presence_peer_records[] = {RECORD_LIST_ROW(presence_peer, online),
                                                          {NULL, NULL, NULL, NULL}};
STATIC_SCENARIO_PEERED_TIMED(presence_peer, presence_peer_states, presence_peer_variables, NULL, no_lists,
                             presence_peer_records)

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

// The wire writer's own rules, which no scenario's values reach: a string's `"`,
// `\` and control characters are escaped and its UTF-8 is not, a number of either
// signedness crosses at its widest, and an object that does not fit its buffer is
// `{}` and says so — never a truncated one.
static int the_wire_writer_escapes_text_and_refuses_a_full_buffer(void) {
    int bad = 0;
    char buffer[160];
    sce_forge_wire_t wire;
    sce_forge_wire_begin(&wire, buffer, sizeof(buffer));
    sce_forge_wire_pair(&wire, "\"s\"", sce_forge_wire_string("a\"b\\c\n\x01\xc3\xa9"));
    sce_forge_wire_pair(&wire, "\"i\"", sce_forge_wire_int(-9223372036854775807LL - 1));
    sce_forge_wire_pair(&wire, "\"u\"", sce_forge_wire_uint(18446744073709551615ULL));
    sce_forge_wire_pair(&wire, "\"b\"", sce_forge_wire_bool(false));
    const char *want = "{\"s\":\"a\\\"b\\\\c\\n\\u0001\xc3\xa9\",\"i\":-9223372036854775808,"
                       "\"u\":18446744073709551615,\"b\":false}";
    if (!sce_forge_wire_end(&wire) || strcmp(buffer, want) != 0) {
        (void)fprintf(stderr, "wire: FAIL - the object is `%s`, want `%s`\n", buffer, want);
        bad = 1;
    }
    char small[8];
    sce_forge_wire_begin(&wire, small, sizeof(small));
    sce_forge_wire_pair(&wire, "\"key\"", sce_forge_wire_string("a value that is too long"));
    if (sce_forge_wire_end(&wire) || strcmp(small, "{}") != 0) {
        (void)fprintf(stderr, "wire: FAIL - an object past its buffer is `%s`, want `{}` and a refusal\n", small);
        bad = 1;
    }
    return bad;
}

// The inject seam writes the wire beside the typed payload, and an enum field
// rides it as the name its document declares, not as the number the enum holds: a
// reader of `data` — a script engine, a child, a host — sees what a saved state
// holds. A value no variant names is one the seam refuses, as it refuses a real
// that is not finite. The text of the one event the seam queued is the witness.
static int a_payload_enum_field_is_written_as_the_name_its_enum_declares(void) {
    static const struct {
        enum_view_mode_enum_t value;
        const char *wire;
    } cases[] = {
        {ENUM_VIEW_MODE_MONTH, "{\"layout\":\"month\",\"zoom\":3}"},
        {ENUM_VIEW_MODE_AGENDA_LIST, "{\"layout\":\"agenda_list\",\"zoom\":3}"},
    };

    int bad = 0;
    for (size_t i = 0u; i < sizeof(cases) / sizeof(cases[0]); i++) {
        static_payload_enum_t sm;
        static_payload_enum_init(&sm);
        static_payload_enum_view_shown_payload_t payload;
        memset(&payload, 0, sizeof(payload));
        payload.layout = cases[i].value;
        payload.zoom = 3u;
        if (!static_payload_enum_raise_view_shown_typed(&sm, &payload)) {
            (void)fprintf(stderr, "static_payload_enum: FAIL - the seam refused a variant of the enum\n");
            bad = 1;
            continue;
        }
        const char *queued = sm.external_queue.buf[sm.external_queue.head].data;
        if (strcmp(queued, cases[i].wire) != 0) {
            (void)fprintf(stderr, "static_payload_enum: FAIL - the seam wrote `%s`, want `%s`\n", queued,
                          cases[i].wire);
            bad = 1;
        }
    }
    static_payload_enum_t sm;
    static_payload_enum_init(&sm);
    static_payload_enum_view_shown_payload_t payload;
    memset(&payload, 0, sizeof(payload));
    payload.layout = (enum_view_mode_enum_t)99;
    if (static_payload_enum_raise_view_shown_typed(&sm, &payload)) {
        (void)fprintf(stderr, "static_payload_enum: FAIL - the seam queued a value no variant names\n");
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
    bad |= static_event_wildcard_scenario("static_event_wildcard", 9);
    bad |= static_overflow_scenario("static_overflow", 5);
    bad |= static_block_ends_scenario("static_block_ends", 5);
    bad |= static_list_scenario("static_list", 11);
    bad |= static_list_index_scenario("static_list_index", 24);
    bad |= static_list_assign_scenario("static_list_assign", 11);
    bad |= static_foreach_scenario("static_foreach", 13);
    bad |= static_real_scenario("static_real", 13);
    bad |= static_real32_scenario("static_real32", 11);
    bad |= static_record_real_scenario("static_record_real", 5);
    bad |= static_record_real32_scenario("static_record_real32", 7);
    bad |= static_record_string_scenario("static_record_string", 32);
    bad |= static_block_ends_list_scenario("static_block_ends_list", 4);
    bad |= static_record_fields_scenario("static_record_fields", 9);
    bad |= static_record_scenario("static_record", 16);
    bad |= static_record_list_scenario("static_record_list", 14);
    bad |= static_state_algorithm_scenario("static_state_algorithm", 17);
    bad |= static_state_algorithm_enum_scenario("static_state_algorithm_enum", 18);
    bad |= static_text_record_scenario("static_text_record", 12);
    bad |= static_record_list_index_scenario("static_record_list_index", 19);
    bad |= static_record_enum_scenario("static_record_enum", 29);
    bad |= static_whole_payload_scenario("static_whole_payload", 9);
    bad |= static_schema_revision_scenario("static_schema_revision", 9);
    bad |= static_wire_wide_scenario("static_wire_wide", 9);
    bad |= static_wire_enum_scenario("static_wire_enum", 7);
    bad |= static_payload_scenario("static_payload", 5);
    bad |= static_enum_scenario("static_enum", 11);
    bad |= static_enum_number_scenario("static_enum_number", 12);
    bad |= static_forge_calls_scenario("static_forge_calls", 24);
    bad |= static_payload_enum_scenario("static_payload_enum", 8);
    bad |= static_payload_relay_scenario("static_payload_relay", 5);
    bad |= static_string_capacity_scenario("static_string_capacity", 14);
    bad |= static_bytes_scenario("static_bytes", 12);
    bad |= static_record_bytes_scenario("static_record_bytes", 36);
    bad |= static_payload_bytes_scenario("static_payload_bytes", 15);
    bad |= static_bytes_wire_scenario("static_bytes_wire", 8);
    bad |= static_donedata_scenario("static_donedata", 6);
    bad |= static_donedata_content_scenario("static_donedata_content", 3);
    bad |= static_donedata_content_scenario("static_donedata_content_value", 4);
    bad |= static_donedata_content_scenario("static_donedata_content_text", 3);
    bad |= static_donedata_content_scenario("static_donedata_content_lost", 5);
    bad |= static_donedata_record_scenario("static_donedata_record", 3);
    bad |= static_send_params_scenario("static_send_params", 5);
    bad |= static_send_namelist_scenario("static_send_namelist", 5);
    bad |= static_send_content_scenario("static_send_content", 5);
    bad |= static_send_event_scenario("static_send_event", 11);
    bad |= static_send_target_scenario("static_send_target", 10);
    bad |= static_send_type_scenario("static_send_type", 9);
    bad |= static_send_delay_scenario("static_send_delay", 12);
    bad |= static_cancel_expr_scenario("static_cancel_expr", 16);
    bad |= static_send_idlocation_scenario("static_send_idlocation", 13);
    bad |= static_history_scenario("static_history", 17);
    bad |= static_host_call_scenario("static_host_call", 5);
    bad |= static_host_duty_scenario("static_host_duty", 4);
    bad |= static_child_host_scenario("static_child_host", 8);
    bad |= static_child_host_hybrid_scenario("static_child_host_hybrid", 3);
    bad |= static_child_mesh_scenario("static_child_mesh", 7);
    bad |= static_invoke_scenario("static_invoke", 6);
    bad |= static_invoke_scenario("static_invoke_abort", 4);
    bad |= static_invoke_params_scenario("static_invoke_params", 2);
    bad |= static_invoke_string_scenario("static_invoke_string", 1);
    bad |= static_timers_scenario("static_timers", 6);
    bad |= static_timers_scenario("static_timers_stop", 5);
    bad |= sync_client_scenario("sync_client", 30);
    bad |= todo_peer_scenario("todo_peer_alice", 13);
    bad |= todo_peer_scenario("todo_peer_bob", 13);
    bad |= todo_peer_scenario("todo_peer_server", 13);
    bad |= record_list_scenario("record_list_person", 29);
    bad |= record_list_scenario("record_list_hub", 21);
    bad |= record_list_scenario("record_list_alone", 5);
    bad |= record_list_unrouted_scenario("record_list_offline", 8);
    bad |= record_list_scenario("record_list_change", 26);
    bad |= record_list_scenario("record_list_retire", 15);
    bad |= record_list_parent_scenario("record_list_parent", 27);
    bad |= sheet_peer_scenario("sheet_peer_alice", 24);
    bad |= sheet_peer_scenario("sheet_peer_bob", 26);
    bad |= sheet_peer_scenario("sheet_peer_server", 26);
    bad |= presence_peer_scenario("presence_peer_alice", 23);
    bad |= presence_peer_scenario("presence_peer_bob", 21);
    bad |= presence_peer_scenario("presence_peer_server", 12);
    bad |= content_that_reads_a_payload_does_not_run_for_a_delivery_without_one();
    bad |= a_payload_enum_field_is_written_as_the_name_its_enum_declares();
    bad |= the_wire_writer_escapes_text_and_refuses_a_full_buffer();
    if (bad != 0) {
        return 1;
    }
    (void)printf("static scalars: ok\n");
    return 0;
}
