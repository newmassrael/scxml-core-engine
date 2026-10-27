/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * How each `sce:type` is written into a saved state and read back on the C11
 * backend: only if it is one, since a value of another kind, or one its width
 * cannot hold, is refused rather than converted. The functions a generated
 * machine's `_save` and `_restore` call, one per type, named as the type is.
 *
 * A value is written as the JSON value model holds it (a number's text, a
 * string, a bool, an array, an object, null) and read back from a span of a
 * judged text (`sce/saved_json.h`), a refusal naming what holds a value that is
 * not the type's. A 64-bit integer is a text, since a reader that holds JSON
 * numbers as doubles would lose its low bits and a saved state outlives the
 * backend that wrote it.
 *
 * Header-only, allocation-free. Cross-language siblings:
 * sce/include/saved/SavedValues.h (C++), backends/go/runtime/saved_values.go,
 * backends/python/runtime/sce_runtime/saved_values.py, com.sce.runtime.SavedValues
 * (Kotlin) and the `SavedValue` trait of sce_rust_runtime::saved_state (Rust).
 */
#ifndef SCE_SAVED_VALUES_H
#define SCE_SAVED_VALUES_H

#include "sce/number_text.h"
#include "sce/saved_json.h"

#include <limits.h>
#include <math.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Write: a value's saved form ──────────────────────────────────────────── */

static inline void sce_saved_write_uint8(sce_jw_t *w, uint8_t v) {
    sce_jw_uint(w, v);
}

static inline void sce_saved_write_uint16(sce_jw_t *w, uint16_t v) {
    sce_jw_uint(w, v);
}

static inline void sce_saved_write_uint32(sce_jw_t *w, uint32_t v) {
    sce_jw_uint(w, v);
}

static inline void sce_saved_write_int8(sce_jw_t *w, int8_t v) {
    sce_jw_int(w, v);
}

static inline void sce_saved_write_int16(sce_jw_t *w, int16_t v) {
    sce_jw_int(w, v);
}

static inline void sce_saved_write_int32(sce_jw_t *w, int32_t v) {
    sce_jw_int(w, v);
}

/* A 64-bit integer is written as a text. */
static inline void sce_saved_write_uint64(sce_jw_t *w, uint64_t v) {
    char text[24];
    const int n = snprintf(text, sizeof(text), "%llu", (unsigned long long)v);
    sce_jw_string_n(w, text, n > 0 ? (size_t)n : 0u);
}

static inline void sce_saved_write_int64(sce_jw_t *w, int64_t v) {
    char text[24];
    const int n = snprintf(text, sizeof(text), "%lld", (long long)v);
    sce_jw_string_n(w, text, n > 0 ? (size_t)n : 0u);
}

/* A real's saved form: the one number text every engine writes (ARCHITECTURE.md,
 * "JSON Number Text"), and, for a value that is not finite, which JSON has no
 * spelling for, the text every backend reads it back from. */
static inline void sce_saved_write_float64(sce_jw_t *w, double v) {
    char text[64];
    if (v != v) {
        sce_jw_string(w, "NaN");
    } else if (v - v != 0.0) {
        sce_jw_string(w, v > 0.0 ? "Infinity" : "-Infinity");
    } else {
        const size_t n = sce_number_text(v, text, sizeof(text));
        sce_jw_number_text(w, text, n);
    }
}

/* A 32-bit real's saved form: that of the 64-bit real it widens to, which is exact. */
static inline void sce_saved_write_float32(sce_jw_t *w, float v) {
    sce_saved_write_float64(w, (double)v);
}

static inline void sce_saved_write_bool(sce_jw_t *w, bool v) {
    sce_jw_bool(w, v);
}

static inline void sce_saved_write_string(sce_jw_t *w, const char *v) {
    sce_jw_string(w, v);
}

