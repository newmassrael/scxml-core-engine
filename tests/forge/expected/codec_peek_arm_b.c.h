// SCE-MAP: codec_peek_arm_b:13 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_PEEK_ARM_B_H
#define SCE_FORGE_CODEC_PEEK_ARM_B_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */

#define CODEC_PEEK_ARM_B_MIN_BYTES 3
#define CODEC_PEEK_ARM_B_MAX_BYTES 3

typedef struct {
    uint8_t header;
    uint16_t payload;
} codec_peek_arm_b_t;

/* RFC variant-default-uniformity (C11): designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_peek_arm_b_t x = CODEC_PEEK_ARM_B_DEFAULT_INIT;
 * codec_peek_arm_b_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_PEEK_ARM_B_DEFAULT_INIT { \
    .header = 0x01u, \
}

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_peek_arm_b_decode(sce_forge_cursor_t *cursor, codec_peek_arm_b_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_PEEK_ARM_B_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->header = raw[0];
    out->payload = ((uint16_t)raw[1] << 8) | raw[2];
    if (!sce_forge_cursor_advance(cursor, CODEC_PEEK_ARM_B_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_peek_arm_b_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_peek_arm_b_t is a source by the table
 * codec_peek_arm_b_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*header)(const void *ctx);
    uint16_t (*payload)(const void *ctx);
} codec_peek_arm_b_source_t;

/* A source of codec_peek_arm_b_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_peek_arm_b_source_t *table;
    const void *ctx;
} codec_peek_arm_b_source_ref_t;

/* A source of codec_peek_arm_b_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_peek_arm_b_source_ref_optional, codec_peek_arm_b_source_ref_t)

/* The `header` of the codec_peek_arm_b_t at `ctx`. */
static inline uint8_t codec_peek_arm_b_value_header(const void *ctx) {
    return ((const codec_peek_arm_b_t *)ctx)->header;
}

/* The `payload` of the codec_peek_arm_b_t at `ctx`. */
static inline uint16_t codec_peek_arm_b_value_payload(const void *ctx) {
    return ((const codec_peek_arm_b_t *)ctx)->payload;
}

/* The table that presents a value of codec_peek_arm_b_t as a source of it, its
 * context a `const codec_peek_arm_b_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_peek_arm_b_source_t *codec_peek_arm_b_value_source(void) {
    static const codec_peek_arm_b_source_t table = {
        codec_peek_arm_b_value_header,
        codec_peek_arm_b_value_payload,
    };
    return &table;
}

/* One-bit flags of codec_peek_arm_b_t a parent may set in place of the ones a value
 * presents, such as the flag a chain whose entries are codec_peek_arm_b_t ends them
 * on, which the chain derives from where each entry stands. A flag absent
 * here is the value's own. */
typedef struct {
    /* The `kind` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_kind;
} codec_peek_arm_b_overrides_t;

/* The overrides that set no flag. */
static inline codec_peek_arm_b_overrides_t codec_peek_arm_b_overrides_none(void) {
    codec_peek_arm_b_overrides_t none;
    none.header_kind = sce_optional_bool_none();
    return none;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_peek_arm_b_encode_source_overriding(const codec_peek_arm_b_source_t *src, const void *ctx, sce_forge_writer_t *w, codec_peek_arm_b_overrides_t overrides) {
    const uint8_t _e_header = src->header(ctx);
    const uint16_t _e_payload = src->payload(ctx);
    uint64_t _f_header = (uint64_t)_e_header;
    if (overrides.header_kind.present) {
        _f_header = (_f_header & ~0x1ull) | (uint64_t)overrides.header_kind.value;
    }
    const uint8_t _c_header = (uint8_t)_f_header;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_header));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_e_payload >> 8) & 0xFF)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(_e_payload & 0xFF)));
    return SCE_FORGE_CODEC_OK;
}

/* codec_peek_arm_b_encode_source_overriding with no flag set in place of the
 * value's: the encode every caller but a parent that sets one calls. */
static inline sce_forge_codec_status_t codec_peek_arm_b_encode_source(const codec_peek_arm_b_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    return codec_peek_arm_b_encode_source_overriding(src, ctx, w, codec_peek_arm_b_overrides_none());
}

/* Whether `self` is a value of codec_peek_arm_b_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_peek_arm_b_check_value(const codec_peek_arm_b_t *self) {
    (void)self;
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_PEEK_ARM_B_MAX_BYTES bytes and use
 * `codec_peek_arm_b_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_peek_arm_b_encode(const codec_peek_arm_b_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_peek_arm_b_check_value(self));
    return codec_peek_arm_b_encode_source(codec_peek_arm_b_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_PEEK_ARM_B_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_PEEK_ARM_B_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_peek_arm_b_encode_to_buf(const codec_peek_arm_b_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_peek_arm_b_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

/* RFC §synth-5-B flags primitive: per-bit-range accessors over
 * the carrier field. Single-bit (width=1) reads as bool; multi-bit
 * (width>=2) reads as the smallest unsigned C11 integer type that fits
 * (uint8_t / uint16_t / uint32_t / uint64_t). Setters mask + shift on
 * the way in so out-of-range callers can't corrupt sibling bits. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline bool codec_peek_arm_b_kind(const codec_peek_arm_b_t *self) {
    return (self->header & 0x01) != 0;
}

static inline void codec_peek_arm_b_set_kind(codec_peek_arm_b_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x01);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_PEEK_ARM_B_H */
