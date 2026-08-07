// SCE-MAP: codec_zenoh_oam:56

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_OAM_H
#define SCE_FORGE_CODEC_ZENOH_OAM_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"
#include "codec_zenoh_ext_unit.h"
#include "codec_zenoh_ext_zint.h"
#include "codec_zenoh_ext_zbuf.h"

#define CODEC_ZENOH_OAM_MIN_BYTES 1
#define CODEC_ZENOH_OAM_MAX_BYTES 45

/* RFC §synth-5-B variant primitive: tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_ZENOH_OAM_BODY_KIND_CODEC_ZENOH_EXT_UNIT,
    CODEC_ZENOH_OAM_BODY_KIND_CODEC_ZENOH_EXT_ZINT,
    CODEC_ZENOH_OAM_BODY_KIND_CODEC_ZENOH_EXT_ZBUF,
    CODEC_ZENOH_OAM_BODY_KIND_DEFAULT,
} codec_zenoh_oam_body_kind_t;

typedef struct {
    codec_zenoh_oam_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_zenoh_ext_unit_t codec_zenoh_ext_unit;
        codec_zenoh_ext_zint_t codec_zenoh_ext_zint;
        codec_zenoh_ext_zbuf_t codec_zenoh_ext_zbuf;
        codec_zenoh_ext_unit_t default_body;
    } arm;
} codec_zenoh_oam_variant_t;

typedef struct {
    uint8_t header;
    uint16_t id;
    /* RFC §synth-5-B B3 tlv-chain: fixed array of codec_zenoh_ext_entry_t entries (max-depth 4, on-overflow=reject) */
    codec_zenoh_ext_entry_t extensions[4];
    size_t  extensions_len;
    codec_zenoh_oam_variant_t body;
} codec_zenoh_oam_t;

/* RFC variant-default-uniformity (C11): designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_zenoh_oam_t x = CODEC_ZENOH_OAM_DEFAULT_INIT;
 * codec_zenoh_oam_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_ZENOH_OAM_DEFAULT_INIT { \
    .header = 0x1fu, \
    .body = { \
        .kind = CODEC_ZENOH_OAM_BODY_KIND_CODEC_ZENOH_EXT_UNIT, \
        .arm = { .codec_zenoh_ext_unit = CODEC_ZENOH_EXT_UNIT_DEFAULT_INIT } \
    }, \
}

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_oam_decode(sce_forge_cursor_t *cursor, codec_zenoh_oam_t *out) {
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
    uint16_t id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u16(cursor, &id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->id = id;
    out->extensions_len = 0;
        if ((out->header & 0x80) != 0) {
            bool _more = false;
            for (size_t _i = 0; _i < 4; ++_i) {
                if (sce_forge_cursor_remaining(cursor) == 0) break;
                sce_forge_codec_status_t _st = codec_zenoh_ext_entry_decode(cursor, &out->extensions[out->extensions_len]);
                if (_st != SCE_FORGE_CODEC_OK) return _st;
                size_t _just = out->extensions_len;
                out->extensions_len++;
                _more = codec_zenoh_ext_entry_z(&out->extensions[_just]);
                if (!_more) break;
            }
            if (_more && sce_forge_cursor_remaining(cursor) == 0) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
            if (_more) return SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW;
        }
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch ((uint8_t)((out->header >> 5) & (uint8_t)0x03)) {
        case 0:
            out->body.kind = CODEC_ZENOH_OAM_BODY_KIND_CODEC_ZENOH_EXT_UNIT;
            _arm_st = codec_zenoh_ext_unit_decode(cursor, &out->body.arm.codec_zenoh_ext_unit);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 1:
            out->body.kind = CODEC_ZENOH_OAM_BODY_KIND_CODEC_ZENOH_EXT_ZINT;
            _arm_st = codec_zenoh_ext_zint_decode(cursor, &out->body.arm.codec_zenoh_ext_zint);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 2:
            out->body.kind = CODEC_ZENOH_OAM_BODY_KIND_CODEC_ZENOH_EXT_ZBUF;
            _arm_st = codec_zenoh_ext_zbuf_decode(cursor, &out->body.arm.codec_zenoh_ext_zbuf);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_ZENOH_OAM_BODY_KIND_DEFAULT;
            out->body.default_tag = (uint8_t)((out->header >> 5) & (uint8_t)0x03);
            _arm_st = codec_zenoh_ext_unit_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_OAM_MAX_BYTES bytes and use
 * `codec_zenoh_oam_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_oam_encode(const codec_zenoh_oam_t *self, sce_forge_writer_t *w) {
    /* RFC §synth-5-B peek-byte / streaming-prefix:
     * streaming prefix encode. Peek-byte mode: arm body's encode
     * prepends its own header byte (which the decoder peeked); no
     * separate tag byte here. Streaming-prefix mode (own-field):
     * carrier is part of the prefix fields and emits via the same
     * per-field path. */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->header));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u16(w, (uint16_t)(self->id)));
    if ((self->header & 0x80) != 0) {
        for (size_t _ti = 0; _ti < self->extensions_len; ++_ti) {
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_entry_encode(&self->extensions[_ti], w));
        }
    }
    /* Append the active arm body's encoded bytes via the same writer. */
    switch (self->body.kind) {
        case CODEC_ZENOH_OAM_BODY_KIND_CODEC_ZENOH_EXT_UNIT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_unit_encode(&self->body.arm.codec_zenoh_ext_unit, w));
            break;
        case CODEC_ZENOH_OAM_BODY_KIND_CODEC_ZENOH_EXT_ZINT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_zint_encode(&self->body.arm.codec_zenoh_ext_zint, w));
            break;
        case CODEC_ZENOH_OAM_BODY_KIND_CODEC_ZENOH_EXT_ZBUF:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_zbuf_encode(&self->body.arm.codec_zenoh_ext_zbuf, w));
            break;
        case CODEC_ZENOH_OAM_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_unit_encode(&self->body.arm.default_body, w));
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_OAM_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_OAM_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_oam_encode_to_buf(const codec_zenoh_oam_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_oam_encode(self, &_w);
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
static inline uint8_t codec_zenoh_oam_mid(const codec_zenoh_oam_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x1F);
}

static inline void codec_zenoh_oam_set_mid(codec_zenoh_oam_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x1F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x1F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline uint8_t codec_zenoh_oam_enc(const codec_zenoh_oam_t *self) {
    return (uint8_t)((self->header >> 5) & (uint8_t)0x03);
}

static inline void codec_zenoh_oam_set_enc(codec_zenoh_oam_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x03 << 5);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x03) << 5);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_oam_z(const codec_zenoh_oam_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_zenoh_oam_set_z(codec_zenoh_oam_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_OAM_H */
