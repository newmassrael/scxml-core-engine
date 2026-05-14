// SCE-MAP: codec_zenoh_request:73

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_REQUEST_H
#define SCE_FORGE_CODEC_ZENOH_REQUEST_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_wireexpr.h"
#include "codec_zenoh_ext_entry.h"
#include "codec_zenoh_msg_put.h"
#include "codec_zenoh_msg_del.h"
#include "codec_zenoh_query.h"

#define CODEC_ZENOH_REQUEST_MIN_BYTES 1
#define CODEC_ZENOH_REQUEST_MAX_BYTES 1218

/* RFC §5.B variant primitive (B1-β): tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_ZENOH_REQUEST_BODY_KIND_CODEC_ZENOH_MSG_PUT,
    CODEC_ZENOH_REQUEST_BODY_KIND_CODEC_ZENOH_MSG_DEL,
    CODEC_ZENOH_REQUEST_BODY_KIND_CODEC_ZENOH_QUERY,
    CODEC_ZENOH_REQUEST_BODY_KIND_DEFAULT,
} codec_zenoh_request_body_kind_t;

typedef struct {
    codec_zenoh_request_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_zenoh_msg_put_t codec_zenoh_msg_put;
        codec_zenoh_msg_del_t codec_zenoh_msg_del;
        codec_zenoh_query_t codec_zenoh_query;
        codec_zenoh_query_t default_body;
    } arm;
} codec_zenoh_request_variant_t;

typedef struct {
    uint8_t header;
    uint64_t rid;
    /* RFC §5.B Y0c embed: nested codec_zenoh_wireexpr_t struct (no length prefix on the wire) */
    codec_zenoh_wireexpr_t keyexpr;
    /* RFC §5.B B3 tlv-chain: fixed array of codec_zenoh_ext_entry_t entries (max-depth 4, on-overflow=reject) */
    codec_zenoh_ext_entry_t extensions[4];
    size_t  extensions_len;
    codec_zenoh_request_variant_t body;
} codec_zenoh_request_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_REQUEST_MAX_BYTES];
    size_t  len;
} codec_zenoh_request_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_request_decode(sce_forge_cursor_t *cursor, codec_zenoh_request_t *out) {
    /* RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
     * streaming prefix decode (variable-length fields supported via
     * per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
     * mode additionally peeks the cursor's next byte for variant tag
     * without advancing — arm body decoder reads it as own header. */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint64_t rid;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &rid);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->rid = rid;
    {
        sce_forge_codec_status_t _st = codec_zenoh_wireexpr_decode(cursor, &out->keyexpr, out->header);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
    }
    out->extensions_len = 0;
        if ((out->header & 0x80) != 0) {
            for (size_t _i = 0; _i < 4; ++_i) {
                if (sce_forge_cursor_remaining(cursor) == 0) break;
                sce_forge_codec_status_t _st = codec_zenoh_ext_entry_decode(cursor, &out->extensions[out->extensions_len]);
                if (_st != SCE_FORGE_CODEC_OK) return _st;
                size_t _just = out->extensions_len;
                out->extensions_len++;
                if (!codec_zenoh_ext_entry_z(&out->extensions[_just])) break;
            }
        }
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
    switch ((uint8_t)((_peek >> 0) & (uint8_t)0x1F)) {
        case 1:
            out->body.kind = CODEC_ZENOH_REQUEST_BODY_KIND_CODEC_ZENOH_MSG_PUT;
            _arm_st = codec_zenoh_msg_put_decode(cursor, &out->body.arm.codec_zenoh_msg_put);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 2:
            out->body.kind = CODEC_ZENOH_REQUEST_BODY_KIND_CODEC_ZENOH_MSG_DEL;
            _arm_st = codec_zenoh_msg_del_decode(cursor, &out->body.arm.codec_zenoh_msg_del);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 3:
            out->body.kind = CODEC_ZENOH_REQUEST_BODY_KIND_CODEC_ZENOH_QUERY;
            _arm_st = codec_zenoh_query_decode(cursor, &out->body.arm.codec_zenoh_query);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_ZENOH_REQUEST_BODY_KIND_DEFAULT;
            out->body.default_tag = (uint8_t)((_peek >> 0) & (uint8_t)0x1F);
            _arm_st = codec_zenoh_query_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_request_encoded_t codec_zenoh_request_encode(const codec_zenoh_request_t *self) {
    codec_zenoh_request_encoded_t r;
    /* RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
     * streaming prefix encode. Peek-byte mode: arm body's encode
     * prepends its own header byte (which the decoder peeked); no
     * separate tag byte here. Streaming-prefix mode (own-field):
     * carrier is part of the prefix fields and emits via the same
     * per-field path. */
    r.len = 0;
    r.bytes[r.len++] = self->header;
    {
        uint64_t _w = (uint64_t)(self->rid);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    {
        codec_zenoh_wireexpr_encoded_t _sub = codec_zenoh_wireexpr_encode(&self->keyexpr, self->header);
        if (r.len + _sub.len <= sizeof(r.bytes)) {
            for (size_t _ej = 0; _ej < _sub.len; ++_ej) r.bytes[r.len + _ej] = _sub.bytes[_ej];
            r.len += _sub.len;
        }
    }
    if ((self->header & 0x80) != 0) {
        for (size_t _ti = 0; _ti < self->extensions_len; ++_ti) {
            codec_zenoh_ext_entry_encoded_t _sub = codec_zenoh_ext_entry_encode(&self->extensions[_ti]);
            if (r.len + _sub.len <= sizeof(r.bytes)) {
                for (size_t _tj = 0; _tj < _sub.len; ++_tj) r.bytes[r.len + _tj] = _sub.bytes[_tj];
                r.len += _sub.len;
            }
        }
    }
    /* Append the active arm body's encoded bytes. */
    switch (self->body.kind) {
        case CODEC_ZENOH_REQUEST_BODY_KIND_CODEC_ZENOH_MSG_PUT: {
            codec_zenoh_msg_put_encoded_t _sub = codec_zenoh_msg_put_encode(&self->body.arm.codec_zenoh_msg_put);
            if (r.len + _sub.len <= CODEC_ZENOH_REQUEST_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_REQUEST_BODY_KIND_CODEC_ZENOH_MSG_DEL: {
            codec_zenoh_msg_del_encoded_t _sub = codec_zenoh_msg_del_encode(&self->body.arm.codec_zenoh_msg_del);
            if (r.len + _sub.len <= CODEC_ZENOH_REQUEST_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_REQUEST_BODY_KIND_CODEC_ZENOH_QUERY: {
            codec_zenoh_query_encoded_t _sub = codec_zenoh_query_encode(&self->body.arm.codec_zenoh_query);
            if (r.len + _sub.len <= CODEC_ZENOH_REQUEST_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_REQUEST_BODY_KIND_DEFAULT: {
            codec_zenoh_query_encoded_t _sub = codec_zenoh_query_encode(&self->body.arm.default_body);
            if (r.len + _sub.len <= CODEC_ZENOH_REQUEST_MAX_BYTES) {
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
static inline uint8_t codec_zenoh_request_mid(const codec_zenoh_request_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x1F);
}

static inline void codec_zenoh_request_set_mid(codec_zenoh_request_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x1F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x1F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_request_n(const codec_zenoh_request_t *self) {
    return (self->header & 0x20) != 0;
}

static inline void codec_zenoh_request_set_n(codec_zenoh_request_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x20);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x20));
    }
}

static inline bool codec_zenoh_request_m(const codec_zenoh_request_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_zenoh_request_set_m(codec_zenoh_request_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_zenoh_request_z(const codec_zenoh_request_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_zenoh_request_set_z(codec_zenoh_request_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_REQUEST_H */
