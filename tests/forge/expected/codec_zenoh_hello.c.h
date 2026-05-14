// SCE-MAP: codec_zenoh_hello:41

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_HELLO_H
#define SCE_FORGE_CODEC_ZENOH_HELLO_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_locator.h"

#define CODEC_ZENOH_HELLO_MIN_BYTES 2
#define CODEC_ZENOH_HELLO_MAX_BYTES 8860

typedef struct {
    uint8_t version;
    uint8_t cbyte;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="16") */
    uint8_t zid[16];
    size_t  zid_len;
    uint64_t num_locators;
    /* RFC §5.B B2 repeat: fixed array of codec_zenoh_locator_t elements (max 64) */
    codec_zenoh_locator_t locators[64];
    size_t  locators_len;
} codec_zenoh_hello_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_HELLO_MAX_BYTES];
    size_t  len;
} codec_zenoh_hello_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_hello_decode(sce_forge_cursor_t *cursor, codec_zenoh_hello_t *out, uint8_t parent_flags) {
    /* RFC §5.B B5-γ: `parent_flags` is the parent codec's flags
     * carrier value, threaded by the variant arm dispatcher. Body
     * fields gated via `parent.<flag>` predicates read from this
     * parameter; defensive `(void)parent_flags` suppresses the
     * `-Wunused-parameter` warning when no gated field happens to
     * consume it (mirrors the Rust `let _ = parent_flags;` and Cpp
     * `(void)parent_flags;` defensive guards). */
    (void)parent_flags;
    /* RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
     * advances the cursor per field. C11 has no nullable wrapper so
     * the gated field's storage stays as plain `T` (with `_len = 0`
     * for absent bytes payloads); the carrier's flag bit is the
     * source of truth for presence. B2-β extends gating to Tail /
     * LengthRef / Vle bit-sizes via dispatch inside the helper.
     * Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified streaming path. */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->version = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->cbyte = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        size_t _n = (size_t)((int64_t)(size_t)((out->cbyte >> 4) & 0xF) + 1);
        if (_n > 16) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->zid, raw, _n);
        out->zid_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((parent_flags & 0x20) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        out->num_locators = _v;
    } else {
        out->num_locators = 0;
    }
    if ((parent_flags & 0x20) != 0) {
        size_t _n = (size_t)out->num_locators;
        if (_n > 64) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        for (size_t _i = 0; _i < _n; ++_i) {
            sce_forge_codec_status_t _st = codec_zenoh_locator_decode(cursor, &out->locators[_i]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
        }
        out->locators_len = _n;
    } else {
        out->locators_len = 0;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_hello_encoded_t codec_zenoh_hello_encode(const codec_zenoh_hello_t *self, uint8_t parent_flags) {
    /* RFC §5.B B5-γ: see decode — same parameter, same suppress. */
    (void)parent_flags;
    codec_zenoh_hello_encoded_t r;
    /* RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    r.len = 0;
    r.bytes[r.len++] = self->version;
    r.bytes[r.len++] = self->cbyte;
    for (size_t _bi = 0; _bi < self->zid_len && _bi < (size_t)((int64_t)(size_t)((self->cbyte >> 4) & 0xF) + 1); ++_bi) r.bytes[r.len++] = self->zid[_bi];
    if ((parent_flags & 0x20) != 0) {
    {
        uint64_t _w = (uint64_t)(self->num_locators);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    }
    if ((parent_flags & 0x20) != 0) {
        for (size_t _ri = 0; _ri < self->locators_len; ++_ri) {
            codec_zenoh_locator_encoded_t _sub = codec_zenoh_locator_encode(&self->locators[_ri]);
            if (r.len + _sub.len <= sizeof(r.bytes)) {
                for (size_t _rj = 0; _rj < _sub.len; ++_rj) r.bytes[r.len + _rj] = _sub.bytes[_rj];
                r.len += _sub.len;
            }
        }
    }
    return r;
}

/* RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
 * the carrier field. Single-bit (width=1) reads as bool; multi-bit
 * (width>=2) reads as the smallest unsigned C11 integer type that fits
 * (uint8_t / uint16_t / uint32_t / uint64_t). Setters mask + shift on
 * the way in so out-of-range callers can't corrupt sibling bits. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline uint8_t codec_zenoh_hello_whatami(const codec_zenoh_hello_t *self) {
    return (uint8_t)((self->cbyte >> 0) & (uint8_t)0x03);
}

static inline void codec_zenoh_hello_set_whatami(codec_zenoh_hello_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x03 << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x03) << 0);
    self->cbyte = (uint8_t)((self->cbyte & (uint8_t)~_shifted_mask) | _val);
}


static inline uint8_t codec_zenoh_hello_zid_len_m1(const codec_zenoh_hello_t *self) {
    return (uint8_t)((self->cbyte >> 4) & (uint8_t)0x0F);
}

static inline void codec_zenoh_hello_set_zid_len_m1(codec_zenoh_hello_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 4);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 4);
    self->cbyte = (uint8_t)((self->cbyte & (uint8_t)~_shifted_mask) | _val);
}


#endif  /* SCE_FORGE_CODEC_ZENOH_HELLO_H */
