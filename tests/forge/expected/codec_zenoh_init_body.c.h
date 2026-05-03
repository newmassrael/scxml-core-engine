/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_INIT_BODY_H
#define SCE_FORGE_CODEC_ZENOH_INIT_BODY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"

#define CODEC_ZENOH_INIT_BODY_MIN_BYTES 6
#define CODEC_ZENOH_INIT_BODY_MAX_BYTES 160

typedef struct {
    uint8_t version;
    uint8_t cbyte;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="16") */
    uint8_t zid[16];
    size_t  zid_len;
    uint8_t sn_res;
    uint16_t batch_size;
    uint64_t cookie_len;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="128") */
    uint8_t cookie[128];
} codec_zenoh_init_body_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_INIT_BODY_MAX_BYTES];
    size_t  len;
} codec_zenoh_init_body_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_init_body_decode(sce_forge_cursor_t *cursor, codec_zenoh_init_body_t *out, uint8_t parent_flags) {
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
    if ((parent_flags & 0x40) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->sn_res = (uint8_t)(raw[0]);
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->sn_res = 0;
    }
    if ((parent_flags & 0x40) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 2);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->batch_size = (uint16_t)(((uint16_t)raw[0] << 8) | raw[1]);
        if (!sce_forge_cursor_advance(cursor, 2)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->batch_size = 0;
    }
    if ((parent_flags & 0x20) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        out->cookie_len = _v;
    } else {
        out->cookie_len = 0;
    }
    if ((parent_flags & 0x20) != 0) {
        size_t _n = (size_t)out->cookie_len;
        if (_n > 128) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->cookie, raw, _n);
        out->cookie_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->cookie_len = 0;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_init_body_encoded_t codec_zenoh_init_body_encode(const codec_zenoh_init_body_t *self, uint8_t parent_flags) {
    /* RFC §5.B B5-γ: see decode — same parameter, same suppress. */
    (void)parent_flags;
    codec_zenoh_init_body_encoded_t r;
    /* RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    r.len = 0;
    r.bytes[r.len++] = self->version;
    r.bytes[r.len++] = self->cbyte;
    for (size_t _bi = 0; _bi < self->zid_len && _bi < (size_t)((int64_t)(size_t)((self->cbyte >> 4) & 0xF) + 1); ++_bi) r.bytes[r.len++] = self->zid[_bi];
    if ((parent_flags & 0x40) != 0) {
        r.bytes[r.len++] = self->sn_res;
    }
    if ((parent_flags & 0x40) != 0) {
        r.bytes[r.len++] = (uint8_t)((self->batch_size >> 8) & 0xFF);
        r.bytes[r.len++] = (uint8_t)(self->batch_size & 0xFF);
    }
    if ((parent_flags & 0x20) != 0) {
    {
        uint64_t _w = (uint64_t)(self->cookie_len);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    }
    if ((parent_flags & 0x20) != 0) {
        for (size_t _bi = 0; _bi < self->cookie_len && _bi < self->cookie_len; ++_bi) r.bytes[r.len++] = self->cookie[_bi];
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
static inline uint8_t codec_zenoh_init_body_whatami(const codec_zenoh_init_body_t *self) {
    return (uint8_t)((self->cbyte >> 0) & (uint8_t)0x03);
}

static inline void codec_zenoh_init_body_set_whatami(codec_zenoh_init_body_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x03 << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x03) << 0);
    self->cbyte = (uint8_t)((self->cbyte & (uint8_t)~_shifted_mask) | _val);
}


static inline uint8_t codec_zenoh_init_body_zid_len_m1(const codec_zenoh_init_body_t *self) {
    return (uint8_t)((self->cbyte >> 4) & (uint8_t)0x0F);
}

static inline void codec_zenoh_init_body_set_zid_len_m1(codec_zenoh_init_body_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 4);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 4);
    self->cbyte = (uint8_t)((self->cbyte & (uint8_t)~_shifted_mask) | _val);
}


#endif  /* SCE_FORGE_CODEC_ZENOH_INIT_BODY_H */
