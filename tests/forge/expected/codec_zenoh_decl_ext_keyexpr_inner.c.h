// SCE-MAP: codec_zenoh_decl_ext_keyexpr_inner:64

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_H
#define SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"

#define CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_MIN_BYTES 1
#define CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_MAX_BYTES 139

typedef struct {
    uint8_t inner_header;
    uint64_t id;
    /* variable-length payload (sce:bit-size="tail", sce:max-size="128") */
    uint8_t suffix[128];
    size_t  suffix_len;
} codec_zenoh_decl_ext_keyexpr_inner_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_inner_decode(sce_forge_cursor_t *cursor, codec_zenoh_decl_ext_keyexpr_inner_t *out) {
    /* RFC §synth-5-B present-if primitive: streaming decode
     * advances the cursor per field. C11 has no nullable wrapper so
     * the gated field's storage stays as plain `T` (with `_len = 0`
     * for absent bytes payloads); the carrier's flag bit is the
     * source of truth for presence. Gating extends to Tail /
     * LengthRef / Vle bit-sizes via dispatch inside the helper.
     * Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified streaming path. */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->inner_header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint64_t id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->id = id;
    if ((out->inner_header & 0x01) != 0) {
        size_t _n = sce_forge_cursor_remaining(cursor);
        if (_n > 128) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->suffix, raw, _n);
        out->suffix_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->suffix_len = 0;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_MAX_BYTES bytes and use
 * `codec_zenoh_decl_ext_keyexpr_inner_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_inner_encode(const codec_zenoh_decl_ext_keyexpr_inner_t *self, sce_forge_writer_t *w) {
    /* RFC §synth-5-B present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->inner_header));
    {
        uint64_t _vle = (uint64_t)(self->id);
        while (_vle >= 0x80u) {
            SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_vle & 0x7Fu) | 0x80u)));
            _vle >>= 7;
        }
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)_vle));
    }
    if ((self->inner_header & 0x01) != 0) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, self->suffix, self->suffix_len));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_inner_encode_to_buf(const codec_zenoh_decl_ext_keyexpr_inner_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_decl_ext_keyexpr_inner_encode(self, &_w);
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
static inline bool codec_zenoh_decl_ext_keyexpr_inner_n(const codec_zenoh_decl_ext_keyexpr_inner_t *self) {
    return (self->inner_header & 0x01) != 0;
}

static inline void codec_zenoh_decl_ext_keyexpr_inner_set_n(codec_zenoh_decl_ext_keyexpr_inner_t *self, bool v) {
    if (v) {
        self->inner_header = (uint8_t)(self->inner_header | 0x01);
    } else {
        self->inner_header = (uint8_t)(self->inner_header & (uint8_t)(~(uint8_t)0x01));
    }
}

static inline bool codec_zenoh_decl_ext_keyexpr_inner_m(const codec_zenoh_decl_ext_keyexpr_inner_t *self) {
    return (self->inner_header & 0x02) != 0;
}

static inline void codec_zenoh_decl_ext_keyexpr_inner_set_m(codec_zenoh_decl_ext_keyexpr_inner_t *self, bool v) {
    if (v) {
        self->inner_header = (uint8_t)(self->inner_header | 0x02);
    } else {
        self->inner_header = (uint8_t)(self->inner_header & (uint8_t)(~(uint8_t)0x02));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_H */
