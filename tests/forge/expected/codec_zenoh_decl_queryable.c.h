// SCE-MAP: codec_zenoh_decl_queryable:46

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_DECL_QUERYABLE_H
#define SCE_FORGE_CODEC_ZENOH_DECL_QUERYABLE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_wireexpr.h"

#define CODEC_ZENOH_DECL_QUERYABLE_MIN_BYTES 3
#define CODEC_ZENOH_DECL_QUERYABLE_MAX_BYTES 274

typedef struct {
    uint32_t id;
    /* RFC §5.B Y0c embed: nested codec_zenoh_wireexpr_t struct (no length prefix on the wire) */
    codec_zenoh_wireexpr_t wireexpr;
    uint8_t ext_type;
    uint64_t ext_value;
} codec_zenoh_decl_queryable_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_DECL_QUERYABLE_MAX_BYTES];
    size_t  len;
} codec_zenoh_decl_queryable_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_decl_queryable_decode(sce_forge_cursor_t *cursor, codec_zenoh_decl_queryable_t *out, uint8_t n, uint8_t z) {
    /* RFC Axis-1 inversion: defensive (void) suppress per declared
     * `<sce:flag-input>` so codecs that haven't consumed an input via
     * `present-if` yet compile cleanly under -Wunused-parameter. */
    (void)n;
    (void)z;
    /* RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
     * advances the cursor per field. C11 has no nullable wrapper so
     * the gated field's storage stays as plain `T` (with `_len = 0`
     * for absent bytes payloads); the carrier's flag bit is the
     * source of truth for presence. B2-β extends gating to Tail /
     * LengthRef / Vle bit-sizes via dispatch inside the helper.
     * Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified streaming path. */
    uint32_t id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u32(cursor, &id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->id = id;
    {
        sce_forge_codec_status_t _st = codec_zenoh_wireexpr_decode(cursor, &out->wireexpr, n);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
    }
    if ((z & 0x01) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->ext_type = (uint8_t)(raw[0]);
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->ext_type = 0;
    }
    if ((z & 0x01) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        out->ext_value = _v;
    } else {
        out->ext_value = 0;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_decl_queryable_encoded_t codec_zenoh_decl_queryable_encode(const codec_zenoh_decl_queryable_t *self, uint8_t n, uint8_t z) {
    /* RFC Axis-1 inversion: see decode — same suppress per input. */
    (void)n;
    (void)z;
    codec_zenoh_decl_queryable_encoded_t r;
    /* RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    r.len = 0;
    {
        uint64_t _w = (uint64_t)(self->id);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    {
        codec_zenoh_wireexpr_encoded_t _sub = codec_zenoh_wireexpr_encode(&self->wireexpr, n);
        if (r.len + _sub.len <= sizeof(r.bytes)) {
            for (size_t _ej = 0; _ej < _sub.len; ++_ej) r.bytes[r.len + _ej] = _sub.bytes[_ej];
            r.len += _sub.len;
        }
    }
    if ((z & 0x01) != 0) {
        r.bytes[r.len++] = self->ext_type;
    }
    if ((z & 0x01) != 0) {
    {
        uint64_t _w = (uint64_t)(self->ext_value);
        while (_w >= 0x80u) {
            r.bytes[r.len++] = (uint8_t)((_w & 0x7Fu) | 0x80u);
            _w >>= 7;
        }
        r.bytes[r.len++] = (uint8_t)_w;
    }
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_DECL_QUERYABLE_H */
