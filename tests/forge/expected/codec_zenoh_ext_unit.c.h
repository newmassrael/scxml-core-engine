// SCE-MAP: codec_zenoh_ext_unit:13 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_EXT_UNIT_H
#define SCE_FORGE_CODEC_ZENOH_EXT_UNIT_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_ZENOH_EXT_UNIT_MIN_BYTES 0
#define CODEC_ZENOH_EXT_UNIT_MAX_BYTES 0

typedef struct {
    /* RFC §synth-5-B empty body — C11 §6.7.2.1 requires a struct to
     * declare at least one member; this placeholder is never on the
     * wire (encoder writes 0 bytes regardless of its value). */
    char _reserved;
} codec_zenoh_ext_unit_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_ext_unit_decode(sce_forge_cursor_t *cursor, codec_zenoh_ext_unit_t *out) {
    /* RFC §synth-5-B empty body — zero-byte payload, no cursor work.
     * The placeholder member is initialised to 0 to keep callers out
     * of UB territory if they ever inspect it. */
    (void)cursor;
    out->_reserved = 0;
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_EXT_UNIT_MAX_BYTES bytes and use
 * `codec_zenoh_ext_unit_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_ext_unit_encode(const codec_zenoh_ext_unit_t *self, sce_forge_writer_t *w) {
    /* RFC §synth-5-B empty body — zero-byte payload. */
    (void)self;
    (void)w;
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_EXT_UNIT_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_EXT_UNIT_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_ext_unit_encode_to_buf(const codec_zenoh_ext_unit_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_ext_unit_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_EXT_UNIT_H */
