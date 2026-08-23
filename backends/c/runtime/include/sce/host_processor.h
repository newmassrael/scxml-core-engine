// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Host-supplied Event I/O Processors — the payload types a host
// registers a handler for, and the bounded registry the generated
// machine dispatches through.
//
// The C11 port of `sce_rust_runtime::host_processor` and
// `sce/include/core/HostProcessor.h`. The three engines describe one
// host: a program that serves an act for the Rust engine must be
// portable to this one by translation rather than redesign, which is why
// the field names below are the same words in C spelling.
//
// The one shape that is NOT a translation is `params`. Its siblings key
// a map by param name and hold each name's values in document order; C
// has no map, and building one here would cost an allocator the MCU
// profile does not have. The flat ordered array below carries strictly
// MORE than the map does — the map loses the order BETWEEN names, this
// keeps it — so a handler written against either sibling can be ported,
// and one written against this one can answer questions they cannot.
//
// INTERFACE-only, like `sce/invoke.h`: the types and `static inline`
// helpers live here, and the per-machine dispatch is emitted into each
// generated `_sm.c` by `tools/codegen/templates/c/state_machine.c.jinja2`.

#ifndef SCE_HOST_PROCESSOR_H
#define SCE_HOST_PROCESSOR_H

#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/types.h"

