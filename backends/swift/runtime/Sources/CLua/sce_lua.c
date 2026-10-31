// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The shim `CLua.h` declares, over the Lua 5.4 sources in `third_party/lua`.
//
// Lua raises an error by `longjmp` to the nearest protected call. A protected call whose frames
// are all C is safe; one with a Swift frame between the raise and the catch is not. Every call
// here that can raise (an allocation, a metamethod on `_G`, the standard libraries opening) is
// therefore either `lua_pcall` itself or runs as the body of one, and a loader that reports its
// own failure (`lua_load`) is called as it is.

#include "CLua.h"

#include <stdlib.h>
#include <string.h>

#include "lauxlib.h"
#include "lua.h"
#include "lualib.h"

struct sce_lua {
    lua_State *L;
    char *message;
    size_t message_length;
    size_t message_capacity;
    char *scratch;
    size_t scratch_capacity;
    sce_lua_state_query query;
    void *query_context;
};

static sce_lua_status status_of(int lua_status) {
    switch (lua_status) {
    case LUA_OK:
        return SCE_LUA_OK;
    case LUA_ERRSYNTAX:
        return SCE_LUA_SYNTAX;
    case LUA_ERRMEM:
        return SCE_LUA_MEMORY;
    default:
        return SCE_LUA_RUNTIME;
    }
}

/// Copies `text` into the buffer `*buffer` of `*capacity` bytes, growing it; NULL when there is no
/// memory, with the buffer as it was.
static const char *copy_into(char **buffer, size_t *capacity, const char *text, size_t length) {
    if (length + 1 > *capacity) {
        char *grown = realloc(*buffer, length + 1);
        if (grown == NULL) {
            return NULL;
        }
        *buffer = grown;
        *capacity = length + 1;
    }
    memcpy(*buffer, text, length);
    (*buffer)[length] = '\0';
    return *buffer;
}

static void remember(sce_lua *state, const char *text, size_t length) {
    const char *kept = copy_into(&state->message, &state->message_capacity, text, length);
    state->message_length = kept == NULL ? 0 : length;
}

/// Takes the error object off the top of the stack as the message. A string is read in place; any
/// other object is named, because converting a number to text allocates and so can raise.
static void remember_top(sce_lua *state) {
    lua_State *L = state->L;
    size_t length = 0;
    const char *text = NULL;
    if (lua_type(L, -1) == LUA_TSTRING) {
        text = lua_tolstring(L, -1, &length);
    } else {
        text = "the error object is not a string";
        length = strlen(text);
    }
    remember(state, text, length);
    lua_pop(L, 1);
}

/// Reads the value at `index` into `out` without raising; -1 when a string result has no memory.
static int read_value(sce_lua *state, int index, sce_lua_value *out) {
    lua_State *L = state->L;
    const char *text = NULL;
    size_t length = 0;
    memset(out, 0, sizeof *out);
    switch (lua_type(L, index)) {
    case LUA_TNIL:
    case LUA_TNONE:
        out->kind = SCE_LUA_NIL;
        return 0;
    case LUA_TBOOLEAN:
        out->kind = SCE_LUA_BOOLEAN;
        out->integer = lua_toboolean(L, index) ? 1 : 0;
        return 0;
    case LUA_TNUMBER:
        if (lua_isinteger(L, index)) {
            out->kind = SCE_LUA_INTEGER;
            out->integer = (int64_t)lua_tointeger(L, index);
        } else {
            out->kind = SCE_LUA_NUMBER;
            out->number = (double)lua_tonumber(L, index);
        }
        return 0;
    case LUA_TSTRING:
        text = lua_tolstring(L, index, &length);
        out->kind = SCE_LUA_STRING;
        break;
    default:
        text = lua_typename(L, lua_type(L, index));
        length = strlen(text);
        out->kind = SCE_LUA_OTHER;
        break;
    }
    out->bytes = copy_into(&state->scratch, &state->scratch_capacity, text, length);
    if (out->bytes == NULL) {
        memset(out, 0, sizeof *out);
        return -1;
    }
    out->length = length;
    return 0;
}

/// Runs `f` with `job` as its one argument under `lua_pcall`.
static sce_lua_status call_protected(sce_lua *state, lua_CFunction f, void *job) {
    lua_State *L = state->L;
    int lua_status;
    lua_pushcfunction(L, f);
    lua_pushlightuserdata(L, job);
    lua_status = lua_pcall(L, 1, 0, 0);
    if (lua_status != LUA_OK) {
        remember_top(state);
    }
    return status_of(lua_status);
}

/// Loads `chunk` as text and runs it for `results` values, which are left on the stack.
static sce_lua_status run_chunk(sce_lua *state, const char *chunk, size_t length, const char *name, int results) {
    lua_State *L = state->L;
    int lua_status = luaL_loadbufferx(L, chunk, length, name, "t");
    if (lua_status == LUA_OK) {
        lua_status = lua_pcall(L, 0, results, 0);
    }
    if (lua_status != LUA_OK) {
        remember_top(state);
    }
    return status_of(lua_status);
}

static int open_libraries(lua_State *L) {
    luaL_openlibs(L);
    return 0;
}

