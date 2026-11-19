// SCE-MAP: codec_tlv_chain_present_if_basic:37 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_TLV_CHAIN_PRESENT_IF_BASIC_H
#define SCE_FORGE_CODEC_TLV_CHAIN_PRESENT_IF_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */
#include "codec_tlv_entry.h"

#define CODEC_TLV_CHAIN_PRESENT_IF_BASIC_MIN_BYTES 1
#define CODEC_TLV_CHAIN_PRESENT_IF_BASIC_MAX_BYTES 137

typedef struct {
    uint8_t carrier;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    struct { bool present; struct { codec_tlv_entry_t data[4]; size_t len; } value; } entries;
} codec_tlv_chain_present_if_basic_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_tlv_chain_present_if_basic_decode(sce_forge_cursor_t *cursor, codec_tlv_chain_present_if_basic_t *out) {
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
        out->carrier = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    memset(&out->entries, 0, sizeof out->entries);
        if ((out->carrier & 0x01) != 0) {
            out->entries.present = true;
            for (size_t _i = 0; _i < 4; ++_i) {
                if (sce_forge_cursor_remaining(cursor) == 0) break;
                sce_forge_codec_status_t _st = codec_tlv_entry_decode(cursor, &out->entries.value.data[out->entries.value.len]);
                if (_st != SCE_FORGE_CODEC_OK) return _st;
                out->entries.value.len++;
            }
            if (sce_forge_cursor_remaining(cursor) > 0) { return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS; }
        }
    return SCE_FORGE_CODEC_OK;
}

/* The `entries` element of codec_tlv_chain_present_if_basic_t: a list of 0 to 4 entries of codec_tlv_entry_t,
 * walked where it is held, which outlives the list. A list is made by
 * codec_tlv_chain_present_if_basic_entries_list_make, which walks it once, counts its entries and refuses one
 * outside the bound, so every list holds it (docs/adr/0081 §2) and the
 * encode over a source writes its `count` entries, refusing nothing of
 * the count. */
typedef struct {
    const sce_forge_walk_t *walk;
    const void *ctx;
    const codec_tlv_entry_source_t *entry;
    size_t count;
} codec_tlv_chain_present_if_basic_entries_list_t;

/* Make `*out` the list `walk` presents over `ctx`, each entry a source of
 * codec_tlv_entry_t by `entry`: SCE_FORGE_CODEC_OK, or the refusal of a list
 * outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_tlv_chain_present_if_basic_entries_list_make(codec_tlv_chain_present_if_basic_entries_list_t *out, const sce_forge_walk_t *walk, const void *ctx, const codec_tlv_entry_source_t *entry) {
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

/* A list of codec_tlv_chain_present_if_basic_entries_list_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_tlv_chain_present_if_basic_entries_list_optional, codec_tlv_chain_present_if_basic_entries_list_t)

/* The elements a value of codec_tlv_chain_present_if_basic_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_tlv_chain_present_if_basic_t is a source by the table
 * codec_tlv_chain_present_if_basic_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*carrier)(const void *ctx);
    codec_tlv_chain_present_if_basic_entries_list_optional_t (*entries)(const void *ctx);
} codec_tlv_chain_present_if_basic_source_t;

/* A source of codec_tlv_chain_present_if_basic_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_tlv_chain_present_if_basic_source_t *table;
    const void *ctx;
} codec_tlv_chain_present_if_basic_source_ref_t;

/* A source of codec_tlv_chain_present_if_basic_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_tlv_chain_present_if_basic_source_ref_optional, codec_tlv_chain_present_if_basic_source_ref_t)

/* The `carrier` of the codec_tlv_chain_present_if_basic_t at `ctx`. */
static inline uint8_t codec_tlv_chain_present_if_basic_value_carrier(const void *ctx) {
    return ((const codec_tlv_chain_present_if_basic_t *)ctx)->carrier;
}

/* The first entry of the `entries` of the codec_tlv_chain_present_if_basic_t at `ctx`, or NULL where
 * it holds none. */
static inline const void *codec_tlv_chain_present_if_basic_value_entries_begin(const void *ctx) {
    const codec_tlv_chain_present_if_basic_t *self = (const codec_tlv_chain_present_if_basic_t *)ctx;
    return self->entries.value.len > 0u ? (const void *)&self->entries.value.data[0] : NULL;
}

/* The entry after `entry` of the `entries` of the codec_tlv_chain_present_if_basic_t at `ctx`, or
 * NULL where `entry` is its last. */
