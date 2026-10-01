// SCE-MAP: codec_chain_has_tagged_entry:9 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_ENTRY_H
#define SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_ENTRY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_CHAIN_HAS_TAGGED_ENTRY_MIN_BYTES 3
#define CODEC_CHAIN_HAS_TAGGED_ENTRY_MAX_BYTES 3

typedef struct {
    uint8_t entry_type;
    uint8_t ctl;
    uint8_t body;
} codec_chain_has_tagged_entry_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_chain_has_tagged_entry_decode(sce_forge_cursor_t *cursor, codec_chain_has_tagged_entry_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_CHAIN_HAS_TAGGED_ENTRY_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->entry_type = raw[0];
    out->ctl = raw[1];
    out->body = raw[2];
    if (!sce_forge_cursor_advance(cursor, CODEC_CHAIN_HAS_TAGGED_ENTRY_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_CHAIN_HAS_TAGGED_ENTRY_MAX_BYTES bytes and use
 * `codec_chain_has_tagged_entry_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_chain_has_tagged_entry_encode(const codec_chain_has_tagged_entry_t *self, sce_forge_writer_t *w) {
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->entry_type));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->ctl));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->body));
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_CHAIN_HAS_TAGGED_ENTRY_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_CHAIN_HAS_TAGGED_ENTRY_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_chain_has_tagged_entry_encode_to_buf(const codec_chain_has_tagged_entry_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_chain_has_tagged_entry_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

/* RFC §synth-5-B flags primitive: per-bit-range accessors over
 * the carrier field. Single-bit (width=1) reads as bool; multi-bit
 * (width>=2) reads as the smallest unsigned C11 integer type that fits
 * (uint8_t / uint16_t / uint32_t / uint64_t). Setters mask + shift on
 * the way in so out-of-range callers can't corrupt sibling bits. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline bool codec_chain_has_tagged_entry_more(const codec_chain_has_tagged_entry_t *self) {
    return (self->ctl & 0x01) != 0;
}

static inline void codec_chain_has_tagged_entry_set_more(codec_chain_has_tagged_entry_t *self, bool v) {
    if (v) {
        self->ctl = (uint8_t)(self->ctl | 0x01);
    } else {
        self->ctl = (uint8_t)(self->ctl & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_ENTRY_H */
