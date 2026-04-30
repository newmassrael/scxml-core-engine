/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_TAIL_H
#define SCE_FORGE_CODEC_TAIL_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#define CODEC_TAIL_MIN_BYTES 2
#define CODEC_TAIL_MAX_BYTES 34

typedef struct {
    uint8_t msg_id;
    uint8_t status;
    /* variable-length payload (sce:bit-size="tail", sce:max-size="32") */
    uint8_t payload[32];
    size_t  payload_len;
} codec_tail_t;

typedef struct {
    uint8_t bytes[CODEC_TAIL_MAX_BYTES];
    size_t  len;
} codec_tail_encoded_t;

static inline bool codec_tail_decode(const uint8_t *raw, size_t len, codec_tail_t *out) {
    if (len < CODEC_TAIL_MIN_BYTES) return false;
    out->msg_id = raw[0];
    out->status = raw[1];
    {
        size_t _n = len - 2;
        if (_n > 32) return false;
        memcpy(out->payload, raw + 2, _n);
        out->payload_len = _n;
    }
    return true;
}

static inline codec_tail_encoded_t codec_tail_encode(const codec_tail_t *self) {
    codec_tail_encoded_t r;
    r.len = CODEC_TAIL_MIN_BYTES;
    r.bytes[0] = self->msg_id;
    r.bytes[1] = self->status;
    if (self->payload_len <= 32) {
        memcpy(&r.bytes[2], self->payload, self->payload_len);
        r.len = 2 + self->payload_len;
    }
    return r;
}

#endif  /* SCE_FORGE_CODEC_TAIL_H */
