// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Which writes to a system variable the Lua state of a generated C11 machine
// refuses, measured on the bindings the machine made.
//
// §scxml-5.10: "The Processor MUST cause any attempt to change the value of a
// system variable to fail and MUST place the error 'error.execution' on the
// internal event queue when such an attempt is made." §scxml-B-2-8 says what is
// read-only in the ECMAScript data model: the variable itself, each of `_event`'s
// fields, and each `_ioprocessors` entry with its `location`. The expectations
// live in tests/ecmascript/system_variable_writes.json, which every backend's
// reader asks; each case is asked here in its Lua spelling (`write_lua`,
// `after_lua`), because the machine evaluates Lua. A refused write is a script
// that fails, which is what is asked of the engine here; the error.execution the
// machine raises for a failing script is each action's own path.
//
// This backend binds the system variables in the generated machine —
// `_sessionid`, `_name` and `_ioprocessors` in `lua_init_engine`, `_event` in the
// static `set_current_event` — so a machine carries the question: each case gets
// one of its own (`system_variable_writes`, a document of this channel alone).
// The machine is first held to having bound each of the four through the shared
// `_scxml_bind_system`: such a variable reads through `_G` and is not in `_G`
// itself, which no ordinary global write can produce. The machine's session
// values are its own (its id is drawn at `_init`), so the table's `session` is
// then bound through the same binder, as every other reader binds it through its
// engine; the table's `event` is handed to the machine through its external
// queue, and the machine's own dequeue binds `_event`. Then the case's write
// runs — it must fail exactly when the table says `refused` — and its
// after-condition must hold, so a write reported as failing that landed anyway
// fails the case too.

#include <lauxlib.h>
#include <lua.h>
#include <lualib.h>

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../require_started.h"
#include "../unit/shared_table.h"
#include "system_variable_writes_sm.h"

#ifndef SCE_SYSTEM_VARIABLE_WRITES_TABLE
#error "SCE_SYSTEM_VARIABLE_WRITES_TABLE names tests/ecmascript/system_variable_writes.json; the build defines it"
#endif

enum { FIELD_CAP = 512, MIN_CASES = 18 };

typedef system_variable_writes_t sm_t;
typedef system_variable_writes_event_with_meta_t event_t;

// The table's `session` and `event` objects, which every case runs with; set by
// `main` before the cases are walked.
static const char *session_object;
static const char *event_object;

// The cases whose write and after-condition were both run, which `main` holds to
// the number the table holds: a case that was walked and never asked must not
// pass as one that agreed.
static int asked;

// The three the machine binds when it starts; `_event` is bound per event.
static const char *const SESSION_VARIABLES[3] = {"_sessionid", "_name", "_ioprocessors"};

// Whether `name` was bound by `_scxml_bind_system`: readable through `_G`, and
// absent from `_G` itself, where a global set the ordinary way — a copy any
// script could rebind — would sit.
static bool bound_by_the_binder(lua_State *L, const char *name) {
    lua_pushglobaltable(L);
    lua_pushstring(L, name);
    lua_rawget(L, -2);
    const bool in_the_table = !lua_isnil(L, -1);
    lua_pop(L, 1);
    lua_getfield(L, -1, name);
    const bool readable = !lua_isnil(L, -1);
    lua_pop(L, 2);
    return readable && !in_the_table;
}

// Bind the value at the top of the stack as system variable `name` the way the
// machine binds its own, through the shared binder in a protected call, and pop
// it. False, with the binder's reason written, when it refused.
static bool bind_as_the_machine_does(lua_State *L, const char *name) {
    lua_getglobal(L, "_scxml_bind_system");
    lua_insert(L, -2);
    lua_pushstring(L, name);
    lua_insert(L, -2);
    if (lua_pcall(L, 2, 0, 0) == LUA_OK) {
        return true;
    }
    (void)fprintf(stderr, "FAIL: binding the table's %s: %s\n", name,
                  lua_tostring(L, -1) != NULL ? lua_tostring(L, -1) : "unknown error");
    lua_pop(L, 1);
    return false;
}

