// SCE-MAP: codec_zenoh_decl_ext_keyexpr:89 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_H
#define SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */
#include "codec_zenoh_decl_ext_keyexpr_inner.h"

#define CODEC_ZENOH_DECL_EXT_KEYEXPR_MIN_BYTES 1
#define CODEC_ZENOH_DECL_EXT_KEYEXPR_MAX_BYTES 148

typedef struct {
    uint8_t outer_header;
    /* RFC §synth-5-B embed: nested codec_zenoh_decl_ext_keyexpr_inner_t struct (no length prefix on the wire) */
    codec_zenoh_decl_ext_keyexpr_inner_t inner;
} codec_zenoh_decl_ext_keyexpr_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_decode(sce_forge_cursor_t *cursor, codec_zenoh_decl_ext_keyexpr_t *out) {
    /* A field or a body past what this node holds: the frame is read on,
     * judged as one that holds it would be, and the capacity is answered
     * once every other refusal has had its turn (docs/adr/0077 §7). */
    bool _over = false;
    (void)_over;
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
        out->outer_header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint64_t _d_total_length;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_d_total_length);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    {
        size_t _len = (size_t)(_d_total_length);
        const uint8_t *_raw = sce_forge_cursor_peek(cursor, _len);
        if (_raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        sce_forge_cursor_t _inner = sce_forge_cursor_init(_raw, _len);
        sce_forge_codec_status_t _st = sce_forge_codec_note_capacity(codec_zenoh_decl_ext_keyexpr_inner_decode(&_inner, &out->inner), &_over);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
        if (!sce_forge_cursor_advance(cursor, _len)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if (_over) return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_zenoh_decl_ext_keyexpr_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_decl_ext_keyexpr_t is a source by the table
 * codec_zenoh_decl_ext_keyexpr_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*outer_header)(const void *ctx);
    codec_zenoh_decl_ext_keyexpr_inner_source_ref_t (*inner)(const void *ctx);
} codec_zenoh_decl_ext_keyexpr_source_t;

/* A source of codec_zenoh_decl_ext_keyexpr_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_decl_ext_keyexpr_source_t *table;
    const void *ctx;
} codec_zenoh_decl_ext_keyexpr_source_ref_t;

/* A source of codec_zenoh_decl_ext_keyexpr_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_decl_ext_keyexpr_source_ref_optional, codec_zenoh_decl_ext_keyexpr_source_ref_t)

/* The `outer_header` of the codec_zenoh_decl_ext_keyexpr_t at `ctx`. */
static inline uint8_t codec_zenoh_decl_ext_keyexpr_value_outer_header(const void *ctx) {
    return ((const codec_zenoh_decl_ext_keyexpr_t *)ctx)->outer_header;
}

/* The `inner` of the codec_zenoh_decl_ext_keyexpr_t at `ctx`. */
static inline codec_zenoh_decl_ext_keyexpr_inner_source_ref_t codec_zenoh_decl_ext_keyexpr_value_inner(const void *ctx) {
    const codec_zenoh_decl_ext_keyexpr_t *self = (const codec_zenoh_decl_ext_keyexpr_t *)ctx;
    codec_zenoh_decl_ext_keyexpr_inner_source_ref_t body;
    body.table = codec_zenoh_decl_ext_keyexpr_inner_value_source();
    body.ctx = &self->inner;
    return body;
}

/* The table that presents a value of codec_zenoh_decl_ext_keyexpr_t as a source of it, its
 * context a `const codec_zenoh_decl_ext_keyexpr_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_decl_ext_keyexpr_source_t *codec_zenoh_decl_ext_keyexpr_value_source(void) {
    static const codec_zenoh_decl_ext_keyexpr_source_t table = {
        codec_zenoh_decl_ext_keyexpr_value_outer_header,
        codec_zenoh_decl_ext_keyexpr_value_inner,
    };
    return &table;
}

/* One-bit flags of codec_zenoh_decl_ext_keyexpr_t a parent may set in place of the ones a value
 * presents, such as the flag a chain whose entries are codec_zenoh_decl_ext_keyexpr_t ends them
 * on, which the chain derives from where each entry stands. A flag absent
 * here is the value's own. */
typedef struct {
    /* The `M` flag of `outer_header`, where a parent sets it. */
    sce_optional_bool_t outer_header_m;
    /* The `Z` flag of `outer_header`, where a parent sets it. */
    sce_optional_bool_t outer_header_z;
} codec_zenoh_decl_ext_keyexpr_overrides_t;

/* The overrides that set no flag. */
static inline codec_zenoh_decl_ext_keyexpr_overrides_t codec_zenoh_decl_ext_keyexpr_overrides_none(void) {
    codec_zenoh_decl_ext_keyexpr_overrides_t none;
    none.outer_header_m = sce_optional_bool_none();
    none.outer_header_z = sce_optional_bool_none();
    return none;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_encode_source_overriding(const codec_zenoh_decl_ext_keyexpr_source_t *src, const void *ctx, sce_forge_writer_t *w, codec_zenoh_decl_ext_keyexpr_overrides_t overrides) {
    const uint8_t _e_outer_header = src->outer_header(ctx);
    const codec_zenoh_decl_ext_keyexpr_inner_source_ref_t _e_inner = src->inner(ctx);
    uint64_t _f_outer_header = (uint64_t)_e_outer_header;
    if (overrides.outer_header_m.present) {
        _f_outer_header = (_f_outer_header & ~0x10ull) | ((uint64_t)overrides.outer_header_m.value << 4);
    }
    if (overrides.outer_header_z.present) {
        _f_outer_header = (_f_outer_header & ~0x80ull) | ((uint64_t)overrides.outer_header_z.value << 7);
    }
    const uint8_t _c_outer_header = (uint8_t)_f_outer_header;
    sce_forge_writer_t _k_inner = sce_forge_writer_init_counting();
    SCE_FORGE_TRY_WRITE(codec_zenoh_decl_ext_keyexpr_inner_encode_source(_e_inner.table, _e_inner.ctx, &_k_inner));
    const uint64_t _d_total_length = (uint64_t)(sce_forge_writer_position(&_k_inner));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_outer_header));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_d_total_length)));
    SCE_FORGE_TRY_WRITE(codec_zenoh_decl_ext_keyexpr_inner_encode_source(_e_inner.table, _e_inner.ctx, w));
    return SCE_FORGE_CODEC_OK;
}

