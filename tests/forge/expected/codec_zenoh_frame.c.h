/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_FRAME_H
#define SCE_FORGE_CODEC_ZENOH_FRAME_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"

#define CODEC_ZENOH_FRAME_MIN_BYTES 0
#define CODEC_ZENOH_FRAME_MAX_BYTES 65546

typedef struct {
    uint64_t sn;
    /* variable-length payload (sce:bit-size="tail", sce:max-size="65536") */
    uint8_t payload[65536];
    size_t  payload_len;
} codec_zenoh_frame_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_FRAME_MAX_BYTES];
    size_t  len;
} codec_zenoh_frame_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_frame_decode(sce_forge_cursor_t *cursor, codec_zenoh_frame_t *out) {
    /* Streaming codec: each field reads from cursor directly (VLE
     * base-128 chain, 1..=ceil(N/7) bytes per field). RFC §5.B B4:
     * per-field bit-size dispatch routes Fixed / LengthRef siblings
     * of VLE fields through `present_if_decode_stmt` (predicate=None
     * arms — for VLE the helper emits the local-decl + `out->` assign
     * fused; for Fixed / LengthRef it writes directly to `out->`).
     * Pure-VLE codecs stay byte-stable. */
    uint64_t sn;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &sn);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->sn = sn;
    {
        size_t _n = sce_forge_cursor_remaining(cursor);
        if (_n > 65536) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->payload, raw, _n);
        out->payload_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_frame_encoded_t codec_zenoh_frame_encode(const codec_zenoh_frame_t *self) {
    codec_zenoh_frame_encoded_t r;
    /* RFC §5.B B4: per-field bit-size dispatch routes Fixed /
     * LengthRef / Tail siblings of VLE fields through
     * `present_if_encode_block` (predicate=None arms). Pure-VLE
     * codecs stay byte-stable. */
    r.len = 0;
    {
        uint64_t _w = (uint64_t)(self->sn);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    for (size_t _bi = 0; _bi < self->payload_len; ++_bi) r.bytes[r.len++] = self->payload[_bi];
    return r;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_FRAME_H */
