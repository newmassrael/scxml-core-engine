// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What an `_event` field the Processor left blank reads as, measured on the
// `_event` a generated C11 machine binds.
//
// W3C SCXML 5.10.1 tells the Processor to leave `sendid`, `origin`, `origintype`
// and `invokeid` blank in named cases, and B.2.8 says what blank is in the
// ECMAScript data model: undefined, never the empty string. This backend lowers
// ECMAScript to Lua, which holds undefined as nil. The expectations live in
// tests/ecmascript/event_blank_fields.json, which every backend's reader asks;
// each case is asked here in its `lua` spelling, because the machine evaluates
// Lua.
//
// This backend binds `_event` in the generated `set_current_event`, which is
// static in the machine, so a machine carries the question: each case gets one of
// its own (`event_blank_fields`, a document of this channel alone), the case's
// event is handed to it through the queue its `type` names, the machine takes it,
// and the case's Lua is evaluated in the state the machine bound it in. A member
// the case's `event` does not name is a field left blank, which the event record
// carries as the empty string.

#include <lauxlib.h>
#include <lua.h>
#include <lualib.h>

#include <stdio.h>
#include <string.h>

#include "../require_started.h"
#include "../unit/shared_table.h"
#include "event_blank_fields_sm.h"

#ifndef SCE_EVENT_BLANK_FIELDS_TABLE
#error "SCE_EVENT_BLANK_FIELDS_TABLE names tests/ecmascript/event_blank_fields.json; the build defines it"
#endif

enum { FIELD_CAP = 512, MIN_CASES = 11 };

typedef event_blank_fields_t sm_t;
typedef event_blank_fields_event_with_meta_t event_t;

// The cases whose Lua was evaluated, which `main` holds to the number the table
// holds: a case that was walked and never asked must not pass as one that agreed.
static int asked;

// One member of the case's `event` into `out`: the empty string when the event
// does not name it, which is a field the Processor left blank. False when the
// member is there and cannot be read whole, which is a case that cannot be asked.
static bool read_member(const char *event, const char *member, char *out, size_t cap) {
    out[0] = '\0';
    if (sce_payload_find_in(event, member) == NULL) {
        return true;
    }
    return read_field(event, member, out, cap, NULL) == NULL;
}

// The case's answer, refused unless `expect` is `{"bool": true}` or
// `{"bool": false}`: reading anything else as an answer would let a typo retire a
// case silently.
static bool read_expected(const char *object, int *expected) {
    const char *expect = sce_payload_find_in(object, "expect");
    const char *want = expect != NULL ? sce_payload_find_in(expect, "bool") : NULL;
    if (want == NULL) {
        return false;
    }
    if (strncmp(want, "true", 4) == 0) {
        *expected = 1;
        return true;
    }
    if (strncmp(want, "false", 5) == 0) {
        *expected = 0;
        return true;
    }
    return false;
}

// Hand the event to the queue its type names: an external event to the external
// queue, an internal or a platform one to the internal queue, where the machine's
// own dequeue gives `_event.type` the category (5.10.1).
static void deliver(sm_t *sm, const event_t *evt, const char *type) {
    if (strcmp(type, "external") == 0) {
        event_blank_fields_raise_external(sm, evt);
    } else {
        event_blank_fields_raise(sm, evt);
    }
    event_blank_fields_run(sm);
}

// Whether the machine bound `_event` for the event named `name`: the case's Lua
// asks one field of it, and a machine that took no event would answer from
// nothing.
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

