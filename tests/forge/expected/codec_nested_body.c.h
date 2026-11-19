// SCE-MAP: codec_nested_body:18 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_NESTED_BODY_H
#define SCE_FORGE_CODEC_NESTED_BODY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */
#include "codec_zenoh_locator.h"

#define CODEC_NESTED_BODY_MIN_BYTES 1
#define CODEC_NESTED_BODY_MAX_BYTES 1057

typedef struct {
    /* RFC §synth-5-B B2 repeat: fixed array of codec_zenoh_locator_t elements (max 4) */
    codec_zenoh_locator_t locs[4];
    size_t  locs_len;
} codec_nested_body_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_nested_body_decode(sce_forge_cursor_t *cursor, codec_nested_body_t *out) {
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
    uint8_t _d_n;
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_n = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        if (_d_n > 4) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
        size_t _n = (size_t)_d_n;
        for (size_t _i = 0; _i < _n; ++_i) {
            sce_forge_codec_status_t _st = codec_zenoh_locator_decode(cursor, &out->locs[_i]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
        }
        out->locs_len = _n;
    }
    return SCE_FORGE_CODEC_OK;
}

/* The `locs` element of codec_nested_body_t: a list of 0 to 4 entries of codec_zenoh_locator_t,
 * walked where it is held, which outlives the list. A list is made by
 * codec_nested_body_locs_list_make, which walks it once, counts its entries and refuses one
 * outside the bound, so every list holds it (docs/adr/0081 §2) and the
 * encode over a source writes its `count` entries, refusing nothing of
 * the count. */
typedef struct {
    const sce_forge_walk_t *walk;
    const void *ctx;
    const codec_zenoh_locator_source_t *entry;
    size_t count;
} codec_nested_body_locs_list_t;

/* Make `*out` the list `walk` presents over `ctx`, each entry a source of
 * codec_zenoh_locator_t by `entry`: SCE_FORGE_CODEC_OK, or the refusal of a list
 * outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_nested_body_locs_list_make(codec_nested_body_locs_list_t *out, const sce_forge_walk_t *walk, const void *ctx, const codec_zenoh_locator_source_t *entry) {
    size_t count = 0;
    for (const void *at = walk->begin(ctx); at != NULL; at = walk->next(ctx, at)) {
        ++count;
        if (!sce_forge_length_within_bound(count, 4)) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
    }
    out->walk = walk;
    out->ctx = ctx;
    out->entry = entry;
    out->count = count;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_nested_body_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_nested_body_t is a source by the table
 * codec_nested_body_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    codec_nested_body_locs_list_t (*locs)(const void *ctx);
} codec_nested_body_source_t;

/* A source of codec_nested_body_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_nested_body_source_t *table;
    const void *ctx;
} codec_nested_body_source_ref_t;

/* A source of codec_nested_body_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_nested_body_source_ref_optional, codec_nested_body_source_ref_t)

/* The first entry of the `locs` of the codec_nested_body_t at `ctx`, or NULL where
 * it holds none. */
static inline const void *codec_nested_body_value_locs_begin(const void *ctx) {
    const codec_nested_body_t *self = (const codec_nested_body_t *)ctx;
    return self->locs_len > 0u ? (const void *)&self->locs[0] : NULL;
}

/* The entry after `entry` of the `locs` of the codec_nested_body_t at `ctx`, or
 * NULL where `entry` is its last. */
static inline const void *codec_nested_body_value_locs_next(const void *ctx, const void *entry) {
    const codec_nested_body_t *self = (const codec_nested_body_t *)ctx;
    const codec_zenoh_locator_t *at = (const codec_zenoh_locator_t *)entry + 1;
    return at < self->locs + self->locs_len ? (const void *)at : NULL;
}

/* The `locs` of the codec_nested_body_t at `ctx`. */
static inline codec_nested_body_locs_list_t codec_nested_body_value_locs(const void *ctx) {
    static const sce_forge_walk_t walk = {
        codec_nested_body_value_locs_begin,
        codec_nested_body_value_locs_next,
    };
    const codec_nested_body_t *self = (const codec_nested_body_t *)ctx;
    codec_nested_body_locs_list_t list;
    list.walk = &walk;
    list.ctx = ctx;
    list.entry = codec_zenoh_locator_value_source();
    list.count = self->locs_len;
    return list;
}

/* The table that presents a value of codec_nested_body_t as a source of it, its
 * context a `const codec_nested_body_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_nested_body_source_t *codec_nested_body_value_source(void) {
    static const codec_nested_body_source_t table = {
        codec_nested_body_value_locs,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_nested_body_encode_source(const codec_nested_body_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const codec_nested_body_locs_list_t _e_locs = src->locs(ctx);
    const uint8_t _d_n = (uint8_t)(_e_locs.count);
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _d_n));
    {
        const void *_at = _e_locs.walk->begin(_e_locs.ctx);
        for (size_t _i = 0; _i < _e_locs.count && _at != NULL; ++_i) {
            SCE_FORGE_TRY_WRITE(codec_zenoh_locator_encode_source(_e_locs.entry, _at, w));
            _at = _e_locs.walk->next(_e_locs.ctx, _at);
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_nested_body_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_nested_body_check_value(const codec_nested_body_t *self) {
    if (!sce_forge_length_within_bound(self->locs_len, 4)) {
        return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
    }
    for (size_t _i = 0; _i < self->locs_len; ++_i) {
        SCE_FORGE_TRY_WRITE(codec_zenoh_locator_check_value(&self->locs[_i]));
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_NESTED_BODY_MAX_BYTES bytes and use
 * `codec_nested_body_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_nested_body_encode(const codec_nested_body_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_nested_body_check_value(self));
    return codec_nested_body_encode_source(codec_nested_body_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_NESTED_BODY_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_NESTED_BODY_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_nested_body_encode_to_buf(const codec_nested_body_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_nested_body_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_NESTED_BODY_H */
