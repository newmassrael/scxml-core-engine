/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_FLAGS_BASIC_H
#define SCE_FORGE_CODEC_FLAGS_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_FLAGS_BASIC_MIN_BYTES 1
#define CODEC_FLAGS_BASIC_MAX_BYTES 1

typedef struct {
    uint8_t header;
} codec_flags_basic_t;

typedef struct {
    uint8_t bytes[CODEC_FLAGS_BASIC_MAX_BYTES];
    size_t  len;
} codec_flags_basic_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_flags_basic_decode(sce_forge_cursor_t *cursor, codec_flags_basic_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_FLAGS_BASIC_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->header = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_FLAGS_BASIC_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

static inline codec_flags_basic_encoded_t codec_flags_basic_encode(const codec_flags_basic_t *self) {
    codec_flags_basic_encoded_t r;
    r.len = CODEC_FLAGS_BASIC_MIN_BYTES;
    r.bytes[0] = self->header;
    return r;
}

/* RFC §5.B B1-γ flags primitive: per-bit accessors over the carrier
 * field. Read returns a bool from `(field & mask) != 0`; write toggles
 * the bit on/off without disturbing siblings on the same carrier. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline bool codec_flags_basic_reliable(const codec_flags_basic_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_flags_basic_set_reliable(codec_flags_basic_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

static inline bool codec_flags_basic_more(const codec_flags_basic_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_flags_basic_set_more(codec_flags_basic_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_flags_basic_drop(const codec_flags_basic_t *self) {
    return (self->header & 0x20) != 0;
}

static inline void codec_flags_basic_set_drop(codec_flags_basic_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x20);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x20));
    }
}

static inline bool codec_flags_basic_first(const codec_flags_basic_t *self) {
    return (self->header & 0x10) != 0;
}

static inline void codec_flags_basic_set_first(codec_flags_basic_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x10);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x10));
    }
}

#endif  /* SCE_FORGE_CODEC_FLAGS_BASIC_H */