/* A byte string's byte-exact Latin-1 text, each byte the character of that code
 * point (docs/adr/0005, decision 2): the spelling it takes on the wire, so one
 * value is one text on every backend. The text is UTF-8, as every string is, so
 * a byte past 0x7F is its two-byte form. */
static inline void sce_saved_write_bytes(sce_jw_t *w, const uint8_t *v, size_t n) {
    sce_jw_put_(w, "\"", 1u);
    for (size_t i = 0u; i < n; ++i) {
        if (v[i] < 0x80u) {
            const char one = (char)v[i];
            sce_jw_escaped_(w, &one, 1u);
        } else {
            char two[2];
            two[0] = (char)(0xC0u | (v[i] >> 6));
            two[1] = (char)(0x80u | (v[i] & 0x3Fu));
            sce_jw_put_(w, two, 2u);
        }
    }
    sce_jw_put_(w, "\"", 1u);
}

/* ── Read: a value back from its saved form ───────────────────────────────── */

/* The refusal of `v`, saved for `what`, which is not `kind` (`a number`, `a
 * text`, ...). A null is named apart: it is an absent value (docs/adr/0055),
 * which only a place the restoring build declares optional reads, so a state
 * saved after a widening is refused at its first null by a build from before it. */
static inline bool sce_saved_not_a_(sce_jv_t v, const char *what, const char *kind, sce_saved_error_t *err) {
    if (sce_jv_is_null(v)) {
        return sce_saved_refuse(err, "'%s' is absent (null), and its type is not optional", what);
    }
    return sce_saved_refuse(err, "'%s' is not %s", what, kind);
}

/* `text` (`n` bytes) as an optional minus and then digits, nothing else, and the
 * signed 64-bit whole number it spells when that fits. */
static inline bool sce_saved_parse_int64_(const char *text, size_t n, int64_t *out) {
    const size_t at = (n > 0u && text[0] == '-') ? 1u : 0u;
    if (at >= n) {
        return false;
    }
    uint64_t magnitude = 0u;
    for (size_t i = at; i < n; ++i) {
        if (text[i] < '0' || text[i] > '9') {
            return false;
        }
        const uint64_t digit = (uint64_t)(text[i] - '0');
        if (magnitude > (UINT64_MAX - digit) / 10u) {
            return false;
        }
        magnitude = magnitude * 10u + digit;
    }
    if (at == 1u) {
        if (magnitude > (uint64_t)INT64_MAX + 1u) {
            return false;
        }
        *out = magnitude == (uint64_t)INT64_MAX + 1u ? INT64_MIN : -(int64_t)magnitude;
    } else {
        if (magnitude > (uint64_t)INT64_MAX) {
            return false;
        }
        *out = (int64_t)magnitude;
    }
    return true;
}

static inline bool sce_saved_parse_uint64_(const char *text, size_t n, uint64_t *out) {
    uint64_t magnitude = 0u;
    if (n == 0u) {
        return false;
    }
    for (size_t i = 0u; i < n; ++i) {
        if (text[i] < '0' || text[i] > '9') {
            return false;
        }
        const uint64_t digit = (uint64_t)(text[i] - '0');
        if (magnitude > (UINT64_MAX - digit) / 10u) {
            return false;
        }
        magnitude = magnitude * 10u + digit;
    }
    *out = magnitude;
    return true;
}

/* A number of at least `least` and at most `most`, written as a whole. */
static inline bool sce_saved_whole_(sce_jv_t v, const char *what, int64_t least, int64_t most, int64_t *out,
                                    sce_saved_error_t *err) {
    int64_t n = 0;
    if (sce_jv_kind(v) != SCE_JSON_NUMBER) {
        return sce_saved_not_a_(v, what, "a number", err);
    }
    if (!sce_saved_parse_int64_(v.p, sce_jv_number_len(v), &n) || n < least || n > most) {
        return sce_saved_refuse(err, "'%s' (%.*s) is not a whole number its type can hold", what,
                                (int)sce_jv_number_len(v), v.p);
    }
    *out = n;
    return true;
}