// The table's `session` bound over the machine's own: `sessionid` as `_sessionid`,
// `name` as `_name`, and each member of `ioprocessors` as the `_ioprocessors`
// entry of that name, its value the entry's `location`.
static bool bind_table_session(lua_State *L) {
    char sessionid[FIELD_CAP];
    char name[FIELD_CAP];
    const char *ioprocessors = sce_payload_find_in(session_object, "ioprocessors");
    if (read_field(session_object, "sessionid", sessionid, sizeof(sessionid), NULL) != NULL ||
        read_field(session_object, "name", name, sizeof(name), NULL) != NULL || ioprocessors == NULL ||
        *ioprocessors != '{') {
        (void)fprintf(stderr, "FAIL: the table's session has no readable sessionid, name or ioprocessors\n");
        return false;
    }
    lua_pushstring(L, sessionid);
    if (!bind_as_the_machine_does(L, "_sessionid")) {
        return false;
    }
    lua_pushstring(L, name);
    if (!bind_as_the_machine_does(L, "_name")) {
        return false;
    }
    lua_newtable(L);
    int entries = 0;
    for (const char *member = sce_payload_skip_space(ioprocessors + 1); member != NULL && *member == '"';
         member = sce_payload_next_member(member)) {
        char processor[FIELD_CAP];
        char location[FIELD_CAP];
        sce_payload_sink_t sink;
        memset(&sink, 0, sizeof(sink));
        sink.text = processor;
        sink.cap = sizeof(processor);
        if (sce_payload_walk_text(member, &sink) != NULL) {
            (void)fprintf(stderr, "FAIL: an ioprocessors entry of the table's session has no readable name\n");
            lua_pop(L, 1);
            return false;
        }
        processor[sink.len] = '\0';
        if (read_field(ioprocessors, processor, location, sizeof(location), NULL) != NULL) {
            (void)fprintf(stderr, "FAIL: the table's ioprocessors entry `%s` has no readable location\n", processor);
            lua_pop(L, 1);
            return false;
        }
        lua_newtable(L);
        lua_pushstring(L, location);
        lua_setfield(L, -2, "location");
        lua_setfield(L, -2, processor);
        entries++;
    }
    if (entries == 0) {
        (void)fprintf(stderr, "FAIL: the table's session names no ioprocessors entry\n");
        lua_pop(L, 1);
        return false;
    }
    return bind_as_the_machine_does(L, "_ioprocessors");
}

// The table's `event` as the event record the machine's queue takes, its name in
// `name` and its data in `data`, of `data_cap` bytes. False when a member cannot
// be read or the carrier has no such event.
static bool read_table_event(event_t *evt, char *name, size_t name_cap, char *data, size_t data_cap) {
    char type[SCE_MAX_EVENT_TYPE_LEN];
    char sendid[SCE_MAX_ID_LEN];
    char invokeid[SCE_MAX_ID_LEN];
    char origin[SCE_MAX_URI_LEN];
    char origintype[SCE_MAX_ID_LEN];
    memset(evt, 0, sizeof(*evt));
    evt->data = data;
    if (read_field(event_object, "name", name, name_cap, NULL) != NULL ||
        read_field(event_object, "type", type, sizeof(type), NULL) != NULL ||
        read_field(event_object, "sendid", sendid, sizeof(sendid), NULL) != NULL ||
        read_field(event_object, "invokeid", invokeid, sizeof(invokeid), NULL) != NULL ||
        read_field(event_object, "origin", origin, sizeof(origin), NULL) != NULL ||
        read_field(event_object, "origintype", origintype, sizeof(origintype), NULL) != NULL ||
        read_field(event_object, "data", data, data_cap, NULL) != NULL) {
        (void)fprintf(stderr, "FAIL: the table's event cannot be read whole\n");
        return false;
    }
    if (strcmp(type, "external") != 0) {
        (void)fprintf(stderr, "FAIL: the table's event is `%s`; this reader delivers an external one\n", type);
        return false;
    }
    if (!system_variable_writes_resolve_event_by_name(name, &evt->event)) {
        (void)fprintf(stderr, "FAIL: the carrier has no event `%s`\n", name);
        return false;
    }
    sce_copy_bounded_event_type(evt->type, type);
    sce_copy_bounded_id(evt->send_id, sendid);
    sce_copy_bounded_id(evt->invoke_id, invokeid);
    if (!sce_event_origin_set(&evt->origin, origintype, origin)) {
        (void)fprintf(stderr, "FAIL: the table's origin does not fit the event record\n");
        return false;
    }
    return true;
}

// Whether the `_event` bound is the one for the event named `name`: a machine
// that took no event would have the cases ask a variable that was never bound.
static bool bound_event_named(lua_State *L, const char *name) {
    lua_getglobal(L, "_event");
    bool named = false;
    if (lua_istable(L, -1)) {
        lua_getfield(L, -1, "name");
        named = lua_type(L, -1) == LUA_TSTRING && strcmp(lua_tostring(L, -1), name) == 0;
        lua_pop(L, 1);
    }
    lua_pop(L, 1);
    return named;
}

