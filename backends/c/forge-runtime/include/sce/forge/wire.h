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
 * 32-bit real is not handed to this: its widening is not a spelling every engine
 * shares. */
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

/* The object being written: `buf` holds `len` bytes of it, `cap` is its size. */
typedef struct {
    char *buf;
    size_t cap;
    size_t len;
    bool any;
    bool overflow;
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

/* Whether the decimal `digits` (`k` of them) times ten to `exponent` — the
 * exponent of the first digit — reads back as `magnitude`. The text handed to
 * `strtod` has no radix character (the digits as one integer, scaled by the
 * exponent of the last), so a locale that spells the radix with a comma cannot
 * change what it reads. */
static inline bool sce_forge_wire_reads_back(const char *digits, int k, int exponent, double magnitude) {
    char text[64];
    (void)snprintf(text, sizeof(text), "%.*se%d", k, digits, exponent - (k - 1));
    return strtod(text, NULL) == magnitude;
}

/* A 64-bit real as ECMAScript's `Number::toString` spells it, radix 10
 * (ARCHITECTURE.md, "JSON Number Text"): the fewest digits that read back as the
 * same double, in decimal notation when `1e-6 <= |x| < 1e21` and as `d[.ddd]e[+-]n`
 * otherwise, no fraction on a whole value and `0` for either zero. A value that
 * is not finite is `NaN`, `Infinity` or `-Infinity`, as `String(x)` has it; a JSON
 * writer, which has none of those, tests for them first.
 *
 * `printf` has no shortest form, so the digits are the first precision at which
 * the rounded decimal reads back, and when that rounding does not, the decimal
 * one unit above it: below a power of two the neighbouring double is half as far,
 * so the nearest decimal can fall outside the interval that reads as the value
 * (2^-44 is `5.684341886080802e-14`, not `...801e-14`).
 *
 * Written into `buf` of `cap` bytes. The length, or 0 when it did not fit, and
 * `buf` then holds the empty string, never a truncated number. */
static inline size_t sce_forge_wire_number_text(double value, char *buf, size_t cap) {
    char out[64];
    size_t n = 0u;
    if (cap == 0u) {
        return 0u;
    }
    if (value != value) {
        n = (size_t)snprintf(out, sizeof(out), "NaN");
    } else if (value - value != 0.0) {
        n = (size_t)snprintf(out, sizeof(out), "%s", value > 0.0 ? "Infinity" : "-Infinity");
    } else if (value == 0.0) {
        n = (size_t)snprintf(out, sizeof(out), "0");
    } else {
        const bool negative = value < 0.0;
        const double magnitude = negative ? -value : value;
        char digits[24] = {0};
        int k = 0;
        int exponent = 0;
        for (int precision = 0; precision <= 16; ++precision) {
            char scientific[48];
            (void)snprintf(scientific, sizeof(scientific), "%.*e", precision, magnitude);
            const char *p = scientific;
            k = 0;
            for (; *p != '\0' && *p != 'e' && *p != 'E'; ++p) {
                if (*p >= '0' && *p <= '9') {
                    digits[k++] = *p;
                }
            }
            exponent = *p != '\0' ? (int)strtol(p + 1, NULL, 10) : 0;
            if (sce_forge_wire_reads_back(digits, k, exponent, magnitude)) {
                break;
            }
            /* The decimal one unit above, a carry out of the first digit
             * leaving a one and zeros. */
            char above[24];
            int above_exponent = exponent;
            int i = k - 1;
            memcpy(above, digits, (size_t)k);
            while (i >= 0 && above[i] == '9') {
                above[i--] = '0';
            }
            if (i >= 0) {
                above[i] = (char)(above[i] + 1);
            } else {
                above[0] = '1';
                above_exponent++;
            }
            if (sce_forge_wire_reads_back(above, k, above_exponent, magnitude)) {
                memcpy(digits, above, (size_t)k);
                exponent = above_exponent;
                break;
            }
        }
        /* The value is 0.<digits> * 10^point: the specification's `k` and `n`. */
        const int point = exponent + 1;
        if (negative) {
            out[n++] = '-';
        }
        if (k <= point && point <= 21) {
            memcpy(out + n, digits, (size_t)k);
            n += (size_t)k;
            for (int i = k; i < point; ++i) {
                out[n++] = '0';
            }
        } else if (0 < point && point <= 21) {
            memcpy(out + n, digits, (size_t)point);
            n += (size_t)point;
            out[n++] = '.';
            memcpy(out + n, digits + point, (size_t)(k - point));
            n += (size_t)(k - point);
        } else if (-6 < point && point <= 0) {
            out[n++] = '0';
            out[n++] = '.';
            for (int i = 0; i < -point; ++i) {
                out[n++] = '0';
            }
            memcpy(out + n, digits, (size_t)k);
            n += (size_t)k;
        } else {
            const int power = point - 1;
            out[n++] = digits[0];
            if (k > 1) {
                out[n++] = '.';
                memcpy(out + n, digits + 1, (size_t)(k - 1));
                n += (size_t)(k - 1);
            }
            out[n++] = 'e';
            out[n++] = power < 0 ? '-' : '+';
            n += (size_t)snprintf(out + n, sizeof(out) - n, "%d", power < 0 ? -power : power);
        }
    }
    if (n >= cap) {
        buf[0] = '\0';
        return 0u;
    }
    memcpy(buf, out, n);
    buf[n] = '\0';
    return n;
}

/* One pair: `key` is the JSON text of the name, quotes included, which the
 * generator has already escaped. */
static inline void sce_forge_wire_pair(sce_forge_wire_t *w, const char *key, sce_forge_wire_value_t value) {
    char number[64];
    if (w->any) {
        sce_forge_wire_put(w, ",", 1u);
    }
    w->any = true;
    sce_forge_wire_put(w, key, strlen(key));
    sce_forge_wire_put(w, ":", 1u);
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
        } else if (sce_forge_wire_number_text(value.as.r, number, sizeof(number)) != 0u) {
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
        return sce_forge_wire_number_text(value.as.r, buf, cap) != 0u;
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

/* Finish the object. False when it did not fit the buffer, which then holds `{}`. */
static inline bool sce_forge_wire_end(sce_forge_wire_t *w) {
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
