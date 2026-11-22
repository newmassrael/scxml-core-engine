// SCE-MAP: codec_fixed_after_lengthref:19 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_FIXED_AFTER_LENGTHREF_H
#define SCE_FORGE_CODEC_FIXED_AFTER_LENGTHREF_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/text.h"

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */

#define CODEC_FIXED_AFTER_LENGTHREF_MIN_BYTES 8
#define CODEC_FIXED_AFTER_LENGTHREF_MAX_BYTES 1508

typedef struct {
    uint8_t header;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="1500") */
    uint8_t payload[1500];
    size_t  payload_len;
    uint32_t crc32;
} codec_fixed_after_lengthref_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_fixed_after_lengthref_decode(sce_forge_cursor_t *cursor, codec_fixed_after_lengthref_t *out) {
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
    uint16_t _d_payload_len;
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 2);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_payload_len = raw[0] | ((uint16_t)raw[1] << 8);
        if (!sce_forge_cursor_advance(cursor, 2)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        if (_d_payload_len > 1500) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        size_t _n = (size_t)_d_payload_len;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->payload, raw, _n);
        out->payload_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 4);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->crc32 = raw[0] | ((uint32_t)raw[1] << 8) | ((uint32_t)raw[2] << 16) | ((uint32_t)raw[3] << 24);
        if (!sce_forge_cursor_advance(cursor, 4)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    return SCE_FORGE_CODEC_OK;
}

/* The `payload` element of codec_fixed_after_lengthref_t: a byte string of 0 to 1500 bytes, viewed where it
 * is held, which outlives the view. A view is made by codec_fixed_after_lengthref_payload_bytes_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const uint8_t *data;
    size_t len;
} codec_fixed_after_lengthref_payload_bytes_t;

/* Make `*out` a view of the `len` bytes at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a byte string outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_fixed_after_lengthref_payload_bytes_make(codec_fixed_after_lengthref_payload_bytes_t *out, const uint8_t *data, size_t len) {
    if (!sce_forge_length_within_bound(len, 1500)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_fixed_after_lengthref_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_fixed_after_lengthref_t is a source by the table
 * codec_fixed_after_lengthref_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*header)(const void *ctx);
    codec_fixed_after_lengthref_payload_bytes_t (*payload)(const void *ctx);
    uint32_t (*crc32)(const void *ctx);
} codec_fixed_after_lengthref_source_t;

/* A source of codec_fixed_after_lengthref_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_fixed_after_lengthref_source_t *table;
    const void *ctx;
} codec_fixed_after_lengthref_source_ref_t;

/* A source of codec_fixed_after_lengthref_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_fixed_after_lengthref_source_ref_optional, codec_fixed_after_lengthref_source_ref_t)

/* The `header` of the codec_fixed_after_lengthref_t at `ctx`. */
static inline uint8_t codec_fixed_after_lengthref_value_header(const void *ctx) {
    return ((const codec_fixed_after_lengthref_t *)ctx)->header;
}

/* The `payload` of the codec_fixed_after_lengthref_t at `ctx`. */
static inline codec_fixed_after_lengthref_payload_bytes_t codec_fixed_after_lengthref_value_payload(const void *ctx) {
    const codec_fixed_after_lengthref_t *self = (const codec_fixed_after_lengthref_t *)ctx;
    codec_fixed_after_lengthref_payload_bytes_t view;
    view.data = self->payload;
    view.len = self->payload_len;
    return view;
}

/* The `crc32` of the codec_fixed_after_lengthref_t at `ctx`. */
static inline uint32_t codec_fixed_after_lengthref_value_crc32(const void *ctx) {
    return ((const codec_fixed_after_lengthref_t *)ctx)->crc32;
}

/* The table that presents a value of codec_fixed_after_lengthref_t as a source of it, its
 * context a `const codec_fixed_after_lengthref_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_fixed_after_lengthref_source_t *codec_fixed_after_lengthref_value_source(void) {
    static const codec_fixed_after_lengthref_source_t table = {
        codec_fixed_after_lengthref_value_header,
        codec_fixed_after_lengthref_value_payload,
        codec_fixed_after_lengthref_value_crc32,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_fixed_after_lengthref_encode_source(const codec_fixed_after_lengthref_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_header = src->header(ctx);
    const codec_fixed_after_lengthref_payload_bytes_t _e_payload = src->payload(ctx);
    const uint32_t _e_crc32 = src->crc32(ctx);
    const uint16_t _d_payload_len = (uint16_t)(_e_payload.len);
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _e_header));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(_d_payload_len & 0xFF)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_d_payload_len >> 8) & 0xFF)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, _e_payload.data, _e_payload.len));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(_e_crc32 & 0xFF)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_e_crc32 >> 8) & 0xFF)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_e_crc32 >> 16) & 0xFF)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_e_crc32 >> 24) & 0xFF)));
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_fixed_after_lengthref_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_fixed_after_lengthref_check_value(const codec_fixed_after_lengthref_t *self) {
    if (!sce_forge_length_within_bound(self->payload_len, 1500)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_FIXED_AFTER_LENGTHREF_MAX_BYTES bytes and use
 * `codec_fixed_after_lengthref_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_fixed_after_lengthref_encode(const codec_fixed_after_lengthref_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_fixed_after_lengthref_check_value(self));
    return codec_fixed_after_lengthref_encode_source(codec_fixed_after_lengthref_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_FIXED_AFTER_LENGTHREF_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_FIXED_AFTER_LENGTHREF_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_fixed_after_lengthref_encode_to_buf(const codec_fixed_after_lengthref_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_fixed_after_lengthref_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_FIXED_AFTER_LENGTHREF_H */