// Whether the machine's own `_sessionid` is the id it drew, so the binding the
// test found is the machine's and not one left from somewhere else.
static bool bound_its_own_session_id(lua_State *L, const sm_t *sm) {
    lua_getglobal(L, "_sessionid");
    const bool same = lua_type(L, -1) == LUA_TSTRING && strcmp(lua_tostring(L, -1), sm->session_id) == 0;
    lua_pop(L, 1);
    return same;
}

// One case: the machine's own bindings, then the table's session and event, then
// the case's write and its after-condition. Every disagreement of the case is
// written; the case counts once.
static int answer_case(sm_t *sm, const char *object, int index) {
    char group[FIELD_CAP];
    char write_lua[FIELD_CAP];
    char after_lua[FIELD_CAP];
    char clause[FIELD_CAP];
    bool refused = false;
    const sce_payload_fields_t fields = {object, NULL};
    if (read_field(object, "group", group, sizeof(group), NULL) != NULL ||
        read_field(object, "write_lua", write_lua, sizeof(write_lua), NULL) != NULL ||
        read_field(object, "after_lua", after_lua, sizeof(after_lua), NULL) != NULL ||
        read_field(object, "clause", clause, sizeof(clause), NULL) != NULL ||
        sce_payload_read_bool(&fields, "refused", &refused) != NULL) {
        (void)fprintf(stderr, "FAIL: case %d: no group, write_lua, after_lua, clause or readable `refused`\n", index);
        return 1;
    }

    lua_State *L = sm->L;
    int failed = 0;
    for (size_t i = 0; i < sizeof(SESSION_VARIABLES) / sizeof(SESSION_VARIABLES[0]); i++) {
        if (!bound_by_the_binder(L, SESSION_VARIABLES[i])) {
            (void)fprintf(stderr,
                          "FAIL: %s [%s]: the machine bound `%s` as an ordinary global, which a script can rebind\n",
                          group, write_lua, SESSION_VARIABLES[i]);
            failed = 1;
        }
    }
    if (!bound_its_own_session_id(L, sm)) {
        (void)fprintf(stderr, "FAIL: %s [%s]: `_sessionid` is not the id the machine drew (`%s`)\n", group, write_lua,
                      sm->session_id);
        failed = 1;
    }
    if (failed != 0 || !bind_table_session(L)) {
        return 1;
    }

    event_t evt;
    char name[SCE_MAX_ID_LEN];
    /* The record holds a view of its data, which the machine's queue copies into
       its own heap (docs/adr/0086, section 8.6 (d)): a script machine has no slot
       to size it by. The text is held at the size of the table's text from the
       event on, which its data, a part of that text, cannot pass. */
    const size_t data_cap = strlen(event_object) + 1u;
    char *data = (char *)malloc(data_cap);
    if (data == NULL) {
        (void)fprintf(stderr, "FAIL: %s [%s]: the table's event data could not be held\n", group, write_lua);
        return 1;
    }
    const bool event_read = read_table_event(&evt, name, sizeof(name), data, data_cap);
    if (event_read) {
        system_variable_writes_raise_external(sm, &evt);
        system_variable_writes_run(sm);
    }
    free(data);
    if (!event_read) {
        return 1;
    }
    if (!bound_by_the_binder(L, "_event") || !bound_event_named(L, name)) {
        (void)fprintf(stderr, "FAIL: %s [%s]: the machine bound no read-only `_event` named `%s`\n", group, write_lua,
                      name);
        return 1;
    }

    // A write that does not compile is a case that cannot be asked, not a write
    // that was refused: only a write that RAN and failed is a refusal.
    if (luaL_loadstring(L, write_lua) != LUA_OK) {
        (void)fprintf(stderr, "FAIL: %s [%s]: does not compile: %s\n", group, write_lua, lua_tostring(L, -1));
        lua_pop(L, 1);
        return 1;
    }
    const bool write_failed = lua_pcall(L, 0, 0, 0) != LUA_OK;
    if (write_failed != refused) {
        (void)fprintf(stderr, "FAIL: %s [%s]: the write %s (%s), %s says it is %s\n", group, write_lua,
                      write_failed ? "failed" : "went through",
                      write_failed && lua_tostring(L, -1) != NULL ? lua_tostring(L, -1) : "no error", clause,
                      refused ? "refused" : "not refused");
        failed = 1;
    }
    if (write_failed) {
        lua_pop(L, 1);
    }

    char chunk[FIELD_CAP + 16];
    (void)snprintf(chunk, sizeof(chunk), "return (%s)", after_lua);
    if (luaL_loadstring(L, chunk) != LUA_OK || lua_pcall(L, 0, 1, 0) != LUA_OK) {
        (void)fprintf(stderr, "FAIL: %s [%s]: `%s` did not evaluate: %s (%s)\n", group, write_lua, after_lua,
                      lua_tostring(L, -1), clause);
        lua_pop(L, 1);
        return 1;
    }
    asked++;
    if (!(lua_type(L, -1) == LUA_TBOOLEAN && lua_toboolean(L, -1) != 0)) {
        (void)fprintf(stderr, "FAIL: %s [%s]: afterwards `%s` answered a %s%s, not true (%s)\n", group, write_lua,
                      after_lua, luaL_typename(L, -1), lua_type(L, -1) == LUA_TBOOLEAN ? " false" : "", clause);
        failed = 1;
    }
    lua_pop(L, 1);
    return failed;
}

