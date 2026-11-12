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

#ifdef __cplusplus
}
#endif

#endif /* SCE_SCRIPT_SESSION_H */
