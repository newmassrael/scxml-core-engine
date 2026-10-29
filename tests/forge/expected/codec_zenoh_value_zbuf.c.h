// SCE-MAP: codec_zenoh_value_zbuf:27 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_VALUE_ZBUF_H
#define SCE_FORGE_CODEC_ZENOH_VALUE_ZBUF_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */
#include "codec_zenoh_value_slice.h"

#define CODEC_ZENOH_VALUE_ZBUF_MIN_BYTES 0
#define CODEC_ZENOH_VALUE_ZBUF_MAX_BYTES 163

#ifndef SCE_OPTIONAL_BYTES_32
#define SCE_OPTIONAL_BYTES_32
#ifndef SCE_STATIC_BYTES_32
#define SCE_STATIC_BYTES_32
/* SCE Accepted Subset §2.15: a `bytes` of at most 32 bytes. */
typedef struct {
    uint8_t data[32];
    size_t len;
} sce_static_bytes_32_t;

static inline sce_static_bytes_32_t sce_static_bytes_32_of(sce_forge_bytes_view_t bytes) {
    sce_static_bytes_32_t value = { { 0 }, 0 };
    for (size_t i = 0; i < bytes.len && i < 32u; ++i) {
        value.data[i] = bytes.data[i];
    }
    value.len = bytes.len < 32u ? bytes.len : 32u;
    return value;
}
#endif
/* docs/adr/0055: a value of `sce_static_bytes_32_t` that may be absent. */
SCE_OPTIONAL_DECLARE_BYTES(32)
#endif

typedef struct {
    uint64_t value_len;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_bytes_32_t value;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_uint64_t encoding;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_uint64_t slice_count;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    struct { bool present; struct { codec_zenoh_value_slice_t data[4]; size_t len; } value; } slices;
} codec_zenoh_value_zbuf_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_value_zbuf_decode(sce_forge_cursor_t *cursor, codec_zenoh_value_zbuf_t *out, uint8_t after_shm) {
    /* Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
     * `<sce:flag-input>` so codecs that haven't consumed an input via
     * `present-if` yet compile cleanly under -Wunused-parameter. */
    (void)after_shm;
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
    uint64_t value_len;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &value_len);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->value_len = value_len;
    if ((after_shm & 0x01) == 0) {
        size_t _n = (size_t)out->value_len;
        if (_n > 32) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->value = sce_optional_bytes_32_none();
        memcpy(out->value.value.data, raw, _n);
        out->value.value.len = _n;
        out->value.present = true;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->value = sce_optional_bytes_32_none();
    }
    if ((after_shm & 0x01) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        out->encoding = sce_optional_uint64_some(_v);
    } else {
        out->encoding = sce_optional_uint64_none();
    }
    if ((after_shm & 0x01) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        out->slice_count = sce_optional_uint64_some(_v);
    } else {
        out->slice_count = sce_optional_uint64_none();
    }
    if ((after_shm & 0x01) != 0) {
        size_t _n = (size_t)out->slice_count.value;
        if (_n > 4) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memset(&out->slices, 0, sizeof out->slices);
        for (size_t _i = 0; _i < _n; ++_i) {
            sce_forge_codec_status_t _st = codec_zenoh_value_slice_decode(cursor, &out->slices.value.data[_i]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
        }
        out->slices.value.len = _n;
        out->slices.present = true;
    } else {
        memset(&out->slices, 0, sizeof out->slices);
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_VALUE_ZBUF_MAX_BYTES bytes and use
 * `codec_zenoh_value_zbuf_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_value_zbuf_encode(const codec_zenoh_value_zbuf_t *self, sce_forge_writer_t *w, uint8_t after_shm) {
    /* Declared-but-unconsumed flag inputs: see decode — same suppress per input. */
    (void)after_shm;
    /* Streaming cursor encode (SSOT selection: `needs_streaming`).
     * Mirrors the streaming decode: every field appends its own bytes in
     * declaration order through the per-field encode blocks, so a gated
     * field skips its append when the carrier flag bit is clear, and a
     * fixed field after a variable-length payload lands after the payload
     * (the positional path appends variable fields last, placing it ahead
     * on the wire). Per-field `is_repeat` / `is_tlv_chain` / `is_embed`
     * route to their dedicated helpers; everything else uses
     * `present_if_encode_block`. */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(self->value_len)));
    if (self->value.present) {
        size_t _n = self->value.value.len;
        if (_n > self->value_len) _n = self->value_len;
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, self->value.value.data, _n));
    }
    if (self->encoding.present) {
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(self->encoding.value)));
    }
    if (self->slice_count.present) {
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(self->slice_count.value)));
    }
    if (self->slices.present) {
        for (size_t _ri = 0; _ri < self->slices.value.len; ++_ri) {
            SCE_FORGE_TRY_WRITE(codec_zenoh_value_slice_encode(&self->slices.value.data[_ri], w));
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_VALUE_ZBUF_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_VALUE_ZBUF_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_value_zbuf_encode_to_buf(const codec_zenoh_value_zbuf_t *self, uint8_t *buf, size_t cap, size_t *out_len, uint8_t after_shm) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_value_zbuf_encode(self, &_w, after_shm);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_VALUE_ZBUF_H */
