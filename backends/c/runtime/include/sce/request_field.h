// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A typed host-run request (`sce:request`, SCE Accepted Subset §2.12).
//
// A request crosses to the host as text, like every `<param>`. What makes it
// typed is that each value is held to its field's type where the invocation
// starts — `sce_request_field_wire` — so the text a host reads back is always
// one its field's type parses. The C11 port of
// `sce_rust_runtime::host_processor`'s typed-request half; the rule is the
// same in every runtime.
//
// Script-engine free: the generated start site reads the evaluated value out
// of Lua into an `sce_script_scalar_t` (the `lua_eval_to_request_field` macro
// in `tools/codegen/templates/c/scriptengine.jinja2`), and the judgement is
// made here on that neutral view. The refusals are `event_payload.h`'s
// sentences, because the lift that reads a completion not fitting its record
// makes the same judgement on the other half of the same invocation.

#ifndef SCE_REQUEST_FIELD_H
#define SCE_REQUEST_FIELD_H

#include <math.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sce/event_payload.h"
#include "sce/host_processor.h"

#ifdef __cplusplus
extern "C" {
#endif

#define SCE_REQUEST_REFUSE_NOT_NUMBER SCE_PAYLOAD_REFUSE_NOT_NUMBER
#define SCE_REQUEST_REFUSE_NOT_FINITE "is not a finite number"
#define SCE_REQUEST_REFUSE_NOT_BYTES "is not a byte string"
#define SCE_REQUEST_REFUSE_PAST_CAP "is longer than its schema's sce:max-size"

/** The type one field of a typed request declares. */
typedef enum sce_request_field_kind_e {
    SCE_REQUEST_FIELD_UINT8,
    SCE_REQUEST_FIELD_UINT16,
    SCE_REQUEST_FIELD_UINT32,
    SCE_REQUEST_FIELD_UINT64,
    SCE_REQUEST_FIELD_INT8,
    SCE_REQUEST_FIELD_INT16,
    SCE_REQUEST_FIELD_INT32,
    SCE_REQUEST_FIELD_INT64,
    SCE_REQUEST_FIELD_FLOAT32,
    SCE_REQUEST_FIELD_FLOAT64,
    SCE_REQUEST_FIELD_BOOL,
    SCE_REQUEST_FIELD_STRING,
    SCE_REQUEST_FIELD_BYTES
} sce_request_field_kind_t;

typedef struct sce_request_field_type_s {
    sce_request_field_kind_t kind;
    /** A `bytes` field's `sce:max-size`; zero for every other kind. */
    size_t cap;
} sce_request_field_type_t;

/** What the data model evaluated one `<param>` to, read out of the script
    engine without keeping it: the start site fills this and the check reads
    it before the engine's value is released. */
typedef enum sce_script_scalar_kind_e {
    SCE_SCRIPT_SCALAR_NIL,
    SCE_SCRIPT_SCALAR_BOOL,
    SCE_SCRIPT_SCALAR_INTEGER,
    SCE_SCRIPT_SCALAR_FLOAT,
    SCE_SCRIPT_SCALAR_STRING,
    SCE_SCRIPT_SCALAR_OTHER
} sce_script_scalar_kind_t;

typedef struct sce_script_scalar_s {
    sce_script_scalar_kind_t kind;
    bool boolean;
    int64_t integer;
    double floating;
    const char *text;
    size_t text_len;
} sce_script_scalar_t;

/* A whole number as a sign and a magnitude, so every 64-bit width — signed
   and unsigned — is judged without a wider integer type. `beyond` is past
   every 64-bit width. */
typedef struct {
    bool negative;
    uint64_t magnitude;
    bool beyond;
} sce_request_whole_t;

SCE_C_UNUSED static inline bool sce_request_whole_of(const sce_script_scalar_t *v, sce_request_whole_t *out) {
    memset(out, 0, sizeof(*out));
    if (v->kind == SCE_SCRIPT_SCALAR_INTEGER) {
        out->negative = v->integer < 0;
        out->magnitude = out->negative ? (uint64_t)0 - (uint64_t)v->integer : (uint64_t)v->integer;
        return true;
    }
    if (v->kind == SCE_SCRIPT_SCALAR_FLOAT) {
        const double two63 = 9223372036854775808.0;
        const double d = v->floating;
        double m;
        if (!isfinite(d) || d != trunc(d)) {
            return false;
        }
        m = fabs(d);
        if (m >= 2.0 * two63) {
            out->negative = d < 0;
            out->beyond = true;
            return true;
        }
        /* Below 2^64 every whole double converts exactly; at or past 2^63 it
           is even, so its half does too. */
        out->magnitude = m >= two63 ? (uint64_t)(m / 2.0) * 2u : (uint64_t)m;
        out->negative = d < 0 && out->magnitude != 0;
        return true;
    }
    return false;
}

SCE_C_UNUSED static inline bool sce_request_whole_fits(const sce_request_whole_t *w, sce_request_field_kind_t kind) {
    uint64_t max = 0;
    bool is_signed = false;
    if (w->beyond) {
        return false;
    }
    switch (kind) {
    case SCE_REQUEST_FIELD_UINT8:
        max = UINT8_MAX;
        break;
    case SCE_REQUEST_FIELD_UINT16:
        max = UINT16_MAX;
        break;
    case SCE_REQUEST_FIELD_UINT32:
        max = UINT32_MAX;
        break;
    case SCE_REQUEST_FIELD_UINT64:
        max = UINT64_MAX;
        break;
    case SCE_REQUEST_FIELD_INT8:
        max = INT8_MAX;
        is_signed = true;
        break;
    case SCE_REQUEST_FIELD_INT16:
        max = INT16_MAX;
        is_signed = true;
        break;
    case SCE_REQUEST_FIELD_INT32:
        max = INT32_MAX;
        is_signed = true;
        break;
    case SCE_REQUEST_FIELD_INT64:
        max = INT64_MAX;
        is_signed = true;
        break;
    default:
        return false;
    }
    if (is_signed) {
        return w->negative ? w->magnitude <= max + 1u : w->magnitude <= max;
    }
    return !w->negative && w->magnitude <= max;
}

/**
 * Hold one evaluated `<param>` to the field it supplies, and spell it into
 * `out` for the request.
 *
 * Returns NULL when it fits, else the refusal — a sentence without the
 * field's name, which the caller prefixes as the payload lift does.
 *
 * §scxml-6.4.1: an argument that cannot be evaluated starts nothing, and a
 * value the record's field cannot hold is such an argument — the host was
 * promised that record.
 *
 * The text is the value at the field's type — a whole number's digits, a
 * fraction as this backend's untyped `<param>` spells one (the Lua `tostring`
 * the `lua_eval_to_wire_string` macro renders: an integral value's digits,
 * else `%.14g`) — which is what the `sce_request_read_*` readers parse back,
 * so the adapter reading a checked request cannot fail. A byte string rides as
 * its text, each character up to U+00FF one byte, the spelling a completion's
 * byte field uses.
 */
SCE_C_UNUSED static inline const char *sce_request_field_wire(const sce_script_scalar_t *v,
                                                              sce_request_field_type_t type, char *out, size_t cap) {
    sce_request_whole_t whole;
    if (cap == 0u) {
        return SCE_PAYLOAD_REFUSE_TOO_LONG;
    }
    out[0] = '\0';
    switch (type.kind) {
    case SCE_REQUEST_FIELD_FLOAT32:
    case SCE_REQUEST_FIELD_FLOAT64: {
        double d;
        int n;
        if (v->kind == SCE_SCRIPT_SCALAR_INTEGER) {
            d = (double)v->integer;
        } else if (v->kind == SCE_SCRIPT_SCALAR_FLOAT) {
            d = v->floating;
        } else {
            return SCE_REQUEST_REFUSE_NOT_NUMBER;
        }
        /* JSON, which a completion's record crosses as, has no spelling for
           these, so a request may not carry one either. */
        if (!isfinite(d)) {
            return SCE_REQUEST_REFUSE_NOT_FINITE;
        }
        if (type.kind == SCE_REQUEST_FIELD_FLOAT32) {
            if (fabs(d) > (double)3.40282346638528859811704183484516925e+38) {
                return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
            }
            d = (double)(float)d;
        }
        if (d == trunc(d) && fabs(d) < 9223372036854775808.0) {
            n = snprintf(out, cap, "%lld", (long long)d);
        } else {
            n = snprintf(out, cap, "%.14g", d);
        }
        return (n < 0 || (size_t)n >= cap) ? SCE_PAYLOAD_REFUSE_TOO_LONG : NULL;
    }
    case SCE_REQUEST_FIELD_BOOL:
        if (v->kind != SCE_SCRIPT_SCALAR_BOOL) {
            return SCE_PAYLOAD_REFUSE_NOT_TRUTH;
        }
        return snprintf(out, cap, "%s", v->boolean ? "true" : "false") >= (int)cap ? SCE_PAYLOAD_REFUSE_TOO_LONG : NULL;
    case SCE_REQUEST_FIELD_STRING:
    case SCE_REQUEST_FIELD_BYTES: {
        if (v->kind != SCE_SCRIPT_SCALAR_STRING) {
            return type.kind == SCE_REQUEST_FIELD_STRING ? SCE_PAYLOAD_REFUSE_NOT_TEXT : SCE_REQUEST_REFUSE_NOT_BYTES;
        }
        if (type.kind == SCE_REQUEST_FIELD_BYTES) {
            /* The script engine hands text as UTF-8; each character up to
               U+00FF is one byte of the Latin-1 spelling. */
            size_t i = 0u;
            size_t chars = 0u;
            while (i < v->text_len) {
                const unsigned char lead = (unsigned char)v->text[i];
                if (lead < 0x80u) {
                    i += 1u;
                } else if (lead == 0xC2u || lead == 0xC3u) {
                    i += 2u;
                } else {
                    return SCE_PAYLOAD_REFUSE_NOT_ONE_BYTE;
                }
                chars++;
            }
            if (chars > type.cap) {
                return SCE_REQUEST_REFUSE_PAST_CAP;
            }
        }
        if (v->text_len >= cap) {
            return SCE_PAYLOAD_REFUSE_TOO_LONG;
        }
        memcpy(out, v->text, v->text_len);
        out[v->text_len] = '\0';
        return NULL;
    }
    default:
        break;
    }
    if (!sce_request_whole_of(v, &whole)) {
        return (v->kind == SCE_SCRIPT_SCALAR_INTEGER || v->kind == SCE_SCRIPT_SCALAR_FLOAT)
                   ? SCE_PAYLOAD_REFUSE_NOT_WHOLE
                   : SCE_REQUEST_REFUSE_NOT_NUMBER;
    }
    if (!sce_request_whole_fits(&whole, type.kind)) {
        return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
    }
    {
        const int n = snprintf(out, cap, "%s%llu", whole.negative ? "-" : "", (unsigned long long)whole.magnitude);
        return (n < 0 || (size_t)n >= cap) ? SCE_PAYLOAD_REFUSE_TOO_LONG : NULL;
    }
}

/* ── The adapter's reading of a checked request ─────────────────────────── */

/* The one text a typed request carries for `name`, or NULL. A request that
   reached a typed adapter was checked field by field where it started, so a
   NULL here — or a reader answering false below — is a broken promise between
   two halves of generated code, not a value a host or document can supply;
   the generated adapter reports it and does not start the invocation. */
SCE_C_UNUSED static inline const char *sce_request_text(const sce_host_invoke_event_t *event, const char *name) {
    int i;
    const char *found = NULL;
    for (i = 0; i < event->param_count; i++) {
        if (strcmp(event->params[i].name, name) == 0) {
            if (found != NULL) {
                return NULL;
            }
            found = event->params[i].value;
        }
    }
    return found;
}

SCE_C_UNUSED static inline bool sce_request_read_whole(const sce_host_invoke_event_t *event, const char *name,
                                                       sce_request_field_kind_t kind, bool *negative,
                                                       uint64_t *magnitude) {
    const char *text = sce_request_text(event, name);
    const char *p;
    sce_request_whole_t whole;
    if (text == NULL) {
        return false;
    }
    memset(&whole, 0, sizeof(whole));
    p = text;
    if (*p == '-') {
        whole.negative = true;
        p++;
    }
    if (*p == '\0') {
        return false;
    }
    for (; *p != '\0'; p++) {
        const uint64_t digit = (uint64_t)(*p - '0');
        if (*p < '0' || *p > '9' || whole.magnitude > (UINT64_MAX - digit) / 10u) {
            return false;
        }
        whole.magnitude = whole.magnitude * 10u + digit;
    }
    if (!sce_request_whole_fits(&whole, kind)) {
        return false;
    }
    *negative = whole.negative;
    *magnitude = whole.magnitude;
    return true;
}

#define SCE_REQUEST_READ_UNSIGNED(suffix, type, kind)                                                                  \
    SCE_C_UNUSED static inline bool sce_request_read_##suffix(const sce_host_invoke_event_t *event, const char *name,  \
                                                              type *out) {                                             \
        bool negative = false;                                                                                         \
        uint64_t magnitude = 0u;                                                                                       \
        if (!sce_request_read_whole(event, name, kind, &negative, &magnitude)) {                                       \
            return false;                                                                                              \
        }                                                                                                              \
        *out = (type)magnitude;                                                                                        \
        return true;                                                                                                   \
    }

