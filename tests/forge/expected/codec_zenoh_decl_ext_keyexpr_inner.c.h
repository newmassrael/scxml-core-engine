// SCE-MAP: codec_zenoh_decl_ext_keyexpr_inner:65 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_H
#define SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */

#define CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_MIN_BYTES 1
#define CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_MAX_BYTES 138

#ifndef SCE_OPTIONAL_BYTES_128
#define SCE_OPTIONAL_BYTES_128
#ifndef SCE_STATIC_BYTES_128
#define SCE_STATIC_BYTES_128
/* SCE Accepted Subset §2.15: a `bytes` of at most 128 bytes. */
typedef struct {
    uint8_t data[128];
    size_t len;
} sce_static_bytes_128_t;

static inline sce_static_bytes_128_t sce_static_bytes_128_of(sce_forge_bytes_view_t bytes) {
    sce_static_bytes_128_t value = { { 0 }, 0 };
    for (size_t i = 0; i < bytes.len && i < 128u; ++i) {
        value.data[i] = bytes.data[i];
    }
    value.len = bytes.len < 128u ? bytes.len : 128u;
    return value;
}
#endif
/* docs/adr/0055: a value of `sce_static_bytes_128_t` that may be absent. */
SCE_OPTIONAL_DECLARE_BYTES(128)
#endif

typedef struct {
    uint8_t inner_header;
    uint64_t id;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_bytes_128_t suffix;
} codec_zenoh_decl_ext_keyexpr_inner_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_inner_decode(sce_forge_cursor_t *cursor, codec_zenoh_decl_ext_keyexpr_inner_t *out) {
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
        out->inner_header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint64_t id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->id = id;
    if ((out->inner_header & 0x01) != 0) {
        size_t _n = sce_forge_cursor_remaining(cursor);
        if (_n > 65535) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->suffix = sce_optional_bytes_128_none();
        out->suffix.value.len = sce_forge_codec_hold(out->suffix.value.data, 128U, raw, _n, &_over);
        out->suffix.present = true;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->suffix = sce_optional_bytes_128_none();
    }
    if (_over) return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
    return SCE_FORGE_CODEC_OK;
}

/* The `suffix` element of codec_zenoh_decl_ext_keyexpr_inner_t: a byte string of 0 to 65535 bytes, viewed where it
 * is held, which outlives the view. A view is made by codec_zenoh_decl_ext_keyexpr_inner_suffix_bytes_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const uint8_t *data;
    size_t len;
} codec_zenoh_decl_ext_keyexpr_inner_suffix_bytes_t;

/* Make `*out` a view of the `len` bytes at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a byte string outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_inner_suffix_bytes_make(codec_zenoh_decl_ext_keyexpr_inner_suffix_bytes_t *out, const uint8_t *data, size_t len) {
    if (!sce_forge_length_within_bound(len, 65535)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* A view of codec_zenoh_decl_ext_keyexpr_inner_suffix_bytes_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_decl_ext_keyexpr_inner_suffix_bytes_optional, codec_zenoh_decl_ext_keyexpr_inner_suffix_bytes_t)

/* The elements a value of codec_zenoh_decl_ext_keyexpr_inner_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_decl_ext_keyexpr_inner_t is a source by the table
 * codec_zenoh_decl_ext_keyexpr_inner_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*inner_header)(const void *ctx);
    uint64_t (*id)(const void *ctx);
    codec_zenoh_decl_ext_keyexpr_inner_suffix_bytes_optional_t (*suffix)(const void *ctx);
} codec_zenoh_decl_ext_keyexpr_inner_source_t;

/* A source of codec_zenoh_decl_ext_keyexpr_inner_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_decl_ext_keyexpr_inner_source_t *table;
    const void *ctx;
} codec_zenoh_decl_ext_keyexpr_inner_source_ref_t;

/* A source of codec_zenoh_decl_ext_keyexpr_inner_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_decl_ext_keyexpr_inner_source_ref_optional, codec_zenoh_decl_ext_keyexpr_inner_source_ref_t)

/* The `inner_header` of the codec_zenoh_decl_ext_keyexpr_inner_t at `ctx`. */
static inline uint8_t codec_zenoh_decl_ext_keyexpr_inner_value_inner_header(const void *ctx) {
    return ((const codec_zenoh_decl_ext_keyexpr_inner_t *)ctx)->inner_header;
}

/* The `id` of the codec_zenoh_decl_ext_keyexpr_inner_t at `ctx`. */
static inline uint64_t codec_zenoh_decl_ext_keyexpr_inner_value_id(const void *ctx) {
    return ((const codec_zenoh_decl_ext_keyexpr_inner_t *)ctx)->id;
}

/* The `suffix` of the codec_zenoh_decl_ext_keyexpr_inner_t at `ctx`. */
static inline codec_zenoh_decl_ext_keyexpr_inner_suffix_bytes_optional_t codec_zenoh_decl_ext_keyexpr_inner_value_suffix(const void *ctx) {
    const codec_zenoh_decl_ext_keyexpr_inner_t *self = (const codec_zenoh_decl_ext_keyexpr_inner_t *)ctx;
    if (!self->suffix.present) {
        return codec_zenoh_decl_ext_keyexpr_inner_suffix_bytes_optional_none();
    }
    codec_zenoh_decl_ext_keyexpr_inner_suffix_bytes_t view;
    view.data = self->suffix.value.data;
    view.len = self->suffix.value.len;
    return codec_zenoh_decl_ext_keyexpr_inner_suffix_bytes_optional_some(view);
}

