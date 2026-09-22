/* SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
   SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

   Lifting an event's typed `_event.data` view out of the data it carries.

   NL→IR Item C1 Path A gives a schema'd event a typed payload that the
   natively lowered guards read. One producer fills it: the generated
   `<machine>_raise_<event>_typed` inject seam. Every other producer — `<send>`
   with `<param>`, namelist or `<content>`, an invoke forwarding an event either
   way, autoforward, BasicHTTP, mesh — fills the event record's `data`, the wire
   §scxml-5.10 describes and §scxml-B-2-8-1 reads.

   Until this header the two never met, so the same guard answered differently
   depending on where its event came from, and a typed payload could not cross
   an invoke boundary at all. What the lift refuses is what the SCRIPT ENGINE
   already refuses for the same guard, measured on the same document: no data, a
   missing field, or a value of another type each give `error.execution` and a
   guard that does not fire (§scxml-3.13). A native lowering that answered
   differently would make the optimisation observable, which is the one thing it
   may not be.

   ⚠ HEADER-ONLY, and that is a requirement rather than a convenience: the C11
   EventSchema gate links NO runtime library, because a native value path that
   links nothing is the MCU-clean proof the whole lowering exists for. A lift in
   a library would have taken that proof away from every machine that uses it.
   `static inline` keeps it in the one translation unit each machine already is.

   ⚠ No allocator and no copy of the payload either: the fields keep a pointer
   into the event record's own `data`, and each read scans it for one name. A
   payload is tens of bytes and a schema names a handful of fields, so the scan
   is bounded by the same small numbers either way. What it does take from the C
   library is `<stdlib.h>`'s `strtoull` / `strtod` and `<string.h>`'s `strncmp`,
   which every freestanding toolchain this backend targets provides.

   ⚠ Every refusal is a STATIC sentence about the field, with no name in it:
   the caller composes "`<event>` payload: '<field>' <reason>" from its own
   generation-time literals, because this backend has no allocator to build a
   message with.

   Cross-language siblings: `sce_runtime.event_payload` (Python),
   `sce.LiftPayload` (Go), `SCE::Common::EventPayloadFields` (C++). */

#ifndef SCE_EVENT_PAYLOAD_H
#define SCE_EVENT_PAYLOAD_H

#include <errno.h>
#include <float.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "sce/types.h"

