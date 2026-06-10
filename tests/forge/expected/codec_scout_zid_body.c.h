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
     * multi-frame consumer" — the TLV chain is that consumer, so
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

/* RFC §5.B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_SCOUT_ZID_BODY_MAX_BYTES bytes and use
 * `codec_scout_zid_body_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_scout_zid_body_encode(const codec_scout_zid_body_t *self, sce_forge_writer_t *w) {
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->zid_len_m1));
    if (self->zid_len <= 16) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, self->zid, self->zid_len));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_SCOUT_ZID_BODY_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_SCOUT_ZID_BODY_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_scout_zid_body_encode_to_buf(const codec_scout_zid_body_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_scout_zid_body_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_SCOUT_ZID_BODY_H */
