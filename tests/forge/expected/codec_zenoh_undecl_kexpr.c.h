// SCE-MAP: codec_zenoh_undecl_kexpr:35

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_UNDECL_KEXPR_H
#define SCE_FORGE_CODEC_ZENOH_UNDECL_KEXPR_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_ZENOH_UNDECL_KEXPR_MIN_BYTES 0
#define CODEC_ZENOH_UNDECL_KEXPR_MAX_BYTES 3

typedef struct {
    uint16_t id;
} codec_zenoh_undecl_kexpr_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_UNDECL_KEXPR_MAX_BYTES];
    size_t  len;
} codec_zenoh_undecl_kexpr_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_undecl_kexpr_decode(sce_forge_cursor_t *cursor, codec_zenoh_undecl_kexpr_t *out) {
    /* Streaming codec: each field reads from cursor directly (VLE
     * base-128 chain, 1..=ceil(N/7) bytes per field). RFC §5.B B4:
     * per-field bit-size dispatch routes Fixed / LengthRef siblings
     * of VLE fields through `present_if_decode_stmt` (predicate=None
     * arms — for VLE the helper emits the local-decl + `out->` assign
     * fused; for Fixed / LengthRef it writes directly to `out->`).
     * Pure-VLE codecs stay byte-stable. */
    uint16_t id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u16(cursor, &id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->id = id;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_undecl_kexpr_encoded_t codec_zenoh_undecl_kexpr_encode(const codec_zenoh_undecl_kexpr_t *self) {
    codec_zenoh_undecl_kexpr_encoded_t r;
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
    return r;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_UNDECL_KEXPR_H */
