// SCE-MAP: codec_repeat_flag_carrier:15 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_REPEAT_FLAG_CARRIER_H
#define SCE_FORGE_CODEC_REPEAT_FLAG_CARRIER_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/text.h"

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */
#include "codec_chain_flag_entry.h"

#define CODEC_REPEAT_FLAG_CARRIER_MIN_BYTES 2
#define CODEC_REPEAT_FLAG_CARRIER_MAX_BYTES 18

typedef struct {
    uint8_t head;
    /* RFC §synth-5-B B2 repeat: fixed array of codec_chain_flag_entry_t elements (max 4) */
    codec_chain_flag_entry_t entries[4];
    size_t  entries_len;
    /* The `wide` every entry of `entries` says (docs/adr/0065 §6 R8). */
    uint8_t entries_start_wide;
} codec_repeat_flag_carrier_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_repeat_flag_carrier_decode(sce_forge_cursor_t *cursor, codec_repeat_flag_carrier_t *out) {
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
        out->head = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint8_t _d_count;
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_count = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        if (_d_count > 4) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
        size_t _n = (size_t)_d_count;
        for (size_t _i = 0; _i < _n; ++_i) {
            sce_forge_codec_status_t _st = codec_chain_flag_entry_decode(cursor, &out->entries[_i], (uint8_t)((out->head >> 0) & 0x1));
            if (_st != SCE_FORGE_CODEC_OK) return _st;
        }
        out->entries_len = _n;
    }
    out->entries_start_wide = (uint8_t)(((((uint64_t)out->head >> 0) & 1u) != 0) ? 1u : 0u);
    return SCE_FORGE_CODEC_OK;
}

/* The `entries` element of codec_repeat_flag_carrier_t: a list of 0 to 4 entries of codec_chain_flag_entry_t,
 * walked where it is held, which outlives the list. A list is made by
 * codec_repeat_flag_carrier_entries_list_make, which walks it once, counts its entries and refuses one
 * outside the bound, so every list holds it (docs/adr/0081 §2) and the
 * encode over a source writes its `count` entries, refusing nothing of
 * the count. */
typedef struct {
    const sce_forge_walk_t *walk;
    const void *ctx;
    const codec_chain_flag_entry_source_t *entry;
    size_t count;
    uint8_t start_wide;
} codec_repeat_flag_carrier_entries_list_t;

/* Make `*out` the list `walk` presents over `ctx`, each entry a source of
 * codec_chain_flag_entry_t by `entry`: SCE_FORGE_CODEC_OK, or the refusal of a list
 * outside its bound, `*out` then as it was.
 * Each entry says the inputs its law asks of its place (docs/adr/0065
 * §6 R8), or the list is refused with
 * SCE_FORGE_CODEC_PRESENT_IF_MISMATCH. */
static inline sce_forge_codec_status_t codec_repeat_flag_carrier_entries_list_make(codec_repeat_flag_carrier_entries_list_t *out, const sce_forge_walk_t *walk, const void *ctx, const codec_chain_flag_entry_source_t *entry, uint8_t start_wide) {
    size_t count = 0;
    for (const void *at = walk->begin(ctx); at != NULL; at = walk->next(ctx, at)) {
        ++count;
        if (!sce_forge_length_within_bound(count, 4)) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
        if (((codec_chain_flag_entry_input_wide(entry, at)) != 0) != (start_wide != 0)) {
            return SCE_FORGE_CODEC_PRESENT_IF_MISMATCH;
        }
    }
    out->walk = walk;
    out->ctx = ctx;
    out->entry = entry;
    out->count = count;
    out->start_wide = start_wide;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_repeat_flag_carrier_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_repeat_flag_carrier_t is a source by the table
 * codec_repeat_flag_carrier_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*head)(const void *ctx);
    codec_repeat_flag_carrier_entries_list_t (*entries)(const void *ctx);
} codec_repeat_flag_carrier_source_t;

/* A source of codec_repeat_flag_carrier_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_repeat_flag_carrier_source_t *table;
    const void *ctx;
} codec_repeat_flag_carrier_source_ref_t;

/* A source of codec_repeat_flag_carrier_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_repeat_flag_carrier_source_ref_optional, codec_repeat_flag_carrier_source_ref_t)

/* The `head` of the codec_repeat_flag_carrier_t at `ctx`. */
static inline uint8_t codec_repeat_flag_carrier_value_head(const void *ctx) {
    return ((const codec_repeat_flag_carrier_t *)ctx)->head;
}

/* The first entry of the `entries` of the codec_repeat_flag_carrier_t at `ctx`, or NULL where
 * it holds none. */
static inline const void *codec_repeat_flag_carrier_value_entries_begin(const void *ctx) {
    const codec_repeat_flag_carrier_t *self = (const codec_repeat_flag_carrier_t *)ctx;
    return self->entries_len > 0u ? (const void *)&self->entries[0] : NULL;
}

