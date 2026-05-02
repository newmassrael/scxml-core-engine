/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_PRESENT_IF_BASIC_H
#define SCE_FORGE_CODEC_PRESENT_IF_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_PRESENT_IF_BASIC_MIN_BYTES 3
#define CODEC_PRESENT_IF_BASIC_MAX_BYTES 3

typedef struct {
    uint8_t flags;
    uint16_t seq;
} codec_present_if_basic_t;

typedef struct {
    uint8_t bytes[CODEC_PRESENT_IF_BASIC_MAX_BYTES];
    size_t  len;
} codec_present_if_basic_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_present_if_basic_decode(sce_forge_cursor_t *cursor, codec_present_if_basic_t *out) {
    /* RFC §5.B B1-δ present-if primitive: streaming decode advances
     * the cursor per field. C11 has no nullable wrapper so the gated
     * field's storage stays as plain `T`; the carrier's flag bit is
     * the source of truth for presence and the absent branch zeroes
     * the field so the struct is fully initialized either way. */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->flags = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((out->flags & 0x01) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 2);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->seq = (uint16_t)(((uint16_t)raw[0] << 8) | raw[1]);
        if (!sce_forge_cursor_advance(cursor, 2)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->seq = 0;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_present_if_basic_encoded_t codec_present_if_basic_encode(const codec_present_if_basic_t *self) {
    codec_present_if_basic_encoded_t r;
    /* RFC §5.B B1-δ encode: per-field byte append. Gated fields skip
     * the append when the carrier's flag bit is clear (author keeps
     * the bit and the logical truth value in sync — same trust
     * contract as the variant primitive). `r.bytes[r.len++]` lets the
     * length grow with the actual payload. */
    r.len = 0;
    r.bytes[r.len++] = self->flags;
    if ((self->flags & 0x01) != 0) {
        r.bytes[r.len++] = (uint8_t)((self->seq >> 8) & 0xFF);
        r.bytes[r.len++] = (uint8_t)(self->seq & 0xFF);
    }
    return r;
}

/* RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
 * field. Read returns a bool from `(field & mask) != 0`; write toggles
 * the bit on/off without disturbing siblings on the same carrier. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline bool codec_present_if_basic_has_seq(const codec_present_if_basic_t *self) {
    return (self->flags & 0x01) != 0;
}

static inline void codec_present_if_basic_set_has_seq(codec_present_if_basic_t *self, bool v) {
    if (v) {
        self->flags = (uint8_t)(self->flags | 0x01);
    } else {
        self->flags = (uint8_t)(self->flags & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_PRESENT_IF_BASIC_H */
