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
 * Phase B1-prep ships the minimum API for fixed-width codec fixtures
 * (sce_forge_cursor_peek + advance + remaining). Encode-side
 * `sce_forge_writer_t` + SCE_FORGE_CODEC_BUFFER_OVERFLOW lands in B1-α:
 * `sce_forge_writer_init_buf` wraps a caller-owned buffer + capacity,
 * raises BUFFER_OVERFLOW when a write would exceed the cap.
 *
 * C11 runtime intentionally has no heap-backed writer variant — `alloc`
 * is the inversion-1 contract boundary (MCU + AP targets share the
 * no-alloc primary). Callers needing dynamic sizing should
 * pre-reserve `{NAME}_MAX_ENCODED_BYTES` (per-codec macro) on the
 * stack or in a static arena and run `{name}_encode_to_buf` over it.
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
    /* RFC §5.B B1-α encode-side counterpart to NEED_MORE_BYTES: the
     * destination writer reported insufficient remaining capacity for
     * the next write. Only the bounded `sce_forge_writer_t`
     * (caller-owned buf + cap) can raise this; C11 has no heap-backed
     * variant per the runtime's no-alloc primary contract. Codec
     * authors pre-reserve `{NAME}_MAX_ENCODED_BYTES` for the codec
     * being emitted, so a sized buffer never produces this status. */
    SCE_FORGE_CODEC_BUFFER_OVERFLOW = 5,
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

/* ── Write-side writer ──────────────────────────────────────────── */

/* Write-side cursor for codec emit. Wraps a caller-owned `uint8_t*`
 * buffer + capacity; bounded — raises SCE_FORGE_CODEC_BUFFER_OVERFLOW
 * when a write would exceed `cap`. Mirrors the read-side
 * `sce_forge_cursor_t` borrow contract: the caller owns the
 * destination storage, the writer holds a non-owning pointer.
 *
 * Fields are exposed by name (not opaque) so generated encode bodies
 * can fast-path through `bytes[pos++] = X` in the inlined helpers
 * below without an accessor call per byte. */
typedef struct {
    uint8_t *bytes;
    size_t   cap;
    size_t   pos;
} sce_forge_writer_t;

/* Wrap `buf` with write position 0. Caller owns the storage; the
 * writer does not free or extend it. */
static inline sce_forge_writer_t sce_forge_writer_init_buf(uint8_t *buf, size_t cap) {
    sce_forge_writer_t w;
    w.bytes = buf;
    w.cap = cap;
    w.pos = 0;
    return w;
}

/* Bytes written by this writer instance since `init_buf`. Mirrors the
 * cpp `SceSink::position()` semantics — distinct from
 * `cap - remaining` so the same accessor lifts onto a future
 * heap-backed writer without contract change. Used by codec emit for
 * offset-aware writes (DMA-aligned field padding, length-prefix
 * back-patching). */
static inline size_t sce_forge_writer_position(const sce_forge_writer_t *w) {
    return w->pos;
}

/* Remaining capacity from current position to end of buffer. */
static inline size_t sce_forge_writer_remaining(const sce_forge_writer_t *w) {
    return w->cap - w->pos;
}

/* Append `n` bytes from `data` to the underlying storage. Raises
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the destination has
 * insufficient remaining capacity. Zero-length writes succeed even at
 * exact saturation (codec emit sites pass zero counts for absent
 * optional fields). */
static inline sce_forge_codec_status_t sce_forge_writer_write_bytes(
    sce_forge_writer_t *w, const uint8_t *data, size_t n) {
    if (n == 0) return SCE_FORGE_CODEC_OK;
    if (w->cap - w->pos < n) return SCE_FORGE_CODEC_BUFFER_OVERFLOW;
    for (size_t i = 0; i < n; ++i) w->bytes[w->pos + i] = data[i];
    w->pos += n;
    return SCE_FORGE_CODEC_OK;
}

/* Append a single byte. Inlined to keep the codec emit hot path one
 * branch per write. */
static inline sce_forge_codec_status_t sce_forge_writer_write_u8(
    sce_forge_writer_t *w, uint8_t v) {
    if (w->pos >= w->cap) return SCE_FORGE_CODEC_BUFFER_OVERFLOW;
    w->bytes[w->pos++] = v;
    return SCE_FORGE_CODEC_OK;
}

/* Multi-byte helpers — explicit endianness, no host-byte-order
 * dependence. Forwards to `write_bytes` so the bounded check fires
 * once per call. */
static inline sce_forge_codec_status_t sce_forge_writer_write_u16_le(
    sce_forge_writer_t *w, uint16_t v) {
    uint8_t buf[2] = { (uint8_t)v, (uint8_t)(v >> 8) };
    return sce_forge_writer_write_bytes(w, buf, 2);
}
static inline sce_forge_codec_status_t sce_forge_writer_write_u16_be(
    sce_forge_writer_t *w, uint16_t v) {
    uint8_t buf[2] = { (uint8_t)(v >> 8), (uint8_t)v };
    return sce_forge_writer_write_bytes(w, buf, 2);
}
static inline sce_forge_codec_status_t sce_forge_writer_write_u32_le(
    sce_forge_writer_t *w, uint32_t v) {
    uint8_t buf[4] = {
        (uint8_t)v, (uint8_t)(v >> 8), (uint8_t)(v >> 16), (uint8_t)(v >> 24)
    };
    return sce_forge_writer_write_bytes(w, buf, 4);
}
static inline sce_forge_codec_status_t sce_forge_writer_write_u32_be(
    sce_forge_writer_t *w, uint32_t v) {
    uint8_t buf[4] = {
        (uint8_t)(v >> 24), (uint8_t)(v >> 16), (uint8_t)(v >> 8), (uint8_t)v
    };
    return sce_forge_writer_write_bytes(w, buf, 4);
}
static inline sce_forge_codec_status_t sce_forge_writer_write_u64_le(
    sce_forge_writer_t *w, uint64_t v) {
    uint8_t buf[8] = {
        (uint8_t)v,         (uint8_t)(v >> 8),  (uint8_t)(v >> 16), (uint8_t)(v >> 24),
        (uint8_t)(v >> 32), (uint8_t)(v >> 40), (uint8_t)(v >> 48), (uint8_t)(v >> 56)
    };
    return sce_forge_writer_write_bytes(w, buf, 8);
}
static inline sce_forge_codec_status_t sce_forge_writer_write_u64_be(
    sce_forge_writer_t *w, uint64_t v) {
    uint8_t buf[8] = {
        (uint8_t)(v >> 56), (uint8_t)(v >> 48), (uint8_t)(v >> 40), (uint8_t)(v >> 32),
        (uint8_t)(v >> 24), (uint8_t)(v >> 16), (uint8_t)(v >> 8),  (uint8_t)v
    };
    return sce_forge_writer_write_bytes(w, buf, 8);
}

/* Generated codec emit helper: propagate writer status or continue.
 * Each write call in a generated encode body returns
 * `sce_forge_codec_status_t`; this macro folds the "if non-OK, return"
 * pattern that would otherwise need a 4-line block per write. Lives in
 * the runtime header (not generated per-codec) so its semantics are
 * reviewable in one place. Local `_sce_fw_st` is name-mangled to keep
 * it from clashing with caller-scoped locals. */
#define SCE_FORGE_TRY_WRITE(expr)                                      \
    do {                                                               \
        sce_forge_codec_status_t _sce_fw_st = (expr);                  \
        if (_sce_fw_st != SCE_FORGE_CODEC_OK) return _sce_fw_st;       \
    } while (0)

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
