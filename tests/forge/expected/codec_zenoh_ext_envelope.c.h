/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_EXT_ENVELOPE_H
#define SCE_FORGE_CODEC_ZENOH_EXT_ENVELOPE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"

#define CODEC_ZENOH_EXT_ENVELOPE_MIN_BYTES 1
#define CODEC_ZENOH_EXT_ENVELOPE_MAX_BYTES 345

typedef struct {
    uint8_t header_flags;
    /* RFC §5.B B3 tlv-chain: fixed array of codec_zenoh_ext_entry_t entries (max-depth 8, on-overflow=reject) */
    codec_zenoh_ext_entry_t extensions[8];
    size_t  extensions_len;
} codec_zenoh_ext_envelope_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_EXT_ENVELOPE_MAX_BYTES];
    size_t  len;
} codec_zenoh_ext_envelope_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_ext_envelope_decode(sce_forge_cursor_t *cursor, codec_zenoh_ext_envelope_t *out) {
    /* RFC §5.B B2 repeat / B3 TLV chain primitives: streaming decode
     * mixes plain fixed-width reads with bounded-iteration loops over
     * imported codec entries. Repeat: bounded by `out-><len_field>`
     * (length-field) or until cursor exhaustion (until-eof); MAX_COUNT
     * overflow → NEED_MORE_BYTES. TLV chain: bounded by `max_depth`
     * with on-overflow check (reject → SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW
     * when residual bytes after cap; truncate → silent). */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->header_flags = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        out->extensions_len = 0;
        for (size_t _i = 0; _i < 8; ++_i) {
            if (sce_forge_cursor_remaining(cursor) == 0) break;
            sce_forge_codec_status_t _st = codec_zenoh_ext_entry_decode(cursor, &out->extensions[out->extensions_len]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
            size_t _just = out->extensions_len;
            out->extensions_len++;
            if (!codec_zenoh_ext_entry_z(&out->extensions[_just])) break;
        }
        if (sce_forge_cursor_remaining(cursor) > 0) return SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_zenoh_ext_envelope_encoded_t codec_zenoh_ext_envelope_encode(const codec_zenoh_ext_envelope_t *self) {
    codec_zenoh_ext_envelope_encoded_t r;
    /* RFC §5.B B2 / B3 encode: fixed prefix appends byte-by-byte;
     * list fields walk the per-codec encoded_t splice loop, bounded
     * by `sizeof(r.bytes)` (== MAX_BYTES). Author keeps count field
     * (repeat) / `<id>_len` ≤ max_depth (tlv-chain) consistent with
     * the in-struct entry count (trust contract). */
    r.len = 0;
    r.bytes[r.len++] = self->header_flags;
    for (size_t _ti = 0; _ti < self->extensions_len; ++_ti) {
        codec_zenoh_ext_entry_encoded_t _sub = codec_zenoh_ext_entry_encode(&self->extensions[_ti]);
        if (r.len + _sub.len <= sizeof(r.bytes)) {
            for (size_t _tj = 0; _tj < _sub.len; ++_tj) r.bytes[r.len + _tj] = _sub.bytes[_tj];
            r.len += _sub.len;
        }
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_EXT_ENVELOPE_H */
