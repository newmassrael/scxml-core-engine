// SCE-MAP: codec_peek_arm_b:13

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_PEEK_ARM_B_H
#define SCE_FORGE_CODEC_PEEK_ARM_B_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_PEEK_ARM_B_MIN_BYTES 3
#define CODEC_PEEK_ARM_B_MAX_BYTES 3

typedef struct {
    uint8_t header;
    uint16_t payload;
} codec_peek_arm_b_t;

/* RFC variant-default-uniformity (C11): designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_peek_arm_b_t x = CODEC_PEEK_ARM_B_DEFAULT_INIT;
 * codec_peek_arm_b_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_PEEK_ARM_B_DEFAULT_INIT { \
    .header = 0x01u, \
}

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_peek_arm_b_decode(sce_forge_cursor_t *cursor, codec_peek_arm_b_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_PEEK_ARM_B_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->header = raw[0];
    out->payload = ((uint16_t)raw[1] << 8) | raw[2];
    if (!sce_forge_cursor_advance(cursor, CODEC_PEEK_ARM_B_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

/* RFC §5.B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_PEEK_ARM_B_MAX_BYTES bytes and use
 * `codec_peek_arm_b_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_peek_arm_b_encode(const codec_peek_arm_b_t *self, sce_forge_writer_t *w) {
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->header));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((self->payload >> 8) & 0xFF)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(self->payload & 0xFF)));
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_PEEK_ARM_B_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_PEEK_ARM_B_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_peek_arm_b_encode_to_buf(const codec_peek_arm_b_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_peek_arm_b_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

/* RFC §5.B flags primitive: per-bit-range accessors over
 * the carrier field. Single-bit (width=1) reads as bool; multi-bit
 * (width>=2) reads as the smallest unsigned C11 integer type that fits
 * (uint8_t / uint16_t / uint32_t / uint64_t). Setters mask + shift on
 * the way in so out-of-range callers can't corrupt sibling bits. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline bool codec_peek_arm_b_kind(const codec_peek_arm_b_t *self) {
    return (self->header & 0x01) != 0;
}

static inline void codec_peek_arm_b_set_kind(codec_peek_arm_b_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x01);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_PEEK_ARM_B_H */
