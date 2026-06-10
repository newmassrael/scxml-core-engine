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

/* RFC §5.B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_REPEAT_ELEM_MAX_BYTES bytes and use
 * `codec_repeat_elem_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_repeat_elem_encode(const codec_repeat_elem_t *self, sce_forge_writer_t *w) {
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((self->seq >> 8) & 0xFF)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(self->seq & 0xFF)));
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_REPEAT_ELEM_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_REPEAT_ELEM_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_repeat_elem_encode_to_buf(const codec_repeat_elem_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_repeat_elem_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_REPEAT_ELEM_H */
