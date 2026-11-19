// SCE-MAP: codec_present_if_string:47 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_PRESENT_IF_STRING_H
#define SCE_FORGE_CODEC_PRESENT_IF_STRING_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */

#define CODEC_PRESENT_IF_STRING_MIN_BYTES 2
#define CODEC_PRESENT_IF_STRING_MAX_BYTES 34

#ifndef SCE_OPTIONAL_BYTES_32
#define SCE_OPTIONAL_BYTES_32
#ifndef SCE_STATIC_BYTES_32
#define SCE_STATIC_BYTES_32
/* SCE Accepted Subset §2.15: a `bytes` of at most 32 bytes. */
typedef struct {
    uint8_t data[32];
    size_t len;
} sce_static_bytes_32_t;

static inline sce_static_bytes_32_t sce_static_bytes_32_of(sce_forge_bytes_view_t bytes) {
    sce_static_bytes_32_t value = { { 0 }, 0 };
    for (size_t i = 0; i < bytes.len && i < 32u; ++i) {
        value.data[i] = bytes.data[i];
    }
    value.len = bytes.len < 32u ? bytes.len : 32u;
    return value;
}
#endif
/* docs/adr/0055: a value of `sce_static_bytes_32_t` that may be absent. */
SCE_OPTIONAL_DECLARE_BYTES(32)
#endif

typedef struct {
    uint8_t carrier;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_bytes_32_t text;
} codec_present_if_string_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_present_if_string_decode(sce_forge_cursor_t *cursor, codec_present_if_string_t *out) {
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
    uint8_t _d_text_len = 0;
    if ((out->carrier & 0x01) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_text_len = (uint8_t)(raw[0]);
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((out->carrier & 0x01) != 0) {
        if (_d_text_len > 32) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        size_t _n = (size_t)_d_text_len;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        if (!sce_forge_is_valid_utf8(raw, _n)) return SCE_FORGE_CODEC_INVALID_UTF8;
        out->text = sce_optional_bytes_32_none();
        memcpy(out->text.value.data, raw, _n);
        out->text.value.len = _n;
        out->text.present = true;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->text = sce_optional_bytes_32_none();
    }
    return SCE_FORGE_CODEC_OK;
}

/* The `text` element of codec_present_if_string_t: a text of 0 to 32 bytes of UTF-8, viewed where it
 * is held, which outlives the view. A view is made by codec_present_if_string_text_text_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const char *data;
    size_t len;
} codec_present_if_string_text_text_t;

/* Make `*out` a view of the `len` bytes of UTF-8 at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a text outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_present_if_string_text_text_make(codec_present_if_string_text_text_t *out, const char *data, size_t len) {
    if (!sce_forge_length_within_bound(len, 32)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    if (!sce_forge_is_valid_utf8((const uint8_t *)data, len)) {
        return SCE_FORGE_CODEC_INVALID_UTF8;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* A view of codec_present_if_string_text_text_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_present_if_string_text_text_optional, codec_present_if_string_text_text_t)

/* The elements a value of codec_present_if_string_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_present_if_string_t is a source by the table
 * codec_present_if_string_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*carrier)(const void *ctx);
    codec_present_if_string_text_text_optional_t (*text)(const void *ctx);
} codec_present_if_string_source_t;

/* A source of codec_present_if_string_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_present_if_string_source_t *table;
    const void *ctx;
} codec_present_if_string_source_ref_t;

/* A source of codec_present_if_string_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_present_if_string_source_ref_optional, codec_present_if_string_source_ref_t)

/* The `carrier` of the codec_present_if_string_t at `ctx`. */
static inline uint8_t codec_present_if_string_value_carrier(const void *ctx) {
    return ((const codec_present_if_string_t *)ctx)->carrier;
}

/* The `text` of the codec_present_if_string_t at `ctx`. */
static inline codec_present_if_string_text_text_optional_t codec_present_if_string_value_text(const void *ctx) {
    const codec_present_if_string_t *self = (const codec_present_if_string_t *)ctx;
    if (!self->text.present) {
        return codec_present_if_string_text_text_optional_none();
    }
    codec_present_if_string_text_text_t view;
    view.data = (const char *)self->text.value.data;
    view.len = self->text.value.len;
    return codec_present_if_string_text_text_optional_some(view);
}

/* The table that presents a value of codec_present_if_string_t as a source of it, its
 * context a `const codec_present_if_string_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_present_if_string_source_t *codec_present_if_string_value_source(void) {
    static const codec_present_if_string_source_t table = {
        codec_present_if_string_value_carrier,
        codec_present_if_string_value_text,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_present_if_string_encode_source(const codec_present_if_string_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_carrier = src->carrier(ctx);
    const codec_present_if_string_text_text_optional_t _e_text = src->text(ctx);
    const uint8_t _d_text_len = _e_text.present ? (uint8_t)(_e_text.value.len) : (uint8_t)0;
    uint64_t _f_carrier = (uint64_t)_e_carrier;
    _f_carrier = (_f_carrier & ~0x1ull) | (uint64_t)(_e_text.present);
    const uint8_t _c_carrier = (uint8_t)_f_carrier;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_carrier));
    if ((((uint64_t)_c_carrier & 0x1ull) != 0)) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _d_text_len));
    }
    if (_e_text.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, (const uint8_t *)_e_text.value.data, _e_text.value.len));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_present_if_string_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_present_if_string_check_value(const codec_present_if_string_t *self) {
    if (self->text.present && (!sce_forge_length_within_bound(self->text.value.len, 32))) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    if (self->text.present && (!sce_forge_is_valid_utf8(self->text.value.data, self->text.value.len))) {
        return SCE_FORGE_CODEC_INVALID_UTF8;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_PRESENT_IF_STRING_MAX_BYTES bytes and use
 * `codec_present_if_string_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_present_if_string_encode(const codec_present_if_string_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_present_if_string_check_value(self));
    return codec_present_if_string_encode_source(codec_present_if_string_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_PRESENT_IF_STRING_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_PRESENT_IF_STRING_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_present_if_string_encode_to_buf(const codec_present_if_string_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_present_if_string_encode(self, &_w);
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
static inline bool codec_present_if_string_has_text(const codec_present_if_string_t *self) {
    return (self->carrier & 0x01) != 0;
}

static inline void codec_present_if_string_set_has_text(codec_present_if_string_t *self, bool v) {
    if (v) {
        self->carrier = (uint8_t)(self->carrier | 0x01);
    } else {
        self->carrier = (uint8_t)(self->carrier & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_PRESENT_IF_STRING_H */
