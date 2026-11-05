/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file send_target.h
 * @brief Where a `<send>` to the SCXML Event I/O Processor goes (§scxml-6.2.4, C.1)
 *
 * The C copy of the table C++ `SendHelper::classifyTarget` holds, and every
 * channel's: a `target` written in the document and a `targetexpr` evaluated
 * at run time are the same value and go to the same place, sent at once or
 * after a delay. Header-only and allocation-free: the id a target names is
 * returned as a pointer into the target itself, so a machine on an MCU links
 * nothing for it.
 */

#ifndef SCE_SEND_TARGET_H
#define SCE_SEND_TARGET_H

#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/types.h"

#ifdef __cplusplus
extern "C" {
#endif

/** The kinds a target value is read as. */
typedef enum {
    SCE_SEND_TARGET_SELF,        /**< the sending session's own external queue */
    SCE_SEND_TARGET_INTERNAL,    /**< `#_internal`: the sending session's internal queue */
    SCE_SEND_TARGET_PARENT,      /**< `#_parent`: the session that invoked this one */
    SCE_SEND_TARGET_SESSION,     /**< a session named by `#_scxml_<id>` or its published location */
    SCE_SEND_TARGET_INVOCATION,  /**< `#_<invokeid>`: an invocation of the sending session */
    SCE_SEND_TARGET_MESH,        /**< `#<name>`: a machine an SCE Mesh deployment binds */
    SCE_SEND_TARGET_UNREACHABLE, /**< a value naming no session: error.communication (C.1) */
    SCE_SEND_TARGET_UNSUPPORTED  /**< a value this processor cannot address: error.execution (6.2.4) */
} sce_send_target_kind_t;

/**
 * @brief Whether a target value names nothing (§scxml-C-1)
 *
 * An empty value, or the datamodel's no-value: ECMAScript's `undefined` and
 * `nil`, the spelling a Lua-lowered `undefined` reads back as. Such a target
 * reaches no one, so a send to it raises error.communication and delivers
 * nothing. The one place this is decided: the table below reads it, and so does
 * the BasicHTTP arm, which the table cannot serve because it holds an http(s)
 * URL to be no target of this processor.
 */
static inline bool sce_send_target_names_nothing(const char *target) {
    return target == NULL || target[0] == '\0' || strcmp(target, "undefined") == 0 || strcmp(target, "nil") == 0;
}

/**
 * @brief Classify a target value (§scxml-6.2.4, C.1)
 *
 * A URI of another scheme — an http(s) URL included, which only a BasicHTTP
 * send reaches — is SCE_SEND_TARGET_UNSUPPORTED, as it is for every generated
 * machine. A published location is `sce://scxml/<session id>`, the id
 * percent-encoded (`sce_scxml_location`), so the id it names is decoded into
 * `id_buf` by the inverse the encoder sits beside; a location naming an id
 * longer than `id_buf` holds names no session of this build, and reaches no one.
 *
 * @param target The target value, NUL-terminated
 * @param own_session_id The sending session, which a session target may name
 * @param id_buf Where the id a location names is decoded, `id_cap` bytes
 * @param id_out Set, for SESSION and INVOCATION, to the id `target` names:
 *        inside `target`, or in `id_buf` for a location
 */
static inline sce_send_target_kind_t sce_classify_send_target(const char *target, const char *own_session_id,
                                                              char *id_buf, size_t id_cap, const char **id_out) {
    static const char session_prefix[] = "#_scxml_";
    const char *id;
    *id_out = NULL;
    if (sce_send_target_names_nothing(target)) {
        return SCE_SEND_TARGET_UNREACHABLE;
    }
    if (target[0] == '!') {
        return SCE_SEND_TARGET_UNSUPPORTED;
    }
    if (strcmp(target, "#_internal") == 0) {
        return SCE_SEND_TARGET_INTERNAL;
    }
    if (strcmp(target, "#_parent") == 0) {
        return SCE_SEND_TARGET_PARENT;
    }
    if (strncmp(target, session_prefix, sizeof(session_prefix) - 1u) == 0) {
        id = target + sizeof(session_prefix) - 1u;
    } else if (strncmp(target, SCE_SCXML_LOCATION_PREFIX, sizeof(SCE_SCXML_LOCATION_PREFIX) - 1u) == 0 &&
               target[sizeof(SCE_SCXML_LOCATION_PREFIX) - 1u] != '\0') {
        if (!sce_session_id_from_scxml_location(id_buf, id_cap, target)) {
            return SCE_SEND_TARGET_UNREACHABLE;
        }
        id = id_buf;
    } else {
        id = NULL;
    }
    if (id != NULL) {
        if (id[0] == '\0' || (own_session_id != NULL && strcmp(id, own_session_id) == 0)) {
            return SCE_SEND_TARGET_SELF;
        }
        *id_out = id;
        return SCE_SEND_TARGET_SESSION;
    }
    if (target[0] == '#' && target[1] == '_') {
        *id_out = target + 2;
        return SCE_SEND_TARGET_INVOCATION;
    }
    if (target[0] == '#' && target[1] != '\0') {
        return SCE_SEND_TARGET_MESH;
    }
    return SCE_SEND_TARGET_UNSUPPORTED;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_SEND_TARGET_H */
