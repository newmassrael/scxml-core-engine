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

#include <sce/portability.h>
#include "sce/forge/codec.h"

#define CODEC_DMA_ALIGNED_BASIC_MIN_BYTES 2
#define CODEC_DMA_ALIGNED_BASIC_MAX_BYTES 66
/* RFC §synth-5-B B3 DMA alignment primitive: structural drift detection.
 * Build-time validation already guaranteed `byte_offset % burst_align
 * == 0` and that all preceding fields are Fixed bit-size. These
 * `SCE_STATIC_ASSERT` declarations catch any future hand-edit to the
 * byte_offset that would break the wire-layout invariant. */
SCE_STATIC_ASSERT(32 % 32 == 0,
                  "RFC §synth-5-B B3: codec field 'aligned_payload' offset must be 32-aligned");

typedef struct {
    uint8_t msg_id;
    uint8_t reserved;
    /* variable-length payload (sce:bit-size="tail", sce:max-size="64") */
    uint8_t aligned_payload[64];
    size_t  aligned_payload_len;
} codec_dma_aligned_basic_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_dma_aligned_basic_decode(sce_forge_cursor_t *cursor, codec_dma_aligned_basic_t *out) {
    /* Variable-length codec. RFC §synth-5-B B3 stream-correct shape:
     * a codec without `<sce:field sce:bit-size="tail">` consumes only
     * the bytes it actually decoded (`min_bytes + length_value`)
     * rather than the entire cursor remaining. Codecs WITH a tail
     * field still consume to end (tail's definition forces it). The
     * prior "consume entire cursor" behaviour deferred to "the first
     * multi-frame consumer" — the TLV chain is that consumer, so
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

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_DMA_ALIGNED_BASIC_MAX_BYTES bytes and use
 * `codec_dma_aligned_basic_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_dma_aligned_basic_encode(const codec_dma_aligned_basic_t *self, sce_forge_writer_t *w) {
    /* RFC §synth-5-B B3 DMA padding: zero-fill the gap between the current
     * writer position and any aligned field's authored byte_offset
     * (deterministic zeros on the wire so peers stay byte-compatible
     * regardless of host allocator). */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->msg_id));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->reserved));
    while (sce_forge_writer_position(w) < 32) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, 0));
    }
    if (self->aligned_payload_len <= 64) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, self->aligned_payload, self->aligned_payload_len));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_DMA_ALIGNED_BASIC_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_DMA_ALIGNED_BASIC_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_dma_aligned_basic_encode_to_buf(const codec_dma_aligned_basic_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_dma_aligned_basic_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_DMA_ALIGNED_BASIC_H */
