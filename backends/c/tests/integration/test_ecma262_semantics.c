// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What ECMA-262 says an expression answers, measured against the Lua state a
// generated C11 machine really opens (ARCHITECTURE.md, "ECMAScript Semantics
// (Single Source of Truth)").
//
// Every engine that claims to run a `datamodel="ecmascript"` document is held to
// tests/ecmascript/ecma262_semantics.json. C11 evaluates Lua, not ECMAScript:
// the translation happened at build time, so the Lua it runs for each case is
// the emission in tests/ecmascript/ecma262_emitted_lua.json, and what is left to
// measure here is this backend's own half: the library the generated bootstrap
// carries (`ecma_semantics.lua` and `json_builtins.lua`, split across several
// `luaL_dostring` calls to stay under the C99 string-literal limit), the Lua 5.4
// the backend links, and the C-side conversion of what a chunk returns.
//
// The Rust reader of the same table measures the chunks C11 embeds on the Rust
// Lua engine and says in its own header what it cannot: the glue between a
// `lua_State` and the C values. This is that glue. The state under test is the
// one `lua_init_engine` opens and loads in `<machine>_init`, not a state this
// test builds, so a chunk the bootstrap fails to load leaves the definitions
// absent here exactly as it does in a shipped machine (the generated code
// discards every result of those calls).
//
// The machine is a carrier only. Any `datamodel="ecmascript"` document opens that
// bootstrap, and `external_chain_is_bounded` is already generated for this
// target's sibling, so its sources are reused rather than a fixture added for the
// bootstrap alone (a stem under integration_resources/ is a seven-channel
// commitment, which nothing here is).
//
// Every case gets a machine of its own: it is initialised, the case runs in the
// state it opened, and the machine is destroyed before the next case. A case's
// `setup` assigns what it uses into globals, and in one state shared by the sweep a
// later case that assumed a name was undeclared would see an earlier case's
// assignment, and one that only passed because of that leftover would pass. With a
// state per case, each sees the bootstrap and nothing else. The Rust reader of the
// chunked embed shares one state across the sweep; this reader is the stricter one,
// at the cost of one machine initialisation per case.
//
// The expectations stay in the shared table. The emission carries only the Lua, and
// the two are held to the same source text case by case.

#include <lauxlib.h>
#include <lua.h>
#include <lualib.h>

#include <math.h>
#include <stdio.h>
#include <string.h>

#include "../unit/shared_table.h"
#include "external_chain_is_bounded_sm.h"

#ifndef SCE_ECMA262_SEMANTICS_TABLE
#error "SCE_ECMA262_SEMANTICS_TABLE names tests/ecmascript/ecma262_semantics.json; the build defines it"
#endif
#ifndef SCE_ECMA262_EMITTED_LUA_TABLE
#error "SCE_ECMA262_EMITTED_LUA_TABLE names tests/ecmascript/ecma262_emitted_lua.json; the build defines it"
#endif

enum { FIELD_CAP = 256, MAX_CASES = 256, MIN_CASES = 55 };

typedef struct {
    char source[FIELD_CAP];
    char setup[FIELD_CAP];
    char expression[FIELD_CAP];
} emission_t;

static emission_t emissions[MAX_CASES];
static int emission_count;

// One row of the emission, kept by position: the table and the emission list the
// same cases in the same order, and `check_case` holds each pair to one source.
static int load_emission(const char *object, int index) {
    if (index >= MAX_CASES) {
        (void)fprintf(stderr, "FAIL: the emission has more than %d cases\n", MAX_CASES);
        return 1;
    }
    emission_t *row = &emissions[index];
    const char *refusal = read_field(object, "source", row->source, FIELD_CAP, NULL);
    if (refusal == NULL) {
        refusal = read_field(object, "setup", row->setup, FIELD_CAP, NULL);
    }
    if (refusal == NULL) {
        refusal = read_field(object, "expression", row->expression, FIELD_CAP, NULL);
    }
    if (refusal != NULL) {
        (void)fprintf(stderr, "FAIL: emission %d: the walker refused it: %s\n", index, refusal);
        return 1;
    }
    emission_count = index + 1;
    return 0;
}

// Run `chunk` as a statement list in the state `L` of a machine.
static int run(lua_State *L, const char *chunk, char *why, size_t why_cap) {
    if (luaL_dostring(L, chunk) != LUA_OK) {
        (void)snprintf(why, why_cap, "%s", lua_tostring(L, -1));
        lua_pop(L, 1);
        return 0;
    }
    return 1;
}

