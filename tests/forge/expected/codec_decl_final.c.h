// SCE-MAP: codec_decl_final:19

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_DECL_FINAL_H
#define SCE_FORGE_CODEC_DECL_FINAL_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_DECL_FINAL_MIN_BYTES 0
#define CODEC_DECL_FINAL_MAX_BYTES 0

typedef struct {
    /* RFC §5.B B5-α empty body — C11 §6.7.2.1 requires a struct to
     * declare at least one member; this placeholder is never on the
     * wire (encoder writes 0 bytes regardless of its value). */
    char _reserved;
} codec_decl_final_t;

typedef struct {
    /* RFC §5.B B5-α empty body — C11 forbids zero-length arrays so the
     * placeholder is `bytes[1]`; the encoder sets `len = 0` and never
     * writes to `bytes`, so callers read 0 bytes from the wire. */
    uint8_t bytes[1];
    size_t  len;
} codec_decl_final_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_decl_final_decode(sce_forge_cursor_t *cursor, codec_decl_final_t *out) {
    /* RFC §5.B B5-α empty body — zero-byte payload, no cursor work.
     * The placeholder member is initialised to 0 to keep callers out
     * of UB territory if they ever inspect it. */
    (void)cursor;
    out->_reserved = 0;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_decl_final_encoded_t codec_decl_final_encode(const codec_decl_final_t *self) {
    codec_decl_final_encoded_t r;
    /* RFC §5.B B5-α empty body — zero-byte payload. */
    (void)self;
    r.bytes[0] = 0;
    r.len = 0;
    return r;
}

#endif  /* SCE_FORGE_CODEC_DECL_FINAL_H */
