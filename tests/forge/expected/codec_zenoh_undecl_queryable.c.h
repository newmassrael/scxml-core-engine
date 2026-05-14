// SCE-MAP: codec_zenoh_undecl_queryable:23

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_UNDECL_QUERYABLE_H
#define SCE_FORGE_CODEC_ZENOH_UNDECL_QUERYABLE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_decl_ext_keyexpr.h"

#define CODEC_ZENOH_UNDECL_QUERYABLE_MIN_BYTES 0
#define CODEC_ZENOH_UNDECL_QUERYABLE_MAX_BYTES 261

typedef struct {
    uint32_t id;
    /* RFC §5.B Y0c embed: nested codec_zenoh_decl_ext_keyexpr_t struct (no length prefix on the wire) */
    codec_zenoh_decl_ext_keyexpr_t ext_keyexpr;
} codec_zenoh_undecl_queryable_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_UNDECL_QUERYABLE_MAX_BYTES];
    size_t  len;
} codec_zenoh_undecl_queryable_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_undecl_queryable_decode(sce_forge_cursor_t *cursor, codec_zenoh_undecl_queryable_t *out, uint8_t parent_flags) {
    /* RFC §5.B B5-γ: `parent_flags` is the parent codec's flags
     * carrier value, threaded by the variant arm dispatcher. Body
     * fields gated via `parent.<flag>` predicates read from this
     * parameter; defensive `(void)parent_flags` suppresses the
     * `-Wunused-parameter` warning when no gated field happens to
     * consume it (mirrors the Rust `let _ = parent_flags;` and Cpp
     * `(void)parent_flags;` defensive guards). */
    (void)parent_flags;
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
    if ((parent_flags & 0x80) != 0) {
        sce_forge_codec_status_t _st = codec_zenoh_decl_ext_keyexpr_decode(cursor, &out->ext_keyexpr);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_undecl_queryable_encoded_t codec_zenoh_undecl_queryable_encode(const codec_zenoh_undecl_queryable_t *self, uint8_t parent_flags) {
    /* RFC §5.B B5-γ: see decode — same parameter, same suppress. */
    (void)parent_flags;
    codec_zenoh_undecl_queryable_encoded_t r;
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
    if ((parent_flags & 0x80) != 0) {
        codec_zenoh_decl_ext_keyexpr_encoded_t _sub = codec_zenoh_decl_ext_keyexpr_encode(&self->ext_keyexpr);
        if (r.len + _sub.len <= sizeof(r.bytes)) {
            for (size_t _ej = 0; _ej < _sub.len; ++_ej) r.bytes[r.len + _ej] = _sub.bytes[_ej];
            r.len += _sub.len;
        }
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_UNDECL_QUERYABLE_H */
