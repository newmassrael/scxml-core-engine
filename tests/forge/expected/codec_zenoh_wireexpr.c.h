// SCE-MAP: codec_zenoh_wireexpr:55 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_WIREEXPR_H
#define SCE_FORGE_CODEC_ZENOH_WIREEXPR_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */

#define CODEC_ZENOH_WIREEXPR_MIN_BYTES 0
#define CODEC_ZENOH_WIREEXPR_MAX_BYTES 146

#ifndef SCE_OPTIONAL_BYTES_128
#define SCE_OPTIONAL_BYTES_128
#ifndef SCE_STATIC_BYTES_128
#define SCE_STATIC_BYTES_128
/* SCE Accepted Subset §2.15: a `bytes` of at most 128 bytes. */
typedef struct {
    uint8_t data[128];
    size_t len;
} sce_static_bytes_128_t;

static inline sce_static_bytes_128_t sce_static_bytes_128_of(sce_forge_bytes_view_t bytes) {
    sce_static_bytes_128_t value = { { 0 }, 0 };
    for (size_t i = 0; i < bytes.len && i < 128u; ++i) {
        value.data[i] = bytes.data[i];
    }
    value.len = bytes.len < 128u ? bytes.len : 128u;
    return value;
}
#endif
/* docs/adr/0055: a value of `sce_static_bytes_128_t` that may be absent. */
SCE_OPTIONAL_DECLARE_BYTES(128)
#endif

typedef struct {
    uint64_t id;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_bytes_128_t suffix;
} codec_zenoh_wireexpr_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_wireexpr_decode(sce_forge_cursor_t *cursor, codec_zenoh_wireexpr_t *out, uint8_t n) {
    /* Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
     * `<sce:flag-input>` so codecs that haven't consumed an input via
     * `present-if` yet compile cleanly under -Wunused-parameter. */
    (void)n;
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
    uint64_t id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->id = id;
    uint64_t _d_suffix_len = 0;
    if ((n & 0x01) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        _d_suffix_len = _v;
    }
    if ((n & 0x01) != 0) {
        if (_d_suffix_len > 65535) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        size_t _n = (size_t)_d_suffix_len;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        if (!sce_forge_is_valid_utf8(raw, _n)) return SCE_FORGE_CODEC_INVALID_UTF8;
        out->suffix = sce_optional_bytes_128_none();
        out->suffix.value.len = sce_forge_codec_hold(out->suffix.value.data, 128U, raw, _n, &_over);
        out->suffix.present = true;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->suffix = sce_optional_bytes_128_none();
    }
    if (_over) return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
    return SCE_FORGE_CODEC_OK;
}

/* The `suffix` element of codec_zenoh_wireexpr_t: a text of 0 to 65535 bytes of UTF-8, viewed where it
 * is held, which outlives the view. A view is made by codec_zenoh_wireexpr_suffix_text_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const char *data;
    size_t len;
} codec_zenoh_wireexpr_suffix_text_t;

/* Make `*out` a view of the `len` bytes of UTF-8 at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a text outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_zenoh_wireexpr_suffix_text_make(codec_zenoh_wireexpr_suffix_text_t *out, const char *data, size_t len) {
    if (!sce_forge_length_within_bound(len, 65535)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    if (!sce_forge_is_valid_utf8((const uint8_t *)data, len)) {
        return SCE_FORGE_CODEC_INVALID_UTF8;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* A view of codec_zenoh_wireexpr_suffix_text_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_wireexpr_suffix_text_optional, codec_zenoh_wireexpr_suffix_text_t)

/* The elements a value of codec_zenoh_wireexpr_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_wireexpr_t is a source by the table
 * codec_zenoh_wireexpr_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint64_t (*id)(const void *ctx);
    codec_zenoh_wireexpr_suffix_text_optional_t (*suffix)(const void *ctx);
} codec_zenoh_wireexpr_source_t;

/* A source of codec_zenoh_wireexpr_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_wireexpr_source_t *table;
    const void *ctx;
} codec_zenoh_wireexpr_source_ref_t;

/* A source of codec_zenoh_wireexpr_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_wireexpr_source_ref_optional, codec_zenoh_wireexpr_source_ref_t)

/* The `id` of the codec_zenoh_wireexpr_t at `ctx`. */
static inline uint64_t codec_zenoh_wireexpr_value_id(const void *ctx) {
    return ((const codec_zenoh_wireexpr_t *)ctx)->id;
}

/* The `suffix` of the codec_zenoh_wireexpr_t at `ctx`. */
static inline codec_zenoh_wireexpr_suffix_text_optional_t codec_zenoh_wireexpr_value_suffix(const void *ctx) {
    const codec_zenoh_wireexpr_t *self = (const codec_zenoh_wireexpr_t *)ctx;
    if (!self->suffix.present) {
        return codec_zenoh_wireexpr_suffix_text_optional_none();
    }
    codec_zenoh_wireexpr_suffix_text_t view;
    view.data = (const char *)self->suffix.value.data;
    view.len = self->suffix.value.len;
    return codec_zenoh_wireexpr_suffix_text_optional_some(view);
}

/* The table that presents a value of codec_zenoh_wireexpr_t as a source of it, its
 * context a `const codec_zenoh_wireexpr_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_wireexpr_source_t *codec_zenoh_wireexpr_value_source(void) {
    static const codec_zenoh_wireexpr_source_t table = {
        codec_zenoh_wireexpr_value_id,
        codec_zenoh_wireexpr_value_suffix,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_wireexpr_encode_source(const codec_zenoh_wireexpr_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint64_t _e_id = src->id(ctx);
    const codec_zenoh_wireexpr_suffix_text_optional_t _e_suffix = src->suffix(ctx);
    const uint8_t n = (uint8_t)(_e_suffix.present ? 1u : 0u);
    (void)n;
    const uint64_t _d_suffix_len = _e_suffix.present ? (uint64_t)(_e_suffix.value.len) : (uint64_t)0;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_e_id)));
    if ((((uint64_t)n & 0x1ull) != 0)) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_d_suffix_len)));
    }
    if (_e_suffix.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, (const uint8_t *)_e_suffix.value.data, _e_suffix.value.len));
    }
    return SCE_FORGE_CODEC_OK;
}

/* The flag input `N` a source of codec_zenoh_wireexpr_t says, which its encode is
 * not handed (docs/adr/0065 §6 R4a). */
static inline uint8_t codec_zenoh_wireexpr_input_n(const codec_zenoh_wireexpr_source_t *src, const void *ctx) {
    return (uint8_t)(src->suffix(ctx).present ? 1u : 0u);
}

/* Whether `self` is a value of codec_zenoh_wireexpr_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_wireexpr_check_value(const codec_zenoh_wireexpr_t *self) {
    if (self->suffix.present && (!sce_forge_length_within_bound(self->suffix.value.len, 128))) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    if (self->suffix.present && (!sce_forge_is_valid_utf8(self->suffix.value.data, self->suffix.value.len))) {
        return SCE_FORGE_CODEC_INVALID_UTF8;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_WIREEXPR_MAX_BYTES bytes and use
 * `codec_zenoh_wireexpr_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_wireexpr_encode(const codec_zenoh_wireexpr_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_wireexpr_check_value(self));
    return codec_zenoh_wireexpr_encode_source(codec_zenoh_wireexpr_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_WIREEXPR_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_WIREEXPR_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_wireexpr_encode_to_buf(const codec_zenoh_wireexpr_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_wireexpr_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_WIREEXPR_H */
