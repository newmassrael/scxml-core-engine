/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_LENGTH_REF_H
#define SCE_FORGE_CODEC_LENGTH_REF_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#define CODEC_LENGTH_REF_MIN_BYTES 2
#define CODEC_LENGTH_REF_MAX_BYTES 34

typedef struct {
    uint8_t msg_id;
    uint8_t len;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="32") */
    uint8_t payload[32];
    size_t  payload_len;
} codec_length_ref_t;

typedef struct {
    uint8_t bytes[CODEC_LENGTH_REF_MAX_BYTES];
    size_t  len;
} codec_length_ref_encoded_t;

static inline bool codec_length_ref_decode(const uint8_t *raw, size_t len, codec_length_ref_t *out) {
    if (len < CODEC_LENGTH_REF_MIN_BYTES) return false;
    out->msg_id = raw[0];
    out->len = raw[1];
    {
        size_t _n = (size_t)out->len;
        if (_n > 32 || 2 + _n > len) return false;
        memcpy(out->payload, raw + 2, _n);
        out->payload_len = _n;
    }
    return true;
}

static inline codec_length_ref_encoded_t codec_length_ref_encode(const codec_length_ref_t *self) {
    codec_length_ref_encoded_t r;
    r.len = CODEC_LENGTH_REF_MIN_BYTES;
    r.bytes[0] = self->msg_id;
    r.bytes[1] = self->len;
    if (self->payload_len <= 32) {
        memcpy(&r.bytes[2], self->payload, self->payload_len);
        r.len = 2 + self->payload_len;
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_LENGTH_REF_H */
