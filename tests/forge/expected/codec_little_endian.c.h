/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_LITTLE_ENDIAN_H
#define SCE_FORGE_CODEC_LITTLE_ENDIAN_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

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

static inline bool codec_little_endian_decode(const uint8_t *raw, size_t len, codec_little_endian_t *out) {
    if (len < CODEC_LITTLE_ENDIAN_MIN_BYTES) return false;
    out->sensor_id = raw[0];
    out->value = raw[1] | ((uint16_t)raw[2] << 8);
    out->status = raw[3];
    return true;
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