#define SCE_SAVED_READ_WHOLE_(suffix, type, least, most)                                                               \
    static inline bool sce_saved_read_##suffix(sce_jv_t v, const char *what, type *out, sce_saved_error_t *err) {      \
        int64_t n = 0;                                                                                                 \
        if (!sce_saved_whole_(v, what, (least), (most), &n, err)) {                                                    \
            return false;                                                                                              \
        }                                                                                                              \
        *out = (type)n;                                                                                                \
        return true;                                                                                                   \
    }

SCE_SAVED_READ_WHOLE_(uint8, uint8_t, 0, 0xFF)
SCE_SAVED_READ_WHOLE_(uint16, uint16_t, 0, 0xFFFF)
SCE_SAVED_READ_WHOLE_(uint32, uint32_t, 0, 0xFFFFFFFFLL)
SCE_SAVED_READ_WHOLE_(int8, int8_t, -128, 127)
SCE_SAVED_READ_WHOLE_(int16, int16_t, -32768, 32767)
SCE_SAVED_READ_WHOLE_(int32, int32_t, INT32_MIN, INT32_MAX)

#undef SCE_SAVED_READ_WHOLE_

/* A 64-bit whole number, written as a text, read into `text`. */
static inline bool sce_saved_wide_text_(sce_jv_t v, const char *what, char *text, size_t cap, size_t *len,
                                        sce_saved_error_t *err) {
    if (sce_jv_kind(v) != SCE_JSON_STRING) {
        return sce_saved_not_a_(v, what, "a 64-bit whole number written as a text", err);
    }
    if (!sce_jv_string(v, text, cap, len)) {
        return sce_saved_refuse(err, "'%s' (%s) is not a whole number its type can hold", what, text);
    }
    return true;
}

static inline bool sce_saved_read_uint64(sce_jv_t v, const char *what, uint64_t *out, sce_saved_error_t *err) {
    char text[40];
    size_t n = 0u;
    if (!sce_saved_wide_text_(v, what, text, sizeof(text), &n, err)) {
        return false;
    }
    if (!sce_saved_parse_uint64_(text, n, out)) {
        return sce_saved_refuse(err, "'%s' (%s) is not a whole number its type can hold", what, text);
    }
    return true;
}

static inline bool sce_saved_read_int64(sce_jv_t v, const char *what, int64_t *out, sce_saved_error_t *err) {
    char text[40];
    size_t n = 0u;
    if (!sce_saved_wide_text_(v, what, text, sizeof(text), &n, err)) {
        return false;
    }
    if (!sce_saved_parse_int64_(text, n, out)) {
        return sce_saved_refuse(err, "'%s' (%s) is not a whole number its type can hold", what, text);
    }
    return true;
}

/* A real: a number, or the text a value JSON cannot spell is written as. A number
 * past the range of a 64-bit real reads as the infinity it rounds to, as every
 * other backend reads it; only a text that is no number is refused. */
static inline bool sce_saved_read_float64(sce_jv_t v, const char *what, double *out, sce_saved_error_t *err) {
    char text[64];
    char *stop = NULL;
    size_t n = 0u;
    double real = 0.0;
    if (sce_jv_kind(v) == SCE_JSON_STRING) {
        if (sce_jv_string_is(v, "NaN")) {
            *out = (double)NAN;
            return true;
        }
        if (sce_jv_string_is(v, "Infinity")) {
            *out = (double)INFINITY;
            return true;
        }
        if (sce_jv_string_is(v, "-Infinity")) {
            *out = -(double)INFINITY;
            return true;
        }
    }
    if (sce_jv_kind(v) != SCE_JSON_NUMBER) {
        return sce_saved_not_a_(v, what, "a number", err);
    }
    n = sce_jv_number_len(v);
    if (n >= sizeof(text)) {
        /* Longer than any real's shortest spelling, but a number all the same. */
        n = sizeof(text) - 1u;
    }
    memcpy(text, v.p, n);
    text[n] = '\0';
    real = strtod(text, &stop);
    if (stop != text + n) {
        return sce_saved_refuse(err, "'%s' (%s) is not a number", what, text);
    }
    *out = real;
    return true;
}

