// SCE-MAP: codec_repeat_elem:5

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_REPEAT_ELEM_H
#define SCE_FORGE_CODEC_REPEAT_ELEM_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_REPEAT_ELEM_MIN_BYTES 2
#define CODEC_REPEAT_ELEM_MAX_BYTES 2

typedef struct {
    uint16_t seq;
} codec_repeat_elem_t;

typedef struct {
    uint8_t bytes[CODEC_REPEAT_ELEM_MAX_BYTES];
    size_t  len;
} codec_repeat_elem_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_repeat_elem_decode(sce_forge_cursor_t *cursor, codec_repeat_elem_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_REPEAT_ELEM_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->seq = ((uint16_t)raw[0] << 8) | raw[1];
    if (!sce_forge_cursor_advance(cursor, CODEC_REPEAT_ELEM_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_repeat_elem_encoded_t codec_repeat_elem_encode(const codec_repeat_elem_t *self) {
    codec_repeat_elem_encoded_t r;
    r.len = CODEC_REPEAT_ELEM_MIN_BYTES;
    r.bytes[0] = (uint8_t)((self->seq >> 8) & 0xFF);
    r.bytes[1] = (uint8_t)(self->seq & 0xFF);
    return r;
}

#endif  /* SCE_FORGE_CODEC_REPEAT_ELEM_H */