// One case of ecma262_semantics.json, answered by the Lua its emission gives, in
// the state `L` of a machine this case alone has used.
static int answer_case(lua_State *L, const char *object, int index) {
    char source[FIELD_CAP];
    char clause[FIELD_CAP];
    const char *refusal = read_field(object, "source", source, FIELD_CAP, NULL);
    if (refusal == NULL) {
        refusal = read_field(object, "clause", clause, FIELD_CAP, NULL);
    }
    const char *expect = sce_payload_find_in(object, "expect");
    if (refusal != NULL || expect == NULL || index >= emission_count) {
        (void)fprintf(stderr, "FAIL: case %d: no source, clause, expectation or emission to hold it to\n", index);
        return 1;
    }
    const emission_t *row = &emissions[index];
    if (strcmp(source, row->source) != 0) {
        (void)fprintf(stderr, "FAIL: [%s]: the table and the emission are out of step (emission has [%s])\n", source,
                      row->source);
        return 1;
    }

    char why[FIELD_CAP];
    if (row->setup[0] != '\0' && !run(L, row->setup, why, sizeof(why))) {
        (void)fprintf(stderr, "FAIL: [%s]: setup did not run: %s\n", source, why);
        return 1;
    }
    char value[FIELD_CAP + 8];
    (void)snprintf(value, sizeof(value), "return %s", row->expression);
    if (luaL_loadstring(L, value) != LUA_OK || lua_pcall(L, 0, 1, 0) != LUA_OK) {
        (void)fprintf(stderr, "FAIL: [%s]: failed to evaluate %s: %s\n", source, row->expression, lua_tostring(L, -1));
        lua_pop(L, 1);
        return 1;
    }

    // The answer's type is the key the table spells, so a case cannot name two.
    int failed = 1;
    const char *want;
    if ((want = sce_payload_find_in(expect, "bool")) != NULL) {
        const int expected = strncmp(want, "true", 4) == 0;
        failed = !(lua_type(L, -1) == LUA_TBOOLEAN && (lua_toboolean(L, -1) != 0) == expected);
    } else if ((want = sce_payload_find_in(expect, "number")) != NULL) {
        const double expected = strtod(want, NULL);
        failed = !(lua_type(L, -1) == LUA_TNUMBER && fabs(lua_tonumber(L, -1) - expected) < 1e-9);
    } else if ((want = sce_payload_find_in(expect, "string")) != NULL) {
        char expected[FIELD_CAP];
        sce_payload_sink_t sink;
        memset(&sink, 0, sizeof(sink));
        sink.text = expected;
        sink.cap = FIELD_CAP;
        if (sce_payload_walk_text(want, &sink) == NULL) {
            expected[sink.len < FIELD_CAP ? sink.len : FIELD_CAP - 1u] = '\0';
            failed = !(lua_type(L, -1) == LUA_TSTRING && strcmp(lua_tostring(L, -1), expected) == 0);
        }
    } else if (sce_payload_find_in(expect, "empty") != NULL) {
        // `null` and `undefined` are one answer: the datamodel cannot tell an absent
        // property from a null one.
        failed = lua_type(L, -1) != LUA_TNIL;
    }
    if (failed) {
        (void)fprintf(stderr, "FAIL: [%s]: answered a %s, ECMAScript says otherwise (%s)\n", source,
                      luaL_typename(L, -1), clause);
    }
    lua_pop(L, 1);
    return failed;
}

// One case in a machine of its own: opened for this case, destroyed before the
// next, so that nothing an earlier case assigned is there to be read.
static int check_case(const char *object, int index) {
    external_chain_is_bounded_t sm;
    external_chain_is_bounded_init(&sm);
    if (sm.L == NULL) {
        (void)fprintf(stderr, "FAIL: case %d: the machine opened no Lua state, so there is no bootstrap to measure\n",
                      index);
        external_chain_is_bounded_destroy(&sm);
        return 1;
    }
    const int failed = answer_case(sm.L, object, index);
    external_chain_is_bounded_destroy(&sm);
    return failed;
}

int main(void) {
    int failures = 0;
    const int loaded = walk_table(SCE_ECMA262_EMITTED_LUA_TABLE, load_emission, &failures);
    const int cases = walk_table(SCE_ECMA262_SEMANTICS_TABLE, check_case, &failures);
    // A floor, not an equality: adding a case must not have to touch this number,
    // but a table that stopped being read must not pass either.
    if (cases < MIN_CASES || loaded != cases) {
        (void)fprintf(stderr, "FAIL: %d case(s) in the table, %d in the emission, at least %d expected, equal\n", cases,
                      loaded, MIN_CASES);
        failures++;
    }

    if (failures != 0) {
        return 1;
    }
    (void)printf("PASS: the C11 bootstrap answers %d ECMA-262 case(s) as the language does\n", cases);
    return 0;
}
