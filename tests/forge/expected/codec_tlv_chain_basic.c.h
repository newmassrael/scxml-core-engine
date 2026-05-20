// SCE-MAP: codec_tlv_chain_basic:16

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_TLV_CHAIN_BASIC_H
#define SCE_FORGE_CODEC_TLV_CHAIN_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_tlv_entry.h"

#define CODEC_TLV_CHAIN_BASIC_MIN_BYTES 1
#define CODEC_TLV_CHAIN_BASIC_MAX_BYTES 273

typedef struct {
    uint8_t header_flags;
    /* RFC §5.B B3 tlv-chain: fixed array of codec_tlv_entry_t entries (max-depth 8, on-overflow=reject) */
    codec_tlv_entry_t extensions[8];
    size_t  extensions_len;
} codec_tlv_chain_basic_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_tlv_chain_basic_decode(sce_forge_cursor_t *cursor, codec_tlv_chain_basic_t *out) {
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
        out->header_flags = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        out->extensions_len = 0;
        for (size_t _i = 0; _i < 8; ++_i) {
            if (sce_forge_cursor_remaining(cursor) == 0) break;
            sce_forge_codec_status_t _st = codec_tlv_entry_decode(cursor, &out->extensions[out->extensions_len]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
            out->extensions_len++;
        }
        if (sce_forge_cursor_remaining(cursor) > 0) return SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §5.B B1-α encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_TLV_CHAIN_BASIC_MAX_BYTES bytes and use
 * `codec_tlv_chain_basic_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_tlv_chain_basic_encode(const codec_tlv_chain_basic_t *self, sce_forge_writer_t *w) {
    /* RFC §5.B B2 / B3 encode: fixed prefix appends byte-by-byte;
     * list fields walk an in-place writer loop. Author keeps count
     * field (repeat) / `<id>_len` ≤ max_depth (tlv-chain) consistent
     * with the in-struct entry count (trust contract). */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->header_flags));
    for (size_t _ti = 0; _ti < self->extensions_len; ++_ti) {
        SCE_FORGE_TRY_WRITE(codec_tlv_entry_encode(&self->extensions[_ti], w));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_TLV_CHAIN_BASIC_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_TLV_CHAIN_BASIC_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_tlv_chain_basic_encode_to_buf(const codec_tlv_chain_basic_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_tlv_chain_basic_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_TLV_CHAIN_BASIC_H */
