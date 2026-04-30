/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_SUBBYTE_H
#define SCE_FORGE_CODEC_SUBBYTE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#define CODEC_SUBBYTE_MIN_BYTES 1
#define CODEC_SUBBYTE_MAX_BYTES 1

typedef struct {
    uint8_t priority;
    uint8_t channel;
    uint8_t direction;
} codec_subbyte_t;

typedef struct {
    uint8_t bytes[CODEC_SUBBYTE_MAX_BYTES];
    size_t  len;
} codec_subbyte_encoded_t;

static inline bool codec_subbyte_decode(const uint8_t *raw, size_t len, codec_subbyte_t *out) {
    if (len < CODEC_SUBBYTE_MIN_BYTES) return false;
    out->priority = (uint8_t)((raw[0] >> 5) & 0x07);
    out->channel = (uint8_t)((raw[0] >> 2) & 0x07);
    out->direction = (uint8_t)((raw[0] >> 0) & 0x03);
    return true;
}

static inline codec_subbyte_encoded_t codec_subbyte_encode(const codec_subbyte_t *self) {
    codec_subbyte_encoded_t r;
    r.len = CODEC_SUBBYTE_MIN_BYTES;
    r.bytes[0] = (uint8_t)(((self->priority & 0x07) << 5) | ((self->channel & 0x07) << 2) | ((self->direction & 0x03) << 0));
    return r;
}

#endif  /* SCE_FORGE_CODEC_SUBBYTE_H */
