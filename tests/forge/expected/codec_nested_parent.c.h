// SCE-MAP: codec_nested_parent:22 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_NESTED_PARENT_H
#define SCE_FORGE_CODEC_NESTED_PARENT_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/text.h"

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */
#include "codec_nested_body.h"

#define CODEC_NESTED_PARENT_MIN_BYTES 2
#define CODEC_NESTED_PARENT_MAX_BYTES 6344

typedef struct {
    uint8_t hdr;
    /* RFC §synth-5-B embed: nested codec_nested_body_t struct (no length prefix on the wire) */
    codec_nested_body_t required_body;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    struct { bool present; codec_nested_body_t value; } optional_body;
    /* RFC §synth-5-B B2 repeat: fixed array of codec_nested_body_t elements (max 4) */
    codec_nested_body_t body_list[4];
    size_t  body_list_len;
} codec_nested_parent_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_nested_parent_decode(sce_forge_cursor_t *cursor, codec_nested_parent_t *out) {
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
    uint8_t _d_m;
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_m = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    {
        sce_forge_codec_status_t _st = codec_nested_body_decode(cursor, &out->required_body);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
    }
    memset(&out->optional_body, 0, sizeof out->optional_body);
    if ((out->hdr & 0x01) != 0) {
        sce_forge_codec_status_t _st = codec_nested_body_decode(cursor, &out->optional_body.value);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
        out->optional_body.present = true;
    }
    {
        if (_d_m > 4) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
        size_t _n = (size_t)_d_m;
        for (size_t _i = 0; _i < _n; ++_i) {
            sce_forge_codec_status_t _st = codec_nested_body_decode(cursor, &out->body_list[_i]);
            if (_st != SCE_FORGE_CODEC_OK) return _st;
        }
        out->body_list_len = _n;
    }
    return SCE_FORGE_CODEC_OK;
}

/* The `body_list` element of codec_nested_parent_t: a list of 0 to 4 entries of codec_nested_body_t,
 * walked where it is held, which outlives the list. A list is made by
 * codec_nested_parent_body_list_list_make, which walks it once, counts its entries and refuses one
 * outside the bound, so every list holds it (docs/adr/0081 §2) and the
 * encode over a source writes its `count` entries, refusing nothing of
 * the count. */
typedef struct {
    const sce_forge_walk_t *walk;
    const void *ctx;
    const codec_nested_body_source_t *entry;
    size_t count;
} codec_nested_parent_body_list_list_t;

/* Make `*out` the list `walk` presents over `ctx`, each entry a source of
 * codec_nested_body_t by `entry`: SCE_FORGE_CODEC_OK, or the refusal of a list
 * outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_nested_parent_body_list_list_make(codec_nested_parent_body_list_list_t *out, const sce_forge_walk_t *walk, const void *ctx, const codec_nested_body_source_t *entry) {
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

/* The elements a value of codec_nested_parent_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_nested_parent_t is a source by the table
 * codec_nested_parent_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*hdr)(const void *ctx);
    codec_nested_body_source_ref_t (*required_body)(const void *ctx);
    codec_nested_body_source_ref_optional_t (*optional_body)(const void *ctx);
    codec_nested_parent_body_list_list_t (*body_list)(const void *ctx);
} codec_nested_parent_source_t;

/* A source of codec_nested_parent_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_nested_parent_source_t *table;
    const void *ctx;
} codec_nested_parent_source_ref_t;

/* A source of codec_nested_parent_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_nested_parent_source_ref_optional, codec_nested_parent_source_ref_t)

/* The `hdr` of the codec_nested_parent_t at `ctx`. */
static inline uint8_t codec_nested_parent_value_hdr(const void *ctx) {
    return ((const codec_nested_parent_t *)ctx)->hdr;
}

/* The `required_body` of the codec_nested_parent_t at `ctx`. */
static inline codec_nested_body_source_ref_t codec_nested_parent_value_required_body(const void *ctx) {
    const codec_nested_parent_t *self = (const codec_nested_parent_t *)ctx;
    codec_nested_body_source_ref_t body;
    body.table = codec_nested_body_value_source();
    body.ctx = &self->required_body;
    return body;
}

/* The `optional_body` of the codec_nested_parent_t at `ctx`. */
static inline codec_nested_body_source_ref_optional_t codec_nested_parent_value_optional_body(const void *ctx) {
    const codec_nested_parent_t *self = (const codec_nested_parent_t *)ctx;
    if (!self->optional_body.present) {
        return codec_nested_body_source_ref_optional_none();
    }
    codec_nested_body_source_ref_t body;
    body.table = codec_nested_body_value_source();
    body.ctx = &self->optional_body.value;
    return codec_nested_body_source_ref_optional_some(body);
}

