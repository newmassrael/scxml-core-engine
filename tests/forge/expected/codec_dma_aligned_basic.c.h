// SCE-MAP: codec_dma_aligned_basic:20

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_DMA_ALIGNED_BASIC_H
#define SCE_FORGE_CODEC_DMA_ALIGNED_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"

#define CODEC_DMA_ALIGNED_BASIC_MIN_BYTES 2
#define CODEC_DMA_ALIGNED_BASIC_MAX_BYTES 66
/* RFC §5.B B3 DMA alignment primitive: structural drift detection.
 * Build-time validation already guaranteed `byte_offset % burst_align
 * == 0` and that all preceding fields are Fixed bit-size. These
 * `_Static_assert` declarations catch any future hand-edit to the
 * byte_offset that would break the wire-layout invariant. */
_Static_assert(32 % 32 == 0,
               "RFC §5.B B3: codec field 'aligned_payload' offset must be 32-aligned");

typedef struct {
    uint8_t msg_id;
    uint8_t reserved;
    /* variable-length payload (sce:bit-size="tail", sce:max-size="64") */
    uint8_t aligned_payload[64];
    size_t  aligned_payload_len;
} codec_dma_aligned_basic_t;

typedef struct {
    uint8_t bytes[CODEC_DMA_ALIGNED_BASIC_MAX_BYTES];
    size_t  len;
} codec_dma_aligned_basic_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_dma_aligned_basic_decode(sce_forge_cursor_t *cursor, codec_dma_aligned_basic_t *out) {
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
    if (_frame_len < CODEC_DMA_ALIGNED_BASIC_MIN_BYTES) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    const uint8_t *raw = sce_forge_cursor_peek(cursor, _frame_len);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    size_t len = _frame_len;  /* alias for tail decode_expr */
    (void)len;
    size_t _consumed = _frame_len;
    out->msg_id = raw[0];
    out->reserved = raw[1];
    {
        size_t _n = _frame_len - 32;
        if (_n > 64) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->aligned_payload, raw + 32, _n);
        out->aligned_payload_len = _n;
    }
    if (!sce_forge_cursor_advance(cursor, _consumed)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_dma_aligned_basic_encoded_t codec_dma_aligned_basic_encode(const codec_dma_aligned_basic_t *self) {
    codec_dma_aligned_basic_encoded_t r;
    /* RFC §5.B B3 DMA padding: zero-fill the encoded buffer so any
     * gap between fixed-prefix fields and aligned variable fields
     * lands as deterministic zeros on the wire (peers MUST see the
     * same byte sequence regardless of host allocator). The
     * positional `r.bytes[<idx>] = ...` writes below overwrite the
     * fixed-field slots; the gap bytes between MIN_BYTES and the
     * aligned field's byte offset stay zero. */
    memset(r.bytes, 0, sizeof(r.bytes));
    r.len = CODEC_DMA_ALIGNED_BASIC_MIN_BYTES;
    r.bytes[0] = self->msg_id;
    r.bytes[1] = self->reserved;
    if (self->aligned_payload_len <= 64) {
        memcpy(&r.bytes[32], self->aligned_payload, self->aligned_payload_len);
        r.len = 32 + self->aligned_payload_len;
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_DMA_ALIGNED_BASIC_H */
