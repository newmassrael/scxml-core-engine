// SCE-MAP: codec_zenoh_scout:41 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_SCOUT_H
#define SCE_FORGE_CODEC_ZENOH_SCOUT_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/text.h"

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */

#define CODEC_ZENOH_SCOUT_MIN_BYTES 2
#define CODEC_ZENOH_SCOUT_MAX_BYTES 18

#ifndef SCE_OPTIONAL_BYTES_16
#define SCE_OPTIONAL_BYTES_16
#ifndef SCE_STATIC_BYTES_16
#define SCE_STATIC_BYTES_16
/* SCE Accepted Subset §2.15: a `bytes` of at most 16 bytes. */
typedef struct {
    uint8_t data[16];
    size_t len;
} sce_static_bytes_16_t;

static inline sce_static_bytes_16_t sce_static_bytes_16_of(sce_forge_bytes_view_t bytes) {
    sce_static_bytes_16_t value = { { 0 }, 0 };
    for (size_t i = 0; i < bytes.len && i < 16u; ++i) {
        value.data[i] = bytes.data[i];
    }
    value.len = bytes.len < 16u ? bytes.len : 16u;
    return value;
}
#endif
/* docs/adr/0055: a value of `sce_static_bytes_16_t` that may be absent. */
SCE_OPTIONAL_DECLARE_BYTES(16)
#endif

typedef struct {
    uint8_t version;
    uint8_t cbyte;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_bytes_16_t zid;
} codec_zenoh_scout_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_scout_decode(sce_forge_cursor_t *cursor, codec_zenoh_scout_t *out) {
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
        out->version = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->cbyte = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((out->cbyte & 0x08) != 0) {
        size_t _n = (size_t)((out->cbyte >> 4) & 0xF) + 1;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->zid = sce_optional_bytes_16_none();
        memcpy(out->zid.value.data, raw, _n);
        out->zid.value.len = _n;
        out->zid.present = true;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->zid = sce_optional_bytes_16_none();
    }
    return SCE_FORGE_CODEC_OK;
}

/* The `zid` element of codec_zenoh_scout_t: a byte string of 1 to 16 bytes, viewed where it
 * is held, which outlives the view. A view is made by codec_zenoh_scout_zid_bytes_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const uint8_t *data;
    size_t len;
} codec_zenoh_scout_zid_bytes_t;

/* Make `*out` a view of the `len` bytes at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a byte string outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_zenoh_scout_zid_bytes_make(codec_zenoh_scout_zid_bytes_t *out, const uint8_t *data, size_t len) {
    if (len < 1u || !sce_forge_length_within_bound(len, 16)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* A view of codec_zenoh_scout_zid_bytes_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_scout_zid_bytes_optional, codec_zenoh_scout_zid_bytes_t)

/* The elements a value of codec_zenoh_scout_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_scout_t is a source by the table
 * codec_zenoh_scout_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*version)(const void *ctx);
    uint8_t (*cbyte)(const void *ctx);
    codec_zenoh_scout_zid_bytes_optional_t (*zid)(const void *ctx);
} codec_zenoh_scout_source_t;

/* A source of codec_zenoh_scout_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_scout_source_t *table;
    const void *ctx;
} codec_zenoh_scout_source_ref_t;

/* A source of codec_zenoh_scout_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_scout_source_ref_optional, codec_zenoh_scout_source_ref_t)

/* The `version` of the codec_zenoh_scout_t at `ctx`. */
static inline uint8_t codec_zenoh_scout_value_version(const void *ctx) {
    return ((const codec_zenoh_scout_t *)ctx)->version;
}

/* The `cbyte` of the codec_zenoh_scout_t at `ctx`. */
static inline uint8_t codec_zenoh_scout_value_cbyte(const void *ctx) {
    return ((const codec_zenoh_scout_t *)ctx)->cbyte;
}

/* The `zid` of the codec_zenoh_scout_t at `ctx`. */
static inline codec_zenoh_scout_zid_bytes_optional_t codec_zenoh_scout_value_zid(const void *ctx) {
    const codec_zenoh_scout_t *self = (const codec_zenoh_scout_t *)ctx;
    if (!self->zid.present) {
        return codec_zenoh_scout_zid_bytes_optional_none();
    }
    codec_zenoh_scout_zid_bytes_t view;
    view.data = self->zid.value.data;
    view.len = self->zid.value.len;
    return codec_zenoh_scout_zid_bytes_optional_some(view);
}

/* The table that presents a value of codec_zenoh_scout_t as a source of it, its
 * context a `const codec_zenoh_scout_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_scout_source_t *codec_zenoh_scout_value_source(void) {
    static const codec_zenoh_scout_source_t table = {
        codec_zenoh_scout_value_version,
        codec_zenoh_scout_value_cbyte,
        codec_zenoh_scout_value_zid,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_scout_encode_source(const codec_zenoh_scout_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_version = src->version(ctx);
    const uint8_t _e_cbyte = src->cbyte(ctx);
    const codec_zenoh_scout_zid_bytes_optional_t _e_zid = src->zid(ctx);
    uint64_t _f_cbyte = (uint64_t)_e_cbyte;
    _f_cbyte = (_f_cbyte & ~0x8ull) | ((uint64_t)(_e_zid.present) << 3);
    if (_e_zid.present) {
        _f_cbyte = (_f_cbyte & ~0xF0ull) | ((uint64_t)(_e_zid.value.len - 1u) << 4);
    }
    const uint8_t _c_cbyte = (uint8_t)_f_cbyte;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _e_version));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_cbyte));
    if (_e_zid.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, _e_zid.value.data, _e_zid.value.len));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_zenoh_scout_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_scout_check_value(const codec_zenoh_scout_t *self) {
    if (self->zid.present && (self->zid.value.len < 1u || !sce_forge_length_within_bound(self->zid.value.len, 16))) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_SCOUT_MAX_BYTES bytes and use
 * `codec_zenoh_scout_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_scout_encode(const codec_zenoh_scout_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_scout_check_value(self));
    return codec_zenoh_scout_encode_source(codec_zenoh_scout_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_SCOUT_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_SCOUT_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_scout_encode_to_buf(const codec_zenoh_scout_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_scout_encode(self, &_w);
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
static inline uint8_t codec_zenoh_scout_what(const codec_zenoh_scout_t *self) {
    return (uint8_t)((self->cbyte >> 0) & (uint8_t)0x07);
}

static inline void codec_zenoh_scout_set_what(codec_zenoh_scout_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x07 << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x07) << 0);
    self->cbyte = (uint8_t)((self->cbyte & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_scout_i(const codec_zenoh_scout_t *self) {
    return (self->cbyte & 0x08) != 0;
}

static inline void codec_zenoh_scout_set_i(codec_zenoh_scout_t *self, bool v) {
    if (v) {
        self->cbyte = (uint8_t)(self->cbyte | 0x08);
    } else {
        self->cbyte = (uint8_t)(self->cbyte & (uint8_t)(~(uint8_t)0x08));
    }
}

static inline uint8_t codec_zenoh_scout_zid_len_m1(const codec_zenoh_scout_t *self) {
    return (uint8_t)((self->cbyte >> 4) & (uint8_t)0x0F);
}

static inline void codec_zenoh_scout_set_zid_len_m1(codec_zenoh_scout_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 4);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 4);
    self->cbyte = (uint8_t)((self->cbyte & (uint8_t)~_shifted_mask) | _val);
}


#endif  /* SCE_FORGE_CODEC_ZENOH_SCOUT_H */
