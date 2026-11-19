// SCE-MAP: codec_little_endian:3 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_LITTLE_ENDIAN_H
#define SCE_FORGE_CODEC_LITTLE_ENDIAN_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */

#define CODEC_LITTLE_ENDIAN_MIN_BYTES 4
#define CODEC_LITTLE_ENDIAN_MAX_BYTES 4

typedef struct {
    uint8_t sensor_id;
    uint16_t value;
    uint8_t status;
} codec_little_endian_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_little_endian_decode(sce_forge_cursor_t *cursor, codec_little_endian_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_LITTLE_ENDIAN_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->sensor_id = raw[0];
    out->value = raw[1] | ((uint16_t)raw[2] << 8);
    out->status = raw[3];
    if (!sce_forge_cursor_advance(cursor, CODEC_LITTLE_ENDIAN_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_little_endian_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_little_endian_t is a source by the table
 * codec_little_endian_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*sensor_id)(const void *ctx);
    uint16_t (*value)(const void *ctx);
    uint8_t (*status)(const void *ctx);
} codec_little_endian_source_t;

/* A source of codec_little_endian_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_little_endian_source_t *table;
    const void *ctx;
} codec_little_endian_source_ref_t;

/* A source of codec_little_endian_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_little_endian_source_ref_optional, codec_little_endian_source_ref_t)

/* The `sensor_id` of the codec_little_endian_t at `ctx`. */
static inline uint8_t codec_little_endian_value_sensor_id(const void *ctx) {
    return ((const codec_little_endian_t *)ctx)->sensor_id;
}

/* The `value` of the codec_little_endian_t at `ctx`. */
static inline uint16_t codec_little_endian_value_value(const void *ctx) {
    return ((const codec_little_endian_t *)ctx)->value;
}

/* The `status` of the codec_little_endian_t at `ctx`. */
static inline uint8_t codec_little_endian_value_status(const void *ctx) {
    return ((const codec_little_endian_t *)ctx)->status;
}

/* The table that presents a value of codec_little_endian_t as a source of it, its
 * context a `const codec_little_endian_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_little_endian_source_t *codec_little_endian_value_source(void) {
    static const codec_little_endian_source_t table = {
        codec_little_endian_value_sensor_id,
        codec_little_endian_value_value,
        codec_little_endian_value_status,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_little_endian_encode_source(const codec_little_endian_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_sensor_id = src->sensor_id(ctx);
    const uint16_t _e_value = src->value(ctx);
    const uint8_t _e_status = src->status(ctx);
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _e_sensor_id));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(_e_value & 0xFF)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_e_value >> 8) & 0xFF)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _e_status));
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_little_endian_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_little_endian_check_value(const codec_little_endian_t *self) {
    (void)self;
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_LITTLE_ENDIAN_MAX_BYTES bytes and use
 * `codec_little_endian_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_little_endian_encode(const codec_little_endian_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_little_endian_check_value(self));
    return codec_little_endian_encode_source(codec_little_endian_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_LITTLE_ENDIAN_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_LITTLE_ENDIAN_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_little_endian_encode_to_buf(const codec_little_endian_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_little_endian_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_LITTLE_ENDIAN_H */
