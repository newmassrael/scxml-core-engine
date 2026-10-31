// SCE-MAP: codec_zenoh_init_body:42 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_INIT_BODY_H
#define SCE_FORGE_CODEC_ZENOH_INIT_BODY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */

#define CODEC_ZENOH_INIT_BODY_MIN_BYTES 6
#define CODEC_ZENOH_INIT_BODY_MAX_BYTES 159

#ifndef SCE_OPTIONAL_BYTES_128
#define SCE_OPTIONAL_BYTES_128
#ifndef SCE_STATIC_BYTES_128
#define SCE_STATIC_BYTES_128
/* SCE Accepted Subset §2.15: a `bytes` of at most 128 bytes. */
typedef struct {
    uint8_t data[128];
    size_t len;
} sce_static_bytes_128_t;

static inline sce_static_bytes_128_t sce_static_bytes_128_of(sce_forge_bytes_view_t bytes) {
    sce_static_bytes_128_t value = { { 0 }, 0 };
    for (size_t i = 0; i < bytes.len && i < 128u; ++i) {
        value.data[i] = bytes.data[i];
    }
    value.len = bytes.len < 128u ? bytes.len : 128u;
    return value;
}
#endif
/* docs/adr/0055: a value of `sce_static_bytes_128_t` that may be absent. */
SCE_OPTIONAL_DECLARE_BYTES(128)
#endif

typedef struct {
    uint8_t version;
    uint8_t cbyte;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="16") */
    uint8_t zid[16];
    size_t  zid_len;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_uint8_t sn_res;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_uint16_t batch_size;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_uint64_t cookie_len;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_bytes_128_t cookie;
} codec_zenoh_init_body_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_init_body_decode(sce_forge_cursor_t *cursor, codec_zenoh_init_body_t *out, uint8_t s, uint8_t a) {
    /* Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
     * `<sce:flag-input>` so codecs that haven't consumed an input via
     * `present-if` yet compile cleanly under -Wunused-parameter. */
    (void)s;
    (void)a;
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
        out->version = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->cbyte = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        size_t _n = (size_t)((int64_t)(size_t)((out->cbyte >> 4) & 0xF) + 1);
        if (_n > 16) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->zid, raw, _n);
        out->zid_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((s & 0x01) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->sn_res = sce_optional_uint8_some((uint8_t)(raw[0]));
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->sn_res = sce_optional_uint8_none();
    }
    if ((s & 0x01) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 2);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->batch_size = sce_optional_uint16_some((uint16_t)(raw[0] | ((uint16_t)raw[1] << 8)));
        if (!sce_forge_cursor_advance(cursor, 2)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->batch_size = sce_optional_uint16_none();
    }
    if ((a & 0x01) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        out->cookie_len = sce_optional_uint64_some(_v);
    } else {
        out->cookie_len = sce_optional_uint64_none();
    }
    if ((a & 0x01) != 0) {
        size_t _n = (size_t)out->cookie_len.value;
        if (_n > 128) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->cookie = sce_optional_bytes_128_none();
        memcpy(out->cookie.value.data, raw, _n);
        out->cookie.value.len = _n;
        out->cookie.present = true;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->cookie = sce_optional_bytes_128_none();
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_INIT_BODY_MAX_BYTES bytes and use
 * `codec_zenoh_init_body_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_init_body_encode(const codec_zenoh_init_body_t *self, sce_forge_writer_t *w, uint8_t s, uint8_t a) {
    /* Declared-but-unconsumed flag inputs: see decode — same suppress per input. */
    (void)s;
    (void)a;
    if ((s & 0x01) != 0) {
        if (!self->sn_res.present) return SCE_FORGE_CODEC_PRESENT_IF_MISMATCH;
    } else if (self->sn_res.present) return SCE_FORGE_CODEC_PRESENT_IF_MISMATCH;
    if ((s & 0x01) != 0) {
        if (!self->batch_size.present) return SCE_FORGE_CODEC_PRESENT_IF_MISMATCH;
    } else if (self->batch_size.present) return SCE_FORGE_CODEC_PRESENT_IF_MISMATCH;
    if ((a & 0x01) != 0) {
        if (!self->cookie.present) return SCE_FORGE_CODEC_PRESENT_IF_MISMATCH;
    } else if (self->cookie.present) return SCE_FORGE_CODEC_PRESENT_IF_MISMATCH;
    /* A byte string, text, list or chain above the bound of the array that
     * holds it is refused before the first byte is written: the array cannot
     * hold that many, and a frame of fewer is not the value given. */
    if (!sce_forge_length_within_bound(self->zid_len, 16)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    if (self->cookie.present && !sce_forge_length_within_bound(self->cookie.value.len, 128)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
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
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->version));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->cbyte));
    {
        size_t _n = self->zid_len;
        if (_n > (size_t)((int64_t)(size_t)((self->cbyte >> 4) & 0xF) + 1)) _n = (size_t)((int64_t)(size_t)((self->cbyte >> 4) & 0xF) + 1);
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, self->zid, _n));
    }
    if (self->sn_res.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->sn_res.value));
    }
    if (self->batch_size.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(self->batch_size.value & 0xFF)));
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((self->batch_size.value >> 8) & 0xFF)));
    }
    if (self->cookie_len.present) {
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(self->cookie_len.value)));
    }
    if (self->cookie.present) {
        size_t _n = self->cookie.value.len;
        if (_n > self->cookie_len.value) _n = self->cookie_len.value;
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, self->cookie.value.data, _n));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_INIT_BODY_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_INIT_BODY_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_init_body_encode_to_buf(const codec_zenoh_init_body_t *self, uint8_t *buf, size_t cap, size_t *out_len, uint8_t s, uint8_t a) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_init_body_encode(self, &_w, s, a);
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
static inline uint8_t codec_zenoh_init_body_whatami(const codec_zenoh_init_body_t *self) {
    return (uint8_t)((self->cbyte >> 0) & (uint8_t)0x03);
}

static inline void codec_zenoh_init_body_set_whatami(codec_zenoh_init_body_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x03 << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x03) << 0);
    self->cbyte = (uint8_t)((self->cbyte & (uint8_t)~_shifted_mask) | _val);
}


static inline uint8_t codec_zenoh_init_body_zid_len_m1(const codec_zenoh_init_body_t *self) {
    return (uint8_t)((self->cbyte >> 4) & (uint8_t)0x0F);
}

static inline void codec_zenoh_init_body_set_zid_len_m1(codec_zenoh_init_body_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 4);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 4);
    self->cbyte = (uint8_t)((self->cbyte & (uint8_t)~_shifted_mask) | _val);
}


#endif  /* SCE_FORGE_CODEC_ZENOH_INIT_BODY_H */
