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
#include "codec_zenoh_interest.h"
#include "codec_zenoh_response_final.h"
#include "codec_zenoh_response.h"
#include "codec_zenoh_request.h"
#include "codec_zenoh_push.h"
#include "codec_zenoh_declare.h"
#include "codec_zenoh_oam.h"

#define CODEC_ZENOH_NETWORK_ENVELOPE_MIN_BYTES 0
#define CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES 1380

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
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES bytes and use
 * `codec_zenoh_network_envelope_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_network_envelope_encode(const codec_zenoh_network_envelope_t *self, sce_forge_writer_t *w) {
    /* RFC §synth-5-B peek-byte / streaming-prefix:
     * streaming prefix encode. Peek-byte mode: arm body's encode
     * prepends its own header byte (which the decoder peeked); no
     * separate tag byte here. Streaming-prefix mode (own-field):
     * carrier is part of the prefix fields and emits via the same
     * per-field path. */
    /* Append the active arm body's encoded bytes via the same writer. */
    switch (self->body.kind) {
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_INTEREST:
            SCE_FORGE_TRY_WRITE(codec_zenoh_interest_encode(&self->body.arm.codec_zenoh_interest, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE_FINAL:
            SCE_FORGE_TRY_WRITE(codec_zenoh_response_final_encode(&self->body.arm.codec_zenoh_response_final, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE:
            SCE_FORGE_TRY_WRITE(codec_zenoh_response_encode(&self->body.arm.codec_zenoh_response, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_REQUEST:
            SCE_FORGE_TRY_WRITE(codec_zenoh_request_encode(&self->body.arm.codec_zenoh_request, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_PUSH:
            SCE_FORGE_TRY_WRITE(codec_zenoh_push_encode(&self->body.arm.codec_zenoh_push, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_DECLARE:
            SCE_FORGE_TRY_WRITE(codec_zenoh_declare_encode(&self->body.arm.codec_zenoh_declare, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_OAM:
            SCE_FORGE_TRY_WRITE(codec_zenoh_oam_encode(&self->body.arm.codec_zenoh_oam, w));
            break;
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_oam_encode(&self->body.arm.default_body, w));
            break;
    }
    return SCE_FORGE_CODEC_OK;
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
