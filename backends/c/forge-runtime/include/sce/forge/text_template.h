/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * sce/forge/text_template.h — the writer a generated text-template unit renders
 * into (docs/adr/0087, section 3.1): the caller's buffer, filled part by part
 * with no allocation, and the fence of an untrusted hole.
 *
 * The fence: the value is put between two markers, and every occurrence of
 * either marker is first removed from it, so that the value cannot close the
 * fence and write outside it. The removal repeats, taking the leftmost
 * occurrence of either marker (the longer first where both start at one byte),
 * until neither occurs: a value cannot rebuild a marker out of two halves around
 * one that is removed. It ends because each pass shortens the value. The
 * markers are printable ASCII, which no byte of a multi-byte UTF-8 sequence is,
 * so removing them from the bytes of a text leaves a UTF-8 text.
 *
 * A rendering that does not fit its buffer leaves the buffer holding "" and is
 * answered as such, so a text is never cut: a machine renders into a buffer the
 * build sized from the template's MAX_LEN, where that never happens.
 *
 * Holds no state and does no I/O, as every forge runtime header.
 */

#ifndef SCE_FORGE_TEXT_TEMPLATE_H
#define SCE_FORGE_TEXT_TEMPLATE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "sce/forge/algorithm.h" /* sce_forge_algorithm_failure_t, which a rendering that did not fit records */

/* A rendering in progress: the buffer, its capacity with the terminator, the
 * bytes written so far, and whether a part did not fit. */
typedef struct {
    char *buf;
    size_t cap;
    size_t len;
    bool overflow;
} sce_forge_text_t;

/* Start a rendering into `buf`, of `cap` bytes with the terminator. */
static inline sce_forge_text_t sce_forge_text_begin(char *buf, size_t cap) {
    sce_forge_text_t text = {buf, cap, 0u, cap == 0u};
    if (cap > 0u) {
        buf[0] = '\0';
    }
    return text;
}

/* Append the `n` bytes at `bytes`, or record that they do not fit. */
static inline void sce_forge_text_put_bytes(sce_forge_text_t *text, const char *bytes, size_t n) {
    if (text->overflow) {
        return;
    }
    if (n >= text->cap - text->len) {
        text->overflow = true;
        return;
    }
    memcpy(text->buf + text->len, bytes, n);
    text->len += n;
}

/* Append the NUL-terminated `part`. */
static inline void sce_forge_text_put(sce_forge_text_t *text, const char *part) {
    sce_forge_text_put_bytes(text, part, strlen(part));
}

/* Append the decimal digits of an unsigned integer. */
static inline void sce_forge_text_put_uint(sce_forge_text_t *text, uint64_t value) {
    char digits[21];
    int n = snprintf(digits, sizeof digits, "%llu", (unsigned long long)value);
    sce_forge_text_put_bytes(text, digits, n > 0 ? (size_t)n : 0u);
}

/* Append the decimal digits of a signed integer, `-` before a negative one. */
static inline void sce_forge_text_put_int(sce_forge_text_t *text, int64_t value) {
    char digits[21];
    int n = snprintf(digits, sizeof digits, "%lld", (long long)value);
    sce_forge_text_put_bytes(text, digits, n > 0 ? (size_t)n : 0u);
}

/* Remove every occurrence of `open` and `close` from the `len` bytes at `text`,
 * in place, by the rule of the header, and answer how many bytes are kept. An
 * empty marker is no marker (a template's fence holds none; the build refuses
 * it). */
static inline size_t sce_forge_text_remove_markers(char *text, size_t len, const char *open, const char *close) {
    size_t open_len = strlen(open);
    size_t close_len = strlen(close);
    size_t longest = open_len > close_len ? open_len : close_len;
    size_t from = 0u;
    for (;;) {
        size_t at = len;
        size_t marker = 0u;
        for (size_t i = from; i < len && at == len; ++i) {
            bool opens = open_len > 0u && len - i >= open_len && memcmp(text + i, open, open_len) == 0;
            bool closes = close_len > 0u && len - i >= close_len && memcmp(text + i, close, close_len) == 0;
            if (opens || closes) {
                at = i;
                marker = opens && closes ? longest : (opens ? open_len : close_len);
            }
        }
        if (at == len) {
            return len;
        }
        memmove(text + at, text + at + marker, len - at - marker);
        len -= marker;
        /* Before at - (longest - 1) nothing changed and nothing matched, so a
         * marker the removal made can only start at or after it. */
        from = at >= longest - 1u ? at - (longest - 1u) : 0u;
    }
}

/* Append an untrusted hole's `value` between `open` and `close`, every marker
 * removed from it first. */
static inline void sce_forge_text_put_fenced(sce_forge_text_t *text, const char *value, const char *open,
                                             const char *close) {
    sce_forge_text_put(text, open);
    size_t start = text->len;
    sce_forge_text_put(text, value);
    if (!text->overflow) {
        text->len = start + sce_forge_text_remove_markers(text->buf + start, text->len - start, open, close);
    }
    sce_forge_text_put(text, close);
}

/* Finish the rendering: the buffer and true when every part fitted, else the
 * buffer holding "" and false. */
static inline bool sce_forge_text_end(sce_forge_text_t *text) {
    if (text->overflow) {
        if (text->cap > 0u) {
            text->buf[0] = '\0';
        }
        return false;
    }
    text->buf[text->len] = '\0';
    return true;
}

/* What a statement of a `sce-static` machine reads of a rendering: `rendered`,
 * or "" with a capacity failure recorded in `f` when the unit answered NULL (the
 * text did not fit), which the statement around it receives as it does a
 * checked operation. */
static inline const char *sce_forge_text_rendered(sce_forge_algorithm_failure_t *f, const char *rendered) {
    if (rendered == NULL) {
        sce_forge_algorithm_fail(f, SCE_FORGE_ALGORITHM_CAPACITY_EXCEEDED);
        return "";
    }
    return rendered;
}

#endif /* SCE_FORGE_TEXT_TEMPLATE_H */
