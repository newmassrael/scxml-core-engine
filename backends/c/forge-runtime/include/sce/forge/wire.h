/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * The wire form of a `sce-static` machine's values (docs/SCE_ACCEPTED_SUBSET.md
 * §2.15) — the JSON object a `<donedata>` or a `<send>` carries as the data of
 * its event, built with no script engine from the machine's own fields.
 *
 * A value is a `sce_forge_wire_value_t`: the typed value of the data model, which
 * is what the pair's JSON value is written from. A pair is written whole or not at
 * all, so a value that failed is left out and the others still cross (§scxml-5.7.1).
 * The object is written into a buffer the caller owns; one that does not fit it
 * is reported by `sce_forge_wire_end`, and the buffer then holds `{}`, never a
 * truncated object.
 */
#ifndef SCE_FORGE_WIRE_H
#define SCE_FORGE_WIRE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* How a real is written: the one spelling every engine writes, in the runtime's
 * own include directory because the typed payload of a machine with no
 * `sce-static` datamodel writes it too. */
#include <sce/number_text.h>

typedef enum {
    SCE_FORGE_WIRE_BOOL,
    SCE_FORGE_WIRE_INT,
    SCE_FORGE_WIRE_UINT,
    SCE_FORGE_WIRE_REAL,
    SCE_FORGE_WIRE_STRING
} sce_forge_wire_kind_t;

/* One value of the data model, as it crosses. */
typedef struct {
    sce_forge_wire_kind_t kind;

    union {
        bool b;
        int64_t i;
        uint64_t u;
        double r;
        const char *s;
    } as;
} sce_forge_wire_value_t;

static inline sce_forge_wire_value_t sce_forge_wire_bool(bool v) {
    sce_forge_wire_value_t value;
    memset(&value, 0, sizeof(value));
    value.kind = SCE_FORGE_WIRE_BOOL;
    value.as.b = v;
    return value;
}

static inline sce_forge_wire_value_t sce_forge_wire_int(int64_t v) {
    sce_forge_wire_value_t value;
    memset(&value, 0, sizeof(value));
    value.kind = SCE_FORGE_WIRE_INT;
    value.as.i = v;
    return value;
}

static inline sce_forge_wire_value_t sce_forge_wire_uint(uint64_t v) {
    sce_forge_wire_value_t value;
    memset(&value, 0, sizeof(value));
    value.kind = SCE_FORGE_WIRE_UINT;
    value.as.u = v;
    return value;
}

/* A 64-bit real. Only the 64-bit form is pinned by the contract below, so a
 * 32-bit real is handed to this as the double it widens to, which is exact: the
 * generator writes the cast, and the single nearest 0.1 crosses as
 * 0.10000000149011612, the number it is. */
static inline sce_forge_wire_value_t sce_forge_wire_real(double v) {
    sce_forge_wire_value_t value;
    memset(&value, 0, sizeof(value));
    value.kind = SCE_FORGE_WIRE_REAL;
    value.as.r = v;
    return value;
}

static inline sce_forge_wire_value_t sce_forge_wire_string(const char *v) {
    sce_forge_wire_value_t value;
    memset(&value, 0, sizeof(value));
    value.kind = SCE_FORGE_WIRE_STRING;
    value.as.s = v;
    return value;
}

/* The object being written: `buf` holds `len` bytes of it, `cap` is its size.
 *
 * The pairs of a name that repeats are written one after another (the generator
 * lists them sorted by name, a name's values in document order), so what is
 * remembered of the last pair is all it takes to collect them into one array:
 * its key, where its value begins, and how many values the key holds so far. */
typedef struct {
    char *buf;
    size_t cap;
    size_t len;
    bool any;
    bool overflow;
    const char *last_key;
    size_t last_value_at;
    size_t last_count;
} sce_forge_wire_t;

static inline void sce_forge_wire_put(sce_forge_wire_t *w, const char *text, size_t n) {
    if (w->overflow || w->len + n >= w->cap) {
        w->overflow = true;
        return;
    }
    memcpy(w->buf + w->len, text, n);
    w->len += n;
}

/* Start an object in `buf`, of `cap` bytes. */
static inline void sce_forge_wire_begin(sce_forge_wire_t *w, char *buf, size_t cap) {
    w->buf = buf;
    w->cap = cap;
    w->len = 0;
    w->any = false;
    w->overflow = false;
    w->last_key = NULL;
    w->last_value_at = 0;
    w->last_count = 0;
    sce_forge_wire_put(w, "{", 1u);
}

