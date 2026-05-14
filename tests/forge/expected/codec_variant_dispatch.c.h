// SCE-MAP: codec_variant_dispatch:8

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_VARIANT_DISPATCH_H
#define SCE_FORGE_CODEC_VARIANT_DISPATCH_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_variant_session_open.h"
#include "codec_variant_session_close.h"

#define CODEC_VARIANT_DISPATCH_MIN_BYTES 1
#define CODEC_VARIANT_DISPATCH_MAX_BYTES 3

/* RFC §5.B variant primitive (B1-β): tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_OPEN,
    CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_CLOSE,
    CODEC_VARIANT_DISPATCH_BODY_KIND_DEFAULT,
} codec_variant_dispatch_body_kind_t;

typedef struct {
    codec_variant_dispatch_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_variant_session_open_t codec_variant_session_open;
        codec_variant_session_close_t codec_variant_session_close;
        codec_variant_session_close_t default_body;
    } arm;
} codec_variant_dispatch_variant_t;

typedef struct {
    uint8_t msg_id;
    codec_variant_dispatch_variant_t body;
} codec_variant_dispatch_t;

typedef struct {
    uint8_t bytes[CODEC_VARIANT_DISPATCH_MAX_BYTES];
    size_t  len;
} codec_variant_dispatch_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_variant_dispatch_decode(sce_forge_cursor_t *cursor, codec_variant_dispatch_t *out) {
    /* Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix). */
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_VARIANT_DISPATCH_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->msg_id = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_VARIANT_DISPATCH_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch (out->msg_id) {
        case 1:
            out->body.kind = CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_OPEN;
            _arm_st = codec_variant_session_open_decode(cursor, &out->body.arm.codec_variant_session_open);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 2:
            out->body.kind = CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_CLOSE;
            _arm_st = codec_variant_session_close_decode(cursor, &out->body.arm.codec_variant_session_close);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_VARIANT_DISPATCH_BODY_KIND_DEFAULT;
            out->body.default_tag = out->msg_id;
            _arm_st = codec_variant_session_close_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_variant_dispatch_encoded_t codec_variant_dispatch_encode(const codec_variant_dispatch_t *self) {
    codec_variant_dispatch_encoded_t r;
    /* Encode fixed prefix (tag field bytes are part of the prefix).
     * The tag value is read from the struct field, NOT derived from
     * the body discriminant — keeping author-set tag / body in sync
     * is the caller's responsibility (v1 keeps the layout simple). */
    r.len = CODEC_VARIANT_DISPATCH_MIN_BYTES;
    r.bytes[0] = self->msg_id;
    /* Append the active arm body's encoded bytes. */
    switch (self->body.kind) {
        case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_OPEN: {
            codec_variant_session_open_encoded_t _sub = codec_variant_session_open_encode(&self->body.arm.codec_variant_session_open);
            if (r.len + _sub.len <= CODEC_VARIANT_DISPATCH_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_CLOSE: {
            codec_variant_session_close_encoded_t _sub = codec_variant_session_close_encode(&self->body.arm.codec_variant_session_close);
            if (r.len + _sub.len <= CODEC_VARIANT_DISPATCH_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_VARIANT_DISPATCH_BODY_KIND_DEFAULT: {
            codec_variant_session_close_encoded_t _sub = codec_variant_session_close_encode(&self->body.arm.default_body);
            if (r.len + _sub.len <= CODEC_VARIANT_DISPATCH_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_VARIANT_DISPATCH_H */
