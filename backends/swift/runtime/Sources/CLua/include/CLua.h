// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The one C surface Swift sees of the Lua 5.4 engine in `third_party/lua`.
//
// Lua reports an error by `longjmp`, and a `longjmp` that crosses a Swift frame is undefined. So
// the raw Lua C API is not exported: every call that can raise runs under `lua_pcall` inside this
// target, and what reaches Swift is a status code and, for a failure, a message. The Lua headers
// stay private to the target (`headerSearchPath("lua")`).

#ifndef SCE_CLUA_H
#define SCE_CLUA_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// One Lua state with its own memory for the last message and for a string result.
typedef struct sce_lua sce_lua;

/// How a call ended. A syntax error is told apart from a run-time one because the generator
/// lowers every expression at build time: a syntax error at run time is a defect of the lowering.
typedef enum {
    SCE_LUA_OK = 0,
    SCE_LUA_SYNTAX = 1,
    SCE_LUA_RUNTIME = 2,
    SCE_LUA_MEMORY = 3,
} sce_lua_status;

typedef enum {
    SCE_LUA_NIL = 0,
    SCE_LUA_BOOLEAN = 1,
    SCE_LUA_INTEGER = 2,
    SCE_LUA_NUMBER = 3,
    SCE_LUA_STRING = 4,
    /// A table, function, userdata or thread: `bytes` names the Lua type.
    SCE_LUA_OTHER = 5,
} sce_lua_kind;

/// A value crossing the boundary. A boolean is `integer` 0 or 1. `bytes` (a string, or the type
/// name of an `SCE_LUA_OTHER`) belongs to the state and stays valid until its next call.
typedef struct {
    sce_lua_kind kind;
    int64_t integer;
    double number;
    const char *bytes;
    size_t length;
} sce_lua_value;

/// A new state with the standard libraries open, or NULL when there is no memory for it.
sce_lua *sce_lua_open(void);

/// Closes the state. NULL is accepted.
void sce_lua_close(sce_lua *state);

/// Loads `chunk` (text, never a precompiled chunk) and runs it, discarding what it returns.
/// `name` is the chunk name an error message quotes, such as `=guard`.
sce_lua_status sce_lua_execute(sce_lua *state, const char *chunk, size_t length, const char *name);

/// Evaluates `expression` as `return (<expression>)` and stores the first value in `out`.
sce_lua_status sce_lua_evaluate(sce_lua *state, const char *expression, size_t length, const char *name,
                                sce_lua_value *out);

/// Evaluates `expression` as `return (<expression>)` and gives the global `name` its first value,
/// whatever that is, a table included. The name is a string and never source text: a `<data id>` is
/// an XML name, and `screen-rules` is not a Lua identifier but a subtraction.
sce_lua_status sce_lua_define_global(sce_lua *state, const char *name, const char *expression, size_t length,
                                     const char *chunk_name);

/// Gives the global `name` the value `value`, which is not of kind `SCE_LUA_OTHER`.
sce_lua_status sce_lua_set_global(sce_lua *state, const char *name, const sce_lua_value *value);

/// Reads the global `name` into `out`; an unset global is `SCE_LUA_NIL`.
sce_lua_status sce_lua_get_global(sce_lua *state, const char *name, sce_lua_value *out);

/// The host's answer to `In(state_id)`: nonzero when the state named by the `length` bytes at
/// `state_id` is active. It runs inside a Lua call, so it must not raise, and it must not call into
/// the same state.
typedef int (*sce_lua_state_query)(void *context, const char *state_id, size_t length);

/// Gives the session the global function `In`, which asks `query` with `context`; a NULL `query`
/// removes `In`. A call to `In` with an argument that is not a string raises in Lua, before the
/// host is asked.
sce_lua_status sce_lua_set_state_query(sce_lua *state, sce_lua_state_query query, void *context);

/// The message of the last call that did not end in `SCE_LUA_OK`, valid until the next call.
const char *sce_lua_message(const sce_lua *state, size_t *length);

#ifdef __cplusplus
}
#endif

#endif
