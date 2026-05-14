// SCE-MAP: codec_tlv_entry:10

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_TLV_ENTRY_H
#define SCE_FORGE_CODEC_TLV_ENTRY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"

#define CODEC_TLV_ENTRY_MIN_BYTES 2
#define CODEC_TLV_ENTRY_MAX_BYTES 34

typedef struct {
    uint8_t entry_type;
    uint8_t entry_len;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="32") */
    uint8_t entry_body[32];
    size_t  entry_body_len;
} codec_tlv_entry_t;

typedef struct {
    uint8_t bytes[CODEC_TLV_ENTRY_MAX_BYTES];
    size_t  len;
} codec_tlv_entry_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_tlv_entry_decode(sce_forge_cursor_t *cursor, codec_tlv_entry_t *out) {
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
    if (_frame_len < CODEC_TLV_ENTRY_MIN_BYTES) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    const uint8_t *raw = sce_forge_cursor_peek(cursor, _frame_len);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    size_t _consumed = CODEC_TLV_ENTRY_MIN_BYTES;
    out->entry_type = raw[0];
    out->entry_len = raw[1];
    {
        size_t _n = (size_t)out->entry_len;
        if (_n > 32 || 2 + _n > _frame_len) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->entry_body, raw + 2, _n);
        out->entry_body_len = _n;
        if (2 + _n > _consumed) _consumed = 2 + _n;
    }
    if (!sce_forge_cursor_advance(cursor, _consumed)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_tlv_entry_encoded_t codec_tlv_entry_encode(const codec_tlv_entry_t *self) {
    codec_tlv_entry_encoded_t r;
    r.len = CODEC_TLV_ENTRY_MIN_BYTES;
    r.bytes[0] = self->entry_type;
    r.bytes[1] = self->entry_len;
    if (self->entry_body_len <= 32) {
        memcpy(&r.bytes[2], self->entry_body, self->entry_body_len);
        r.len = 2 + self->entry_body_len;
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_TLV_ENTRY_H */
