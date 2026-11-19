// SCE-MAP: codec_dma_aligned_basic:20 :: _forge_body

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
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */

#define CODEC_DMA_ALIGNED_BASIC_MIN_BYTES 32
#define CODEC_DMA_ALIGNED_BASIC_MAX_BYTES 96
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
        if (_n > 64) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        memcpy(out->aligned_payload, raw + 32, _n);
        out->aligned_payload_len = _n;
    }
    if (!sce_forge_cursor_advance(cursor, _consumed)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

/* The `aligned_payload` element of codec_dma_aligned_basic_t: a byte string of 0 to 64 bytes, viewed where it
 * is held, which outlives the view. A view is made by codec_dma_aligned_basic_aligned_payload_bytes_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const uint8_t *data;
    size_t len;
} codec_dma_aligned_basic_aligned_payload_bytes_t;

/* Make `*out` a view of the `len` bytes at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a byte string outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_dma_aligned_basic_aligned_payload_bytes_make(codec_dma_aligned_basic_aligned_payload_bytes_t *out, const uint8_t *data, size_t len) {
    if (!sce_forge_length_within_bound(len, 64)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_dma_aligned_basic_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_dma_aligned_basic_t is a source by the table
 * codec_dma_aligned_basic_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*msg_id)(const void *ctx);
    uint8_t (*reserved)(const void *ctx);
    codec_dma_aligned_basic_aligned_payload_bytes_t (*aligned_payload)(const void *ctx);
} codec_dma_aligned_basic_source_t;

/* A source of codec_dma_aligned_basic_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_dma_aligned_basic_source_t *table;
    const void *ctx;
} codec_dma_aligned_basic_source_ref_t;

/* A source of codec_dma_aligned_basic_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_dma_aligned_basic_source_ref_optional, codec_dma_aligned_basic_source_ref_t)

/* The `msg_id` of the codec_dma_aligned_basic_t at `ctx`. */
static inline uint8_t codec_dma_aligned_basic_value_msg_id(const void *ctx) {
    return ((const codec_dma_aligned_basic_t *)ctx)->msg_id;
}

/* The `reserved` of the codec_dma_aligned_basic_t at `ctx`. */
static inline uint8_t codec_dma_aligned_basic_value_reserved(const void *ctx) {
    return ((const codec_dma_aligned_basic_t *)ctx)->reserved;
}

/* The `aligned_payload` of the codec_dma_aligned_basic_t at `ctx`. */
static inline codec_dma_aligned_basic_aligned_payload_bytes_t codec_dma_aligned_basic_value_aligned_payload(const void *ctx) {
    const codec_dma_aligned_basic_t *self = (const codec_dma_aligned_basic_t *)ctx;
    codec_dma_aligned_basic_aligned_payload_bytes_t view;
    view.data = self->aligned_payload;
    view.len = self->aligned_payload_len;
    return view;
}

/* The table that presents a value of codec_dma_aligned_basic_t as a source of it, its
 * context a `const codec_dma_aligned_basic_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_dma_aligned_basic_source_t *codec_dma_aligned_basic_value_source(void) {
    static const codec_dma_aligned_basic_source_t table = {
        codec_dma_aligned_basic_value_msg_id,
        codec_dma_aligned_basic_value_reserved,
        codec_dma_aligned_basic_value_aligned_payload,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_dma_aligned_basic_encode_source(const codec_dma_aligned_basic_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_msg_id = src->msg_id(ctx);
    const uint8_t _e_reserved = src->reserved(ctx);
    const codec_dma_aligned_basic_aligned_payload_bytes_t _e_aligned_payload = src->aligned_payload(ctx);
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _e_msg_id));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _e_reserved));
    while (sce_forge_writer_position(w) < 32) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, 0));
    }
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, _e_aligned_payload.data, _e_aligned_payload.len));
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_dma_aligned_basic_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_dma_aligned_basic_check_value(const codec_dma_aligned_basic_t *self) {
    if (!sce_forge_length_within_bound(self->aligned_payload_len, 64)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_DMA_ALIGNED_BASIC_MAX_BYTES bytes and use
 * `codec_dma_aligned_basic_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_dma_aligned_basic_encode(const codec_dma_aligned_basic_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_dma_aligned_basic_check_value(self));
    return codec_dma_aligned_basic_encode_source(codec_dma_aligned_basic_value_source(), self, w);
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
