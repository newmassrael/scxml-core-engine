/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_PUT_H
#define SCE_FORGE_CODEC_ZENOH_PUT_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_ZENOH_PUT_MIN_BYTES 1
#define CODEC_ZENOH_PUT_MAX_BYTES 1

typedef struct {
    uint8_t payload;
} codec_zenoh_put_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_PUT_MAX_BYTES];
    size_t  len;
} codec_zenoh_put_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_put_decode(sce_forge_cursor_t *cursor, codec_zenoh_put_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_ZENOH_PUT_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->payload = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_ZENOH_PUT_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_put_encoded_t codec_zenoh_put_encode(const codec_zenoh_put_t *self) {
    codec_zenoh_put_encoded_t r;
    r.len = CODEC_ZENOH_PUT_MIN_BYTES;
    r.bytes[0] = self->payload;
    return r;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_PUT_H */
