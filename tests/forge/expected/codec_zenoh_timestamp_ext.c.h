// SCE-MAP: codec_zenoh_timestamp_ext:48 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_TIMESTAMP_EXT_H
#define SCE_FORGE_CODEC_ZENOH_TIMESTAMP_EXT_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */
#include "codec_zenoh_timestamp.h"

#define CODEC_ZENOH_TIMESTAMP_EXT_MIN_BYTES 0
#define CODEC_ZENOH_TIMESTAMP_EXT_MAX_BYTES 43

typedef struct {
    /* RFC §synth-5-B embed: nested codec_zenoh_timestamp_t struct (no length prefix on the wire) */
    codec_zenoh_timestamp_t ts;
} codec_zenoh_timestamp_ext_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_timestamp_ext_decode(sce_forge_cursor_t *cursor, codec_zenoh_timestamp_ext_t *out) {
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
    uint64_t _d_ext_size;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_d_ext_size);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    {
        size_t _len = (size_t)(_d_ext_size);
        const uint8_t *_raw = sce_forge_cursor_peek(cursor, _len);
        if (_raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        sce_forge_cursor_t _inner = sce_forge_cursor_init(_raw, _len);
        sce_forge_codec_status_t _st = codec_zenoh_timestamp_decode(&_inner, &out->ts);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
        if (!sce_forge_cursor_advance(cursor, _len)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_zenoh_timestamp_ext_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_timestamp_ext_t is a source by the table
 * codec_zenoh_timestamp_ext_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    codec_zenoh_timestamp_source_ref_t (*ts)(const void *ctx);
} codec_zenoh_timestamp_ext_source_t;

/* A source of codec_zenoh_timestamp_ext_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_timestamp_ext_source_t *table;
    const void *ctx;
} codec_zenoh_timestamp_ext_source_ref_t;

/* A source of codec_zenoh_timestamp_ext_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_timestamp_ext_source_ref_optional, codec_zenoh_timestamp_ext_source_ref_t)

/* The `ts` of the codec_zenoh_timestamp_ext_t at `ctx`. */
static inline codec_zenoh_timestamp_source_ref_t codec_zenoh_timestamp_ext_value_ts(const void *ctx) {
    const codec_zenoh_timestamp_ext_t *self = (const codec_zenoh_timestamp_ext_t *)ctx;
    codec_zenoh_timestamp_source_ref_t body;
    body.table = codec_zenoh_timestamp_value_source();
    body.ctx = &self->ts;
    return body;
}

/* The table that presents a value of codec_zenoh_timestamp_ext_t as a source of it, its
 * context a `const codec_zenoh_timestamp_ext_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_timestamp_ext_source_t *codec_zenoh_timestamp_ext_value_source(void) {
    static const codec_zenoh_timestamp_ext_source_t table = {
        codec_zenoh_timestamp_ext_value_ts,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_timestamp_ext_encode_source(const codec_zenoh_timestamp_ext_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const codec_zenoh_timestamp_source_ref_t _e_ts = src->ts(ctx);
    sce_forge_writer_t _k_ts = sce_forge_writer_init_counting();
    SCE_FORGE_TRY_WRITE(codec_zenoh_timestamp_encode_source(_e_ts.table, _e_ts.ctx, &_k_ts));
    const uint64_t _d_ext_size = (uint64_t)(sce_forge_writer_position(&_k_ts));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_d_ext_size)));
    SCE_FORGE_TRY_WRITE(codec_zenoh_timestamp_encode_source(_e_ts.table, _e_ts.ctx, w));
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_zenoh_timestamp_ext_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_timestamp_ext_check_value(const codec_zenoh_timestamp_ext_t *self) {
    SCE_FORGE_TRY_WRITE(codec_zenoh_timestamp_check_value(&self->ts));
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_TIMESTAMP_EXT_MAX_BYTES bytes and use
 * `codec_zenoh_timestamp_ext_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_timestamp_ext_encode(const codec_zenoh_timestamp_ext_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_timestamp_ext_check_value(self));
    return codec_zenoh_timestamp_ext_encode_source(codec_zenoh_timestamp_ext_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_TIMESTAMP_EXT_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_TIMESTAMP_EXT_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_timestamp_ext_encode_to_buf(const codec_zenoh_timestamp_ext_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_timestamp_ext_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_TIMESTAMP_EXT_H */