// One case: the answer its Lua gives in the state of a machine that has just
// taken its event, against the answer the table holds.
static int answer_case(sm_t *sm, const char *object, int index) {
    char group[FIELD_CAP];
    char lua[FIELD_CAP];
    char clause[FIELD_CAP];
    int expected = 0;
    if (read_field(object, "group", group, FIELD_CAP, NULL) != NULL ||
        read_field(object, "lua", lua, FIELD_CAP, NULL) != NULL ||
        read_field(object, "clause", clause, FIELD_CAP, NULL) != NULL || !read_expected(object, &expected)) {
        (void)fprintf(stderr, "FAIL: case %d: no group, lua, clause or readable expectation\n", index);
        return 1;
    }

    const char *event = sce_payload_find_in(object, "event");
    char name[SCE_MAX_ID_LEN];
    char type[SCE_MAX_EVENT_TYPE_LEN];
    char sendid[SCE_MAX_ID_LEN];
    char invokeid[SCE_MAX_ID_LEN];
    char origin[SCE_MAX_URI_LEN];
    char origintype[SCE_MAX_ID_LEN];
    if (event == NULL || *event != '{' || read_field(event, "name", name, sizeof(name), NULL) != NULL ||
        read_field(event, "type", type, sizeof(type), NULL) != NULL ||
        !read_member(event, "sendid", sendid, sizeof(sendid)) ||
        !read_member(event, "invokeid", invokeid, sizeof(invokeid)) ||
        !read_member(event, "origin", origin, sizeof(origin)) ||
        !read_member(event, "origintype", origintype, sizeof(origintype))) {
        (void)fprintf(stderr, "FAIL: %s [%s]: the case's event cannot be read\n", group, lua);
        return 1;
    }

    event_t evt;
    memset(&evt, 0, sizeof(evt));
    if (!event_blank_fields_resolve_event_by_name(name, &evt.event)) {
        (void)fprintf(stderr, "FAIL: %s [%s]: the carrier has no event `%s`\n", group, lua, name);
        return 1;
    }
    sce_copy_bounded_event_type(evt.type, type);
    sce_copy_bounded_id(evt.send_id, sendid);
    sce_copy_bounded_id(evt.invoke_id, invokeid);
    if ((origin[0] != '\0' || origintype[0] != '\0') && !sce_event_origin_set(&evt.origin, origintype, origin)) {
        (void)fprintf(stderr, "FAIL: %s [%s]: the case's origin does not fit the event record\n", group, lua);
        return 1;
    }

    deliver(sm, &evt, type);
    lua_State *L = sm->L;
    if (!bound_event_named(L, name)) {
        (void)fprintf(stderr, "FAIL: %s [%s]: the machine bound no `_event` named `%s`\n", group, lua, name);
        return 1;
    }

    char chunk[FIELD_CAP + 8];
    (void)snprintf(chunk, sizeof(chunk), "return %s", lua);
    if (luaL_loadstring(L, chunk) != LUA_OK || lua_pcall(L, 0, 1, 0) != LUA_OK) {
        (void)fprintf(stderr, "FAIL: %s [%s]: did not evaluate: %s (%s)\n", group, lua, lua_tostring(L, -1), clause);
        lua_pop(L, 1);
        return 1;
    }
    asked++;
    const int failed = !(lua_type(L, -1) == LUA_TBOOLEAN && (lua_toboolean(L, -1) != 0) == expected);
    if (failed) {
        (void)fprintf(stderr, "FAIL: %s [%s]: answered a %s%s, %s says %s\n", group, lua, luaL_typename(L, -1),
                      lua_type(L, -1) == LUA_TBOOLEAN ? (lua_toboolean(L, -1) != 0 ? " true" : " false") : "", clause,
                      expected ? "true" : "false");
    }
    lua_pop(L, 1);
    return failed;
}

// One case in a machine of its own, so that no field an earlier case's event
// carried is there to be read.
static int check_case(const char *object, int index) {
    sm_t sm;
    REQUIRE_STARTED(event_blank_fields_init(&sm), EVENT_BLANK_FIELDS_START_OK);
    if (sm.L == NULL) {
        (void)fprintf(stderr, "FAIL: case %d: the machine opened no Lua state, so it binds no `_event`\n", index);
        event_blank_fields_destroy(&sm);
        return 1;
    }
    const int failed = answer_case(&sm, object, index);
    event_blank_fields_destroy(&sm);
    return failed;
}

int main(void) {
    int failures = 0;
    const int cases = walk_table(SCE_EVENT_BLANK_FIELDS_TABLE, check_case, &failures);
    // Every case the table holds was asked, and the table was read at all: a floor
    // on the walk, and an equality between what was walked and what was asked.
    if (cases < MIN_CASES || asked != cases) {
        (void)fprintf(stderr, "FAIL: %d case(s) in the table, %d asked, at least %d expected, all asked\n", cases,
                      asked, MIN_CASES);
        failures++;
    }
    if (failures != 0) {
        (void)fprintf(stderr, "%d of %d blank-field case(s) disagree\n", failures, cases);
        return 1;
    }
    printf("PASS: %d blank-field case(s) read as the table says\n", cases);
    return 0;
}