/* The text of a string as a JSON string's body: `"`, `\` and the control
 * characters escaped, every other byte — UTF-8 included — as it is. */
static inline void sce_forge_wire_put_text(sce_forge_wire_t *w, const char *text) {
    for (const char *p = text; *p != '\0'; ++p) {
        const unsigned char c = (unsigned char)*p;
        char escaped[7];
        switch (c) {
        case '"':
            sce_forge_wire_put(w, "\\\"", 2u);
            break;
        case '\\':
            sce_forge_wire_put(w, "\\\\", 2u);
            break;
        case '\n':
            sce_forge_wire_put(w, "\\n", 2u);
            break;
        case '\r':
            sce_forge_wire_put(w, "\\r", 2u);
            break;
        case '\t':
            sce_forge_wire_put(w, "\\t", 2u);
            break;
        default:
            if (c < 0x20u) {
                (void)snprintf(escaped, sizeof(escaped), "\\u%04x", (unsigned)c);
                sce_forge_wire_put(w, escaped, 6u);
            } else {
                sce_forge_wire_put(w, (const char *)&c, 1u);
            }
            break;
        }
    }
}

/* A value as JSON: what a pair's value, and the whole of an event's data when a
 * `<send>`'s `<content expr>` names one value, are written as. */
static inline void sce_forge_wire_put_value(sce_forge_wire_t *w, sce_forge_wire_value_t value) {
    char number[64];
    switch (value.kind) {
    case SCE_FORGE_WIRE_BOOL:
        sce_forge_wire_put(w, value.as.b ? "true" : "false", value.as.b ? 4u : 5u);
        break;
    case SCE_FORGE_WIRE_INT:
        (void)snprintf(number, sizeof(number), "%lld", (long long)value.as.i);
        sce_forge_wire_put(w, number, strlen(number));
        break;
    case SCE_FORGE_WIRE_UINT:
        (void)snprintf(number, sizeof(number), "%llu", (unsigned long long)value.as.u);
        sce_forge_wire_put(w, number, strlen(number));
        break;
    case SCE_FORGE_WIRE_REAL:
        /* JSON has no spelling for a value that is not finite: it crosses as
         * `null`, and as `NaN` / `Infinity` / `-Infinity` where a request carries
         * text (`sce_forge_wire_text`). */
        if (value.as.r - value.as.r != 0.0) {
            sce_forge_wire_put(w, "null", 4u);
        } else if (sce_number_text(value.as.r, number, sizeof(number)) != 0u) {
            sce_forge_wire_put(w, number, strlen(number));
        } else {
            w->overflow = true;
        }
        break;
    case SCE_FORGE_WIRE_STRING:
        sce_forge_wire_put(w, "\"", 1u);
        sce_forge_wire_put_text(w, value.as.s);
        sce_forge_wire_put(w, "\"", 1u);
        break;
    }
}

/* End the array the last key's values were collected into, when it holds more
 * than one: a name with a single value is written as that value alone. */
static inline void sce_forge_wire_close_array(sce_forge_wire_t *w) {
    if (w->last_count > 1u) {
        sce_forge_wire_put(w, "]", 1u);
    }
    w->last_count = 0;
}

/* One pair: `key` is the JSON text of the name, quotes included, which the
 * generator has already escaped, and a string literal, so that the pointer
 * outlives the object.
 *
 * A name that repeats collects its values, in the order they are written, into
 * one array (ARCHITECTURE.md, "JSON Object Key Order"; W3C SCXML test178): the
 * second value of a key turns the first into the array's first element, which
 * moves its text up one byte for the `[`. A key that has one value only — the
 * others failed to compute and were left out — is that value, not an array of
 * one. */
static inline void sce_forge_wire_pair(sce_forge_wire_t *w, const char *key, sce_forge_wire_value_t value) {
    if (w->overflow) {
        return;
    }
    if (w->any && w->last_key != NULL && strcmp(w->last_key, key) == 0) {
        if (w->last_count == 1u) {
            if (w->len + 1u >= w->cap) {
                w->overflow = true;
                return;
            }
            memmove(w->buf + w->last_value_at + 1u, w->buf + w->last_value_at, w->len - w->last_value_at);
            w->buf[w->last_value_at] = '[';
            w->len += 1u;
        }
        sce_forge_wire_put(w, ",", 1u);
        sce_forge_wire_put_value(w, value);
        w->last_count += 1u;
        return;
    }
    if (w->any) {
        sce_forge_wire_close_array(w);
        sce_forge_wire_put(w, ",", 1u);
    }
    w->any = true;
    sce_forge_wire_put(w, key, strlen(key));
    sce_forge_wire_put(w, ":", 1u);
    w->last_key = key;
    w->last_value_at = w->len;
    w->last_count = 1u;
    sce_forge_wire_put_value(w, value);
}

