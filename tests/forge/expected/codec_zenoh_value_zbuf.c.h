// SCE-MAP: codec_zenoh_value_zbuf:27 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_VALUE_ZBUF_H
#define SCE_FORGE_CODEC_ZENOH_VALUE_ZBUF_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/text.h"

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */
#include "codec_zenoh_value_slice.h"

#define CODEC_ZENOH_VALUE_ZBUF_MIN_BYTES 0
#define CODEC_ZENOH_VALUE_ZBUF_MAX_BYTES 1123

#ifndef SCE_OPTIONAL_BYTES_32
#define SCE_OPTIONAL_BYTES_32
#ifndef SCE_STATIC_BYTES_32
#define SCE_STATIC_BYTES_32
/* SCE Accepted Subset §2.15: a `bytes` of at most 32 bytes. */
typedef struct {
    uint8_t data[32];
    size_t len;
} sce_static_bytes_32_t;

static inline sce_static_bytes_32_t sce_static_bytes_32_of(sce_forge_bytes_view_t bytes) {
    sce_static_bytes_32_t value = { { 0 }, 0 };
    for (size_t i = 0; i < bytes.len && i < 32u; ++i) {
        value.data[i] = bytes.data[i];
    }
    value.len = bytes.len < 32u ? bytes.len : 32u;
    return value;
}
#endif
/* docs/adr/0055: a value of `sce_static_bytes_32_t` that may be absent. */
SCE_OPTIONAL_DECLARE_BYTES(32)
#endif

#ifndef CODEC_ZENOH_VALUE_SLICE_LIST_4
#define CODEC_ZENOH_VALUE_SLICE_LIST_4
/* A list of at most 4 entries of codec_zenoh_value_slice_t. */
typedef struct {
    size_t len;
    codec_zenoh_value_slice_t data[4];
} codec_zenoh_value_slice_list_4_t;

/* The list of the `len` elements at `data`, which the build proved fit its bound. */
static inline codec_zenoh_value_slice_list_4_t codec_zenoh_value_slice_list_4_of(const codec_zenoh_value_slice_t *data, size_t len) {
    codec_zenoh_value_slice_list_4_t value = { .len = 0u };
    for (size_t i = 0u; i < len && i < 4u; ++i) {
        value.data[i] = data[i];
    }
    value.len = len < 4u ? len : 4u;
    return value;
}
#endif

/* The arms of the `after_shm` group of codec_zenoh_value_zbuf_t (docs/adr/0065 §6 R4). */
typedef enum {
    CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_AFTER_SHM,
    CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_NOT_AFTER_SHM,
} codec_zenoh_value_zbuf_after_shm_kind_t;

/* The `after_shm` group of codec_zenoh_value_zbuf_t where `after_shm` holds: the members present
 * then. */
typedef struct {
    uint64_t encoding;
    codec_zenoh_value_slice_list_4_t slices;
    uint64_t value_len;
} codec_zenoh_value_zbuf_after_shm_after_shm_t;

/* The `after_shm` group of codec_zenoh_value_zbuf_t where `after_shm` does not hold: the members present
 * then. */
typedef struct {
    sce_static_bytes_32_t value;
} codec_zenoh_value_zbuf_after_shm_not_after_shm_t;

/* The `after_shm` group of codec_zenoh_value_zbuf_t: the arm the value holds, which `kind`
 * names, each the members present while the gates hold its values
 * (docs/adr/0065 §6 R4). */
typedef struct {
    codec_zenoh_value_zbuf_after_shm_kind_t kind;
    union {
        codec_zenoh_value_zbuf_after_shm_after_shm_t after_shm;
        codec_zenoh_value_zbuf_after_shm_not_after_shm_t not_after_shm;
    } arm;
} codec_zenoh_value_zbuf_after_shm_t;


