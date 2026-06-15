// SCE-MAP: codec_zenoh_undecl_subscriber:46

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_UNDECL_SUBSCRIBER_H
#define SCE_FORGE_CODEC_ZENOH_UNDECL_SUBSCRIBER_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_decl_ext_keyexpr.h"

#define CODEC_ZENOH_UNDECL_SUBSCRIBER_MIN_BYTES 0
#define CODEC_ZENOH_UNDECL_SUBSCRIBER_MAX_BYTES 261

typedef struct {
    uint32_t id;
    /* RFC §synth-5-B embed: nested codec_zenoh_decl_ext_keyexpr_t struct (no length prefix on the wire) */
    codec_zenoh_decl_ext_keyexpr_t ext_keyexpr;
} codec_zenoh_undecl_subscriber_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_undecl_subscriber_decode(sce_forge_cursor_t *cursor, codec_zenoh_undecl_subscriber_t *out, uint8_t z) {
    /* Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
     * `<sce:flag-input>` so codecs that haven't consumed an input via
     * `present-if` yet compile cleanly under -Wunused-parameter. */
    (void)z;
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
    uint32_t id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u32(cursor, &id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->id = id;
    if ((z & 0x01) != 0) {
        sce_forge_codec_status_t _st = codec_zenoh_decl_ext_keyexpr_decode(cursor, &out->ext_keyexpr);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_UNDECL_SUBSCRIBER_MAX_BYTES bytes and use
 * `codec_zenoh_undecl_subscriber_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_undecl_subscriber_encode(const codec_zenoh_undecl_subscriber_t *self, sce_forge_writer_t *w, uint8_t z) {
    /* Declared-but-unconsumed flag inputs: see decode — same suppress per input. */
    (void)z;
    /* Streaming cursor encode (SSOT selection: `needs_streaming`).
     * Mirrors the streaming decode: every field appends its own bytes in
     * declaration order through the per-field encode blocks, so a gated
     * field skips its append when the carrier flag bit is clear, and a
     * fixed field after a variable-length payload lands after the payload
     * (the positional path appends variable fields last, placing it ahead
     * on the wire). Per-field `is_repeat` / `is_tlv_chain` / `is_embed`
     * route to their dedicated helpers; everything else uses
     * `present_if_encode_block`. */
    {
        uint64_t _vle = (uint64_t)(self->id);
        while (_vle >= 0x80u) {
            SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_vle & 0x7Fu) | 0x80u)));
            _vle >>= 7;
        }
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)_vle));
    }
    if ((z & 0x01) != 0) {
        SCE_FORGE_TRY_WRITE(codec_zenoh_decl_ext_keyexpr_encode(&self->ext_keyexpr, w));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_UNDECL_SUBSCRIBER_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_UNDECL_SUBSCRIBER_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_undecl_subscriber_encode_to_buf(const codec_zenoh_undecl_subscriber_t *self, uint8_t *buf, size_t cap, size_t *out_len, uint8_t z) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_undecl_subscriber_encode(self, &_w, z);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_UNDECL_SUBSCRIBER_H */
