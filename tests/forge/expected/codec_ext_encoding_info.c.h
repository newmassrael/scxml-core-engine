// SCE-MAP: codec_ext_encoding_info:44 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_EXT_ENCODING_INFO_H
#define SCE_FORGE_CODEC_EXT_ENCODING_INFO_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */

#define CODEC_EXT_ENCODING_INFO_MIN_BYTES 2
#define CODEC_EXT_ENCODING_INFO_MAX_BYTES 71

#ifndef SCE_OPTIONAL_BYTES_64
#define SCE_OPTIONAL_BYTES_64
#ifndef SCE_STATIC_BYTES_64
#define SCE_STATIC_BYTES_64
/* SCE Accepted Subset §2.15: a `bytes` of at most 64 bytes. */
typedef struct {
    uint8_t data[64];
    size_t len;
} sce_static_bytes_64_t;

static inline sce_static_bytes_64_t sce_static_bytes_64_of(sce_forge_bytes_view_t bytes) {
    sce_static_bytes_64_t value = { { 0 }, 0 };
    for (size_t i = 0; i < bytes.len && i < 64u; ++i) {
        value.data[i] = bytes.data[i];
    }
    value.len = bytes.len < 64u ? bytes.len : 64u;
    return value;
}
#endif
/* docs/adr/0055: a value of `sce_static_bytes_64_t` that may be absent. */
SCE_OPTIONAL_DECLARE_BYTES(64)
#endif

/* The arms of the `combined_id_has_schema` group of codec_ext_encoding_info_t (docs/adr/0065 §6 R4). */
typedef enum {
    CODEC_EXT_ENCODING_INFO_COMBINED_ID_HAS_SCHEMA_HAS_SCHEMA,
    CODEC_EXT_ENCODING_INFO_COMBINED_ID_HAS_SCHEMA_NOT_HAS_SCHEMA,
} codec_ext_encoding_info_combined_id_has_schema_kind_t;

/* The `combined_id_has_schema` group of codec_ext_encoding_info_t where `combined_id.has_schema` holds: the members present
 * then. */
typedef struct {
    sce_static_bytes_64_t schema;
} codec_ext_encoding_info_combined_id_has_schema_has_schema_t;

/* The `combined_id_has_schema` group of codec_ext_encoding_info_t where `combined_id.has_schema` does not hold: the members present
 * then. */
typedef struct {
    uint8_t schema_size;
} codec_ext_encoding_info_combined_id_has_schema_not_has_schema_t;

/* The `combined_id_has_schema` group of codec_ext_encoding_info_t: the arm the value holds, which `kind`
 * names, each the members present while the gates hold its values
 * (docs/adr/0065 §6 R4). */
typedef struct {
    codec_ext_encoding_info_combined_id_has_schema_kind_t kind;
    union {
        codec_ext_encoding_info_combined_id_has_schema_has_schema_t has_schema;
        codec_ext_encoding_info_combined_id_has_schema_not_has_schema_t not_has_schema;
    } arm;
} codec_ext_encoding_info_combined_id_has_schema_t;


