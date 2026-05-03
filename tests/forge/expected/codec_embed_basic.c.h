/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_EMBED_BASIC_H
#define SCE_FORGE_CODEC_EMBED_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_locator.h"

#define CODEC_EMBED_BASIC_MIN_BYTES 1
#define CODEC_EMBED_BASIC_MAX_BYTES 257

typedef struct {
    uint8_t tag;
    /* RFC §5.B Y0c embed: nested codec_zenoh_locator_t struct (no length prefix on the wire) */
    codec_zenoh_locator_t locator;
} codec_embed_basic_t;

typedef struct {
    uint8_t bytes[CODEC_EMBED_BASIC_MAX_BYTES];
    size_t  len;
} codec_embed_basic_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_embed_basic_decode(sce_forge_cursor_t *cursor, codec_embed_basic_t *out) {
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
        out->tag = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        sce_forge_codec_status_t _st = codec_zenoh_locator_decode(cursor, &out->locator);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_embed_basic_encoded_t codec_embed_basic_encode(const codec_embed_basic_t *self) {
    codec_embed_basic_encoded_t r;
    /* RFC §5.B B2 / B3 encode: fixed prefix appends byte-by-byte;
     * list fields walk the per-codec encoded_t splice loop, bounded
     * by `sizeof(r.bytes)` (== MAX_BYTES). Author keeps count field
     * (repeat) / `<id>_len` ≤ max_depth (tlv-chain) consistent with
     * the in-struct entry count (trust contract). */
    r.len = 0;
    r.bytes[r.len++] = self->tag;
    {
        codec_zenoh_locator_encoded_t _sub = codec_zenoh_locator_encode(&self->locator);
        if (r.len + _sub.len <= sizeof(r.bytes)) {
            for (size_t _ej = 0; _ej < _sub.len; ++_ej) r.bytes[r.len + _ej] = _sub.bytes[_ej];
            r.len += _sub.len;
        }
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_EMBED_BASIC_H */
