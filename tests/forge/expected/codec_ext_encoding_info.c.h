/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_EXT_ENCODING_INFO_H
#define SCE_FORGE_CODEC_EXT_ENCODING_INFO_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_EXT_ENCODING_INFO_MIN_BYTES 2
#define CODEC_EXT_ENCODING_INFO_MAX_BYTES 71

typedef struct {
    uint32_t combined_id;
    uint8_t schema_size;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="64") */
    uint8_t schema[64];
    size_t  schema_len;
} codec_ext_encoding_info_t;

typedef struct {
    uint8_t bytes[CODEC_EXT_ENCODING_INFO_MAX_BYTES];
    size_t  len;
} codec_ext_encoding_info_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_ext_encoding_info_decode(sce_forge_cursor_t *cursor, codec_ext_encoding_info_t *out) {
    /* RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
     * advances the cursor per field. C11 has no nullable wrapper so
     * the gated field's storage stays as plain `T` (with `_len = 0`
     * for absent bytes payloads); the carrier's flag bit is the
     * source of truth for presence. B2-β extends gating to Tail /
     * LengthRef / Vle bit-sizes via dispatch inside the helper.
     * Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified streaming path. */
    uint32_t combined_id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u32(cursor, &combined_id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->combined_id = combined_id;
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->schema_size = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((out->combined_id & 0x00000001) != 0) {
        size_t _n = (size_t)out->schema_size;
        if (_n > 64) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->schema, raw, _n);
        out->schema_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->schema_len = 0;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_ext_encoding_info_encoded_t codec_ext_encoding_info_encode(const codec_ext_encoding_info_t *self) {
    codec_ext_encoding_info_encoded_t r;
    /* RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    r.len = 0;
    {
        uint64_t _w = (uint64_t)(self->combined_id);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    r.bytes[r.len++] = self->schema_size;
    if ((self->combined_id & 0x00000001) != 0) {
        for (size_t _bi = 0; _bi < self->schema_len && _bi < self->schema_size; ++_bi) r.bytes[r.len++] = self->schema[_bi];
    }
    return r;
}

/* RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
 * field. Read returns a bool from `(field & mask) != 0`; write toggles
 * the bit on/off without disturbing siblings on the same carrier. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline bool codec_ext_encoding_info_has_schema(const codec_ext_encoding_info_t *self) {
    return (self->combined_id & 0x00000001) != 0;
}

static inline void codec_ext_encoding_info_set_has_schema(codec_ext_encoding_info_t *self, bool v) {
    if (v) {
        self->combined_id = (uint32_t)(self->combined_id | 0x00000001);
    } else {
        self->combined_id = (uint32_t)(self->combined_id & (uint32_t)(~(uint32_t)0x00000001));
    }
}

#endif  /* SCE_FORGE_CODEC_EXT_ENCODING_INFO_H */
