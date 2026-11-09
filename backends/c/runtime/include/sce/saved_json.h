/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * The writer and the reader a saved state's JSON text goes through on the C11
 * backend (docs/adr/0045, section 7). A saved state is one JSON document, the
 * same on every backend, and what one backend saved another reads. That asks
 * two things of the reader that a general JSON library does not give: a number
 * keeps the spelling it was written in, so a 64-bit value never passes through
 * a double, and an object that names a member twice is refused, since a reader
 * that takes the first and one that takes the last would read two values from
 * one text.
 *
 * It allocates nothing. The writer writes into a buffer the host hands it and
 * counts what it did not have room for, so a call with no buffer measures the
 * text and a call with one writes it; the reader walks the text in place, a
 * value being a span of it, and decodes a string into a buffer the caller owns.
 * Nesting is bounded by SCE_JSON_MAX_DEPTH, which a tree of sessions stays
 * inside to a depth of about fourteen sessions. Over the same writer, `sce_jcs_t`
 * writes a value in the JSON Canonicalization Scheme (RFC 8785), each object's
 * members sorted in the buffer it writes into: the text of the test log
 * (docs/SCE_TEST_PROFILE.md §12.5).
 *
 * Header-only, C11, and consumable from C++ (the gate in
 * `cxx_consumption_gate.cmake`). Cross-language siblings:
 * sce/include/saved/SavedJson.h (C++), backends/go/runtime/saved_json.go,
 * backends/python/runtime/sce_runtime/saved_json.py, com.sce.runtime.Json
 * (Kotlin) and sce_rust_runtime::json (Rust).
 */
