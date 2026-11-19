// SCE-MAP: codec_until_eof_basic:10 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_UNTIL_EOF_BASIC_H
#define SCE_FORGE_CODEC_UNTIL_EOF_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */
#include "codec_repeat_elem.h"

#define CODEC_UNTIL_EOF_BASIC_MIN_BYTES 0
#define CODEC_UNTIL_EOF_BASIC_MAX_BYTES 128

typedef struct {
    /* RFC §synth-5-B B2 repeat: fixed array of codec_repeat_elem_t elements (max 64) */
    codec_repeat_elem_t msgs[64];
    size_t  msgs_len;
} codec_until_eof_basic_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_until_eof_basic_decode(sce_forge_cursor_t *cursor, codec_until_eof_basic_t *out) {
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
        out->msgs_len = 0;
        while (sce_forge_cursor_remaining(cursor) > 0) {
            if (out->msgs_len > 63) {
                return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
            }
            sce_forge_codec_status_t _st = codec_repeat_elem_decode(cursor, &out->msgs[out->msgs_len]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
            out->msgs_len++;
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* The `msgs` element of codec_until_eof_basic_t: a list of 0 to 64 entries of codec_repeat_elem_t,
 * walked where it is held, which outlives the list. A list is made by
 * codec_until_eof_basic_msgs_list_make, which walks it once, counts its entries and refuses one
 * outside the bound, so every list holds it (docs/adr/0081 §2) and the
 * encode over a source writes its `count` entries, refusing nothing of
 * the count. */
typedef struct {
    const sce_forge_walk_t *walk;
    const void *ctx;
    const codec_repeat_elem_source_t *entry;
    size_t count;
} codec_until_eof_basic_msgs_list_t;

/* Make `*out` the list `walk` presents over `ctx`, each entry a source of
 * codec_repeat_elem_t by `entry`: SCE_FORGE_CODEC_OK, or the refusal of a list
 * outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_until_eof_basic_msgs_list_make(codec_until_eof_basic_msgs_list_t *out, const sce_forge_walk_t *walk, const void *ctx, const codec_repeat_elem_source_t *entry) {
    size_t count = 0;
    for (const void *at = walk->begin(ctx); at != NULL; at = walk->next(ctx, at)) {
        ++count;
        if (!sce_forge_length_within_bound(count, 64)) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
    }
    out->walk = walk;
    out->ctx = ctx;
    out->entry = entry;
    out->count = count;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_until_eof_basic_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_until_eof_basic_t is a source by the table
 * codec_until_eof_basic_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    codec_until_eof_basic_msgs_list_t (*msgs)(const void *ctx);
} codec_until_eof_basic_source_t;

/* A source of codec_until_eof_basic_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_until_eof_basic_source_t *table;
    const void *ctx;
} codec_until_eof_basic_source_ref_t;

/* A source of codec_until_eof_basic_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_until_eof_basic_source_ref_optional, codec_until_eof_basic_source_ref_t)

/* The first entry of the `msgs` of the codec_until_eof_basic_t at `ctx`, or NULL where
 * it holds none. */
static inline const void *codec_until_eof_basic_value_msgs_begin(const void *ctx) {
    const codec_until_eof_basic_t *self = (const codec_until_eof_basic_t *)ctx;
    return self->msgs_len > 0u ? (const void *)&self->msgs[0] : NULL;
}

/* The entry after `entry` of the `msgs` of the codec_until_eof_basic_t at `ctx`, or
 * NULL where `entry` is its last. */
static inline const void *codec_until_eof_basic_value_msgs_next(const void *ctx, const void *entry) {
    const codec_until_eof_basic_t *self = (const codec_until_eof_basic_t *)ctx;
    const codec_repeat_elem_t *at = (const codec_repeat_elem_t *)entry + 1;
    return at < self->msgs + self->msgs_len ? (const void *)at : NULL;
}

/* The `msgs` of the codec_until_eof_basic_t at `ctx`. */
static inline codec_until_eof_basic_msgs_list_t codec_until_eof_basic_value_msgs(const void *ctx) {
    static const sce_forge_walk_t walk = {
        codec_until_eof_basic_value_msgs_begin,
        codec_until_eof_basic_value_msgs_next,
    };
    const codec_until_eof_basic_t *self = (const codec_until_eof_basic_t *)ctx;
    codec_until_eof_basic_msgs_list_t list;
    list.walk = &walk;
    list.ctx = ctx;
    list.entry = codec_repeat_elem_value_source();
    list.count = self->msgs_len;
    return list;
}

/* The table that presents a value of codec_until_eof_basic_t as a source of it, its
 * context a `const codec_until_eof_basic_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_until_eof_basic_source_t *codec_until_eof_basic_value_source(void) {
    static const codec_until_eof_basic_source_t table = {
        codec_until_eof_basic_value_msgs,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_until_eof_basic_encode_source(const codec_until_eof_basic_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const codec_until_eof_basic_msgs_list_t _e_msgs = src->msgs(ctx);
    {
        const void *_at = _e_msgs.walk->begin(_e_msgs.ctx);
        for (size_t _i = 0; _i < _e_msgs.count && _at != NULL; ++_i) {
            SCE_FORGE_TRY_WRITE(codec_repeat_elem_encode_source(_e_msgs.entry, _at, w));
            _at = _e_msgs.walk->next(_e_msgs.ctx, _at);
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_until_eof_basic_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_until_eof_basic_check_value(const codec_until_eof_basic_t *self) {
    if (!sce_forge_length_within_bound(self->msgs_len, 64)) {
        return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
    }
    for (size_t _i = 0; _i < self->msgs_len; ++_i) {
        SCE_FORGE_TRY_WRITE(codec_repeat_elem_check_value(&self->msgs[_i]));
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_UNTIL_EOF_BASIC_MAX_BYTES bytes and use
 * `codec_until_eof_basic_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_until_eof_basic_encode(const codec_until_eof_basic_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_until_eof_basic_check_value(self));
    return codec_until_eof_basic_encode_source(codec_until_eof_basic_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_UNTIL_EOF_BASIC_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_UNTIL_EOF_BASIC_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_until_eof_basic_encode_to_buf(const codec_until_eof_basic_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_until_eof_basic_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_UNTIL_EOF_BASIC_H */
