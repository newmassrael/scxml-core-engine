// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
#include <stdint.h>
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
 * A value as the code that holds it lends it: the name of its C type, where it
 * is, and its size in bytes.
 *
 * C has no `std::any` or `Any` to hold a value of any type, and a box would
 * need storage this backend does not allocate, so a value crosses as a view.
 * `type` is the C name of the value's type exactly as the code spells it
 * (`JobStatusPayload_t`), a string of static storage, by which a reader of the
 * value tells one type from another (`SCE_HOST_VALUE` writes it with the
 * pointer, so the two cannot disagree); `data` points into the lender's frame
 * and is valid for the call it is handed to, as every pointer of a request is.
 * A reader that keeps the value copies its `size` bytes, which holds the
 * values the C11 backend writes in a value of fixed layout.
 *
 * No value is `type` NULL, `data` NULL and `size` 0 (`sce_host_value_none`).
 */
typedef struct sce_host_value_s {
    const char *type;
    const void *data;
    size_t size;
} sce_host_value_t;

/** The view of the value `*pointer`, of the C type `Type`. */
#define SCE_HOST_VALUE(Type, pointer) sce_host_value_of(#Type, (pointer), sizeof(Type))

/** The view of the `size` bytes at `data`, a value of the C type named `type`. */
static inline sce_host_value_t sce_host_value_of(const char *type, const void *data, size_t size) {
    sce_host_value_t value;
    value.type = type;
    value.data = data;
    value.size = size;
    return value;
}

/** No value: what every request carries but a test-port send's. */
static inline sce_host_value_t sce_host_value_none(void) {
    return sce_host_value_of(NULL, NULL, 0u);
}

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
    /** The event's data exactly as a local delivery of this send would
        carry it in `_event.data` (§scxml-5.10): the namelist and
        `<param>` pairs as JSON, or the `<content>`, or empty. Computed once
        by the generated send site, so a host that forwards the event — the
        Mesh router puts it in an envelope — forwards the engine's
        serialisation rather than a second one made from `params`. */
    const char *event_data;
    /** `_event.invokeid` of the event being processed when the `<send>`
        executed, empty when it had none. A send made while handling an
        event carries its invokeid back out, as a W3C child's send to its
        parent does (§scxml-6.4.1); a Mesh router matches the reply a
        document sends to the request it received by nothing else
        (SCE_MESH.md §mesh-10.7). Taken when the send executes, so a delayed
        one carries the invokeid current then, not when it fires. */
    const char *invoke_id;
    /** The message as the value the generated code holds, for a processor
        whose messages are values and never encodings: the test profile's
        test-port processor, whose send carries its specific template's value,
        a record of the type the event schema's unit declares
        (docs/SCE_TEST_PROFILE.md §12.3; docs/adr/0091, C2). No value
        (`sce_host_value_none`) for every other send, whose data is
        `event_data`. A saved state does not carry it: a test-port send has no
        delay (the test profile's P6), so none is pending when a machine is
        saved, and a delayed host send is one of another processor. */
    sce_host_value_t value;
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
    /* `_event.origin` and `_event.origintype` of the reply, in the host's own
       terms (docs/adr/0090, section 2): the host processor's type and the
       address a `<send>` of that type reaches the party at. None when the reply
       names no party (`sce_host_response_push`), and then both fields read
       empty; the engine never fills in the SCXML processor's type for a host. */
    sce_event_origin_t origin;
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
 * Append one event that names its origin (docs/adr/0090, section 2): the
 * pair the host writes for the party the reply comes from, in its own terms,
 * or NULL for none, which is what `sce_host_response_push` appends.
 *
 * Refuses as `sce_host_response_push` refuses, and returns what it returns.
 */
