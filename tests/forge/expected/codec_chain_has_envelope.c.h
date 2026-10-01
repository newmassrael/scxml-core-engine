// SCE-MAP: codec_chain_has_envelope:23 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_CHAIN_HAS_ENVELOPE_H
#define SCE_FORGE_CODEC_CHAIN_HAS_ENVELOPE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"
#include "codec_chain_has_slice.h"

#define CODEC_CHAIN_HAS_ENVELOPE_MIN_BYTES 1
#define CODEC_CHAIN_HAS_ENVELOPE_MAX_BYTES 1511

typedef struct {
    uint8_t header;
    /* RFC §synth-5-B B3 tlv-chain: fixed array of codec_zenoh_ext_entry_t entries (max-depth 4, on-overflow=reject) */
    codec_zenoh_ext_entry_t extensions[4];
    size_t  extensions_len;
    uint64_t payload_len;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="256") */
    uint8_t payload[256];
    uint32_t slice_count;
    /* RFC §synth-5-B B2 repeat: fixed array of codec_chain_has_slice_t elements (max 4) */
    codec_chain_has_slice_t slices[4];
    size_t  slices_len;
} codec_chain_has_envelope_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_chain_has_envelope_decode(sce_forge_cursor_t *cursor, codec_chain_has_envelope_t *out) {
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
        out->header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    out->extensions_len = 0;
        if ((out->header & 0x80) != 0) {
            bool _more = false;
            for (size_t _i = 0; _i < 4; ++_i) {
                if (sce_forge_cursor_remaining(cursor) == 0) break;
                sce_forge_codec_status_t _st = codec_zenoh_ext_entry_decode(cursor, &out->extensions[out->extensions_len]);
                if (_st != SCE_FORGE_CODEC_OK) return _st;
                size_t _just = out->extensions_len;
                out->extensions_len++;
                _more = codec_zenoh_ext_entry_z(&out->extensions[_just]);
                if (!_more) break;
            }
            if (_more && sce_forge_cursor_remaining(cursor) == 0) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
            if (_more) return SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW;
        }
    bool _has_extensions_2 = false;
    for (size_t _i = 0; _i < out->extensions_len; ++_i) {
        if ((uint64_t)(codec_zenoh_ext_entry_ext_id(&out->extensions[_i])) == 2ULL) {
            _has_extensions_2 = true;
            break;
        }
    }
    if (!_has_extensions_2) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        out->payload_len = _v;
    } else {
        out->payload_len = 0;
    }
    if (!_has_extensions_2) {
        size_t _n = (size_t)out->payload_len;
        if (_n > 256) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->payload, raw, _n);
        out->payload_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->payload_len = 0;
    }
    if (_has_extensions_2) {
        uint32_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u32(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        out->slice_count = _v;
    } else {
        out->slice_count = 0;
    }
    if (_has_extensions_2) {
        size_t _n = (size_t)out->slice_count;
        if (_n > 4) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        for (size_t _i = 0; _i < _n; ++_i) {
            sce_forge_codec_status_t _st = codec_chain_has_slice_decode(cursor, &out->slices[_i]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
        }
        out->slices_len = _n;
    } else {
        out->slices_len = 0;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_CHAIN_HAS_ENVELOPE_MAX_BYTES bytes and use
 * `codec_chain_has_envelope_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_chain_has_envelope_encode(const codec_chain_has_envelope_t *self, sce_forge_writer_t *w) {
    bool _has_extensions_2 = false;
    for (size_t _i = 0; _i < self->extensions_len; ++_i) {
        if ((uint64_t)(codec_zenoh_ext_entry_ext_id(&self->extensions[_i])) == 2ULL) {
            _has_extensions_2 = true;
            break;
        }
    }
    /* Streaming cursor encode (SSOT selection: `needs_streaming`).
     * Mirrors the streaming decode: every field appends its own bytes in
     * declaration order through the per-field encode blocks, so a gated
     * field skips its append when the carrier flag bit is clear, and a
     * fixed field after a variable-length payload lands after the payload
     * (the positional path appends variable fields last, placing it ahead
     * on the wire). Per-field `is_repeat` / `is_tlv_chain` / `is_embed`
     * route to their dedicated helpers; everything else uses
     * `present_if_encode_block`. */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->header));
    if ((self->header & 0x80) != 0) {
        for (size_t _ti = 0; _ti < self->extensions_len; ++_ti) {
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_entry_encode(&self->extensions[_ti], w));
        }
    }
    if (!_has_extensions_2) {
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(self->payload_len)));
    }
    if (!_has_extensions_2) {
        size_t _n = self->payload_len;
        if (_n > self->payload_len) _n = self->payload_len;
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, self->payload, _n));
    }
    if (_has_extensions_2) {
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u32(w, (uint32_t)(self->slice_count)));
    }
    if (_has_extensions_2) {
        for (size_t _ri = 0; _ri < self->slices_len; ++_ri) {
            SCE_FORGE_TRY_WRITE(codec_chain_has_slice_encode(&self->slices[_ri], w));
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_CHAIN_HAS_ENVELOPE_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_CHAIN_HAS_ENVELOPE_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_chain_has_envelope_encode_to_buf(const codec_chain_has_envelope_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_chain_has_envelope_encode(self, &_w);
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
static inline uint8_t codec_chain_has_envelope_kind(const codec_chain_has_envelope_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x1F);
}

static inline void codec_chain_has_envelope_set_kind(codec_chain_has_envelope_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x1F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x1F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_chain_has_envelope_e(const codec_chain_has_envelope_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_chain_has_envelope_set_e(codec_chain_has_envelope_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_CHAIN_HAS_ENVELOPE_H */