// One case in a machine of its own, so that no write an earlier case made — a
// refusal that did not hold, or a variable of the document's own — is there to
// be read.
static int check_case(const char *object, int index) {
    sm_t sm;
    REQUIRE_STARTED(system_variable_writes_init(&sm), SYSTEM_VARIABLE_WRITES_START_OK);
    if (sm.L == NULL) {
        (void)fprintf(stderr, "FAIL: case %d: the machine opened no Lua state, so it binds no system variable\n",
                      index);
        system_variable_writes_destroy(&sm);
        return 1;
    }
    const int failed = answer_case(&sm, object, index);
    system_variable_writes_destroy(&sm);
    return failed;
}

// docs/adr/0045: a machine its host starts under an id binds `_sessionid` to that
// id, read-only like the drawn one: the id reaches the Lua state before it opens,
// so the first script already reads it. A drawn id would read here as
// `system_variable_writes_session_<n>`.
static int check_given_session_id(void) {
    static const char GIVEN[] = "host-given.root";
    sm_t sm;
    const sce_start_refusal_t refused = system_variable_writes_init_with_session_id(&sm, GIVEN, NULL);
    if (refused != SCE_START_REFUSAL_NONE) {
        (void)fprintf(stderr, "FAIL: the machine refused the id `%s`: %s\n", GIVEN, sce_start_refusal_text(refused));
        return 1;
    }
    int failed = 0;
    if (sm.L == NULL || !bound_by_the_binder(sm.L, "_sessionid")) {
        (void)fprintf(stderr, "FAIL: a machine started under a given id bound no read-only `_sessionid`\n");
        failed = 1;
    } else if (strcmp(sm.session_id, GIVEN) != 0 || !bound_its_own_session_id(sm.L, &sm)) {
        (void)fprintf(stderr, "FAIL: `_sessionid` is not the id the host gave (`%s`, the machine holds `%s`)\n", GIVEN,
                      sm.session_id);
        failed = 1;
    }
    system_variable_writes_destroy(&sm);
    return failed;
}

int main(void) {
    if (check_given_session_id() != 0) {
        return 1;
    }
    char *table = read_table(SCE_SYSTEM_VARIABLE_WRITES_TABLE);
    if (table == NULL) {
        (void)fprintf(stderr, "FAIL: cannot read %s\n", SCE_SYSTEM_VARIABLE_WRITES_TABLE);
        return 1;
    }
    session_object = sce_payload_find_in(table, "session");
    event_object = sce_payload_find_in(table, "event");
    if (session_object == NULL || *session_object != '{' || event_object == NULL || *event_object != '{') {
        (void)fprintf(stderr, "FAIL: %s has no `session` or `event` object\n", SCE_SYSTEM_VARIABLE_WRITES_TABLE);
        free(table);
        return 1;
    }

    int failures = 0;
    const int cases = walk_table(SCE_SYSTEM_VARIABLE_WRITES_TABLE, check_case, &failures);
    free(table);
    // Every case the table holds was asked, and the table was read at all: a floor
    // on the walk, and an equality between what was walked and what was asked.
    if (cases < MIN_CASES || asked != cases) {
        (void)fprintf(stderr, "FAIL: %d case(s) in the table, %d asked, at least %d expected, all asked\n", cases,
                      asked, MIN_CASES);
        failures++;
    }
    if (failures != 0) {
        (void)fprintf(stderr, "%d of %d system-variable write case(s) disagree\n", failures, cases);
        return 1;
    }
    printf("PASS: %d system-variable write case(s) answered as the table says\n", cases);
    return 0;
}
