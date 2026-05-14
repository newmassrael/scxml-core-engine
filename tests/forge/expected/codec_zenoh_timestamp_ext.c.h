// SCE-MAP: codec_zenoh_timestamp_ext:48

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_TIMESTAMP_EXT_H
#define SCE_FORGE_CODEC_ZENOH_TIMESTAMP_EXT_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_timestamp.h"

#define CODEC_ZENOH_TIMESTAMP_EXT_MIN_BYTES 0
#define CODEC_ZENOH_TIMESTAMP_EXT_MAX_BYTES 266

typedef struct {
    uint64_t ext_size;
    /* RFC §5.B Y0c embed: nested codec_zenoh_timestamp_t struct (no length prefix on the wire) */
    codec_zenoh_timestamp_t ts;
} codec_zenoh_timestamp_ext_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_TIMESTAMP_EXT_MAX_BYTES];
    size_t  len;
} codec_zenoh_timestamp_ext_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_timestamp_ext_decode(sce_forge_cursor_t *cursor, codec_zenoh_timestamp_ext_t *out) {
    /* Streaming codec: each field reads from cursor directly (VLE
     * base-128 chain, 1..=ceil(N/7) bytes per field). RFC §5.B B4:
     * per-field bit-size dispatch routes Fixed / LengthRef siblings
     * of VLE fields through `present_if_decode_stmt` (predicate=None
     * arms — for VLE the helper emits the local-decl + `out->` assign
     * fused; for Fixed / LengthRef it writes directly to `out->`).
     * Pure-VLE codecs stay byte-stable. */
    uint64_t ext_size;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &ext_size);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->ext_size = ext_size;
    {
        size_t _len = (size_t)(out->ext_size);
        const uint8_t *_raw = sce_forge_cursor_peek(cursor, _len);
        if (_raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        sce_forge_cursor_t _inner = sce_forge_cursor_init(_raw, _len);
        sce_forge_codec_status_t _st = codec_zenoh_timestamp_decode(&_inner, &out->ts);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
        if (!sce_forge_cursor_advance(cursor, _len)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_timestamp_ext_encoded_t codec_zenoh_timestamp_ext_encode(const codec_zenoh_timestamp_ext_t *self) {
    codec_zenoh_timestamp_ext_encoded_t r;
    /* RFC §5.B B4: per-field bit-size dispatch routes Fixed /
     * LengthRef / Tail siblings of VLE fields through
     * `present_if_encode_block` (predicate=None arms). Pure-VLE
     * codecs stay byte-stable. */
    r.len = 0;
    {
        uint64_t _w = (uint64_t)(self->ext_size);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    {
        codec_zenoh_timestamp_encoded_t _sub = codec_zenoh_timestamp_encode(&self->ts);
        if (r.len + _sub.len <= sizeof(r.bytes)) {
            for (size_t _ej = 0; _ej < _sub.len; ++_ej) r.bytes[r.len + _ej] = _sub.bytes[_ej];
            r.len += _sub.len;
        }
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_TIMESTAMP_EXT_H */
