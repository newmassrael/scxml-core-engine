// SCE-MAP: codec_zenoh_interest_body:56

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
    /* RFC §5.B Y0c embed: nested codec_zenoh_wireexpr_t struct (no length prefix on the wire) */
    codec_zenoh_wireexpr_t keyexpr;
} codec_zenoh_interest_body_t;

typedef struct {
    uint8_t bytes[CODEC_ZENOH_INTEREST_BODY_MAX_BYTES];
    size_t  len;
} codec_zenoh_interest_body_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_interest_body_decode(sce_forge_cursor_t *cursor, codec_zenoh_interest_body_t *out) {
    /* RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
     * advances the cursor per field. C11 has no nullable wrapper so
     * the gated field's storage stays as plain `T` (with `_len = 0`
     * for absent bytes payloads); the carrier's flag bit is the
     * source of truth for presence. B2-β extends gating to Tail /
     * LengthRef / Vle bit-sizes via dispatch inside the helper.
     * Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified streaming path. */
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

static inline codec_zenoh_interest_body_encoded_t codec_zenoh_interest_body_encode(const codec_zenoh_interest_body_t *self) {
    codec_zenoh_interest_body_encoded_t r;
    /* RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    r.len = 0;
    r.bytes[r.len++] = self->header;
    if ((self->header & 0x10) != 0) {
        codec_zenoh_wireexpr_encoded_t _sub = codec_zenoh_wireexpr_encode(&self->keyexpr, (uint8_t)((self->header >> 5) & 0x1));
        if (r.len + _sub.len <= sizeof(r.bytes)) {
            for (size_t _ej = 0; _ej < _sub.len; ++_ej) r.bytes[r.len + _ej] = _sub.bytes[_ej];
            r.len += _sub.len;
        }
    }
    return r;
}

/* RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
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
