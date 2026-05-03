/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_VARIANT_PEEK_BASIC_H
#define SCE_FORGE_CODEC_VARIANT_PEEK_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_peek_arm_a.h"
#include "codec_peek_arm_b.h"

#define CODEC_VARIANT_PEEK_BASIC_MIN_BYTES 0
#define CODEC_VARIANT_PEEK_BASIC_MAX_BYTES 3

/* RFC §5.B variant primitive (B1-β): tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_VARIANT_PEEK_BASIC_BODY_KIND_CODEC_PEEK_ARM_A,
    CODEC_VARIANT_PEEK_BASIC_BODY_KIND_CODEC_PEEK_ARM_B,
} codec_variant_peek_basic_body_kind_t;

typedef struct {
    codec_variant_peek_basic_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_peek_arm_a_t codec_peek_arm_a;
        codec_peek_arm_b_t codec_peek_arm_b;
    } arm;
} codec_variant_peek_basic_variant_t;

typedef struct {
    codec_variant_peek_basic_variant_t body;
} codec_variant_peek_basic_t;

typedef struct {
    uint8_t bytes[CODEC_VARIANT_PEEK_BASIC_MAX_BYTES];
    size_t  len;
} codec_variant_peek_basic_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_variant_peek_basic_decode(sce_forge_cursor_t *cursor, codec_variant_peek_basic_t *out) {
    /* RFC §5.B Y3 atomic 2b-ii peek-byte — peek-byte mode: streaming
     * prefix decode (variable-length supported), then peek the cursor's
     * next byte for variant tag without advancing. Arm body decoder
     * reads peeked byte as own header. */
    const uint8_t *_peek_raw = sce_forge_cursor_peek(cursor, 1);
    if (_peek_raw == NULL) {
        return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    const uint8_t _peek = _peek_raw[0];
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch ((uint8_t)((_peek >> 0) & (uint8_t)0x01)) {
        case 0:
            out->body.kind = CODEC_VARIANT_PEEK_BASIC_BODY_KIND_CODEC_PEEK_ARM_A;
            _arm_st = codec_peek_arm_a_decode(cursor, &out->body.arm.codec_peek_arm_a);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 1:
            out->body.kind = CODEC_VARIANT_PEEK_BASIC_BODY_KIND_CODEC_PEEK_ARM_B;
            _arm_st = codec_peek_arm_b_decode(cursor, &out->body.arm.codec_peek_arm_b);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            /* codec/variant-arm-unreachable rejected this case at parse time. */
            return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_variant_peek_basic_encoded_t codec_variant_peek_basic_encode(const codec_variant_peek_basic_t *self) {
    codec_variant_peek_basic_encoded_t r;
    /* RFC §5.B Y3 atomic 2b-ii peek-byte — peek-byte mode: streaming
     * prefix encode. Arm body's encode prepends its own header byte
     * (which the decoder peeked); no separate tag byte here. */
    r.len = 0;
    /* Append the active arm body's encoded bytes. */
    switch (self->body.kind) {
        case CODEC_VARIANT_PEEK_BASIC_BODY_KIND_CODEC_PEEK_ARM_A: {
            codec_peek_arm_a_encoded_t _sub = codec_peek_arm_a_encode(&self->body.arm.codec_peek_arm_a);
            if (r.len + _sub.len <= CODEC_VARIANT_PEEK_BASIC_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_VARIANT_PEEK_BASIC_BODY_KIND_CODEC_PEEK_ARM_B: {
            codec_peek_arm_b_encoded_t _sub = codec_peek_arm_b_encode(&self->body.arm.codec_peek_arm_b);
            if (r.len + _sub.len <= CODEC_VARIANT_PEEK_BASIC_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_VARIANT_PEEK_BASIC_H */