typedef struct {
    codec_zenoh_value_zbuf_after_shm_t after_shm;
} codec_zenoh_value_zbuf_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_value_zbuf_decode(sce_forge_cursor_t *cursor, codec_zenoh_value_zbuf_t *out, uint8_t after_shm) {
    /* Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
     * `<sce:flag-input>` so codecs that haven't consumed an input via
     * `present-if` yet compile cleanly under -Wunused-parameter. */
    (void)after_shm;
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
    uint64_t _d_value_len;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_d_value_len);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    /* The `after_shm` group the gates the decode read select (docs/adr/0065
     * §6 R4), opened before the members it reads in place. */
    memset(&out->after_shm, 0, sizeof out->after_shm);
    codec_zenoh_value_slice_list_4_t *_g_slices = NULL;
    sce_static_bytes_32_t *_g_value = NULL;
    if ((after_shm != 0)) {
        out->after_shm.kind = CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_AFTER_SHM;
        _g_slices = &out->after_shm.arm.after_shm.slices;
    } else {
        out->after_shm.kind = CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_NOT_AFTER_SHM;
        _g_value = &out->after_shm.arm.not_after_shm.value;
    }
    if ((after_shm & 0x01) == 0) {
        if (_d_value_len > 4294967295ULL) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        if ((uint64_t)_d_value_len > (uint64_t)SIZE_MAX - 0u) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        size_t _n = (size_t)_d_value_len;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _g_value->len = sce_forge_codec_hold(_g_value->data, 32U, raw, _n, &_over);
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint64_t _d_encoding = 0;
    if ((after_shm & 0x01) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        _d_encoding = _v;
    }
    uint64_t _d_slice_count = 0;
    if ((after_shm & 0x01) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        _d_slice_count = _v;
    }
    if ((after_shm & 0x01) != 0) {
        size_t _n = (size_t)_d_slice_count;
        for (size_t _i = 0; _i < _n; ++_i) {
            size_t _at = _i < 4U ? _i : 4U - 1U;
            if (_i >= 4U) _over = true;
            sce_forge_codec_status_t _st = sce_forge_codec_note_capacity(codec_zenoh_value_slice_decode(cursor, &_g_slices->data[_at]), &_over);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
        }
        _g_slices->len = _n < 4U ? _n : 4U;
    }
    /* The numbers of the `after_shm` group, read into locals, set in the arm
     * the decode opened (docs/adr/0065 §6 R4). */
    if (out->after_shm.kind == CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_AFTER_SHM) {
        out->after_shm.arm.after_shm.encoding = _d_encoding;
        out->after_shm.arm.after_shm.value_len = _d_value_len;
    }
    if (_over) return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
    return SCE_FORGE_CODEC_OK;
}

/* The `slices` element of codec_zenoh_value_zbuf_t: a list of 0 or more entries, the protocol stating no most of codec_zenoh_value_slice_t,
 * walked where it is held, which outlives the list. A list is made by
 * codec_zenoh_value_zbuf_slices_list_make, which walks it once, counts its entries and refuses one
 * outside the bound, so every list holds it (docs/adr/0081 §2) and the
 * encode over a source writes its `count` entries, refusing nothing of
 * the count. */
typedef struct {
    const sce_forge_walk_t *walk;
    const void *ctx;
    const codec_zenoh_value_slice_source_t *entry;
    size_t count;
} codec_zenoh_value_zbuf_slices_list_t;

/* Make `*out` the list `walk` presents over `ctx`, each entry a source of
 * codec_zenoh_value_slice_t by `entry`: SCE_FORGE_CODEC_OK, or the refusal of a list
 * outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_zenoh_value_zbuf_slices_list_make(codec_zenoh_value_zbuf_slices_list_t *out, const sce_forge_walk_t *walk, const void *ctx, const codec_zenoh_value_slice_source_t *entry) {
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

/* A list of codec_zenoh_value_zbuf_slices_list_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_value_zbuf_slices_list_optional, codec_zenoh_value_zbuf_slices_list_t)

/* The `value` element of codec_zenoh_value_zbuf_t: a byte string of 0 to 4294967295 bytes, viewed where it
 * is held, which outlives the view. A view is made by codec_zenoh_value_zbuf_value_bytes_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const uint8_t *data;
    size_t len;
} codec_zenoh_value_zbuf_value_bytes_t;

/* Make `*out` a view of the `len` bytes at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a byte string outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_zenoh_value_zbuf_value_bytes_make(codec_zenoh_value_zbuf_value_bytes_t *out, const uint8_t *data, size_t len) {
    if (!sce_forge_length_within_bound(len, 4294967295)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* A view of codec_zenoh_value_zbuf_value_bytes_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_value_zbuf_value_bytes_optional, codec_zenoh_value_zbuf_value_bytes_t)

/* The `after_shm` group a source of codec_zenoh_value_zbuf_t presents where `after_shm` holds: the members present
 * then. */
typedef struct {
    uint64_t encoding;
    codec_zenoh_value_zbuf_slices_list_t slices;
    uint64_t value_len;
} codec_zenoh_value_zbuf_after_shm_after_shm_ref_t;

/* The `after_shm` group a source of codec_zenoh_value_zbuf_t presents where `after_shm` does not hold: the members present
 * then. */
