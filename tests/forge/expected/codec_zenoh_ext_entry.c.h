// SCE-MAP: codec_zenoh_ext_entry:52

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_EXT_ENTRY_H
#define SCE_FORGE_CODEC_ZENOH_EXT_ENTRY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_unit.h"
#include "codec_zenoh_ext_zint.h"
#include "codec_zenoh_ext_zbuf.h"

#define CODEC_ZENOH_EXT_ENTRY_MIN_BYTES 1
#define CODEC_ZENOH_EXT_ENTRY_MAX_BYTES 43

/* RFC §5.B variant primitive: tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_UNIT,
    CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZINT,
    CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZBUF,
    CODEC_ZENOH_EXT_ENTRY_BODY_KIND_DEFAULT,
} codec_zenoh_ext_entry_body_kind_t;

typedef struct {
    codec_zenoh_ext_entry_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_zenoh_ext_unit_t codec_zenoh_ext_unit;
        codec_zenoh_ext_zint_t codec_zenoh_ext_zint;
        codec_zenoh_ext_zbuf_t codec_zenoh_ext_zbuf;
        codec_zenoh_ext_unit_t default_body;
    } arm;
} codec_zenoh_ext_entry_variant_t;

typedef struct {
    uint8_t header;
    codec_zenoh_ext_entry_variant_t body;
} codec_zenoh_ext_entry_t;

/* RFC variant-default-uniformity (C11): designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_zenoh_ext_entry_t x = CODEC_ZENOH_EXT_ENTRY_DEFAULT_INIT;
 * codec_zenoh_ext_entry_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_ZENOH_EXT_ENTRY_DEFAULT_INIT { \
    .body = { \
        .kind = CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_UNIT, \
        .arm = { .codec_zenoh_ext_unit = CODEC_ZENOH_EXT_UNIT_DEFAULT_INIT } \
    }, \
}

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_ext_entry_decode(sce_forge_cursor_t *cursor, codec_zenoh_ext_entry_t *out) {
    /* Decode fixed prefix (RFC §5.B variant: fields before tag suffix). */
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_ZENOH_EXT_ENTRY_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->header = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_ZENOH_EXT_ENTRY_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch ((uint8_t)((out->header >> 5) & (uint8_t)0x03)) {
        case 0:
            out->body.kind = CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_UNIT;
            _arm_st = codec_zenoh_ext_unit_decode(cursor, &out->body.arm.codec_zenoh_ext_unit);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 1:
            out->body.kind = CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZINT;
            _arm_st = codec_zenoh_ext_zint_decode(cursor, &out->body.arm.codec_zenoh_ext_zint);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 2:
            out->body.kind = CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZBUF;
            _arm_st = codec_zenoh_ext_zbuf_decode(cursor, &out->body.arm.codec_zenoh_ext_zbuf);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_ZENOH_EXT_ENTRY_BODY_KIND_DEFAULT;
            out->body.default_tag = (uint8_t)((out->header >> 5) & (uint8_t)0x03);
            _arm_st = codec_zenoh_ext_unit_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §5.B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_EXT_ENTRY_MAX_BYTES bytes and use
 * `codec_zenoh_ext_entry_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_ext_entry_encode(const codec_zenoh_ext_entry_t *self, sce_forge_writer_t *w) {
    /* Encode fixed prefix (tag field bytes are part of the prefix).
     * The tag value is read from the struct field, NOT derived from
     * the body discriminant — keeping author-set tag / body in sync
     * is the caller's responsibility (v1 keeps the layout simple). */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->header));
    /* Append the active arm body's encoded bytes via the same writer. */
    switch (self->body.kind) {
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_UNIT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_unit_encode(&self->body.arm.codec_zenoh_ext_unit, w));
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZINT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_zint_encode(&self->body.arm.codec_zenoh_ext_zint, w));
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZBUF:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_zbuf_encode(&self->body.arm.codec_zenoh_ext_zbuf, w));
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_unit_encode(&self->body.arm.default_body, w));
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_EXT_ENTRY_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_EXT_ENTRY_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_ext_entry_encode_to_buf(const codec_zenoh_ext_entry_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_ext_entry_encode(self, &_w);
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
static inline uint8_t codec_zenoh_ext_entry_ext_id(const codec_zenoh_ext_entry_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x0F);
}

static inline void codec_zenoh_ext_entry_set_ext_id(codec_zenoh_ext_entry_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_ext_entry_m(const codec_zenoh_ext_entry_t *self) {
    return (self->header & 0x10) != 0;
}

static inline void codec_zenoh_ext_entry_set_m(codec_zenoh_ext_entry_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x10);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x10));
    }
}

static inline uint8_t codec_zenoh_ext_entry_enc(const codec_zenoh_ext_entry_t *self) {
    return (uint8_t)((self->header >> 5) & (uint8_t)0x03);
}

static inline void codec_zenoh_ext_entry_set_enc(codec_zenoh_ext_entry_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x03 << 5);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x03) << 5);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_ext_entry_z(const codec_zenoh_ext_entry_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_zenoh_ext_entry_set_z(codec_zenoh_ext_entry_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_EXT_ENTRY_H */
