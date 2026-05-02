/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_VLE_ZINT_U64_H
#define SCE_FORGE_CODEC_VLE_ZINT_U64_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_VLE_ZINT_U64_MIN_BYTES 0
#define CODEC_VLE_ZINT_U64_MAX_BYTES 10

typedef struct {
    uint64_t value;
} codec_vle_zint_u64_t;

typedef struct {
    uint8_t bytes[CODEC_VLE_ZINT_U64_MAX_BYTES];
    size_t  len;
} codec_vle_zint_u64_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_vle_zint_u64_decode(sce_forge_cursor_t *cursor, codec_vle_zint_u64_t *out) {
    /* Streaming codec: each field reads from cursor directly (VLE
     * base-128 chain, 1..=ceil(N/7) bytes per field). */
    uint64_t value;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &value);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->value = value;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_vle_zint_u64_encoded_t codec_vle_zint_u64_encode(const codec_vle_zint_u64_t *self) {
    codec_vle_zint_u64_encoded_t r;
    r.len = 0;
    {
        uint64_t _w = (uint64_t)(self->value);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_VLE_ZINT_U64_H */
