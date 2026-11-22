// SCE-MAP: codec_zenoh_query_value:24 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_QUERY_VALUE_H
#define SCE_FORGE_CODEC_ZENOH_QUERY_VALUE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/text.h"

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */
#include "codec_zenoh_value_entry.h"

#define CODEC_ZENOH_QUERY_VALUE_MIN_BYTES 0
#define CODEC_ZENOH_QUERY_VALUE_MAX_BYTES 18446744073709551615

typedef struct {
    /* RFC §synth-5-B B3 tlv-chain: fixed array of codec_zenoh_value_entry_t entries (max-depth 4) */
    codec_zenoh_value_entry_t exts[4];
    size_t  exts_len;
} codec_zenoh_query_value_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_query_value_decode(sce_forge_cursor_t *cursor, codec_zenoh_query_value_t *out) {
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
        out->exts_len = 0;
        uint8_t _prev_exts_after_shm = 0;
        bool _more = false;
        for (;;) {
            size_t _just = out->exts_len < 4U ? out->exts_len : 4U - 1U;
            sce_forge_codec_status_t _st = sce_forge_codec_note_capacity(codec_zenoh_value_entry_decode(cursor, &out->exts[_just], _prev_exts_after_shm), &_over);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
            if (out->exts_len < 4U) out->exts_len++; else _over = true;
            _more = codec_zenoh_value_entry_z(&out->exts[_just]);
            _prev_exts_after_shm = (uint8_t)(((uint64_t)(codec_zenoh_value_entry_ext_id(&out->exts[_just])) == 4ULL) ? 1 : 0);
            if (!_more) break;
        }
        if (_more && sce_forge_cursor_remaining(cursor) == 0) { return SCE_FORGE_CODEC_NEED_MORE_BYTES; }
    }
    if (_over) return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
    return SCE_FORGE_CODEC_OK;
}

/* The `exts` element of codec_zenoh_query_value_t: a list of 1 or more entries, the protocol stating no most of codec_zenoh_value_entry_t,
 * walked where it is held, which outlives the list. A list is made by
 * codec_zenoh_query_value_exts_list_make, which walks it once, counts its entries and refuses one
 * outside the bound, so every list holds it (docs/adr/0081 §2) and the
 * encode over a source writes its `count` entries, refusing nothing of
 * the count. */
typedef struct {
    const sce_forge_walk_t *walk;
    const void *ctx;
    const codec_zenoh_value_entry_source_t *entry;
    size_t count;
} codec_zenoh_query_value_exts_list_t;

/* Make `*out` the list `walk` presents over `ctx`, each entry a source of
 * codec_zenoh_value_entry_t by `entry`: SCE_FORGE_CODEC_OK, or the refusal of a list
 * outside its bound, `*out` then as it was.
 * Each entry says the inputs its law asks of its place (docs/adr/0065
 * §6 R8), or the list is refused with
 * SCE_FORGE_CODEC_PRESENT_IF_MISMATCH. */
static inline sce_forge_codec_status_t codec_zenoh_query_value_exts_list_make(codec_zenoh_query_value_exts_list_t *out, const sce_forge_walk_t *walk, const void *ctx, const codec_zenoh_value_entry_source_t *entry) {
    size_t count = 0;
    uint8_t _after_after_shm = 0;
    for (const void *at = walk->begin(ctx); at != NULL; at = walk->next(ctx, at)) {
        ++count;
        {
            const sce_optional_uint8_t _s = codec_zenoh_value_entry_input_after_shm(entry, at);
            if (_s.present && ((_s.value != 0) != (_after_after_shm != 0))) {
                return SCE_FORGE_CODEC_PRESENT_IF_MISMATCH;
            }
        }
        _after_after_shm = (uint8_t)(((((uint64_t)entry->header(at)) >> 0) & 0xFull) == 0x4ull);
    }
    if (count < 1u) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    out->walk = walk;
    out->ctx = ctx;
    out->entry = entry;
    out->count = count;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_zenoh_query_value_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_query_value_t is a source by the table
 * codec_zenoh_query_value_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    codec_zenoh_query_value_exts_list_t (*exts)(const void *ctx);
} codec_zenoh_query_value_source_t;

