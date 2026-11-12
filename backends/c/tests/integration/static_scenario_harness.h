// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The driver of a `datamodel="sce-static"` machine generated as C (docs/SCE_ACCEPTED_SUBSET.md
// §2.15) for the shared scenarios every backend replays
// (`sce-build/tests/fixtures/static_datamodel/scenarios/<machine>.json`): the readers of what a
// machine publishes, the replay of one scenario, the Mesh router that keeps what a machine sends
// its peers, and the readers of the records of the standard merge rules. A test program binds each
// machine it replays with one of the `STATIC_SCENARIO*` macros and calls `<machine>_scenario`.
//
// Shared by `test_static_scalars.c`, which replays every machine at the bounds its document
// writes, and `test_static_large.c`, which replays the shared to-do list built at the bounds of a
// team (docs/adr/0082). A machine is held in static storage, never in a local of the replay: at a
// large bound it is megabytes, which a stack does not hold.

#ifndef SCE_TESTS_STATIC_SCENARIO_HARNESS_H
#define SCE_TESTS_STATIC_SCENARIO_HARNESS_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include <sce/forge/wire.h>
#include <sce/host_processor.h>
#include <sce/types.h>

#include "../require_started.h"
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
static SCE_C_UNUSED const list_variable_t no_lists[] = {{NULL, NULL}};

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
static SCE_C_UNUSED const record_variable_t no_records[] = {{NULL, NULL, NULL, NULL}};

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
// given none. PRESENCE answers whether a value that may be absent is present
// (docs/adr/0055), or is NULL for a machine that holds none.
#define STATIC_SCENARIO_CORE_RESTORING(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, DONE, REAL, REAL_LIST, RECORD_REAL, \
                                       INIT, ADVANCE, HOST_CALLS, SENT, CHILD_HOSTS, DELIVER, PRESENCE, RESTORE)       \
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
        /* A view of the step's data; the machine's queue copies it, and data past the slot fails the step. */         \
        if (data != NULL && strlen(data) >= sizeof(((M##_queued_event_t *)0)->data)) {                                 \
            return false;                                                                                              \
        }                                                                                                              \
        meta.data = data;                                                                                              \
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
                                              PRESENCE,                                                                \
                                              RESTORE};                                                                \
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
        /* Static: a machine built at a large bound is megabytes (docs/adr/0082). INIT starts it anew. */              \
        static M##_t sm;                                                                                               \
        INIT;                                                                                                          \
        return M##_replay(&sm, SENT, scenario, min_steps);                                                             \
    }

// A machine whose driver does not save and restore it, which is every machine but a tree.
#define STATIC_SCENARIO_CORE(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, DONE, REAL, REAL_LIST, RECORD_REAL, INIT,     \
                             ADVANCE, HOST_CALLS, SENT, CHILD_HOSTS, DELIVER, PRESENCE)                                \
    STATIC_SCENARIO_CORE_RESTORING(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, DONE, REAL, REAL_LIST, RECORD_REAL,     \
                                   INIT, ADVANCE, HOST_CALLS, SENT, CHILD_HOSTS, DELIVER, PRESENCE, NULL)

