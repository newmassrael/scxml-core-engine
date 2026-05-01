/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_SUBBYTE_H
#define SCE_FORGE_CODEC_SUBBYTE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

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

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_subbyte_decode(sce_forge_cursor_t *cursor, codec_subbyte_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_SUBBYTE_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->priority = (uint8_t)((raw[0] >> 5) & 0x07);
    out->channel = (uint8_t)((raw[0] >> 2) & 0x07);
    out->direction = (uint8_t)((raw[0] >> 0) & 0x03);
    if (!sce_forge_cursor_advance(cursor, CODEC_SUBBYTE_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_subbyte_encoded_t codec_subbyte_encode(const codec_subbyte_t *self) {
    codec_subbyte_encoded_t r;
    r.len = CODEC_SUBBYTE_MIN_BYTES;
    r.bytes[0] = (uint8_t)(((self->priority & 0x07) << 5) | ((self->channel & 0x07) << 2) | ((self->direction & 0x03) << 0));
    return r;
}

#endif  /* SCE_FORGE_CODEC_SUBBYTE_H */
