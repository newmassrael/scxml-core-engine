// SCE-MAP: codec_peek_arm_a:13

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_PEEK_ARM_A_H
#define SCE_FORGE_CODEC_PEEK_ARM_A_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_PEEK_ARM_A_MIN_BYTES 2
#define CODEC_PEEK_ARM_A_MAX_BYTES 2

typedef struct {
    uint8_t header;
    uint8_t payload;
} codec_peek_arm_a_t;

/* RFC variant-default-uniformity Atomic β-c11: designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_peek_arm_a_t x = CODEC_PEEK_ARM_A_DEFAULT_INIT;
 * codec_peek_arm_a_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_PEEK_ARM_A_DEFAULT_INIT { \
    .header = 0x00u, \
}

typedef struct {
    uint8_t bytes[CODEC_PEEK_ARM_A_MAX_BYTES];
    size_t  len;
} codec_peek_arm_a_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_peek_arm_a_decode(sce_forge_cursor_t *cursor, codec_peek_arm_a_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_PEEK_ARM_A_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->header = raw[0];
    out->payload = raw[1];
    if (!sce_forge_cursor_advance(cursor, CODEC_PEEK_ARM_A_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_peek_arm_a_encoded_t codec_peek_arm_a_encode(const codec_peek_arm_a_t *self) {
    codec_peek_arm_a_encoded_t r;
    r.len = CODEC_PEEK_ARM_A_MIN_BYTES;
    r.bytes[0] = self->header;
    r.bytes[1] = self->payload;
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
static inline bool codec_peek_arm_a_kind(const codec_peek_arm_a_t *self) {
    return (self->header & 0x01) != 0;
}

static inline void codec_peek_arm_a_set_kind(codec_peek_arm_a_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x01);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_PEEK_ARM_A_H */
