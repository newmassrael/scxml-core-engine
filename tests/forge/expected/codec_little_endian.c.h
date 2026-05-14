// SCE-MAP: codec_little_endian:3

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_LITTLE_ENDIAN_H
#define SCE_FORGE_CODEC_LITTLE_ENDIAN_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_LITTLE_ENDIAN_MIN_BYTES 4
#define CODEC_LITTLE_ENDIAN_MAX_BYTES 4

typedef struct {
    uint8_t sensor_id;
    uint16_t value;
    uint8_t status;
} codec_little_endian_t;

typedef struct {
    uint8_t bytes[CODEC_LITTLE_ENDIAN_MAX_BYTES];
    size_t  len;
} codec_little_endian_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_little_endian_decode(sce_forge_cursor_t *cursor, codec_little_endian_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_LITTLE_ENDIAN_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->sensor_id = raw[0];
    out->value = raw[1] | ((uint16_t)raw[2] << 8);
    out->status = raw[3];
    if (!sce_forge_cursor_advance(cursor, CODEC_LITTLE_ENDIAN_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_little_endian_encoded_t codec_little_endian_encode(const codec_little_endian_t *self) {
    codec_little_endian_encoded_t r;
    r.len = CODEC_LITTLE_ENDIAN_MIN_BYTES;
    r.bytes[0] = self->sensor_id;
    r.bytes[1] = (uint8_t)(self->value & 0xFF);
    r.bytes[2] = (uint8_t)((self->value >> 8) & 0xFF);
    r.bytes[3] = self->status;
    return r;
}

#endif  /* SCE_FORGE_CODEC_LITTLE_ENDIAN_H */
