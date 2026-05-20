// SCE-MAP: codec_length_ref_uint32_le:13

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_LENGTH_REF_UINT32_LE_H
#define SCE_FORGE_CODEC_LENGTH_REF_UINT32_LE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"

#define CODEC_LENGTH_REF_UINT32_LE_MIN_BYTES 4
#define CODEC_LENGTH_REF_UINT32_LE_MAX_BYTES 1028

typedef struct {
    uint32_t payload_len;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="1024") */
    uint8_t payload[1024];
} codec_length_ref_uint32_le_t;

typedef struct {
    uint8_t bytes[CODEC_LENGTH_REF_UINT32_LE_MAX_BYTES];
    size_t  len;
} codec_length_ref_uint32_le_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_length_ref_uint32_le_decode(sce_forge_cursor_t *cursor, codec_length_ref_uint32_le_t *out) {
    /* Variable-length codec. RFC §5.B B3 stream-correct shape:
     * a codec without `<sce:field sce:bit-size="tail">` consumes only
     * the bytes it actually decoded (`min_bytes + length_value`)
     * rather than the entire cursor remaining. Codecs WITH a tail
     * field still consume to end (tail's definition forces it). The
     * prior "consume entire cursor" behaviour deferred to "the first
     * multi-frame consumer" — TLV chain (B3-α) is that consumer, so
     * length-ref entry codecs now decode-iterably from a shared
     * cursor without each entry eating the next entry's bytes. */
    size_t _frame_len = sce_forge_cursor_remaining(cursor);
    if (_frame_len < CODEC_LENGTH_REF_UINT32_LE_MIN_BYTES) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    const uint8_t *raw = sce_forge_cursor_peek(cursor, _frame_len);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    size_t _consumed = CODEC_LENGTH_REF_UINT32_LE_MIN_BYTES;
    out->payload_len = raw[0] | ((uint32_t)raw[1] << 8) | ((uint32_t)raw[2] << 16) | ((uint32_t)raw[3] << 24);
    {
        size_t _n = (size_t)out->payload_len;
        if (_n > 1024 || 4 + _n > _frame_len) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->payload, raw + 4, _n);
        out->payload_len = _n;
        if (4 + _n > _consumed) _consumed = 4 + _n;
    }
    if (!sce_forge_cursor_advance(cursor, _consumed)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_length_ref_uint32_le_encoded_t codec_length_ref_uint32_le_encode(const codec_length_ref_uint32_le_t *self) {
    codec_length_ref_uint32_le_encoded_t r;
    r.len = CODEC_LENGTH_REF_UINT32_LE_MIN_BYTES;
    r.bytes[0] = (uint8_t)(self->payload_len & 0xFF);
    r.bytes[1] = (uint8_t)((self->payload_len >> 8) & 0xFF);
    r.bytes[2] = (uint8_t)((self->payload_len >> 16) & 0xFF);
    r.bytes[3] = (uint8_t)((self->payload_len >> 24) & 0xFF);
    if (self->payload_len <= 1024) {
        memcpy(&r.bytes[4], self->payload, self->payload_len);
        r.len = 4 + self->payload_len;
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_LENGTH_REF_UINT32_LE_H */
