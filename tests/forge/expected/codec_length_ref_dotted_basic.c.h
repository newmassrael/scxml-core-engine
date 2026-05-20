// SCE-MAP: codec_length_ref_dotted_basic:27

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_LENGTH_REF_DOTTED_BASIC_H
#define SCE_FORGE_CODEC_LENGTH_REF_DOTTED_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"

#define CODEC_LENGTH_REF_DOTTED_BASIC_MIN_BYTES 1
#define CODEC_LENGTH_REF_DOTTED_BASIC_MAX_BYTES 16

typedef struct {
    uint8_t carrier;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="15") */
    uint8_t payload[15];
    size_t  payload_len;
} codec_length_ref_dotted_basic_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_length_ref_dotted_basic_decode(sce_forge_cursor_t *cursor, codec_length_ref_dotted_basic_t *out) {
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
    if (_frame_len < CODEC_LENGTH_REF_DOTTED_BASIC_MIN_BYTES) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    const uint8_t *raw = sce_forge_cursor_peek(cursor, _frame_len);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    size_t _consumed = CODEC_LENGTH_REF_DOTTED_BASIC_MIN_BYTES;
    out->carrier = raw[0];
    {
        size_t _n = (size_t)((out->carrier >> 4) & 0xF);
        if (_n > 15 || 1 + _n > _frame_len) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->payload, raw + 1, _n);
        out->payload_len = _n;
        if (1 + _n > _consumed) _consumed = 1 + _n;
    }
    if (!sce_forge_cursor_advance(cursor, _consumed)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

/* RFC §5.B B1-α encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_LENGTH_REF_DOTTED_BASIC_MAX_BYTES bytes and use
 * `codec_length_ref_dotted_basic_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_length_ref_dotted_basic_encode(const codec_length_ref_dotted_basic_t *self, sce_forge_writer_t *w) {
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->carrier));
    if (self->payload_len <= 15) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, self->payload, self->payload_len));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_LENGTH_REF_DOTTED_BASIC_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_LENGTH_REF_DOTTED_BASIC_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_length_ref_dotted_basic_encode_to_buf(const codec_length_ref_dotted_basic_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_length_ref_dotted_basic_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

/* RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
 * the carrier field. Single-bit (width=1) reads as bool; multi-bit
 * (width>=2) reads as the smallest unsigned C11 integer type that fits
 * (uint8_t / uint16_t / uint32_t / uint64_t). Setters mask + shift on
 * the way in so out-of-range callers can't corrupt sibling bits. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline uint8_t codec_length_ref_dotted_basic_hdr(const codec_length_ref_dotted_basic_t *self) {
    return (uint8_t)((self->carrier >> 0) & (uint8_t)0x0F);
}

static inline void codec_length_ref_dotted_basic_set_hdr(codec_length_ref_dotted_basic_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 0);
    self->carrier = (uint8_t)((self->carrier & (uint8_t)~_shifted_mask) | _val);
}


static inline uint8_t codec_length_ref_dotted_basic_payload_len(const codec_length_ref_dotted_basic_t *self) {
    return (uint8_t)((self->carrier >> 4) & (uint8_t)0x0F);
}

static inline void codec_length_ref_dotted_basic_set_payload_len(codec_length_ref_dotted_basic_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 4);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 4);
    self->carrier = (uint8_t)((self->carrier & (uint8_t)~_shifted_mask) | _val);
}


#endif  /* SCE_FORGE_CODEC_LENGTH_REF_DOTTED_BASIC_H */
