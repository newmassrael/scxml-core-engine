// SCE-MAP: codec_zenoh_query:52 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_QUERY_H
#define SCE_FORGE_CODEC_ZENOH_QUERY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/text.h"

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */
#include "codec_zenoh_ext_entry.h"

#define CODEC_ZENOH_QUERY_MIN_BYTES 2
#define CODEC_ZENOH_QUERY_MAX_BYTES 34359738707

#ifndef SCE_OPTIONAL_BYTES_256
#define SCE_OPTIONAL_BYTES_256
#ifndef SCE_STATIC_BYTES_256
#define SCE_STATIC_BYTES_256
/* SCE Accepted Subset §2.15: a `bytes` of at most 256 bytes. */
typedef struct {
    uint8_t data[256];
    size_t len;
} sce_static_bytes_256_t;

static inline sce_static_bytes_256_t sce_static_bytes_256_of(sce_forge_bytes_view_t bytes) {
    sce_static_bytes_256_t value = { { 0 }, 0 };
    for (size_t i = 0; i < bytes.len && i < 256u; ++i) {
        value.data[i] = bytes.data[i];
    }
    value.len = bytes.len < 256u ? bytes.len : 256u;
    return value;
}
#endif
/* docs/adr/0055: a value of `sce_static_bytes_256_t` that may be absent. */
SCE_OPTIONAL_DECLARE_BYTES(256)
#endif

typedef struct {
    uint8_t header;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_uint8_t consolidation;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_bytes_256_t parameters;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    struct { bool present; struct { codec_zenoh_ext_entry_t data[8]; size_t len; } value; } extensions;
} codec_zenoh_query_t;

/* RFC variant-default-uniformity (C11): designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_zenoh_query_t x = CODEC_ZENOH_QUERY_DEFAULT_INIT;
 * codec_zenoh_query_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_ZENOH_QUERY_DEFAULT_INIT { \
    .header = 0x03u, \
}

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_query_decode(sce_forge_cursor_t *cursor, codec_zenoh_query_t *out) {
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
        out->header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((out->header & 0x20) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->consolidation = sce_optional_uint8_some((uint8_t)(raw[0]));
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->consolidation = sce_optional_uint8_none();
    }
    uint64_t _d_parameters_len = 0;
    if ((out->header & 0x40) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        _d_parameters_len = _v;
    }
    if ((out->header & 0x40) != 0) {
        if ((uint64_t)_d_parameters_len > (uint64_t)SIZE_MAX - 0u) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        size_t _n = (size_t)_d_parameters_len;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->parameters = sce_optional_bytes_256_none();
        out->parameters.value.len = sce_forge_codec_hold(out->parameters.value.data, 256U, raw, _n, &_over);
        out->parameters.present = true;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->parameters = sce_optional_bytes_256_none();
    }
    memset(&out->extensions, 0, sizeof out->extensions);
        if ((out->header & 0x80) != 0) {
            out->extensions.present = true;
            for (;;) {
                if (sce_forge_cursor_remaining(cursor) == 0) break;
                size_t _just = out->extensions.value.len < 8U ? out->extensions.value.len : 8U - 1U;
                sce_forge_codec_status_t _st = sce_forge_codec_note_capacity(codec_zenoh_ext_entry_decode(cursor, &out->extensions.value.data[_just]), &_over);
                if (_st != SCE_FORGE_CODEC_OK) return _st;
                if (out->extensions.value.len < 8U) out->extensions.value.len++; else _over = true;
            }
        }
    if (_over) return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
    return SCE_FORGE_CODEC_OK;
}

/* The `parameters` element of codec_zenoh_query_t: a byte string of 0 or more bytes, the protocol stating no most, viewed where it
 * is held, which outlives the view. A view is made by codec_zenoh_query_parameters_bytes_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const uint8_t *data;
    size_t len;
} codec_zenoh_query_parameters_bytes_t;

/* Make `*out` a view of the `len` bytes at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a byte string outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_zenoh_query_parameters_bytes_make(codec_zenoh_query_parameters_bytes_t *out, const uint8_t *data, size_t len) {
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* A view of codec_zenoh_query_parameters_bytes_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_query_parameters_bytes_optional, codec_zenoh_query_parameters_bytes_t)

/* The `extensions` element of codec_zenoh_query_t: a list of 0 or more entries, the protocol stating no most of codec_zenoh_ext_entry_t,
 * walked where it is held, which outlives the list. A list is made by
 * codec_zenoh_query_extensions_list_make, which walks it once, counts its entries and refuses one
 * outside the bound, so every list holds it (docs/adr/0081 §2) and the
 * encode over a source writes its `count` entries, refusing nothing of
 * the count. */
