// SCE-MAP: codec_until_eof_basic:10

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_UNTIL_EOF_BASIC_H
#define SCE_FORGE_CODEC_UNTIL_EOF_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_repeat_elem.h"

#define CODEC_UNTIL_EOF_BASIC_MIN_BYTES 0
#define CODEC_UNTIL_EOF_BASIC_MAX_BYTES 128

typedef struct {
    /* RFC §synth-5-B B2 repeat: fixed array of codec_repeat_elem_t elements (max 64) */
    codec_repeat_elem_t msgs[64];
    size_t  msgs_len;
} codec_until_eof_basic_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_until_eof_basic_decode(sce_forge_cursor_t *cursor, codec_until_eof_basic_t *out) {
    /* RFC §synth-5-B B2 repeat / B3 TLV chain primitives: streaming decode
     * mixes plain fixed-width reads with bounded-iteration loops over
     * imported codec entries. Repeat: bounded by `out-><len_field>`
     * (length-field) or until cursor exhaustion (until-eof); MAX_COUNT
     * overflow → NEED_MORE_BYTES. TLV chain: bounded by `max_depth`
     * with on-overflow check (reject → SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW
     * when residual bytes after cap; truncate → silent). */
    {
        out->msgs_len = 0;
        while (sce_forge_cursor_remaining(cursor) > 0) {
            if (out->msgs_len >= 64) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
            sce_forge_codec_status_t _st = codec_repeat_elem_decode(cursor, &out->msgs[out->msgs_len]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
            out->msgs_len++;
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_UNTIL_EOF_BASIC_MAX_BYTES bytes and use
 * `codec_until_eof_basic_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_until_eof_basic_encode(const codec_until_eof_basic_t *self, sce_forge_writer_t *w) {
    /* RFC §synth-5-B B2 / B3 encode: fixed prefix appends byte-by-byte;
     * list fields walk an in-place writer loop. Author keeps count
     * field (repeat) / `<id>_len` ≤ max_depth (tlv-chain) consistent
     * with the in-struct entry count (trust contract). */
    for (size_t _ri = 0; _ri < self->msgs_len; ++_ri) {
        SCE_FORGE_TRY_WRITE(codec_repeat_elem_encode(&self->msgs[_ri], w));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_UNTIL_EOF_BASIC_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_UNTIL_EOF_BASIC_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_until_eof_basic_encode_to_buf(const codec_until_eof_basic_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_until_eof_basic_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_UNTIL_EOF_BASIC_H */
