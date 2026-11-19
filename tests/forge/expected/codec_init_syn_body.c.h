// SCE-MAP: codec_init_syn_body:30 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_INIT_SYN_BODY_H
#define SCE_FORGE_CODEC_INIT_SYN_BODY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */

#define CODEC_INIT_SYN_BODY_MIN_BYTES 4
#define CODEC_INIT_SYN_BODY_MAX_BYTES 4

/* The `s` group of codec_init_syn_body_t where `S` holds: the members present then
 * (docs/adr/0065 §6 R4). */
typedef struct {
    uint8_t sn_res;
    uint16_t batch_size;
} codec_init_syn_body_s_t;

/* The `s` group of codec_init_syn_body_t, absent where the gates allow no arm. */
SCE_OPTIONAL_DECLARE_NAMED(codec_init_syn_body_s_optional, codec_init_syn_body_s_t)


typedef struct {
    uint8_t version;
    codec_init_syn_body_s_optional_t s;
} codec_init_syn_body_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_init_syn_body_decode(sce_forge_cursor_t *cursor, codec_init_syn_body_t *out, uint8_t s) {
    /* Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
     * `<sce:flag-input>` so codecs that haven't consumed an input via
     * `present-if` yet compile cleanly under -Wunused-parameter. */
    (void)s;
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
    uint8_t _d_sn_res = 0;
    if ((s & 0x01) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_sn_res = (uint8_t)(raw[0]);
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint16_t _d_batch_size = 0;
    if ((s & 0x01) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 2);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_batch_size = (uint16_t)(((uint16_t)raw[0] << 8) | raw[1]);
        if (!sce_forge_cursor_advance(cursor, 2)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    /* The `s` group the gates the decode read select (docs/adr/0065
     * §6 R4). */
    memset(&out->s, 0, sizeof out->s);
    if ((s != 0)) {
        out->s.present = true;
        out->s.value.sn_res = _d_sn_res;
        out->s.value.batch_size = _d_batch_size;
    }
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_init_syn_body_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_init_syn_body_t is a source by the table
 * codec_init_syn_body_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*version)(const void *ctx);
    codec_init_syn_body_s_optional_t (*s)(const void *ctx);
} codec_init_syn_body_source_t;

/* A source of codec_init_syn_body_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_init_syn_body_source_t *table;
    const void *ctx;
} codec_init_syn_body_source_ref_t;

/* A source of codec_init_syn_body_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_init_syn_body_source_ref_optional, codec_init_syn_body_source_ref_t)

/* The `version` of the codec_init_syn_body_t at `ctx`. */
static inline uint8_t codec_init_syn_body_value_version(const void *ctx) {
    return ((const codec_init_syn_body_t *)ctx)->version;
}

/* The `s` group of the codec_init_syn_body_t at `ctx`. */
static inline codec_init_syn_body_s_optional_t codec_init_syn_body_value_s(const void *ctx) {
    return ((const codec_init_syn_body_t *)ctx)->s;
}

/* The table that presents a value of codec_init_syn_body_t as a source of it, its
 * context a `const codec_init_syn_body_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_init_syn_body_source_t *codec_init_syn_body_value_source(void) {
    static const codec_init_syn_body_source_t table = {
        codec_init_syn_body_value_version,
        codec_init_syn_body_value_s,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_init_syn_body_encode_source(const codec_init_syn_body_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_version = src->version(ctx);
    const codec_init_syn_body_s_optional_t _e_s = src->s(ctx);
    const sce_optional_uint8_t _e_sn_res = (_e_s.present ? sce_optional_uint8_some(_e_s.value.sn_res) : sce_optional_uint8_none());
    const sce_optional_uint16_t _e_batch_size = (_e_s.present ? sce_optional_uint16_some(_e_s.value.batch_size) : sce_optional_uint16_none());
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _e_version));
    if (_e_sn_res.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _e_sn_res.value));
    }
    if (_e_batch_size.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_e_batch_size.value >> 8) & 0xFF)));
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(_e_batch_size.value & 0xFF)));
    }
    return SCE_FORGE_CODEC_OK;
}

/* The flag input `S` a source of codec_init_syn_body_t says, which its encode is
 * not handed (docs/adr/0065 §6 R4a). */
static inline uint8_t codec_init_syn_body_input_s(const codec_init_syn_body_source_t *src, const void *ctx) {
    return (uint8_t)((src->s(ctx).present) ? 1u : 0u);
}

/* Whether `self` is a value of codec_init_syn_body_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_init_syn_body_check_value(const codec_init_syn_body_t *self) {
    (void)self;
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_INIT_SYN_BODY_MAX_BYTES bytes and use
 * `codec_init_syn_body_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_init_syn_body_encode(const codec_init_syn_body_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_init_syn_body_check_value(self));
    return codec_init_syn_body_encode_source(codec_init_syn_body_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_INIT_SYN_BODY_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_INIT_SYN_BODY_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_init_syn_body_encode_to_buf(const codec_init_syn_body_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_init_syn_body_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_INIT_SYN_BODY_H */