/* One value alone, as the JSON text of an event's data, written into `buf` of
 * `cap` bytes. False when it did not fit, and `buf` then holds the empty string,
 * never a truncated value (truncated JSON does not parse, so it would arrive as
 * some other value). */
static inline bool sce_forge_wire_json(sce_forge_wire_value_t value, char *buf, size_t cap) {
    sce_forge_wire_t w;
    if (cap == 0u) {
        return false;
    }
    w.buf = buf;
    w.cap = cap;
    w.len = 0;
    w.any = false;
    w.overflow = false;
    w.last_key = NULL;
    w.last_value_at = 0;
    w.last_count = 0;
    sce_forge_wire_put_value(&w, value);
    if (w.overflow) {
        buf[0] = '\0';
        return false;
    }
    buf[w.len] = '\0';
    return true;
}

/* The text a value crosses as where a request carries it as text — a host's
 * `params` — written into `buf` of `cap` bytes: `true` / `false` for a bool, the
 * decimal digits of an integer, a real as ECMAScript spells it (`NaN` and the
 * infinities included), and a string as itself. False when it did not
 * fit, and `buf` then holds the empty string, never a truncated value. */
static inline bool sce_forge_wire_text(sce_forge_wire_value_t value, char *buf, size_t cap) {
    int written = 0;
    if (cap == 0u) {
        return false;
    }
    switch (value.kind) {
    case SCE_FORGE_WIRE_BOOL:
        written = snprintf(buf, cap, "%s", value.as.b ? "true" : "false");
        break;
    case SCE_FORGE_WIRE_INT:
        written = snprintf(buf, cap, "%lld", (long long)value.as.i);
        break;
    case SCE_FORGE_WIRE_UINT:
        written = snprintf(buf, cap, "%llu", (unsigned long long)value.as.u);
        break;
    case SCE_FORGE_WIRE_REAL:
        return sce_number_text(value.as.r, buf, cap) != 0u;
    case SCE_FORGE_WIRE_STRING:
        written = snprintf(buf, cap, "%s", value.as.s);
        break;
    }
    if (written < 0 || (size_t)written >= cap) {
        buf[0] = '\0';
        return false;
    }
    return true;
}

/* `n` values joined into one string, written into `buf` of `cap` bytes — what
 * `a + b` is when either operand is a string (ECMA-262 §13.15.3): each part is
 * written as `sce_forge_wire_text` writes it, a string as itself and an integer
 * as its decimal digits. The generator sizes `buf` as the sum of the bounds the
 * data model declares for the parts, plus the terminator, so every part fits;
 * a part that did not would leave the empty string, never a truncated join.
 * Returns `buf`, so the call is an expression a statement can read. */
static inline char *sce_forge_concat_into(char *buf, size_t cap, const sce_forge_wire_value_t *parts, size_t n) {
    size_t len = 0u;
    if (cap == 0u) {
        return buf;
    }
    buf[0] = '\0';
    for (size_t i = 0u; i < n; ++i) {
        if (!sce_forge_wire_text(parts[i], buf + len, cap - len)) {
            buf[0] = '\0';
            return buf;
        }
        len += strlen(buf + len);
    }
    return buf;
}

/* The same, with the parts written where it is called: the buffer is the
 * caller's (a compound literal the generator sizes), and the parts are
 * `sce_forge_wire_*` values. */
#define SCE_FORGE_CONCAT(buf, cap, ...)                                                                                \
    sce_forge_concat_into((buf), (cap), (const sce_forge_wire_value_t[]){__VA_ARGS__},                                 \
                          sizeof((const sce_forge_wire_value_t[]){__VA_ARGS__}) / sizeof(sce_forge_wire_value_t))

/* Finish the object. False when it did not fit the buffer, which then holds `{}`. */
static inline bool sce_forge_wire_end(sce_forge_wire_t *w) {
    sce_forge_wire_close_array(w);
    if (!w->overflow) {
        sce_forge_wire_put(w, "}", 1u);
    }
    if (w->overflow) {
        if (w->cap >= 3u) {
            memcpy(w->buf, "{}", 3u);
        } else if (w->cap > 0u) {
            w->buf[0] = '\0';
        }
        return false;
    }
    w->buf[w->len] = '\0';
    return true;
}

#endif /* SCE_FORGE_WIRE_H */