#ifndef SCE_SAVED_JSON_H
#define SCE_SAVED_JSON_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#ifdef __cplusplus
extern "C" {
#endif

/* How deep a JSON value may nest. */
#define SCE_JSON_MAX_DEPTH 64u

/* The room a refusal's words have. A longer one is cut, not lost: the start of
 * it names what the refusal is about. */
#define SCE_SAVED_ERROR_LEN 256u

/* Why a text cannot be written, read or restored: one sentence, in the words
 * every backend uses. Every function below that can refuse answers false and
 * fills this in; a caller that passes NULL gets the answer without the words. */
typedef struct sce_saved_error_s {
    char message[SCE_SAVED_ERROR_LEN];
} sce_saved_error_t;

/* Refuse with `format`. Always false, so a caller can `return sce_saved_refuse(...)`. */
static inline bool sce_saved_refuse(sce_saved_error_t *err, const char *format, ...) {
    if (err != NULL) {
        va_list args;
        va_start(args, format);
        (void)vsnprintf(err->message, sizeof(err->message), format, args);
        va_end(args);
    }
    return false;
}

/* ── Write ────────────────────────────────────────────────────────────────── */

/* A JSON writer over a buffer the host owns. `len` counts every byte the text
 * needs, whether or not the buffer had room for it, so a writer over no buffer
 * measures the text. The text is never terminated mid-write; `sce_jw_finish`
 * terminates it when there is room. */
typedef struct sce_jw_s {
    char *buf;
    size_t cap;
    size_t len;
    uint64_t first; /* bit d is set while the container at depth d has written no member */
    uint8_t depth;
    bool too_deep;
} sce_jw_t;

static inline void sce_jw_init(sce_jw_t *w, char *buf, size_t cap) {
    w->buf = buf;
    w->cap = buf == NULL ? 0u : cap;
    w->len = 0u;
    w->first = 0u;
    w->depth = 0u;
    w->too_deep = false;
}

static inline void sce_jw_put_(sce_jw_t *w, const char *s, size_t n) {
    if (w->len < w->cap) {
        size_t room = w->cap - w->len;
        memcpy(w->buf + w->len, s, n < room ? n : room);
    }
    w->len += n;
}

static inline void sce_jw_raw(sce_jw_t *w, const char *s) {
    sce_jw_put_(w, s, strlen(s));
}

/* Terminate the text when there is room. True when the whole text fit, so a
 * false answer is a buffer to enlarge to `w->len + 1`, not a broken text. */
static inline bool sce_jw_finish(sce_jw_t *w) {
    if (w->len < w->cap) {
        w->buf[w->len] = '\0';
        return !w->too_deep;
    }
    if (w->cap > 0u) {
        w->buf[w->cap - 1u] = '\0';
    }
    return false;
}

/* A string's bytes as a JSON string writes them: the one set of escapes every
 * engine writes (ARCHITECTURE.md, "JSON Text (Single Source of Truth)"). A byte
 * is compared as an unsigned value, since `char` is signed on the common ABIs
 * and a signed comparison reads every UTF-8 byte above 0x7F as a control
 * character. */
static inline void sce_jw_escaped_(sce_jw_t *w, const char *s, size_t n) {
    static const char hex[] = "0123456789abcdef";
    for (size_t i = 0u; i < n; ++i) {
        const unsigned char c = (unsigned char)s[i];
        switch (c) {
        case '"':
            sce_jw_put_(w, "\\\"", 2u);
            break;
        case '\\':
            sce_jw_put_(w, "\\\\", 2u);
            break;
        case '\b':
            sce_jw_put_(w, "\\b", 2u);
            break;
        case '\f':
            sce_jw_put_(w, "\\f", 2u);
            break;
        case '\n':
            sce_jw_put_(w, "\\n", 2u);
            break;
        case '\r':
            sce_jw_put_(w, "\\r", 2u);
            break;
        case '\t':
            sce_jw_put_(w, "\\t", 2u);
            break;
        default:
            if (c < 0x20u) {
                char escape[6];
                escape[0] = '\\';
                escape[1] = 'u';
                escape[2] = '0';
                escape[3] = '0';
                escape[4] = hex[c >> 4];
                escape[5] = hex[c & 0x0Fu];
                sce_jw_put_(w, escape, 6u);
            } else {
                sce_jw_put_(w, s + i, 1u);
            }
        }
    }
}

static inline void sce_jw_string_n(sce_jw_t *w, const char *s, size_t n) {
    sce_jw_put_(w, "\"", 1u);
    sce_jw_escaped_(w, s, n);
    sce_jw_put_(w, "\"", 1u);
}

static inline void sce_jw_string(sce_jw_t *w, const char *s) {
    sce_jw_string_n(w, s, strlen(s));
}

static inline void sce_jw_null(sce_jw_t *w) {
    sce_jw_put_(w, "null", 4u);
}

static inline void sce_jw_bool(sce_jw_t *w, bool v) {
    if (v) {
        sce_jw_put_(w, "true", 4u);
    } else {
        sce_jw_put_(w, "false", 5u);
    }
}

/* A number in the spelling it carries. */
static inline void sce_jw_number_text(sce_jw_t *w, const char *s, size_t n) {
    sce_jw_put_(w, s, n);
}

static inline void sce_jw_uint(sce_jw_t *w, uint64_t v) {
    char text[24];
    int n = snprintf(text, sizeof(text), "%llu", (unsigned long long)v);
    sce_jw_put_(w, text, n > 0 ? (size_t)n : 0u);
}

static inline void sce_jw_int(sce_jw_t *w, int64_t v) {
    char text[24];
    int n = snprintf(text, sizeof(text), "%lld", (long long)v);
    sce_jw_put_(w, text, n > 0 ? (size_t)n : 0u);
}

/* The comma that stands before a member or an element that is not the first of
 * its container. */
static inline void sce_jw_separate_(sce_jw_t *w) {
    if (w->depth == 0u) {
        return;
    }
    const uint64_t bit = (uint64_t)1u << (w->depth - 1u);
    if ((w->first & bit) != 0u) {
        w->first &= ~bit;
    } else {
        sce_jw_put_(w, ",", 1u);
    }
}

static inline void sce_jw_open_(sce_jw_t *w, char open) {
    if (w->depth >= SCE_JSON_MAX_DEPTH) {
        w->too_deep = true;
        return;
    }
    sce_jw_put_(w, &open, 1u);
    w->first |= (uint64_t)1u << w->depth;
    ++w->depth;
}

static inline void sce_jw_close_(sce_jw_t *w, char close) {
    if (w->depth == 0u) {
        return;
    }
    --w->depth;
    sce_jw_put_(w, &close, 1u);
}

/* A container as a value: one that is an element or a member's value follows
 * `sce_jw_element` or `sce_jw_key`. */
static inline void sce_jw_begin_object(sce_jw_t *w) {
    sce_jw_open_(w, '{');
}

static inline void sce_jw_end_object(sce_jw_t *w) {
    sce_jw_close_(w, '}');
}

static inline void sce_jw_begin_array(sce_jw_t *w) {
    sce_jw_open_(w, '[');
}

static inline void sce_jw_end_array(sce_jw_t *w) {
    sce_jw_close_(w, ']');
}

/* The name of a member of the object being written; its value follows. */
static inline void sce_jw_key(sce_jw_t *w, const char *name) {
    sce_jw_separate_(w);
    sce_jw_string(w, name);
    sce_jw_put_(w, ":", 1u);
}

/* The start of an element of the array being written; its value follows. */
static inline void sce_jw_element(sce_jw_t *w) {
    sce_jw_separate_(w);
}

/* ── Read ─────────────────────────────────────────────────────────────────── */

/* A JSON value of a text, as the span `[p, end)` of it, with no whitespace
 * around it. A value is judged once, by `sce_json_parse`; every function below
 * that takes one relies on that and walks the span without checking it again. */
typedef struct sce_jv_s {
    const char *p;
    const char *end;
} sce_jv_t;

typedef enum sce_jkind_e {
    SCE_JSON_NULL = 0,
    SCE_JSON_BOOL,
    SCE_JSON_STRING,
    SCE_JSON_NUMBER,
    SCE_JSON_ARRAY,
    SCE_JSON_OBJECT
} sce_jkind_t;

static inline sce_jkind_t sce_jv_kind(sce_jv_t v) {
    switch (*v.p) {
    case '{':
        return SCE_JSON_OBJECT;
    case '[':
        return SCE_JSON_ARRAY;
    case '"':
        return SCE_JSON_STRING;
    case 't':
    case 'f':
        return SCE_JSON_BOOL;
    case 'n':
        return SCE_JSON_NULL;
    default:
        return SCE_JSON_NUMBER;
    }
}

static inline bool sce_jv_is_null(sce_jv_t v) {
    return sce_jv_kind(v) == SCE_JSON_NULL;
}

static inline bool sce_jv_as_bool(sce_jv_t v) {
    return *v.p == 't';
}

/* The text of a number, in the spelling it was written in. */
static inline size_t sce_jv_number_len(sce_jv_t v) {
    return (size_t)(v.end - v.p);
}

/* The end of the whitespace that starts at `p`. */
static inline const char *sce_json_skip_ws_(const char *p, const char *end) {
    while (p < end && (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r')) {
        ++p;
    }
    return p;
}

/* A string's decoded bytes, one at a time: the escapes of its span turned into
 * the UTF-8 they spell. A `\u` escape of a high surrogate that a low one does
 * not complete, and a lone low one, is no character and reads as U+FFFD, as the
 * other backends read it. */
typedef struct sce_jstr_s {
    const char *p;   /* inside the string, past its opening quote */
    const char *end; /* the end of the text */
    unsigned char pending[4];
    uint8_t n_pending;
    uint8_t at_pending;
} sce_jstr_t;

static inline int sce_json_hex4_(const char *p, const char *end, uint32_t *out) {
    if (end - p < 4) {
        return -1;
    }
    uint32_t value = 0u;
    for (int i = 0; i < 4; ++i) {
        const char d = p[i];
        uint32_t nibble;
        if (d >= '0' && d <= '9') {
            nibble = (uint32_t)(d - '0');
        } else if (d >= 'a' && d <= 'f') {
            nibble = (uint32_t)(d - 'a' + 10);
        } else if (d >= 'A' && d <= 'F') {
            nibble = (uint32_t)(d - 'A' + 10);
        } else {
            return -2;
        }
        value = value * 16u + nibble;
    }
    *out = value;
    return 0;
}

static inline uint8_t sce_json_utf8_(uint32_t point, unsigned char *out) {
    if (point < 0x80u) {
        out[0] = (unsigned char)point;
        return 1u;
    }
    if (point < 0x800u) {
        out[0] = (unsigned char)(0xC0u | (point >> 6));
        out[1] = (unsigned char)(0x80u | (point & 0x3Fu));
        return 2u;
    }
    if (point < 0x10000u) {
        out[0] = (unsigned char)(0xE0u | (point >> 12));
        out[1] = (unsigned char)(0x80u | ((point >> 6) & 0x3Fu));
        out[2] = (unsigned char)(0x80u | (point & 0x3Fu));
        return 3u;
    }
    out[0] = (unsigned char)(0xF0u | (point >> 18));
    out[1] = (unsigned char)(0x80u | ((point >> 12) & 0x3Fu));
    out[2] = (unsigned char)(0x80u | ((point >> 6) & 0x3Fu));
    out[3] = (unsigned char)(0x80u | (point & 0x3Fu));
    return 4u;
}

static inline void sce_jstr_init(sce_jstr_t *s, sce_jv_t v) {
    s->p = v.p + 1;
    s->end = v.end;
    s->n_pending = 0u;
    s->at_pending = 0u;
}

/* The next decoded byte of the string; false at its closing quote. The span was
 * judged, so an escape is well formed. */
static inline bool sce_jstr_next(sce_jstr_t *s, unsigned char *out) {
    if (s->at_pending < s->n_pending) {
        *out = s->pending[s->at_pending++];
        return true;
    }
    if (s->p >= s->end || *s->p == '"') {
        return false;
    }
    const char c = *s->p++;
    if (c != '\\') {
        *out = (unsigned char)c;
        return true;
    }
    const char escape = *s->p++;
    switch (escape) {
    case '"':
        *out = '"';
        return true;
    case '\\':
        *out = '\\';
        return true;
    case '/':
        *out = '/';
        return true;
    case 'b':
        *out = '\b';
        return true;
    case 'f':
        *out = '\f';
        return true;
    case 'n':
        *out = '\n';
        return true;
    case 'r':
        *out = '\r';
        return true;
    case 't':
        *out = '\t';
        return true;
    default:
        break;
    }
    /* `\u` */
    uint32_t unit = 0u;
    (void)sce_json_hex4_(s->p, s->end, &unit);
    s->p += 4;
    uint32_t point = unit;
    if (unit >= 0xD800u && unit <= 0xDFFFu) {
        uint32_t low = 0u;
        if (unit < 0xDC00u && s->end - s->p >= 6 && s->p[0] == '\\' && s->p[1] == 'u' &&
            sce_json_hex4_(s->p + 2, s->end, &low) == 0 && low >= 0xDC00u && low <= 0xDFFFu) {
            s->p += 6;
            point = 0x10000u + ((unit - 0xD800u) << 10) + (low - 0xDC00u);
        } else {
            point = 0xFFFDu;
        }
    }
    s->n_pending = sce_json_utf8_(point, s->pending);
    s->at_pending = 1u;
    *out = s->pending[0];
    return true;
}

/* A string value decoded into `out`, terminated. False when it does not fit in
 * `cap` bytes with its terminator; `*len` is the length decoded so far then. */
static inline bool sce_jv_string(sce_jv_t v, char *out, size_t cap, size_t *len) {
    sce_jstr_t s;
    unsigned char byte;
    size_t n = 0u;
    sce_jstr_init(&s, v);
    while (sce_jstr_next(&s, &byte)) {
        if (n + 1u >= cap) {
            if (len != NULL) {
                *len = n;
            }
            if (cap > 0u) {
                out[n < cap ? n : cap - 1u] = '\0';
            }
            return false;
        }
        out[n++] = (char)byte;
    }
    out[n] = '\0';
    if (len != NULL) {
        *len = n;
    }
    return true;
}

/* How many bytes a string value decodes to. */
static inline size_t sce_jv_string_len(sce_jv_t v) {
    sce_jstr_t s;
    unsigned char byte;
    size_t n = 0u;
    sce_jstr_init(&s, v);
    while (sce_jstr_next(&s, &byte)) {
        ++n;
    }
    return n;
}

/* Whether a string value decodes to exactly `literal`. */
static inline bool sce_jv_string_is(sce_jv_t v, const char *literal) {
    sce_jstr_t s;
    unsigned char byte;
    size_t n = 0u;
    sce_jstr_init(&s, v);
    while (sce_jstr_next(&s, &byte)) {
        if (literal[n] == '\0' || (unsigned char)literal[n] != byte) {
            return false;
        }
        ++n;
    }
    return literal[n] == '\0';
}

/* Whether two string values decode to the same bytes. */
static inline bool sce_jv_string_equal(sce_jv_t a, sce_jv_t b) {
    sce_jstr_t x;
    sce_jstr_t y;
    unsigned char bx;
    unsigned char by;
    sce_jstr_init(&x, a);
    sce_jstr_init(&y, b);
    for (;;) {
        const bool more_x = sce_jstr_next(&x, &bx);
        const bool more_y = sce_jstr_next(&y, &by);
        if (more_x != more_y) {
            return false;
        }
        if (!more_x) {
            return true;
        }
        if (bx != by) {
            return false;
        }
    }
}

/* The end of the value that starts at `p`, in a text that was judged. */
static inline const char *sce_jv_skip_(const char *p, const char *end) {
    if (*p == '"') {
        ++p;
        while (p < end && *p != '"') {
            p += (*p == '\\') ? 2 : 1;
        }
        return p + 1;
    }
    if (*p == '{' || *p == '[') {
        const char close = *p == '{' ? '}' : ']';
        ++p;
        p = sce_json_skip_ws_(p, end);
        if (*p == close) {
            return p + 1;
        }
        for (;;) {
            p = sce_json_skip_ws_(p, end);
            if (close == '}') {
                p = sce_jv_skip_(p, end); /* the member's name */
                p = sce_json_skip_ws_(p, end);
                ++p; /* ':' */
                p = sce_json_skip_ws_(p, end);
            }
            p = sce_jv_skip_(p, end);
            p = sce_json_skip_ws_(p, end);
            if (*p == ',') {
                ++p;
                continue;
            }
            return p + 1;
        }
    }
    while (p < end && *p != ',' && *p != '}' && *p != ']' && *p != ' ' && *p != '\t' && *p != '\n' && *p != '\r') {
        ++p;
    }
    return p;
}

/* Where the walk of an object's members or an array's elements stands. */
typedef struct sce_jiter_s {
    const char *p;
    const char *end;
    bool done;
} sce_jiter_t;

/* Begin a walk of an object's members or an array's elements. False when `v` is
 * neither. */
static inline bool sce_jv_iter(sce_jv_t v, sce_jiter_t *it) {
    const sce_jkind_t kind = sce_jv_kind(v);
    /* Left finished when `v` holds nothing to walk, so a caller that ignores the answer walks nothing. */
    it->p = v.p;
    it->end = v.end;
    it->done = true;
    if (kind != SCE_JSON_OBJECT && kind != SCE_JSON_ARRAY) {
        return false;
    }
    it->p = sce_json_skip_ws_(v.p + 1, v.end);
    it->end = v.end;
    it->done = *it->p == '}' || *it->p == ']';
    return true;
}

/* The next element of an array; false past the last. */
static inline bool sce_jiter_element(sce_jiter_t *it, sce_jv_t *out) {
    if (it->done) {
        return false;
    }
    it->p = sce_json_skip_ws_(it->p, it->end);
    out->p = it->p;
    out->end = sce_jv_skip_(it->p, it->end);
    it->p = sce_json_skip_ws_(out->end, it->end);
    if (*it->p == ',') {
        ++it->p;
    } else {
        it->done = true;
    }
    return true;
}

/* The next member of an object; false past the last. */
static inline bool sce_jiter_member(sce_jiter_t *it, sce_jv_t *key, sce_jv_t *value) {
    if (it->done) {
        return false;
    }
    it->p = sce_json_skip_ws_(it->p, it->end);
    key->p = it->p;
    key->end = sce_jv_skip_(it->p, it->end);
    it->p = sce_json_skip_ws_(key->end, it->end);
    ++it->p; /* ':' */
    it->p = sce_json_skip_ws_(it->p, it->end);
    value->p = it->p;
    value->end = sce_jv_skip_(it->p, it->end);
    it->p = sce_json_skip_ws_(value->end, it->end);
    if (*it->p == ',') {
        ++it->p;
    } else {
        it->done = true;
    }
    return true;
}

/* The member `name` of an object; false when `v` is no object or holds none. */
static inline bool sce_jv_get(sce_jv_t v, const char *name, sce_jv_t *out) {
    sce_jiter_t it;
    sce_jv_t key;
    if (sce_jv_kind(v) != SCE_JSON_OBJECT || !sce_jv_iter(v, &it)) {
        return false;
    }
    while (sce_jiter_member(&it, &key, out)) {
        if (sce_jv_string_is(key, name)) {
            return true;
        }
    }
    return false;
}

/* How many elements an array holds, or members an object. */
static inline size_t sce_jv_count(sce_jv_t v) {
    sce_jiter_t it;
    sce_jv_t a;
    sce_jv_t b;
    size_t n = 0u;
    if (!sce_jv_iter(v, &it)) {
        return 0u;
    }
    if (sce_jv_kind(v) == SCE_JSON_OBJECT) {
        while (sce_jiter_member(&it, &a, &b)) {
            ++n;
        }
    } else {
        while (sce_jiter_element(&it, &a)) {
            ++n;
        }
    }
    return n;
}

/* ── Judge ────────────────────────────────────────────────────────────────── */

static inline bool sce_json_malformed_(sce_saved_error_t *err, const char *what) {
    return sce_saved_refuse(err, "not JSON (%s)", what);
}

static inline bool sce_json_value_(const char **pp, const char *end, unsigned depth, sce_jv_t *out,
                                   sce_saved_error_t *err);

static inline bool sce_json_string_(const char **pp, const char *end, sce_saved_error_t *err) {
    const char *p = *pp + 1;
    for (;;) {
        if (p >= end) {
            return sce_json_malformed_(err, "a text ends unclosed");
        }
        const char c = *p++;
        if (c == '"') {
            *pp = p;
            return true;
        }
        if (c != '\\') {
            continue;
        }
        if (p >= end) {
            return sce_json_malformed_(err, "an escape ends the text");
        }
        const char escape = *p++;
        if (escape == 'u') {
            uint32_t unit = 0u;
            const int rc = sce_json_hex4_(p, end, &unit);
            if (rc == -1) {
                return sce_json_malformed_(err, "a \\u escape is short");
            }
            if (rc != 0) {
                return sce_json_malformed_(err, "a \\u escape is not hex");
            }
            p += 4;
            /* A high surrogate followed by a `\u` low one is one character; the
             * second escape is still an escape, and is judged as one. */
        } else if (escape != '"' && escape != '\\' && escape != '/' && escape != 'b' && escape != 'f' &&
                   escape != 'n' && escape != 'r' && escape != 't') {
            if (err != NULL) {
                (void)snprintf(err->message, sizeof(err->message), "not JSON (unknown escape '\\%c')", escape);
            }
            return false;
        }
    }
}

static inline bool sce_json_number_(const char **pp, const char *end, sce_saved_error_t *err) {
    const char *p = *pp;
    const char *start = p;
    if (*p == '-') {
        ++p;
    }
    while (p < end && *p >= '0' && *p <= '9') {
        ++p;
    }
    if (p < end && *p == '.') {
        ++p;
        while (p < end && *p >= '0' && *p <= '9') {
            ++p;
        }
    }
    if (p < end && (*p == 'e' || *p == 'E')) {
        ++p;
        if (p < end && (*p == '+' || *p == '-')) {
            ++p;
        }
        while (p < end && *p >= '0' && *p <= '9') {
            ++p;
        }
    }
    if (p == start || (p - start == 1 && *start == '-')) {
        return sce_json_malformed_(err, "a number has no digits");
    }
    *pp = p;
    return true;
}

static inline bool sce_json_keyword_(const char **pp, const char *end, const char *word, sce_saved_error_t *err) {
    const size_t n = strlen(word);
    if ((size_t)(end - *pp) < n || memcmp(*pp, word, n) != 0) {
        if (err != NULL) {
            (void)snprintf(err->message, sizeof(err->message), "not JSON (expected '%s')", word);
        }
        return false;
    }
    *pp += n;
    return true;
}

static inline bool sce_json_value_(const char **pp, const char *end, unsigned depth, sce_jv_t *out,
                                   sce_saved_error_t *err) {
    const char *p = sce_json_skip_ws_(*pp, end);
    if (p >= end) {
        return sce_json_malformed_(err, "it ends where a value was expected");
    }
    out->p = p;
    const char c = *p;
    if (c == '{' || c == '[') {
        if (depth >= SCE_JSON_MAX_DEPTH) {
            return sce_json_malformed_(err, "it nests too deep");
        }
        const char close = c == '{' ? '}' : ']';
        ++p;
        p = sce_json_skip_ws_(p, end);
        if (p < end && *p == close) {
            *pp = p + 1;
            out->end = *pp;
            return true;
        }
        const char *first_key = NULL;
        for (;;) {
            p = sce_json_skip_ws_(p, end);
            if (c == '{') {
                if (p >= end || *p != '"') {
                    return sce_json_malformed_(err, "a field name was expected");
                }
                sce_jv_t key;
                key.p = p;
                if (!sce_json_string_(&p, end, err)) {
                    return false;
                }
                key.end = p;
                /* An object whose names repeat has no single meaning: a reader that takes the
                 * first and one that takes the last would read two values from one text, so it
                 * is refused as the other backends refuse it. */
                if (first_key != NULL) {
                    const char *q = first_key;
                    while (q < key.p) {
                        sce_jv_t earlier;
                        earlier.p = q;
                        earlier.end = sce_jv_skip_(q, end);
                        if (sce_jv_string_equal(earlier, key)) {
                            if (err != NULL) {
                                sce_jstr_t s;
                                unsigned char byte;
                                char name[96];
                                size_t n = 0u;
                                sce_jstr_init(&s, key);
                                while (n + 1u < sizeof(name) && sce_jstr_next(&s, &byte)) {
                                    name[n++] = (char)byte;
                                }
                                name[n] = '\0';
                                (void)snprintf(err->message, sizeof(err->message),
                                               "not JSON (the field \"%s\" appears twice)", name);
                            }
                            return false;
                        }
                        /* Past this earlier name and its value, to the next name. */
                        q = sce_json_skip_ws_(earlier.end, end);
                        ++q; /* ':' */
                        q = sce_json_skip_ws_(q, end);
                        q = sce_jv_skip_(q, end);
                        q = sce_json_skip_ws_(q, end);
                        ++q; /* ',' */
                        q = sce_json_skip_ws_(q, end);
                    }
                } else {
                    first_key = key.p;
                }
                p = sce_json_skip_ws_(p, end);
                if (p >= end || *p != ':') {
                    return sce_json_malformed_(err, "a ':' was expected");
                }
                ++p;
            }
            sce_jv_t element;
            if (!sce_json_value_(&p, end, depth + 1u, &element, err)) {
                return false;
            }
            p = sce_json_skip_ws_(p, end);
            if (p >= end) {
                return sce_json_malformed_(err, "it ends unclosed");
            }
            if (*p == ',') {
                ++p;
            } else if (*p == close) {
                ++p;
                break;
            } else {
                return sce_json_malformed_(err, c == '{' ? "a ',' or '}' was expected" : "a ',' or ']' was expected");
            }
        }
        *pp = p;
        out->end = p;
        return true;
    }
    if (c == '"') {
        if (!sce_json_string_(&p, end, err)) {
            return false;
        }
    } else if (c == 't') {
        if (!sce_json_keyword_(&p, end, "true", err)) {
            return false;
        }
    } else if (c == 'f') {
        if (!sce_json_keyword_(&p, end, "false", err)) {
            return false;
        }
    } else if (c == 'n') {
        if (!sce_json_keyword_(&p, end, "null", err)) {
            return false;
        }
    } else if (c == '-' || (c >= '0' && c <= '9')) {
        if (!sce_json_number_(&p, end, err)) {
            return false;
        }
    } else {
        if (err != NULL) {
            (void)snprintf(err->message, sizeof(err->message), "not JSON (unexpected '%c')", c);
        }
        return false;
    }
    *pp = p;
    out->end = p;
    return true;
}

/* Judge `text` as exactly one JSON value, surrounding whitespace aside, and give
 * the value. The refusal is in the words every backend uses. This is the one
 * place a text is judged: the functions above rely on it. */
static inline bool sce_json_parse(const char *text, size_t len, sce_jv_t *out, sce_saved_error_t *err) {
    const char *p = text;
    const char *end = text + len;
    if (!sce_json_value_(&p, end, 0u, out, err)) {
        return false;
    }
    p = sce_json_skip_ws_(p, end);
    if (p < end) {
        return sce_saved_refuse(err, "more than one JSON value");
    }
    return true;
}

/* Write a value read from a text back out, compactly: a number in the spelling it
 * has, a string re-escaped by the writer's rules. A value read and written is the
 * text it was when it was compact already. */
static inline void sce_jw_value(sce_jw_t *w, sce_jv_t v) {
    switch (sce_jv_kind(v)) {
    case SCE_JSON_NULL:
        sce_jw_null(w);
        return;
    case SCE_JSON_BOOL:
        sce_jw_bool(w, sce_jv_as_bool(v));
        return;
    case SCE_JSON_NUMBER:
        sce_jw_number_text(w, v.p, sce_jv_number_len(v));
        return;
    case SCE_JSON_STRING: {
        sce_jstr_t s;
        unsigned char byte;
        char one;
        sce_jw_put_(w, "\"", 1u);
        sce_jstr_init(&s, v);
        while (sce_jstr_next(&s, &byte)) {
            one = (char)byte;
            sce_jw_escaped_(w, &one, 1u);
        }
        sce_jw_put_(w, "\"", 1u);
        return;
    }
    case SCE_JSON_ARRAY: {
        sce_jiter_t it;
        sce_jv_t element;
        sce_jw_begin_array(w);
        (void)sce_jv_iter(v, &it);
        while (sce_jiter_element(&it, &element)) {
            sce_jw_element(w);
            sce_jw_value(w, element);
        }
        sce_jw_end_array(w);
        return;
    }
    case SCE_JSON_OBJECT: {
        sce_jiter_t it;
        sce_jv_t key;
        sce_jv_t value;
        sce_jw_begin_object(w);
        (void)sce_jv_iter(v, &it);
        while (sce_jiter_member(&it, &key, &value)) {
            sce_jw_separate_(w);
            sce_jw_value(w, key);
            sce_jw_put_(w, ":", 1u);
            sce_jw_value(w, value);
        }
        sce_jw_end_object(w);
        return;
    }
    }
}

/* ── Write canonically (RFC 8785) ─────────────────────────────────────────── */

/* A writer of the JSON Canonicalization Scheme (RFC 8785) over the writer above,
 * into a buffer the host owns: no whitespace, each text escaped as ECMAScript's
 * `JSON.stringify` escapes it, which is the writer's one set of escapes
 * (`sce_jw_escaped_`, RFC 8785 section 3.2.2.2), and the members of each object
 * sorted by their names as arrays of UTF-16 code units (section 3.2.3), whatever
 * order they were written in. Two values equal under the scheme are then the
 * same bytes. The C11 sibling of `SCE::Saved::writeCanonical` (C++) and
 * `sce_rust_runtime::json::write_canonical` (Rust).
 *
 * A member is named by `sce_jcs_key` and written with the value that follows the
 * name, which is lent until then. A name no value follows before the next name
 * or the end of its object names no member: that is how a record leaves out a
 * field that is absent.
 *
 * It writes texts, truth values, null, arrays and objects, and no number: the
 * scheme writes a number as ECMAScript spells the double it reads as, and the
 * texts this writer is for hold every integer as the text of its digits
 * (docs/SCE_TEST_PROFILE.md §12.5).
 *
 * It allocates nothing. The members of an object are sorted in the buffer when
 * the object ends, which moves bytes and adds none, so over no buffer the writer
 * measures the text as `sce_jw_t` does; a text past its buffer is not sorted, and
 * does not fit. Refused, `refusal` saying why and `sce_jcs_finish` answering
 * false: an object that names one member twice, which has no single meaning
 * (section 3.1), found where the text is held; a value of an object written with
 * no name before it, and a name outside an object; and a value nested deeper than
 * SCE_JSON_MAX_DEPTH. */
typedef struct sce_jcs_s {
    sce_jw_t w;
    /* Where the text of each open container starts, by its depth. */
    size_t opened[SCE_JSON_MAX_DEPTH];
    /* Bit d is set while the container at depth d is an object. */
    uint64_t objects;
    /* The name of the member the next value is written as, or NULL. */
    const char *key;
    /* Why the text is not in the scheme, or NULL. */
    const char *refusal;
} sce_jcs_t;

static inline void sce_jcs_init(sce_jcs_t *j, char *buf, size_t cap) {
    sce_jw_init(&j->w, buf, cap);
    j->objects = 0u;
    j->key = NULL;
    j->refusal = NULL;
}

/* Refuse the text for `why`, a sentence of static storage; the first refusal is
 * the one kept. */
static inline void sce_jcs_refuse(sce_jcs_t *j, const char *why) {
    if (j->refusal == NULL) {
        j->refusal = why;
    }
}

/* Whether the container being written is an object. */
static inline bool sce_jcs_in_object_(const sce_jcs_t *j) {
    return j->w.depth > 0u && (j->objects & ((uint64_t)1u << (j->w.depth - 1u))) != 0u;
}

/* What stands before a value: the name of its member in an object, the comma
 * between it and the element before it in an array, nothing at the top. */
static inline void sce_jcs_place_(sce_jcs_t *j) {
    if (j->w.depth == 0u) {
        return;
    }
    if (!sce_jcs_in_object_(j)) {
        sce_jw_element(&j->w);
        return;
    }
    if (j->key == NULL) {
        sce_jcs_refuse(j, "a value of an object is written with no member name before it");
        sce_jw_separate_(&j->w);
        return;
    }
    sce_jw_key(&j->w, j->key);
    j->key = NULL;
}

/* Name the member the next value is written as. */
static inline void sce_jcs_key(sce_jcs_t *j, const char *name) {
    if (!sce_jcs_in_object_(j)) {
        sce_jcs_refuse(j, "a member name is written outside an object");
        return;
    }
    j->key = name;
}

static inline void sce_jcs_open_(sce_jcs_t *j, bool object) {
    sce_jcs_place_(j);
    const unsigned depth = j->w.depth;
    if (depth < SCE_JSON_MAX_DEPTH) {
        const uint64_t bit = (uint64_t)1u << depth;
        j->opened[depth] = j->w.len;
        j->objects = object ? (j->objects | bit) : (j->objects & ~bit);
    }
    if (object) {
        sce_jw_begin_object(&j->w);
    } else {
        sce_jw_begin_array(&j->w);
    }
}

static inline void sce_jcs_begin_object(sce_jcs_t *j) {
    sce_jcs_open_(j, true);
}

static inline void sce_jcs_begin_array(sce_jcs_t *j) {
    sce_jcs_open_(j, false);
}

static inline void sce_jcs_end_array(sce_jcs_t *j) {
    sce_jw_end_array(&j->w);
}

/* The UTF-16 code units of a member name, one at a time, read from its decoded
 * bytes. A character past U+FFFF is its two surrogates; a byte that begins no
 * UTF-8 sequence, or a sequence cut short, stands for U+FFFD, as it does where
 * the other backends read the name. */
typedef struct sce_jcs_units_s {
    sce_jstr_t s;
    /* The low surrogate of the character whose high one was read, or 0. */
    uint32_t low;
} sce_jcs_units_t;

static inline bool sce_jcs_next_unit_(sce_jcs_units_t *u, uint32_t *unit) {
    if (u->low != 0u) {
        *unit = u->low;
        u->low = 0u;
        return true;
    }
    unsigned char byte = 0u;
    if (!sce_jstr_next(&u->s, &byte)) {
        return false;
    }
    const uint32_t lead = byte;
    uint32_t point;
    unsigned more;
    if (lead < 0x80u) {
        *unit = lead;
        return true;
    } else if ((lead & 0xE0u) == 0xC0u) {
        point = lead & 0x1Fu;
        more = 1u;
    } else if ((lead & 0xF0u) == 0xE0u) {
        point = lead & 0x0Fu;
        more = 2u;
    } else if ((lead & 0xF8u) == 0xF0u) {
        point = lead & 0x07u;
        more = 3u;
    } else {
        *unit = 0xFFFDu;
        return true;
    }
    for (unsigned i = 0u; i < more; ++i) {
        const sce_jstr_t before = u->s;
        unsigned char next = 0u;
        const bool continues = sce_jstr_next(&u->s, &next) && ((uint32_t)next & 0xC0u) == 0x80u;
        if (!continues) {
            u->s = before;
            *unit = 0xFFFDu;
            return true;
        }
        point = (point << 6) | ((uint32_t)next & 0x3Fu);
    }
    if (point < 0x10000u) {
        *unit = point;
        return true;
    }
    point -= 0x10000u;
    *unit = 0xD800u + (point >> 10);
    u->low = 0xDC00u + (point & 0x3FFu);
    return true;
}

/* The order of two member names, each the span of a JSON string, as arrays of
 * UTF-16 code units: below zero when `a` comes first, zero when they are one
 * name. For valid UTF-8 it is the order of the bytes but for a character past
 * U+FFFF against one of U+E000 to U+FFFF, whose first code unit, a surrogate,
 * comes before it. */
static inline int sce_jcs_compare_names_(sce_jv_t a, sce_jv_t b) {
    sce_jcs_units_t x;
    sce_jcs_units_t y;
    sce_jstr_init(&x.s, a);
    sce_jstr_init(&y.s, b);
    x.low = 0u;
    y.low = 0u;
    for (;;) {
        uint32_t ux = 0u;
        uint32_t uy = 0u;
        const bool more_x = sce_jcs_next_unit_(&x, &ux);
        const bool more_y = sce_jcs_next_unit_(&y, &uy);
        if (!more_x || !more_y) {
            return more_x ? 1 : (more_y ? -1 : 0);
        }
        if (ux != uy) {
            return ux < uy ? -1 : 1;
        }
    }
}

/* The `n` bytes at `p` in the opposite order. */
static inline void sce_jcs_reverse_(char *p, size_t n) {
    for (size_t i = 0u; i < n / 2u; ++i) {
        const char held = p[i];
        p[i] = p[n - 1u - i];
        p[n - 1u - i] = held;
    }
}

/* Sort the members of the object whose text starts at `opened` and ends the text
 * written so far, which the buffer holds whole and which this writer wrote, so it
 * has no whitespace. A selection: each pass finds the least name among the
 * members not yet in place and turns it to the front of them, the members it
 * passes keeping their order (three reversals of the bytes, so no room is
 * needed). The same name twice refuses the text. */
static inline void sce_jcs_sort_members_(sce_jcs_t *j, size_t opened) {
    char *const text = j->w.buf;
    const char *const end = text + j->w.len;
    size_t first = opened + 1u; /* where the first member not yet in place starts */
    for (;;) {
        sce_jiter_t it;
        it.p = text + first;
        it.end = end;
        it.done = text[first] == '}';
        sce_jv_t key;
        sce_jv_t value;
        sce_jv_t least;
        least.p = NULL;
        least.end = NULL;
        size_t least_at = first;
        size_t least_end = first;
        size_t count = 0u;
        while (sce_jiter_member(&it, &key, &value)) {
            const int order = count == 0u ? -1 : sce_jcs_compare_names_(key, least);
            if (order == 0) {
                sce_jcs_refuse(j, "an object names one member twice");
                return;
            }
            if (order < 0) {
                least = key;
                least_at = (size_t)(key.p - text);
                least_end = (size_t)(value.end - text);
            }
            ++count;
        }
        if (count < 2u) {
            return;
        }
        if (least_at == first) {
            first = least_end + 1u;
            continue;
        }
        /* [first, least_end) is the members before the least, a comma, and the
           least; turned about, it is the least, a comma, and the others. */
        const size_t span = least_end - first;
        const size_t least_len = least_end - least_at;
        sce_jcs_reverse_(text + first, span);
        sce_jcs_reverse_(text + first, least_len);
        sce_jcs_reverse_(text + first + least_len + 1u, span - least_len - 1u);
        first += least_len + 1u;
    }
}

/* The end of an object: a name no value followed names no member, and the
 * members are sorted where the buffer holds the object's text whole. */
static inline void sce_jcs_end_object(sce_jcs_t *j) {
    j->key = NULL;
    if (j->w.depth == 0u) {
        return;
    }
    const size_t opened = j->opened[j->w.depth - 1u];
    sce_jw_end_object(&j->w);
    if (j->refusal == NULL && !j->w.too_deep && j->w.buf != NULL && j->w.len <= j->w.cap) {
        sce_jcs_sort_members_(j, opened);
    }
}

static inline void sce_jcs_string_n(sce_jcs_t *j, const char *s, size_t n) {
    sce_jcs_place_(j);
    sce_jw_string_n(&j->w, s, n);
}

static inline void sce_jcs_string(sce_jcs_t *j, const char *s) {
    sce_jcs_string_n(j, s, strlen(s));
}

static inline void sce_jcs_bool(sce_jcs_t *j, bool v) {
    sce_jcs_place_(j);
    sce_jw_bool(&j->w, v);
}

static inline void sce_jcs_null(sce_jcs_t *j) {
    sce_jcs_place_(j);
    sce_jw_null(&j->w);
}

/* A value whose whole text this writer wrote already, the `n` bytes at `text`,
 * as they are: it is in the scheme, so nothing in it moves. */
static inline void sce_jcs_written(sce_jcs_t *j, const char *text, size_t n) {
    sce_jcs_place_(j);
    sce_jw_put_(&j->w, text, n);
}

/* Terminate the text where there is room. True when the whole text fit and is in
 * the scheme. Where it is false, `refusal` says why when the text is not in the
 * scheme; otherwise the buffer is to be enlarged to `w.len + 1` bytes. */
static inline bool sce_jcs_finish(sce_jcs_t *j) {
    if (j->w.too_deep) {
        sce_jcs_refuse(j, "a value nests deeper than SCE_JSON_MAX_DEPTH");
    }
    const bool whole = sce_jw_finish(&j->w);
    return whole && j->refusal == NULL;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_SAVED_JSON_H */
