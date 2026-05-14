// SCE-MAP: codec_repeat_present_if_basic:37

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_REPEAT_PRESENT_IF_BASIC_H
#define SCE_FORGE_CODEC_REPEAT_PRESENT_IF_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_repeat_elem.h"

#define CODEC_REPEAT_PRESENT_IF_BASIC_MIN_BYTES 2
#define CODEC_REPEAT_PRESENT_IF_BASIC_MAX_BYTES 66

typedef struct {
    uint8_t carrier;
    uint8_t num_elems;
    /* RFC §5.B B2 repeat: fixed array of codec_repeat_elem_t elements (max 32) */
    codec_repeat_elem_t elems[32];
    size_t  elems_len;
} codec_repeat_present_if_basic_t;

typedef struct {
    uint8_t bytes[CODEC_REPEAT_PRESENT_IF_BASIC_MAX_BYTES];
    size_t  len;
} codec_repeat_present_if_basic_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_repeat_present_if_basic_decode(sce_forge_cursor_t *cursor, codec_repeat_present_if_basic_t *out) {
    /* RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
     * advances the cursor per field. C11 has no nullable wrapper so
     * the gated field's storage stays as plain `T` (with `_len = 0`
     * for absent bytes payloads); the carrier's flag bit is the
     * source of truth for presence. B2-β extends gating to Tail /
     * LengthRef / Vle bit-sizes via dispatch inside the helper.
     * Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified streaming path. */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->carrier = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((out->carrier & 0x01) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->num_elems = (uint8_t)(raw[0]);
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->num_elems = 0;
    }
    if ((out->carrier & 0x01) != 0) {
        size_t _n = (size_t)out->num_elems;
        if (_n > 32) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        for (size_t _i = 0; _i < _n; ++_i) {
            sce_forge_codec_status_t _st = codec_repeat_elem_decode(cursor, &out->elems[_i]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
        }
        out->elems_len = _n;
    } else {
        out->elems_len = 0;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_repeat_present_if_basic_encoded_t codec_repeat_present_if_basic_encode(const codec_repeat_present_if_basic_t *self) {
    codec_repeat_present_if_basic_encoded_t r;
    /* RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    r.len = 0;
    r.bytes[r.len++] = self->carrier;
    if ((self->carrier & 0x01) != 0) {
        r.bytes[r.len++] = self->num_elems;
    }
    if ((self->carrier & 0x01) != 0) {
        for (size_t _ri = 0; _ri < self->elems_len; ++_ri) {
            codec_repeat_elem_encoded_t _sub = codec_repeat_elem_encode(&self->elems[_ri]);
            if (r.len + _sub.len <= sizeof(r.bytes)) {
                for (size_t _rj = 0; _rj < _sub.len; ++_rj) r.bytes[r.len + _rj] = _sub.bytes[_rj];
                r.len += _sub.len;
            }
        }
    }
    return r;
}

/* RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
 * the carrier field. Single-bit (width=1) reads as bool; multi-bit
 * (width>=2) reads as the smallest unsigned C11 integer type that fits
 * (uint8_t / uint16_t / uint32_t / uint64_t). Setters mask + shift on
 * the way in so out-of-range callers can't corrupt sibling bits. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline bool codec_repeat_present_if_basic_has_list(const codec_repeat_present_if_basic_t *self) {
    return (self->carrier & 0x01) != 0;
}

static inline void codec_repeat_present_if_basic_set_has_list(codec_repeat_present_if_basic_t *self, bool v) {
    if (v) {
        self->carrier = (uint8_t)(self->carrier | 0x01);
    } else {
        self->carrier = (uint8_t)(self->carrier & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_REPEAT_PRESENT_IF_BASIC_H */
