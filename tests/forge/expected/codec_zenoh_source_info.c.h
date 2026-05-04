/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_H
#define SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"

#define CODEC_ZENOH_SOURCE_INFO_MIN_BYTES 1
#define CODEC_ZENOH_SOURCE_INFO_MAX_BYTES 27

typedef struct {
    uint8_t header;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="16") */
    uint8_t zid[16];
    size_t  zid_len;
    uint32_t eid;
    uint32_t sn;
} codec_zenoh_source_info_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_SOURCE_INFO_MAX_BYTES];
    size_t  len;
} codec_zenoh_source_info_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_source_info_decode(sce_forge_cursor_t *cursor, codec_zenoh_source_info_t *out) {
    /* Streaming codec: each field reads from cursor directly (VLE
     * base-128 chain, 1..=ceil(N/7) bytes per field). RFC §5.B B4:
     * per-field bit-size dispatch routes Fixed / LengthRef siblings
     * of VLE fields through `present_if_decode_stmt` (predicate=None
     * arms — for VLE the helper emits the local-decl + `out->` assign
     * fused; for Fixed / LengthRef it writes directly to `out->`).
     * Pure-VLE codecs stay byte-stable. */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        size_t _n = (size_t)((int64_t)(size_t)((out->header >> 4) & 0xF) + 1);
        if (_n > 16) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->zid, raw, _n);
        out->zid_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint32_t eid;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u32(cursor, &eid);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->eid = eid;
    uint32_t sn;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u32(cursor, &sn);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->sn = sn;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_source_info_encoded_t codec_zenoh_source_info_encode(const codec_zenoh_source_info_t *self) {
    codec_zenoh_source_info_encoded_t r;
    /* RFC §5.B B4: per-field bit-size dispatch routes Fixed /
     * LengthRef / Tail siblings of VLE fields through
     * `present_if_encode_block` (predicate=None arms). Pure-VLE
     * codecs stay byte-stable. */
    r.len = 0;
    r.bytes[r.len++] = self->header;
    for (size_t _bi = 0; _bi < self->zid_len && _bi < (size_t)((int64_t)(size_t)((self->header >> 4) & 0xF) + 1); ++_bi) r.bytes[r.len++] = self->zid[_bi];
    {
        uint64_t _w = (uint64_t)(self->eid);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    {
        uint64_t _w = (uint64_t)(self->sn);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
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
static inline uint8_t codec_zenoh_source_info_zidlen_m1(const codec_zenoh_source_info_t *self) {
    return (uint8_t)((self->header >> 4) & (uint8_t)0x0F);
}

static inline void codec_zenoh_source_info_set_zidlen_m1(codec_zenoh_source_info_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 4);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 4);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


#endif  /* SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_H */