#ifdef __cplusplus
extern "C" {
#endif

/* How many processor types one machine's host may serve. A document
   declares its types on the build command line, so the ceiling is known
   before the build runs; a machine declaring more overrides at build
   time (`-DSCE_MAX_HOST_PROCESSORS=N`). Four matches SCE_MAX_INVOKES,
   the sibling per-machine bound. */
#ifndef SCE_MAX_HOST_PROCESSORS
#define SCE_MAX_HOST_PROCESSORS 4
#endif

/* How many `<param>` values one host-served `<send>` may carry into the
   handler. The corpus maximum is 2 (`examples/ai_loop` carries one per
   act); eight leaves headroom for a namelist-shaped act. */
#ifndef SCE_MAX_HOST_SEND_PARAMS
#define SCE_MAX_HOST_SEND_PARAMS 8
#endif

/* How many events one act may report. Two is the shape that decided the
   list — `examples/ai_loop`'s `prompt.start` answers `prompt.sent` and
   then the turn result — and four leaves headroom without reserving a
   page of stack per send. */
#ifndef SCE_MAX_HOST_RESPONSES
#define SCE_MAX_HOST_RESPONSES 4
#endif

/* ── What the `<send>` said ─────────────────────────────────────── */

/**
 * One `<param>` as the document wrote it.
 *
 * Both fields point INTO the caller's frame and are valid only for the
 * duration of the handler call. A handler that needs a value afterwards
 * copies it — which is the same discipline the request itself carries,
 * and the reason nothing here owns memory.
 */
typedef struct sce_host_send_param_s {
    const char *name;
    const char *value;
} sce_host_send_param_t;

/**
 * What a `<send>` addressed to a host-served processor said.
 *
 * Every field is what the document wrote, not an interpretation of it: a
 * handler that wants to reject a malformed request needs to see the same
 * thing the author typed. Never NULL — a field the document did not
 * carry is the empty string, so a handler may read every field without
 * a null test and a missing value cannot be mistaken for a missing
 * field.
 */
typedef struct sce_host_send_request_s {
    /** The `type` this send named. Present even though the handler was
        looked up by it, because one handler may serve several types. */
    const char *processor_type;
    /** `<send event="...">`, or the value `eventexpr` evaluated to. */
    const char *event_name;
    /** `<send target="...">`, empty when the document named none. The
        specification leaves a target's meaning to the processor that
        serves it, so SCE passes it through uninterpreted. */
    const char *target;
    /** Inline `<content>`, empty when the document carried none. */
    const char *content;
    /** The send's id, auto-generated when the document declared none. A
        handler correlating a reply, or honouring a `<cancel>`, needs
        it. */
    const char *send_id;
    /** `<param>` values in document order, repeats included. */
    const sce_host_send_param_t *params;
    int param_count;
} sce_host_send_request_t;

/**
 * The first value the request carries for `name`, or NULL.
 *
 * The common read, spelled once here rather than in every host: a
 * handler asking for a name the send did not carry gets NULL and can say
 * so, which a linear scan open-coded per handler tends not to.
 */
static inline const char *sce_host_send_param(const sce_host_send_request_t *request, const char *name) {
    if (request == NULL || name == NULL) {
        return NULL;
    }
    for (int i = 0; i < request->param_count; i++) {
        if (strcmp(request->params[i].name, name) == 0) {
            return request->params[i].value;
        }
    }
    return NULL;
}

/* ── What the act produced ──────────────────────────────────────── */

/**
 * One event a host-served act produced.
 *
 * Value-typed rather than pointer-typed, unlike the request: the request
 * is read during the call and these outlive it, so a handler returning
 * pointers into its own frame would be handing the engine a dangling
 * read. The engine raises each on the EXTERNAL queue, which is where a
 * reply from outside the machine belongs.
 */
typedef struct sce_host_send_response_s {
    char event_name[SCE_MAX_ID_LEN];
    char event_data[SCE_MAX_DATA_LEN];
} sce_host_send_response_t;

/**
 * The ordered list a handler answers with.
 *
 * A list rather than a single reply for the reason its siblings give: an
 * act can produce two observations the document must see in a particular
 * order, and every other way of expressing that costs portability or
 * puts a pending slot back in the host.
 *
 * `overflowed` is the C-specific half. Its siblings grow a vector, so
 * they cannot run out of room; a bounded list can, and a report that
 * does not fit must not be quietly shortened — an act that produced two
 * events and delivered one leaves the machine somewhere the document
 * never described. The generated dispatch reads this flag and raises
 * `error.execution`, so the failure is loud and local rather than a
 * missing event nobody can trace.
 */
typedef struct sce_host_send_response_list_s {
    sce_host_send_response_t entries[SCE_MAX_HOST_RESPONSES];
    int count;
    bool overflowed;
} sce_host_send_response_list_t;

/**
 * Append one event to the report.
 *
 * Refuses rather than truncates, in both directions: a list already full
 * and a name or payload longer than this build reserved both leave the
 * entry unwritten and set `overflowed`. Truncating an event NAME would
 * be worse than dropping it — a shortened name can match a DIFFERENT
 * transition, so the machine would take a step the host never asked for.
 *
 * Returns whether the entry was stored, so a handler can stop early
 * rather than pushing into a list that is refusing.
 */
static inline bool sce_host_response_push(sce_host_send_response_list_t *out, const char *event_name,
                                          const char *event_data) {
    if (out == NULL || event_name == NULL) {
        return false;
    }
    if (out->count >= (int)SCE_MAX_HOST_RESPONSES) {
        out->overflowed = true;
        return false;
    }
    const char *data = (event_data != NULL) ? event_data : "";
    if (strlen(event_name) >= sizeof(out->entries[0].event_name) ||
        strlen(data) >= sizeof(out->entries[0].event_data)) {
        out->overflowed = true;
        return false;
    }
    sce_host_send_response_t *slot = &out->entries[out->count];
    memset(slot, 0, sizeof(*slot));
    strcpy(slot->event_name, event_name);
    strcpy(slot->event_data, data);
    out->count++;
    return true;
}

/**
 * What a host registers for one declared processor type.
 *
 * `user_data` is C's answer to the closure its siblings capture: a
 * handler needs the host it belongs to, and a function pointer alone
 * cannot carry one. Passed back unchanged on every call.
 *
 * A handler answers by pushing into `out`, which arrives empty. Pushing
 * nothing is "performed, nothing to report" — the common case for a
 * fire-and-forget act and for real work that will answer later through
 * the host's own loop.
 */
typedef void (*sce_host_send_handler_fn)(void *user_data, const sce_host_send_request_t *request,
                                         sce_host_send_response_list_t *out);

/* ── The registry ───────────────────────────────────────────────── */

typedef struct sce_host_processor_entry_s {
    char type[SCE_MAX_ID_LEN];
    sce_host_send_handler_fn handler;
    void *user_data;
} sce_host_processor_entry_t;

/**
 * Per-machine registry, embedded in the generated struct.
 *
 * Emitted only into a machine whose build declared a host processor, so
 * a machine without one pays nothing — the MCU profile is the reason
 * this backend exists, and an unconditional registry would add
 * `SCE_MAX_HOST_PROCESSORS * (SCE_MAX_ID_LEN + 2 pointers)` to every
 * generated struct in the corpus.
 */
typedef struct sce_host_processor_registry_s {
    sce_host_processor_entry_t entries[SCE_MAX_HOST_PROCESSORS];
    int count;
} sce_host_processor_registry_t;

/**
 * Register `handler` for `type`, replacing any handler already there.
 *
 * Replacement rather than a second entry: two handlers for one type
 * would make dispatch depend on registration order, and a host
 * re-registering during a run means to change what serves the act.
 *
 * Returns false when the registry is full or the type does not fit,
 * because a registration that did not happen must not read as one that
 * did — the send would then raise `error.execution` with nothing
 * naming why.
 */
static inline bool sce_host_registry_register(sce_host_processor_registry_t *registry, const char *type,
                                              sce_host_send_handler_fn handler, void *user_data) {
    if (registry == NULL || type == NULL || handler == NULL) {
        return false;
    }
    if (strlen(type) >= sizeof(registry->entries[0].type)) {
        return false;
    }
    for (int i = 0; i < registry->count; i++) {
        if (strcmp(registry->entries[i].type, type) == 0) {
            registry->entries[i].handler = handler;
            registry->entries[i].user_data = user_data;
            return true;
        }
    }
    if (registry->count >= (int)SCE_MAX_HOST_PROCESSORS) {
        return false;
    }
    sce_host_processor_entry_t *slot = &registry->entries[registry->count];
    memset(slot, 0, sizeof(*slot));
    strcpy(slot->type, type);
    slot->handler = handler;
    slot->user_data = user_data;
    registry->count++;
    return true;
}

/** The entry serving `type`, or NULL when nothing is registered. */
static inline const sce_host_processor_entry_t *sce_host_registry_find(const sce_host_processor_registry_t *registry,
                                                                       const char *type) {
    if (registry == NULL || type == NULL) {
        return NULL;
    }
    for (int i = 0; i < registry->count; i++) {
        if (strcmp(registry->entries[i].type, type) == 0) {
            return &registry->entries[i];
        }
    }
    return NULL;
}

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SCE_HOST_PROCESSOR_H */
