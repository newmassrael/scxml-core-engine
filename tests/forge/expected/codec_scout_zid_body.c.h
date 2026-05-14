// SCE-MAP: codec_scout_zid_body:35

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_SCOUT_ZID_BODY_H
#define SCE_FORGE_CODEC_SCOUT_ZID_BODY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"

#define CODEC_SCOUT_ZID_BODY_MIN_BYTES 1
#define CODEC_SCOUT_ZID_BODY_MAX_BYTES 17

typedef struct {
    uint8_t zid_len_m1;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="16") */
    uint8_t zid[16];
    size_t  zid_len;
} codec_scout_zid_body_t;

typedef struct {
    uint8_t bytes[CODEC_SCOUT_ZID_BODY_MAX_BYTES];
    size_t  len;
} codec_scout_zid_body_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_scout_zid_body_decode(sce_forge_cursor_t *cursor, codec_scout_zid_body_t *out) {
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
    if (_frame_len < CODEC_SCOUT_ZID_BODY_MIN_BYTES) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    const uint8_t *raw = sce_forge_cursor_peek(cursor, _frame_len);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    size_t _consumed = CODEC_SCOUT_ZID_BODY_MIN_BYTES;
    out->zid_len_m1 = raw[0];
    {
        size_t _n = (size_t)((int64_t)out->zid_len_m1 + 1);
        if (_n > 16 || 1 + _n > _frame_len) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->zid, raw + 1, _n);
        out->zid_len = _n;
        if (1 + _n > _consumed) _consumed = 1 + _n;
    }
    if (!sce_forge_cursor_advance(cursor, _consumed)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_scout_zid_body_encoded_t codec_scout_zid_body_encode(const codec_scout_zid_body_t *self) {
    codec_scout_zid_body_encoded_t r;
    r.len = CODEC_SCOUT_ZID_BODY_MIN_BYTES;
    r.bytes[0] = self->zid_len_m1;
    if (self->zid_len <= 16) {
        memcpy(&r.bytes[1], self->zid, self->zid_len);
        r.len = 1 + self->zid_len;
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_SCOUT_ZID_BODY_H */