typedef struct {
    uint32_t combined_id;
    codec_ext_encoding_info_combined_id_has_schema_t combined_id_has_schema;
} codec_ext_encoding_info_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_ext_encoding_info_decode(sce_forge_cursor_t *cursor, codec_ext_encoding_info_t *out) {
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
    uint32_t combined_id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u32(cursor, &combined_id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->combined_id = combined_id;
    uint8_t _d_schema_size;
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_schema_size = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    /* The `combined_id_has_schema` group the gates the decode read select (docs/adr/0065
     * §6 R4), opened before the members it reads in place. */
    memset(&out->combined_id_has_schema, 0, sizeof out->combined_id_has_schema);
    sce_static_bytes_64_t *_g_schema = NULL;
    if (((((uint64_t)out->combined_id >> 0) & 1u) != 0)) {
        out->combined_id_has_schema.kind = CODEC_EXT_ENCODING_INFO_COMBINED_ID_HAS_SCHEMA_HAS_SCHEMA;
        _g_schema = &out->combined_id_has_schema.arm.has_schema.schema;
    } else {
        out->combined_id_has_schema.kind = CODEC_EXT_ENCODING_INFO_COMBINED_ID_HAS_SCHEMA_NOT_HAS_SCHEMA;
    }
    if ((out->combined_id & 0x00000001) != 0) {
        if (_d_schema_size > 64) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        size_t _n = (size_t)_d_schema_size;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(_g_schema->data, raw, _n);
        _g_schema->len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    /* The numbers of the `combined_id_has_schema` group, read into locals, set in the arm
     * the decode opened (docs/adr/0065 §6 R4). */
    if (out->combined_id_has_schema.kind == CODEC_EXT_ENCODING_INFO_COMBINED_ID_HAS_SCHEMA_NOT_HAS_SCHEMA) {
        out->combined_id_has_schema.arm.not_has_schema.schema_size = _d_schema_size;
    }
    return SCE_FORGE_CODEC_OK;
}

/* The `schema` element of codec_ext_encoding_info_t: a byte string of 0 to 64 bytes, viewed where it
 * is held, which outlives the view. A view is made by codec_ext_encoding_info_schema_bytes_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const uint8_t *data;
    size_t len;
} codec_ext_encoding_info_schema_bytes_t;

/* Make `*out` a view of the `len` bytes at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a byte string outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_ext_encoding_info_schema_bytes_make(codec_ext_encoding_info_schema_bytes_t *out, const uint8_t *data, size_t len) {
    if (!sce_forge_length_within_bound(len, 64)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* A view of codec_ext_encoding_info_schema_bytes_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_ext_encoding_info_schema_bytes_optional, codec_ext_encoding_info_schema_bytes_t)

/* The `combined_id_has_schema` group a source of codec_ext_encoding_info_t presents where `combined_id.has_schema` holds: the members present
 * then. */
typedef struct {
    codec_ext_encoding_info_schema_bytes_t schema;
} codec_ext_encoding_info_combined_id_has_schema_has_schema_ref_t;

/* The `combined_id_has_schema` group a source of codec_ext_encoding_info_t presents where `combined_id.has_schema` does not hold: the members present
 * then. */
typedef struct {
    uint8_t schema_size;
} codec_ext_encoding_info_combined_id_has_schema_not_has_schema_ref_t;

/* The `combined_id_has_schema` group a source of codec_ext_encoding_info_t presents: the arm the value holds, which `kind`
 * names, each the members present while the gates hold its values
 * (docs/adr/0065 §6 R4). */
typedef struct {
    codec_ext_encoding_info_combined_id_has_schema_kind_t kind;
    union {
        codec_ext_encoding_info_combined_id_has_schema_has_schema_ref_t has_schema;
        codec_ext_encoding_info_combined_id_has_schema_not_has_schema_ref_t not_has_schema;
    } arm;
} codec_ext_encoding_info_combined_id_has_schema_ref_t;

/* The elements a value of codec_ext_encoding_info_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_ext_encoding_info_t is a source by the table
 * codec_ext_encoding_info_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint32_t (*combined_id)(const void *ctx);
    codec_ext_encoding_info_combined_id_has_schema_ref_t (*combined_id_has_schema)(const void *ctx);
} codec_ext_encoding_info_source_t;

/* A source of codec_ext_encoding_info_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_ext_encoding_info_source_t *table;
    const void *ctx;
} codec_ext_encoding_info_source_ref_t;

/* A source of codec_ext_encoding_info_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_ext_encoding_info_source_ref_optional, codec_ext_encoding_info_source_ref_t)

/* The `combined_id` of the codec_ext_encoding_info_t at `ctx`. */
static inline uint32_t codec_ext_encoding_info_value_combined_id(const void *ctx) {
    return ((const codec_ext_encoding_info_t *)ctx)->combined_id;
}

/* The `combined_id_has_schema` group of the codec_ext_encoding_info_t at `ctx`, each member presented
 * where the value holds it. */
static inline codec_ext_encoding_info_combined_id_has_schema_ref_t codec_ext_encoding_info_value_combined_id_has_schema(const void *ctx) {
    const codec_ext_encoding_info_t *self = (const codec_ext_encoding_info_t *)ctx;
    codec_ext_encoding_info_combined_id_has_schema_ref_t ref;
    memset(&ref, 0, sizeof ref);
    ref.kind = self->combined_id_has_schema.kind;
    if (self->combined_id_has_schema.kind == CODEC_EXT_ENCODING_INFO_COMBINED_ID_HAS_SCHEMA_HAS_SCHEMA) {
        ref.arm.has_schema.schema.data = self->combined_id_has_schema.arm.has_schema.schema.data;
        ref.arm.has_schema.schema.len = self->combined_id_has_schema.arm.has_schema.schema.len;
    }
    if (self->combined_id_has_schema.kind == CODEC_EXT_ENCODING_INFO_COMBINED_ID_HAS_SCHEMA_NOT_HAS_SCHEMA) {
        ref.arm.not_has_schema.schema_size = self->combined_id_has_schema.arm.not_has_schema.schema_size;
    }
    return ref;
}

/* The table that presents a value of codec_ext_encoding_info_t as a source of it, its
 * context a `const codec_ext_encoding_info_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_ext_encoding_info_source_t *codec_ext_encoding_info_value_source(void) {
    static const codec_ext_encoding_info_source_t table = {
        codec_ext_encoding_info_value_combined_id,
        codec_ext_encoding_info_value_combined_id_has_schema,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_ext_encoding_info_encode_source(const codec_ext_encoding_info_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint32_t _e_combined_id = src->combined_id(ctx);
    const codec_ext_encoding_info_combined_id_has_schema_ref_t _e_combined_id_has_schema = src->combined_id_has_schema(ctx);
    const codec_ext_encoding_info_schema_bytes_optional_t _e_schema = (_e_combined_id_has_schema.kind == CODEC_EXT_ENCODING_INFO_COMBINED_ID_HAS_SCHEMA_HAS_SCHEMA ? codec_ext_encoding_info_schema_bytes_optional_some(_e_combined_id_has_schema.arm.has_schema.schema) : codec_ext_encoding_info_schema_bytes_optional_none());
    const sce_optional_uint8_t _e_schema_size = (_e_combined_id_has_schema.kind == CODEC_EXT_ENCODING_INFO_COMBINED_ID_HAS_SCHEMA_NOT_HAS_SCHEMA ? sce_optional_uint8_some(_e_combined_id_has_schema.arm.not_has_schema.schema_size) : sce_optional_uint8_none());
    const uint8_t _d_schema_size = _e_schema.present ? (uint8_t)(_e_schema.value.len) : (_e_schema_size.present ? _e_schema_size.value : (uint8_t)0);
    uint64_t _f_combined_id = (uint64_t)_e_combined_id;
    _f_combined_id = (_f_combined_id & ~0x1ull) | (uint64_t)(_e_schema.present);
    const uint32_t _c_combined_id = (uint32_t)_f_combined_id;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u32(w, (uint32_t)(_c_combined_id)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _d_schema_size));
    if (_e_schema.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, _e_schema.value.data, _e_schema.value.len));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_ext_encoding_info_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_ext_encoding_info_check_value(const codec_ext_encoding_info_t *self) {
    if (self->combined_id_has_schema.kind == CODEC_EXT_ENCODING_INFO_COMBINED_ID_HAS_SCHEMA_HAS_SCHEMA) {
        if (!sce_forge_length_within_bound(self->combined_id_has_schema.arm.has_schema.schema.len, 64)) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_EXT_ENCODING_INFO_MAX_BYTES bytes and use
 * `codec_ext_encoding_info_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_ext_encoding_info_encode(const codec_ext_encoding_info_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_ext_encoding_info_check_value(self));
    return codec_ext_encoding_info_encode_source(codec_ext_encoding_info_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_EXT_ENCODING_INFO_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_EXT_ENCODING_INFO_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_ext_encoding_info_encode_to_buf(const codec_ext_encoding_info_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_ext_encoding_info_encode(self, &_w);
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
static inline bool codec_ext_encoding_info_has_schema(const codec_ext_encoding_info_t *self) {
    return (self->combined_id & 0x00000001) != 0;
}

static inline void codec_ext_encoding_info_set_has_schema(codec_ext_encoding_info_t *self, bool v) {
    if (v) {
        self->combined_id = (uint32_t)(self->combined_id | 0x00000001);
    } else {
        self->combined_id = (uint32_t)(self->combined_id & (uint32_t)(~(uint32_t)0x00000001));
    }
}

#endif  /* SCE_FORGE_CODEC_EXT_ENCODING_INFO_H */