static inline bool sce_host_response_push_from(sce_host_send_response_list_t *out, const char *event_name,
                                               const char *event_data, const sce_event_origin_t *origin) {
    if (!sce_host_response_push(out, event_name, event_data)) {
        return false;
    }
    if (origin != NULL) {
        out->entries[out->count - 1].origin = *origin;
    }
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

/* ── The answer schemas a machine names (docs/adr/0087, section 4.1) ── */

/*
 * One schema of a machine's table of answer schemas: the schemas its invokes
 * of `sce-host-model-generate` name with `sce:answer`, which a generated
 * machine hands out by `<machine>_answer_schemas()`. A host resolves the
 * `answer_schema` of a request through it, with no file and no second copy of
 * a grammar, and hands its provider the form it takes. Every text is
 * NUL-terminated constant storage of the machine's, alive for the program.
 */
typedef struct sce_answer_schema_s {
    /* `urn:sce:schema:<document name>:<revision>`: what the request carries. */
    const char *id;
    /* The payload as a JSON Schema (2020-12), every bound as the build
       resolved it. */
    const char *json_schema;
    /* The payload as a GBNF grammar, every text's bound counted in bytes. */
    const char *gbnf;
} sce_answer_schema_t;

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
/* The router a child session is given by its parent; defined with the
   invoker types below, which it carries. */
struct sce_child_mesh_router_s;

/**
 * The factory of the routers of a machine's children (SCE_MESH.md, Child
 * Sessions in Mesh; docs/adr/0043): asked, each time an invocation starts a
 * child, with the invoke's id and the stem of the child's document (empty for a
 * child written inline). It fills `out` and answers true to give the child a
 * router, or answers false to leave the child with none, whose peer sends then
 * raise `error.execution` in the child.
 */
typedef bool (*sce_child_mesh_routers_fn)(void *user_data, const char *invoke_id, const char *document,
                                          struct sce_child_mesh_router_s *out);

/** A registered factory of children's routers and the pointer it is called with. */
typedef struct sce_child_mesh_routers_s {
    sce_child_mesh_routers_fn ask;
    void *user_data;
} sce_child_mesh_routers_t;

typedef struct sce_host_processor_registry_s {
    sce_host_processor_entry_t entries[SCE_MAX_HOST_PROCESSORS];
    int count;
    /* The factory of the routers of the machine's children, beside the router
       the machine's own sends reach: `sce_host_registry_register_child_mesh_routers`. */
    sce_child_mesh_routers_t child_mesh_routers;
} sce_host_processor_registry_t;

/**
 * The prefix SCE keeps for the Event I/O Processors and invoke types it
 * defines itself: `SCE_MESH_PROCESSOR_TYPE` and `sce:mesh-rpc`.
 *
 * §scxml-6.2.5 leaves the processor set open to the platform, and SCE shares
 * that namespace with its host. A handler registered under `sce:` would not
 * add a processor but replace one of SCE's, with nothing on the wire saying
 * so — so registration refuses it, as the build refuses the same declaration
 * (`cli/reserved-host-type`).
 */
#define SCE_RESERVED_HOST_TYPE_PREFIX "sce:"

/**
 * The `<send type>` a Mesh send is lowered to on every backend whose Mesh
 * router the host registers — see `sce_host_registry_register_mesh_router`.
 */
#define SCE_MESH_PROCESSOR_TYPE "sce:mesh"

/**
 * The `<invoke type>` a Mesh request is lowered to on every backend whose
 * Mesh router the host registers — see
 * `sce_host_invoker_register_mesh_rpc_invoker`.
 */
#define SCE_MESH_RPC_INVOKE_TYPE "sce:mesh-rpc"

/**
 * Whether a host may not register `type`: it starts with
 * `SCE_RESERVED_HOST_TYPE_PREFIX`, spelled exactly.
 *
 * The C11 copy of the rule the build applies to a declaration. Every copy
 * reads `sce-build/tests/fixtures/host_processor/reserved_type_cases.json`, so
 * a type one of them refuses is one they all refuse.
 */
static inline bool sce_is_reserved_host_type(const char *type) {
    return type != NULL && strncmp(type, SCE_RESERVED_HOST_TYPE_PREFIX, sizeof(SCE_RESERVED_HOST_TYPE_PREFIX) - 1) == 0;
}

/**
 * Whether a `<send>` target names a Mesh peer (SCE_MESH.md §mesh-19): `#`
 * followed by at least one character that is not `_`, which stays W3C's
 * (§scxml-6.2.4 `#_internal`, `#_parent`, `#_<invokeid>`).
 *
 * The C11 copy of the rule, asked of a `targetexpr` once it is evaluated —
 * the build decides the literal targets itself. Every copy reads
 * `tests/mesh/mesh_target_cases.json`, so a target one of them routes over
 * Mesh is one they all route.
 */
static inline bool sce_is_mesh_target(const char *target) {
    return target != NULL && target[0] == '#' && target[1] != '\0' && target[1] != '_';
}

/**
 * Put `handler` in the slot for `type`, replacing any handler already there.
 *
 * The one body behind `sce_host_registry_register` and
 * `sce_host_registry_register_mesh_router`, which differ only in which types
 * they accept.
 */
static inline bool sce_host_registry_put(sce_host_processor_registry_t *registry, const char *type,
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

/**
 * Register `handler` for `type`, replacing any handler already there.
 *
 * Replacement rather than a second entry: two handlers for one type
 * would make dispatch depend on registration order, and a host
 * re-registering during a run means to change what serves the act.
 *
 * Returns false when the registry is full, the type does not fit, or the
 * type is under `SCE_RESERVED_HOST_TYPE_PREFIX` (the Mesh router has its own
 * door, `sce_host_registry_register_mesh_router`), because a registration
 * that did not happen must not read as one that did — the send would then
 * raise `error.execution` with nothing naming why.
 */
static inline bool sce_host_registry_register(sce_host_processor_registry_t *registry, const char *type,
                                              sce_host_send_handler_fn handler, void *user_data) {
    if (sce_is_reserved_host_type(type)) {
        return false;
    }
    return sce_host_registry_put(registry, type, handler, user_data);
}

/**
 * Register `router` as the Event I/O Processor Mesh sends reach — the build
 * lowers a `<send target="#peer">` to `<send type="sce:mesh">` and reports
 * `needs_mesh_router` on its manifest when the machine has one.
 *
 * The one way to serve `SCE_MESH_PROCESSOR_TYPE`: `sce_host_registry_register`
 * refuses the reserved prefix, so a router is only ever installed by a call
 * that names itself one. Unregistered, a Mesh send raises `error.execution` as
 * any host processor without a handler does; a registered router that cannot
 * reach the peer raises `error.communication` itself.
 */
static inline bool sce_host_registry_register_mesh_router(sce_host_processor_registry_t *registry,
                                                          sce_host_send_handler_fn router, void *user_data) {
    return sce_host_registry_put(registry, SCE_MESH_PROCESSOR_TYPE, router, user_data);
}

/**
 * Register the factory of the routers of the machine's children (SCE_MESH.md
 * §mesh-21.3, Child Sessions in Mesh; docs/adr/0043). A child session is given none of the
 * machine's registrations, since its peer sends would leave under the
 * machine's name; the generated parent asks `ask` each time an invocation
 * starts a child, and installs what it answers on the child before the child is
 * entered. Registered in the registry the machine is started with, so the
 * children of its initial configuration, started by `_init`, are asked too.
 */
static inline bool sce_host_registry_register_child_mesh_routers(sce_host_processor_registry_t *registry,
                                                                 sce_child_mesh_routers_fn ask, void *user_data) {
    if (registry == NULL) {
        return false;
    }
    registry->child_mesh_routers.ask = ask;
    registry->child_mesh_routers.user_data = user_data;
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

/* ── §scxml-6.4.1: `<invoke>` the HOST runs ─────────────────────── */
/*
 * The clause leaves the invokable set to the platform in the same words
 * §scxml-6.2.5 uses for `<send>`, so a host may implement its own `type`
 * here too — but an invoke is not a send. It has a LIFETIME: it starts
 * when the state is entered, it is cancelled if the state exits, and the
 * document may be waiting on `done.invoke.<id>`. That is why the handler
 * is told WHICH turn it is being called for, and why this is a second
 * registry rather than a second use of the first: a host that can deliver
 * an event is not thereby able to run a process it must also be able to
 * stop.
 *
 * The C11 port of the invoker half in `sce_rust_runtime::host_processor`,
 * `sce/include/core/HostProcessor.h`, `backends/go/runtime/host_processor.go`
 * and `backends/python/runtime/sce_runtime/host_processor.py`. The shape
 * differs in one way only, and it is the freestanding profile's: a tagged
 * struct instead of a variant, because C has no sum type and a pointer
 * pair would let a caller engage both arms.
 */

/** How many invoker types one machine may serve. */
#ifndef SCE_MAX_HOST_INVOKERS
#define SCE_MAX_HOST_INVOKERS 4
#endif

/** §scxml-6.3.1: what every `done.invoke.<id>` begins with, spelled once so
    the completion a host reports and the name the machine declares cannot
    disagree. */
#define SCE_DONE_INVOKE_PREFIX "done.invoke."

/** §scxml-3.12.1: the generic completion descriptor. A document that names
    no specific `done.invoke.<id>` matches every completion through it, so it
    is the event a completion falls back to. */
#define SCE_DONE_INVOKE_EVENT "done.invoke"

/** The event an invocation that did not finish raises instead of its
    completion — a host-run invocation past its `_sce_deadline_ms` — named the
    way `sce:mesh-rpc` names a request that ran out of time (SCE_MESH.md §9.5),
    so a document handles the two alike. Specific form: `error.invoke.<id>`. */
#define SCE_ERROR_INVOKE_PREFIX "error.invoke."

/** The generic form of SCE_ERROR_INVOKE_PREFIX, for a document that names no
    specific `error.invoke.<id>` — as SCE_DONE_INVOKE_EVENT is for a
    completion. */
#define SCE_ERROR_INVOKE_EVENT "error.invoke"

/** The reserved `<param>` a host-run `<invoke>` names its deadline with, in
    milliseconds. The machine reads it and does not hand it to the host: past
    the deadline, an invocation still running is cancelled and the document
    receives `error.invoke.<id>` instead of its completion. */
#define SCE_HOST_INVOKE_DEADLINE_PARAM "_sce_deadline_ms"

/** Read the text of a SCE_HOST_INVOKE_DEADLINE_PARAM value as milliseconds
    into `*out`, returning whether it is one.

    One or more ASCII digits, optionally followed by `.` and one or more `0` —
    a `<param expr>` reaches the request as the text of its value, and a script
    engine may render a whole number `5000.0` — within a signed 64-bit count.
    No sign, no whitespace, no exponent, no digit separator. Spelled out rather
    than left to `strtoull`, which skips leading whitespace and reads a sign,
    because every runtime implements this and one table holds them all:
    `sce-build/tests/fixtures/host_processor/host_invoke_deadline_values.json`. */
static inline bool sce_parse_host_invoke_deadline_ms(const char *written, uint64_t *out) {
    const uint64_t max = (uint64_t)INT64_MAX;
    uint64_t ms = 0u;
    const char *p = written;
    if (written == NULL || *p < '0' || *p > '9') {
        return false;
    }
    for (; *p >= '0' && *p <= '9'; ++p) {
        const uint64_t digit = (uint64_t)(*p - '0');
        if (ms > (max - digit) / 10u) {
            return false;
        }
        ms = ms * 10u + digit;
    }
    if (*p == '.') {
        ++p;
        if (*p != '0') {
            return false;
        }
        while (*p == '0') {
            ++p;
        }
    }
    if (*p != '\0') {
        return false;
    }
    *out = ms;
    return true;
}

/** How many host-run invocations may be in flight at once. Each generated
    machine asserts it covers the most its own configurations can hold. */
#ifndef SCE_MAX_HOST_INVOCATIONS
#define SCE_MAX_HOST_INVOCATIONS 4
#endif

/** Which turn of the lifecycle the handler is being called for. */
typedef enum sce_host_invoke_phase_e {
    /** §scxml-6.4: the state was entered and the macrostep has settled.
        Begin the invoked process. */
    SCE_HOST_INVOKE_START = 0,
    /** §scxml-6.4: the state exited. Stop it.

        Delivered only for an invocation that actually started: a state
        that exits before the macrostep ends never runs its invoke, and
        cancelling something that never began would have the host tearing
        down state it never built. */
    SCE_HOST_INVOKE_CANCEL = 1
} sce_host_invoke_phase_t;

/**
 * One turn of a host-run invoke's lifecycle.
 *
 * Every field is what the document wrote. On a cancel only
 * `processor_type` and `invoke_id` are meaningful — the rest is what the
 * start carried and is not re-read, because a cancel names an invocation
 * rather than describing one.
 *
 * Like the send request beside it, nothing here owns memory: a handler
 * that needs a value after it returns copies it.
 */
typedef struct sce_host_invoke_event_s {
    sce_host_invoke_phase_t phase;
    /** The `type` this `<invoke>` named. */
    const char *processor_type;
    /** The invoke's id (§scxml-6.4.1), auto-derived when the author
        declared none. This is the name the DOCUMENT waits on: a
        completion is `done.invoke.<invoke_id>`, so a host that finishes
        asynchronously must keep it. */
    const char *invoke_id;
    /** `<invoke src="...">`, empty when the document named none. SCE does
        not interpret it — what a src means is the invoked processor's
        business. */
    const char *src;
    /** Inline `<content>`, empty when the document carried none. */
    const char *content;
    /** The namelist and `<param>` pairs as JSON, typed as the data model
        holds them — the text a local `<send>` with the same pairs would
        carry in `_event.data` (§scxml-5.10) — or empty when there are none.
        Params the build marked as envelope metadata (SCE_MESH.md
        §mesh-9.5's `_mesh_*`) are left out. Computed once by the generated
        invoke site, so a host that forwards the request — the Mesh router
        puts it in an envelope — forwards the machine's serialisation rather
        than a second one made from `params`, which are text and have lost
        their types. NULL on a cancel. */
    const char *event_data;
    /** `<param>` values in document order, repeats included. Shares the
        send half's pair type: a name and a value is the same shape here,
        and two structs would be two spellings of one fact. */
    const sce_host_send_param_t *params;
    int param_count;
    /** Which start of this invoke this is, on both a start and a cancel.
        The engine assigns it, and a host that finishes later hands it back
        to the machine's `_complete_host_invoke`.

        The id alone cannot say it: a state that exits and is entered again
        starts the same `<invoke>` a second time under the same id, and a
        result the first run produces after it was cancelled would otherwise
        read as the second run's (§scxml-6.4 — once the state has exited,
        what the cancelled process sends is ignored). Distinct for every
        start of every invocation within one machine. */
    uint64_t token;
    /** The session that started the invocation (`_sessionid`; docs/adr/0045):
        the root the host built, or a child session of it,
        `<parent>.<invokeid>`. The invokers a host registers serve the whole
        tree of sessions, so a host that means to serve a type for part of it
        only refuses the others by this id. NULL on a cancel. */
    const char *session_id;
    /** Whether this start is a restore starting the invocation again: the
        machine saved while it ran, and the process the host began for it did
        not outlive the one that saved. What the host does with that is its
        business; a host that cannot tell a second start from a first does not
        read it. false on a cancel. */
    bool restarted;
} sce_host_invoke_event_t;

/**
 * A host invoker's answer to a start.
 *
 * Read only for a start; an answer to a cancel is ignored, because there
 * is nothing left for it to mean.
 */
typedef struct sce_host_invoke_response_s {
    /** Whether `done_data` below carries a completion.

        false is the ordinary case: the work outlives the call, and the
        host reports it through the machine's `_complete_host_invoke` with
        the start's token when it finishes. SCE does not synthesise a
        completion the host did not report — an invoked process that never
        terminates never fires `done.invoke`, which is what §scxml-6.4
        says. */
    bool has_done_data;
    /** Payload for an immediate `done.invoke.<invoke_id>`. Copied by the
        engine before this struct goes out of scope. */
    char done_data[SCE_MAX_DATA_LEN];
    /** Whether `refusal` below says the host could not start the invocation
        at all. Then `refusal` is `_event.data` for the `error.execution` the
        machine raises instead (§scxml-6.4.1 — an invocation that cannot
        be started is an error of the element, not an `error.invoke` of a
        process that ran): text, or JSON when the host reports a structured
        reason, as a Mesh router does with SCE_MESH.md §mesh-10.7.1's
        `{"reason":"INVOKE_SRC_NOT_FOUND", …}`. A refused invocation never
        started: its deadline is dropped, no cancel follows, and a completion
        reported for it later is refused. It overrides `has_done_data`, since
        nothing ran to complete. */
    bool has_refusal;
    char refusal[SCE_MAX_DATA_LEN];
} sce_host_invoke_response_t;

/**
 * What runs one declared invoke type.
 *
 * `user_data` is handed back unchanged — C's answer to the closure the
 * other engines capture. `out` is zero-initialised by the caller; a
 * handler with nothing to report leaves it alone.
 */
typedef void (*sce_host_invoke_handler_fn)(void *user_data, const sce_host_invoke_event_t *event,
                                           sce_host_invoke_response_t *out);

/** One registered invoker and the type it runs. */
typedef struct sce_host_invoker_entry_s {
    char type[SCE_MAX_ID_LEN];
    sce_host_invoke_handler_fn handler;
    void *user_data;
} sce_host_invoker_entry_t;

/**
 * The set of invoke types a host has registered handlers for.
 *
 * Bounded like its send sibling and for the same reason: the freestanding
 * profile has no allocator, so the ceiling is a build-time decision a
 * deployment can raise (`-DSCE_MAX_HOST_INVOKERS=N`).
 */
typedef struct sce_host_invoker_registry_s {
    sce_host_invoker_entry_t entries[SCE_MAX_HOST_INVOKERS];
    int count;
} sce_host_invoker_registry_t;

/**
 * Put `handler` under `type` with no reserved-prefix check: the one path
 * both the host's door and the runtime's own Mesh door share.
 */
static inline bool sce_host_invoker_put(sce_host_invoker_registry_t *registry, const char *type,
                                        sce_host_invoke_handler_fn handler, void *user_data) {
    int i;
    sce_host_invoker_entry_t *slot;
    if (registry == NULL || type == NULL || handler == NULL) {
        return false;
    }
    if (strlen(type) >= (size_t)SCE_MAX_ID_LEN) {
        return false;
    }
    for (i = 0; i < registry->count; i++) {
        if (strcmp(registry->entries[i].type, type) == 0) {
            registry->entries[i].handler = handler;
            registry->entries[i].user_data = user_data;
            return true;
        }
    }
    if (registry->count >= (int)SCE_MAX_HOST_INVOKERS) {
        return false;
    }
    slot = &registry->entries[registry->count];
    sce_copy_bounded_id(slot->type, type);
    slot->handler = handler;
    slot->user_data = user_data;
    registry->count++;
    return true;
}

/**
 * Register `handler` for `type`, replacing any handler already there.
 *
 * Returns false when the registry is full, the type does not fit its slot,
 * or the type is under `SCE_RESERVED_HOST_TYPE_PREFIX`, because a
 * registration that did not happen must not read as one that did.
 */
static inline bool sce_host_invoker_register(sce_host_invoker_registry_t *registry, const char *type,
                                             sce_host_invoke_handler_fn handler, void *user_data) {
    if (type == NULL || sce_is_reserved_host_type(type)) {
        return false;
    }
    return sce_host_invoker_put(registry, type, handler, user_data);
}

/**
 * SCE Mesh §9.5: register the Mesh router's request half as the runner of
 * `SCE_MESH_RPC_INVOKE_TYPE`, the type a Mesh `<invoke>` is lowered to.
 *
 * `sce:` is reserved so a host cannot claim it by accident; this is the one
 * door that fills it, the invoke twin of
 * `sce_host_registry_register_mesh_router`.
 */
static inline bool sce_host_invoker_register_mesh_rpc_invoker(sce_host_invoker_registry_t *registry,
                                                              sce_host_invoke_handler_fn router, void *user_data) {
    return sce_host_invoker_put(registry, SCE_MESH_RPC_INVOKE_TYPE, router, user_data);
}

/** The entry running `type`, or NULL when nothing is registered. */
static inline const sce_host_invoker_entry_t *sce_host_invoker_find(const sce_host_invoker_registry_t *registry,
                                                                    const char *type) {
    int i;
    if (registry == NULL || type == NULL) {
        return NULL;
    }
    for (i = 0; i < registry->count; i++) {
        if (strcmp(registry->entries[i].type, type) == 0) {
            return &registry->entries[i];
        }
    }
    return NULL;
}

/**
 * How a host ends a host-run invocation of some session of a tree, as one
 * value: the start it ends, and whether it completed (`done.invoke`) or failed
 * (`error.invoke`), with `data` as the event's `_event.data`.
 *
 * Everything the machine's completion and failure take, so an ending posted to
 * a machine's inbox from another context carries what a direct call on its
 * owner would have passed (docs/adr/0084). The C11 port of
 * `SCE::HostInvokeEnding` (`sce/include/core/HostProcessor.h`), with the
 * session it names beside it, and every string held rather than pointed at,
 * because a post copies the value into the inbox.
 */
typedef struct sce_host_invoke_ending_s {
    /** The session that started the invocation, as its request named it
        (`sce_host_invoke_event_t::session_id`); empty for the machine the
        ending is posted to. */
    char session_id[SCE_MAX_SESSION_ID_LEN];
    /** The `type` the request named. */
    char processor_type[SCE_MAX_ID_LEN];
    /** The invocation's id, as the request named it. */
    char invoke_id[SCE_MAX_ID_LEN];
    /** The token the request carried. */
    uint64_t token;
    /** True for a completion and false for a failure. */
    bool done;
    /** `_event.data` of the event the ending raises. */
    char data[SCE_MAX_DATA_LEN];
    /** `_event.origin` and `_event.origintype` of that event, for a host whose
        invocation stands for a party it can name, in the host's own terms
        (docs/adr/0090, section 2): its declared type and the address a
        `<send>` of that type reaches the party at. None (zeroed) for a host
        that names no party; the engine never fills in the SCXML processor. */
    sce_event_origin_t origin;
} sce_host_invoke_ending_t;

/**
 * Every host-run invocation started and neither completed nor cancelled,
 * with its start's token.
 *
 * Held by the generated machine but WALKED here, so "is this one still
 * running?" — the question both the cancel path and a completion have to
 * answer — is answered once rather than once per backend. Bounded for the
 * reason the registries are.
 *
 * One entry per `(type, invoke_id)`: an `<invoke>` belongs to one state and a
 * state is in the configuration at most once, so the same id is never
 * running twice. A restart replaces the entry's token, which is what makes
 * the first run's late reply stale.
 */
typedef struct sce_host_invocation_set_s {
    char types[SCE_MAX_HOST_INVOCATIONS][SCE_MAX_ID_LEN];
    char ids[SCE_MAX_HOST_INVOCATIONS][SCE_MAX_ID_LEN];
    uint64_t tokens[SCE_MAX_HOST_INVOCATIONS];
    int count;
    /** The token the next start receives. */
    uint64_t next_token;
} sce_host_invocation_set_t;

/* The slot holding `(type, invoke_id)`, or -1. */
static inline int sce_host_invocation_find_(const sce_host_invocation_set_t *set, const char *type,
                                            const char *invoke_id) {
    int i;
    for (i = 0; i < set->count; i++) {
        if (strcmp(set->types[i], type) == 0 && strcmp(set->ids[i], invoke_id) == 0) {
            return i;
        }
    }
    return -1;
}

/* Close the gap slot `i` leaves. */
static inline void sce_host_invocation_remove_(sce_host_invocation_set_t *set, int i) {
    int j;
    for (j = i; j + 1 < set->count; j++) {
        sce_copy_bounded_id(set->types[j], set->types[j + 1]);
        sce_copy_bounded_id(set->ids[j], set->ids[j + 1]);
        set->tokens[j] = set->tokens[j + 1];
    }
    set->count--;
}

/** Record that `(type, invoke_id)` started, returning the token this start
    receives. Called BEFORE the handler runs, so a handler that completes
    synchronously completes an invocation the set already holds.

    A generated machine cannot reach the full case: its header asserts at
    compile time that `SCE_MAX_HOST_INVOCATIONS` covers the most invocations
    its configurations hold at once (§scxml-6.4), so a ceiling too small is a
    build error rather than a cancel the host never receives. The guard stays
    for a hand-written caller, where it keeps the bound. */
static inline uint64_t sce_host_invocation_mark(sce_host_invocation_set_t *set, const char *type,
                                                const char *invoke_id) {
    uint64_t token;
    int i;
    if (set == NULL || type == NULL || invoke_id == NULL) {
        return 0u;
    }
    token = set->next_token++;
    i = sce_host_invocation_find_(set, type, invoke_id);
    if (i >= 0) {
        set->tokens[i] = token;
        return token;
    }
    if (set->count >= (int)SCE_MAX_HOST_INVOCATIONS) {
        return token;
    }
    sce_copy_bounded_id(set->types[set->count], type);
    sce_copy_bounded_id(set->ids[set->count], invoke_id);
    set->tokens[set->count] = token;
    set->count++;
    return token;
}

/** Remove `(type, invoke_id)` whatever its token — the cancel path's
    question, since a state's exit ends the run it started. Reports whether
    it was running and, when it was, writes its token to `token_out`. */
static inline bool sce_host_invocation_take(sce_host_invocation_set_t *set, const char *type, const char *invoke_id,
                                            uint64_t *token_out) {
    int i;
    if (set == NULL || type == NULL || invoke_id == NULL) {
        return false;
    }
    i = sce_host_invocation_find_(set, type, invoke_id);
    if (i < 0) {
        return false;
    }
    if (token_out != NULL) {
        *token_out = set->tokens[i];
    }
    sce_host_invocation_remove_(set, i);
    return true;
}

/** Remove `(type, invoke_id)` only while it is still the start `token`
    names — the completion's question (§scxml-6.4). Taking it is what makes a
    completion exactly-once: a second completion, a cancel, or a stale token
    from an earlier start all find nothing. */
static inline bool sce_host_invocation_take_token(sce_host_invocation_set_t *set, const char *type,
                                                  const char *invoke_id, uint64_t token) {
    int i;
    if (set == NULL || type == NULL || invoke_id == NULL) {
        return false;
    }
    i = sce_host_invocation_find_(set, type, invoke_id);
    if (i < 0 || set->tokens[i] != token) {
        return false;
    }
    sce_host_invocation_remove_(set, i);
    return true;
}

/* ── The Mesh router a child session is given by its parent ─────── */
/*
 * SCE_MESH.md, Child Sessions in Mesh (docs/adr/0043). The C11 port of the
 * Rust runtime's `ChildMeshRouter` and `MeshCall`: a table of functions and the
 * pointer they are called with, since the freestanding profile has no trait
 * objects, and a tagged struct for a call, since C has no sum type.
 */

/** `_event.origin` of a Mesh-delivered event is this scheme and the sending
    machine's name (SCE_MESH.md §mesh-10.7). */
#define SCE_MESH_ORIGIN_SCHEME "mesh://"

/* The pair `{processor, mesh://<peer>}`: false, leaving `origin` none, when the
   address does not fit. */
static inline bool sce_event_origin_mesh_(sce_event_origin_t *origin, const char *processor, const char *peer) {
    const size_t scheme_len = sizeof(SCE_MESH_ORIGIN_SCHEME) - 1u;
    const size_t peer_len = strlen(peer);
    if (scheme_len + peer_len >= sizeof(origin->address) || !sce_event_origin_set(origin, processor, "")) {
        sce_event_origin_clear(origin);
        return false;
    }
    memcpy(origin->address, SCE_MESH_ORIGIN_SCHEME, scheme_len);
    memcpy(origin->address + scheme_len, peer, peer_len + 1u);
    return true;
}

/** The pair the Mesh transport writes for an event a peer's envelope delivers
    (docs/adr/0090, section 2): the SCXML Event I/O Processor, since every
    envelope is SCE's own inter-SCXML traffic, at the address
    `mesh://<source>` reaches the peer by (SCE_MESH.md §mesh-10.7). None for an
    empty `source`, which names no peer. */
static inline bool sce_event_origin_mesh_peer(sce_event_origin_t *origin, const char *source) {
    if (source == NULL || source[0] == '\0') {
        sce_event_origin_clear(origin);
        return true;
    }
    return sce_event_origin_mesh_(origin, SCE_SCXML_EVENT_PROCESSOR_TYPE, source);
}

/** The pair the Mesh transport writes for the answer to an `<invoke
    type="sce:mesh-rpc">`: the request's own type at `mesh://<replier>`, the peer
    that answered (SCE_MESH.md §mesh-9.5). None for an empty `replier`: an
    outcome the router reached itself, a deadline, names no peer. */
static inline bool sce_event_origin_mesh_rpc_reply(sce_event_origin_t *origin, const char *replier) {
    if (replier == NULL || replier[0] == '\0') {
        sce_event_origin_clear(origin);
        return true;
    }
    return sce_event_origin_mesh_(origin, SCE_MESH_RPC_INVOKE_TYPE, replier);
}

/** Which of a `sce_mesh_call_t`'s two shapes it is. */
typedef enum sce_mesh_call_kind_e {
    /* Raise `name` on the external queue with `data`, `send_id`, and the
       origin `sce_event_origin_mesh_peer` writes for `source`. */
    SCE_MESH_CALL_RAISE = 0,
    /* End the start `token` of the Mesh request `invoke_id` — with
       `done.invoke`, or with `error.invoke` when `failed` — carrying `data`;
       `source` is the machine that answered, empty for a deadline the
       requester reached itself. */
    SCE_MESH_CALL_END_INVOKE = 1
} sce_mesh_call_kind_t;

/** One thing a Mesh endpoint has for the machine it serves. */
typedef struct sce_mesh_call_s {
    sce_mesh_call_kind_t kind;
    char name[SCE_MAX_ID_LEN];
    char data[SCE_MAX_DATA_LEN];
    char send_id[SCE_MAX_ID_LEN];
    char source[SCE_MAX_ID_LEN];
    char invoke_id[SCE_MAX_ID_LEN];
    uint64_t token;
    bool failed;
} sce_mesh_call_t;

/**
 * The Mesh router a child session is given by its parent: the child's
 * endpoint attached to that one session.
 *
 * What a host installs on a machine it starts, the parent installs on the
 * child it starts, before the child is entered: the child's
 * `<send type="sce:mesh">` goes to `send` and its `<invoke type="sce:mesh-rpc">`
 * to `invoke` (NULL for a router that serves no request). What arrives for the
 * child waits in the router until the parent drives the child, which takes it
 * one call at a time with `take_call` (true while one waits) before the
 * child's macrostep. When the session ends the parent says so with `detach`,
 * once, after the child has stopped. `child_router` answers for the child's own
 * children, as the parent's factory is asked (NULL for none).
 */
typedef struct sce_child_mesh_router_s {
    void *user_data;
    sce_host_send_handler_fn send;
    sce_host_invoke_handler_fn invoke;
    bool (*take_call)(void *user_data, sce_mesh_call_t *out);
    void (*detach)(void *user_data);
    sce_child_mesh_routers_fn child_router;
} sce_child_mesh_router_t;

/** Ask `routers` for the router of a child the invoke `invoke_id` starts from
    the document whose stem is `document`: `out` when it answered one, NULL
    when there is no factory or it answered none. */
static inline const sce_child_mesh_router_t *sce_child_mesh_routers_ask(const sce_child_mesh_routers_t *routers,
                                                                        const char *invoke_id, const char *document,
                                                                        sce_child_mesh_router_t *out) {
    if (routers == NULL || routers->ask == NULL || out == NULL) {
        return NULL;
    }
    memset(out, 0, sizeof(*out));
    return routers->ask(routers->user_data, invoke_id, document, out) ? out : NULL;
}

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* SCE_HOST_PROCESSOR_H */
