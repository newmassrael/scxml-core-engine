// SCE-MAP: codec_repeat_basic:11

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_REPEAT_BASIC_H
#define SCE_FORGE_CODEC_REPEAT_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_repeat_elem.h"

#define CODEC_REPEAT_BASIC_MIN_BYTES 1
#define CODEC_REPEAT_BASIC_MAX_BYTES 65

typedef struct {
    uint8_t num_frags;
    /* RFC §5.B B2 repeat: fixed array of codec_repeat_elem_t elements (max 32) */
    codec_repeat_elem_t frags[32];
    size_t  frags_len;
} codec_repeat_basic_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_repeat_basic_decode(sce_forge_cursor_t *cursor, codec_repeat_basic_t *out) {
    /* RFC §5.B B2 repeat / B3 TLV chain primitives: streaming decode
     * mixes plain fixed-width reads with bounded-iteration loops over
     * imported codec entries. Repeat: bounded by `out-><len_field>`
     * (length-field) or until cursor exhaustion (until-eof); MAX_COUNT
     * overflow → NEED_MORE_BYTES. TLV chain: bounded by `max_depth`
     * with on-overflow check (reject → SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW
     * when residual bytes after cap; truncate → silent). */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->num_frags = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        size_t _n = (size_t)out->num_frags;
        if (_n > 32) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        for (size_t _i = 0; _i < _n; ++_i) {
            sce_forge_codec_status_t _st = codec_repeat_elem_decode(cursor, &out->frags[_i]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
        }
        out->frags_len = _n;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §5.B B1-α encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_REPEAT_BASIC_MAX_BYTES bytes and use
 * `codec_repeat_basic_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_repeat_basic_encode(const codec_repeat_basic_t *self, sce_forge_writer_t *w) {
    /* RFC §5.B B2 / B3 encode: fixed prefix appends byte-by-byte;
     * list fields walk an in-place writer loop. Author keeps count
     * field (repeat) / `<id>_len` ≤ max_depth (tlv-chain) consistent
     * with the in-struct entry count (trust contract). */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->num_frags));
    for (size_t _ri = 0; _ri < self->frags_len; ++_ri) {
        SCE_FORGE_TRY_WRITE(codec_repeat_elem_encode(&self->frags[_ri], w));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_REPEAT_BASIC_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_REPEAT_BASIC_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_repeat_basic_encode_to_buf(const codec_repeat_basic_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_repeat_basic_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_REPEAT_BASIC_H */