static inline bool sce_saved_read_float32(sce_jv_t v, const char *what, float *out, sce_saved_error_t *err) {
    double real = 0.0;
    if (!sce_saved_read_float64(v, what, &real, err)) {
        return false;
    }
    *out = (float)real;
    return true;
}

static inline bool sce_saved_read_bool(sce_jv_t v, const char *what, bool *out, sce_saved_error_t *err) {
    if (sce_jv_kind(v) != SCE_JSON_BOOL) {
        return sce_saved_not_a_(v, what, "a truth value", err);
    }
    *out = sce_jv_as_bool(v);
    return true;
}

/* A text read back only if it holds no more than the `capacity` UTF-8 bytes the
 * machine bounds it by: a machine never holds more, and a restored one must not
 * be the first to. `out` has room for `capacity` bytes and a terminator. */
static inline bool sce_saved_read_string_bounded(sce_jv_t v, const char *what, char *out, size_t capacity,
                                                 sce_saved_error_t *err) {
    size_t held = 0u;
    if (sce_jv_kind(v) != SCE_JSON_STRING) {
        return sce_saved_not_a_(v, what, "a text", err);
    }
    held = sce_jv_string_len(v);
    if (held > capacity) {
        return sce_saved_refuse(err, "'%s' holds %lu UTF-8 bytes, past the %lu it is bounded by", what,
                                (unsigned long)held, (unsigned long)capacity);
    }
    return sce_jv_string(v, out, capacity + 1u, NULL);
}

/* The code point of the UTF-8 sequence at `text[*at]`, advancing past it; U+FFFD
 * for a byte that starts no well-formed sequence, one byte consumed, as the other
 * backends read a malformed text. */
static inline uint32_t sce_saved_next_code_point_(const unsigned char *text, size_t n, size_t *at) {
    const uint32_t lead = text[*at];
    size_t length;
    uint32_t point;
    if (lead < 0x80u) {
        ++*at;
        return lead;
    }
    if (lead >= 0xC2u && lead <= 0xDFu) {
        length = 2u;
        point = lead & 0x1Fu;
    } else if (lead >= 0xE0u && lead <= 0xEFu) {
        length = 3u;
        point = lead & 0x0Fu;
    } else if (lead >= 0xF0u && lead <= 0xF4u) {
        length = 4u;
        point = lead & 0x07u;
    } else {
        ++*at;
        return 0xFFFDu;
    }
    if (*at + length > n) {
        ++*at;
        return 0xFFFDu;
    }
    for (size_t i = 1u; i < length; ++i) {
        const uint32_t continuation = text[*at + i];
        if ((continuation & 0xC0u) != 0x80u) {
            ++*at;
            return 0xFFFDu;
        }
        point = (point << 6) | (continuation & 0x3Fu);
    }
    if ((length == 3u && point < 0x800u) || (length == 4u && point < 0x10000u) || point > 0x10FFFFu ||
        (point >= 0xD800u && point <= 0xDFFFu)) {
        ++*at;
        return 0xFFFDu;
    }
    *at += length;
    return point;
}

/* The decoded bytes of a string, with room to take back the bytes a malformed
 * sequence read ahead and left unconsumed. */
typedef struct sce_saved_bytes_s {
    sce_jstr_t s;
    unsigned char held[4];
    uint8_t n_held;
} sce_saved_bytes_t;