sce_lua *sce_lua_open(void) {
    sce_lua *state = calloc(1, sizeof *state);
    if (state == NULL) {
        return NULL;
    }
    state->L = luaL_newstate();
    if (state->L == NULL) {
        free(state);
        return NULL;
    }
    if (call_protected(state, open_libraries, NULL) != SCE_LUA_OK) {
        sce_lua_close(state);
        return NULL;
    }
    return state;
}

void sce_lua_close(sce_lua *state) {
    if (state == NULL) {
        return;
    }
    if (state->L != NULL) {
        lua_close(state->L);
    }
    free(state->message);
    free(state->scratch);
    free(state);
}

sce_lua_status sce_lua_execute(sce_lua *state, const char *chunk, size_t length, const char *name) {
    return run_chunk(state, chunk, length, name, 0);
}

sce_lua_status sce_lua_evaluate(sce_lua *state, const char *expression, size_t length, const char *name,
                                sce_lua_value *out) {
    static const char prefix[] = "return (";
    static const char suffix[] = "\n)";
    size_t prefix_length = sizeof prefix - 1;
    size_t suffix_length = sizeof suffix - 1;
    char *chunk = malloc(prefix_length + length + suffix_length);
    sce_lua_status status;
    memset(out, 0, sizeof *out);
    if (chunk == NULL) {
        remember(state, "not enough memory", strlen("not enough memory"));
        return SCE_LUA_MEMORY;
    }
    memcpy(chunk, prefix, prefix_length);
    memcpy(chunk + prefix_length, expression, length);
    memcpy(chunk + prefix_length + length, suffix, suffix_length);
    status = run_chunk(state, chunk, prefix_length + length + suffix_length, name, 1);
    free(chunk);
    if (status != SCE_LUA_OK) {
        return status;
    }
    if (read_value(state, -1, out) != 0) {
        lua_pop(state->L, 1);
        remember(state, "not enough memory", strlen("not enough memory"));
        return SCE_LUA_MEMORY;
    }
    lua_pop(state->L, 1);
    return SCE_LUA_OK;
}

typedef struct {
    const char *name;
    const sce_lua_value *value;
} set_job;

static int set_global_protected(lua_State *L) {
    set_job *job = (set_job *)lua_touserdata(L, 1);
    const sce_lua_value *value = job->value;
    switch (value->kind) {
    case SCE_LUA_NIL:
        lua_pushnil(L);
        break;
    case SCE_LUA_BOOLEAN:
        lua_pushboolean(L, value->integer != 0);
        break;
    case SCE_LUA_INTEGER:
        lua_pushinteger(L, (lua_Integer)value->integer);
        break;
    case SCE_LUA_NUMBER:
        lua_pushnumber(L, (lua_Number)value->number);
        break;
    case SCE_LUA_STRING:
        lua_pushlstring(L, value->bytes, value->length);
        break;
    default:
        return luaL_error(L, "a value of kind %d cannot be given to a global", (int)value->kind);
    }
    lua_setglobal(L, job->name);
    return 0;
}

sce_lua_status sce_lua_set_global(sce_lua *state, const char *name, const sce_lua_value *value) {
    set_job job = {name, value};
    return call_protected(state, set_global_protected, &job);
}

typedef struct {
    sce_lua *state;
    const char *name;
    sce_lua_value *out;
    int exhausted;
} get_job;

static int get_global_protected(lua_State *L) {
    get_job *job = (get_job *)lua_touserdata(L, 1);
    lua_getglobal(L, job->name);
    job->exhausted = read_value(job->state, -1, job->out);
    return 0;
}

sce_lua_status sce_lua_get_global(sce_lua *state, const char *name, sce_lua_value *out) {
    get_job job = {state, name, out, 0};
    sce_lua_status status;
    memset(out, 0, sizeof *out);
    status = call_protected(state, get_global_protected, &job);
    if (status == SCE_LUA_OK && job.exhausted != 0) {
        remember(state, "not enough memory", strlen("not enough memory"));
        return SCE_LUA_MEMORY;
    }
    return status;
}

/// `In(state_id)`. The argument is checked before the host is asked: a bad argument raises, and a
/// raise while the host's frame is live would jump across it.
static int in_predicate(lua_State *L) {
    sce_lua *state = (sce_lua *)lua_touserdata(L, lua_upvalueindex(1));
    size_t length = 0;
    const char *state_id = luaL_checklstring(L, 1, &length);
    int active = state->query != NULL && state->query(state->query_context, state_id, length) != 0;
    lua_pushboolean(L, active);
    return 1;
}

typedef struct {
    sce_lua *state;
    int install;
} query_job;

static int install_in(lua_State *L) {
    query_job *job = (query_job *)lua_touserdata(L, 1);
    if (job->install) {
        lua_pushlightuserdata(L, job->state);
        lua_pushcclosure(L, in_predicate, 1);
    } else {
        lua_pushnil(L);
    }
    lua_setglobal(L, "In");
    return 0;
}

sce_lua_status sce_lua_set_state_query(sce_lua *state, sce_lua_state_query query, void *context) {
    query_job job = {state, query != NULL};
    state->query = query;
    state->query_context = context;
    return call_protected(state, install_in, &job);
}

const char *sce_lua_message(const sce_lua *state, size_t *length) {
    if (state->message == NULL) {
        *length = 0;
        return "";
    }
    *length = state->message_length;
    return state->message;
}