/* The table that presents a value of codec_zenoh_decl_ext_keyexpr_inner_t as a source of it, its
 * context a `const codec_zenoh_decl_ext_keyexpr_inner_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_decl_ext_keyexpr_inner_source_t *codec_zenoh_decl_ext_keyexpr_inner_value_source(void) {
    static const codec_zenoh_decl_ext_keyexpr_inner_source_t table = {
        codec_zenoh_decl_ext_keyexpr_inner_value_inner_header,
        codec_zenoh_decl_ext_keyexpr_inner_value_id,
        codec_zenoh_decl_ext_keyexpr_inner_value_suffix,
    };
    return &table;
}

/* One-bit flags of codec_zenoh_decl_ext_keyexpr_inner_t a parent may set in place of the ones a value
 * presents, such as the flag a chain whose entries are codec_zenoh_decl_ext_keyexpr_inner_t ends them
 * on, which the chain derives from where each entry stands. A flag absent
 * here is the value's own. */
typedef struct {
    /* The `M` flag of `inner_header`, where a parent sets it. */
    sce_optional_bool_t inner_header_m;
} codec_zenoh_decl_ext_keyexpr_inner_overrides_t;

/* The overrides that set no flag. */
static inline codec_zenoh_decl_ext_keyexpr_inner_overrides_t codec_zenoh_decl_ext_keyexpr_inner_overrides_none(void) {
    codec_zenoh_decl_ext_keyexpr_inner_overrides_t none;
    none.inner_header_m = sce_optional_bool_none();
    return none;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_inner_encode_source_overriding(const codec_zenoh_decl_ext_keyexpr_inner_source_t *src, const void *ctx, sce_forge_writer_t *w, codec_zenoh_decl_ext_keyexpr_inner_overrides_t overrides) {
    const uint8_t _e_inner_header = src->inner_header(ctx);
    const uint64_t _e_id = src->id(ctx);
    const codec_zenoh_decl_ext_keyexpr_inner_suffix_bytes_optional_t _e_suffix = src->suffix(ctx);
    uint64_t _f_inner_header = (uint64_t)_e_inner_header;
    _f_inner_header = (_f_inner_header & ~0x1ull) | (uint64_t)(_e_suffix.present);
    if (overrides.inner_header_m.present) {
        _f_inner_header = (_f_inner_header & ~0x2ull) | ((uint64_t)overrides.inner_header_m.value << 1);
    }
    const uint8_t _c_inner_header = (uint8_t)_f_inner_header;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_inner_header));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_e_id)));
    if (_e_suffix.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, _e_suffix.value.data, _e_suffix.value.len));
    }
    return SCE_FORGE_CODEC_OK;
}

/* codec_zenoh_decl_ext_keyexpr_inner_encode_source_overriding with no flag set in place of the
 * value's: the encode every caller but a parent that sets one calls. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_inner_encode_source(const codec_zenoh_decl_ext_keyexpr_inner_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    return codec_zenoh_decl_ext_keyexpr_inner_encode_source_overriding(src, ctx, w, codec_zenoh_decl_ext_keyexpr_inner_overrides_none());
}

/* Whether `self` is a value of codec_zenoh_decl_ext_keyexpr_inner_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_inner_check_value(const codec_zenoh_decl_ext_keyexpr_inner_t *self) {
    if (self->suffix.present && (!sce_forge_length_within_bound(self->suffix.value.len, 128))) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_MAX_BYTES bytes and use
 * `codec_zenoh_decl_ext_keyexpr_inner_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_inner_encode(const codec_zenoh_decl_ext_keyexpr_inner_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_decl_ext_keyexpr_inner_check_value(self));
    return codec_zenoh_decl_ext_keyexpr_inner_encode_source(codec_zenoh_decl_ext_keyexpr_inner_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_decl_ext_keyexpr_inner_encode_to_buf(const codec_zenoh_decl_ext_keyexpr_inner_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_decl_ext_keyexpr_inner_encode(self, &_w);
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
static inline bool codec_zenoh_decl_ext_keyexpr_inner_n(const codec_zenoh_decl_ext_keyexpr_inner_t *self) {
    return (self->inner_header & 0x01) != 0;
}

static inline void codec_zenoh_decl_ext_keyexpr_inner_set_n(codec_zenoh_decl_ext_keyexpr_inner_t *self, bool v) {
    if (v) {
        self->inner_header = (uint8_t)(self->inner_header | 0x01);
    } else {
        self->inner_header = (uint8_t)(self->inner_header & (uint8_t)(~(uint8_t)0x01));
    }
}

static inline bool codec_zenoh_decl_ext_keyexpr_inner_m(const codec_zenoh_decl_ext_keyexpr_inner_t *self) {
    return (self->inner_header & 0x02) != 0;
}

static inline void codec_zenoh_decl_ext_keyexpr_inner_set_m(codec_zenoh_decl_ext_keyexpr_inner_t *self, bool v) {
    if (v) {
        self->inner_header = (uint8_t)(self->inner_header | 0x02);
    } else {
        self->inner_header = (uint8_t)(self->inner_header & (uint8_t)(~(uint8_t)0x02));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_DECL_EXT_KEYEXPR_INNER_H */