typedef struct {
    const sce_forge_walk_t *walk;
    const void *ctx;
    const codec_zenoh_ext_entry_source_t *entry;
    size_t count;
} codec_zenoh_query_extensions_list_t;

/* Make `*out` the list `walk` presents over `ctx`, each entry a source of
 * codec_zenoh_ext_entry_t by `entry`: SCE_FORGE_CODEC_OK, or the refusal of a list
 * outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_zenoh_query_extensions_list_make(codec_zenoh_query_extensions_list_t *out, const sce_forge_walk_t *walk, const void *ctx, const codec_zenoh_ext_entry_source_t *entry) {
    size_t count = 0;
    for (const void *at = walk->begin(ctx); at != NULL; at = walk->next(ctx, at)) {
        ++count;
    }
    out->walk = walk;
    out->ctx = ctx;
    out->entry = entry;
    out->count = count;
    return SCE_FORGE_CODEC_OK;
}

/* A list of codec_zenoh_query_extensions_list_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_query_extensions_list_optional, codec_zenoh_query_extensions_list_t)

/* The elements a value of codec_zenoh_query_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_query_t is a source by the table
 * codec_zenoh_query_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*header)(const void *ctx);
    sce_optional_uint8_t (*consolidation)(const void *ctx);
    codec_zenoh_query_parameters_bytes_optional_t (*parameters)(const void *ctx);
    codec_zenoh_query_extensions_list_optional_t (*extensions)(const void *ctx);
} codec_zenoh_query_source_t;

/* A source of codec_zenoh_query_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_query_source_t *table;
    const void *ctx;
} codec_zenoh_query_source_ref_t;

/* A source of codec_zenoh_query_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_query_source_ref_optional, codec_zenoh_query_source_ref_t)

/* The `header` of the codec_zenoh_query_t at `ctx`. */
static inline uint8_t codec_zenoh_query_value_header(const void *ctx) {
    return ((const codec_zenoh_query_t *)ctx)->header;
}

/* The `consolidation` of the codec_zenoh_query_t at `ctx`. */
static inline sce_optional_uint8_t codec_zenoh_query_value_consolidation(const void *ctx) {
    return ((const codec_zenoh_query_t *)ctx)->consolidation;
}

/* The `parameters` of the codec_zenoh_query_t at `ctx`. */
static inline codec_zenoh_query_parameters_bytes_optional_t codec_zenoh_query_value_parameters(const void *ctx) {
    const codec_zenoh_query_t *self = (const codec_zenoh_query_t *)ctx;
    if (!self->parameters.present) {
        return codec_zenoh_query_parameters_bytes_optional_none();
    }
    codec_zenoh_query_parameters_bytes_t view;
    view.data = self->parameters.value.data;
    view.len = self->parameters.value.len;
    return codec_zenoh_query_parameters_bytes_optional_some(view);
}

/* The first entry of the `extensions` of the codec_zenoh_query_t at `ctx`, or NULL where
 * it holds none. */
static inline const void *codec_zenoh_query_value_extensions_begin(const void *ctx) {
    const codec_zenoh_query_t *self = (const codec_zenoh_query_t *)ctx;
    return self->extensions.value.len > 0u ? (const void *)&self->extensions.value.data[0] : NULL;
}

/* The entry after `entry` of the `extensions` of the codec_zenoh_query_t at `ctx`, or
 * NULL where `entry` is its last. */
static inline const void *codec_zenoh_query_value_extensions_next(const void *ctx, const void *entry) {
    const codec_zenoh_query_t *self = (const codec_zenoh_query_t *)ctx;
    const codec_zenoh_ext_entry_t *at = (const codec_zenoh_ext_entry_t *)entry + 1;
    return at < self->extensions.value.data + self->extensions.value.len ? (const void *)at : NULL;
}

