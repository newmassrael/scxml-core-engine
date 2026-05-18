// SCE-MAP: codec_zenoh_declaration:54

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_DECLARATION_H
#define SCE_FORGE_CODEC_ZENOH_DECLARATION_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_decl_keyexpr.h"
#include "codec_zenoh_undecl_keyexpr.h"
#include "codec_zenoh_decl_subscriber.h"
#include "codec_zenoh_undecl_subscriber.h"
#include "codec_zenoh_decl_queryable.h"
#include "codec_zenoh_undecl_queryable.h"
#include "codec_zenoh_decl_token.h"
#include "codec_zenoh_undecl_token.h"
#include "codec_decl_final.h"

#define CODEC_ZENOH_DECLARATION_MIN_BYTES 1
#define CODEC_ZENOH_DECLARATION_MAX_BYTES 275

/* RFC §5.B variant primitive (B1-β): tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_KEYEXPR,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_KEYEXPR,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_SUBSCRIBER,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_SUBSCRIBER,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_QUERYABLE,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_QUERYABLE,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_TOKEN,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_TOKEN,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_DECL_FINAL,
    CODEC_ZENOH_DECLARATION_BODY_KIND_DEFAULT,
} codec_zenoh_declaration_body_kind_t;

typedef struct {
    codec_zenoh_declaration_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_zenoh_decl_keyexpr_t codec_zenoh_decl_keyexpr;
        codec_zenoh_undecl_keyexpr_t codec_zenoh_undecl_keyexpr;
        codec_zenoh_decl_subscriber_t codec_zenoh_decl_subscriber;
        codec_zenoh_undecl_subscriber_t codec_zenoh_undecl_subscriber;
        codec_zenoh_decl_queryable_t codec_zenoh_decl_queryable;
        codec_zenoh_undecl_queryable_t codec_zenoh_undecl_queryable;
        codec_zenoh_decl_token_t codec_zenoh_decl_token;
        codec_zenoh_undecl_token_t codec_zenoh_undecl_token;
        codec_decl_final_t codec_decl_final;
        codec_decl_final_t default_body;
    } arm;
} codec_zenoh_declaration_variant_t;

typedef struct {
    uint8_t header;
    codec_zenoh_declaration_variant_t body;
} codec_zenoh_declaration_t;

/* RFC variant-default-uniformity Atomic β-c11: designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_zenoh_declaration_t x = CODEC_ZENOH_DECLARATION_DEFAULT_INIT;
 * codec_zenoh_declaration_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_ZENOH_DECLARATION_DEFAULT_INIT { \
    .body = { \
        .kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_DECL_FINAL, \
        .arm = { .codec_decl_final = CODEC_DECL_FINAL_DEFAULT_INIT } \
    }, \
}

typedef struct {
    uint8_t bytes[CODEC_ZENOH_DECLARATION_MAX_BYTES];
    size_t  len;
} codec_zenoh_declaration_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_declaration_decode(sce_forge_cursor_t *cursor, codec_zenoh_declaration_t *out) {
    /* Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix). */
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_ZENOH_DECLARATION_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->header = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_ZENOH_DECLARATION_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch ((uint8_t)((out->header >> 0) & (uint8_t)0x1F)) {
        case 0:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_KEYEXPR;
            _arm_st = codec_zenoh_decl_keyexpr_decode(cursor, &out->body.arm.codec_zenoh_decl_keyexpr, out->header);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 1:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_KEYEXPR;
            _arm_st = codec_zenoh_undecl_keyexpr_decode(cursor, &out->body.arm.codec_zenoh_undecl_keyexpr);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 2:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_SUBSCRIBER;
            _arm_st = codec_zenoh_decl_subscriber_decode(cursor, &out->body.arm.codec_zenoh_decl_subscriber, out->header);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 3:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_SUBSCRIBER;
            _arm_st = codec_zenoh_undecl_subscriber_decode(cursor, &out->body.arm.codec_zenoh_undecl_subscriber, out->header);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 4:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_QUERYABLE;
            _arm_st = codec_zenoh_decl_queryable_decode(cursor, &out->body.arm.codec_zenoh_decl_queryable, out->header);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 5:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_QUERYABLE;
            _arm_st = codec_zenoh_undecl_queryable_decode(cursor, &out->body.arm.codec_zenoh_undecl_queryable, out->header);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 6:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_TOKEN;
            _arm_st = codec_zenoh_decl_token_decode(cursor, &out->body.arm.codec_zenoh_decl_token, out->header);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 7:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_TOKEN;
            _arm_st = codec_zenoh_undecl_token_decode(cursor, &out->body.arm.codec_zenoh_undecl_token, out->header);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 26:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_DECL_FINAL;
            _arm_st = codec_decl_final_decode(cursor, &out->body.arm.codec_decl_final);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_DEFAULT;
            out->body.default_tag = (uint8_t)((out->header >> 0) & (uint8_t)0x1F);
            _arm_st = codec_decl_final_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_declaration_encoded_t codec_zenoh_declaration_encode(const codec_zenoh_declaration_t *self) {
    codec_zenoh_declaration_encoded_t r;
    /* Encode fixed prefix (tag field bytes are part of the prefix).
     * The tag value is read from the struct field, NOT derived from
     * the body discriminant — keeping author-set tag / body in sync
     * is the caller's responsibility (v1 keeps the layout simple). */
    r.len = CODEC_ZENOH_DECLARATION_MIN_BYTES;
    r.bytes[0] = self->header;
    /* Append the active arm body's encoded bytes. */
    switch (self->body.kind) {
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_KEYEXPR: {
            codec_zenoh_decl_keyexpr_encoded_t _sub = codec_zenoh_decl_keyexpr_encode(&self->body.arm.codec_zenoh_decl_keyexpr, self->header);
            if (r.len + _sub.len <= CODEC_ZENOH_DECLARATION_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_KEYEXPR: {
            codec_zenoh_undecl_keyexpr_encoded_t _sub = codec_zenoh_undecl_keyexpr_encode(&self->body.arm.codec_zenoh_undecl_keyexpr);
            if (r.len + _sub.len <= CODEC_ZENOH_DECLARATION_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_SUBSCRIBER: {
            codec_zenoh_decl_subscriber_encoded_t _sub = codec_zenoh_decl_subscriber_encode(&self->body.arm.codec_zenoh_decl_subscriber, self->header);
            if (r.len + _sub.len <= CODEC_ZENOH_DECLARATION_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_SUBSCRIBER: {
            codec_zenoh_undecl_subscriber_encoded_t _sub = codec_zenoh_undecl_subscriber_encode(&self->body.arm.codec_zenoh_undecl_subscriber, self->header);
            if (r.len + _sub.len <= CODEC_ZENOH_DECLARATION_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_QUERYABLE: {
            codec_zenoh_decl_queryable_encoded_t _sub = codec_zenoh_decl_queryable_encode(&self->body.arm.codec_zenoh_decl_queryable, self->header);
            if (r.len + _sub.len <= CODEC_ZENOH_DECLARATION_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_QUERYABLE: {
            codec_zenoh_undecl_queryable_encoded_t _sub = codec_zenoh_undecl_queryable_encode(&self->body.arm.codec_zenoh_undecl_queryable, self->header);
            if (r.len + _sub.len <= CODEC_ZENOH_DECLARATION_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_TOKEN: {
            codec_zenoh_decl_token_encoded_t _sub = codec_zenoh_decl_token_encode(&self->body.arm.codec_zenoh_decl_token, self->header);
            if (r.len + _sub.len <= CODEC_ZENOH_DECLARATION_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_TOKEN: {
            codec_zenoh_undecl_token_encoded_t _sub = codec_zenoh_undecl_token_encode(&self->body.arm.codec_zenoh_undecl_token, self->header);
            if (r.len + _sub.len <= CODEC_ZENOH_DECLARATION_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_DECL_FINAL: {
            codec_decl_final_encoded_t _sub = codec_decl_final_encode(&self->body.arm.codec_decl_final);
            if (r.len + _sub.len <= CODEC_ZENOH_DECLARATION_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_DECLARATION_BODY_KIND_DEFAULT: {
            codec_decl_final_encoded_t _sub = codec_decl_final_encode(&self->body.arm.default_body);
            if (r.len + _sub.len <= CODEC_ZENOH_DECLARATION_MAX_BYTES) {
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
static inline uint8_t codec_zenoh_declaration_mid(const codec_zenoh_declaration_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x1F);
}

static inline void codec_zenoh_declaration_set_mid(codec_zenoh_declaration_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x1F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x1F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_declaration_n(const codec_zenoh_declaration_t *self) {
    return (self->header & 0x20) != 0;
}

static inline void codec_zenoh_declaration_set_n(codec_zenoh_declaration_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x20);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x20));
    }
}

static inline bool codec_zenoh_declaration_m(const codec_zenoh_declaration_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_zenoh_declaration_set_m(codec_zenoh_declaration_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_zenoh_declaration_z(const codec_zenoh_declaration_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_zenoh_declaration_set_z(codec_zenoh_declaration_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_DECLARATION_H */