/* The entry after `entry` of the `entries` of the codec_repeat_flag_carrier_t at `ctx`, or
 * NULL where `entry` is its last. */
static inline const void *codec_repeat_flag_carrier_value_entries_next(const void *ctx, const void *entry) {
    const codec_repeat_flag_carrier_t *self = (const codec_repeat_flag_carrier_t *)ctx;
    const codec_chain_flag_entry_t *at = (const codec_chain_flag_entry_t *)entry + 1;
    return at < self->entries + self->entries_len ? (const void *)at : NULL;
}

/* The `entries` of the codec_repeat_flag_carrier_t at `ctx`. */
static inline codec_repeat_flag_carrier_entries_list_t codec_repeat_flag_carrier_value_entries(const void *ctx) {
    static const sce_forge_walk_t walk = {
        codec_repeat_flag_carrier_value_entries_begin,
        codec_repeat_flag_carrier_value_entries_next,
    };
    const codec_repeat_flag_carrier_t *self = (const codec_repeat_flag_carrier_t *)ctx;
    codec_repeat_flag_carrier_entries_list_t list;
    list.walk = &walk;
    list.ctx = ctx;
    list.entry = codec_chain_flag_entry_value_source();
    list.count = self->entries_len;
    list.start_wide = self->entries_start_wide;
    return list;
}

/* The table that presents a value of codec_repeat_flag_carrier_t as a source of it, its
 * context a `const codec_repeat_flag_carrier_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_repeat_flag_carrier_source_t *codec_repeat_flag_carrier_value_source(void) {
    static const codec_repeat_flag_carrier_source_t table = {
        codec_repeat_flag_carrier_value_head,
        codec_repeat_flag_carrier_value_entries,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_repeat_flag_carrier_encode_source(const codec_repeat_flag_carrier_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_head = src->head(ctx);
    const codec_repeat_flag_carrier_entries_list_t _e_entries = src->entries(ctx);
    const uint8_t _d_count = (uint8_t)(_e_entries.count);
    uint64_t _f_head = (uint64_t)_e_head;
    _f_head = (_f_head & ~0x1ull) | (uint64_t)((size_t)_e_entries.start_wide);
    const uint8_t _c_head = (uint8_t)_f_head;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_head));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _d_count));
    {
        const void *_at = _e_entries.walk->begin(_e_entries.ctx);
        for (size_t _i = 0; _i < _e_entries.count && _at != NULL; ++_i) {
            SCE_FORGE_TRY_WRITE(codec_chain_flag_entry_encode_source(_e_entries.entry, _at, w));
            _at = _e_entries.walk->next(_e_entries.ctx, _at);
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_repeat_flag_carrier_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_repeat_flag_carrier_check_value(const codec_repeat_flag_carrier_t *self) {
    if (!sce_forge_length_within_bound(self->entries_len, 4)) {
        return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
    }
    for (size_t _i = 0; _i < self->entries_len; ++_i) {
        SCE_FORGE_TRY_WRITE(codec_chain_flag_entry_check_value(&self->entries[_i]));
    }
    {
        const codec_repeat_flag_carrier_entries_list_t _l = codec_repeat_flag_carrier_value_entries(self);
        codec_repeat_flag_carrier_entries_list_t _made;
        SCE_FORGE_TRY_WRITE(codec_repeat_flag_carrier_entries_list_make(&_made, _l.walk, _l.ctx, _l.entry, _l.start_wide));
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_REPEAT_FLAG_CARRIER_MAX_BYTES bytes and use
 * `codec_repeat_flag_carrier_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_repeat_flag_carrier_encode(const codec_repeat_flag_carrier_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_repeat_flag_carrier_check_value(self));
    return codec_repeat_flag_carrier_encode_source(codec_repeat_flag_carrier_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_REPEAT_FLAG_CARRIER_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_REPEAT_FLAG_CARRIER_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_repeat_flag_carrier_encode_to_buf(const codec_repeat_flag_carrier_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_repeat_flag_carrier_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

/* RFC §synth-5-B flags primitive: per-bit-range accessors over
 * the carrier field. Single-bit (width=1) reads as bool; multi-bit
 * (width>=2) reads as the smallest unsigned C11 integer type that fits
 * (uint8_t / uint16_t / uint32_t / uint64_t). Setters mask + shift on
 * the way in so out-of-range callers can't corrupt sibling bits. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline bool codec_repeat_flag_carrier_w(const codec_repeat_flag_carrier_t *self) {
    return (self->head & 0x01) != 0;
}

static inline void codec_repeat_flag_carrier_set_w(codec_repeat_flag_carrier_t *self, bool v) {
    if (v) {
        self->head = (uint8_t)(self->head | 0x01);
    } else {
        self->head = (uint8_t)(self->head & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_REPEAT_FLAG_CARRIER_H */