static inline const void *codec_tlv_chain_present_if_basic_value_entries_next(const void *ctx, const void *entry) {
    const codec_tlv_chain_present_if_basic_t *self = (const codec_tlv_chain_present_if_basic_t *)ctx;
    const codec_tlv_entry_t *at = (const codec_tlv_entry_t *)entry + 1;
    return at < self->entries.value.data + self->entries.value.len ? (const void *)at : NULL;
}

/* The `entries` of the codec_tlv_chain_present_if_basic_t at `ctx`. */
static inline codec_tlv_chain_present_if_basic_entries_list_optional_t codec_tlv_chain_present_if_basic_value_entries(const void *ctx) {
    static const sce_forge_walk_t walk = {
        codec_tlv_chain_present_if_basic_value_entries_begin,
        codec_tlv_chain_present_if_basic_value_entries_next,
    };
    const codec_tlv_chain_present_if_basic_t *self = (const codec_tlv_chain_present_if_basic_t *)ctx;
    if (!self->entries.present) {
        return codec_tlv_chain_present_if_basic_entries_list_optional_none();
    }
    codec_tlv_chain_present_if_basic_entries_list_t list;
    list.walk = &walk;
    list.ctx = ctx;
    list.entry = codec_tlv_entry_value_source();
    list.count = self->entries.value.len;
    return codec_tlv_chain_present_if_basic_entries_list_optional_some(list);
}

/* The table that presents a value of codec_tlv_chain_present_if_basic_t as a source of it, its
 * context a `const codec_tlv_chain_present_if_basic_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_tlv_chain_present_if_basic_source_t *codec_tlv_chain_present_if_basic_value_source(void) {
    static const codec_tlv_chain_present_if_basic_source_t table = {
        codec_tlv_chain_present_if_basic_value_carrier,
        codec_tlv_chain_present_if_basic_value_entries,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_tlv_chain_present_if_basic_encode_source(const codec_tlv_chain_present_if_basic_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_carrier = src->carrier(ctx);
    const codec_tlv_chain_present_if_basic_entries_list_optional_t _e_entries = src->entries(ctx);
    uint64_t _f_carrier = (uint64_t)_e_carrier;
    _f_carrier = (_f_carrier & ~0x1ull) | (uint64_t)(_e_entries.present);
    const uint8_t _c_carrier = (uint8_t)_f_carrier;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_carrier));
    if (_e_entries.present) {
        {
            const void *_at = _e_entries.value.walk->begin(_e_entries.value.ctx);
            for (size_t _i = 0; _i < _e_entries.value.count && _at != NULL; ++_i) {
                SCE_FORGE_TRY_WRITE(codec_tlv_entry_encode_source(_e_entries.value.entry, _at, w));
                _at = _e_entries.value.walk->next(_e_entries.value.ctx, _at);
            }
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_tlv_chain_present_if_basic_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_tlv_chain_present_if_basic_check_value(const codec_tlv_chain_present_if_basic_t *self) {
    if (self->entries.present) {
        if (!sce_forge_length_within_bound(self->entries.value.len, 4)) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
        for (size_t _i = 0; _i < self->entries.value.len; ++_i) {
            SCE_FORGE_TRY_WRITE(codec_tlv_entry_check_value(&self->entries.value.data[_i]));
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_TLV_CHAIN_PRESENT_IF_BASIC_MAX_BYTES bytes and use
 * `codec_tlv_chain_present_if_basic_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_tlv_chain_present_if_basic_encode(const codec_tlv_chain_present_if_basic_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_tlv_chain_present_if_basic_check_value(self));
    return codec_tlv_chain_present_if_basic_encode_source(codec_tlv_chain_present_if_basic_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_TLV_CHAIN_PRESENT_IF_BASIC_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_TLV_CHAIN_PRESENT_IF_BASIC_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_tlv_chain_present_if_basic_encode_to_buf(const codec_tlv_chain_present_if_basic_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_tlv_chain_present_if_basic_encode(self, &_w);
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
static inline bool codec_tlv_chain_present_if_basic_has_chain(const codec_tlv_chain_present_if_basic_t *self) {
    return (self->carrier & 0x01) != 0;
}

static inline void codec_tlv_chain_present_if_basic_set_has_chain(codec_tlv_chain_present_if_basic_t *self, bool v) {
    if (v) {
        self->carrier = (uint8_t)(self->carrier | 0x01);
    } else {
        self->carrier = (uint8_t)(self->carrier & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_TLV_CHAIN_PRESENT_IF_BASIC_H */
