// SCE-MAP: codec_transport_envelope:69

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_TRANSPORT_ENVELOPE_H
#define SCE_FORGE_CODEC_TRANSPORT_ENVELOPE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_init_body.h"
#include "codec_zenoh_open_body.h"
#include "codec_zenoh_close.h"
#include "codec_zenoh_keep_alive.h"
#include "codec_zenoh_frame.h"
#include "codec_zenoh_fragment.h"
#include "codec_zenoh_join.h"

#define CODEC_TRANSPORT_ENVELOPE_MIN_BYTES 1
#define CODEC_TRANSPORT_ENVELOPE_MAX_BYTES 65547

/* RFC §5.B variant primitive (B1-β): tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_INIT_BODY,
    CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_OPEN_BODY,
    CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_CLOSE,
    CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_KEEP_ALIVE,
    CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_FRAME,
    CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_FRAGMENT,
    CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_JOIN,
    CODEC_TRANSPORT_ENVELOPE_BODY_KIND_DEFAULT,
} codec_transport_envelope_body_kind_t;

typedef struct {
    codec_transport_envelope_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_zenoh_init_body_t codec_zenoh_init_body;
        codec_zenoh_open_body_t codec_zenoh_open_body;
        codec_zenoh_close_t codec_zenoh_close;
        codec_zenoh_keep_alive_t codec_zenoh_keep_alive;
        codec_zenoh_frame_t codec_zenoh_frame;
        codec_zenoh_fragment_t codec_zenoh_fragment;
        codec_zenoh_join_t codec_zenoh_join;
        codec_zenoh_close_t default_body;
    } arm;
} codec_transport_envelope_variant_t;

typedef struct {
    uint8_t header;
    codec_transport_envelope_variant_t body;
} codec_transport_envelope_t;

/* RFC variant-default-uniformity Atomic β-c11: designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_transport_envelope_t x = CODEC_TRANSPORT_ENVELOPE_DEFAULT_INIT;
 * codec_transport_envelope_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_TRANSPORT_ENVELOPE_DEFAULT_INIT { \
    .body = { \
        .kind = CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_CLOSE, \
        .arm = { .codec_zenoh_close = CODEC_ZENOH_CLOSE_DEFAULT_INIT } \
    }, \
}

typedef struct {
    uint8_t bytes[CODEC_TRANSPORT_ENVELOPE_MAX_BYTES];
    size_t  len;
} codec_transport_envelope_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_transport_envelope_decode(sce_forge_cursor_t *cursor, codec_transport_envelope_t *out) {
    /* Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix). */
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_TRANSPORT_ENVELOPE_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->header = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_TRANSPORT_ENVELOPE_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch ((uint8_t)((out->header >> 0) & (uint8_t)0x1F)) {
        case 1:
            out->body.kind = CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_INIT_BODY;
            _arm_st = codec_zenoh_init_body_decode(cursor, &out->body.arm.codec_zenoh_init_body, out->header);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 2:
            out->body.kind = CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_OPEN_BODY;
            _arm_st = codec_zenoh_open_body_decode(cursor, &out->body.arm.codec_zenoh_open_body, out->header);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 3:
            out->body.kind = CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_CLOSE;
            _arm_st = codec_zenoh_close_decode(cursor, &out->body.arm.codec_zenoh_close);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 4:
            out->body.kind = CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_KEEP_ALIVE;
            _arm_st = codec_zenoh_keep_alive_decode(cursor, &out->body.arm.codec_zenoh_keep_alive);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 5:
            out->body.kind = CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_FRAME;
            _arm_st = codec_zenoh_frame_decode(cursor, &out->body.arm.codec_zenoh_frame);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 6:
            out->body.kind = CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_FRAGMENT;
            _arm_st = codec_zenoh_fragment_decode(cursor, &out->body.arm.codec_zenoh_fragment);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 7:
            out->body.kind = CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_JOIN;
            _arm_st = codec_zenoh_join_decode(cursor, &out->body.arm.codec_zenoh_join, out->header);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_TRANSPORT_ENVELOPE_BODY_KIND_DEFAULT;
            out->body.default_tag = (uint8_t)((out->header >> 0) & (uint8_t)0x1F);
            _arm_st = codec_zenoh_close_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_transport_envelope_encoded_t codec_transport_envelope_encode(const codec_transport_envelope_t *self) {
    codec_transport_envelope_encoded_t r;
    /* Encode fixed prefix (tag field bytes are part of the prefix).
     * The tag value is read from the struct field, NOT derived from
     * the body discriminant — keeping author-set tag / body in sync
     * is the caller's responsibility (v1 keeps the layout simple). */
    r.len = CODEC_TRANSPORT_ENVELOPE_MIN_BYTES;
    r.bytes[0] = self->header;
    /* Append the active arm body's encoded bytes. */
    switch (self->body.kind) {
        case CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_INIT_BODY: {
            codec_zenoh_init_body_encoded_t _sub = codec_zenoh_init_body_encode(&self->body.arm.codec_zenoh_init_body, self->header);
            if (r.len + _sub.len <= CODEC_TRANSPORT_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_OPEN_BODY: {
            codec_zenoh_open_body_encoded_t _sub = codec_zenoh_open_body_encode(&self->body.arm.codec_zenoh_open_body, self->header);
            if (r.len + _sub.len <= CODEC_TRANSPORT_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_CLOSE: {
            codec_zenoh_close_encoded_t _sub = codec_zenoh_close_encode(&self->body.arm.codec_zenoh_close);
            if (r.len + _sub.len <= CODEC_TRANSPORT_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_KEEP_ALIVE: {
            codec_zenoh_keep_alive_encoded_t _sub = codec_zenoh_keep_alive_encode(&self->body.arm.codec_zenoh_keep_alive);
            if (r.len + _sub.len <= CODEC_TRANSPORT_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_FRAME: {
            codec_zenoh_frame_encoded_t _sub = codec_zenoh_frame_encode(&self->body.arm.codec_zenoh_frame);
            if (r.len + _sub.len <= CODEC_TRANSPORT_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_FRAGMENT: {
            codec_zenoh_fragment_encoded_t _sub = codec_zenoh_fragment_encode(&self->body.arm.codec_zenoh_fragment);
            if (r.len + _sub.len <= CODEC_TRANSPORT_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_TRANSPORT_ENVELOPE_BODY_KIND_CODEC_ZENOH_JOIN: {
            codec_zenoh_join_encoded_t _sub = codec_zenoh_join_encode(&self->body.arm.codec_zenoh_join, self->header);
            if (r.len + _sub.len <= CODEC_TRANSPORT_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_TRANSPORT_ENVELOPE_BODY_KIND_DEFAULT: {
            codec_zenoh_close_encoded_t _sub = codec_zenoh_close_encode(&self->body.arm.default_body);
            if (r.len + _sub.len <= CODEC_TRANSPORT_ENVELOPE_MAX_BYTES) {
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
static inline uint8_t codec_transport_envelope_mid(const codec_transport_envelope_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x1F);
}

static inline void codec_transport_envelope_set_mid(codec_transport_envelope_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x1F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x1F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_transport_envelope_a(const codec_transport_envelope_t *self) {
    return (self->header & 0x20) != 0;
}

static inline void codec_transport_envelope_set_a(codec_transport_envelope_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x20);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x20));
    }
}

static inline bool codec_transport_envelope_s(const codec_transport_envelope_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_transport_envelope_set_s(codec_transport_envelope_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_transport_envelope_z(const codec_transport_envelope_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_transport_envelope_set_z(codec_transport_envelope_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_TRANSPORT_ENVELOPE_H */
