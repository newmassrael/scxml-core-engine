/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_PRESENT_IF_VLE_H
#define SCE_FORGE_CODEC_PRESENT_IF_VLE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_PRESENT_IF_VLE_MIN_BYTES 1
#define CODEC_PRESENT_IF_VLE_MAX_BYTES 11

typedef struct {
    uint8_t flags;
    uint64_t optional_id;
} codec_present_if_vle_t;

typedef struct {
    uint8_t bytes[CODEC_PRESENT_IF_VLE_MAX_BYTES];
    size_t  len;
} codec_present_if_vle_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_present_if_vle_decode(sce_forge_cursor_t *cursor, codec_present_if_vle_t *out) {
    /* RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
     * advances the cursor per field. C11 has no nullable wrapper so
     * the gated field's storage stays as plain `T` (with `_len = 0`
     * for absent bytes payloads); the carrier's flag bit is the
     * source of truth for presence. B2-β extends gating to Tail /
     * LengthRef / Vle bit-sizes via dispatch inside the helper.
     * Per-field `is_repeat` routes Repeat fields to the dedicated
     * helper. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified streaming path. */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->flags = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((out->flags & 0x01) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        out->optional_id = _v;
    } else {
        out->optional_id = 0;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_present_if_vle_encoded_t codec_present_if_vle_encode(const codec_present_if_vle_t *self) {
    codec_present_if_vle_encoded_t r;
    /* RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` routes Repeat fields to the
     * dedicated helper. Branch fires before has_vle_fields so a
     * codec mixing VLE + present-if uses the unified encode path. */
    r.len = 0;
    r.bytes[r.len++] = self->flags;
    if ((self->flags & 0x01) != 0) {
    {
        uint64_t _w = (uint64_t)(self->optional_id);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    }
    return r;
}

/* RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
 * field. Read returns a bool from `(field & mask) != 0`; write toggles
 * the bit on/off without disturbing siblings on the same carrier. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline bool codec_present_if_vle_has_id(const codec_present_if_vle_t *self) {
    return (self->flags & 0x01) != 0;
}

static inline void codec_present_if_vle_set_has_id(codec_present_if_vle_t *self, bool v) {
    if (v) {
        self->flags = (uint8_t)(self->flags | 0x01);
    } else {
        self->flags = (uint8_t)(self->flags & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_PRESENT_IF_VLE_H */
