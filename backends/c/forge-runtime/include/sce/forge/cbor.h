/* SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * SCE Forge — the CBOR (RFC 8949) items a `sce:encoding="cbor"` codec reads
 * and writes (SCE_FORGE.md §4.6.1): one definite-length map whose keys are
 * small unsigned integers and whose values are unsigned integers, booleans,
 * text strings and byte strings.
 *
 * Mirrors `backends/rust/forge-runtime/src/cbor.rs`, rule for rule. A
 * generated codec calls these; it spells no CBOR of its own.
 *
 * Writing is deterministic (RFC 8949 §4.2.1): every head in its shortest
 * form, so two backends given the same value write the same bytes. Reading
 * takes a head in any valid length — what it reads is the value — and
 * refuses what the codec cannot read as its entry: a reserved
 * additional-information value, an indefinite length, another major type.
 *
 * No allocation and no recursion, as the rest of this runtime: a string is
 * read into the caller's fixed array, and the skip of an unknown value walks
 * a fixed stack of SCE_FORGE_CBOR_MAX_SKIP_DEPTH + 1 levels rather than
 * calling itself.
 */

#ifndef SCE_FORGE_CBOR_H
#define SCE_FORGE_CBOR_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "sce/forge/codec.h"

#ifdef __cplusplus
extern "C" {
#endif

enum {
    SCE_FORGE_CBOR_MAJOR_UNSIGNED = 0,
    SCE_FORGE_CBOR_MAJOR_BYTES = 2,
    SCE_FORGE_CBOR_MAJOR_TEXT = 3,
    SCE_FORGE_CBOR_MAJOR_MAP = 5,
    SCE_FORGE_CBOR_MAJOR_SIMPLE = 7,
    /* The deepest an unknown entry's value may nest before a skip refuses
     * it (SCE_FORGE.md §4.6.1). */
    SCE_FORGE_CBOR_MAX_SKIP_DEPTH = 16
};

/* ── Writing ───────────────────────────────────────────────────────── */

/* Write a head of `major` carrying `value`, in its shortest form. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_head(sce_forge_writer_t *w, uint8_t major, uint64_t value) {
    uint8_t head[9];
    size_t n = 0;
    size_t width = 0;
    const uint8_t m = (uint8_t)(major << 5);
    if (value < 24U) {
        head[n++] = (uint8_t)(m | (uint8_t)value);
    } else if (value <= 0xFFU) {
        head[n++] = (uint8_t)(m | 24U);
        width = 1;
    } else if (value <= 0xFFFFU) {
        head[n++] = (uint8_t)(m | 25U);
        width = 2;
    } else if (value <= 0xFFFFFFFFU) {
        head[n++] = (uint8_t)(m | 26U);
        width = 4;
    } else {
        head[n++] = (uint8_t)(m | 27U);
        width = 8;
    }
    while (width > 0U) {
        width--;
        head[n++] = (uint8_t)(value >> (8U * width));
    }
    return sce_forge_writer_write_bytes(w, head, n);
}

/* Write an unsigned integer. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_uint(sce_forge_writer_t *w, uint64_t value) {
    return sce_forge_cbor_write_head(w, SCE_FORGE_CBOR_MAJOR_UNSIGNED, value);
}

/* Write `false` or `true`. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_bool(sce_forge_writer_t *w, bool value) {
    return sce_forge_writer_write_u8(w, (uint8_t)((SCE_FORGE_CBOR_MAJOR_SIMPLE << 5) | (value ? 21U : 20U)));
}

/* Write a text string of `len` bytes. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_text(sce_forge_writer_t *w, const char *data, size_t len) {
    sce_forge_codec_status_t s = sce_forge_cbor_write_head(w, SCE_FORGE_CBOR_MAJOR_TEXT, (uint64_t)len);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    return sce_forge_writer_write_bytes(w, (const uint8_t *)data, len);
}

/* Write a byte string of `len` bytes. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_bytes(sce_forge_writer_t *w, const uint8_t *data,
                                                                  size_t len) {
    sce_forge_codec_status_t s = sce_forge_cbor_write_head(w, SCE_FORGE_CBOR_MAJOR_BYTES, (uint64_t)len);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    return sce_forge_writer_write_bytes(w, data, len);
}

/* Write the head of a definite-length map of `entries` entries. */
static inline sce_forge_codec_status_t sce_forge_cbor_write_map_head(sce_forge_writer_t *w, uint64_t entries) {
    return sce_forge_cbor_write_head(w, SCE_FORGE_CBOR_MAJOR_MAP, entries);
}

/* ── Reading ───────────────────────────────────────────────────────── */

static inline sce_forge_codec_status_t sce_forge_cbor_read_be(sce_forge_cursor_t *c, size_t n, uint64_t *out) {
    const uint8_t *p = sce_forge_cursor_peek(c, n);
    uint64_t v = 0;
    size_t i;
    if (p == NULL) {
        return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    for (i = 0; i < n; ++i) {
        v = (v << 8) | p[i];
    }
    (void)sce_forge_cursor_advance(c, n);
    *out = v;
    return SCE_FORGE_CODEC_OK;
}

/* Read one head: its major type and the value its additional information
 * carries (for a string or a map, the length). An indefinite length and the
 * reserved values 28–30 are refused. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_head(sce_forge_cursor_t *c, uint8_t *major,
                                                                uint64_t *value) {
    uint64_t initial = 0;
    uint8_t info;
    sce_forge_codec_status_t s = sce_forge_cbor_read_be(c, 1, &initial);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    *major = (uint8_t)(initial >> 5);
    info = (uint8_t)(initial & 0x1FU);
    if (info < 24U) {
        *value = info;
        return SCE_FORGE_CODEC_OK;
    }
    switch (info) {
    case 24:
        return sce_forge_cbor_read_be(c, 1, value);
    case 25:
        return sce_forge_cbor_read_be(c, 2, value);
    case 26:
        return sce_forge_cbor_read_be(c, 4, value);
    case 27:
        return sce_forge_cbor_read_be(c, 8, value);
    default:
        return SCE_FORGE_CODEC_CBOR_MALFORMED;
    }
}

static inline sce_forge_codec_status_t sce_forge_cbor_expect(sce_forge_cursor_t *c, uint8_t major, uint64_t *value) {
    uint8_t m = 0;
    sce_forge_codec_status_t s = sce_forge_cbor_read_head(c, &m, value);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    return m == major ? SCE_FORGE_CODEC_OK : SCE_FORGE_CODEC_CBOR_MALFORMED;
}

/* Read an unsigned integer. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_uint(sce_forge_cursor_t *c, uint64_t *out) {
    return sce_forge_cbor_expect(c, SCE_FORGE_CBOR_MAJOR_UNSIGNED, out);
}

/* Read an unsigned integer an entry of `max` holds. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_uint_upto(sce_forge_cursor_t *c, uint64_t max,
                                                                     uint64_t *out) {
    sce_forge_codec_status_t s = sce_forge_cbor_read_uint(c, out);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    return *out > max ? SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE : SCE_FORGE_CODEC_OK;
}

/* Read `false` or `true`. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_bool(sce_forge_cursor_t *c, bool *out) {
    uint8_t m = 0;
    uint64_t value = 0;
    sce_forge_codec_status_t s = sce_forge_cbor_read_head(c, &m, &value);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (m != SCE_FORGE_CBOR_MAJOR_SIMPLE || (value != 20U && value != 21U)) {
        return SCE_FORGE_CODEC_CBOR_MALFORMED;
    }
    *out = value == 21U;
    return SCE_FORGE_CODEC_OK;
}

/* Copy a string payload of `len` bytes into `dst`. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_payload(sce_forge_cursor_t *c, uint64_t len, uint8_t *dst) {
    const uint8_t *p;
    if (len > (uint64_t)sce_forge_cursor_remaining(c)) {
        return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    p = sce_forge_cursor_peek(c, (size_t)len);
    if (dst != NULL && len > 0U) {
        memcpy(dst, p, (size_t)len);
    }
    (void)sce_forge_cursor_advance(c, (size_t)len);
    return SCE_FORGE_CODEC_OK;
}

/* Read a UTF-8 text string of at most `max_size` bytes into `dst` (which
 * holds `max_size`), its length into `*len`. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_text(sce_forge_cursor_t *c, size_t max_size, char *dst,
                                                                size_t *len) {
    uint64_t n = 0;
    sce_forge_codec_status_t s = sce_forge_cbor_expect(c, SCE_FORGE_CBOR_MAJOR_TEXT, &n);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (n > (uint64_t)max_size) {
        return SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE;
    }
    s = sce_forge_cbor_read_payload(c, n, (uint8_t *)dst);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (!sce_forge_is_valid_utf8((const uint8_t *)dst, (size_t)n)) {
        return SCE_FORGE_CODEC_INVALID_UTF8;
    }
    *len = (size_t)n;
    return SCE_FORGE_CODEC_OK;
}

/* Read a byte string of at most `max_size` bytes into `dst` (which holds
 * `max_size`), its length into `*len`. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_bytes(sce_forge_cursor_t *c, size_t max_size, uint8_t *dst,
                                                                 size_t *len) {
    uint64_t n = 0;
    sce_forge_codec_status_t s = sce_forge_cbor_expect(c, SCE_FORGE_CBOR_MAJOR_BYTES, &n);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (n > (uint64_t)max_size) {
        return SCE_FORGE_CODEC_CBOR_OUT_OF_RANGE;
    }
    s = sce_forge_cbor_read_payload(c, n, dst);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    *len = (size_t)n;
    return SCE_FORGE_CODEC_OK;
}

/* Read a byte string of exactly `length` bytes into `dst`. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_bytes_exact(sce_forge_cursor_t *c, size_t length,
                                                                       uint8_t *dst) {
    uint64_t n = 0;
    sce_forge_codec_status_t s = sce_forge_cbor_expect(c, SCE_FORGE_CBOR_MAJOR_BYTES, &n);
    if (s != SCE_FORGE_CODEC_OK) {
        return s;
    }
    if (n != (uint64_t)length) {
        return SCE_FORGE_CODEC_CBOR_WRONG_LENGTH;
    }
    return sce_forge_cbor_read_payload(c, n, dst);
}

/* Read the head of a definite-length map: how many entries follow. */
static inline sce_forge_codec_status_t sce_forge_cbor_read_map_len(sce_forge_cursor_t *c, uint64_t *out) {
    return sce_forge_cbor_expect(c, SCE_FORGE_CBOR_MAJOR_MAP, out);
}

/* Skip one item — the value of a key the codec does not declare — and
 * everything nested in it, refusing one nested deeper than
 * SCE_FORGE_CBOR_MAX_SKIP_DEPTH. `pending[d]` counts what is still to be
 * read at nesting depth `d` — items of an array or a tag, entries of a map,
 * whose key and value `in_value[d]` tells apart — so a map's count is used
 * as written, never doubled. An item is refused when it would be read at
 * depth SCE_FORGE_CBOR_MAX_SKIP_DEPTH, the depth at which the Rust
 * runtime's recursive skip refuses it, and every other refusal is the one
 * the recursion meets first on the same input. */
static inline sce_forge_codec_status_t sce_forge_cbor_skip(sce_forge_cursor_t *c) {
    uint64_t pending[SCE_FORGE_CBOR_MAX_SKIP_DEPTH + 1];
    bool is_map[SCE_FORGE_CBOR_MAX_SKIP_DEPTH + 1];
    bool in_value[SCE_FORGE_CBOR_MAX_SKIP_DEPTH + 1];
    size_t depth = 0;
    pending[0] = 1;
    is_map[0] = false;
    in_value[0] = false;
    for (;;) {
        uint8_t major = 0;
        uint64_t value = 0;
        sce_forge_codec_status_t s;
        if (pending[depth] == 0U) {
            if (depth == 0U) {
                return SCE_FORGE_CODEC_OK;
            }
            depth--;
            continue;
        }
        if (depth >= (size_t)SCE_FORGE_CBOR_MAX_SKIP_DEPTH) {
            return SCE_FORGE_CODEC_CBOR_TOO_DEEP;
        }
        /* An entry of a map is spent once its value is read. */
        if (is_map[depth] && !in_value[depth]) {
            in_value[depth] = true;
        } else {
            in_value[depth] = false;
            pending[depth]--;
        }
        s = sce_forge_cbor_read_head(c, &major, &value);
        if (s != SCE_FORGE_CODEC_OK) {
            return s;
        }
        switch (major) {
        /* Unsigned, negative, simple / float: the head is the whole item. */
        case 0:
        case 1:
        case 7:
            break;
        case 2:
        case 3:
            s = sce_forge_cbor_read_payload(c, value, NULL);
            if (s != SCE_FORGE_CODEC_OK) {
                return s;
            }
            break;
        case 4:
        case 5:
        /* A tag: its one enclosed item follows. */
        case 6:
            depth++;
            pending[depth] = major == 6U ? 1U : value;
            is_map[depth] = major == 5U;
            in_value[depth] = false;
            break;
        default:
            return SCE_FORGE_CODEC_CBOR_MALFORMED;
        }
    }
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_FORGE_CBOR_H */
