// SCE-MAP: codec_ext_timestamp:24 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_EXT_TIMESTAMP_H
#define SCE_FORGE_CODEC_EXT_TIMESTAMP_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */

#define CODEC_EXT_TIMESTAMP_MIN_BYTES 2
#define CODEC_EXT_TIMESTAMP_MAX_BYTES 27

typedef struct {
    uint64_t time;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="16") */
    uint8_t zid[16];
    size_t  zid_len;
} codec_ext_timestamp_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_ext_timestamp_decode(sce_forge_cursor_t *cursor, codec_ext_timestamp_t *out) {
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
    uint64_t time;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &time);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->time = time;
    uint8_t _d_zid_size;
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_zid_size = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        if (_d_zid_size > 16) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        size_t _n = (size_t)_d_zid_size;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->zid, raw, _n);
        out->zid_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    return SCE_FORGE_CODEC_OK;
}

/* The `zid` element of codec_ext_timestamp_t: a byte string of 0 to 16 bytes, viewed where it
 * is held, which outlives the view. A view is made by codec_ext_timestamp_zid_bytes_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const uint8_t *data;
    size_t len;
} codec_ext_timestamp_zid_bytes_t;

/* Make `*out` a view of the `len` bytes at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a byte string outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_ext_timestamp_zid_bytes_make(codec_ext_timestamp_zid_bytes_t *out, const uint8_t *data, size_t len) {
    if (!sce_forge_length_within_bound(len, 16)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_ext_timestamp_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_ext_timestamp_t is a source by the table
 * codec_ext_timestamp_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint64_t (*time)(const void *ctx);
    codec_ext_timestamp_zid_bytes_t (*zid)(const void *ctx);
} codec_ext_timestamp_source_t;

/* A source of codec_ext_timestamp_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_ext_timestamp_source_t *table;
    const void *ctx;
} codec_ext_timestamp_source_ref_t;

/* A source of codec_ext_timestamp_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_ext_timestamp_source_ref_optional, codec_ext_timestamp_source_ref_t)

/* The `time` of the codec_ext_timestamp_t at `ctx`. */
static inline uint64_t codec_ext_timestamp_value_time(const void *ctx) {
    return ((const codec_ext_timestamp_t *)ctx)->time;
}

/* The `zid` of the codec_ext_timestamp_t at `ctx`. */
static inline codec_ext_timestamp_zid_bytes_t codec_ext_timestamp_value_zid(const void *ctx) {
    const codec_ext_timestamp_t *self = (const codec_ext_timestamp_t *)ctx;
    codec_ext_timestamp_zid_bytes_t view;
    view.data = self->zid;
    view.len = self->zid_len;
    return view;
}

/* The table that presents a value of codec_ext_timestamp_t as a source of it, its
 * context a `const codec_ext_timestamp_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_ext_timestamp_source_t *codec_ext_timestamp_value_source(void) {
    static const codec_ext_timestamp_source_t table = {
        codec_ext_timestamp_value_time,
        codec_ext_timestamp_value_zid,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_ext_timestamp_encode_source(const codec_ext_timestamp_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint64_t _e_time = src->time(ctx);
    const codec_ext_timestamp_zid_bytes_t _e_zid = src->zid(ctx);
    const uint8_t _d_zid_size = (uint8_t)(_e_zid.len);
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_e_time)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _d_zid_size));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, _e_zid.data, _e_zid.len));
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_ext_timestamp_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_ext_timestamp_check_value(const codec_ext_timestamp_t *self) {
    if (!sce_forge_length_within_bound(self->zid_len, 16)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_EXT_TIMESTAMP_MAX_BYTES bytes and use
 * `codec_ext_timestamp_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_ext_timestamp_encode(const codec_ext_timestamp_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_ext_timestamp_check_value(self));
    return codec_ext_timestamp_encode_source(codec_ext_timestamp_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_EXT_TIMESTAMP_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_EXT_TIMESTAMP_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_ext_timestamp_encode_to_buf(const codec_ext_timestamp_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_ext_timestamp_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_EXT_TIMESTAMP_H */