/* codec_zenoh_decl_ext_keyexpr_encode_source_overriding with no flag set in place of the
 * value's: the encode every caller but a parent that sets one calls. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_encode_source(const codec_zenoh_decl_ext_keyexpr_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    return codec_zenoh_decl_ext_keyexpr_encode_source_overriding(src, ctx, w, codec_zenoh_decl_ext_keyexpr_overrides_none());
}

/* Whether `self` is a value of codec_zenoh_decl_ext_keyexpr_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_check_value(const codec_zenoh_decl_ext_keyexpr_t *self) {
    SCE_FORGE_TRY_WRITE(codec_zenoh_decl_ext_keyexpr_inner_check_value(&self->inner));
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_DECL_EXT_KEYEXPR_MAX_BYTES bytes and use
 * `codec_zenoh_decl_ext_keyexpr_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_encode(const codec_zenoh_decl_ext_keyexpr_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_decl_ext_keyexpr_check_value(self));
    return codec_zenoh_decl_ext_keyexpr_encode_source(codec_zenoh_decl_ext_keyexpr_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_DECL_EXT_KEYEXPR_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_DECL_EXT_KEYEXPR_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_encode_to_buf(const codec_zenoh_decl_ext_keyexpr_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_decl_ext_keyexpr_encode(self, &_w);
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
static inline uint8_t codec_zenoh_decl_ext_keyexpr_ext_id(const codec_zenoh_decl_ext_keyexpr_t *self) {
    return (uint8_t)((self->outer_header >> 0) & (uint8_t)0x0F);
}

static inline void codec_zenoh_decl_ext_keyexpr_set_ext_id(codec_zenoh_decl_ext_keyexpr_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 0);
    self->outer_header = (uint8_t)((self->outer_header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_decl_ext_keyexpr_m(const codec_zenoh_decl_ext_keyexpr_t *self) {
    return (self->outer_header & 0x10) != 0;
}

static inline void codec_zenoh_decl_ext_keyexpr_set_m(codec_zenoh_decl_ext_keyexpr_t *self, bool v) {
    if (v) {
        self->outer_header = (uint8_t)(self->outer_header | 0x10);
    } else {
        self->outer_header = (uint8_t)(self->outer_header & (uint8_t)(~(uint8_t)0x10));
    }
}

static inline uint8_t codec_zenoh_decl_ext_keyexpr_enc(const codec_zenoh_decl_ext_keyexpr_t *self) {
    return (uint8_t)((self->outer_header >> 5) & (uint8_t)0x03);
}

static inline void codec_zenoh_decl_ext_keyexpr_set_enc(codec_zenoh_decl_ext_keyexpr_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x03 << 5);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x03) << 5);
    self->outer_header = (uint8_t)((self->outer_header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_decl_ext_keyexpr_z(const codec_zenoh_decl_ext_keyexpr_t *self) {
    return (self->outer_header & 0x80) != 0;
}

static inline void codec_zenoh_decl_ext_keyexpr_set_z(codec_zenoh_decl_ext_keyexpr_t *self, bool v) {
    if (v) {
        self->outer_header = (uint8_t)(self->outer_header | 0x80);
    } else {
        self->outer_header = (uint8_t)(self->outer_header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_H */
