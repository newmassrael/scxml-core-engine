// SCE-MAP: codec_zenoh_msg_put:64

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_MSG_PUT_H
#define SCE_FORGE_CODEC_ZENOH_MSG_PUT_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_timestamp.h"
#include "codec_zenoh_encoding.h"
#include "codec_zenoh_ext_entry.h"

#define CODEC_ZENOH_MSG_PUT_MIN_BYTES 1
#define CODEC_ZENOH_MSG_PUT_MAX_BYTES 951

typedef struct {
    uint8_t header;
    /* RFC §5.B Y0c embed: nested codec_zenoh_timestamp_t struct (no length prefix on the wire) */
    codec_zenoh_timestamp_t timestamp;
    /* RFC §5.B Y0c embed: nested codec_zenoh_encoding_t struct (no length prefix on the wire) */
    codec_zenoh_encoding_t encoding;
    /* RFC §5.B B3 tlv-chain: fixed array of codec_zenoh_ext_entry_t entries (max-depth 4, on-overflow=reject) */
    codec_zenoh_ext_entry_t extensions[4];
    size_t  extensions_len;
    uint64_t payload_len;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="256") */
    uint8_t payload[256];
} codec_zenoh_msg_put_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_MSG_PUT_MAX_BYTES];
    size_t  len;
} codec_zenoh_msg_put_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_msg_put_decode(sce_forge_cursor_t *cursor, codec_zenoh_msg_put_t *out) {
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
        out->header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((out->header & 0x20) != 0) {
        sce_forge_codec_status_t _st = codec_zenoh_timestamp_decode(cursor, &out->timestamp);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
    }
    if ((out->header & 0x40) != 0) {
        sce_forge_codec_status_t _st = codec_zenoh_encoding_decode(cursor, &out->encoding);
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
    uint64_t payload_len;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &payload_len);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->payload_len = payload_len;
    {
        size_t _n = (size_t)out->payload_len;
        if (_n > 256) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->payload, raw, _n);
        out->payload_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_msg_put_encoded_t codec_zenoh_msg_put_encode(const codec_zenoh_msg_put_t *self) {
    codec_zenoh_msg_put_encoded_t r;
    /* RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    r.len = 0;
    r.bytes[r.len++] = self->header;
    if ((self->header & 0x20) != 0) {
        codec_zenoh_timestamp_encoded_t _sub = codec_zenoh_timestamp_encode(&self->timestamp);
        if (r.len + _sub.len <= sizeof(r.bytes)) {
            for (size_t _ej = 0; _ej < _sub.len; ++_ej) r.bytes[r.len + _ej] = _sub.bytes[_ej];
            r.len += _sub.len;
        }
    }
    if ((self->header & 0x40) != 0) {
        codec_zenoh_encoding_encoded_t _sub = codec_zenoh_encoding_encode(&self->encoding);
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
    {
        uint64_t _w = (uint64_t)(self->payload_len);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    for (size_t _bi = 0; _bi < self->payload_len && _bi < self->payload_len; ++_bi) r.bytes[r.len++] = self->payload[_bi];
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
static inline uint8_t codec_zenoh_msg_put_mid(const codec_zenoh_msg_put_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x1F);
}

static inline void codec_zenoh_msg_put_set_mid(codec_zenoh_msg_put_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x1F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x1F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_msg_put_t_flag(const codec_zenoh_msg_put_t *self) {
    return (self->header & 0x20) != 0;
}

static inline void codec_zenoh_msg_put_set_t(codec_zenoh_msg_put_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x20);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x20));
    }
}

static inline bool codec_zenoh_msg_put_e(const codec_zenoh_msg_put_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_zenoh_msg_put_set_e(codec_zenoh_msg_put_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_zenoh_msg_put_z(const codec_zenoh_msg_put_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_zenoh_msg_put_set_z(codec_zenoh_msg_put_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_MSG_PUT_H */
