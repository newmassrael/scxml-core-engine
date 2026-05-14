// SCE-MAP: codec_repeat_basic:11

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_REPEAT_BASIC_H
#define SCE_FORGE_CODEC_REPEAT_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_repeat_elem.h"

#define CODEC_REPEAT_BASIC_MIN_BYTES 1
#define CODEC_REPEAT_BASIC_MAX_BYTES 65

typedef struct {
    uint8_t num_frags;
    /* RFC §5.B B2 repeat: fixed array of codec_repeat_elem_t elements (max 32) */
    codec_repeat_elem_t frags[32];
    size_t  frags_len;
} codec_repeat_basic_t;

typedef struct {
    uint8_t bytes[CODEC_REPEAT_BASIC_MAX_BYTES];
    size_t  len;
} codec_repeat_basic_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_repeat_basic_decode(sce_forge_cursor_t *cursor, codec_repeat_basic_t *out) {
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
        out->num_frags = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        size_t _n = (size_t)out->num_frags;
        if (_n > 32) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        for (size_t _i = 0; _i < _n; ++_i) {
            sce_forge_codec_status_t _st = codec_repeat_elem_decode(cursor, &out->frags[_i]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
        }
        out->frags_len = _n;
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_repeat_basic_encoded_t codec_repeat_basic_encode(const codec_repeat_basic_t *self) {
    codec_repeat_basic_encoded_t r;
    /* RFC §5.B B2 / B3 encode: fixed prefix appends byte-by-byte;
     * list fields walk the per-codec encoded_t splice loop, bounded
     * by `sizeof(r.bytes)` (== MAX_BYTES). Author keeps count field
     * (repeat) / `<id>_len` ≤ max_depth (tlv-chain) consistent with
     * the in-struct entry count (trust contract). */
    r.len = 0;
    r.bytes[r.len++] = self->num_frags;
    for (size_t _ri = 0; _ri < self->frags_len; ++_ri) {
        codec_repeat_elem_encoded_t _sub = codec_repeat_elem_encode(&self->frags[_ri]);
        if (r.len + _sub.len <= sizeof(r.bytes)) {
            for (size_t _rj = 0; _rj < _sub.len; ++_rj) r.bytes[r.len + _rj] = _sub.bytes[_rj];
            r.len += _sub.len;
        }
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_REPEAT_BASIC_H */
