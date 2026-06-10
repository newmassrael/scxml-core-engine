// SCE-MAP: codec_qos_byte:15

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_QOS_BYTE_H
#define SCE_FORGE_CODEC_QOS_BYTE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"

#define CODEC_QOS_BYTE_MIN_BYTES 1
#define CODEC_QOS_BYTE_MAX_BYTES 1

typedef struct {
    uint8_t qos;
} codec_qos_byte_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_qos_byte_decode(sce_forge_cursor_t *cursor, codec_qos_byte_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_QOS_BYTE_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->qos = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_QOS_BYTE_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

/* RFC §5.B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_QOS_BYTE_MAX_BYTES bytes and use
 * `codec_qos_byte_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_qos_byte_encode(const codec_qos_byte_t *self, sce_forge_writer_t *w) {
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->qos));
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_QOS_BYTE_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_QOS_BYTE_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_qos_byte_encode_to_buf(const codec_qos_byte_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_qos_byte_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

/* RFC §5.B flags primitive: per-bit-range accessors over
 * the carrier field. Single-bit (width=1) reads as bool; multi-bit
 * (width>=2) reads as the smallest unsigned C11 integer type that fits
 * (uint8_t / uint16_t / uint32_t / uint64_t). Setters mask + shift on
 * the way in so out-of-range callers can't corrupt sibling bits. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline uint8_t codec_qos_byte_priority(const codec_qos_byte_t *self) {
    return (uint8_t)((self->qos >> 0) & (uint8_t)0x07);
}

static inline void codec_qos_byte_set_priority(codec_qos_byte_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x07 << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x07) << 0);
    self->qos = (uint8_t)((self->qos & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_qos_byte_reliable(const codec_qos_byte_t *self) {
    return (self->qos & 0x08) != 0;
}

static inline void codec_qos_byte_set_reliable(codec_qos_byte_t *self, bool v) {
    if (v) {
        self->qos = (uint8_t)(self->qos | 0x08);
    } else {
        self->qos = (uint8_t)(self->qos & (uint8_t)(~(uint8_t)0x08));
    }
}

static inline uint8_t codec_qos_byte_congestion(const codec_qos_byte_t *self) {
    return (uint8_t)((self->qos >> 4) & (uint8_t)0x03);
}

static inline void codec_qos_byte_set_congestion(codec_qos_byte_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x03 << 4);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x03) << 4);
    self->qos = (uint8_t)((self->qos & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_qos_byte_express(const codec_qos_byte_t *self) {
    return (self->qos & 0x40) != 0;
}

static inline void codec_qos_byte_set_express(codec_qos_byte_t *self, bool v) {
    if (v) {
        self->qos = (uint8_t)(self->qos | 0x40);
    } else {
        self->qos = (uint8_t)(self->qos & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_qos_byte_reserved(const codec_qos_byte_t *self) {
    return (self->qos & 0x80) != 0;
}

static inline void codec_qos_byte_set_reserved(codec_qos_byte_t *self, bool v) {
    if (v) {
        self->qos = (uint8_t)(self->qos | 0x80);
    } else {
        self->qos = (uint8_t)(self->qos & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_QOS_BYTE_H */
