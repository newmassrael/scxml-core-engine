/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_PRESENT_IF_LENGTH_REF_H
#define SCE_FORGE_CODEC_PRESENT_IF_LENGTH_REF_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"

#define CODEC_PRESENT_IF_LENGTH_REF_MIN_BYTES 2
#define CODEC_PRESENT_IF_LENGTH_REF_MAX_BYTES 34

typedef struct {
    uint8_t flags;
    uint8_t payload_size;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="32") */
    uint8_t payload[32];
    size_t  payload_len;
} codec_present_if_length_ref_t;

typedef struct {
    uint8_t bytes[CODEC_PRESENT_IF_LENGTH_REF_MAX_BYTES];
    size_t  len;
} codec_present_if_length_ref_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_present_if_length_ref_decode(sce_forge_cursor_t *cursor, codec_present_if_length_ref_t *out) {
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
        out->flags = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->payload_size = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((out->flags & 0x01) != 0) {
        size_t _n = (size_t)out->payload_size;
        if (_n > 32) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->payload, raw, _n);
        out->payload_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->payload_len = 0;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_present_if_length_ref_encoded_t codec_present_if_length_ref_encode(const codec_present_if_length_ref_t *self) {
    codec_present_if_length_ref_encoded_t r;
    /* RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    r.len = 0;
    r.bytes[r.len++] = self->flags;
    r.bytes[r.len++] = self->payload_size;
    if ((self->flags & 0x01) != 0) {
        for (size_t _bi = 0; _bi < self->payload_len && _bi < self->payload_size; ++_bi) r.bytes[r.len++] = self->payload[_bi];
    }
    return r;
}

/* RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
 * field. Read returns a bool from `(field & mask) != 0`; write toggles
 * the bit on/off without disturbing siblings on the same carrier. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline bool codec_present_if_length_ref_has_payload(const codec_present_if_length_ref_t *self) {
    return (self->flags & 0x01) != 0;
}

static inline void codec_present_if_length_ref_set_has_payload(codec_present_if_length_ref_t *self, bool v) {
    if (v) {
        self->flags = (uint8_t)(self->flags | 0x01);
    } else {
        self->flags = (uint8_t)(self->flags & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_PRESENT_IF_LENGTH_REF_H */