static inline bool sce_saved_bytes_next_(sce_saved_bytes_t *b, unsigned char *out) {
    if (b->n_held > 0u) {
        *out = b->held[0];
        for (uint8_t i = 1u; i < b->n_held; ++i) {
            b->held[i - 1u] = b->held[i];
        }
        --b->n_held;
        return true;
    }
    return sce_jstr_next(&b->s, out);
}

/* The next code point of the decoded string; false at its end. */
static inline bool sce_saved_bytes_point_(sce_saved_bytes_t *b, uint32_t *point) {
    unsigned char seq[4];
    unsigned char c;
    size_t length;
    size_t got = 1u;
    if (!sce_saved_bytes_next_(b, &seq[0])) {
        return false;
    }
    if (seq[0] < 0x80u) {
        *point = seq[0];
        return true;
    }
    if (seq[0] >= 0xC2u && seq[0] <= 0xDFu) {
        length = 2u;
    } else if (seq[0] >= 0xE0u && seq[0] <= 0xEFu) {
        length = 3u;
    } else if (seq[0] >= 0xF0u && seq[0] <= 0xF4u) {
        length = 4u;
    } else {
        *point = 0xFFFDu;
        return true;
    }
    while (got < length && sce_saved_bytes_next_(b, &c)) {
        seq[got++] = c;
    }
    {
        size_t at = 0u;
        const uint32_t read = sce_saved_next_code_point_(seq, got, &at);
        /* What the sequence did not use is read again as the start of the next. */
        for (size_t i = got; i > at; --i) {
            for (uint8_t k = b->n_held; k > 0u; --k) {
                b->held[k] = b->held[k - 1u];
            }
            b->held[0] = seq[i - 1u];
            ++b->n_held;
        }
        *point = read;
    }
    return true;
}

/* A byte string read back from its Latin-1 text (`sce_saved_write_bytes`), only if
 * it holds no more than the `capacity` bytes the machine bounds it by. A character
 * past U+00FF is no byte, and is refused rather than cut. */
static inline bool sce_saved_read_bytes(sce_jv_t v, const char *what, uint8_t *out, size_t capacity, size_t *len,
                                        sce_saved_error_t *err) {
    sce_saved_bytes_t b;
    uint32_t point = 0u;
    size_t held = 0u;
    if (sce_jv_kind(v) != SCE_JSON_STRING) {
        return sce_saved_not_a_(v, what, "a text", err);
    }
    sce_jstr_init(&b.s, v);
    b.n_held = 0u;
    while (sce_saved_bytes_point_(&b, &point)) {
        if (point > 0xFFu) {
            return sce_saved_refuse(err, "'%s' holds U+%04X, which is past U+00FF and so no byte", what,
                                    (unsigned)point);
        }
        if (held < capacity) {
            out[held] = (uint8_t)point;
        }
        ++held;
    }
    if (held > capacity) {
        return sce_saved_refuse(err, "'%s' holds %lu bytes, past the %lu it is bounded by", what, (unsigned long)held,
                                (unsigned long)capacity);
    }
    *len = held;
    return true;
}

/* The field `name` of a saved record value: what a generated record reads each
 * field with. */
static inline bool sce_saved_field(sce_jv_t v, const char *record, const char *name, sce_jv_t *out,
                                   sce_saved_error_t *err) {
    if (sce_jv_kind(v) != SCE_JSON_OBJECT) {
        return sce_saved_refuse(err, "'%s' is not an object", record);
    }
    if (!sce_jv_get(v, name, out)) {
        return sce_saved_refuse(err, "'%s' has no field '%s'", record, name);
    }
    return true;
}

/* A list's element count, refused if it holds more than the `capacity` the machine
 * bounds it by, or if the value is no array. */
static inline bool sce_saved_list(sce_jv_t v, const char *what, size_t capacity, size_t *count,
                                  sce_saved_error_t *err) {
    size_t n = 0u;
    if (sce_jv_kind(v) != SCE_JSON_ARRAY) {
        return sce_saved_not_a_(v, what, "an array", err);
    }
    n = sce_jv_count(v);
    if (n > capacity) {
        return sce_saved_refuse(err, "'%s' holds %lu elements, past the %lu it is bounded by", what, (unsigned long)n,
                                (unsigned long)capacity);
    }
    *count = n;
    return true;
}

