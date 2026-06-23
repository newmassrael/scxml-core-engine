// SCE-MAP: codec_zenoh_response:75

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_RESPONSE_H
#define SCE_FORGE_CODEC_ZENOH_RESPONSE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"
#include "codec_zenoh_reply.h"
#include "codec_zenoh_err.h"

#define CODEC_ZENOH_RESPONSE_MIN_BYTES 1
#define CODEC_ZENOH_RESPONSE_MAX_BYTES 970

/* RFC §synth-5-B variant primitive: tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_REPLY,
    CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_ERR,
    CODEC_ZENOH_RESPONSE_BODY_KIND_DEFAULT,
} codec_zenoh_response_body_kind_t;

typedef struct {
    codec_zenoh_response_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_zenoh_reply_t codec_zenoh_reply;
        codec_zenoh_err_t codec_zenoh_err;
        codec_zenoh_reply_t default_body;
    } arm;
} codec_zenoh_response_variant_t;

typedef struct {
    uint8_t header;
    uint64_t request_id;
    uint32_t key_id;
    uint64_t suffix_len;
    /* RFC §synth-5-B sce:type="string" payload (sce:max-size="256").
     * `char[N] + size_t len` parallels the bytes pair (uint8_t[N] + len)
     * but the host-language type signals UTF-8 text storage; the C
     * string is NOT NUL-terminated — payloads of exactly `max_size`
     * bytes are valid wire input. */
    char    suffix[256];
    /* RFC §synth-5-B B3 tlv-chain: fixed array of codec_zenoh_ext_entry_t entries (max-depth 4, on-overflow=reject) */
    codec_zenoh_ext_entry_t extensions[4];
    size_t  extensions_len;
    codec_zenoh_response_variant_t body;
} codec_zenoh_response_t;

/* RFC variant-default-uniformity (C11): designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_zenoh_response_t x = CODEC_ZENOH_RESPONSE_DEFAULT_INIT;
 * codec_zenoh_response_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_ZENOH_RESPONSE_DEFAULT_INIT { \
    .header = 0x1bu, \
    .body = { \
        .kind = CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_REPLY, \
        .arm = { .codec_zenoh_reply = CODEC_ZENOH_REPLY_DEFAULT_INIT } \
    }, \
}

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_response_decode(sce_forge_cursor_t *cursor, codec_zenoh_response_t *out) {
    /* RFC §synth-5-B peek-byte / streaming-prefix:
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
    uint64_t request_id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &request_id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->request_id = request_id;
    uint32_t key_id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u32(cursor, &key_id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->key_id = key_id;
    if ((out->header & 0x20) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        out->suffix_len = _v;
    } else {
        out->suffix_len = 0;
    }
    if ((out->header & 0x20) != 0) {
        size_t _n = (size_t)out->suffix_len;
        if (_n > 256) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        if (!sce_forge_is_valid_utf8(raw, _n)) return SCE_FORGE_CODEC_INVALID_UTF8;
        memcpy(out->suffix, raw, _n);
        out->suffix_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->suffix_len = 0;
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
        case 4:
            out->body.kind = CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_REPLY;
            _arm_st = codec_zenoh_reply_decode(cursor, &out->body.arm.codec_zenoh_reply);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 5:
            out->body.kind = CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_ERR;
            _arm_st = codec_zenoh_err_decode(cursor, &out->body.arm.codec_zenoh_err);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_ZENOH_RESPONSE_BODY_KIND_DEFAULT;
            out->body.default_tag = (uint8_t)((_peek >> 0) & (uint8_t)0x1F);
            _arm_st = codec_zenoh_reply_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_RESPONSE_MAX_BYTES bytes and use
 * `codec_zenoh_response_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_response_encode(const codec_zenoh_response_t *self, sce_forge_writer_t *w) {
    /* RFC §synth-5-B peek-byte / streaming-prefix:
     * streaming prefix encode. Peek-byte mode: arm body's encode
     * prepends its own header byte (which the decoder peeked); no
     * separate tag byte here. Streaming-prefix mode (own-field):
     * carrier is part of the prefix fields and emits via the same
     * per-field path. */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->header));
    {
        uint64_t _vle = (uint64_t)(self->request_id);
        uint32_t _vn = 0u;
        while (_vle >= 0x80u && _vn < 8u) {
            SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_vle & 0x7Fu) | 0x80u)));
            _vle >>= 7;
            _vn++;
        }
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)_vle));
    }
    {
        uint64_t _vle = (uint64_t)(self->key_id);
        uint32_t _vn = 0u;
        while (_vle >= 0x80u && _vn < 4u) {
            SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_vle & 0x7Fu) | 0x80u)));
            _vle >>= 7;
            _vn++;
        }
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)_vle));
    }
    if ((self->header & 0x20) != 0) {
    {
        uint64_t _vle = (uint64_t)(self->suffix_len);
        uint32_t _vn = 0u;
        while (_vle >= 0x80u && _vn < 8u) {
            SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_vle & 0x7Fu) | 0x80u)));
            _vle >>= 7;
            _vn++;
        }
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)_vle));
    }
    }
    if ((self->header & 0x20) != 0) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, (const uint8_t*)self->suffix, self->suffix_len));
    }
    if ((self->header & 0x80) != 0) {
        for (size_t _ti = 0; _ti < self->extensions_len; ++_ti) {
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_entry_encode(&self->extensions[_ti], w));
        }
    }
    /* Append the active arm body's encoded bytes via the same writer. */
    switch (self->body.kind) {
        case CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_REPLY:
            SCE_FORGE_TRY_WRITE(codec_zenoh_reply_encode(&self->body.arm.codec_zenoh_reply, w));
            break;
        case CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_ERR:
            SCE_FORGE_TRY_WRITE(codec_zenoh_err_encode(&self->body.arm.codec_zenoh_err, w));
            break;
        case CODEC_ZENOH_RESPONSE_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_reply_encode(&self->body.arm.default_body, w));
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_RESPONSE_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_RESPONSE_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_response_encode_to_buf(const codec_zenoh_response_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_response_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

/* RFC §synth-5-B flags primitive: per-bit-range accessors over
 * the carrier field. Single-bit (width=1) reads as bool; multi-bit
 * (width>=2) reads as the smallest unsigned C11 integer type that fits
 * (uint8_t / uint16_t / uint32_t / uint64_t). Setters mask + shift on
 * the way in so out-of-range callers can't corrupt sibling bits. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline uint8_t codec_zenoh_response_mid(const codec_zenoh_response_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x1F);
}

static inline void codec_zenoh_response_set_mid(codec_zenoh_response_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x1F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x1F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_response_n(const codec_zenoh_response_t *self) {
    return (self->header & 0x20) != 0;
}

static inline void codec_zenoh_response_set_n(codec_zenoh_response_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x20);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x20));
    }
}

static inline bool codec_zenoh_response_m(const codec_zenoh_response_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_zenoh_response_set_m(codec_zenoh_response_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_zenoh_response_z(const codec_zenoh_response_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_zenoh_response_set_z(codec_zenoh_response_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_RESPONSE_H */