#define SCE_REQUEST_READ_SIGNED(suffix, type, kind)                                                                    \
    SCE_C_UNUSED static inline bool sce_request_read_##suffix(const sce_host_invoke_event_t *event, const char *name,  \
                                                              type *out) {                                             \
        bool negative = false;                                                                                         \
        uint64_t magnitude = 0u;                                                                                       \
        if (!sce_request_read_whole(event, name, kind, &negative, &magnitude)) {                                       \
            return false;                                                                                              \
        }                                                                                                              \
        *out = (negative && magnitude != 0u) ? (type)(-(int64_t)(magnitude - 1u) - 1) : (type)magnitude;               \
        return true;                                                                                                   \
    }

SCE_REQUEST_READ_UNSIGNED(u8, uint8_t, SCE_REQUEST_FIELD_UINT8)
SCE_REQUEST_READ_UNSIGNED(u16, uint16_t, SCE_REQUEST_FIELD_UINT16)
SCE_REQUEST_READ_UNSIGNED(u32, uint32_t, SCE_REQUEST_FIELD_UINT32)
SCE_REQUEST_READ_UNSIGNED(u64, uint64_t, SCE_REQUEST_FIELD_UINT64)
SCE_REQUEST_READ_SIGNED(i8, int8_t, SCE_REQUEST_FIELD_INT8)
SCE_REQUEST_READ_SIGNED(i16, int16_t, SCE_REQUEST_FIELD_INT16)
SCE_REQUEST_READ_SIGNED(i32, int32_t, SCE_REQUEST_FIELD_INT32)
SCE_REQUEST_READ_SIGNED(i64, int64_t, SCE_REQUEST_FIELD_INT64)

