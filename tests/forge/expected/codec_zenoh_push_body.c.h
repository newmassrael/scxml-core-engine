// SCE-MAP: codec_zenoh_push_body:30

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_PUSH_BODY_H
#define SCE_FORGE_CODEC_ZENOH_PUSH_BODY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_put.h"
#include "codec_zenoh_del.h"

#define CODEC_ZENOH_PUSH_BODY_MIN_BYTES 1
#define CODEC_ZENOH_PUSH_BODY_MAX_BYTES 2

/* RFC §5.B variant primitive (B1-β): tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_ZENOH_PUSH_BODY_BODY_KIND_CODEC_ZENOH_PUT,
    CODEC_ZENOH_PUSH_BODY_BODY_KIND_CODEC_ZENOH_DEL,
    CODEC_ZENOH_PUSH_BODY_BODY_KIND_DEFAULT,
} codec_zenoh_push_body_body_kind_t;

typedef struct {
    codec_zenoh_push_body_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_zenoh_put_t codec_zenoh_put;
        codec_zenoh_del_t codec_zenoh_del;
        codec_zenoh_put_t default_body;
    } arm;
} codec_zenoh_push_body_variant_t;

typedef struct {
    uint8_t header;
    codec_zenoh_push_body_variant_t body;
} codec_zenoh_push_body_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_PUSH_BODY_MAX_BYTES];
    size_t  len;
} codec_zenoh_push_body_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_push_body_decode(sce_forge_cursor_t *cursor, codec_zenoh_push_body_t *out) {
    /* Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix). */
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_ZENOH_PUSH_BODY_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->header = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_ZENOH_PUSH_BODY_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch ((uint8_t)((out->header >> 0) & (uint8_t)0x1F)) {
        case 1:
            out->body.kind = CODEC_ZENOH_PUSH_BODY_BODY_KIND_CODEC_ZENOH_PUT;
            _arm_st = codec_zenoh_put_decode(cursor, &out->body.arm.codec_zenoh_put);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 2:
            out->body.kind = CODEC_ZENOH_PUSH_BODY_BODY_KIND_CODEC_ZENOH_DEL;
            _arm_st = codec_zenoh_del_decode(cursor, &out->body.arm.codec_zenoh_del);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_ZENOH_PUSH_BODY_BODY_KIND_DEFAULT;
            out->body.default_tag = (uint8_t)((out->header >> 0) & (uint8_t)0x1F);
            _arm_st = codec_zenoh_put_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_push_body_encoded_t codec_zenoh_push_body_encode(const codec_zenoh_push_body_t *self) {
    codec_zenoh_push_body_encoded_t r;
    /* Encode fixed prefix (tag field bytes are part of the prefix).
     * The tag value is read from the struct field, NOT derived from
     * the body discriminant — keeping author-set tag / body in sync
     * is the caller's responsibility (v1 keeps the layout simple). */
    r.len = CODEC_ZENOH_PUSH_BODY_MIN_BYTES;
    r.bytes[0] = self->header;
    /* Append the active arm body's encoded bytes. */
    switch (self->body.kind) {
        case CODEC_ZENOH_PUSH_BODY_BODY_KIND_CODEC_ZENOH_PUT: {
            codec_zenoh_put_encoded_t _sub = codec_zenoh_put_encode(&self->body.arm.codec_zenoh_put);
            if (r.len + _sub.len <= CODEC_ZENOH_PUSH_BODY_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_PUSH_BODY_BODY_KIND_CODEC_ZENOH_DEL: {
            codec_zenoh_del_encoded_t _sub = codec_zenoh_del_encode(&self->body.arm.codec_zenoh_del);
            if (r.len + _sub.len <= CODEC_ZENOH_PUSH_BODY_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_PUSH_BODY_BODY_KIND_DEFAULT: {
            codec_zenoh_put_encoded_t _sub = codec_zenoh_put_encode(&self->body.arm.default_body);
            if (r.len + _sub.len <= CODEC_ZENOH_PUSH_BODY_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
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
static inline uint8_t codec_zenoh_push_body_mid(const codec_zenoh_push_body_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x1F);
}

static inline void codec_zenoh_push_body_set_mid(codec_zenoh_push_body_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x1F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x1F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline uint8_t codec_zenoh_push_body_rest(const codec_zenoh_push_body_t *self) {
    return (uint8_t)((self->header >> 5) & (uint8_t)0x07);
}

static inline void codec_zenoh_push_body_set_rest(codec_zenoh_push_body_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x07 << 5);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x07) << 5);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


#endif  /* SCE_FORGE_CODEC_ZENOH_PUSH_BODY_H */
