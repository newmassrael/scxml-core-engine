// SCE-MAP: codec_zenoh_join:41

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_JOIN_H
#define SCE_FORGE_CODEC_ZENOH_JOIN_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"

#define CODEC_ZENOH_JOIN_MIN_BYTES 6
#define CODEC_ZENOH_JOIN_MAX_BYTES 52

typedef struct {
    uint8_t version;
    uint8_t cbyte;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="16") */
    uint8_t zid[16];
    size_t  zid_len;
    uint8_t sn_res;
    uint16_t batch_size;
    uint64_t lease;
    uint64_t next_sn_reliable;
    uint64_t next_sn_best_effort;
} codec_zenoh_join_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_JOIN_MAX_BYTES];
    size_t  len;
} codec_zenoh_join_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_join_decode(sce_forge_cursor_t *cursor, codec_zenoh_join_t *out, uint8_t s) {
    /* RFC Axis-1 inversion: defensive (void) suppress per declared
     * `<sce:flag-input>` so codecs that haven't consumed an input via
     * `present-if` yet compile cleanly under -Wunused-parameter. */
    (void)s;
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
    if ((s & 0x01) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->sn_res = (uint8_t)(raw[0]);
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->sn_res = 0;
    }
    if ((s & 0x01) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 2);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->batch_size = (uint16_t)(raw[0] | ((uint16_t)raw[1] << 8));
        if (!sce_forge_cursor_advance(cursor, 2)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->batch_size = 0;
    }
    uint64_t lease;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &lease);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->lease = lease;
    uint64_t next_sn_reliable;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &next_sn_reliable);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->next_sn_reliable = next_sn_reliable;
    uint64_t next_sn_best_effort;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &next_sn_best_effort);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->next_sn_best_effort = next_sn_best_effort;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_join_encoded_t codec_zenoh_join_encode(const codec_zenoh_join_t *self, uint8_t s) {
    /* RFC Axis-1 inversion: see decode — same suppress per input. */
    (void)s;
    codec_zenoh_join_encoded_t r;
    /* RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    r.len = 0;
    r.bytes[r.len++] = self->version;
    r.bytes[r.len++] = self->cbyte;
    for (size_t _bi = 0; _bi < self->zid_len && _bi < (size_t)((int64_t)(size_t)((self->cbyte >> 4) & 0xF) + 1); ++_bi) r.bytes[r.len++] = self->zid[_bi];
    if ((s & 0x01) != 0) {
        r.bytes[r.len++] = self->sn_res;
    }
    if ((s & 0x01) != 0) {
        r.bytes[r.len++] = (uint8_t)(self->batch_size & 0xFF);
        r.bytes[r.len++] = (uint8_t)((self->batch_size >> 8) & 0xFF);
    }
    {
        uint64_t _w = (uint64_t)(self->lease);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    {
        uint64_t _w = (uint64_t)(self->next_sn_reliable);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    {
        uint64_t _w = (uint64_t)(self->next_sn_best_effort);
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
static inline uint8_t codec_zenoh_join_whatami(const codec_zenoh_join_t *self) {
    return (uint8_t)((self->cbyte >> 0) & (uint8_t)0x03);
}

static inline void codec_zenoh_join_set_whatami(codec_zenoh_join_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x03 << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x03) << 0);
    self->cbyte = (uint8_t)((self->cbyte & (uint8_t)~_shifted_mask) | _val);
}


static inline uint8_t codec_zenoh_join_zid_len_m1(const codec_zenoh_join_t *self) {
    return (uint8_t)((self->cbyte >> 4) & (uint8_t)0x0F);
}

static inline void codec_zenoh_join_set_zid_len_m1(codec_zenoh_join_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 4);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 4);
    self->cbyte = (uint8_t)((self->cbyte & (uint8_t)~_shifted_mask) | _val);
}


#endif  /* SCE_FORGE_CODEC_ZENOH_JOIN_H */
