// SCE-MAP: codec_present_if_negation:12 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_PRESENT_IF_NEGATION_H
#define SCE_FORGE_CODEC_PRESENT_IF_NEGATION_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */

#define CODEC_PRESENT_IF_NEGATION_MIN_BYTES 3
#define CODEC_PRESENT_IF_NEGATION_MAX_BYTES 3

typedef struct {
    uint8_t flags;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_uint16_t seq;
} codec_present_if_negation_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_present_if_negation_decode(sce_forge_cursor_t *cursor, codec_present_if_negation_t *out) {
    /* Streaming cursor decode (SSOT selection: `needs_streaming`).
     * The positional `raw[byte_off]` path is valid only when every
     * field's absolute offset is fixed at codegen time; this branch
     * handles every codec where it is not — present-if-gated fields
     * (runtime presence; C11 stores plain `T` with `_len = 0` for absent
     * bytes, the carrier flag bit being the truth), VLE / repeat /
     * TLV-chain / embed fields (runtime width), and a fixed field after a
     * variable-length payload (offset depends on the payload length).
     * Each field reads its own bytes and advances past what it consumed.
     * Per-field `is_repeat` / `is_tlv_chain` / `is_embed` route to their
     * dedicated helpers; every other field flows through
     * `present_if_decode_stmt`. */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->flags = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((out->flags & 0x01) == 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 2);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->seq = sce_optional_uint16_some((uint16_t)(((uint16_t)raw[0] << 8) | raw[1]));
        if (!sce_forge_cursor_advance(cursor, 2)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->seq = sce_optional_uint16_none();
    }
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_present_if_negation_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_present_if_negation_t is a source by the table
 * codec_present_if_negation_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*flags)(const void *ctx);
    sce_optional_uint16_t (*seq)(const void *ctx);
} codec_present_if_negation_source_t;

/* A source of codec_present_if_negation_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_present_if_negation_source_t *table;
    const void *ctx;
} codec_present_if_negation_source_ref_t;

/* A source of codec_present_if_negation_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_present_if_negation_source_ref_optional, codec_present_if_negation_source_ref_t)

/* The `flags` of the codec_present_if_negation_t at `ctx`. */
static inline uint8_t codec_present_if_negation_value_flags(const void *ctx) {
    return ((const codec_present_if_negation_t *)ctx)->flags;
}

/* The `seq` of the codec_present_if_negation_t at `ctx`. */
static inline sce_optional_uint16_t codec_present_if_negation_value_seq(const void *ctx) {
    return ((const codec_present_if_negation_t *)ctx)->seq;
}

/* The table that presents a value of codec_present_if_negation_t as a source of it, its
 * context a `const codec_present_if_negation_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_present_if_negation_source_t *codec_present_if_negation_value_source(void) {
    static const codec_present_if_negation_source_t table = {
        codec_present_if_negation_value_flags,
        codec_present_if_negation_value_seq,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_present_if_negation_encode_source(const codec_present_if_negation_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_flags = src->flags(ctx);
    const sce_optional_uint16_t _e_seq = src->seq(ctx);
    uint64_t _f_flags = (uint64_t)_e_flags;
    _f_flags = (_f_flags & ~0x1ull) | (uint64_t)(!_e_seq.present);
    const uint8_t _c_flags = (uint8_t)_f_flags;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_flags));
    if (_e_seq.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_e_seq.value >> 8) & 0xFF)));
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(_e_seq.value & 0xFF)));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_present_if_negation_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_present_if_negation_check_value(const codec_present_if_negation_t *self) {
    (void)self;
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_PRESENT_IF_NEGATION_MAX_BYTES bytes and use
 * `codec_present_if_negation_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_present_if_negation_encode(const codec_present_if_negation_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_present_if_negation_check_value(self));
    return codec_present_if_negation_encode_source(codec_present_if_negation_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_PRESENT_IF_NEGATION_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_PRESENT_IF_NEGATION_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_present_if_negation_encode_to_buf(const codec_present_if_negation_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_present_if_negation_encode(self, &_w);
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
static inline bool codec_present_if_negation_absent_seq(const codec_present_if_negation_t *self) {
    return (self->flags & 0x01) != 0;
}

static inline void codec_present_if_negation_set_absent_seq(codec_present_if_negation_t *self, bool v) {
    if (v) {
        self->flags = (uint8_t)(self->flags | 0x01);
    } else {
        self->flags = (uint8_t)(self->flags & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_PRESENT_IF_NEGATION_H */
