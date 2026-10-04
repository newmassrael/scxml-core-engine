// SCE-MAP: codec_zenoh_query_value:24 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_QUERY_VALUE_H
#define SCE_FORGE_CODEC_ZENOH_QUERY_VALUE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_value_entry.h"

#define CODEC_ZENOH_QUERY_VALUE_MIN_BYTES 0
#define CODEC_ZENOH_QUERY_VALUE_MAX_BYTES 656

typedef struct {
    /* RFC §synth-5-B B3 tlv-chain: fixed array of codec_zenoh_value_entry_t entries (max-depth 4, on-overflow=reject) */
    codec_zenoh_value_entry_t exts[4];
    size_t  exts_len;
} codec_zenoh_query_value_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_query_value_decode(sce_forge_cursor_t *cursor, codec_zenoh_query_value_t *out) {
    /* Streaming cursor decode (SSOT selection: `needs_streaming`).
     * The positional `raw[byte_off]` path is valid only when every
     * field's absolute offset is fixed at codegen time; this branch
     * handles every codec where it is not — present-if-gated fields
     * (runtime presence; C11 stores plain `T` with `_len = 0` for absent
     * bytes, the carrier flag bit being the truth), VLE / repeat /
     * TLV-chain / embed fields (runtime width), and a fixed field after a
     * variable-length payload (offset depends on the payload length).
     * Each field reads its own bytes and advances past what it consumed.
     * Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
     * dedicated helpers; every other field flows through
     * `present_if_decode_stmt`. */
    {
        out->exts_len = 0;
        uint8_t _prev_exts_after_shm = 0;
        bool _more = false;
        for (size_t _i = 0; _i < 4; ++_i) {
            if (sce_forge_cursor_remaining(cursor) == 0) break;
            sce_forge_codec_status_t _st = codec_zenoh_value_entry_decode(cursor, &out->exts[out->exts_len], _prev_exts_after_shm);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
            size_t _just = out->exts_len;
            out->exts_len++;
            _more = codec_zenoh_value_entry_z(&out->exts[_just]);
            _prev_exts_after_shm = (uint8_t)(((uint64_t)(codec_zenoh_value_entry_ext_id(&out->exts[_just])) == 4ULL) ? 1 : 0);
            if (!_more) break;
        }
        if (_more && sce_forge_cursor_remaining(cursor) == 0) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        if (_more) return SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_QUERY_VALUE_MAX_BYTES bytes and use
 * `codec_zenoh_query_value_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_query_value_encode(const codec_zenoh_query_value_t *self, sce_forge_writer_t *w) {
    /* Streaming cursor encode (SSOT selection: `needs_streaming`).
     * Mirrors the streaming decode: every field appends its own bytes in
     * declaration order through the per-field encode blocks, so a gated
     * field skips its append when the carrier flag bit is clear, and a
     * fixed field after a variable-length payload lands after the payload
     * (the positional path appends variable fields last, placing it ahead
     * on the wire). Per-field `is_repeat` / `is_tlv_chain` / `is_embed`
     * route to their dedicated helpers; everything else uses
     * `present_if_encode_block`. */
    uint8_t _prev_exts_after_shm = 0;
    for (size_t _ti = 0; _ti < self->exts_len; ++_ti) {
        SCE_FORGE_TRY_WRITE(codec_zenoh_value_entry_encode(&self->exts[_ti], w, _prev_exts_after_shm));
        _prev_exts_after_shm = (uint8_t)(((uint64_t)(codec_zenoh_value_entry_ext_id(&self->exts[_ti])) == 4ULL) ? 1 : 0);
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_QUERY_VALUE_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_QUERY_VALUE_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_query_value_encode_to_buf(const codec_zenoh_query_value_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_query_value_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_QUERY_VALUE_H */
