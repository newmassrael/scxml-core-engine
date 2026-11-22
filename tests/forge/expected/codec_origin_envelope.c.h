// SCE-MAP: codec_origin_envelope:19 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ORIGIN_ENVELOPE_H
#define SCE_FORGE_CODEC_ORIGIN_ENVELOPE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/text.h"

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */
#include "codec_origin_leaf.h"

#define CODEC_ORIGIN_ENVELOPE_MIN_BYTES 4
#define CODEC_ORIGIN_ENVELOPE_MAX_BYTES 105

typedef struct {
    uint8_t hdr;
    /* RFC §synth-5-B sce:type="string" payload (sce:max-size="16").
     * `char[N] + size_t len` parallels the bytes pair (uint8_t[N] + len)
     * but the host-language type signals UTF-8 text storage; the C
     * string is NOT NUL-terminated — payloads of exactly `max_size`
     * bytes are valid wire input. */
    char    note[16];
    size_t  note_len;
    /* RFC §synth-5-B embed: nested codec_origin_leaf_t struct (no length prefix on the wire) */
    codec_origin_leaf_t required;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    struct { bool present; codec_origin_leaf_t value; } optional;
    /* RFC §synth-5-B B2 repeat: fixed array of codec_origin_leaf_t elements (max 3) */
    codec_origin_leaf_t items[3];
    size_t  items_len;
} codec_origin_envelope_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_origin_envelope_decode(sce_forge_cursor_t *cursor, codec_origin_envelope_t *out) {
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
        out->hdr = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint8_t _d_note_len;
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_note_len = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        if (_d_note_len > 16) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        size_t _n = (size_t)_d_note_len;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        if (!sce_forge_is_valid_utf8(raw, _n)) return SCE_FORGE_CODEC_INVALID_UTF8;
        memcpy(out->note, raw, _n);
        out->note_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint8_t _d_m;
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_m = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        sce_forge_codec_status_t _st = codec_origin_leaf_decode(cursor, &out->required);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
    }
    memset(&out->optional, 0, sizeof out->optional);
    if ((out->hdr & 0x01) != 0) {
        sce_forge_codec_status_t _st = codec_origin_leaf_decode(cursor, &out->optional.value);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
        out->optional.present = true;
    }
    {
        if (_d_m > 3) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
        size_t _n = (size_t)_d_m;
        for (size_t _i = 0; _i < _n; ++_i) {
            sce_forge_codec_status_t _st = codec_origin_leaf_decode(cursor, &out->items[_i]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
        }
        out->items_len = _n;
    }
    return SCE_FORGE_CODEC_OK;
}

/* The `note` element of codec_origin_envelope_t: a text of 0 to 16 bytes of UTF-8, viewed where it
 * is held, which outlives the view. A view is made by codec_origin_envelope_note_text_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const char *data;
    size_t len;
} codec_origin_envelope_note_text_t;

/* Make `*out` a view of the `len` bytes of UTF-8 at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a text outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_origin_envelope_note_text_make(codec_origin_envelope_note_text_t *out, const char *data, size_t len) {
    if (!sce_forge_length_within_bound(len, 16)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    if (!sce_forge_is_valid_utf8((const uint8_t *)data, len)) {
        return SCE_FORGE_CODEC_INVALID_UTF8;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* The `items` element of codec_origin_envelope_t: a list of 0 to 3 entries of codec_origin_leaf_t,
 * walked where it is held, which outlives the list. A list is made by
 * codec_origin_envelope_items_list_make, which walks it once, counts its entries and refuses one
 * outside the bound, so every list holds it (docs/adr/0081 §2) and the
 * encode over a source writes its `count` entries, refusing nothing of
 * the count. */
typedef struct {
    const sce_forge_walk_t *walk;
    const void *ctx;
    const codec_origin_leaf_source_t *entry;
    size_t count;
} codec_origin_envelope_items_list_t;

