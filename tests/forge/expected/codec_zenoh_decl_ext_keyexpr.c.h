// SCE-MAP: codec_zenoh_decl_ext_keyexpr:89

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_H
#define SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_decl_ext_keyexpr_inner.h"

#define CODEC_ZENOH_DECL_EXT_KEYEXPR_MIN_BYTES 1
#define CODEC_ZENOH_DECL_EXT_KEYEXPR_MAX_BYTES 267

typedef struct {
    uint8_t outer_header;
    uint64_t total_length;
    /* RFC §synth-5-B embed: nested codec_zenoh_decl_ext_keyexpr_inner_t struct (no length prefix on the wire) */
    codec_zenoh_decl_ext_keyexpr_inner_t inner;
} codec_zenoh_decl_ext_keyexpr_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_decode(sce_forge_cursor_t *cursor, codec_zenoh_decl_ext_keyexpr_t *out) {
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
        out->outer_header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint64_t total_length;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &total_length);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->total_length = total_length;
    {
        size_t _len = (size_t)(out->total_length);
        const uint8_t *_raw = sce_forge_cursor_peek(cursor, _len);
        if (_raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        sce_forge_cursor_t _inner = sce_forge_cursor_init(_raw, _len);
        sce_forge_codec_status_t _st = codec_zenoh_decl_ext_keyexpr_inner_decode(&_inner, &out->inner);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
        if (!sce_forge_cursor_advance(cursor, _len)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_DECL_EXT_KEYEXPR_MAX_BYTES bytes and use
 * `codec_zenoh_decl_ext_keyexpr_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_encode(const codec_zenoh_decl_ext_keyexpr_t *self, sce_forge_writer_t *w) {
    /* Streaming cursor encode (SSOT selection: `needs_streaming`).
     * Mirrors the streaming decode: every field appends its own bytes in
     * declaration order through the per-field encode blocks, so a gated
     * field skips its append when the carrier flag bit is clear, and a
     * fixed field after a variable-length payload lands after the payload
     * (the positional path appends variable fields last, placing it ahead
     * on the wire). Per-field `is_repeat` / `is_tlv_chain` / `is_embed`
     * route to their dedicated helpers; everything else uses
     * `present_if_encode_block`. */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->outer_header));
    {
        uint64_t _vle = (uint64_t)(self->total_length);
        while (_vle >= 0x80u) {
            SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_vle & 0x7Fu) | 0x80u)));
            _vle >>= 7;
        }
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)_vle));
    }
    SCE_FORGE_TRY_WRITE(codec_zenoh_decl_ext_keyexpr_inner_encode(&self->inner, w));
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_DECL_EXT_KEYEXPR_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_DECL_EXT_KEYEXPR_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_encode_to_buf(const codec_zenoh_decl_ext_keyexpr_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_decl_ext_keyexpr_encode(self, &_w);
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
static inline uint8_t codec_zenoh_decl_ext_keyexpr_ext_id(const codec_zenoh_decl_ext_keyexpr_t *self) {
    return (uint8_t)((self->outer_header >> 0) & (uint8_t)0x0F);
}

static inline void codec_zenoh_decl_ext_keyexpr_set_ext_id(codec_zenoh_decl_ext_keyexpr_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 0);
    self->outer_header = (uint8_t)((self->outer_header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_decl_ext_keyexpr_m(const codec_zenoh_decl_ext_keyexpr_t *self) {
    return (self->outer_header & 0x10) != 0;
}

static inline void codec_zenoh_decl_ext_keyexpr_set_m(codec_zenoh_decl_ext_keyexpr_t *self, bool v) {
    if (v) {
        self->outer_header = (uint8_t)(self->outer_header | 0x10);
    } else {
        self->outer_header = (uint8_t)(self->outer_header & (uint8_t)(~(uint8_t)0x10));
    }
}

static inline uint8_t codec_zenoh_decl_ext_keyexpr_enc(const codec_zenoh_decl_ext_keyexpr_t *self) {
    return (uint8_t)((self->outer_header >> 5) & (uint8_t)0x03);
}

static inline void codec_zenoh_decl_ext_keyexpr_set_enc(codec_zenoh_decl_ext_keyexpr_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x03 << 5);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x03) << 5);
    self->outer_header = (uint8_t)((self->outer_header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_decl_ext_keyexpr_z(const codec_zenoh_decl_ext_keyexpr_t *self) {
    return (self->outer_header & 0x80) != 0;
}

static inline void codec_zenoh_decl_ext_keyexpr_set_z(codec_zenoh_decl_ext_keyexpr_t *self, bool v) {
    if (v) {
        self->outer_header = (uint8_t)(self->outer_header | 0x80);
    } else {
        self->outer_header = (uint8_t)(self->outer_header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_H */