// A machine that waits on no clock, started as `_init` starts it.
#define STATIC_SCENARIO_FULL(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, DONE, REAL, REAL_LIST, RECORD_REAL)           \
    STATIC_SCENARIO_CORE(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, DONE, REAL, REAL_LIST, RECORD_REAL,               \
                         REQUIRE_STARTED(M##_init(&sm), PASTED_START_OK), NULL, NULL, NULL, NULL, NULL, NULL)

// A machine whose `<sce:action>`s go to a host that records them: INIT starts it
// with that host installed, which it must be before the first entry runs, and
// HOST_CALLS answers what the host heard since it was last asked.
#define STATIC_SCENARIO_HOSTED(M, STATES, VARIABLES, INIT, HOST_CALLS)                                                 \
    STATIC_SCENARIO_CORE(M, STATES, VARIABLES, NULL, no_lists, no_records, NULL, NULL, NULL, NULL, INIT, NULL,         \
                         HOST_CALLS, NULL, NULL, NULL, NULL)

// A machine that declares no act of its own and whose children are given hosts
// that record: INIT starts it with the table of the host that answers each child's
// installed, which it must be before the first entry runs, and CHILD_HOSTS answers
// what those hosts were asked since it was last asked.
#define STATIC_SCENARIO_CHILD_HOSTED(M, STATES, VARIABLES, INIT, CHILD_HOSTS)                                          \
    STATIC_SCENARIO_CORE(M, STATES, VARIABLES, NULL, no_lists, no_records, NULL, NULL, NULL, NULL, INIT, NULL, NULL,   \
                         NULL, CHILD_HOSTS, NULL, NULL)

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
        REQUIRE_STARTED(M##_init_with_host_processors(sm, &wiring), PASTED_START_OK);                                  \
    }                                                                                                                  \
    STATIC_SCENARIO_CORE(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, NULL, NULL, NULL, NULL,                           \
                         M##_start_with_router(&sm), NULL, NULL, peers_sent, NULL, NULL, NULL)

// The same machine, started as a host that attached no link starts it (docs/adr/0046,
// section 4): with no Mesh router installed, so every send to a peer is refused where
// it is made, and no record of what it sent. It replays a scenario that says
// `"router": false`, which the reader refuses for a driver that records sends.
#define STATIC_SCENARIO_UNROUTED(M)                                                                                    \
    static int M##_unrouted_scenario(const char *scenario, int min_steps) {                                            \
        static M##_t sm;                                                                                               \
        REQUIRE_STARTED(M##_init(&sm), PASTED_START_OK);                                                               \
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
                         REQUIRE_STARTED(M##_init_with_clock(&sm, sce_clock_manual(0u)), PASTED_START_OK),             \
                         M##_advance_clock, NULL, NULL, NULL, NULL, NULL)

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
        REQUIRE_STARTED(M##_init_with_clock_and_host_processors(sm, sce_clock_manual(0u), &wiring), PASTED_START_OK);  \
    }                                                                                                                  \
    static bool M##_advance_clock(void *sm, int64_t ms) {                                                              \
        M##_advance_time_ms((M##_t *)sm, (uint64_t)ms);                                                                \
        M##_run((M##_t *)sm);                                                                                          \
        return true;                                                                                                   \
    }                                                                                                                  \
    STATIC_SCENARIO_CORE(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, NULL, NULL, NULL, NULL,                           \
                         M##_start_with_router_and_clock(&sm), M##_advance_clock, NULL, peers_sent, NULL, NULL, NULL)

// A machine that publishes no real: DONE as above, and REAL, REAL_LIST and
// RECORD_REAL — the readers of a real variable, of a list of reals and of a real
// field of a record — left unset.
#define STATIC_SCENARIO_DONE(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, DONE)                                         \
    STATIC_SCENARIO_FULL(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, DONE, NULL, NULL, NULL)

// A machine with no `<donedata>`.
#define STATIC_SCENARIO(M, STATES, VARIABLES, TEXT, LISTS, RECORDS)                                                    \
    STATIC_SCENARIO_DONE(M, STATES, VARIABLES, TEXT, LISTS, RECORDS, NULL)

// A 64-bit integer as a scenario states it, the text of its digits: a JSON number
// past 2^53 is not the integer it spells in every reader, so a variable or a
// record's field of that width is stated, and answered, as a string. The text is
// compared before the next is asked for.
static SCE_C_UNUSED const char *wide_digits(uint64_t value) {
    static char text[24];
    (void)snprintf(text, sizeof(text), "%llu", (unsigned long long)value);
    return text;
}

static SCE_C_UNUSED const char *wide_signed_digits(int64_t value) {
    static char text[24];
    (void)snprintf(text, sizeof(text), "%lld", (long long)value);
    return text;
}

// The records of the standard merge rules (sce:std/merge), as the machines made of
// them hold them: a hybrid logical clock's stamp, a last-writer-wins cell, an
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

#endif