/* A source of codec_zenoh_query_value_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_query_value_source_t *table;
    const void *ctx;
} codec_zenoh_query_value_source_ref_t;

/* A source of codec_zenoh_query_value_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_query_value_source_ref_optional, codec_zenoh_query_value_source_ref_t)

/* The first entry of the `exts` of the codec_zenoh_query_value_t at `ctx`, or NULL where
 * it holds none. */
static inline const void *codec_zenoh_query_value_value_exts_begin(const void *ctx) {
    const codec_zenoh_query_value_t *self = (const codec_zenoh_query_value_t *)ctx;
    return self->exts_len > 0u ? (const void *)&self->exts[0] : NULL;
}

/* The entry after `entry` of the `exts` of the codec_zenoh_query_value_t at `ctx`, or
 * NULL where `entry` is its last. */
static inline const void *codec_zenoh_query_value_value_exts_next(const void *ctx, const void *entry) {
    const codec_zenoh_query_value_t *self = (const codec_zenoh_query_value_t *)ctx;
    const codec_zenoh_value_entry_t *at = (const codec_zenoh_value_entry_t *)entry + 1;
    return at < self->exts + self->exts_len ? (const void *)at : NULL;
}

/* The `exts` of the codec_zenoh_query_value_t at `ctx`. */
static inline codec_zenoh_query_value_exts_list_t codec_zenoh_query_value_value_exts(const void *ctx) {
    static const sce_forge_walk_t walk = {
        codec_zenoh_query_value_value_exts_begin,
        codec_zenoh_query_value_value_exts_next,
    };
    const codec_zenoh_query_value_t *self = (const codec_zenoh_query_value_t *)ctx;
    codec_zenoh_query_value_exts_list_t list;
    list.walk = &walk;
    list.ctx = ctx;
    list.entry = codec_zenoh_value_entry_value_source();
    list.count = self->exts_len;
    return list;
}

/* The table that presents a value of codec_zenoh_query_value_t as a source of it, its
 * context a `const codec_zenoh_query_value_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_query_value_source_t *codec_zenoh_query_value_value_source(void) {
    static const codec_zenoh_query_value_source_t table = {
        codec_zenoh_query_value_value_exts,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_query_value_encode_source(const codec_zenoh_query_value_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const codec_zenoh_query_value_exts_list_t _e_exts = src->exts(ctx);
    {
        const void *_at = _e_exts.walk->begin(_e_exts.ctx);
        for (size_t _i = 0; _i < _e_exts.count && _at != NULL; ++_i) {
            codec_zenoh_value_entry_overrides_t _o = codec_zenoh_value_entry_overrides_none();
            _o.header_z = sce_optional_bool_some(_i + 1u < _e_exts.count);
            SCE_FORGE_TRY_WRITE(codec_zenoh_value_entry_encode_source_overriding(_e_exts.entry, _at, w, _o));
            _at = _e_exts.walk->next(_e_exts.ctx, _at);
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_zenoh_query_value_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_query_value_check_value(const codec_zenoh_query_value_t *self) {
    if (!sce_forge_length_within_bound(self->exts_len, 4)) {
        return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
    }
    if (self->exts_len < 1u) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    for (size_t _i = 0; _i < self->exts_len; ++_i) {
        SCE_FORGE_TRY_WRITE(codec_zenoh_value_entry_check_value(&self->exts[_i]));
    }
    {
        const codec_zenoh_query_value_exts_list_t _l = codec_zenoh_query_value_value_exts(self);
        codec_zenoh_query_value_exts_list_t _made;
        SCE_FORGE_TRY_WRITE(codec_zenoh_query_value_exts_list_make(&_made, _l.walk, _l.ctx, _l.entry));
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_QUERY_VALUE_MAX_BYTES bytes and use
 * `codec_zenoh_query_value_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_query_value_encode(const codec_zenoh_query_value_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_query_value_check_value(self));
    return codec_zenoh_query_value_encode_source(codec_zenoh_query_value_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_QUERY_VALUE_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_QUERY_VALUE_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_query_value_encode_to_buf(const codec_zenoh_query_value_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_query_value_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_QUERY_VALUE_H */
