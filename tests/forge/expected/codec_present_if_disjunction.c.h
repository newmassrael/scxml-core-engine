/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_PRESENT_IF_DISJUNCTION_H
#define SCE_FORGE_CODEC_PRESENT_IF_DISJUNCTION_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_PRESENT_IF_DISJUNCTION_MIN_BYTES 3
#define CODEC_PRESENT_IF_DISJUNCTION_MAX_BYTES 3

typedef struct {
    uint8_t flags;
    uint16_t seq;
} codec_present_if_disjunction_t;

typedef struct {
    uint8_t bytes[CODEC_PRESENT_IF_DISJUNCTION_MAX_BYTES];
    size_t  len;
} codec_present_if_disjunction_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_present_if_disjunction_decode(sce_forge_cursor_t *cursor, codec_present_if_disjunction_t *out) {
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
    if ((out->flags & 0x01) != 0 || (out->flags & 0x02) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 2);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->seq = (uint16_t)(((uint16_t)raw[0] << 8) | raw[1]);
        if (!sce_forge_cursor_advance(cursor, 2)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->seq = 0;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_present_if_disjunction_encoded_t codec_present_if_disjunction_encode(const codec_present_if_disjunction_t *self) {
    codec_present_if_disjunction_encoded_t r;
    /* RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    r.len = 0;
    r.bytes[r.len++] = self->flags;
    if ((self->flags & 0x01) != 0 || (self->flags & 0x02) != 0) {
        r.bytes[r.len++] = (uint8_t)((self->seq >> 8) & 0xFF);
        r.bytes[r.len++] = (uint8_t)(self->seq & 0xFF);
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
static inline bool codec_present_if_disjunction_wants_a(const codec_present_if_disjunction_t *self) {
    return (self->flags & 0x01) != 0;
}

static inline void codec_present_if_disjunction_set_wants_a(codec_present_if_disjunction_t *self, bool v) {
    if (v) {
        self->flags = (uint8_t)(self->flags | 0x01);
    } else {
        self->flags = (uint8_t)(self->flags & (uint8_t)(~(uint8_t)0x01));
    }
}

static inline bool codec_present_if_disjunction_wants_b(const codec_present_if_disjunction_t *self) {
    return (self->flags & 0x02) != 0;
}

static inline void codec_present_if_disjunction_set_wants_b(codec_present_if_disjunction_t *self, bool v) {
    if (v) {
        self->flags = (uint8_t)(self->flags | 0x02);
    } else {
        self->flags = (uint8_t)(self->flags & (uint8_t)(~(uint8_t)0x02));
    }
}

#endif  /* SCE_FORGE_CODEC_PRESENT_IF_DISJUNCTION_H */