/* The `extensions` of the codec_zenoh_query_t at `ctx`. */
static inline codec_zenoh_query_extensions_list_optional_t codec_zenoh_query_value_extensions(const void *ctx) {
    static const sce_forge_walk_t walk = {
        codec_zenoh_query_value_extensions_begin,
        codec_zenoh_query_value_extensions_next,
    };
    const codec_zenoh_query_t *self = (const codec_zenoh_query_t *)ctx;
    if (!self->extensions.present) {
        return codec_zenoh_query_extensions_list_optional_none();
    }
    codec_zenoh_query_extensions_list_t list;
    list.walk = &walk;
    list.ctx = ctx;
    list.entry = codec_zenoh_ext_entry_value_source();
    list.count = self->extensions.value.len;
    return codec_zenoh_query_extensions_list_optional_some(list);
}

/* The table that presents a value of codec_zenoh_query_t as a source of it, its
 * context a `const codec_zenoh_query_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_query_source_t *codec_zenoh_query_value_source(void) {
    static const codec_zenoh_query_source_t table = {
        codec_zenoh_query_value_header,
        codec_zenoh_query_value_consolidation,
        codec_zenoh_query_value_parameters,
        codec_zenoh_query_value_extensions,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_query_encode_source(const codec_zenoh_query_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_header = src->header(ctx);
    const sce_optional_uint8_t _e_consolidation = src->consolidation(ctx);
    const codec_zenoh_query_parameters_bytes_optional_t _e_parameters = src->parameters(ctx);
    const codec_zenoh_query_extensions_list_optional_t _e_extensions = src->extensions(ctx);
    const uint64_t _d_parameters_len = _e_parameters.present ? (uint64_t)(_e_parameters.value.len) : (uint64_t)0;
    uint64_t _f_header = (uint64_t)_e_header;
    _f_header = (_f_header & ~0x20ull) | ((uint64_t)(_e_consolidation.present) << 5);
    _f_header = (_f_header & ~0x40ull) | ((uint64_t)(_e_parameters.present) << 6);
    _f_header = (_f_header & ~0x80ull) | ((uint64_t)(_e_extensions.present) << 7);
    const uint8_t _c_header = (uint8_t)_f_header;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_header));
    if (_e_consolidation.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _e_consolidation.value));
    }
    if ((((uint64_t)_c_header & 0x40ull) != 0)) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_d_parameters_len)));
    }
    if (_e_parameters.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, _e_parameters.value.data, _e_parameters.value.len));
    }
    if (_e_extensions.present) {
        {
            const void *_at = _e_extensions.value.walk->begin(_e_extensions.value.ctx);
            for (size_t _i = 0; _i < _e_extensions.value.count && _at != NULL; ++_i) {
                SCE_FORGE_TRY_WRITE(codec_zenoh_ext_entry_encode_source(_e_extensions.value.entry, _at, w));
                _at = _e_extensions.value.walk->next(_e_extensions.value.ctx, _at);
            }
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_zenoh_query_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_query_check_value(const codec_zenoh_query_t *self) {
    if (self->parameters.present && (!sce_forge_length_within_bound(self->parameters.value.len, 256))) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    if (self->extensions.present) {
        if (!sce_forge_length_within_bound(self->extensions.value.len, 8)) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
        for (size_t _i = 0; _i < self->extensions.value.len; ++_i) {
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_entry_check_value(&self->extensions.value.data[_i]));
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_QUERY_MAX_BYTES bytes and use
 * `codec_zenoh_query_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_query_encode(const codec_zenoh_query_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_query_check_value(self));
    return codec_zenoh_query_encode_source(codec_zenoh_query_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_QUERY_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_QUERY_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_query_encode_to_buf(const codec_zenoh_query_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_query_encode(self, &_w);
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
static inline uint8_t codec_zenoh_query_mid(const codec_zenoh_query_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x1F);
}

static inline void codec_zenoh_query_set_mid(codec_zenoh_query_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x1F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x1F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_query_c(const codec_zenoh_query_t *self) {
    return (self->header & 0x20) != 0;
}

static inline void codec_zenoh_query_set_c(codec_zenoh_query_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x20);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x20));
    }
}

static inline bool codec_zenoh_query_p(const codec_zenoh_query_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_zenoh_query_set_p(codec_zenoh_query_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_zenoh_query_z(const codec_zenoh_query_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_zenoh_query_set_z(codec_zenoh_query_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_QUERY_H */
