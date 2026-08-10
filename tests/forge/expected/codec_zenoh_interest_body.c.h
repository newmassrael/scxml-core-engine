// SCE-MAP: codec_zenoh_interest_body:56 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_INTEREST_BODY_H
#define SCE_FORGE_CODEC_ZENOH_INTEREST_BODY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_wireexpr.h"

#define CODEC_ZENOH_INTEREST_BODY_MIN_BYTES 1
#define CODEC_ZENOH_INTEREST_BODY_MAX_BYTES 257

typedef struct {
    uint8_t header;
    /* RFC §synth-5-B embed: nested codec_zenoh_wireexpr_t struct (no length prefix on the wire) */
    codec_zenoh_wireexpr_t keyexpr;
} codec_zenoh_interest_body_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_interest_body_decode(sce_forge_cursor_t *cursor, codec_zenoh_interest_body_t *out) {
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
    if ((out->header & 0x10) != 0) {
        sce_forge_codec_status_t _st = codec_zenoh_wireexpr_decode(cursor, &out->keyexpr, (uint8_t)((out->header >> 5) & 0x1));
        if (_st != SCE_FORGE_CODEC_OK) return _st;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_INTEREST_BODY_MAX_BYTES bytes and use
 * `codec_zenoh_interest_body_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_interest_body_encode(const codec_zenoh_interest_body_t *self, sce_forge_writer_t *w) {
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
    if ((self->header & 0x10) != 0) {
        SCE_FORGE_TRY_WRITE(codec_zenoh_wireexpr_encode(&self->keyexpr, w, (uint8_t)((self->header >> 5) & 0x1)));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_INTEREST_BODY_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_INTEREST_BODY_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_interest_body_encode_to_buf(const codec_zenoh_interest_body_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_interest_body_encode(self, &_w);
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
static inline bool codec_zenoh_interest_body_keyexprs(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x01) != 0;
}

static inline void codec_zenoh_interest_body_set_keyexprs(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x01);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x01));
    }
}

static inline bool codec_zenoh_interest_body_subscribers(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x02) != 0;
}

static inline void codec_zenoh_interest_body_set_subscribers(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x02);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x02));
    }
}

static inline bool codec_zenoh_interest_body_queryables(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x04) != 0;
}

static inline void codec_zenoh_interest_body_set_queryables(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x04);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x04));
    }
}

static inline bool codec_zenoh_interest_body_tokens(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x08) != 0;
}

static inline void codec_zenoh_interest_body_set_tokens(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x08);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x08));
    }
}

static inline bool codec_zenoh_interest_body_restricted(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x10) != 0;
}

static inline void codec_zenoh_interest_body_set_restricted(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x10);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x10));
    }
}

static inline bool codec_zenoh_interest_body_n(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x20) != 0;
}

static inline void codec_zenoh_interest_body_set_n(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x20);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x20));
    }
}

static inline bool codec_zenoh_interest_body_m(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_zenoh_interest_body_set_m(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_zenoh_interest_body_aggregate(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_zenoh_interest_body_set_aggregate(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_INTEREST_BODY_H */
