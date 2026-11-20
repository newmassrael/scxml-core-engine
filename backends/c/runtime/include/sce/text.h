/*
 * SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
 * SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
 *
 * The text routines a C11 machine's core calls, written once (docs/adr/0110, D1).
 *
 * A machine's core includes only the headers C11 requires of a freestanding
 * implementation and calls no library function but `memcpy`, `memset`, `memmove`
 * and `memcmp`, which GCC and Clang emit calls to even under `-ffreestanding`,
 * so every C environment supplies them. Every other routine the core needs on
 * text (a length, a comparison, a prefix, the last place of a character, a copy,
 * the decimal or hexadecimal digits of an integer) is here, and every unit that
 * needs one calls it from here, the hosted units included, so the runtime holds
 * one of each.
 *
 * The four memory functions are declared here rather than taken from
 * `<string.h>`, which is not a freestanding header. A declaration of a library
 * function without its header is one C allows (C11 7.1.4p2), and it is
 * compatible with the one a hosted unit's `<string.h>` makes. C++ has no such
 * allowance once the library's own declaration carries an exception
 * specification, so a C++ consumer takes them from `<cstring>`: a C++
 * translation unit is never a machine's core.
 *
 * Integer digits are written by division of the width the value has, so a
 * 32-bit value never calls the 64-bit division a 32-bit target supplies in a
 * support library. No routine here formats or reads a real number: the core
 * never does (docs/adr/0110, D1).
 *
 * Block comments, as `sce/portability.h` has them, so the header parses under
 * every standard a consumer may compile it with.
 */
