// SCE-MAP: codec_zenoh_value_slice:15 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_VALUE_SLICE_H
#define SCE_FORGE_CODEC_ZENOH_VALUE_SLICE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */

#define CODEC_ZENOH_VALUE_SLICE_MIN_BYTES 1
#define CODEC_ZENOH_VALUE_SLICE_MAX_BYTES 26

typedef struct {
    uint8_t kind;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="16") */
    uint8_t data[16];
    size_t  data_len;
} codec_zenoh_value_slice_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_value_slice_decode(sce_forge_cursor_t *cursor, codec_zenoh_value_slice_t *out) {
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
        out->kind = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint64_t _d_len;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_d_len);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    {
        if (_d_len > 4294967295ULL) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        if ((uint64_t)_d_len > (uint64_t)SIZE_MAX - 0u) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        size_t _n = (size_t)_d_len;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->data_len = sce_forge_codec_hold(out->data, 16U, raw, _n, &_over);
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if (_over) return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
    return SCE_FORGE_CODEC_OK;
}

/* The `data` element of codec_zenoh_value_slice_t: a byte string of 0 to 4294967295 bytes, viewed where it
 * is held, which outlives the view. A view is made by codec_zenoh_value_slice_data_bytes_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const uint8_t *data;
    size_t len;
} codec_zenoh_value_slice_data_bytes_t;

/* Make `*out` a view of the `len` bytes at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a byte string outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_zenoh_value_slice_data_bytes_make(codec_zenoh_value_slice_data_bytes_t *out, const uint8_t *data, size_t len) {
    if (!sce_forge_length_within_bound(len, 4294967295)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_zenoh_value_slice_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_value_slice_t is a source by the table
 * codec_zenoh_value_slice_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*kind)(const void *ctx);
    codec_zenoh_value_slice_data_bytes_t (*data)(const void *ctx);
} codec_zenoh_value_slice_source_t;

/* A source of codec_zenoh_value_slice_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_value_slice_source_t *table;
    const void *ctx;
} codec_zenoh_value_slice_source_ref_t;

/* A source of codec_zenoh_value_slice_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_value_slice_source_ref_optional, codec_zenoh_value_slice_source_ref_t)

/* The `kind` of the codec_zenoh_value_slice_t at `ctx`. */
static inline uint8_t codec_zenoh_value_slice_value_kind(const void *ctx) {
    return ((const codec_zenoh_value_slice_t *)ctx)->kind;
}

/* The `data` of the codec_zenoh_value_slice_t at `ctx`. */
static inline codec_zenoh_value_slice_data_bytes_t codec_zenoh_value_slice_value_data(const void *ctx) {
    const codec_zenoh_value_slice_t *self = (const codec_zenoh_value_slice_t *)ctx;
    codec_zenoh_value_slice_data_bytes_t view;
    view.data = self->data;
    view.len = self->data_len;
    return view;
}

/* The table that presents a value of codec_zenoh_value_slice_t as a source of it, its
 * context a `const codec_zenoh_value_slice_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_value_slice_source_t *codec_zenoh_value_slice_value_source(void) {
    static const codec_zenoh_value_slice_source_t table = {
        codec_zenoh_value_slice_value_kind,
        codec_zenoh_value_slice_value_data,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_value_slice_encode_source(const codec_zenoh_value_slice_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_kind = src->kind(ctx);
    const codec_zenoh_value_slice_data_bytes_t _e_data = src->data(ctx);
    const uint64_t _d_len = (uint64_t)(_e_data.len);
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _e_kind));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_d_len)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, _e_data.data, _e_data.len));
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_zenoh_value_slice_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_value_slice_check_value(const codec_zenoh_value_slice_t *self) {
    if (!sce_forge_length_within_bound(self->data_len, 16)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_VALUE_SLICE_MAX_BYTES bytes and use
 * `codec_zenoh_value_slice_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_value_slice_encode(const codec_zenoh_value_slice_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_value_slice_check_value(self));
    return codec_zenoh_value_slice_encode_source(codec_zenoh_value_slice_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_VALUE_SLICE_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_VALUE_SLICE_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_value_slice_encode_to_buf(const codec_zenoh_value_slice_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_value_slice_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_VALUE_SLICE_H */
