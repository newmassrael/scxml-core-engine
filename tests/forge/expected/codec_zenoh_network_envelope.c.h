// SCE-MAP: codec_zenoh_network_envelope:60

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
#define CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES 1218

/* RFC §5.B variant primitive (B1-β): tagged-union body for the codec's
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

typedef struct {
    uint8_t bytes[CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES];
    size_t  len;
} codec_zenoh_network_envelope_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_network_envelope_decode(sce_forge_cursor_t *cursor, codec_zenoh_network_envelope_t *out) {
    /* RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
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

static inline codec_zenoh_network_envelope_encoded_t codec_zenoh_network_envelope_encode(const codec_zenoh_network_envelope_t *self) {
    codec_zenoh_network_envelope_encoded_t r;
    /* RFC §5.B Y3 atomic 2b-ii peek-byte / 2b-iv streaming-prefix:
     * streaming prefix encode. Peek-byte mode: arm body's encode
     * prepends its own header byte (which the decoder peeked); no
     * separate tag byte here. Streaming-prefix mode (own-field):
     * carrier is part of the prefix fields and emits via the same
     * per-field path. */
    r.len = 0;
    /* Append the active arm body's encoded bytes. */
    switch (self->body.kind) {
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_INTEREST: {
            codec_zenoh_interest_encoded_t _sub = codec_zenoh_interest_encode(&self->body.arm.codec_zenoh_interest);
            if (r.len + _sub.len <= CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE_FINAL: {
            codec_zenoh_response_final_encoded_t _sub = codec_zenoh_response_final_encode(&self->body.arm.codec_zenoh_response_final);
            if (r.len + _sub.len <= CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_RESPONSE: {
            codec_zenoh_response_encoded_t _sub = codec_zenoh_response_encode(&self->body.arm.codec_zenoh_response);
            if (r.len + _sub.len <= CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_REQUEST: {
            codec_zenoh_request_encoded_t _sub = codec_zenoh_request_encode(&self->body.arm.codec_zenoh_request);
            if (r.len + _sub.len <= CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_PUSH: {
            codec_zenoh_push_encoded_t _sub = codec_zenoh_push_encode(&self->body.arm.codec_zenoh_push);
            if (r.len + _sub.len <= CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_DECLARE: {
            codec_zenoh_declare_encoded_t _sub = codec_zenoh_declare_encode(&self->body.arm.codec_zenoh_declare);
            if (r.len + _sub.len <= CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_CODEC_ZENOH_OAM: {
            codec_zenoh_oam_encoded_t _sub = codec_zenoh_oam_encode(&self->body.arm.codec_zenoh_oam);
            if (r.len + _sub.len <= CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
        case CODEC_ZENOH_NETWORK_ENVELOPE_BODY_KIND_DEFAULT: {
            codec_zenoh_oam_encoded_t _sub = codec_zenoh_oam_encode(&self->body.arm.default_body);
            if (r.len + _sub.len <= CODEC_ZENOH_NETWORK_ENVELOPE_MAX_BYTES) {
                for (size_t _i = 0; _i < _sub.len; ++_i) r.bytes[r.len + _i] = _sub.bytes[_i];
                r.len += _sub.len;
            }
            break;
        }
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_NETWORK_ENVELOPE_H */
