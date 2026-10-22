/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * A `datamodel="sce-static"` machine's whole state, saved at a macrostep
 * boundary and restored into a new process (SCE Accepted Subset §2.15, E17;
 * docs/adr/0045), on the C11 backend.
 *
 * The format is one JSON document (`sce-saved-state`, format 1 or 2), the same
 * on every backend, so what one backend saved another reads. This header is the
 * part of the C11 side that is the same for every machine: it judges the
 * members of a session (the words of a refusal are the other backends'), hands
 * back each as a span of the text, and writes the members a machine's own
 * `_save` has no knowledge of (a waiting send, a queued event, a host-run
 * invocation, a record of revisions).
 *
 * Nothing is allocated. A session read is a handful of spans into the text the
 * caller owns; a send, an event or an invocation is read from its span when a
 * machine asks for it, and a text the machine's bounds cannot hold is refused by
 * the machine that holds the bound.
 *
 * Header-only. Cross-language siblings: sce/include/saved/SavedState.h (C++),
 * backends/go/runtime/saved_state.go, backends/python/runtime/sce_runtime/saved_state.py,
 * com.sce.runtime.SavedState (Kotlin) and sce_rust_runtime::saved_state (Rust).
 */
#ifndef SCE_SAVED_STATE_H
#define SCE_SAVED_STATE_H

#include "sce/saved_json.h"
#include "sce/saved_values.h"

#ifdef __cplusplus
extern "C" {
#endif

/* A state that names its running children by id alone. */
#define SCE_SAVED_FORMAT_1 1
/* A tree of sessions, each running child saved inside its parent (docs/adr/0045). */
#define SCE_SAVED_FORMAT_2 2
/* The newest format this runtime writes; it reads this one and format 1. */
#define SCE_SAVED_FORMAT SCE_SAVED_FORMAT_2

/* The stability status of the saved-state wire surface, held in lockstep with
 * `x-sce-schema-status` in `schemas/sce-saved-state.v1.schema.json` and
 * `schemas/sce-saved-state.v2.schema.json` (SCE_WIRE_CONTRACTS.md). */
#define SCE_SAVED_SCHEMA_STATUS "pre-release"

/* The most bytes of a session id, a text or a name a refusal quotes. */
#define SCE_SAVED_QUOTE_LEN 96u
/* The room for the path to a member (`invokestarts.<name>`, `pending[3]`), and for a path one
 * member further (`<path>.session`): the first holds a quoted name and what leads it, the
 * second the first and what follows it. */
#define SCE_SAVED_WHAT_LEN 128u
#define SCE_SAVED_AT_LEN 160u

/* ── The words of a refusal ───────────────────────────────────────────────── */

/* What a refusal inside a child session of a tree begins with, before the
 * session's path. */
#define SCE_SAVED_SESSION_AT_ "the session at "

/* `err`, a refusal of the child session at `path` of the session that reads it
 * (`invokes[0].session`): a tree is refused as a whole, and the reason names
 * the session it is about by its path from the root (docs/adr/0045, section 2),
 * `invokes[0].session.invokes[1].session` for a grandchild. */
static inline bool sce_saved_within(sce_saved_error_t *err, const char *path) {
    char reason[SCE_SAVED_ERROR_LEN];
    const size_t lead = strlen(SCE_SAVED_SESSION_AT_);
    if (err == NULL) {
        return false;
    }
    memcpy(reason, err->message, sizeof(reason));
    if (strncmp(reason, SCE_SAVED_SESSION_AT_, lead) == 0) {
        return sce_saved_refuse(err, "%s%s.%s", SCE_SAVED_SESSION_AT_, path, reason + lead);
    }
    return sce_saved_refuse(err, "%s%s: %s", SCE_SAVED_SESSION_AT_, path, reason);
}

/* The path a refusal names a member of `what` by: `what.id`, cut at what `dst` has room for. */
static inline void sce_saved_path_field(char *dst, size_t cap, const char *what, const char *id) {
    size_t n = strlen(what);
    const size_t m = strlen(id);
    if (cap == 0u) {
        return;
    }
    if (n > cap - 1u) {
        n = cap - 1u;
    }
    memcpy(dst, what, n);
    if (n + 1u < cap) {
        dst[n++] = '.';
    }
    if (n + m >= cap) {
        memcpy(dst + n, id, cap - 1u - n);
        dst[cap - 1u] = '\0';
        return;
    }
    memcpy(dst + n, id, m + 1u);
}

/* The path a refusal names an element of `what` by: `what[index]`, cut at what `dst` has room for. */
static inline void sce_saved_path_index(char *dst, size_t cap, const char *what, size_t index) {
    char digits[24];
    size_t n = strlen(what);
    size_t m;
    if (cap == 0u) {
        return;
    }
    (void)snprintf(digits, sizeof(digits), "[%lu]", (unsigned long)index);
    m = strlen(digits);
    if (n > cap - 1u) {
        n = cap - 1u;
    }
    memcpy(dst, what, n);
    if (n + m >= cap) {
        m = cap - 1u - n;
    }
    memcpy(dst + n, digits, m);
    dst[n + m] = '\0';
}

/* A text a refusal quotes: `value` decoded, cut at what a refusal has room for. */
static inline void sce_saved_quote_(sce_jv_t value, char *out, size_t cap) {
    size_t len = 0u;
    (void)sce_jv_string(value, out, cap, &len);
}

/* A text a machine holds in a buffer of `capacity` bytes, terminator included, which `what` names
 * in a refusal: refused, in the words of a bounded text, unless it fits. `dst` may be NULL, which
 * judges the text and keeps nothing. */
static inline bool sce_saved_text_into(sce_jv_t v, const char *what, char *dst, size_t capacity,
                                       sce_saved_error_t *err) {
    const size_t held = sce_jv_string_len(v);
    if (held + 1u > capacity) {
        return sce_saved_refuse(err, "'%s' holds %lu UTF-8 bytes, past the %lu it is bounded by", what,
                                (unsigned long)held, (unsigned long)(capacity - 1u));
    }
    if (dst != NULL) {
        (void)sce_jv_string(v, dst, capacity, NULL);
    }
    return true;
}

/* ── One session's members ────────────────────────────────────────────────── */

/* The members of one saved session, each as the span of the text that holds it.
 * Judged by `sce_saved_session_read`: every member is of the kind the format
 * names, and so is each element of those that hold elements. What an element
 * means to the machine is the machine's to judge. */
typedef struct sce_saved_session_s {
    int format;             /* SCE_SAVED_FORMAT_1 or SCE_SAVED_FORMAT_2 */
    sce_jv_t shape;         /* a text */
    sce_jv_t session_id;    /* a text; format 2 only */
    sce_jv_t configuration; /* an array of texts: the state ids */
    sce_jv_t current;       /* a text */
    sce_jv_t variables;     /* an object */
    sce_jv_t history;       /* an object of arrays of texts */
    sce_jv_t pending;       /* an array of sends */
    sce_jv_t invokes;       /* format 1: an array of ids; format 2: an array of children */
    sce_jv_t invoke_starts; /* an object of counts; format 2 only */
    sce_jv_t host_invokes;  /* an array of host-run invocations */
    sce_jv_t external;      /* an array of events */
    sce_jv_t revisions;     /* an object; valid when `has_revisions` */
    bool has_revisions;
    int64_t host_invoke_token;
    int64_t auto_send_seq;
} sce_saved_session_t;

/* What a waiting `<send>` does when it comes due. */
typedef enum sce_saved_act_e {
    SCE_SAVED_ACT_PARENT = 0, /* an event for the session that started this one (`#_parent`) */
    SCE_SAVED_ACT_CHILD,      /* an event for a running child of this session (`#_<invokeid>`) */
    SCE_SAVED_ACT_RAISE,      /* an event for this session's external queue */
    SCE_SAVED_ACT_INTERNAL,   /* an event for this session's internal queue */
    SCE_SAVED_ACT_HOST        /* a `<send>` a host-served processor performs (§scxml-6.2.5) */
} sce_saved_act_t;

/* The word the format writes for an act. */
static inline const char *sce_saved_act_name(sce_saved_act_t act) {
    switch (act) {
    case SCE_SAVED_ACT_PARENT:
        return "parent";
    case SCE_SAVED_ACT_CHILD:
        return "child";
    case SCE_SAVED_ACT_RAISE:
        return "raise";
    case SCE_SAVED_ACT_INTERNAL:
        return "internal";
    case SCE_SAVED_ACT_HOST:
        return "host";
    }
    return "";
}

/* One delayed `<send>` that has not been delivered yet, as the text holds it. `due` is when it
 * comes due, in milliseconds since the Unix epoch on the wall clock of the host that saved it.
 * A member the act does not have is the empty span (`p == end`). */
typedef struct sce_saved_send_s {
    int64_t due;
    sce_saved_act_t act;
    sce_jv_t event;
    sce_jv_t data;
    sce_jv_t send_id;
    sce_jv_t origin;
    sce_jv_t invoke_id;
    sce_jv_t type;
    sce_jv_t target;
    sce_jv_t content;
    sce_jv_t params;
    bool has_params;
} sce_saved_send_t;

/* One event of a saved external queue. */
typedef struct sce_saved_event_s {
    sce_jv_t name;
    sce_jv_t data;
    sce_jv_t type;
    sce_jv_t send_id;
    sce_jv_t origin;
    sce_jv_t origin_type;
    sce_jv_t invoke_id;
    bool has_host_invoke_token;
    int64_t host_invoke_token;
} sce_saved_event_t;

/* An `<invoke>` a declared host invoker is running, as it is started again (§scxml-6.4.1). */
typedef struct sce_saved_host_invoke_s {
    sce_jv_t type;
    sce_jv_t id;
    sce_jv_t src;
    sce_jv_t params;
    sce_jv_t data;
    sce_jv_t content;
    bool has_due;
    int64_t due;
} sce_saved_host_invoke_t;

/* A running child session, saved inside the session that started it. */
typedef struct sce_saved_child_s {
    sce_jv_t id;        /* the id the document gives the `<invoke>` that started it */
    int64_t start;      /* which start of that `<invoke>` this is in its parent, from 1 */
    sce_jv_t invoke_id; /* the id the child was started under */
    sce_jv_t document;  /* the document the child runs */
    sce_jv_t session;   /* the child's own members: an object, read by `sce_saved_session_read` */
} sce_saved_child_t;

/* ── Reading ──────────────────────────────────────────────────────────────── */

/* The span of nothing: what a member an act does not have is. */
static inline sce_jv_t sce_saved_nothing_(void) {
    sce_jv_t none;
    static const char empty[1] = {0};
    none.p = empty;
    none.end = empty;
    return none;
}

static inline bool sce_saved_is_nothing_(sce_jv_t v) {
    return v.p == v.end;
}

/* A text of digits as a signed 64-bit whole number: false when it is anything else. */
static inline bool sce_saved_digits_(sce_jv_t v, char *text, size_t cap, int64_t *out) {
    size_t len = 0u;
    uint64_t wide = 0u;
    size_t at;
    (void)sce_jv_string(v, text, cap, &len);
    if (len == 0u) {
        return false;
    }
    for (at = 0u; at < len; ++at) {
        const unsigned digit = (unsigned)(unsigned char)text[at] - (unsigned)'0';
        if (digit > 9u) {
            return false;
        }
        if (wide > (UINT64_MAX - digit) / 10u) {
            return false;
        }
        wide = wide * 10u + digit;
    }
    if (len >= cap || wide > (uint64_t)INT64_MAX) {
        return false;
    }
    *out = (int64_t)wide;
    return true;
}

/* A count of starts, as the text a saved state writes it as: a whole number from 1 with no
 * leading zero, within a signed 64-bit count, as every backend reads one. */
static inline bool sce_saved_read_count(sce_jv_t v, const char *what, int64_t *out, sce_saved_error_t *err) {
    char text[SCE_SAVED_QUOTE_LEN];
    if (!sce_saved_digits_(v, text, sizeof(text), out) || *out < 1 || text[0] == '0') {
        return sce_saved_refuse(err, "'%s' (%s) is not a count of starts, a whole number from 1", what, text);
    }
    return true;
}

/* A host-run invocation's token, or the number of ids a machine has generated. */
static inline bool sce_saved_read_token(sce_jv_t v, const char *what, int64_t *out, sce_saved_error_t *err) {
    char text[SCE_SAVED_QUOTE_LEN];
    if (!sce_saved_digits_(v, text, sizeof(text), out)) {
        return sce_saved_refuse(err, "'%s' (%s) is not a whole number a token can be", what, text);
    }
    return true;
}

/* A moment, milliseconds since the Unix epoch, as the text of its digits. */
static inline bool sce_saved_read_moment(sce_jv_t v, const char *what, int64_t *out, sce_saved_error_t *err) {
    char text[SCE_SAVED_QUOTE_LEN];
    if (!sce_saved_digits_(v, text, sizeof(text), out)) {
        return sce_saved_refuse(err, "'%s' (%s) is not a whole number of milliseconds", what, text);
    }
    return true;
}

/* The member `key` of `members`, which `what` names in a refusal. */
static inline bool sce_saved_member_(sce_jv_t members, const char *what, const char *key, sce_jv_t *out,
                                     sce_saved_error_t *err) {
    if (!sce_jv_get(members, key, out)) {
        return sce_saved_refuse(err, "'%s' has no '%s'", what, key);
    }
    return true;
}

/* The text member `key` of `members`, which `what` names in a refusal. */
static inline bool sce_saved_member_text_(sce_jv_t members, const char *what, const char *key, sce_jv_t *out,
                                          sce_saved_error_t *err) {
    if (!sce_saved_member_(members, what, key, out, err)) {
        return false;
    }
    if (sce_jv_kind(*out) != SCE_JSON_STRING) {
        return sce_saved_refuse(err, "'%s.%s' is not a text", what, key);
    }
    return true;
}

/* `value` as an object, which `what` names in a refusal. */
static inline bool sce_saved_object_(sce_jv_t value, const char *what, sce_saved_error_t *err) {
    if (sce_jv_kind(value) != SCE_JSON_OBJECT) {
        return sce_saved_refuse(err, "'%s' is not an object", what);
    }
    return true;
}

/* The member `key` of `members`, which is null or a text read by `read`. */
static inline bool sce_saved_optional_count_(sce_jv_t members, const char *what, const char *key,
                                             bool (*read)(sce_jv_t, const char *, int64_t *, sce_saved_error_t *),
                                             bool *present, int64_t *out, sce_saved_error_t *err) {
    sce_jv_t value;
    char at[SCE_SAVED_AT_LEN];
    if (!sce_saved_member_(members, what, key, &value, err)) {
        return false;
    }
    *present = false;
    if (sce_jv_is_null(value)) {
        return true;
    }
    (void)snprintf(at, sizeof(at), "%s.%s", what, key);
    if (sce_jv_kind(value) != SCE_JSON_STRING) {
        return sce_saved_refuse(err, "'%s' is neither a text nor null", at);
    }
    *present = true;
    return read(value, at, out, err);
}

/* A `params` member read as an object of arrays of texts, by name: judged, since a machine
 * reads its values when it needs them. */
static inline bool sce_saved_params_(sce_jv_t members, const char *what, sce_jv_t *out, sce_saved_error_t *err) {
    sce_jiter_t members_it;
    sce_jiter_t items_it;
    sce_jv_t name;
    sce_jv_t values;
    sce_jv_t item;
    char quoted[SCE_SAVED_QUOTE_LEN];
    if (!sce_saved_member_(members, what, "params", out, err)) {
        return false;
    }
    if (sce_jv_kind(*out) != SCE_JSON_OBJECT) {
        return sce_saved_refuse(err, "'%s.params' is not an object", what);
    }
    (void)sce_jv_iter(*out, &members_it);
    while (sce_jiter_member(&members_it, &name, &values)) {
        sce_saved_quote_(name, quoted, sizeof(quoted));
        if (sce_jv_kind(values) != SCE_JSON_ARRAY) {
            return sce_saved_refuse(err, "'%s.params.%s' is not an array", what, quoted);
        }
        (void)sce_jv_iter(values, &items_it);
        while (sce_jiter_element(&items_it, &item)) {
            if (sce_jv_kind(item) != SCE_JSON_STRING) {
                return sce_saved_refuse(err, "'%s.params.%s' holds a value that is not a text", what, quoted);
            }
        }
    }
    return true;
}

/* The waiting send `v`, of a state of `form`; a send to the parent or to a child is an act of
 * format 2 only. `what` names it in a refusal. */
static inline bool sce_saved_send_read(sce_jv_t v, const char *what, int form, sce_saved_send_t *out,
                                       sce_saved_error_t *err) {
    sce_jv_t due;
    sce_jv_t act;
    char name[SCE_SAVED_QUOTE_LEN];
    char at[SCE_SAVED_AT_LEN];
    const sce_jv_t none = sce_saved_nothing_();
    if (!sce_saved_object_(v, what, err)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    out->event = out->data = out->send_id = out->origin = out->invoke_id = none;
    out->type = out->target = out->content = out->params = none;
    (void)snprintf(at, sizeof(at), "%s.due", what);
    if (!sce_saved_member_text_(v, what, "due", &due, err) || !sce_saved_read_moment(due, at, &out->due, err)) {
        return false;
    }
    if (!sce_saved_member_text_(v, what, "act", &act, err)) {
        return false;
    }
    sce_saved_quote_(act, name, sizeof(name));
    if (strcmp(name, "parent") == 0 || strcmp(name, "child") == 0) {
        if (form != SCE_SAVED_FORMAT_2) {
            return sce_saved_refuse(err, "'%s.act' is '%s', which is not raise, internal or host, the acts of format 1",
                                    what, name);
        }
        out->act = name[0] == 'p' ? SCE_SAVED_ACT_PARENT : SCE_SAVED_ACT_CHILD;
        if (out->act == SCE_SAVED_ACT_CHILD && !sce_saved_member_text_(v, what, "invokeid", &out->invoke_id, err)) {
            return false;
        }
    } else if (strcmp(name, "raise") == 0 || strcmp(name, "internal") == 0) {
        out->act = name[0] == 'r' ? SCE_SAVED_ACT_RAISE : SCE_SAVED_ACT_INTERNAL;
    } else if (strcmp(name, "host") == 0) {
        out->act = SCE_SAVED_ACT_HOST;
        if (!sce_saved_member_text_(v, what, "type", &out->type, err) ||
            !sce_saved_member_text_(v, what, "event", &out->event, err) ||
            !sce_saved_member_text_(v, what, "target", &out->target, err) ||
            !sce_saved_member_text_(v, what, "content", &out->content, err) ||
            !sce_saved_params_(v, what, &out->params, err) ||
            !sce_saved_member_text_(v, what, "sendid", &out->send_id, err) ||
            !sce_saved_member_text_(v, what, "data", &out->data, err) ||
            !sce_saved_member_text_(v, what, "invokeid", &out->invoke_id, err)) {
            return false;
        }
        out->has_params = true;
        return true;
    } else if (form == SCE_SAVED_FORMAT_1) {
        return sce_saved_refuse(err, "'%s.act' is '%s', which is not raise, internal or host, the acts of format 1",
                                what, name);
    } else {
        return sce_saved_refuse(err, "'%s.act' is '%s', which is not raise, internal, host, parent or child", what,
                                name);
    }
    return sce_saved_member_text_(v, what, "event", &out->event, err) &&
           sce_saved_member_text_(v, what, "data", &out->data, err) &&
           sce_saved_member_text_(v, what, "sendid", &out->send_id, err) &&
           sce_saved_member_text_(v, what, "origin", &out->origin, err);
}

/* The queued event `v`. */
static inline bool sce_saved_event_read(sce_jv_t v, const char *what, sce_saved_event_t *out, sce_saved_error_t *err) {
    if (!sce_saved_object_(v, what, err)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    /* The token is judged first, as every backend judges it. */
    return sce_saved_optional_count_(v, what, "hostinvoketoken", sce_saved_read_token, &out->has_host_invoke_token,
                                     &out->host_invoke_token, err) &&
           sce_saved_member_text_(v, what, "name", &out->name, err) &&
           sce_saved_member_text_(v, what, "data", &out->data, err) &&
           sce_saved_member_text_(v, what, "type", &out->type, err) &&
           sce_saved_member_text_(v, what, "sendid", &out->send_id, err) &&
           sce_saved_member_text_(v, what, "origin", &out->origin, err) &&
           sce_saved_member_text_(v, what, "origintype", &out->origin_type, err) &&
           sce_saved_member_text_(v, what, "invokeid", &out->invoke_id, err);
}

/* The host-run invocation `v`. */
static inline bool sce_saved_host_invoke_read(sce_jv_t v, const char *what, sce_saved_host_invoke_t *out,
                                              sce_saved_error_t *err) {
    if (!sce_saved_object_(v, what, err)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    return sce_saved_optional_count_(v, what, "due", sce_saved_read_moment, &out->has_due, &out->due, err) &&
           sce_saved_member_text_(v, what, "type", &out->type, err) &&
           sce_saved_member_text_(v, what, "id", &out->id, err) &&
           sce_saved_member_text_(v, what, "src", &out->src, err) && sce_saved_params_(v, what, &out->params, err) &&
           sce_saved_member_text_(v, what, "data", &out->data, err) &&
           sce_saved_member_text_(v, what, "content", &out->content, err);
}

static inline bool sce_saved_session_read(sce_jv_t members, int form, sce_saved_session_t *out, sce_saved_error_t *err);

/* The non-empty text member `key` of the child `members`, which `what` names. */
static inline bool sce_saved_non_empty_(sce_jv_t members, const char *what, const char *key, sce_jv_t *out,
                                        sce_saved_error_t *err) {
    if (!sce_saved_member_text_(members, what, key, out, err)) {
        return false;
    }
    if (sce_jv_string_len(*out) == 0u) {
        return sce_saved_refuse(err, "'%s.%s' is empty", what, key);
    }
    return true;
}

/* The running child `v`, which also judges the session it holds, to any depth. */
static inline bool sce_saved_child_read(sce_jv_t v, size_t index, sce_saved_child_t *out, sce_saved_error_t *err) {
    char what[SCE_SAVED_WHAT_LEN];
    char at[SCE_SAVED_AT_LEN];
    sce_jv_t start;
    sce_jv_t unused;
    sce_saved_session_t session;
    (void)snprintf(what, sizeof(what), "invokes[%lu]", (unsigned long)index);
    if (!sce_saved_object_(v, what, err)) {
        return false;
    }
    memset(out, 0, sizeof(*out));
    (void)snprintf(at, sizeof(at), "%s.start", what);
    if (!sce_saved_non_empty_(v, what, "start", &start, err) || !sce_saved_read_count(start, at, &out->start, err)) {
        return false;
    }
    if (!sce_jv_get(v, "session", &out->session)) {
        return sce_saved_refuse(err, "'%s' has no 'session'", what);
    }
    (void)snprintf(at, sizeof(at), "%s.session", what);
    if (!sce_saved_object_(out->session, at, err)) {
        return false;
    }
    if (sce_jv_get(out->session, "format", &unused)) {
        return sce_saved_refuse(err, "'%s' states a format, which only the root of a tree does", at);
    }
    if (!sce_saved_session_read(out->session, SCE_SAVED_FORMAT_2, &session, err)) {
        return sce_saved_within(err, at);
    }
    return sce_saved_non_empty_(v, what, "id", &out->id, err) &&
           sce_saved_non_empty_(v, what, "invokeid", &out->invoke_id, err) &&
           sce_saved_non_empty_(v, what, "document", &out->document, err);
}

/* A revision entry of `revisions`: a number past the first and the SHA-256 of what it declares. */
static inline bool sce_saved_revision_read(sce_jv_t value, const char *what, int64_t *revision, sce_jv_t *fields,
                                           sce_saved_error_t *err) {
    sce_jv_t number;
    char text[SCE_SAVED_QUOTE_LEN];
    size_t at;
    if (!sce_saved_object_(value, what, err)) {
        return false;
    }
    if (!sce_jv_get(value, "revision", &number)) {
        return sce_saved_refuse(err, "'%s' has no 'revision'", what);
    }
    if (!sce_jv_get(value, "fields", fields)) {
        return sce_saved_refuse(err, "'%s' has no 'fields'", what);
    }
    if (sce_jv_kind(number) != SCE_JSON_NUMBER) {
        return sce_saved_refuse(err, "'%s.revision' is not a number", what);
    }
    {
        const size_t len =
            sce_jv_number_len(number) < sizeof(text) - 1u ? sce_jv_number_len(number) : sizeof(text) - 1u;
        bool digits = len > 0u;
        uint64_t wide = 0u;
        memcpy(text, number.p, len);
        text[len] = '\0';
        for (at = 0u; at < len; ++at) {
            if (text[at] < '0' || text[at] > '9') {
                digits = false;
            } else if (wide < (UINT64_MAX / 100u)) {
                wide = wide * 10u + (uint64_t)(text[at] - '0');
            }
        }
        if (!digits || wide < 2u || wide > (uint64_t)INT64_MAX) {
            return sce_saved_refuse(err,
                                    "'%s.revision' (%s) is not a revision past the first; a schema in its first "
                                    "revision is left out",
                                    what, text);
        }
        *revision = (int64_t)wide;
    }
    {
        bool lower_hex = sce_jv_kind(*fields) == SCE_JSON_STRING && sce_jv_string_len(*fields) == 64u;
        if (lower_hex) {
            sce_jstr_t s;
            unsigned char byte;
            sce_jstr_init(&s, *fields);
            while (sce_jstr_next(&s, &byte)) {
                if (!((byte >= '0' && byte <= '9') || (byte >= 'a' && byte <= 'f'))) {
                    lower_hex = false;
                }
            }
        }
        if (!lower_hex) {
            return sce_saved_refuse(err, "'%s.fields' is not a SHA-256 written as lowercase hex", what);
        }
    }
    return true;
}

/* A session id read from a span: an XML Nmtoken, which a session id is. */
static inline bool sce_saved_session_id_view_ok(sce_jv_t v, sce_saved_error_t *err) {
    char id[SCE_SAVED_QUOTE_LEN * 4u];
    size_t len = 0u;
    sce_saved_error_t why;
    (void)sce_jv_string(v, id, sizeof(id), &len);
    if (!sce_saved_session_id_ok(id, &why)) {
        return sce_saved_refuse(err, "'sessionid' (%.*s) is not an XML Nmtoken, which a session id is",
                                (int)SCE_SAVED_QUOTE_LEN, id);
    }
    return true;
}

/* The session `members` hold, of a text of `form`: the root, or a child's `session`, which states
 * no `format` of its own. The shape is not judged here; the machine that restores it knows its own. */
static inline bool sce_saved_session_read(sce_jv_t members, int form, sce_saved_session_t *out,
                                          sce_saved_error_t *err) {
    sce_jiter_t it;
    sce_jv_t item;
    sce_jv_t name;
    sce_jv_t value;
    sce_jv_t counted;
    char what[SCE_SAVED_WHAT_LEN];
    char quoted[SCE_SAVED_QUOTE_LEN];
    size_t at = 0u;
    memset(out, 0, sizeof(*out));
    out->format = form;

    if (!sce_jv_get(members, "configuration", &out->configuration)) {
        return sce_saved_refuse(err, "the saved state has no 'configuration'");
    }
    if (sce_jv_kind(out->configuration) != SCE_JSON_ARRAY) {
        return sce_saved_refuse(err, "'configuration' is not an array");
    }
    (void)sce_jv_iter(out->configuration, &it);
    while (sce_jiter_element(&it, &item)) {
        if (sce_jv_kind(item) != SCE_JSON_STRING) {
            return sce_saved_refuse(err, "'configuration' is not a text");
        }
    }

    if (!sce_jv_get(members, "variables", &out->variables)) {
        return sce_saved_refuse(err, "the saved state has no 'variables'");
    }
    if (sce_jv_kind(out->variables) != SCE_JSON_OBJECT) {
        return sce_saved_refuse(err, "'variables' is not an object");
    }

    if (!sce_jv_get(members, "history", &out->history)) {
        return sce_saved_refuse(err, "the saved state has no 'history'");
    }
    if (sce_jv_kind(out->history) != SCE_JSON_OBJECT) {
        return sce_saved_refuse(err, "'history' is not an object");
    }
    (void)sce_jv_iter(out->history, &it);
    while (sce_jiter_member(&it, &name, &value)) {
        sce_jiter_t listed;
        sce_saved_quote_(name, quoted, sizeof(quoted));
        if (sce_jv_kind(value) != SCE_JSON_ARRAY) {
            return sce_saved_refuse(err, "'history.%s' is not an array", quoted);
        }
        (void)sce_jv_iter(value, &listed);
        while (sce_jiter_element(&listed, &item)) {
            if (sce_jv_kind(item) != SCE_JSON_STRING) {
                return sce_saved_refuse(err, "'history.%s' is not a text", quoted);
            }
        }
    }

    if (!sce_jv_get(members, "pending", &out->pending)) {
        return sce_saved_refuse(err, "the saved state has no 'pending'");
    }
    if (sce_jv_kind(out->pending) != SCE_JSON_ARRAY) {
        return sce_saved_refuse(err, "'pending' is not an array");
    }
    (void)sce_jv_iter(out->pending, &it);
    for (at = 0u; sce_jiter_element(&it, &item); ++at) {
        sce_saved_send_t send;
        (void)snprintf(what, sizeof(what), "pending[%lu]", (unsigned long)at);
        if (!sce_saved_send_read(item, what, form, &send, err)) {
            return false;
        }
    }

    if (!sce_jv_get(members, "invokes", &out->invokes)) {
        return sce_saved_refuse(err, "the saved state has no 'invokes'");
    }
    if (sce_jv_kind(out->invokes) != SCE_JSON_ARRAY) {
        return sce_saved_refuse(err, "'invokes' is not an array");
    }
    (void)sce_jv_iter(out->invokes, &it);
    for (at = 0u; sce_jiter_element(&it, &item); ++at) {
        if (form == SCE_SAVED_FORMAT_1) {
            if (sce_jv_kind(item) != SCE_JSON_STRING) {
                return sce_saved_refuse(err, "'invokes' is not a text");
            }
        } else {
            sce_saved_child_t child;
            if (!sce_saved_child_read(item, at, &child, err)) {
                return false;
            }
        }
    }

    /* Format 2's own members: the session's id and its count of starts. */
    if (form == SCE_SAVED_FORMAT_2) {
        if (!sce_jv_get(members, "sessionid", &out->session_id)) {
            return sce_saved_refuse(err, "the saved state has no 'sessionid'");
        }
        if (sce_jv_kind(out->session_id) != SCE_JSON_STRING) {
            return sce_saved_refuse(err, "'sessionid' is not a text");
        }
        if (!sce_saved_session_id_view_ok(out->session_id, err)) {
            return false;
        }
        if (!sce_jv_get(members, "invokestarts", &counted)) {
            return sce_saved_refuse(err, "the saved state has no 'invokestarts'");
        }
        if (sce_jv_kind(counted) != SCE_JSON_OBJECT) {
            return sce_saved_refuse(err, "'invokestarts' is not an object");
        }
        out->invoke_starts = counted;
        (void)sce_jv_iter(counted, &it);
        while (sce_jiter_member(&it, &name, &value)) {
            int64_t count;
            sce_saved_quote_(name, quoted, sizeof(quoted));
            (void)snprintf(what, sizeof(what), "invokestarts.%s", quoted);
            if (sce_jv_kind(value) != SCE_JSON_STRING) {
                return sce_saved_refuse(err, "'%s' is not a text", what);
            }
            if (!sce_saved_read_count(value, what, &count, err)) {
                return false;
            }
        }
    }

    if (!sce_jv_get(members, "hostinvokes", &out->host_invokes)) {
        return sce_saved_refuse(err, "the saved state has no 'hostinvokes'");
    }
    if (sce_jv_kind(out->host_invokes) != SCE_JSON_ARRAY) {
        return sce_saved_refuse(err, "'hostinvokes' is not an array");
    }
    (void)sce_jv_iter(out->host_invokes, &it);
    for (at = 0u; sce_jiter_element(&it, &item); ++at) {
        sce_saved_host_invoke_t host;
        (void)snprintf(what, sizeof(what), "hostinvokes[%lu]", (unsigned long)at);
        if (!sce_saved_host_invoke_read(item, what, &host, err)) {
            return false;
        }
    }

    if (!sce_jv_get(members, "hostinvoketoken", &value)) {
        return sce_saved_refuse(err, "the saved state has no 'hostinvoketoken'");
    }
    if (sce_jv_kind(value) != SCE_JSON_STRING) {
        return sce_saved_refuse(err, "'hostinvoketoken' is not a text");
    }
    if (!sce_saved_read_token(value, "hostinvoketoken", &out->host_invoke_token, err)) {
        return false;
    }
    if (!sce_jv_get(members, "sendseq", &value)) {
        return sce_saved_refuse(err, "the saved state has no 'sendseq'");
    }
    if (sce_jv_kind(value) != SCE_JSON_STRING) {
        return sce_saved_refuse(err, "'sendseq' is not a text");
    }
    if (!sce_saved_read_token(value, "sendseq", &out->auto_send_seq, err)) {
        return false;
    }

    if (!sce_jv_get(members, "external", &out->external)) {
        return sce_saved_refuse(err, "the saved state has no 'external'");
    }
    if (sce_jv_kind(out->external) != SCE_JSON_ARRAY) {
        return sce_saved_refuse(err, "'external' is not an array");
    }
    (void)sce_jv_iter(out->external, &it);
    for (at = 0u; sce_jiter_element(&it, &item); ++at) {
        sce_saved_event_t event;
        (void)snprintf(what, sizeof(what), "external[%lu]", (unsigned long)at);
        if (!sce_saved_event_read(item, what, &event, err)) {
            return false;
        }
    }

    /* Optional, unlike every member above: a state whose record schemas are all in their first
     * revision leaves it out, as every state saved before schemas had revisions does. */
    if (sce_jv_get(members, "revisions", &out->revisions)) {
        out->has_revisions = true;
        if (sce_jv_kind(out->revisions) != SCE_JSON_OBJECT) {
            return sce_saved_refuse(err, "'revisions' is not an object");
        }
        if (sce_jv_count(out->revisions) == 0u) {
            return sce_saved_refuse(err, "'revisions' names no schema; a state whose record schemas are all in "
                                         "their first revision leaves it out");
        }
        (void)sce_jv_iter(out->revisions, &it);
        while (sce_jiter_member(&it, &name, &value)) {
            int64_t revision;
            sce_jv_t fields;
            sce_saved_quote_(name, quoted, sizeof(quoted));
            (void)snprintf(what, sizeof(what), "revisions.%s", quoted);
            if (!sce_saved_revision_read(value, what, &revision, &fields, err)) {
                return false;
            }
        }
    }

    if (!sce_jv_get(members, "shape", &out->shape)) {
        return sce_saved_refuse(err, "the saved state has no 'shape'");
    }
    if (sce_jv_kind(out->shape) != SCE_JSON_STRING) {
        return sce_saved_refuse(err, "'shape' is not a text");
    }
    if (!sce_jv_get(members, "current", &out->current)) {
        return sce_saved_refuse(err, "the saved state has no 'current'");
    }
    if (sce_jv_kind(out->current) != SCE_JSON_STRING) {
        return sce_saved_refuse(err, "'current' is not a text");
    }
    return true;
}

/* Reads `sce-saved-state` JSON, of format 1 or 2, giving the root session. Judges the text as
 * JSON first, then the members of the session and, in a tree, of every session below it. */
static inline bool sce_saved_read(const char *text, size_t len, sce_saved_session_t *out, sce_saved_error_t *err) {
    sce_jv_t root;
    sce_jv_t written;
    int form;
    if (!sce_json_parse(text, len, &root, err)) {
        char reason[SCE_SAVED_ERROR_LEN];
        if (err != NULL) {
            memcpy(reason, err->message, sizeof(reason));
            return sce_saved_refuse(err, "the saved state is %s", reason);
        }
        return false;
    }
    if (sce_jv_kind(root) != SCE_JSON_OBJECT) {
        return sce_saved_refuse(err, "the saved state is not an object");
    }
    if (!sce_jv_get(root, "format", &written)) {
        return sce_saved_refuse(err, "the saved state has no 'format'");
    }
    if (sce_jv_kind(written) == SCE_JSON_NUMBER && sce_jv_number_len(written) == 1u && *written.p == '1') {
        form = SCE_SAVED_FORMAT_1;
    } else if (sce_jv_kind(written) == SCE_JSON_NUMBER && sce_jv_number_len(written) == 1u && *written.p == '2') {
        form = SCE_SAVED_FORMAT_2;
    } else {
        sce_jw_t w;
        char quoted[SCE_SAVED_QUOTE_LEN];
        sce_jw_init(&w, quoted, sizeof(quoted));
        sce_jw_value(&w, written);
        (void)sce_jw_finish(&w);
        return sce_saved_refuse(err, "the saved state is format %s, and this runtime reads formats %d and %d", quoted,
                                SCE_SAVED_FORMAT_1, SCE_SAVED_FORMAT_2);
    }
    return sce_saved_session_read(root, form, out, err);
}

/* ── A session's own words ────────────────────────────────────────────────── */

/* Whether `v` is a text that decodes to exactly `want`. */
static inline bool sce_saved_text_is(sce_jv_t v, const char *want) {
    return sce_jv_kind(v) == SCE_JSON_STRING && sce_jv_string_is(v, want);
}

/* How many times the `<invoke>` `id` had started in the session when it was saved: 0 for one that
 * had not, and for every one of a format-1 state, which holds no count. */
static inline int64_t sce_saved_invoke_start(const sce_saved_session_t *session, const char *id) {
    sce_jiter_t it;
    sce_jv_t name;
    sce_jv_t value;
    int64_t count = 0;
    if (session->format != SCE_SAVED_FORMAT_2 || !sce_jv_iter(session->invoke_starts, &it)) {
        return 0;
    }
    while (sce_jiter_member(&it, &name, &value)) {
        if (sce_jv_string_is(name, id)) {
            sce_saved_error_t ignored;
            (void)sce_saved_read_count(value, "invokestarts", &count, &ignored);
            return count;
        }
    }
    return 0;
}

/* ── Shape and revisions (docs/adr/0044) ──────────────────────────────────── */

/* A variant a revision past the first added to an enum. */
typedef struct sce_saved_added_variant_s {
    const char *name;
    unsigned since;
} sce_saved_added_variant_t;

/* Where a value of an enum is held: a variable of the enum (`field` NULL), or the field of the
 * record a variable holds, `list` being whether it holds a list of such records. */
typedef struct sce_saved_enum_holder_s {
    const char *variable;
    const char *field;
    bool list;
} sce_saved_enum_holder_t;

/* A record schema or an enum a machine's state holds, as a saved state names it: `key` is the
 * event the record schema declares or `enum:` and the enum's document; `revision` is its
 * revision in this machine, and `digests[r - 2]` the digest of what revision r declares. An
 * enum also says which variants a revision past the first added and where a value of it is
 * held, which is how a value of a variant the saved revision did not have is found. */
typedef struct sce_saved_lineage_s {
    const char *key;
    unsigned revision;
    const char *const *digests;
    size_t digest_count;
    const sce_saved_added_variant_t *added;
    size_t added_count;
    const sce_saved_enum_holder_t *holders;
    size_t holder_count;
} sce_saved_lineage_t;

/* What a machine's saved state is bound to: the document's shape, and the revisions of each schema
 * a variable of it is of. The generator computes it, and every saved state is written and judged
 * against it. */
typedef struct sce_saved_shape_s {
    /* The shape a state names when every record schema it holds is in its first revision. */
    const char *first;
    /* The shape a state names when it names a later revision of any: a reader that predates
     * revisions knows only the first, so it refuses such a state rather than read it without the
     * fields it does not know. */
    const char *revised;
    /* The same two, as a format-2 state names them (docs/adr/0045, section 2). */
    const char *format2_first;
    const char *format2_revised;
    const sce_saved_lineage_t *records;
    size_t record_count;
    const sce_saved_lineage_t *enums;
    size_t enum_count;
} sce_saved_shape_t;

/* Refuses the value `held`, at `what`, if it names a variant of `lineage` that a revision past
 * `at`, the one the state was saved at, added. What is not a text where one is held is left for
 * the value's own read to refuse. */
static inline bool sce_saved_enum_value_(const sce_saved_lineage_t *lineage, unsigned at, const char *what,
                                         sce_jv_t held, sce_saved_error_t *err) {
    size_t i;
    if (sce_jv_kind(held) != SCE_JSON_STRING) {
        return true;
    }
    for (i = 0u; i < lineage->added_count; ++i) {
        if (lineage->added[i].since > at && sce_jv_string_is(held, lineage->added[i].name)) {
            return sce_saved_refuse(err,
                                    "'%s' holds '%s', which revision %u of '%s' does not declare: revision %u added it",
                                    what, lineage->added[i].name, at, lineage->key, lineage->added[i].since);
        }
    }
    return true;
}

/* Refuses a value of `lineage`, an enum, that `session` holds and that names a variant a revision
 * past `at` added: judged on the values as saved, before a record is read with the defaults of the
 * fields later revisions added. */
static inline bool sce_saved_enum_holders_(const sce_saved_session_t *session, const sce_saved_lineage_t *lineage,
                                           unsigned at, sce_saved_error_t *err) {
    size_t h;
    for (h = 0u; h < lineage->holder_count; ++h) {
        const sce_saved_enum_holder_t *holder = &lineage->holders[h];
        char what[SCE_SAVED_WHAT_LEN];
        sce_jv_t value;
        sce_jv_t member;
        if (!sce_jv_get(session->variables, holder->variable, &value)) {
            continue;
        }
        if (holder->field == NULL) {
            if (!sce_saved_enum_value_(lineage, at, holder->variable, value, err)) {
                return false;
            }
        } else if (!holder->list) {
            if (sce_jv_kind(value) == SCE_JSON_OBJECT && sce_jv_get(value, holder->field, &member)) {
                (void)snprintf(what, sizeof(what), "%s.%s", holder->variable, holder->field);
                if (!sce_saved_enum_value_(lineage, at, what, member, err)) {
                    return false;
                }
            }
        } else if (sce_jv_kind(value) == SCE_JSON_ARRAY) {
            sce_jiter_t it;
            sce_jv_t item;
            size_t index = 0u;
            (void)sce_jv_iter(value, &it);
            for (; sce_jiter_element(&it, &item); ++index) {
                if (sce_jv_kind(item) == SCE_JSON_OBJECT && sce_jv_get(item, holder->field, &member)) {
                    (void)snprintf(what, sizeof(what), "%s[%lu].%s", holder->variable, (unsigned long)index,
                                   holder->field);
                    if (!sce_saved_enum_value_(lineage, at, what, member, err)) {
                        return false;
                    }
                }
            }
        }
    }
    return true;
}

/* Refuses `session` unless it was saved from a document of `shape`, and says at which revision of
 * each record schema and enum it was saved: `record_at[i]` for `shape->records[i]` and
 * `enum_at[i]` for `shape->enums[i]`, 1 for one the state names none of (docs/adr/0044).
 *
 * A state that names no revision is of the first shape; one that does is of the revised shape,
 * and each revision it names is of a lineage this machine has, no later than this machine's
 * revision of it, and declares what this machine's revision of that number declares. Judged
 * before any variable is read, so a state saved from another document is refused for that and
 * not for whichever variable it happens to lack. */
static inline bool sce_saved_check_shape(const sce_saved_session_t *session, const sce_saved_shape_t *shape,
                                         unsigned *record_at, unsigned *enum_at, sce_saved_error_t *err) {
    const char *first = shape->first;
    const char *revised = shape->revised;
    const char *want;
    sce_jiter_t it;
    sce_jv_t key;
    sce_jv_t value;
    size_t i;
    char written[SCE_SAVED_QUOTE_LEN];
    for (i = 0u; i < shape->record_count; ++i) {
        record_at[i] = 1u;
    }
    for (i = 0u; i < shape->enum_count; ++i) {
        enum_at[i] = 1u;
    }
    /* A format-2 state names an enum by its document and a format-1 state by the alias it is
     * imported under, so each is judged against its own shape (docs/adr/0045, section 2). */
    if (session->format == SCE_SAVED_FORMAT_2) {
        first = shape->format2_first;
        revised = shape->format2_revised;
    }
    sce_saved_quote_(session->shape, written, sizeof(written));
    if (!session->has_revisions) {
        want = first;
    } else if (sce_jv_string_is(session->shape, first)) {
        return sce_saved_refuse(err, "the saved state names revisions of its record schemas and has the shape of a "
                                     "state that names none");
    } else {
        want = revised;
    }
    if (!sce_jv_string_is(session->shape, want)) {
        return sce_saved_refuse(err, "the saved state is of a document of shape %s, and this machine's is %s", written,
                                want);
    }
    /* A state that names no revision has nothing to walk. */
    memset(&it, 0, sizeof(it));
    it.done = true;
    if (session->has_revisions) {
        (void)sce_jv_iter(session->revisions, &it);
    }
    while (sce_jiter_member(&it, &key, &value)) {
        const sce_saved_lineage_t *found = NULL;
        unsigned *at = NULL;
        sce_jv_t fields;
        char name[SCE_SAVED_QUOTE_LEN];
        int64_t revision = 0;
        size_t digest;
        sce_saved_error_t ignored;
        sce_saved_quote_(key, name, sizeof(name));
        (void)sce_saved_revision_read(value, "revisions", &revision, &fields, &ignored);
        /* A record schema by the event it declares, an enum by its key. */
        for (i = 0u; i < shape->record_count && found == NULL; ++i) {
            if (sce_jv_string_is(key, shape->records[i].key)) {
                found = &shape->records[i];
                at = &record_at[i];
            }
        }
        for (i = 0u; i < shape->enum_count && found == NULL; ++i) {
            if (sce_jv_string_is(key, shape->enums[i].key)) {
                found = &shape->enums[i];
                at = &enum_at[i];
            }
        }
        if (found == NULL) {
            return sce_saved_refuse(err, "'revisions' names '%s', which no record or enum of this machine is", name);
        }
        if (revision > (int64_t)found->revision) {
            return sce_saved_refuse(err,
                                    "the saved state was saved at revision %ld of '%s', later than this machine's %u: "
                                    "a machine does not restore what it does not know",
                                    (long)revision, name, found->revision);
        }
        /* The digest of revision r is the (r - 2)th: the first has none. */
        digest = (size_t)(revision - 2);
        if (revision < 2 || digest >= found->digest_count || !sce_jv_string_is(fields, found->digests[digest])) {
            return sce_saved_refuse(err,
                                    "revision %ld of '%s' declared other members where the state was saved than it "
                                    "does in this machine",
                                    (long)revision, name);
        }
        *at = (unsigned)revision;
    }
    /* An enum value saved at an earlier revision of its enum is one of that revision's variants. */
    for (i = 0u; i < shape->enum_count; ++i) {
        if (!sce_saved_enum_holders_(session, &shape->enums[i], enum_at[i], err)) {
            return false;
        }
    }
    return true;
}

/* ── Time ─────────────────────────────────────────────────────────────────── */

/* `base` and `add` added, held to what a signed 64-bit count can say. */
static inline int64_t sce_saved_saturating_add(int64_t base, int64_t add) {
    if (add > 0 && base > INT64_MAX - add) {
        return INT64_MAX;
    }
    return base + add;
}

/* The wall-clock moment, in milliseconds since the Unix epoch, a thing that comes due at
 * `ready_at_ms` of the engine's clock comes due at, given that `wall_now_ms` is what time it is
 * on the wall now and `now_ms` on the engine's clock: none earlier than now, for one already
 * past. */
static inline int64_t sce_saved_due_on(int64_t wall_now_ms, uint64_t now_ms, uint64_t ready_at_ms) {
    const uint64_t wait = ready_at_ms > now_ms ? ready_at_ms - now_ms : 0u;
    return sce_saved_saturating_add(wall_now_ms, wait > (uint64_t)INT64_MAX ? INT64_MAX : (int64_t)wait);
}

/* The moment of the engine's clock a thing saved as coming due at `due` comes due at, given that
 * `wall_now_ms` is what time it is on the wall clock the `due`s were written against and `now_ms`
 * on the engine's: `due - wall_now_ms` after now, and now for one that came due while the machine
 * was away. */
static inline uint64_t sce_saved_ready_at(int64_t wall_now_ms, uint64_t now_ms, int64_t due) {
    const uint64_t wait = due > wall_now_ms ? (uint64_t)(due - wall_now_ms) : 0u;
    return now_ms > UINT64_MAX - wait ? UINT64_MAX : now_ms + wait;
}

/* ── Writing ──────────────────────────────────────────────────────────────── */

/* `text` as a number written the way a saved state writes a count: the digits as a text. */
static inline void sce_saved_write_count(sce_jw_t *w, int64_t count) {
    char digits[24];
    (void)snprintf(digits, sizeof(digits), "%ld", (long)count);
    sce_jw_string(w, digits);
}

/* The `name` member of an object, a text. */
static inline void sce_saved_write_member_text(sce_jw_t *w, const char *name, const char *text) {
    sce_jw_key(w, name);
    sce_jw_string(w, text);
}

/* The parameters of a host-served send or invocation, as the text a saved state writes them:
 * an object by name, each name's values in the order they were given, and the names sorted so
 * the text one machine writes is every backend's. The pairs sit in two arrays of fixed-width
 * text slots, which is how a machine holds them. */
static inline void sce_saved_write_params(sce_jw_t *w, size_t count, const char *names, size_t name_stride,
                                          const char *values, size_t value_stride) {
    const char *floor = NULL; /* the last name written */
    size_t i;
    sce_jw_begin_object(w);
    for (;;) {
        const char *next = NULL;
        for (i = 0u; i < count; ++i) {
            const char *name = names + i * name_stride;
            if ((floor == NULL || strcmp(name, floor) > 0) && (next == NULL || strcmp(name, next) < 0)) {
                next = name;
            }
        }
        if (next == NULL) {
            break;
        }
        sce_jw_key(w, next);
        sce_jw_begin_array(w);
        for (i = 0u; i < count; ++i) {
            if (strcmp(names + i * name_stride, next) == 0) {
                sce_jw_element(w);
                sce_jw_string(w, values + i * value_stride);
            }
        }
        sce_jw_end_array(w);
        floor = next;
    }
    sce_jw_end_object(w);
}

/* A waiting send, as the members of its act. A text a kind of act does not carry is not written. */
typedef struct sce_saved_send_out_s {
    int64_t due;
    sce_saved_act_t act;
    const char *event;
    const char *data;
    const char *send_id;
    const char *origin;
    const char *invoke_id;
    const char *type;
    const char *target;
    const char *content;
    size_t param_count;
    const char *param_names;
    size_t param_name_stride;
    const char *param_values;
    size_t param_value_stride;
} sce_saved_send_out_t;

static inline void sce_saved_write_send(sce_jw_t *w, const sce_saved_send_out_t *send) {
    sce_jw_begin_object(w);
    sce_jw_key(w, "due");
    sce_saved_write_count(w, send->due);
    sce_saved_write_member_text(w, "act", sce_saved_act_name(send->act));
    if (send->act == SCE_SAVED_ACT_HOST) {
        sce_saved_write_member_text(w, "type", send->type);
        sce_saved_write_member_text(w, "event", send->event);
        sce_saved_write_member_text(w, "target", send->target);
        sce_saved_write_member_text(w, "content", send->content);
        sce_jw_key(w, "params");
        sce_saved_write_params(w, send->param_count, send->param_names, send->param_name_stride, send->param_values,
                               send->param_value_stride);
        sce_saved_write_member_text(w, "sendid", send->send_id);
        sce_saved_write_member_text(w, "data", send->data);
        sce_saved_write_member_text(w, "invokeid", send->invoke_id);
    } else {
        if (send->act == SCE_SAVED_ACT_CHILD) {
            sce_saved_write_member_text(w, "invokeid", send->invoke_id);
        }
        sce_saved_write_member_text(w, "event", send->event);
        sce_saved_write_member_text(w, "data", send->data);
        sce_saved_write_member_text(w, "sendid", send->send_id);
        sce_saved_write_member_text(w, "origin", send->origin);
    }
    sce_jw_end_object(w);
}

/* A queued event. */
typedef struct sce_saved_event_out_s {
    const char *name;
    const char *data;
    const char *type;
    const char *send_id;
    const char *origin;
    const char *origin_type;
    const char *invoke_id;
    bool has_host_invoke_token;
    uint64_t host_invoke_token;
} sce_saved_event_out_t;

static inline void sce_saved_write_event(sce_jw_t *w, const sce_saved_event_out_t *event) {
    sce_jw_begin_object(w);
    sce_saved_write_member_text(w, "name", event->name);
    sce_saved_write_member_text(w, "data", event->data);
    sce_saved_write_member_text(w, "type", event->type);
    sce_saved_write_member_text(w, "sendid", event->send_id);
    sce_saved_write_member_text(w, "origin", event->origin);
    sce_saved_write_member_text(w, "origintype", event->origin_type);
    sce_saved_write_member_text(w, "invokeid", event->invoke_id);
    sce_jw_key(w, "hostinvoketoken");
    if (event->has_host_invoke_token) {
        sce_saved_write_count(w, (int64_t)event->host_invoke_token);
    } else {
        sce_jw_null(w);
    }
    sce_jw_end_object(w);
}

/* A host-run invocation, as the request it was started with. */
typedef struct sce_saved_host_invoke_out_s {
    const char *type;
    const char *id;
    const char *src;
    const char *data;
    const char *content;
    bool has_due;
    int64_t due;
    size_t param_count;
    const char *param_names;
    size_t param_name_stride;
    const char *param_values;
    size_t param_value_stride;
} sce_saved_host_invoke_out_t;

static inline void sce_saved_write_host_invoke(sce_jw_t *w, const sce_saved_host_invoke_out_t *host) {
    sce_jw_begin_object(w);
    sce_saved_write_member_text(w, "type", host->type);
    sce_saved_write_member_text(w, "id", host->id);
    sce_saved_write_member_text(w, "src", host->src);
    sce_jw_key(w, "params");
    sce_saved_write_params(w, host->param_count, host->param_names, host->param_name_stride, host->param_values,
                           host->param_value_stride);
    sce_saved_write_member_text(w, "data", host->data);
    sce_saved_write_member_text(w, "content", host->content);
    sce_jw_key(w, "due");
    if (host->has_due) {
        sce_saved_write_count(w, host->due);
    } else {
        sce_jw_null(w);
    }
    sce_jw_end_object(w);
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_SAVED_STATE_H */