typedef struct {
    codec_zenoh_value_zbuf_value_bytes_t value;
} codec_zenoh_value_zbuf_after_shm_not_after_shm_ref_t;

/* The `after_shm` group a source of codec_zenoh_value_zbuf_t presents: the arm the value holds, which `kind`
 * names, each the members present while the gates hold its values
 * (docs/adr/0065 §6 R4). */
typedef struct {
    codec_zenoh_value_zbuf_after_shm_kind_t kind;
    union {
        codec_zenoh_value_zbuf_after_shm_after_shm_ref_t after_shm;
        codec_zenoh_value_zbuf_after_shm_not_after_shm_ref_t not_after_shm;
    } arm;
} codec_zenoh_value_zbuf_after_shm_ref_t;

/* The elements a value of codec_zenoh_value_zbuf_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_value_zbuf_t is a source by the table
 * codec_zenoh_value_zbuf_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    codec_zenoh_value_zbuf_after_shm_ref_t (*after_shm)(const void *ctx);
} codec_zenoh_value_zbuf_source_t;

/* A source of codec_zenoh_value_zbuf_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_value_zbuf_source_t *table;
    const void *ctx;
} codec_zenoh_value_zbuf_source_ref_t;

/* A source of codec_zenoh_value_zbuf_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_value_zbuf_source_ref_optional, codec_zenoh_value_zbuf_source_ref_t)

/* The first entry of the `slices` of the `after_shm` group of the
 * codec_zenoh_value_zbuf_t at `ctx`, or NULL where it holds none. */
static inline const void *codec_zenoh_value_zbuf_value_after_shm_after_shm_slices_begin(const void *ctx) {
    const codec_zenoh_value_zbuf_t *self = (const codec_zenoh_value_zbuf_t *)ctx;
    return self->after_shm.arm.after_shm.slices.len > 0u ? (const void *)&self->after_shm.arm.after_shm.slices.data[0] : NULL;
}

/* The entry after `entry` of the `slices` of the `after_shm` group of
 * the codec_zenoh_value_zbuf_t at `ctx`, or NULL where `entry` is its last. */
static inline const void *codec_zenoh_value_zbuf_value_after_shm_after_shm_slices_next(const void *ctx, const void *entry) {
    const codec_zenoh_value_zbuf_t *self = (const codec_zenoh_value_zbuf_t *)ctx;
    const codec_zenoh_value_slice_t *at = (const codec_zenoh_value_slice_t *)entry + 1;
    return at < self->after_shm.arm.after_shm.slices.data + self->after_shm.arm.after_shm.slices.len ? (const void *)at : NULL;
}

/* The `after_shm` group of the codec_zenoh_value_zbuf_t at `ctx`, each member presented
 * where the value holds it. */
static inline codec_zenoh_value_zbuf_after_shm_ref_t codec_zenoh_value_zbuf_value_after_shm(const void *ctx) {
    static const sce_forge_walk_t codec_zenoh_value_zbuf_value_after_shm_after_shm_slices_walk = {
        codec_zenoh_value_zbuf_value_after_shm_after_shm_slices_begin,
        codec_zenoh_value_zbuf_value_after_shm_after_shm_slices_next,
    };
    const codec_zenoh_value_zbuf_t *self = (const codec_zenoh_value_zbuf_t *)ctx;
    codec_zenoh_value_zbuf_after_shm_ref_t ref;
    memset(&ref, 0, sizeof ref);
    ref.kind = self->after_shm.kind;
    if (self->after_shm.kind == CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_AFTER_SHM) {
        ref.arm.after_shm.encoding = self->after_shm.arm.after_shm.encoding;
        ref.arm.after_shm.slices.walk = &codec_zenoh_value_zbuf_value_after_shm_after_shm_slices_walk;
        ref.arm.after_shm.slices.ctx = ctx;
        ref.arm.after_shm.slices.entry = codec_zenoh_value_slice_value_source();
        ref.arm.after_shm.slices.count = self->after_shm.arm.after_shm.slices.len;
        ref.arm.after_shm.value_len = self->after_shm.arm.after_shm.value_len;
    }
    if (self->after_shm.kind == CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_NOT_AFTER_SHM) {
        ref.arm.not_after_shm.value.data = self->after_shm.arm.not_after_shm.value.data;
        ref.arm.not_after_shm.value.len = self->after_shm.arm.not_after_shm.value.len;
    }
    return ref;
}

