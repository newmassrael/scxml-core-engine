/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_UNTIL_EOF_BASIC_H
#define SCE_FORGE_CODEC_UNTIL_EOF_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_repeat_elem.h"

#define CODEC_UNTIL_EOF_BASIC_MIN_BYTES 0
#define CODEC_UNTIL_EOF_BASIC_MAX_BYTES 128

typedef struct {
    /* RFC §5.B B2 repeat: fixed array of codec_repeat_elem_t elements (max 64) */
    codec_repeat_elem_t msgs[64];
    size_t  msgs_len;
} codec_until_eof_basic_t;

typedef struct {
    uint8_t bytes[CODEC_UNTIL_EOF_BASIC_MAX_BYTES];
    size_t  len;
} codec_until_eof_basic_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_until_eof_basic_decode(sce_forge_cursor_t *cursor, codec_until_eof_basic_t *out) {
    /* RFC §5.B B2 repeat primitive: streaming decode mixes plain
     * fixed-width reads (per-field via the present-if helper's
     * non-gated arm) with fixed-array repeat loops that iterate the
     * imported codec's `<snake>_decode(cursor, &out-><id>[_i])`
     * either `out-><len_field>` times (length-field) or until cursor
     * exhaustion (until-eof). MAX_COUNT overflow surfaces as
     * NEED_MORE_BYTES so the consumer treats it like cursor
     * exhaustion (typed buffer-overflow lands in B7). */
    {
        out->msgs_len = 0;
        while (sce_forge_cursor_remaining(cursor) > 0) {
            if (out->msgs_len >= 64) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
            sce_forge_codec_status_t _st = codec_repeat_elem_decode(cursor, &out->msgs[out->msgs_len]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
            out->msgs_len++;
        }
    }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_until_eof_basic_encoded_t codec_until_eof_basic_encode(const codec_until_eof_basic_t *self) {
    codec_until_eof_basic_encoded_t r;
    /* RFC §5.B B2 encode: fixed prefix appends byte-by-byte; repeat
     * fields walk the per-codec encoded_t splice loop, bounded by
     * `sizeof(r.bytes)` (== MAX_BYTES). Author keeps count field ==
     * `<id>_len` (trust contract). */
    r.len = 0;
    for (size_t _ri = 0; _ri < self->msgs_len; ++_ri) {
        codec_repeat_elem_encoded_t _sub = codec_repeat_elem_encode(&self->msgs[_ri]);
        if (r.len + _sub.len <= sizeof(r.bytes)) {
            for (size_t _rj = 0; _rj < _sub.len; ++_rj) r.bytes[r.len + _rj] = _sub.bytes[_rj];
            r.len += _sub.len;
        }
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_UNTIL_EOF_BASIC_H */
