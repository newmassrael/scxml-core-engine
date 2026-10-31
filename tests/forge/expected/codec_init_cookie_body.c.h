// SCE-MAP: codec_init_cookie_body:36 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_INIT_COOKIE_BODY_H
#define SCE_FORGE_CODEC_INIT_COOKIE_BODY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */

#define CODEC_INIT_COOKIE_BODY_MIN_BYTES 1
#define CODEC_INIT_COOKIE_BODY_MAX_BYTES 68

#ifndef SCE_OPTIONAL_BYTES_64
#define SCE_OPTIONAL_BYTES_64
#ifndef SCE_STATIC_BYTES_64
#define SCE_STATIC_BYTES_64
/* SCE Accepted Subset §2.15: a `bytes` of at most 64 bytes. */
typedef struct {
    uint8_t data[64];
    size_t len;
} sce_static_bytes_64_t;

static inline sce_static_bytes_64_t sce_static_bytes_64_of(sce_forge_bytes_view_t bytes) {
    sce_static_bytes_64_t value = { { 0 }, 0 };
    for (size_t i = 0; i < bytes.len && i < 64u; ++i) {
        value.data[i] = bytes.data[i];
    }
    value.len = bytes.len < 64u ? bytes.len : 64u;
    return value;
}
#endif
/* docs/adr/0055: a value of `sce_static_bytes_64_t` that may be absent. */
SCE_OPTIONAL_DECLARE_BYTES(64)
#endif

typedef struct {
    uint8_t version;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_uint16_t cookie_size;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_bytes_64_t cookie;
} codec_init_cookie_body_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_init_cookie_body_decode(sce_forge_cursor_t *cursor, codec_init_cookie_body_t *out, uint8_t a) {
    /* Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
     * `<sce:flag-input>` so codecs that haven't consumed an input via
     * `present-if` yet compile cleanly under -Wunused-parameter. */
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
    if ((a & 0x01) != 0) {
        uint16_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u16(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        out->cookie_size = sce_optional_uint16_some(_v);
    } else {
        out->cookie_size = sce_optional_uint16_none();
    }
    if ((a & 0x01) != 0) {
        size_t _n = (size_t)out->cookie_size.value;
        if (_n > 64) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->cookie = sce_optional_bytes_64_none();
        memcpy(out->cookie.value.data, raw, _n);
        out->cookie.value.len = _n;
        out->cookie.present = true;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->cookie = sce_optional_bytes_64_none();
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_INIT_COOKIE_BODY_MAX_BYTES bytes and use
 * `codec_init_cookie_body_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_init_cookie_body_encode(const codec_init_cookie_body_t *self, sce_forge_writer_t *w, uint8_t a) {
    /* Declared-but-unconsumed flag inputs: see decode — same suppress per input. */
    (void)a;
    if ((a & 0x01) != 0) {
        if (!self->cookie.present) return SCE_FORGE_CODEC_PRESENT_IF_MISMATCH;
    } else if (self->cookie.present) return SCE_FORGE_CODEC_PRESENT_IF_MISMATCH;
    /* A byte string, text, list or chain above the bound of the array that
     * holds it is refused before the first byte is written: the array cannot
     * hold that many, and a frame of fewer is not the value given. */
    if (self->cookie.present && !sce_forge_length_within_bound(self->cookie.value.len, 64)) {
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
    if (self->cookie_size.present) {
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u16(w, (uint16_t)(self->cookie_size.value)));
    }
    if (self->cookie.present) {
        size_t _n = self->cookie.value.len;
        if (_n > self->cookie_size.value) _n = self->cookie_size.value;
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, self->cookie.value.data, _n));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_INIT_COOKIE_BODY_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_INIT_COOKIE_BODY_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_init_cookie_body_encode_to_buf(const codec_init_cookie_body_t *self, uint8_t *buf, size_t cap, size_t *out_len, uint8_t a) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_init_cookie_body_encode(self, &_w, a);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_INIT_COOKIE_BODY_H */
