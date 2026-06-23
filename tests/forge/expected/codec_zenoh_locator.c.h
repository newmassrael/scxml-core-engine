// SCE-MAP: codec_zenoh_locator:25

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_LOCATOR_H
#define SCE_FORGE_CODEC_ZENOH_LOCATOR_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"

#define CODEC_ZENOH_LOCATOR_MIN_BYTES 0
#define CODEC_ZENOH_LOCATOR_MAX_BYTES 137

typedef struct {
    uint64_t locator_len;
    /* RFC §synth-5-B sce:type="string" payload (sce:max-size="128").
     * `char[N] + size_t len` parallels the bytes pair (uint8_t[N] + len)
     * but the host-language type signals UTF-8 text storage; the C
     * string is NOT NUL-terminated — payloads of exactly `max_size`
     * bytes are valid wire input. */
    char    locator[128];
} codec_zenoh_locator_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_locator_decode(sce_forge_cursor_t *cursor, codec_zenoh_locator_t *out) {
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
    uint64_t locator_len;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &locator_len);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->locator_len = locator_len;
    {
        size_t _n = (size_t)out->locator_len;
        if (_n > 128) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        if (!sce_forge_is_valid_utf8(raw, _n)) return SCE_FORGE_CODEC_INVALID_UTF8;
        memcpy(out->locator, raw, _n);
        out->locator_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_LOCATOR_MAX_BYTES bytes and use
 * `codec_zenoh_locator_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_locator_encode(const codec_zenoh_locator_t *self, sce_forge_writer_t *w) {
    /* Streaming cursor encode (SSOT selection: `needs_streaming`).
     * Mirrors the streaming decode: every field appends its own bytes in
     * declaration order through the per-field encode blocks, so a gated
     * field skips its append when the carrier flag bit is clear, and a
     * fixed field after a variable-length payload lands after the payload
     * (the positional path appends variable fields last, placing it ahead
     * on the wire). Per-field `is_repeat` / `is_tlv_chain` / `is_embed`
     * route to their dedicated helpers; everything else uses
     * `present_if_encode_block`. */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(self->locator_len)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, (const uint8_t*)self->locator, self->locator_len));
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_LOCATOR_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_LOCATOR_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_locator_encode_to_buf(const codec_zenoh_locator_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_locator_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_LOCATOR_H */
