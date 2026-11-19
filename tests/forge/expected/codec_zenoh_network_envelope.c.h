// SCE-MAP: codec_zenoh_network_envelope:60 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_NETWORK_ENVELOPE_H
#define SCE_FORGE_CODEC_ZENOH_NETWORK_ENVELOPE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */
#include "codec_zenoh_interest.h"
#include "codec_zenoh_response_final.h"
#include "codec_zenoh_response.h"
#include "codec_zenoh_request.h"
#include "codec_zenoh_push.h"
#include "codec_zenoh_declare.h"
#include "codec_zenoh_oam.h"

#define CODEC_ZENOH_NETWORK_ENVELOPE_MIN_BYTES 0
#define CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES 18446744073709551615

/* RFC §synth-5-B variant primitive: tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_INTEREST,
    CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE_FINAL,
    CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE,
    CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_REQUEST,
    CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_PUSH,
    CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_DECLARE,
    CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_OAM,
    CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_DEFAULT,
} codec_zenoh_network_envelope_body_kind_t;

typedef struct {
    codec_zenoh_network_envelope_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_zenoh_interest_t codec_zenoh_interest;
        codec_zenoh_response_final_t codec_zenoh_response_final;
        codec_zenoh_response_t codec_zenoh_response;
        codec_zenoh_request_t codec_zenoh_request;
        codec_zenoh_push_t codec_zenoh_push;
        codec_zenoh_declare_t codec_zenoh_declare;
        codec_zenoh_oam_t codec_zenoh_oam;
        codec_zenoh_oam_t default_body;
    } arm;
} codec_zenoh_network_envelope_variant_t;

typedef struct {
    codec_zenoh_network_envelope_variant_t body;
} codec_zenoh_network_envelope_t;

/* RFC variant-default-uniformity (C11): designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_zenoh_network_envelope_t x = CODEC_ZENOH_NETWORK_ENVELOPE_DEFAULT_INIT;
 * codec_zenoh_network_envelope_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_ZENOH_NETWORK_ENVELOPE_DEFAULT_INIT { \
    .body = { \
        .kind = CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_OAM, \
        .arm = { .codec_zenoh_oam = CODEC_ZENOH_OAM_DEFAULT_INIT } \
    }, \
}

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_network_envelope_decode(sce_forge_cursor_t *cursor, codec_zenoh_network_envelope_t *out) {
    /* A field or a body past what this node holds: the frame is read on,
     * judged as one that holds it would be, and the capacity is answered
     * once every other refusal has had its turn (docs/adr/0077 §7). */
    bool _over = false;
    (void)_over;
    /* RFC §synth-5-B peek-byte / streaming-prefix:
     * streaming prefix decode (variable-length fields supported via
     * per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
     * mode additionally peeks the cursor's next byte for variant tag
     * without advancing — arm body decoder reads it as own header. */
    const uint8_t *_peek_raw = sce_forge_cursor_peek(cursor, 1);
    if (_peek_raw == NULL) {
        return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    const uint8_t _peek = _peek_raw[0];
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch ((uint8_t)((_peek >> 0) & (uint8_t)0x1F)) {
        case 25:
            out->body.kind = CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_INTEREST;
            _arm_st = codec_zenoh_interest_decode(cursor, &out->body.arm.codec_zenoh_interest);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 26:
            out->body.kind = CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE_FINAL;
            _arm_st = codec_zenoh_response_final_decode(cursor, &out->body.arm.codec_zenoh_response_final);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 27:
            out->body.kind = CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE;
            _arm_st = codec_zenoh_response_decode(cursor, &out->body.arm.codec_zenoh_response);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 28:
            out->body.kind = CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_REQUEST;
            _arm_st = codec_zenoh_request_decode(cursor, &out->body.arm.codec_zenoh_request);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 29:
            out->body.kind = CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_PUSH;
            _arm_st = codec_zenoh_push_decode(cursor, &out->body.arm.codec_zenoh_push);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 30:
            out->body.kind = CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_DECLARE;
            _arm_st = codec_zenoh_declare_decode(cursor, &out->body.arm.codec_zenoh_declare);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 31:
            out->body.kind = CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_OAM;
            _arm_st = codec_zenoh_oam_decode(cursor, &out->body.arm.codec_zenoh_oam);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_DEFAULT;
            out->body.default_tag = (uint8_t)((_peek >> 0) & (uint8_t)0x1F);
            _arm_st = codec_zenoh_oam_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    if (_over) return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
    return SCE_FORGE_CODEC_OK;
}

/* The arm of the variant a source of codec_zenoh_network_envelope_t presents (docs/adr/0081
 * §2): which arm it is, the tag the catch-all holds, and the arm's body,
 * a source of the arm's codec in the member `kind` names. */
typedef struct {
    codec_zenoh_network_envelope_body_kind_t kind;
    uint8_t default_tag;  /* the catch-all's */
    union {
        codec_zenoh_interest_source_ref_t codec_zenoh_interest;
        codec_zenoh_response_final_source_ref_t codec_zenoh_response_final;
        codec_zenoh_response_source_ref_t codec_zenoh_response;
        codec_zenoh_request_source_ref_t codec_zenoh_request;
        codec_zenoh_push_source_ref_t codec_zenoh_push;
        codec_zenoh_declare_source_ref_t codec_zenoh_declare;
        codec_zenoh_oam_source_ref_t codec_zenoh_oam;
        codec_zenoh_oam_source_ref_t default_body;
    } arm;
} codec_zenoh_network_envelope_body_ref_t;

/* The elements a value of codec_zenoh_network_envelope_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_network_envelope_t is a source by the table
 * codec_zenoh_network_envelope_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    codec_zenoh_network_envelope_body_ref_t (*body)(const void *ctx);
} codec_zenoh_network_envelope_source_t;

/* A source of codec_zenoh_network_envelope_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_network_envelope_source_t *table;
    const void *ctx;
} codec_zenoh_network_envelope_source_ref_t;

/* A source of codec_zenoh_network_envelope_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_network_envelope_source_ref_optional, codec_zenoh_network_envelope_source_ref_t)

/* The arm of the variant of the codec_zenoh_network_envelope_t at `ctx`: its kind, the tag the
 * catch-all holds, and its body as a source of the arm's codec. */
static inline codec_zenoh_network_envelope_body_ref_t codec_zenoh_network_envelope_value_body(const void *ctx) {
    const codec_zenoh_network_envelope_t *self = (const codec_zenoh_network_envelope_t *)ctx;
    codec_zenoh_network_envelope_body_ref_t ref;
    memset(&ref, 0, sizeof ref);
    ref.kind = self->body.kind;
    ref.default_tag = self->body.default_tag;
    switch (self->body.kind) {
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_INTEREST:
            ref.arm.codec_zenoh_interest.table = codec_zenoh_interest_value_source();
            ref.arm.codec_zenoh_interest.ctx = &self->body.arm.codec_zenoh_interest;
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE_FINAL:
            ref.arm.codec_zenoh_response_final.table = codec_zenoh_response_final_value_source();
            ref.arm.codec_zenoh_response_final.ctx = &self->body.arm.codec_zenoh_response_final;
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE:
            ref.arm.codec_zenoh_response.table = codec_zenoh_response_value_source();
            ref.arm.codec_zenoh_response.ctx = &self->body.arm.codec_zenoh_response;
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_REQUEST:
            ref.arm.codec_zenoh_request.table = codec_zenoh_request_value_source();
            ref.arm.codec_zenoh_request.ctx = &self->body.arm.codec_zenoh_request;
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_PUSH:
            ref.arm.codec_zenoh_push.table = codec_zenoh_push_value_source();
            ref.arm.codec_zenoh_push.ctx = &self->body.arm.codec_zenoh_push;
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_DECLARE:
            ref.arm.codec_zenoh_declare.table = codec_zenoh_declare_value_source();
            ref.arm.codec_zenoh_declare.ctx = &self->body.arm.codec_zenoh_declare;
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_OAM:
            ref.arm.codec_zenoh_oam.table = codec_zenoh_oam_value_source();
            ref.arm.codec_zenoh_oam.ctx = &self->body.arm.codec_zenoh_oam;
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_DEFAULT:
            ref.arm.default_body.table = codec_zenoh_oam_value_source();
            ref.arm.default_body.ctx = &self->body.arm.default_body;
            break;
        default:
            break;
    }
    return ref;
}

/* The table that presents a value of codec_zenoh_network_envelope_t as a source of it, its
 * context a `const codec_zenoh_network_envelope_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_network_envelope_source_t *codec_zenoh_network_envelope_value_source(void) {
    static const codec_zenoh_network_envelope_source_t table = {
        codec_zenoh_network_envelope_value_body,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_network_envelope_encode_source(const codec_zenoh_network_envelope_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const codec_zenoh_network_envelope_body_ref_t _e_body = src->body(ctx);
    switch (_e_body.kind) {
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_INTEREST:
            SCE_FORGE_TRY_WRITE(codec_zenoh_interest_encode_source(_e_body.arm.codec_zenoh_interest.table, _e_body.arm.codec_zenoh_interest.ctx, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE_FINAL:
            SCE_FORGE_TRY_WRITE(codec_zenoh_response_final_encode_source(_e_body.arm.codec_zenoh_response_final.table, _e_body.arm.codec_zenoh_response_final.ctx, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE:
            SCE_FORGE_TRY_WRITE(codec_zenoh_response_encode_source(_e_body.arm.codec_zenoh_response.table, _e_body.arm.codec_zenoh_response.ctx, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_REQUEST:
            SCE_FORGE_TRY_WRITE(codec_zenoh_request_encode_source(_e_body.arm.codec_zenoh_request.table, _e_body.arm.codec_zenoh_request.ctx, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_PUSH:
            SCE_FORGE_TRY_WRITE(codec_zenoh_push_encode_source(_e_body.arm.codec_zenoh_push.table, _e_body.arm.codec_zenoh_push.ctx, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_DECLARE:
            SCE_FORGE_TRY_WRITE(codec_zenoh_declare_encode_source(_e_body.arm.codec_zenoh_declare.table, _e_body.arm.codec_zenoh_declare.ctx, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_OAM:
            SCE_FORGE_TRY_WRITE(codec_zenoh_oam_encode_source(_e_body.arm.codec_zenoh_oam.table, _e_body.arm.codec_zenoh_oam.ctx, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_oam_encode_source(_e_body.arm.default_body.table, _e_body.arm.default_body.ctx, w));
            break;
        default:
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_zenoh_network_envelope_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_network_envelope_check_value(const codec_zenoh_network_envelope_t *self) {
    switch (self->body.kind) {
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_INTEREST:
            SCE_FORGE_TRY_WRITE(codec_zenoh_interest_check_value(&self->body.arm.codec_zenoh_interest));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE_FINAL:
            SCE_FORGE_TRY_WRITE(codec_zenoh_response_final_check_value(&self->body.arm.codec_zenoh_response_final));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE:
            SCE_FORGE_TRY_WRITE(codec_zenoh_response_check_value(&self->body.arm.codec_zenoh_response));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_REQUEST:
            SCE_FORGE_TRY_WRITE(codec_zenoh_request_check_value(&self->body.arm.codec_zenoh_request));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_PUSH:
            SCE_FORGE_TRY_WRITE(codec_zenoh_push_check_value(&self->body.arm.codec_zenoh_push));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_DECLARE:
            SCE_FORGE_TRY_WRITE(codec_zenoh_declare_check_value(&self->body.arm.codec_zenoh_declare));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_OAM:
            SCE_FORGE_TRY_WRITE(codec_zenoh_oam_check_value(&self->body.arm.codec_zenoh_oam));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_oam_check_value(&self->body.arm.default_body));
            break;
        default:
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES bytes and use
 * `codec_zenoh_network_envelope_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_network_envelope_encode(const codec_zenoh_network_envelope_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_network_envelope_check_value(self));
    return codec_zenoh_network_envelope_encode_source(codec_zenoh_network_envelope_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_network_envelope_encode_to_buf(const codec_zenoh_network_envelope_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_network_envelope_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_NETWORK_ENVELOPE_H */
