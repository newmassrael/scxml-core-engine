// SCE-MAP: codec_zenoh_source_info:57 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_H
#define SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/text.h"

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */

#define CODEC_ZENOH_SOURCE_INFO_MIN_BYTES 1
#define CODEC_ZENOH_SOURCE_INFO_MAX_BYTES 27

typedef struct {
    uint8_t header;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="16") */
    uint8_t zid[16];
    size_t  zid_len;
    uint32_t eid;
    uint32_t sn;
} codec_zenoh_source_info_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_source_info_decode(sce_forge_cursor_t *cursor, codec_zenoh_source_info_t *out) {
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
    {
        size_t _n = (size_t)((out->header >> 4) & 0xF) + 1;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->zid, raw, _n);
        out->zid_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint32_t eid;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u32(cursor, &eid);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->eid = eid;
    uint32_t sn;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u32(cursor, &sn);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->sn = sn;
    return SCE_FORGE_CODEC_OK;
}

/* The `zid` element of codec_zenoh_source_info_t: a byte string of 1 to 16 bytes, viewed where it
 * is held, which outlives the view. A view is made by codec_zenoh_source_info_zid_bytes_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const uint8_t *data;
    size_t len;
} codec_zenoh_source_info_zid_bytes_t;

/* Make `*out` a view of the `len` bytes at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a byte string outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_zenoh_source_info_zid_bytes_make(codec_zenoh_source_info_zid_bytes_t *out, const uint8_t *data, size_t len) {
    if (len < 1u || !sce_forge_length_within_bound(len, 16)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_zenoh_source_info_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_source_info_t is a source by the table
 * codec_zenoh_source_info_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*header)(const void *ctx);
    codec_zenoh_source_info_zid_bytes_t (*zid)(const void *ctx);
    uint32_t (*eid)(const void *ctx);
    uint32_t (*sn)(const void *ctx);
} codec_zenoh_source_info_source_t;

/* A source of codec_zenoh_source_info_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_source_info_source_t *table;
    const void *ctx;
} codec_zenoh_source_info_source_ref_t;

/* A source of codec_zenoh_source_info_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_source_info_source_ref_optional, codec_zenoh_source_info_source_ref_t)

/* The `header` of the codec_zenoh_source_info_t at `ctx`. */
static inline uint8_t codec_zenoh_source_info_value_header(const void *ctx) {
    return ((const codec_zenoh_source_info_t *)ctx)->header;
}

/* The `zid` of the codec_zenoh_source_info_t at `ctx`. */
static inline codec_zenoh_source_info_zid_bytes_t codec_zenoh_source_info_value_zid(const void *ctx) {
    const codec_zenoh_source_info_t *self = (const codec_zenoh_source_info_t *)ctx;
    codec_zenoh_source_info_zid_bytes_t view;
    view.data = self->zid;
    view.len = self->zid_len;
    return view;
}

/* The `eid` of the codec_zenoh_source_info_t at `ctx`. */
static inline uint32_t codec_zenoh_source_info_value_eid(const void *ctx) {
    return ((const codec_zenoh_source_info_t *)ctx)->eid;
}

/* The `sn` of the codec_zenoh_source_info_t at `ctx`. */
static inline uint32_t codec_zenoh_source_info_value_sn(const void *ctx) {
    return ((const codec_zenoh_source_info_t *)ctx)->sn;
}

/* The table that presents a value of codec_zenoh_source_info_t as a source of it, its
 * context a `const codec_zenoh_source_info_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_source_info_source_t *codec_zenoh_source_info_value_source(void) {
    static const codec_zenoh_source_info_source_t table = {
        codec_zenoh_source_info_value_header,
        codec_zenoh_source_info_value_zid,
        codec_zenoh_source_info_value_eid,
        codec_zenoh_source_info_value_sn,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_source_info_encode_source(const codec_zenoh_source_info_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_header = src->header(ctx);
    const codec_zenoh_source_info_zid_bytes_t _e_zid = src->zid(ctx);
    const uint32_t _e_eid = src->eid(ctx);
    const uint32_t _e_sn = src->sn(ctx);
    uint64_t _f_header = (uint64_t)_e_header;
    _f_header = (_f_header & ~0xF0ull) | ((uint64_t)(_e_zid.len - 1u) << 4);
    const uint8_t _c_header = (uint8_t)_f_header;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_header));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, _e_zid.data, _e_zid.len));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u32(w, (uint32_t)(_e_eid)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u32(w, (uint32_t)(_e_sn)));
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_zenoh_source_info_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_source_info_check_value(const codec_zenoh_source_info_t *self) {
    if (self->zid_len < 1u || !sce_forge_length_within_bound(self->zid_len, 16)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_SOURCE_INFO_MAX_BYTES bytes and use
 * `codec_zenoh_source_info_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_source_info_encode(const codec_zenoh_source_info_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_source_info_check_value(self));
    return codec_zenoh_source_info_encode_source(codec_zenoh_source_info_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_SOURCE_INFO_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_SOURCE_INFO_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_source_info_encode_to_buf(const codec_zenoh_source_info_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_source_info_encode(self, &_w);
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
static inline uint8_t codec_zenoh_source_info_zidlen_m1(const codec_zenoh_source_info_t *self) {
    return (uint8_t)((self->header >> 4) & (uint8_t)0x0F);
}

static inline void codec_zenoh_source_info_set_zidlen_m1(codec_zenoh_source_info_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 4);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 4);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


#endif  /* SCE_FORGE_CODEC_ZENOH_SOURCE_INFO_H */
