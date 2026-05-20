// SCE-MAP: codec_zenoh_decl_subscriber:41

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_DECL_SUBSCRIBER_H
#define SCE_FORGE_CODEC_ZENOH_DECL_SUBSCRIBER_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_wireexpr.h"

#define CODEC_ZENOH_DECL_SUBSCRIBER_MIN_BYTES 0
#define CODEC_ZENOH_DECL_SUBSCRIBER_MAX_BYTES 261

typedef struct {
    uint32_t id;
    /* RFC §5.B Y0c embed: nested codec_zenoh_wireexpr_t struct (no length prefix on the wire) */
    codec_zenoh_wireexpr_t wireexpr;
} codec_zenoh_decl_subscriber_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_DECL_SUBSCRIBER_MAX_BYTES];
    size_t  len;
} codec_zenoh_decl_subscriber_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_decl_subscriber_decode(sce_forge_cursor_t *cursor, codec_zenoh_decl_subscriber_t *out, uint8_t n) {
    /* RFC Axis-1 inversion: defensive (void) suppress per declared
     * `<sce:flag-input>` so codecs that haven't consumed an input via
     * `present-if` yet compile cleanly under -Wunused-parameter. */
    (void)n;
    /* Streaming codec: each field reads from cursor directly (VLE
     * base-128 chain, 1..=ceil(N/7) bytes per field). RFC §5.B B4:
     * per-field bit-size dispatch routes Fixed / LengthRef siblings
     * of VLE fields through `present_if_decode_stmt` (predicate=None
     * arms — for VLE the helper emits the local-decl + `out->` assign
     * fused; for Fixed / LengthRef it writes directly to `out->`).
     * Pure-VLE codecs stay byte-stable. */
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
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_decl_subscriber_encoded_t codec_zenoh_decl_subscriber_encode(const codec_zenoh_decl_subscriber_t *self, uint8_t n) {
    /* RFC Axis-1 inversion: see decode — same suppress per input. */
    (void)n;
    codec_zenoh_decl_subscriber_encoded_t r;
    /* RFC §5.B B4: per-field bit-size dispatch routes Fixed /
     * LengthRef / Tail siblings of VLE fields through
     * `present_if_encode_block` (predicate=None arms). Pure-VLE
     * codecs stay byte-stable. */
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
    return r;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_DECL_SUBSCRIBER_H */
