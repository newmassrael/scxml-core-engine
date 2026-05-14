// SCE-MAP: codec_tail:3

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_TAIL_H
#define SCE_FORGE_CODEC_TAIL_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"

#define CODEC_TAIL_MIN_BYTES 2
#define CODEC_TAIL_MAX_BYTES 34

typedef struct {
    uint8_t msg_id;
    uint8_t status;
    /* variable-length payload (sce:bit-size="tail", sce:max-size="32") */
    uint8_t payload[32];
    size_t  payload_len;
} codec_tail_t;

typedef struct {
    uint8_t bytes[CODEC_TAIL_MAX_BYTES];
    size_t  len;
} codec_tail_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_tail_decode(sce_forge_cursor_t *cursor, codec_tail_t *out) {
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
    if (_frame_len < CODEC_TAIL_MIN_BYTES) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    const uint8_t *raw = sce_forge_cursor_peek(cursor, _frame_len);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    size_t len = _frame_len;  /* alias for tail decode_expr */
    (void)len;
    size_t _consumed = _frame_len;
    out->msg_id = raw[0];
    out->status = raw[1];
    {
        size_t _n = _frame_len - 2;
        if (_n > 32) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->payload, raw + 2, _n);
        out->payload_len = _n;
    }
    if (!sce_forge_cursor_advance(cursor, _consumed)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_tail_encoded_t codec_tail_encode(const codec_tail_t *self) {
    codec_tail_encoded_t r;
    r.len = CODEC_TAIL_MIN_BYTES;
    r.bytes[0] = self->msg_id;
    r.bytes[1] = self->status;
    if (self->payload_len <= 32) {
        memcpy(&r.bytes[2], self->payload, self->payload_len);
        r.len = 2 + self->payload_len;
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_TAIL_H */