SCE_C_UNUSED static inline bool sce_request_read_f64(const sce_host_invoke_event_t *event, const char *name,
                                                     double *out) {
    const char *text = sce_request_text(event, name);
    char *end = NULL;
    if (text == NULL || *text == '\0') {
        return false;
    }
    *out = strtod(text, &end);
    return *end == '\0';
}

SCE_C_UNUSED static inline bool sce_request_read_f32(const sce_host_invoke_event_t *event, const char *name,
                                                     float *out) {
    double d = 0.0;
    if (!sce_request_read_f64(event, name, &d)) {
        return false;
    }
    *out = (float)d;
    return true;
}

SCE_C_UNUSED static inline bool sce_request_read_bool(const sce_host_invoke_event_t *event, const char *name,
                                                      bool *out) {
    const char *text = sce_request_text(event, name);
    if (text == NULL) {
        return false;
    }
    if (strcmp(text, "true") == 0) {
        *out = true;
        return true;
    }
    if (strcmp(text, "false") == 0) {
        *out = false;
        return true;
    }
    return false;
}

/* A text field: borrowed from the request, valid for the start call. */
SCE_C_UNUSED static inline bool sce_request_read_text(const sce_host_invoke_event_t *event, const char *name,
                                                      const char **out) {
    *out = sce_request_text(event, name);
    return *out != NULL;
}

/* A byte-string field, from the Latin-1 spelling `sce_request_field_wire`
   checked, into `out` of `cap` bytes. */
SCE_C_UNUSED static inline bool sce_request_read_bytes(const sce_host_invoke_event_t *event, const char *name,
                                                       uint8_t *out, size_t cap, size_t *len) {
    const char *text = sce_request_text(event, name);
    size_t i = 0u;
    size_t n = 0u;
    if (text == NULL) {
        return false;
    }
    while (text[i] != '\0') {
        const unsigned char lead = (unsigned char)text[i];
        uint8_t byte;
        if (lead < 0x80u) {
            byte = lead;
            i += 1u;
        } else if ((lead == 0xC2u || lead == 0xC3u) && text[i + 1u] != '\0') {
            byte = (uint8_t)(((lead & 0x03u) << 6) | ((unsigned char)text[i + 1u] & 0x3Fu));
            i += 2u;
        } else {
            return false;
        }
        if (n >= cap) {
            return false;
        }
        out[n++] = byte;
    }
    *len = n;
    return true;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_REQUEST_FIELD_H */
