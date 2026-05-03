/* SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael */

/*
 * SCE Forge — codec cursor + typed error contract (C11).
 *
 * Mirrors the Rust reference at `sce-forge-runtime/rust/src/codec.rs`
 * and the C++ header at `sce-forge-runtime/cpp/include/sce/forge/codec.h`.
 * RFC §5.B L494-519 pins a per-language cursor + need-more-bytes
 * contract on decode so a truncated input never aborts — the caller
 * resumes after additional bytes arrive (DMA boundary, fragmented
 * network read).
 *
 * Phase B1-prep: minimum API for fixed-width codec fixtures
 * (sce_forge_cursor_peek + advance + remaining). Streaming readers
 * (read_u8, read_vle_*, read_tag) land in B1-α/β with their first
 * consumer. Encode-side cursor + SCE_FORGE_CODEC_BUFFER_OVERFLOW lands
 * in B1-α.
 */

#ifndef SCE_FORGE_CODEC_H
#define SCE_FORGE_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Typed decode error. The B1-β variant primitive intentionally does
 * NOT need a typed UnknownVariantTag — RFC §5.B requires <sce:default>
 * when arms don't exhaust the tag domain (build-time
 * codec/variant-arm-unreachable otherwise), so the default arm catches
 * every unmatched tag at runtime. */
typedef enum {
    SCE_FORGE_CODEC_OK = 0,
    SCE_FORGE_CODEC_NEED_MORE_BYTES = 1,
    /* A vle_u<N> field's continuation chain implies a value wider than
     * the declared type. RFC §5.B `codec/vle-width-overflow`. */
    SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW = 2,
    /* RFC §5.B B3 TLV chain primitive: the wire carried more entries
     * than the codec author declared (max-depth=N exhausted while the
     * cursor still had bytes) AND the codec declared
     * on-overflow="reject". Truncate-mode codecs never raise this. */
    SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW = 3,
    /* RFC §5.B B5-ζ Surface H string primitive: the byte slice declared
     * `sce:type="string"` was not well-formed UTF-8. Mirrors the typed
     * `CodecError::InvalidUtf8` (Rust / Go / Python) — the C11 enum
     * return is uniform across every codec so adding the variant does
     * not change per-codec signatures (cpp / kotlin collapse to the
     * `std::optional<T>` / `T?` truncation sentinel because their
     * signatures are type-narrow). */
    SCE_FORGE_CODEC_INVALID_UTF8 = 4,
} sce_forge_codec_status_t;

/* Read-only cursor over a borrowed input buffer. Decode bodies bind a
 * cursor at the call site, peek the minimum frame, then advance after
 * the construction succeeds.
 *
 * Fields are exposed by name (not opaque) so the generated decode
 * bodies can `cursor->data + cursor->pos` for positional reads at
 * fixed offsets without an accessor call per field. */
typedef struct {
    const uint8_t *data;
    size_t         len;
    size_t         pos;
} sce_forge_cursor_t;

static inline sce_forge_cursor_t sce_forge_cursor_init(const uint8_t *data, size_t len) {
    sce_forge_cursor_t c;
    c.data = data;
    c.len = len;
    c.pos = 0;
    return c;
}

static inline size_t sce_forge_cursor_remaining(const sce_forge_cursor_t *c) {
    return c->len - c->pos;
}

/* Peek the next `n` bytes without advancing. Returns NULL when the
 * cursor's tail is shorter than `n`. */
static inline const uint8_t *sce_forge_cursor_peek(const sce_forge_cursor_t *c, size_t n) {
    if (sce_forge_cursor_remaining(c) < n) return NULL;
    return c->data + c->pos;
}

/* Advance the cursor by `n` bytes. Returns false if `n` would
 * overrun the buffer. */
static inline bool sce_forge_cursor_advance(sce_forge_cursor_t *c, size_t n) {
    if (sce_forge_cursor_remaining(c) < n) return false;
    c->pos += n;
    return true;
}

/* Read a base-128 variable-length encoded unsigned value of up to
 * `max_bits` payload width into *out. Each byte carries 7 data bits
 * in its low 7; bit 7 is the continuation flag. LSB-first byte order.
 * Mirrors the Zenoh ZInt wire format (RFC §5.B Appendix B).
 *
 * Returns SCE_FORGE_CODEC_OK on success, SCE_FORGE_CODEC_NEED_MORE_BYTES
 * on truncation, SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW when the
 * continuation chain implies a value wider than max_bits. */
