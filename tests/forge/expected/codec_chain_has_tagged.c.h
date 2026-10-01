// SCE-MAP: codec_chain_has_tagged:11 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_H
#define SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_chain_has_tagged_entry.h"

#define CODEC_CHAIN_HAS_TAGGED_MIN_BYTES 5
#define CODEC_CHAIN_HAS_TAGGED_MAX_BYTES 14

typedef struct {
    uint8_t header;
    /* RFC §synth-5-B B3 tlv-chain: fixed array of codec_chain_has_tagged_entry_t entries (max-depth 3, on-overflow=reject) */
    codec_chain_has_tagged_entry_t entries[3];
    size_t  entries_len;
    uint8_t priority;
    uint16_t checksum;
} codec_chain_has_tagged_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_chain_has_tagged_decode(sce_forge_cursor_t *cursor, codec_chain_has_tagged_t *out) {
    /* Streaming cursor decode (SSOT selection: `needs_streaming`).
     * The positional `raw[byte_off]` path is valid only when every
     * field's absolute offset is fixed at codegen time; this branch
     * handles every codec where it is not — present-if-gated fields
     * (runtime presence; C11 stores plain `T` with `_len = 0` for absent
     * bytes, the carrier flag bit being the truth), VLE / repeat /
     * TLV-chain / embed fields (runtime width), and a fixed field after a
     * variable-length payload (offset depends on the payload length).
     * Each field reads its own bytes and advances past what it consumed.
     * Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
     * dedicated helpers; every other field flows through
     * `present_if_decode_stmt`. */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        out->entries_len = 0;
        bool _more = false;
        for (size_t _i = 0; _i < 3; ++_i) {
            if (sce_forge_cursor_remaining(cursor) == 0) break;
            sce_forge_codec_status_t _st = codec_chain_has_tagged_entry_decode(cursor, &out->entries[out->entries_len]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
            size_t _just = out->entries_len;
            out->entries_len++;
            _more = codec_chain_has_tagged_entry_more(&out->entries[_just]);
            if (!_more) break;
        }
        if (_more && sce_forge_cursor_remaining(cursor) == 0) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        if (_more) return SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW;
    }
    bool _has_entries_7 = false;
    for (size_t _i = 0; _i < out->entries_len; ++_i) {
        if ((uint64_t)(out->entries[_i].entry_type) == 7ULL) {
            _has_entries_7 = true;
            break;
        }
    }
    bool _has_entries_9 = false;
    for (size_t _i = 0; _i < out->entries_len; ++_i) {
        if ((uint64_t)(out->entries[_i].entry_type) == 9ULL) {
            _has_entries_9 = true;
            break;
        }
    }
    if (_has_entries_7 || (out->header & 0x01) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->priority = (uint8_t)(raw[0]);
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->priority = 0;
    }
    if (!_has_entries_9) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 2);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->checksum = (uint16_t)(((uint16_t)raw[0] << 8) | raw[1]);
        if (!sce_forge_cursor_advance(cursor, 2)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->checksum = 0;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_CHAIN_HAS_TAGGED_MAX_BYTES bytes and use
 * `codec_chain_has_tagged_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_chain_has_tagged_encode(const codec_chain_has_tagged_t *self, sce_forge_writer_t *w) {
    bool _has_entries_7 = false;
    for (size_t _i = 0; _i < self->entries_len; ++_i) {
        if ((uint64_t)(self->entries[_i].entry_type) == 7ULL) {
            _has_entries_7 = true;
            break;
        }
    }
    bool _has_entries_9 = false;
    for (size_t _i = 0; _i < self->entries_len; ++_i) {
        if ((uint64_t)(self->entries[_i].entry_type) == 9ULL) {
            _has_entries_9 = true;
            break;
        }
    }
    /* Streaming cursor encode (SSOT selection: `needs_streaming`).
     * Mirrors the streaming decode: every field appends its own bytes in
     * declaration order through the per-field encode blocks, so a gated
     * field skips its append when the carrier flag bit is clear, and a
     * fixed field after a variable-length payload lands after the payload
     * (the positional path appends variable fields last, placing it ahead
     * on the wire). Per-field `is_repeat` / `is_tlv_chain` / `is_embed`
     * route to their dedicated helpers; everything else uses
     * `present_if_encode_block`. */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->header));
    for (size_t _ti = 0; _ti < self->entries_len; ++_ti) {
        SCE_FORGE_TRY_WRITE(codec_chain_has_tagged_entry_encode(&self->entries[_ti], w));
    }
    if (_has_entries_7 || (self->header & 0x01) != 0) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->priority));
    }
    if (!_has_entries_9) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((self->checksum >> 8) & 0xFF)));
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(self->checksum & 0xFF)));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_CHAIN_HAS_TAGGED_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_CHAIN_HAS_TAGGED_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_chain_has_tagged_encode_to_buf(const codec_chain_has_tagged_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_chain_has_tagged_encode(self, &_w);
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
static inline bool codec_chain_has_tagged_wide(const codec_chain_has_tagged_t *self) {
    return (self->header & 0x01) != 0;
}

static inline void codec_chain_has_tagged_set_wide(codec_chain_has_tagged_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x01);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_CHAIN_HAS_TAGGED_H */