#ifndef SCE_TEXT_H
#define SCE_TEXT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#if defined(__cplusplus)
#include <cstring>
extern "C" {
#else
/* Each name in parentheses, so a library that spells one of them as a
 * function-like macro (a fortified one may) declares the function and not the
 * macro's expansion. */
void *(memcpy)(void *dst, const void *src, size_t n);
void *(memmove)(void *dst, const void *src, size_t n);
void *(memset)(void *dst, int value, size_t n);
int(memcmp)(const void *a, const void *b, size_t n);
#endif

/* The bytes of the longest decimal text of a 64-bit integer, its sign and the
 * terminator included: `-9223372036854775808` is twenty characters. A buffer of
 * this size holds the text of every integer type the core holds. */
#define SCE_TEXT_DECIMAL_CAP 21u

/* The length of `text`, which ends at its first NUL. */
static inline size_t sce_text_length(const char *text) {
    size_t n = 0u;
    while (text[n] != '\0') {
        ++n;
    }
    return n;
}

/* `a` against `b`, byte by byte as unsigned values: below zero when `a` orders
 * first, zero when the two are the same text, above zero otherwise. The order
 * and the sign `strcmp` answers. */
static inline int sce_text_compare(const char *a, const char *b) {
    const unsigned char *x = (const unsigned char *)a;
    const unsigned char *y = (const unsigned char *)b;
    while (*x != 0u && *x == *y) {
        ++x;
        ++y;
    }
    return (int)*x - (int)*y;
}

/* The same over at most the first `n` bytes of each: zero when they agree that
 * far, or as far as both end together. The answer `strncmp` gives. */
static inline int sce_text_compare_n(const char *a, const char *b, size_t n) {
    const unsigned char *x = (const unsigned char *)a;
    const unsigned char *y = (const unsigned char *)b;
    for (size_t i = 0u; i < n; ++i) {
        if (x[i] != y[i] || x[i] == 0u) {
            return (int)x[i] - (int)y[i];
        }
    }
    return 0;
}

/* Whether `text` begins with the `prefix_len` bytes of `prefix`. */
static inline bool sce_text_starts_with(const char *text, const char *prefix, size_t prefix_len) {
    return sce_text_compare_n(text, prefix, prefix_len) == 0;
}

/* The last place of `c` in `text`, or NULL when it holds none. The NUL that ends
 * the text is not looked for. */
static inline const char *sce_text_find_last(const char *text, char c) {
    const char *found = NULL;
    for (const char *p = text; *p != '\0'; ++p) {
        if (*p == c) {
            found = p;
        }
    }
    return found;
}

/* The first place of `c` among the `n` bytes at `bytes`, or NULL when none is
 * `c`. A NUL among them is a byte like any other. */
static inline const void *sce_text_find_byte(const void *bytes, int c, size_t n) {
    const unsigned char *p = (const unsigned char *)bytes;
    for (size_t i = 0u; i < n; ++i) {
        if (p[i] == (unsigned char)c) {
            return p + i;
        }
    }
    return NULL;
}

/* Copy `src` and its NUL to `dst`, which the caller has sized for both. */
static inline char *sce_text_copy(char *dst, const char *src) {
    memcpy(dst, src, sce_text_length(src) + 1u);
    return dst;
}

/* The decimal digits of `value`, written backwards from `end` (one past the last
 * digit), and the place of the first. */
static inline char *sce_text_digits_u32(uint32_t value, char *end) {
    char *p = end;
    do {
        *--p = (char)('0' + (int)(value % 10u));
        value /= 10u;
    } while (value != 0u);
    return p;
}

static inline char *sce_text_digits_u64(uint64_t value, char *end) {
    char *p = end;
    /* The high part by 64-bit division until it fits 32 bits, the rest by the
     * narrower one. */
    while (value > (uint64_t)UINT32_MAX) {
        *--p = (char)('0' + (int)(value % 10u));
        value /= 10u;
    }
    return sce_text_digits_u32((uint32_t)value, p);
}

/* `n` bytes of `text` into `buf` of `cap` bytes, with its NUL: the length, or 0
 * when they do not fit, and `buf` then holds the empty string, never a cut text. */
static inline size_t sce_text_place(const char *text, size_t n, char *buf, size_t cap) {
    if (cap == 0u) {
        return 0u;
    }
    if (n >= cap) {
        buf[0] = '\0';
        return 0u;
    }
    memcpy(buf, text, n);
    buf[n] = '\0';
    return n;
}

/* The decimal text of an integer, a `-` leading a negative one, into `buf` of
 * `cap` bytes: its length, or 0 when it does not fit, and `buf` then holds the
 * empty string. No text of an integer is empty, so 0 is never a length. */
static inline size_t sce_text_of_u32(uint32_t value, char *buf, size_t cap) {
    char digits[SCE_TEXT_DECIMAL_CAP];
    char *end = digits + sizeof(digits);
    const char *first = sce_text_digits_u32(value, end);
    return sce_text_place(first, (size_t)(end - first), buf, cap);
}

static inline size_t sce_text_of_u64(uint64_t value, char *buf, size_t cap) {
    char digits[SCE_TEXT_DECIMAL_CAP];
    char *end = digits + sizeof(digits);
    const char *first = sce_text_digits_u64(value, end);
    return sce_text_place(first, (size_t)(end - first), buf, cap);
}

static inline size_t sce_text_of_i32(int32_t value, char *buf, size_t cap) {
    char digits[SCE_TEXT_DECIMAL_CAP];
    char *end = digits + sizeof(digits);
    /* The magnitude in the unsigned type, which holds that of INT32_MIN. */
    const uint32_t magnitude = value < 0 ? 0u - (uint32_t)value : (uint32_t)value;
    char *first = sce_text_digits_u32(magnitude, end);
    if (value < 0) {
        *--first = '-';
    }
    return sce_text_place(first, (size_t)(end - first), buf, cap);
}

static inline size_t sce_text_of_i64(int64_t value, char *buf, size_t cap) {
    char digits[SCE_TEXT_DECIMAL_CAP];
    char *end = digits + sizeof(digits);
    const uint64_t magnitude = value < 0 ? 0u - (uint64_t)value : (uint64_t)value;
    char *first = sce_text_digits_u64(magnitude, end);
    if (value < 0) {
        *--first = '-';
    }
    return sce_text_place(first, (size_t)(end - first), buf, cap);
}

/* A text built in parts into a buffer the caller owns, as `snprintf` builds one:
 * what fits is kept, the buffer always ends in a NUL (when it has a byte), and a
 * part that did not fit whole leaves `cut` set, so a caller that wants all or
 * nothing asks it once at the end. `len` counts the bytes kept. */
typedef struct sce_text_writer {
    char *buf;
    size_t cap;
    size_t len;
    bool cut;
} sce_text_writer_t;

/* A writer over `buf` of `cap` bytes, which holds the empty string from here. */
static inline sce_text_writer_t sce_text_writer(char *buf, size_t cap) {
    sce_text_writer_t w;
    w.buf = buf;
    w.cap = cap;
    w.len = 0u;
    w.cut = cap == 0u;
    if (cap > 0u) {
        buf[0] = '\0';
    }
    return w;
}

/* `n` bytes of `text`, or as many as fit before the terminator. */
static inline void sce_text_put(sce_text_writer_t *w, const char *text, size_t n) {
    if (w->cap == 0u) {
        w->cut = w->cut || n > 0u;
        return;
    }
    const size_t room = w->cap - 1u - w->len;
    size_t kept = n;
    if (kept > room) {
        kept = room;
        w->cut = true;
    }
    memcpy(w->buf + w->len, text, kept);
    w->len += kept;
    w->buf[w->len] = '\0';
}

/* The string literal `literal` (a literal, never a pointer), its length the
 * compiler's: a text the program spells is put at the size it is known to have,
 * so no length is measured at run time and an optimizer can bound every copy
 * by the literal's own storage. A text of the run is put by
 * `sce_text_put_text`. */
#define SCE_TEXT_PUT_LITERAL(w, literal) sce_text_put((w), (literal), sizeof(literal) - 1u)

static inline void sce_text_put_text(sce_text_writer_t *w, const char *text) {
    sce_text_put(w, text, sce_text_length(text));
}

/* At most `max` bytes of `text`, which ends at its first NUL or there: what
 * `%.<max>s` of `printf` writes. */
static inline void sce_text_put_within(sce_text_writer_t *w, const char *text, size_t max) {
    size_t n = 0u;
    while (n < max && text[n] != '\0') {
        ++n;
    }
    sce_text_put(w, text, n);
}

static inline void sce_text_put_char(sce_text_writer_t *w, char c) {
    sce_text_put(w, &c, 1u);
}

static inline void sce_text_put_u64(sce_text_writer_t *w, uint64_t value) {
    char digits[SCE_TEXT_DECIMAL_CAP];
    sce_text_put(w, digits, sce_text_of_u64(value, digits, sizeof(digits)));
}

static inline void sce_text_put_i64(sce_text_writer_t *w, int64_t value) {
    char digits[SCE_TEXT_DECIMAL_CAP];
    sce_text_put(w, digits, sce_text_of_i64(value, digits, sizeof(digits)));
}

/* The lowercase hexadecimal digits of `value`, at least `width` of them, zeros
 * leading: `%0<width>x` of `printf`, of any unsigned width up to 64 bits. */
static inline void sce_text_put_hex(sce_text_writer_t *w, uint64_t value, unsigned width) {
    static const char hex[] = "0123456789abcdef";
    char digits[16];
    unsigned n = 0u;
    do {
        digits[n++] = hex[value & 0xFu];
        value >>= 4;
    } while (value != 0u);
    while (n < width && n < (unsigned)sizeof(digits)) {
        digits[n++] = '0';
    }
    while (n > 0u) {
        sce_text_put_char(w, digits[--n]);
    }
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_TEXT_H */
