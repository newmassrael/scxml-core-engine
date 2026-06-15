// SCE-MAP: codec_nested_parent:22

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_NESTED_PARENT_H
#define SCE_FORGE_CODEC_NESTED_PARENT_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_nested_body.h"

#define CODEC_NESTED_PARENT_MIN_BYTES 2
#define CODEC_NESTED_PARENT_MAX_BYTES 2726

typedef struct {
    uint8_t hdr;
    uint8_t m;
    /* RFC §synth-5-B embed: nested codec_nested_body_t struct (no length prefix on the wire) */
    codec_nested_body_t required_body;
    /* RFC §synth-5-B embed: nested codec_nested_body_t struct (no length prefix on the wire) */
    codec_nested_body_t optional_body;
    /* RFC §synth-5-B B2 repeat: fixed array of codec_nested_body_t elements (max 4) */
    codec_nested_body_t body_list[4];
    size_t  body_list_len;
} codec_nested_parent_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_nested_parent_decode(sce_forge_cursor_t *cursor, codec_nested_parent_t *out) {
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
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->hdr = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->m = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        sce_forge_codec_status_t _st = codec_nested_body_decode(cursor, &out->required_body);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
    }
    if ((out->hdr & 0x01) != 0) {
        sce_forge_codec_status_t _st = codec_nested_body_decode(cursor, &out->optional_body);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
    }
    {
        size_t _n = (size_t)out->m;
        if (_n > 4) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        for (size_t _i = 0; _i < _n; ++_i) {
            sce_forge_codec_status_t _st = codec_nested_body_decode(cursor, &out->body_list[_i]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
        }
        out->body_list_len = _n;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_NESTED_PARENT_MAX_BYTES bytes and use
 * `codec_nested_parent_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_nested_parent_encode(const codec_nested_parent_t *self, sce_forge_writer_t *w) {
    /* Streaming cursor encode (SSOT selection: `needs_streaming`).
     * Mirrors the streaming decode: every field appends its own bytes in
     * declaration order through the per-field encode blocks, so a gated
     * field skips its append when the carrier flag bit is clear, and a
     * fixed field after a variable-length payload lands after the payload
     * (the positional path appends variable fields last, placing it ahead
     * on the wire). Per-field `is_repeat` / `is_tlv_chain` / `is_embed`
     * route to their dedicated helpers; everything else uses
     * `present_if_encode_block`. */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->hdr));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->m));
    SCE_FORGE_TRY_WRITE(codec_nested_body_encode(&self->required_body, w));
    if ((self->hdr & 0x01) != 0) {
        SCE_FORGE_TRY_WRITE(codec_nested_body_encode(&self->optional_body, w));
    }
    for (size_t _ri = 0; _ri < self->body_list_len; ++_ri) {
        SCE_FORGE_TRY_WRITE(codec_nested_body_encode(&self->body_list[_ri], w));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_NESTED_PARENT_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_NESTED_PARENT_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_nested_parent_encode_to_buf(const codec_nested_parent_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_nested_parent_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

/* RFC §synth-5-B flags primitive: per-bit-range accessors over
 * the carrier field. Single-bit (width=1) reads as bool; multi-bit
 * (width>=2) reads as the smallest unsigned C11 integer type that fits
 * (uint8_t / uint16_t / uint32_t / uint64_t). Setters mask + shift on
 * the way in so out-of-range callers can't corrupt sibling bits. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline bool codec_nested_parent_has_opt(const codec_nested_parent_t *self) {
    return (self->hdr & 0x01) != 0;
}

static inline void codec_nested_parent_set_has_opt(codec_nested_parent_t *self, bool v) {
    if (v) {
        self->hdr = (uint8_t)(self->hdr | 0x01);
    } else {
        self->hdr = (uint8_t)(self->hdr & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_NESTED_PARENT_H */