/* The first entry of the `body_list` of the codec_nested_parent_t at `ctx`, or NULL where
 * it holds none. */
static inline const void *codec_nested_parent_value_body_list_begin(const void *ctx) {
    const codec_nested_parent_t *self = (const codec_nested_parent_t *)ctx;
    return self->body_list_len > 0u ? (const void *)&self->body_list[0] : NULL;
}

/* The entry after `entry` of the `body_list` of the codec_nested_parent_t at `ctx`, or
 * NULL where `entry` is its last. */
static inline const void *codec_nested_parent_value_body_list_next(const void *ctx, const void *entry) {
    const codec_nested_parent_t *self = (const codec_nested_parent_t *)ctx;
    const codec_nested_body_t *at = (const codec_nested_body_t *)entry + 1;
    return at < self->body_list + self->body_list_len ? (const void *)at : NULL;
}

/* The `body_list` of the codec_nested_parent_t at `ctx`. */
static inline codec_nested_parent_body_list_list_t codec_nested_parent_value_body_list(const void *ctx) {
    static const sce_forge_walk_t walk = {
        codec_nested_parent_value_body_list_begin,
        codec_nested_parent_value_body_list_next,
    };
    const codec_nested_parent_t *self = (const codec_nested_parent_t *)ctx;
    codec_nested_parent_body_list_list_t list;
    list.walk = &walk;
    list.ctx = ctx;
    list.entry = codec_nested_body_value_source();
    list.count = self->body_list_len;
    return list;
}

/* The table that presents a value of codec_nested_parent_t as a source of it, its
 * context a `const codec_nested_parent_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_nested_parent_source_t *codec_nested_parent_value_source(void) {
    static const codec_nested_parent_source_t table = {
        codec_nested_parent_value_hdr,
        codec_nested_parent_value_required_body,
        codec_nested_parent_value_optional_body,
        codec_nested_parent_value_body_list,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_nested_parent_encode_source(const codec_nested_parent_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_hdr = src->hdr(ctx);
    const codec_nested_body_source_ref_t _e_required_body = src->required_body(ctx);
    const codec_nested_body_source_ref_optional_t _e_optional_body = src->optional_body(ctx);
    const codec_nested_parent_body_list_list_t _e_body_list = src->body_list(ctx);
    const uint8_t _d_m = (uint8_t)(_e_body_list.count);
    uint64_t _f_hdr = (uint64_t)_e_hdr;
    _f_hdr = (_f_hdr & ~0x1ull) | (uint64_t)(_e_optional_body.present);
    const uint8_t _c_hdr = (uint8_t)_f_hdr;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_hdr));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _d_m));
    SCE_FORGE_TRY_WRITE(codec_nested_body_encode_source(_e_required_body.table, _e_required_body.ctx, w));
    if (_e_optional_body.present) {
        SCE_FORGE_TRY_WRITE(codec_nested_body_encode_source(_e_optional_body.value.table, _e_optional_body.value.ctx, w));
    }
    {
        const void *_at = _e_body_list.walk->begin(_e_body_list.ctx);
        for (size_t _i = 0; _i < _e_body_list.count && _at != NULL; ++_i) {
            SCE_FORGE_TRY_WRITE(codec_nested_body_encode_source(_e_body_list.entry, _at, w));
            _at = _e_body_list.walk->next(_e_body_list.ctx, _at);
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_nested_parent_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_nested_parent_check_value(const codec_nested_parent_t *self) {
    SCE_FORGE_TRY_WRITE(codec_nested_body_check_value(&self->required_body));
    if (self->optional_body.present) {
        SCE_FORGE_TRY_WRITE(codec_nested_body_check_value(&self->optional_body.value));
    }
    if (!sce_forge_length_within_bound(self->body_list_len, 4)) {
        return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
    }
    for (size_t _i = 0; _i < self->body_list_len; ++_i) {
        SCE_FORGE_TRY_WRITE(codec_nested_body_check_value(&self->body_list[_i]));
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_NESTED_PARENT_MAX_BYTES bytes and use
 * `codec_nested_parent_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_nested_parent_encode(const codec_nested_parent_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_nested_parent_check_value(self));
    return codec_nested_parent_encode_source(codec_nested_parent_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_NESTED_PARENT_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_NESTED_PARENT_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_nested_parent_encode_to_buf(const codec_nested_parent_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_nested_parent_encode(self, &_w);
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
static inline bool codec_nested_parent_has_opt(const codec_nested_parent_t *self) {
    return (self->hdr & 0x01) != 0;
}

static inline void codec_nested_parent_set_has_opt(codec_nested_parent_t *self, bool v) {
    if (v) {
        self->hdr = (uint8_t)(self->hdr | 0x01);
    } else {
        self->hdr = (uint8_t)(self->hdr & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_NESTED_PARENT_H */
