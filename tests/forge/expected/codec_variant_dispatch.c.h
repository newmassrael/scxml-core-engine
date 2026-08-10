// SCE-MAP: codec_variant_dispatch:8 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_VARIANT_DISPATCH_H
#define SCE_FORGE_CODEC_VARIANT_DISPATCH_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_variant_session_open.h"
#include "codec_variant_session_close.h"

#define CODEC_VARIANT_DISPATCH_MIN_BYTES 1
#define CODEC_VARIANT_DISPATCH_MAX_BYTES 3

/* RFC §synth-5-B variant primitive: tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_OPEN,
    CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_CLOSE,
    CODEC_VARIANT_DISPATCH_BODY_KIND_DEFAULT,
} codec_variant_dispatch_body_kind_t;

typedef struct {
    codec_variant_dispatch_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_variant_session_open_t codec_variant_session_open;
        codec_variant_session_close_t codec_variant_session_close;
        codec_variant_session_close_t default_body;
    } arm;
} codec_variant_dispatch_variant_t;

typedef struct {
    uint8_t msg_id;
    codec_variant_dispatch_variant_t body;
} codec_variant_dispatch_t;

/* RFC variant-default-uniformity (C11): designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_variant_dispatch_t x = CODEC_VARIANT_DISPATCH_DEFAULT_INIT;
 * codec_variant_dispatch_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_VARIANT_DISPATCH_DEFAULT_INIT { \
    .body = { \
        .kind = CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_CLOSE, \
        .arm = { .codec_variant_session_close = CODEC_VARIANT_SESSION_CLOSE_DEFAULT_INIT } \
    }, \
}

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_variant_dispatch_decode(sce_forge_cursor_t *cursor, codec_variant_dispatch_t *out) {
    /* Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix). */
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_VARIANT_DISPATCH_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->msg_id = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_VARIANT_DISPATCH_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch (out->msg_id) {
        case 1:
            out->body.kind = CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_OPEN;
            _arm_st = codec_variant_session_open_decode(cursor, &out->body.arm.codec_variant_session_open);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 2:
            out->body.kind = CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_CLOSE;
            _arm_st = codec_variant_session_close_decode(cursor, &out->body.arm.codec_variant_session_close);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_VARIANT_DISPATCH_BODY_KIND_DEFAULT;
            out->body.default_tag = out->msg_id;
            _arm_st = codec_variant_session_close_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_VARIANT_DISPATCH_MAX_BYTES bytes and use
 * `codec_variant_dispatch_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_variant_dispatch_encode(const codec_variant_dispatch_t *self, sce_forge_writer_t *w) {
    /* Encode fixed prefix (tag field bytes are part of the prefix).
     * The tag value is read from the struct field, NOT derived from
     * the body discriminant — keeping author-set tag / body in sync
     * is the caller's responsibility (v1 keeps the layout simple). */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->msg_id));
    /* Append the active arm body's encoded bytes via the same writer. */
    switch (self->body.kind) {
        case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_OPEN:
            SCE_FORGE_TRY_WRITE(codec_variant_session_open_encode(&self->body.arm.codec_variant_session_open, w));
            break;
        case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_CLOSE:
            SCE_FORGE_TRY_WRITE(codec_variant_session_close_encode(&self->body.arm.codec_variant_session_close, w));
            break;
        case CODEC_VARIANT_DISPATCH_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_variant_session_close_encode(&self->body.arm.default_body, w));
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_VARIANT_DISPATCH_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_VARIANT_DISPATCH_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_variant_dispatch_encode_to_buf(const codec_variant_dispatch_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_variant_dispatch_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_VARIANT_DISPATCH_H */
