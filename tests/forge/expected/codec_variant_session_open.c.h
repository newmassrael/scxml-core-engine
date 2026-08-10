// SCE-MAP: codec_variant_session_open:5 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_VARIANT_SESSION_OPEN_H
#define SCE_FORGE_CODEC_VARIANT_SESSION_OPEN_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_VARIANT_SESSION_OPEN_MIN_BYTES 2
#define CODEC_VARIANT_SESSION_OPEN_MAX_BYTES 2

typedef struct {
    uint16_t version;
} codec_variant_session_open_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_variant_session_open_decode(sce_forge_cursor_t *cursor, codec_variant_session_open_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_VARIANT_SESSION_OPEN_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->version = ((uint16_t)raw[0] << 8) | raw[1];
    if (!sce_forge_cursor_advance(cursor, CODEC_VARIANT_SESSION_OPEN_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_VARIANT_SESSION_OPEN_MAX_BYTES bytes and use
 * `codec_variant_session_open_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_variant_session_open_encode(const codec_variant_session_open_t *self, sce_forge_writer_t *w) {
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((self->version >> 8) & 0xFF)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(self->version & 0xFF)));
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_VARIANT_SESSION_OPEN_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_VARIANT_SESSION_OPEN_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_variant_session_open_encode_to_buf(const codec_variant_session_open_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_variant_session_open_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_VARIANT_SESSION_OPEN_H */
