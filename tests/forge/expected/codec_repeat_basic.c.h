// SCE-MAP: codec_repeat_basic:11 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_REPEAT_BASIC_H
#define SCE_FORGE_CODEC_REPEAT_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */
#include "codec_repeat_elem.h"

#define CODEC_REPEAT_BASIC_MIN_BYTES 1
#define CODEC_REPEAT_BASIC_MAX_BYTES 65

typedef struct {
    /* RFC §synth-5-B B2 repeat: fixed array of codec_repeat_elem_t elements (max 32) */
    codec_repeat_elem_t frags[32];
    size_t  frags_len;
} codec_repeat_basic_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_repeat_basic_decode(sce_forge_cursor_t *cursor, codec_repeat_basic_t *out) {
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
    uint8_t _d_num_frags;
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_num_frags = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        if (_d_num_frags > 32) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
        size_t _n = (size_t)_d_num_frags;
        for (size_t _i = 0; _i < _n; ++_i) {
            sce_forge_codec_status_t _st = codec_repeat_elem_decode(cursor, &out->frags[_i]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
        }
        out->frags_len = _n;
    }
    return SCE_FORGE_CODEC_OK;
}

/* The `frags` element of codec_repeat_basic_t: a list of 0 to 32 entries of codec_repeat_elem_t,
 * walked where it is held, which outlives the list. A list is made by
 * codec_repeat_basic_frags_list_make, which walks it once, counts its entries and refuses one
 * outside the bound, so every list holds it (docs/adr/0081 §2) and the
 * encode over a source writes its `count` entries, refusing nothing of
 * the count. */
typedef struct {
    const sce_forge_walk_t *walk;
    const void *ctx;
    const codec_repeat_elem_source_t *entry;
    size_t count;
} codec_repeat_basic_frags_list_t;

/* Make `*out` the list `walk` presents over `ctx`, each entry a source of
 * codec_repeat_elem_t by `entry`: SCE_FORGE_CODEC_OK, or the refusal of a list
 * outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_repeat_basic_frags_list_make(codec_repeat_basic_frags_list_t *out, const sce_forge_walk_t *walk, const void *ctx, const codec_repeat_elem_source_t *entry) {
    size_t count = 0;
    for (const void *at = walk->begin(ctx); at != NULL; at = walk->next(ctx, at)) {
        ++count;
        if (!sce_forge_length_within_bound(count, 32)) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
    }
    out->walk = walk;
    out->ctx = ctx;
    out->entry = entry;
    out->count = count;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_repeat_basic_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_repeat_basic_t is a source by the table
 * codec_repeat_basic_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    codec_repeat_basic_frags_list_t (*frags)(const void *ctx);
} codec_repeat_basic_source_t;

/* A source of codec_repeat_basic_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_repeat_basic_source_t *table;
    const void *ctx;
} codec_repeat_basic_source_ref_t;

/* A source of codec_repeat_basic_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_repeat_basic_source_ref_optional, codec_repeat_basic_source_ref_t)

/* The first entry of the `frags` of the codec_repeat_basic_t at `ctx`, or NULL where
 * it holds none. */
static inline const void *codec_repeat_basic_value_frags_begin(const void *ctx) {
    const codec_repeat_basic_t *self = (const codec_repeat_basic_t *)ctx;
    return self->frags_len > 0u ? (const void *)&self->frags[0] : NULL;
}

/* The entry after `entry` of the `frags` of the codec_repeat_basic_t at `ctx`, or
 * NULL where `entry` is its last. */
static inline const void *codec_repeat_basic_value_frags_next(const void *ctx, const void *entry) {
    const codec_repeat_basic_t *self = (const codec_repeat_basic_t *)ctx;
    const codec_repeat_elem_t *at = (const codec_repeat_elem_t *)entry + 1;
    return at < self->frags + self->frags_len ? (const void *)at : NULL;
}

/* The `frags` of the codec_repeat_basic_t at `ctx`. */
static inline codec_repeat_basic_frags_list_t codec_repeat_basic_value_frags(const void *ctx) {
    static const sce_forge_walk_t walk = {
        codec_repeat_basic_value_frags_begin,
        codec_repeat_basic_value_frags_next,
    };
    const codec_repeat_basic_t *self = (const codec_repeat_basic_t *)ctx;
    codec_repeat_basic_frags_list_t list;
    list.walk = &walk;
    list.ctx = ctx;
    list.entry = codec_repeat_elem_value_source();
    list.count = self->frags_len;
    return list;
}

/* The table that presents a value of codec_repeat_basic_t as a source of it, its
 * context a `const codec_repeat_basic_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_repeat_basic_source_t *codec_repeat_basic_value_source(void) {
    static const codec_repeat_basic_source_t table = {
        codec_repeat_basic_value_frags,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_repeat_basic_encode_source(const codec_repeat_basic_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const codec_repeat_basic_frags_list_t _e_frags = src->frags(ctx);
    const uint8_t _d_num_frags = (uint8_t)(_e_frags.count);
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _d_num_frags));
    {
        const void *_at = _e_frags.walk->begin(_e_frags.ctx);
        for (size_t _i = 0; _i < _e_frags.count && _at != NULL; ++_i) {
            SCE_FORGE_TRY_WRITE(codec_repeat_elem_encode_source(_e_frags.entry, _at, w));
            _at = _e_frags.walk->next(_e_frags.ctx, _at);
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_repeat_basic_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_repeat_basic_check_value(const codec_repeat_basic_t *self) {
    if (!sce_forge_length_within_bound(self->frags_len, 32)) {
        return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
    }
    for (size_t _i = 0; _i < self->frags_len; ++_i) {
        SCE_FORGE_TRY_WRITE(codec_repeat_elem_check_value(&self->frags[_i]));
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_REPEAT_BASIC_MAX_BYTES bytes and use
 * `codec_repeat_basic_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_repeat_basic_encode(const codec_repeat_basic_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_repeat_basic_check_value(self));
    return codec_repeat_basic_encode_source(codec_repeat_basic_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_REPEAT_BASIC_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_REPEAT_BASIC_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_repeat_basic_encode_to_buf(const codec_repeat_basic_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_repeat_basic_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_REPEAT_BASIC_H */
