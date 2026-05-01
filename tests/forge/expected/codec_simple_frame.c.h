/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_SIMPLE_FRAME_H
#define SCE_FORGE_CODEC_SIMPLE_FRAME_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_SIMPLE_FRAME_MIN_BYTES 4
#define CODEC_SIMPLE_FRAME_MAX_BYTES 4

typedef struct {
    uint8_t msg_id;
    uint8_t length;
    uint16_t payload;
} codec_simple_frame_t;

typedef struct {
    uint8_t bytes[CODEC_SIMPLE_FRAME_MAX_BYTES];
    size_t  len;
} codec_simple_frame_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). */
static inline sce_forge_codec_status_t codec_simple_frame_decode(sce_forge_cursor_t *cursor, codec_simple_frame_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_SIMPLE_FRAME_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->msg_id = raw[0];
    out->length = raw[1];
    out->payload = ((uint16_t)raw[2] << 8) | raw[3];
    if (!sce_forge_cursor_advance(cursor, CODEC_SIMPLE_FRAME_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_simple_frame_encoded_t codec_simple_frame_encode(const codec_simple_frame_t *self) {
    codec_simple_frame_encoded_t r;
    r.len = CODEC_SIMPLE_FRAME_MIN_BYTES;
    r.bytes[0] = self->msg_id;
    r.bytes[1] = self->length;
    r.bytes[2] = (uint8_t)((self->payload >> 8) & 0xFF);
    r.bytes[3] = (uint8_t)(self->payload & 0xFF);
    return r;
}

#endif  /* SCE_FORGE_CODEC_SIMPLE_FRAME_H */
