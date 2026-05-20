// SCE-MAP: codec_init_syn_envelope:24

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_INIT_SYN_ENVELOPE_H
#define SCE_FORGE_CODEC_INIT_SYN_ENVELOPE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_init_syn_body.h"

#define CODEC_INIT_SYN_ENVELOPE_MIN_BYTES 1
#define CODEC_INIT_SYN_ENVELOPE_MAX_BYTES 5

/* RFC §5.B variant primitive (B1-β): tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_INIT_SYN_ENVELOPE_BODY_KIND_CODEC_INIT_SYN_BODY,
    CODEC_INIT_SYN_ENVELOPE_BODY_KIND_DEFAULT,
} codec_init_syn_envelope_body_kind_t;

typedef struct {
    codec_init_syn_envelope_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_init_syn_body_t codec_init_syn_body;
        codec_init_syn_body_t default_body;
    } arm;
} codec_init_syn_envelope_variant_t;

typedef struct {
    uint8_t header;
    codec_init_syn_envelope_variant_t body;
} codec_init_syn_envelope_t;

/* RFC variant-default-uniformity Atomic β-c11: designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_init_syn_envelope_t x = CODEC_INIT_SYN_ENVELOPE_DEFAULT_INIT;
 * codec_init_syn_envelope_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_INIT_SYN_ENVELOPE_DEFAULT_INIT { \
    .body = { \
        .kind = CODEC_INIT_SYN_ENVELOPE_BODY_KIND_CODEC_INIT_SYN_BODY, \
        .arm = { .codec_init_syn_body = CODEC_INIT_SYN_BODY_DEFAULT_INIT } \
    }, \
}

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_init_syn_envelope_decode(sce_forge_cursor_t *cursor, codec_init_syn_envelope_t *out) {
    /* Decode fixed prefix (RFC §5.B variant B1-β: fields before tag suffix). */
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_INIT_SYN_ENVELOPE_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->header = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_INIT_SYN_ENVELOPE_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch ((uint8_t)((out->header >> 0) & (uint8_t)0x1F)) {
        case 1:
            out->body.kind = CODEC_INIT_SYN_ENVELOPE_BODY_KIND_CODEC_INIT_SYN_BODY;
            _arm_st = codec_init_syn_body_decode(cursor, &out->body.arm.codec_init_syn_body, (uint8_t)((out->header >> 6) & 0x1));
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_INIT_SYN_ENVELOPE_BODY_KIND_DEFAULT;
            out->body.default_tag = (uint8_t)((out->header >> 0) & (uint8_t)0x1F);
            _arm_st = codec_init_syn_body_decode(cursor, &out->body.arm.default_body, (uint8_t)((out->header >> 6) & 0x1));
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §5.B B1-α encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_INIT_SYN_ENVELOPE_MAX_BYTES bytes and use
 * `codec_init_syn_envelope_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_init_syn_envelope_encode(const codec_init_syn_envelope_t *self, sce_forge_writer_t *w) {
    /* Encode fixed prefix (tag field bytes are part of the prefix).
     * The tag value is read from the struct field, NOT derived from
     * the body discriminant — keeping author-set tag / body in sync
     * is the caller's responsibility (v1 keeps the layout simple). */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->header));
    /* Append the active arm body's encoded bytes via the same writer. */
    switch (self->body.kind) {
        case CODEC_INIT_SYN_ENVELOPE_BODY_KIND_CODEC_INIT_SYN_BODY:
            SCE_FORGE_TRY_WRITE(codec_init_syn_body_encode(&self->body.arm.codec_init_syn_body, w, (uint8_t)((self->header >> 6) & 0x1)));
            break;
        case CODEC_INIT_SYN_ENVELOPE_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_init_syn_body_encode(&self->body.arm.default_body, w, (uint8_t)((self->header >> 6) & 0x1)));
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_INIT_SYN_ENVELOPE_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_INIT_SYN_ENVELOPE_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_init_syn_envelope_encode_to_buf(const codec_init_syn_envelope_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_init_syn_envelope_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

/* RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
 * the carrier field. Single-bit (width=1) reads as bool; multi-bit
 * (width>=2) reads as the smallest unsigned C11 integer type that fits
 * (uint8_t / uint16_t / uint32_t / uint64_t). Setters mask + shift on
 * the way in so out-of-range callers can't corrupt sibling bits. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline uint8_t codec_init_syn_envelope_mid(const codec_init_syn_envelope_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x1F);
}

static inline void codec_init_syn_envelope_set_mid(codec_init_syn_envelope_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x1F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x1F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_init_syn_envelope_s(const codec_init_syn_envelope_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_init_syn_envelope_set_s(codec_init_syn_envelope_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

#endif  /* SCE_FORGE_CODEC_INIT_SYN_ENVELOPE_H */
