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
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */
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
    const uint8_t _d_msg_id = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_VARIANT_DISPATCH_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch (_d_msg_id) {
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
            out->body.default_tag = _d_msg_id;
            _arm_st = codec_variant_session_close_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* The arm of the variant a source of codec_variant_dispatch_t presents (docs/adr/0081
 * §2): which arm it is, the tag the catch-all holds, and the arm's body,
 * a source of the arm's codec in the member `kind` names. */
typedef struct {
    codec_variant_dispatch_body_kind_t kind;
    uint8_t default_tag;  /* the catch-all's */
    union {
        codec_variant_session_open_source_ref_t codec_variant_session_open;
        codec_variant_session_close_source_ref_t codec_variant_session_close;
        codec_variant_session_close_source_ref_t default_body;
    } arm;
} codec_variant_dispatch_body_ref_t;

/* The elements a value of codec_variant_dispatch_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_variant_dispatch_t is a source by the table
 * codec_variant_dispatch_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    codec_variant_dispatch_body_ref_t (*body)(const void *ctx);
} codec_variant_dispatch_source_t;

/* A source of codec_variant_dispatch_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_variant_dispatch_source_t *table;
    const void *ctx;
} codec_variant_dispatch_source_ref_t;

/* A source of codec_variant_dispatch_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_variant_dispatch_source_ref_optional, codec_variant_dispatch_source_ref_t)

/* The arm of the variant of the codec_variant_dispatch_t at `ctx`: its kind, the tag the
 * catch-all holds, and its body as a source of the arm's codec. */
static inline codec_variant_dispatch_body_ref_t codec_variant_dispatch_value_body(const void *ctx) {
    const codec_variant_dispatch_t *self = (const codec_variant_dispatch_t *)ctx;
    codec_variant_dispatch_body_ref_t ref;
    memset(&ref, 0, sizeof ref);
    ref.kind = self->body.kind;
    ref.default_tag = self->body.default_tag;
    switch (self->body.kind) {
        case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_OPEN:
            ref.arm.codec_variant_session_open.table = codec_variant_session_open_value_source();
            ref.arm.codec_variant_session_open.ctx = &self->body.arm.codec_variant_session_open;
            break;
        case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_CLOSE:
            ref.arm.codec_variant_session_close.table = codec_variant_session_close_value_source();
            ref.arm.codec_variant_session_close.ctx = &self->body.arm.codec_variant_session_close;
            break;
        case CODEC_VARIANT_DISPATCH_BODY_KIND_DEFAULT:
            ref.arm.default_body.table = codec_variant_session_close_value_source();
            ref.arm.default_body.ctx = &self->body.arm.default_body;
            break;
        default:
            break;
    }
    return ref;
}

/* The table that presents a value of codec_variant_dispatch_t as a source of it, its
 * context a `const codec_variant_dispatch_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_variant_dispatch_source_t *codec_variant_dispatch_value_source(void) {
    static const codec_variant_dispatch_source_t table = {
        codec_variant_dispatch_value_body,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_variant_dispatch_encode_source(const codec_variant_dispatch_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const codec_variant_dispatch_body_ref_t _e_body = src->body(ctx);
    uint64_t _t_body = 0;
    switch (_e_body.kind) {
        case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_OPEN:
            _t_body = 0x1ull;
            break;
        case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_CLOSE:
            _t_body = 0x2ull;
            break;
        case CODEC_VARIANT_DISPATCH_BODY_KIND_DEFAULT:
            _t_body = (uint64_t)_e_body.default_tag;
            break;
        default:
            break;
    }
    const uint8_t _d_msg_id = (uint8_t)_t_body;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _d_msg_id));
    switch (_e_body.kind) {
        case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_OPEN:
            SCE_FORGE_TRY_WRITE(codec_variant_session_open_encode_source(_e_body.arm.codec_variant_session_open.table, _e_body.arm.codec_variant_session_open.ctx, w));
            break;
        case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_CLOSE:
            SCE_FORGE_TRY_WRITE(codec_variant_session_close_encode_source(_e_body.arm.codec_variant_session_close.table, _e_body.arm.codec_variant_session_close.ctx, w));
            break;
        case CODEC_VARIANT_DISPATCH_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_variant_session_close_encode_source(_e_body.arm.default_body.table, _e_body.arm.default_body.ctx, w));
            break;
        default:
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_variant_dispatch_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_variant_dispatch_check_value(const codec_variant_dispatch_t *self) {
    switch (self->body.kind) {
        case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_OPEN:
            SCE_FORGE_TRY_WRITE(codec_variant_session_open_check_value(&self->body.arm.codec_variant_session_open));
            break;
        case CODEC_VARIANT_DISPATCH_BODY_KIND_CODEC_VARIANT_SESSION_CLOSE:
            SCE_FORGE_TRY_WRITE(codec_variant_session_close_check_value(&self->body.arm.codec_variant_session_close));
            break;
        case CODEC_VARIANT_DISPATCH_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_variant_session_close_check_value(&self->body.arm.default_body));
            break;
        default:
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_VARIANT_DISPATCH_MAX_BYTES bytes and use
 * `codec_variant_dispatch_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_variant_dispatch_encode(const codec_variant_dispatch_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_variant_dispatch_check_value(self));
    return codec_variant_dispatch_encode_source(codec_variant_dispatch_value_source(), self, w);
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
