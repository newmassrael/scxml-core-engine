/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * Why a C11 machine's script session could not be established (docs/adr/0096).
 *
 * A machine whose document needs a datamodel runs its conditions, assignments
 * and scripts in a Lua state of its own. Every start establishes it in three
 * steps, in this order: create the state, load the shared library into it (the
 * standard libraries, the ECMAScript semantics, the JSON builtins and the
 * machine's own helpers), and bind the system variables `_sessionid`, `_name`
 * and `_ioprocessors`. When a step fails, the start is refused: nothing is
 * entered, nothing runs, the state the steps made is closed, and the machine
 * keeps the step and the engine's words here, where its host reads them through
 * `<machine>_script_session_failure`. The same failure every other backend hands
 * its host, and named by the same words.
 *
 * Below it, what a start of an invoked child is handed before it declares its
 * datamodel (docs/adr/0098).
 *
 * Header-only and allocation-free.
 */
#ifndef SCE_SCRIPT_SESSION_H
#define SCE_SCRIPT_SESSION_H

#include <stddef.h>

#include "sce/types.h"

#ifdef __cplusplus
extern "C" {
#endif

/* The bytes a failure keeps of the engine's words, its terminator included.
 *
 * Lua writes an error as the short name of the chunk that raised it, which it
 * bounds at `LUA_IDSIZE` (60 bytes in Lua 5.4's luaconf.h, the brackets and
 * quotes of `[string "..."]` included), then a colon, the line and a colon (a
 * dozen bytes for any chunk a machine loads), then the message, which for the
 * three steps is one clause: "not enough memory", a library chunk's runtime
 * error, "attempt to call a nil value (global '_scxml_bind_system')". 160 holds
 * that whole with room. It is not larger because every machine with a script
 * engine holds one, and so does every child slot of a parent: on a device
 * without a heap it is a cost of every session. Longer words are cut, never
 * overrun; the step, not the words, is what a host decides on. A build may set
 * its own. */
#ifndef SCE_MAX_SCRIPT_SESSION_MESSAGE_LEN
#define SCE_MAX_SCRIPT_SESSION_MESSAGE_LEN 160
#endif

/* The step that failed. `SCE_SCRIPT_SESSION_STAGE_NONE` is a machine whose last
 * start failed at none: it started, or it has no script engine to fail. */
typedef enum sce_script_session_stage_e {
    SCE_SCRIPT_SESSION_STAGE_NONE = 0,
    /* The Lua state could not be created. */
    SCE_SCRIPT_SESSION_STAGE_CREATE,
    /* The shared library did not load into the new state. A library that does
     * not load is a defect of the build that shipped it, or the memory ran out
     * while it loaded. */
    SCE_SCRIPT_SESSION_STAGE_LIBRARY,
    /* The system variables could not be bound. */
    SCE_SCRIPT_SESSION_STAGE_SYSTEM_VARIABLES
} sce_script_session_stage_t;

/* The step's name, the same word on every backend: "create", "library" or
 * "system-variables"; "" for none. */
SCE_C_UNUSED static inline const char *sce_script_session_stage_name(sce_script_session_stage_t stage) {
    switch (stage) {
    case SCE_SCRIPT_SESSION_STAGE_NONE:
        return "";
    case SCE_SCRIPT_SESSION_STAGE_CREATE:
        return "create";
    case SCE_SCRIPT_SESSION_STAGE_LIBRARY:
        return "library";
    case SCE_SCRIPT_SESSION_STAGE_SYSTEM_VARIABLES:
        return "system-variables";
    }
    return "";
}

/* A script session that could not be established: the step, and what the
 * engine said, NUL-terminated. */
typedef struct sce_script_session_failure_s {
    sce_script_session_stage_t stage;
    char message[SCE_MAX_SCRIPT_SESSION_MESSAGE_LEN];
} sce_script_session_failure_t;

/* Record that `stage` failed with the engine's `message` (NULL for none), cut
 * to the bytes the failure keeps. */
SCE_C_UNUSED static inline void sce_script_session_failure_set(sce_script_session_failure_t *failure,
                                                               sce_script_session_stage_t stage, const char *message) {
    failure->stage = stage;
    sce_copy_bounded_n(failure->message, message, sizeof(failure->message));
}

/* ── The values an invoke hands the child it starts (docs/adr/0098) ──────────
 *
 * A parent evaluates each `<param>` and reads each `namelist` name in its own
 * session, and hands the child every value it evaluated. The child's start
 * takes them after its session is established and before its datamodel is
 * declared: a top-level `<data>` whose id names a handed value is declared with
 * that value in place of its own, which is then never evaluated; every other
 * name is left out. The parent alone knows the values and the child alone knows
 * which of them it declares, so each decides its own half.
 *
 * The record allocates nothing. The parent keeps each value on its own Lua
 * stack, where it evaluated it, and lists where: the record and the list live
 * in the parent's invoke site, for as long as the child's start runs, and the
 * parent drops the values once the start returns. The child copies the values
 * it takes into its own session, whole: null, boolean, number, string, array
 * and object, the structure of an array or an object included. */
struct lua_State;

/* One value handed under `name`, at the absolute `index` of the parent's
 * stack. */
typedef struct sce_handed_value_s {
    const char *name;
    int index;
} sce_handed_value_t;

/* Every value an invoke hands one child: `count` of them in `values`, held on
 * the stack of the parent's session `from`. A start given NULL is handed
 * nothing, as a start by a host is. */
typedef struct sce_handed_values_s {
    struct lua_State *from;
    const sce_handed_value_t *values;
    size_t count;
} sce_handed_values_t;

/* How deep a handed value may nest: the most arrays and objects it may hold
 * one inside another, the outermost counted. The copy recurses once per level,
 * so this bounds the C stack the child's start spends on it, and a value that
 * holds itself is refused at this depth rather than copied for ever. A build
 * may set its own. */
#ifndef SCE_MAX_HANDED_VALUE_DEPTH
#define SCE_MAX_HANDED_VALUE_DEPTH 16
#endif

/* How many entries a handed value may hold, its arrays' elements and its
 * objects' members counted across every level. This bounds the work the
 * child's start does for one value. A build may set its own.
 *
 * A value past either bound, one of a kind no datamodel value is (a function,
 * a userdata), or one the child's session has no memory left for, cannot be
 * held: the `<data>` it was handed for fails to initialize, raises
 * error.execution in the child, and is left without a value, as a `<data>`
 * whose own expression failed is. */
#ifndef SCE_MAX_HANDED_VALUE_ENTRIES
#define SCE_MAX_HANDED_VALUE_ENTRIES 1024
#endif

#ifdef __cplusplus
}
#endif

#endif /* SCE_SCRIPT_SESSION_H */