static inline sce_forge_codec_status_t sce_forge_cursor_read_vle_inner(
    sce_forge_cursor_t *c, uint32_t max_bits, uint64_t *out) {
    uint32_t max_bytes = (max_bits + 6u) / 7u;
    uint64_t value = 0;
    uint32_t shift = 0;
    for (uint32_t i = 0; i < max_bytes; ++i) {
        const uint8_t *p = sce_forge_cursor_peek(c, 1);
        if (p == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        (void)sce_forge_cursor_advance(c, 1);
        uint64_t payload = (uint64_t)(*p & 0x7Fu);
        if (shift + 7u > max_bits) {
            uint32_t allowed = max_bits - shift;
            uint64_t max_payload = ((uint64_t)1 << allowed) - 1u;
            if (payload > max_payload) return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW;
        }
        value |= payload << shift;
        if ((*p & 0x80u) == 0u) {
            *out = value;
            return SCE_FORGE_CODEC_OK;
        }
        shift += 7u;
    }
    return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW;
}

static inline sce_forge_codec_status_t sce_forge_cursor_read_vle_u16(sce_forge_cursor_t *c, uint16_t *out) {
    uint64_t v;
    sce_forge_codec_status_t st = sce_forge_cursor_read_vle_inner(c, 16, &v);
    if (st == SCE_FORGE_CODEC_OK) *out = (uint16_t)v;
    return st;
}

static inline sce_forge_codec_status_t sce_forge_cursor_read_vle_u32(sce_forge_cursor_t *c, uint32_t *out) {
    uint64_t v;
    sce_forge_codec_status_t st = sce_forge_cursor_read_vle_inner(c, 32, &v);
    if (st == SCE_FORGE_CODEC_OK) *out = (uint32_t)v;
    return st;
}

static inline sce_forge_codec_status_t sce_forge_cursor_read_vle_u64(sce_forge_cursor_t *c, uint64_t *out) {
    return sce_forge_cursor_read_vle_inner(c, 64, out);
}

/* RFC §5.B B5-ζ Surface H — validate that `[p, p + n)` is a well-formed
 * UTF-8 byte sequence. Returns true for valid UTF-8 (including the
 * empty range), false on any malformed sequence. Mirrors the cpp
 * `is_valid_utf8` (sce-forge-runtime/cpp/include/sce/forge/codec.h)
 * RFC 3629 §4 four-form table:
 *   1-byte:  0x00..0x7F                      (ASCII)
 *   2-byte:  0xC2..0xDF + cont                (overlong-rejecting)
 *   3-byte:  0xE0..0xEF + 2*cont              (E0 overlong + ED surrogate guards)
 *   4-byte:  0xF0..0xF4 + 3*cont              (F0 overlong + F4 U+10FFFF cap)
 * Lead bytes 0x80..0xC1 (continuation in lead position + overlong
 * 1-byte) and 0xF5..0xFF (above U+10FFFF) reject.
 *
 * Used by codec decode bodies emitted from `sce:type="string"` fields —
 * malformed input surfaces as SCE_FORGE_CODEC_INVALID_UTF8. */
static inline bool sce_forge_is_valid_utf8(const uint8_t *p, size_t n) {
    size_t i = 0;
    while (i < n) {
        const uint8_t b0 = p[i];
        if (b0 <= 0x7Fu) {
            i += 1;
        } else if (b0 >= 0xC2u && b0 <= 0xDFu) {
            if (i + 1 >= n) return false;
            const uint8_t b1 = p[i + 1];
            if (b1 < 0x80u || b1 > 0xBFu) return false;
            i += 2;
        } else if (b0 >= 0xE0u && b0 <= 0xEFu) {
            if (i + 2 >= n) return false;
            const uint8_t b1 = p[i + 1];
            const uint8_t b2 = p[i + 2];
            const uint8_t b1_min = (b0 == 0xE0u) ? 0xA0u : 0x80u;
            const uint8_t b1_max = (b0 == 0xEDu) ? 0x9Fu : 0xBFu;
            if (b1 < b1_min || b1 > b1_max) return false;
            if (b2 < 0x80u || b2 > 0xBFu) return false;
            i += 3;
        } else if (b0 >= 0xF0u && b0 <= 0xF4u) {
            if (i + 3 >= n) return false;
            const uint8_t b1 = p[i + 1];
            const uint8_t b2 = p[i + 2];
            const uint8_t b3 = p[i + 3];
            const uint8_t b1_min = (b0 == 0xF0u) ? 0x90u : 0x80u;
            const uint8_t b1_max = (b0 == 0xF4u) ? 0x8Fu : 0xBFu;
            if (b1 < b1_min || b1 > b1_max) return false;
            if (b2 < 0x80u || b2 > 0xBFu) return false;
            if (b3 < 0x80u || b3 > 0xBFu) return false;
            i += 4;
        } else {
            return false;
        }
    }
    return true;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_FORGE_CODEC_H */