/* An enum value read back from the name its document declares for the variant:
 * refused unless it is a text and one of `names`, by that name. `index` is its
 * position in `names`. `alias` is the enum, as the document imports it. */
static inline bool sce_saved_read_variant(sce_jv_t v, const char *what, const char *alias, const char *const *names,
                                          size_t count, size_t *index, sce_saved_error_t *err) {
    char declared[SCE_SAVED_ERROR_LEN / 2u];
    if (sce_jv_kind(v) != SCE_JSON_STRING) {
        return sce_saved_not_a_(v, what, "a text", err);
    }
    for (size_t i = 0u; i < count; ++i) {
        if (sce_jv_string_is(v, names[i])) {
            *index = i;
            return true;
        }
    }
    (void)sce_jv_string(v, declared, sizeof(declared), NULL);
    return sce_saved_refuse(err, "'%s' (%s) is not a variant of %s", what, declared, alias);
}

/* ── A session id ──────────────────────────────────────────────────────────── */

/* XML 1.0 `NameStartChar` and `NameChar`. */
static inline bool sce_saved_name_start_(uint32_t c) {
    return c == 0x3Au || (c >= 0x41u && c <= 0x5Au) || c == 0x5Fu || (c >= 0x61u && c <= 0x7Au) ||
           (c >= 0xC0u && c <= 0xD6u) || (c >= 0xD8u && c <= 0xF6u) || (c >= 0xF8u && c <= 0x2FFu) ||
           (c >= 0x370u && c <= 0x37Du) || (c >= 0x37Fu && c <= 0x1FFFu) || (c >= 0x200Cu && c <= 0x200Du) ||
           (c >= 0x2070u && c <= 0x218Fu) || (c >= 0x2C00u && c <= 0x2FEFu) || (c >= 0x3001u && c <= 0xD7FFu) ||
           (c >= 0xF900u && c <= 0xFDCFu) || (c >= 0xFDF0u && c <= 0xFFFDu) || (c >= 0x10000u && c <= 0xEFFFFu);
}

static inline bool sce_saved_name_char_(uint32_t c) {
    return sce_saved_name_start_(c) || c == 0x2Du || c == 0x2Eu || (c >= 0x30u && c <= 0x39u) || c == 0xB7u ||
           (c >= 0x300u && c <= 0x36Fu) || (c >= 0x203Fu && c <= 0x2040u);
}

/* Whether `id` may be a session's id: an XML 1.0 Nmtoken, one name character or
 * more (production `Nmtoken`, fifth edition), `_sessionid` being one and another
 * session addressing this one as `#_scxml_<id>`. False with the reason in `err`. */
static inline bool sce_saved_session_id_ok(const char *id, sce_saved_error_t *err) {
    const size_t n = strlen(id);
    size_t at = 0u;
    if (n == 0u) {
        return sce_saved_refuse(err, "the session id is empty, and a session id is one name character or more (an "
                                     "XML Nmtoken)");
    }
    while (at < n) {
        const size_t before = at;
        const uint32_t point = sce_saved_next_code_point_((const unsigned char *)id, n, &at);
        /* A byte that starts no well-formed sequence reads as U+FFFD, one byte wide, and is no
         * name character however the replacement character itself is classed. */
        const bool malformed = point == 0xFFFDu && at - before == 1u;
        if (malformed || !sce_saved_name_char_(point)) {
            return sce_saved_refuse(err,
                                    "the session id holds U+%04X, which is not a name character: a session id is an "
                                    "XML Nmtoken (letters, digits, '.', '-', '_', ':')",
                                    (unsigned)point);
        }
    }
    return true;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_SAVED_VALUES_H */