/* The table that presents a value of codec_zenoh_value_zbuf_t as a source of it, its
 * context a `const codec_zenoh_value_zbuf_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_value_zbuf_source_t *codec_zenoh_value_zbuf_value_source(void) {
    static const codec_zenoh_value_zbuf_source_t table = {
        codec_zenoh_value_zbuf_value_after_shm,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_value_zbuf_encode_source(const codec_zenoh_value_zbuf_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const codec_zenoh_value_zbuf_after_shm_ref_t _e_after_shm = src->after_shm(ctx);
    const sce_optional_uint64_t _e_encoding = (_e_after_shm.kind == CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_AFTER_SHM ? sce_optional_uint64_some(_e_after_shm.arm.after_shm.encoding) : sce_optional_uint64_none());
    const codec_zenoh_value_zbuf_slices_list_optional_t _e_slices = (_e_after_shm.kind == CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_AFTER_SHM ? codec_zenoh_value_zbuf_slices_list_optional_some(_e_after_shm.arm.after_shm.slices) : codec_zenoh_value_zbuf_slices_list_optional_none());
    const sce_optional_uint64_t _e_value_len = (_e_after_shm.kind == CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_AFTER_SHM ? sce_optional_uint64_some(_e_after_shm.arm.after_shm.value_len) : sce_optional_uint64_none());
    const codec_zenoh_value_zbuf_value_bytes_optional_t _e_value = (_e_after_shm.kind == CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_NOT_AFTER_SHM ? codec_zenoh_value_zbuf_value_bytes_optional_some(_e_after_shm.arm.not_after_shm.value) : codec_zenoh_value_zbuf_value_bytes_optional_none());
    const uint8_t after_shm = (uint8_t)((_e_after_shm.kind == CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_AFTER_SHM) ? 1u : 0u);
    (void)after_shm;
    const uint64_t _d_value_len = _e_value.present ? (uint64_t)(_e_value.value.len) : (_e_value_len.present ? _e_value_len.value : (uint64_t)0);
    const uint64_t _d_slice_count = _e_slices.present ? (uint64_t)(_e_slices.value.count) : (uint64_t)0;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_d_value_len)));
    if (_e_value.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, _e_value.value.data, _e_value.value.len));
    }
    if (_e_encoding.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_e_encoding.value)));
    }
    if ((((uint64_t)after_shm & 0x1ull) != 0)) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_d_slice_count)));
    }
    if (_e_slices.present) {
        {
            const void *_at = _e_slices.value.walk->begin(_e_slices.value.ctx);
            for (size_t _i = 0; _i < _e_slices.value.count && _at != NULL; ++_i) {
                SCE_FORGE_TRY_WRITE(codec_zenoh_value_slice_encode_source(_e_slices.value.entry, _at, w));
                _at = _e_slices.value.walk->next(_e_slices.value.ctx, _at);
            }
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* The flag input `after_shm` a source of codec_zenoh_value_zbuf_t says, which its encode is
 * not handed (docs/adr/0065 §6 R4a). */
static inline uint8_t codec_zenoh_value_zbuf_input_after_shm(const codec_zenoh_value_zbuf_source_t *src, const void *ctx) {
    return (uint8_t)((src->after_shm(ctx).kind == CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_AFTER_SHM) ? 1u : 0u);
}

/* Whether `self` is a value of codec_zenoh_value_zbuf_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_value_zbuf_check_value(const codec_zenoh_value_zbuf_t *self) {
    if (self->after_shm.kind == CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_AFTER_SHM) {
        if (!sce_forge_length_within_bound(self->after_shm.arm.after_shm.slices.len, 4)) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
        for (size_t _i = 0; _i < self->after_shm.arm.after_shm.slices.len; ++_i) {
            SCE_FORGE_TRY_WRITE(codec_zenoh_value_slice_check_value(&self->after_shm.arm.after_shm.slices.data[_i]));
        }
    }
    if (self->after_shm.kind == CODEC_ZENOH_VALUE_ZBUF_AFTER_SHM_NOT_AFTER_SHM) {
        if (!sce_forge_length_within_bound(self->after_shm.arm.not_after_shm.value.len, 32)) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_VALUE_ZBUF_MAX_BYTES bytes and use
 * `codec_zenoh_value_zbuf_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_value_zbuf_encode(const codec_zenoh_value_zbuf_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_value_zbuf_check_value(self));
    return codec_zenoh_value_zbuf_encode_source(codec_zenoh_value_zbuf_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_VALUE_ZBUF_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_VALUE_ZBUF_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_value_zbuf_encode_to_buf(const codec_zenoh_value_zbuf_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_value_zbuf_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_VALUE_ZBUF_H */