/* Make `*out` the list `walk` presents over `ctx`, each entry a source of
 * codec_origin_leaf_t by `entry`: SCE_FORGE_CODEC_OK, or the refusal of a list
 * outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_origin_envelope_items_list_make(codec_origin_envelope_items_list_t *out, const sce_forge_walk_t *walk, const void *ctx, const codec_origin_leaf_source_t *entry) {
    size_t count = 0;
    for (const void *at = walk->begin(ctx); at != NULL; at = walk->next(ctx, at)) {
        ++count;
        if (!sce_forge_length_within_bound(count, 3)) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
    }
    out->walk = walk;
    out->ctx = ctx;
    out->entry = entry;
    out->count = count;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_origin_envelope_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_origin_envelope_t is a source by the table
 * codec_origin_envelope_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*hdr)(const void *ctx);
    codec_origin_envelope_note_text_t (*note)(const void *ctx);
    codec_origin_leaf_source_ref_t (*required)(const void *ctx);
    codec_origin_leaf_source_ref_optional_t (*optional)(const void *ctx);
    codec_origin_envelope_items_list_t (*items)(const void *ctx);
} codec_origin_envelope_source_t;

/* A source of codec_origin_envelope_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_origin_envelope_source_t *table;
    const void *ctx;
} codec_origin_envelope_source_ref_t;

/* A source of codec_origin_envelope_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_origin_envelope_source_ref_optional, codec_origin_envelope_source_ref_t)

/* The `hdr` of the codec_origin_envelope_t at `ctx`. */
static inline uint8_t codec_origin_envelope_value_hdr(const void *ctx) {
    return ((const codec_origin_envelope_t *)ctx)->hdr;
}

/* The `note` of the codec_origin_envelope_t at `ctx`. */
static inline codec_origin_envelope_note_text_t codec_origin_envelope_value_note(const void *ctx) {
    const codec_origin_envelope_t *self = (const codec_origin_envelope_t *)ctx;
    codec_origin_envelope_note_text_t view;
    view.data = self->note;
    view.len = self->note_len;
    return view;
}

/* The `required` of the codec_origin_envelope_t at `ctx`. */
static inline codec_origin_leaf_source_ref_t codec_origin_envelope_value_required(const void *ctx) {
    const codec_origin_envelope_t *self = (const codec_origin_envelope_t *)ctx;
    codec_origin_leaf_source_ref_t body;
    body.table = codec_origin_leaf_value_source();
    body.ctx = &self->required;
    return body;
}

/* The `optional` of the codec_origin_envelope_t at `ctx`. */
static inline codec_origin_leaf_source_ref_optional_t codec_origin_envelope_value_optional(const void *ctx) {
    const codec_origin_envelope_t *self = (const codec_origin_envelope_t *)ctx;
    if (!self->optional.present) {
        return codec_origin_leaf_source_ref_optional_none();
    }
    codec_origin_leaf_source_ref_t body;
    body.table = codec_origin_leaf_value_source();
    body.ctx = &self->optional.value;
    return codec_origin_leaf_source_ref_optional_some(body);
}

/* The first entry of the `items` of the codec_origin_envelope_t at `ctx`, or NULL where
 * it holds none. */
static inline const void *codec_origin_envelope_value_items_begin(const void *ctx) {
    const codec_origin_envelope_t *self = (const codec_origin_envelope_t *)ctx;
    return self->items_len > 0u ? (const void *)&self->items[0] : NULL;
}

/* The entry after `entry` of the `items` of the codec_origin_envelope_t at `ctx`, or
 * NULL where `entry` is its last. */
static inline const void *codec_origin_envelope_value_items_next(const void *ctx, const void *entry) {
    const codec_origin_envelope_t *self = (const codec_origin_envelope_t *)ctx;
    const codec_origin_leaf_t *at = (const codec_origin_leaf_t *)entry + 1;
    return at < self->items + self->items_len ? (const void *)at : NULL;
}

/* The `items` of the codec_origin_envelope_t at `ctx`. */
static inline codec_origin_envelope_items_list_t codec_origin_envelope_value_items(const void *ctx) {
    static const sce_forge_walk_t walk = {
        codec_origin_envelope_value_items_begin,
        codec_origin_envelope_value_items_next,
    };
    const codec_origin_envelope_t *self = (const codec_origin_envelope_t *)ctx;
    codec_origin_envelope_items_list_t list;
    list.walk = &walk;
    list.ctx = ctx;
    list.entry = codec_origin_leaf_value_source();
    list.count = self->items_len;
    return list;
}

