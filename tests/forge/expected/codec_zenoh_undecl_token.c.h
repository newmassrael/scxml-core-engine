// SCE-MAP: codec_zenoh_undecl_token:16 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_UNDECL_TOKEN_H
#define SCE_FORGE_CODEC_ZENOH_UNDECL_TOKEN_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */
#include "codec_zenoh_decl_ext_keyexpr.h"

#define CODEC_ZENOH_UNDECL_TOKEN_MIN_BYTES 0
#define CODEC_ZENOH_UNDECL_TOKEN_MAX_BYTES 65560

typedef struct {
    uint32_t id;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    struct { bool present; codec_zenoh_decl_ext_keyexpr_t value; } ext_keyexpr;
} codec_zenoh_undecl_token_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_undecl_token_decode(sce_forge_cursor_t *cursor, codec_zenoh_undecl_token_t *out, uint8_t z) {
    /* Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
     * `<sce:flag-input>` so codecs that haven't consumed an input via
     * `present-if` yet compile cleanly under -Wunused-parameter. */
    (void)z;
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
    uint32_t id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u32(cursor, &id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->id = id;
    memset(&out->ext_keyexpr, 0, sizeof out->ext_keyexpr);
    if ((z & 0x01) != 0) {
        sce_forge_codec_status_t _st = sce_forge_codec_note_capacity(codec_zenoh_decl_ext_keyexpr_decode(cursor, &out->ext_keyexpr.value), &_over);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
        out->ext_keyexpr.present = true;
    }
    if (_over) return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_zenoh_undecl_token_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_undecl_token_t is a source by the table
 * codec_zenoh_undecl_token_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint32_t (*id)(const void *ctx);
    codec_zenoh_decl_ext_keyexpr_source_ref_optional_t (*ext_keyexpr)(const void *ctx);
} codec_zenoh_undecl_token_source_t;

/* A source of codec_zenoh_undecl_token_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_undecl_token_source_t *table;
    const void *ctx;
} codec_zenoh_undecl_token_source_ref_t;

/* A source of codec_zenoh_undecl_token_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_undecl_token_source_ref_optional, codec_zenoh_undecl_token_source_ref_t)

/* The `id` of the codec_zenoh_undecl_token_t at `ctx`. */
static inline uint32_t codec_zenoh_undecl_token_value_id(const void *ctx) {
    return ((const codec_zenoh_undecl_token_t *)ctx)->id;
}

/* The `ext_keyexpr` of the codec_zenoh_undecl_token_t at `ctx`. */
static inline codec_zenoh_decl_ext_keyexpr_source_ref_optional_t codec_zenoh_undecl_token_value_ext_keyexpr(const void *ctx) {
    const codec_zenoh_undecl_token_t *self = (const codec_zenoh_undecl_token_t *)ctx;
    if (!self->ext_keyexpr.present) {
        return codec_zenoh_decl_ext_keyexpr_source_ref_optional_none();
    }
    codec_zenoh_decl_ext_keyexpr_source_ref_t body;
    body.table = codec_zenoh_decl_ext_keyexpr_value_source();
    body.ctx = &self->ext_keyexpr.value;
    return codec_zenoh_decl_ext_keyexpr_source_ref_optional_some(body);
}

/* The table that presents a value of codec_zenoh_undecl_token_t as a source of it, its
 * context a `const codec_zenoh_undecl_token_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_undecl_token_source_t *codec_zenoh_undecl_token_value_source(void) {
    static const codec_zenoh_undecl_token_source_t table = {
        codec_zenoh_undecl_token_value_id,
        codec_zenoh_undecl_token_value_ext_keyexpr,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_undecl_token_encode_source(const codec_zenoh_undecl_token_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint32_t _e_id = src->id(ctx);
    const codec_zenoh_decl_ext_keyexpr_source_ref_optional_t _e_ext_keyexpr = src->ext_keyexpr(ctx);
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u32(w, (uint32_t)(_e_id)));
    if (_e_ext_keyexpr.present) {
        SCE_FORGE_TRY_WRITE(codec_zenoh_decl_ext_keyexpr_encode_source(_e_ext_keyexpr.value.table, _e_ext_keyexpr.value.ctx, w));
    }
    return SCE_FORGE_CODEC_OK;
}

/* The flag input `Z` a source of codec_zenoh_undecl_token_t says, which its encode is
 * not handed (docs/adr/0065 §6 R4a). */
static inline uint8_t codec_zenoh_undecl_token_input_z(const codec_zenoh_undecl_token_source_t *src, const void *ctx) {
    return (uint8_t)(src->ext_keyexpr(ctx).present ? 1u : 0u);
}

/* Whether `self` is a value of codec_zenoh_undecl_token_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_undecl_token_check_value(const codec_zenoh_undecl_token_t *self) {
    if (self->ext_keyexpr.present) {
        SCE_FORGE_TRY_WRITE(codec_zenoh_decl_ext_keyexpr_check_value(&self->ext_keyexpr.value));
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_UNDECL_TOKEN_MAX_BYTES bytes and use
 * `codec_zenoh_undecl_token_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_undecl_token_encode(const codec_zenoh_undecl_token_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_undecl_token_check_value(self));
    return codec_zenoh_undecl_token_encode_source(codec_zenoh_undecl_token_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_UNDECL_TOKEN_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_UNDECL_TOKEN_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_undecl_token_encode_to_buf(const codec_zenoh_undecl_token_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_undecl_token_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_UNDECL_TOKEN_H */