#ifdef __cplusplus
extern "C" {
#endif

/* ── The refusals, spelled once ─────────────────────────────────────────── */

#define SCE_PAYLOAD_REFUSE_NO_DATA "the event carries no data"
#define SCE_PAYLOAD_REFUSE_NOT_OBJECT "the event's data is not an object of named fields"
#define SCE_PAYLOAD_REFUSE_MISSING "is missing from the event's data"
#define SCE_PAYLOAD_REFUSE_NOT_NUMBER "is not a number"
#define SCE_PAYLOAD_REFUSE_NOT_WHOLE "is not a whole number"
#define SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH "does not fit the width its schema declares"
#define SCE_PAYLOAD_REFUSE_NOT_TRUTH "is not a truth value"
#define SCE_PAYLOAD_REFUSE_NOT_TEXT "is not a text"
#define SCE_PAYLOAD_REFUSE_TOO_LONG "is longer than this build reserved room for"
#define SCE_PAYLOAD_REFUSE_NOT_ONE_BYTE "carries a character above U+00FF, which no single byte spells"

/* The fields an event's data names — a borrowed view, valid only while the
   event record it came from is. */
typedef struct {
    const char *object; /* at the '{' of the payload object */
} sce_payload_fields_t;

/* ── Walking one payload ────────────────────────────────────────────────── */

SCE_C_UNUSED static inline const char *sce_payload_skip_space(const char *p) {
    while (*p == ' ' || *p == '\t' || *p == '\r' || *p == '\n') {
        p++;
    }
    return p;
}

/* Past one JSON string, from its opening quote. NULL when it never closes. */
SCE_C_UNUSED static inline const char *sce_payload_skip_string(const char *p) {
    if (*p != '"') {
        return NULL;
    }
    p++;
    while (*p != '\0') {
        if (*p == '\\') {
            if (p[1] == '\0') {
                return NULL;
            }
            p += 2;
            continue;
        }
        if (*p == '"') {
            return p + 1;
        }
        p++;
    }
    return NULL;
}

/* Past one object or array, counting nesting. A payload may carry fields this
   document's schema does not name, and a nested one still has to be walked past
   to reach the fields that follow. */
SCE_C_UNUSED static inline const char *sce_payload_skip_container(const char *p) {
    const char open = *p;
    const char close = (open == '{') ? '}' : ']';
    int depth = 0;
    while (*p != '\0') {
        if (*p == '"') {
            const char *after = sce_payload_skip_string(p);
            if (after == NULL) {
                return NULL;
            }
            p = after;
            continue;
        }
        if (*p == open) {
            depth++;
        } else if (*p == close) {
            depth--;
            if (depth == 0) {
                return p + 1;
            }
        }
        p++;
    }
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_skip_value(const char *p) {
    p = sce_payload_skip_space(p);
    if (*p == '"') {
        return sce_payload_skip_string(p);
    }
    if (*p == '{' || *p == '[') {
        return sce_payload_skip_container(p);
    }
    /* A number, or one of the three keywords. Either way it ends where the next
       member does. */
    while (*p != '\0' && *p != ',' && *p != '}' && *p != ']') {
        p++;
    }
    return p;
}

/* Whether the JSON string starting at `p` spells exactly `name`. */
SCE_C_UNUSED static inline bool sce_payload_string_is(const char *p, const char *name) {
    p++; /* past the opening quote */
    while (*name != '\0') {
        if (*p == '\0' || *p == '"') {
            return false;
        }
        if (*p == '\\') {
            /* An escaped field name is legal JSON and no schema writes one, so
               it cannot be the name being looked for. */
            return false;
        }
        if (*p != *name) {
            return false;
        }
        p++;
        name++;
    }
    return *p == '"';
}

/* The value `name` carries, or NULL when the object does not name it. */
SCE_C_UNUSED static inline const char *sce_payload_find(const sce_payload_fields_t *fields, const char *name) {
    if (fields == NULL || fields->object == NULL) {
        return NULL;
    }
    const char *p = sce_payload_skip_space(fields->object + 1); /* past '{' */
    while (*p != '\0' && *p != '}') {
        if (*p != '"') {
            return NULL;
        }
        const bool matched = sce_payload_string_is(p, name);
        const char *after_key = sce_payload_skip_string(p);
        if (after_key == NULL) {
            return NULL;
        }
        p = sce_payload_skip_space(after_key);
        if (*p != ':') {
            return NULL;
        }
        p = sce_payload_skip_space(p + 1);
        if (matched) {
            return p;
        }
        const char *after_value = sce_payload_skip_value(p);
        if (after_value == NULL) {
            return NULL;
        }
        p = sce_payload_skip_space(after_value);
        if (*p == ',') {
            p = sce_payload_skip_space(p + 1);
            continue;
        }
        break;
    }
    return NULL;
}

/* Read an event's data as the fields it names.

   Returns NULL when `out` was filled, else a static sentence saying why not.
   The JSON read is §scxml-B-2-8-1's second rung, the same one the script engine
   takes for the same string. */
SCE_C_UNUSED static inline const char *sce_payload_decode(const char *data, sce_payload_fields_t *out) {
    if (out == NULL) {
        return SCE_PAYLOAD_REFUSE_NOT_OBJECT;
    }
    out->object = NULL;
    if (data == NULL) {
        return SCE_PAYLOAD_REFUSE_NO_DATA;
    }
    const char *p = sce_payload_skip_space(data);
    if (*p == '\0') {
        return SCE_PAYLOAD_REFUSE_NO_DATA;
    }
    if (*p != '{') {
        /* §scxml-B-2-8-1's other rungs: a bare value or a space-normalized
           string names no fields. */
        return SCE_PAYLOAD_REFUSE_NOT_OBJECT;
    }
    out->object = p;
    return NULL;
}

/* ── Reading one field at its declared type ─────────────────────────────── */

/* A whole number, as wide as this backend reads them, with the refusals the
   width-specific readers narrow further.

   ⚠ A truth value is not a number here. JSON spells both, and a schema that
   declared `uint32` and received `true` has been handed something its own type
   says cannot occur. */
SCE_C_UNUSED static inline const char *sce_payload_read_whole(const sce_payload_fields_t *fields, const char *name,
                                                              bool *negative, uint64_t *magnitude) {
    const char *p = sce_payload_find(fields, name);
    if (p == NULL) {
        return SCE_PAYLOAD_REFUSE_MISSING;
    }
    if (*p != '-' && (*p < '0' || *p > '9')) {
        return SCE_PAYLOAD_REFUSE_NOT_NUMBER;
    }
    *negative = (*p == '-');
    const char *digits = *negative ? p + 1 : p;
    if (*digits < '0' || *digits > '9') {
        return SCE_PAYLOAD_REFUSE_NOT_NUMBER;
    }
    errno = 0;
    char *end = NULL;
    const unsigned long long value = strtoull(digits, &end, 10);
    if (end == digits || errno == ERANGE) {
        return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
    }
    /* A fraction or an exponent is a different value, and rounding it would
       answer a guard for something nobody sent. */
    if (*end == '.' || *end == 'e' || *end == 'E') {
        return SCE_PAYLOAD_REFUSE_NOT_WHOLE;
    }
    *magnitude = (uint64_t)value;
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_read_signed(const sce_payload_fields_t *fields, const char *name,
                                                               int64_t min, int64_t max, int64_t *out) {
    bool negative = false;
    uint64_t magnitude = 0;
    const char *refusal = sce_payload_read_whole(fields, name, &negative, &magnitude);
    if (refusal != NULL) {
        return refusal;
    }
    if (negative) {
        /* -(min) does not fit an int64_t when min is its floor, so the
           comparison is made on the magnitude instead. */
        const uint64_t floor_magnitude = (uint64_t)(-(min + 1)) + 1u;
        if (magnitude > floor_magnitude) {
            return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
        }
        *out = (magnitude == floor_magnitude) ? min : -(int64_t)magnitude;
        return NULL;
    }
    if (magnitude > (uint64_t)max) {
        return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
    }
    *out = (int64_t)magnitude;
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_read_unsigned(const sce_payload_fields_t *fields, const char *name,
                                                                 uint64_t max, uint64_t *out) {
    bool negative = false;
    uint64_t magnitude = 0;
    const char *refusal = sce_payload_read_whole(fields, name, &negative, &magnitude);
    if (refusal != NULL) {
        return refusal;
    }
    if (negative || magnitude > max) {
        return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
    }
    *out = magnitude;
    return NULL;
}

/* One reader per declared width. A value the width cannot hold is refused
   rather than wrapped — silently truncating is how a guard on a boundary
   answers for a value the document never received. */
#define SCE_PAYLOAD_SIGNED_READER(suffix, type, min, max)                                                              \
    SCE_C_UNUSED static inline const char *sce_payload_read_##suffix(const sce_payload_fields_t *fields,               \
                                                                     const char *name, type *out) {                    \
        int64_t wide = 0;                                                                                              \
        const char *refusal = sce_payload_read_signed(fields, name, (min), (max), &wide);                              \
        if (refusal != NULL) {                                                                                         \
            return refusal;                                                                                            \
        }                                                                                                              \
        *out = (type)wide;                                                                                             \
        return NULL;                                                                                                   \
    }

#define SCE_PAYLOAD_UNSIGNED_READER(suffix, type, max)                                                                 \
    SCE_C_UNUSED static inline const char *sce_payload_read_##suffix(const sce_payload_fields_t *fields,               \
                                                                     const char *name, type *out) {                    \
        uint64_t wide = 0;                                                                                             \
        const char *refusal = sce_payload_read_unsigned(fields, name, (max), &wide);                                   \
        if (refusal != NULL) {                                                                                         \
            return refusal;                                                                                            \
        }                                                                                                              \
        *out = (type)wide;                                                                                             \
        return NULL;                                                                                                   \
    }

SCE_PAYLOAD_SIGNED_READER(i8, int8_t, INT8_MIN, INT8_MAX)
SCE_PAYLOAD_SIGNED_READER(i16, int16_t, INT16_MIN, INT16_MAX)
SCE_PAYLOAD_SIGNED_READER(i32, int32_t, INT32_MIN, INT32_MAX)
SCE_PAYLOAD_SIGNED_READER(i64, int64_t, INT64_MIN, INT64_MAX)
SCE_PAYLOAD_UNSIGNED_READER(u8, uint8_t, UINT8_MAX)
SCE_PAYLOAD_UNSIGNED_READER(u16, uint16_t, UINT16_MAX)
SCE_PAYLOAD_UNSIGNED_READER(u32, uint32_t, UINT32_MAX)
SCE_PAYLOAD_UNSIGNED_READER(u64, uint64_t, UINT64_MAX)

SCE_C_UNUSED static inline const char *sce_payload_read_floating(const sce_payload_fields_t *fields, const char *name,
                                                                 double *out) {
    const char *p = sce_payload_find(fields, name);
    if (p == NULL) {
        return SCE_PAYLOAD_REFUSE_MISSING;
    }
    if (*p != '-' && (*p < '0' || *p > '9')) {
        return SCE_PAYLOAD_REFUSE_NOT_NUMBER;
    }
    errno = 0;
    char *end = NULL;
    const double value = strtod(p, &end);
    if (end == p) {
        return SCE_PAYLOAD_REFUSE_NOT_NUMBER;
    }
    if (errno == ERANGE) {
        return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
    }
    *out = value;
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_read_f32(const sce_payload_fields_t *fields, const char *name,
                                                            float *out) {
    double wide = 0.0;
    const char *refusal = sce_payload_read_floating(fields, name, &wide);
    if (refusal != NULL) {
        return refusal;
    }
    if (wide > (double)FLT_MAX || wide < -(double)FLT_MAX) {
        return SCE_PAYLOAD_REFUSE_NOT_IN_WIDTH;
    }
    *out = (float)wide;
    return NULL;
}

SCE_C_UNUSED static inline const char *sce_payload_read_f64(const sce_payload_fields_t *fields, const char *name,
                                                            double *out) {
    return sce_payload_read_floating(fields, name, out);
}

SCE_C_UNUSED static inline const char *sce_payload_read_bool(const sce_payload_fields_t *fields, const char *name,
                                                             bool *out) {
    const char *p = sce_payload_find(fields, name);
    if (p == NULL) {
        return SCE_PAYLOAD_REFUSE_MISSING;
    }
    if (strncmp(p, "true", 4) == 0) {
        *out = true;
        return NULL;
    }
    if (strncmp(p, "false", 5) == 0) {
        *out = false;
        return NULL;
    }
    return SCE_PAYLOAD_REFUSE_NOT_TRUTH;
}

/* Where one JSON string's characters go as they are unescaped. `emit` answers
   false when it has no room left, or when the character has no one-byte
   spelling and the sink is a byte string. */
typedef struct {
    char *text;     /* set for a text field */
    uint8_t *bytes; /* set for a byte-string field */
    size_t cap;
    size_t len;
    bool above_one_byte;
} sce_payload_sink_t;

SCE_C_UNUSED static inline bool sce_payload_emit(sce_payload_sink_t *sink, unsigned long code) {
    if (sink->bytes != NULL) {
        if (code > 0xFFu) {
            sink->above_one_byte = true;
            return false;
        }
        if (sink->len >= sink->cap) {
            return false;
        }
        sink->bytes[sink->len++] = (uint8_t)code;
        return true;
    }
    if (sink->len + 1u >= sink->cap) {
        return false;
    }
    sink->text[sink->len++] = (char)(code & 0xFFu);
    return true;
}

/* One JSON string's characters, unescaped, into `sink`. */
SCE_C_UNUSED static inline const char *sce_payload_walk_text(const char *p, sce_payload_sink_t *sink) {
    if (*p != '"') {
        return SCE_PAYLOAD_REFUSE_NOT_TEXT;
    }
    p++;
    while (*p != '\0' && *p != '"') {
        unsigned long code;
        if (*p == '\\') {
            p++;
            switch (*p) {
            case '"':
                code = '"';
                break;
            case '\\':
                code = '\\';
                break;
            case '/':
                code = '/';
                break;
            case 'b':
                code = '\b';
                break;
            case 'f':
                code = '\f';
                break;
            case 'n':
                code = '\n';
                break;
            case 'r':
                code = '\r';
                break;
            case 't':
                code = '\t';
                break;
            case 'u': {
                char hex[5];
                int i;
                for (i = 0; i < 4; i++) {
                    if (p[1 + i] == '\0') {
                        return SCE_PAYLOAD_REFUSE_NOT_TEXT;
                    }
                    hex[i] = p[1 + i];
                }
                hex[4] = '\0';
                char *end = NULL;
                code = strtoul(hex, &end, 16);
                if (end != hex + 4) {
                    return SCE_PAYLOAD_REFUSE_NOT_TEXT;
                }
                p += 4;
                break;
            }
            default:
                return SCE_PAYLOAD_REFUSE_NOT_TEXT;
            }
            p++;
        } else {
            code = (unsigned char)*p;
            p++;
        }
        if (!sce_payload_emit(sink, code)) {
            return sink->above_one_byte ? SCE_PAYLOAD_REFUSE_NOT_ONE_BYTE : SCE_PAYLOAD_REFUSE_TOO_LONG;
        }
    }
    if (*p != '"') {
        return SCE_PAYLOAD_REFUSE_NOT_TEXT;
    }
    return NULL;
}

/* A text field, copied into a caller-owned buffer of `cap` bytes including the
   terminator. A text longer than the buffer is refused rather than truncated: a
   guard comparing a cut-off text answers for a value nobody sent. */
SCE_C_UNUSED static inline const char *sce_payload_read_text(const sce_payload_fields_t *fields, const char *name,
                                                             char *out, size_t cap) {
    const char *p = sce_payload_find(fields, name);
    if (p == NULL) {
        return SCE_PAYLOAD_REFUSE_MISSING;
    }
    if (cap == 0u) {
        return SCE_PAYLOAD_REFUSE_TOO_LONG;
    }
    sce_payload_sink_t sink;
    memset(&sink, 0, sizeof(sink));
    sink.text = out;
    sink.cap = cap;
    const char *refusal = sce_payload_walk_text(p, &sink);
    if (refusal != NULL) {
        return refusal;
    }
    out[sink.len] = '\0';
    return NULL;
}

/* A byte-string field, copied into a caller-owned buffer of `cap` bytes, with
   the length written to `out_len`.

   JSON has no byte string, so the wire carries the byte-exact Latin-1 text the
   inject seam writes, and this reads it back the same way: every one of the 256
   values is one character and back. Printable ASCII — what a bytes guard
   compares — is the same bytes under either reading. */
SCE_C_UNUSED static inline const char *sce_payload_read_bytes(const sce_payload_fields_t *fields, const char *name,
                                                              uint8_t *out, size_t cap, size_t *out_len) {
    const char *p = sce_payload_find(fields, name);
    if (p == NULL) {
        return SCE_PAYLOAD_REFUSE_MISSING;
    }
    sce_payload_sink_t sink;
    memset(&sink, 0, sizeof(sink));
    sink.bytes = out;
    sink.cap = cap;
    const char *refusal = sce_payload_walk_text(p, &sink);
    if (refusal != NULL) {
        return refusal;
    }
    *out_len = sink.len;
    return NULL;
}

/* The wire spelling of one byte string, for the inject seam's `data`: the
   writing half of sce_payload_read_bytes. Writes at most `cap` bytes including
   the terminator and answers false when the text does not fit. */
SCE_C_UNUSED static inline bool sce_payload_bytes_as_text(const uint8_t *bytes, size_t len, char *out, size_t cap) {
    if (cap == 0u || len + 1u > cap) {
        return false;
    }
    for (size_t i = 0; i < len; i++) {
        out[i] = (char)bytes[i];
    }
    out[len] = '\0';
    return true;
}

/* The JSON spelling of one text, for the inject seam's `data`. Writes at most
   `cap` bytes including the terminator and answers false when it does not fit;
   the quotes and escapes are included. */
SCE_C_UNUSED static inline bool sce_payload_quote(const char *text, char *out, size_t cap) {
    static const char HEX[] = "0123456789abcdef";
    size_t at = 0u;
    if (cap < 3u) {
        return false;
    }
    out[at++] = '"';
    for (const char *p = text; *p != '\0'; p++) {
        const unsigned char c = (unsigned char)*p;
        const char *escape = NULL;
        switch (c) {
        case '"':
            escape = "\\\"";
            break;
        case '\\':
            escape = "\\\\";
            break;
        case '\n':
            escape = "\\n";
            break;
        case '\r':
            escape = "\\r";
            break;
        case '\t':
            escape = "\\t";
            break;
        default:
            break;
        }
        if (escape != NULL) {
            if (at + 2u + 1u > cap) {
                return false;
            }
            out[at++] = escape[0];
            out[at++] = escape[1];
            continue;
        }
        if (c < 0x20u) {
            if (at + 6u + 1u > cap) {
                return false;
            }
            out[at++] = '\\';
            out[at++] = 'u';
            out[at++] = '0';
            out[at++] = '0';
            out[at++] = HEX[(c >> 4) & 0xFu];
            out[at++] = HEX[c & 0xFu];
            continue;
        }
        if (at + 1u + 1u > cap) {
            return false;
        }
        out[at++] = (char)c;
    }
    if (at + 1u + 1u > cap) {
        return false;
    }
    out[at++] = '"';
    out[at] = '\0';
    return true;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_EVENT_PAYLOAD_H */