/* The table that presents a value of codec_origin_envelope_t as a source of it, its
 * context a `const codec_origin_envelope_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_origin_envelope_source_t *codec_origin_envelope_value_source(void) {
    static const codec_origin_envelope_source_t table = {
        codec_origin_envelope_value_hdr,
        codec_origin_envelope_value_note,
        codec_origin_envelope_value_required,
        codec_origin_envelope_value_optional,
        codec_origin_envelope_value_items,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_origin_envelope_encode_source(const codec_origin_envelope_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_hdr = src->hdr(ctx);
    const codec_origin_envelope_note_text_t _e_note = src->note(ctx);
    const codec_origin_leaf_source_ref_t _e_required = src->required(ctx);
    const codec_origin_leaf_source_ref_optional_t _e_optional = src->optional(ctx);
    const codec_origin_envelope_items_list_t _e_items = src->items(ctx);
    const uint8_t _d_note_len = (uint8_t)(_e_note.len);
    const uint8_t _d_m = (uint8_t)(_e_items.count);
    uint64_t _f_hdr = (uint64_t)_e_hdr;
    _f_hdr = (_f_hdr & ~0x1ull) | (uint64_t)(_e_optional.present);
    const uint8_t _c_hdr = (uint8_t)_f_hdr;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_hdr));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _d_note_len));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, (const uint8_t *)_e_note.data, _e_note.len));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _d_m));
    SCE_FORGE_TRY_WRITE(codec_origin_leaf_encode_source(_e_required.table, _e_required.ctx, w));
    if (_e_optional.present) {
        SCE_FORGE_TRY_WRITE(codec_origin_leaf_encode_source(_e_optional.value.table, _e_optional.value.ctx, w));
    }
    {
        const void *_at = _e_items.walk->begin(_e_items.ctx);
        for (size_t _i = 0; _i < _e_items.count && _at != NULL; ++_i) {
            SCE_FORGE_TRY_WRITE(codec_origin_leaf_encode_source(_e_items.entry, _at, w));
            _at = _e_items.walk->next(_e_items.ctx, _at);
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_origin_envelope_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_origin_envelope_check_value(const codec_origin_envelope_t *self) {
    if (!sce_forge_length_within_bound(self->note_len, 16)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    if (!sce_forge_is_valid_utf8((const uint8_t *)self->note, self->note_len)) {
        return SCE_FORGE_CODEC_INVALID_UTF8;
    }
    SCE_FORGE_TRY_WRITE(codec_origin_leaf_check_value(&self->required));
    if (self->optional.present) {
        SCE_FORGE_TRY_WRITE(codec_origin_leaf_check_value(&self->optional.value));
    }
    if (!sce_forge_length_within_bound(self->items_len, 3)) {
        return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
    }
    for (size_t _i = 0; _i < self->items_len; ++_i) {
        SCE_FORGE_TRY_WRITE(codec_origin_leaf_check_value(&self->items[_i]));
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ORIGIN_ENVELOPE_MAX_BYTES bytes and use
 * `codec_origin_envelope_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_origin_envelope_encode(const codec_origin_envelope_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_origin_envelope_check_value(self));
    return codec_origin_envelope_encode_source(codec_origin_envelope_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ORIGIN_ENVELOPE_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ORIGIN_ENVELOPE_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_origin_envelope_encode_to_buf(const codec_origin_envelope_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_origin_envelope_encode(self, &_w);
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
static inline bool codec_origin_envelope_has_opt(const codec_origin_envelope_t *self) {
    return (self->hdr & 0x01) != 0;
}

static inline void codec_origin_envelope_set_has_opt(codec_origin_envelope_t *self, bool v) {
    if (v) {
        self->hdr = (uint8_t)(self->hdr | 0x01);
    } else {
        self->hdr = (uint8_t)(self->hdr & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_ORIGIN_ENVELOPE_H */
