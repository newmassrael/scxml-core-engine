// SCE-MAP: codec_present_if_tail:8 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_PRESENT_IF_TAIL_H
#define SCE_FORGE_CODEC_PRESENT_IF_TAIL_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */

#define CODEC_PRESENT_IF_TAIL_MIN_BYTES 1
#define CODEC_PRESENT_IF_TAIL_MAX_BYTES 65

#ifndef SCE_OPTIONAL_BYTES_64
#define SCE_OPTIONAL_BYTES_64
#ifndef SCE_STATIC_BYTES_64
#define SCE_STATIC_BYTES_64
/* SCE Accepted Subset §2.15: a `bytes` of at most 64 bytes. */
typedef struct {
    uint8_t data[64];
    size_t len;
} sce_static_bytes_64_t;

static inline sce_static_bytes_64_t sce_static_bytes_64_of(sce_forge_bytes_view_t bytes) {
    sce_static_bytes_64_t value = { { 0 }, 0 };
    for (size_t i = 0; i < bytes.len && i < 64u; ++i) {
        value.data[i] = bytes.data[i];
    }
    value.len = bytes.len < 64u ? bytes.len : 64u;
    return value;
}
#endif
/* docs/adr/0055: a value of `sce_static_bytes_64_t` that may be absent. */
SCE_OPTIONAL_DECLARE_BYTES(64)
#endif

typedef struct {
    uint8_t flags;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_bytes_64_t payload;
} codec_present_if_tail_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_present_if_tail_decode(sce_forge_cursor_t *cursor, codec_present_if_tail_t *out) {
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
        out->flags = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((out->flags & 0x01) != 0) {
        size_t _n = sce_forge_cursor_remaining(cursor);
        if (_n > 64) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->payload = sce_optional_bytes_64_none();
        memcpy(out->payload.value.data, raw, _n);
        out->payload.value.len = _n;
        out->payload.present = true;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->payload = sce_optional_bytes_64_none();
    }
    return SCE_FORGE_CODEC_OK;
}

/* The `payload` element of codec_present_if_tail_t: a byte string of 0 to 64 bytes, viewed where it
 * is held, which outlives the view. A view is made by codec_present_if_tail_payload_bytes_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const uint8_t *data;
    size_t len;
} codec_present_if_tail_payload_bytes_t;

/* Make `*out` a view of the `len` bytes at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a byte string outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_present_if_tail_payload_bytes_make(codec_present_if_tail_payload_bytes_t *out, const uint8_t *data, size_t len) {
    if (!sce_forge_length_within_bound(len, 64)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* A view of codec_present_if_tail_payload_bytes_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_present_if_tail_payload_bytes_optional, codec_present_if_tail_payload_bytes_t)

/* The elements a value of codec_present_if_tail_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_present_if_tail_t is a source by the table
 * codec_present_if_tail_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*flags)(const void *ctx);
    codec_present_if_tail_payload_bytes_optional_t (*payload)(const void *ctx);
} codec_present_if_tail_source_t;

/* A source of codec_present_if_tail_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_present_if_tail_source_t *table;
    const void *ctx;
} codec_present_if_tail_source_ref_t;

/* A source of codec_present_if_tail_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_present_if_tail_source_ref_optional, codec_present_if_tail_source_ref_t)

/* The `flags` of the codec_present_if_tail_t at `ctx`. */
static inline uint8_t codec_present_if_tail_value_flags(const void *ctx) {
    return ((const codec_present_if_tail_t *)ctx)->flags;
}

/* The `payload` of the codec_present_if_tail_t at `ctx`. */
static inline codec_present_if_tail_payload_bytes_optional_t codec_present_if_tail_value_payload(const void *ctx) {
    const codec_present_if_tail_t *self = (const codec_present_if_tail_t *)ctx;
    if (!self->payload.present) {
        return codec_present_if_tail_payload_bytes_optional_none();
    }
    codec_present_if_tail_payload_bytes_t view;
    view.data = self->payload.value.data;
    view.len = self->payload.value.len;
    return codec_present_if_tail_payload_bytes_optional_some(view);
}

/* The table that presents a value of codec_present_if_tail_t as a source of it, its
 * context a `const codec_present_if_tail_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_present_if_tail_source_t *codec_present_if_tail_value_source(void) {
    static const codec_present_if_tail_source_t table = {
        codec_present_if_tail_value_flags,
        codec_present_if_tail_value_payload,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_present_if_tail_encode_source(const codec_present_if_tail_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_flags = src->flags(ctx);
    const codec_present_if_tail_payload_bytes_optional_t _e_payload = src->payload(ctx);
    uint64_t _f_flags = (uint64_t)_e_flags;
    _f_flags = (_f_flags & ~0x1ull) | (uint64_t)(_e_payload.present);
    const uint8_t _c_flags = (uint8_t)_f_flags;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_flags));
    if (_e_payload.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, _e_payload.value.data, _e_payload.value.len));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_present_if_tail_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_present_if_tail_check_value(const codec_present_if_tail_t *self) {
    if (self->payload.present && (!sce_forge_length_within_bound(self->payload.value.len, 64))) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_PRESENT_IF_TAIL_MAX_BYTES bytes and use
 * `codec_present_if_tail_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_present_if_tail_encode(const codec_present_if_tail_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_present_if_tail_check_value(self));
    return codec_present_if_tail_encode_source(codec_present_if_tail_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_PRESENT_IF_TAIL_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_PRESENT_IF_TAIL_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_present_if_tail_encode_to_buf(const codec_present_if_tail_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_present_if_tail_encode(self, &_w);
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
static inline bool codec_present_if_tail_has_payload(const codec_present_if_tail_t *self) {
    return (self->flags & 0x01) != 0;
}

static inline void codec_present_if_tail_set_has_payload(codec_present_if_tail_t *self, bool v) {
    if (v) {
        self->flags = (uint8_t)(self->flags | 0x01);
    } else {
        self->flags = (uint8_t)(self->flags & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_PRESENT_IF_TAIL_H */
