// SCE-MAP: codec_present_if_length_ref:16 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_PRESENT_IF_LENGTH_REF_H
#define SCE_FORGE_CODEC_PRESENT_IF_LENGTH_REF_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "sce/text.h"

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */

#define CODEC_PRESENT_IF_LENGTH_REF_MIN_BYTES 2
#define CODEC_PRESENT_IF_LENGTH_REF_MAX_BYTES 34

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

/* The arms of the `flags_has_payload` group of codec_present_if_length_ref_t (docs/adr/0065 §6 R4). */
typedef enum {
    CODEC_PRESENT_IF_LENGTH_REF_FLAGS_HAS_PAYLOAD_HAS_PAYLOAD,
    CODEC_PRESENT_IF_LENGTH_REF_FLAGS_HAS_PAYLOAD_NOT_HAS_PAYLOAD,
} codec_present_if_length_ref_flags_has_payload_kind_t;

/* The `flags_has_payload` group of codec_present_if_length_ref_t where `flags.has_payload` holds: the members present
 * then. */
typedef struct {
    sce_static_bytes_32_t payload;
} codec_present_if_length_ref_flags_has_payload_has_payload_t;

/* The `flags_has_payload` group of codec_present_if_length_ref_t where `flags.has_payload` does not hold: the members present
 * then. */
typedef struct {
    uint8_t payload_size;
} codec_present_if_length_ref_flags_has_payload_not_has_payload_t;

/* The `flags_has_payload` group of codec_present_if_length_ref_t: the arm the value holds, which `kind`
 * names, each the members present while the gates hold its values
 * (docs/adr/0065 §6 R4). */
typedef struct {
    codec_present_if_length_ref_flags_has_payload_kind_t kind;
    union {
        codec_present_if_length_ref_flags_has_payload_has_payload_t has_payload;
        codec_present_if_length_ref_flags_has_payload_not_has_payload_t not_has_payload;
    } arm;
} codec_present_if_length_ref_flags_has_payload_t;


typedef struct {
    uint8_t flags;
    codec_present_if_length_ref_flags_has_payload_t flags_has_payload;
} codec_present_if_length_ref_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_present_if_length_ref_decode(sce_forge_cursor_t *cursor, codec_present_if_length_ref_t *out) {
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
    uint8_t _d_payload_size;
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_payload_size = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    /* The `flags_has_payload` group the gates the decode read select (docs/adr/0065
     * §6 R4), opened before the members it reads in place. */
    memset(&out->flags_has_payload, 0, sizeof out->flags_has_payload);
    sce_static_bytes_32_t *_g_payload = NULL;
    if (((((uint64_t)out->flags >> 0) & 1u) != 0)) {
        out->flags_has_payload.kind = CODEC_PRESENT_IF_LENGTH_REF_FLAGS_HAS_PAYLOAD_HAS_PAYLOAD;
        _g_payload = &out->flags_has_payload.arm.has_payload.payload;
    } else {
        out->flags_has_payload.kind = CODEC_PRESENT_IF_LENGTH_REF_FLAGS_HAS_PAYLOAD_NOT_HAS_PAYLOAD;
    }
    if ((out->flags & 0x01) != 0) {
        if (_d_payload_size > 32) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        size_t _n = (size_t)_d_payload_size;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(_g_payload->data, raw, _n);
        _g_payload->len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    /* The numbers of the `flags_has_payload` group, read into locals, set in the arm
     * the decode opened (docs/adr/0065 §6 R4). */
    if (out->flags_has_payload.kind == CODEC_PRESENT_IF_LENGTH_REF_FLAGS_HAS_PAYLOAD_NOT_HAS_PAYLOAD) {
        out->flags_has_payload.arm.not_has_payload.payload_size = _d_payload_size;
    }
    return SCE_FORGE_CODEC_OK;
}

/* The `payload` element of codec_present_if_length_ref_t: a byte string of 0 to 32 bytes, viewed where it
 * is held, which outlives the view. A view is made by codec_present_if_length_ref_payload_bytes_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const uint8_t *data;
    size_t len;
} codec_present_if_length_ref_payload_bytes_t;

/* Make `*out` a view of the `len` bytes at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a byte string outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_present_if_length_ref_payload_bytes_make(codec_present_if_length_ref_payload_bytes_t *out, const uint8_t *data, size_t len) {
    if (!sce_forge_length_within_bound(len, 32)) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    out->data = data;
    out->len = len;
    return SCE_FORGE_CODEC_OK;
}

/* A view of codec_present_if_length_ref_payload_bytes_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_present_if_length_ref_payload_bytes_optional, codec_present_if_length_ref_payload_bytes_t)

/* The `flags_has_payload` group a source of codec_present_if_length_ref_t presents where `flags.has_payload` holds: the members present
 * then. */
typedef struct {
    codec_present_if_length_ref_payload_bytes_t payload;
} codec_present_if_length_ref_flags_has_payload_has_payload_ref_t;

/* The `flags_has_payload` group a source of codec_present_if_length_ref_t presents where `flags.has_payload` does not hold: the members present
 * then. */
typedef struct {
    uint8_t payload_size;
} codec_present_if_length_ref_flags_has_payload_not_has_payload_ref_t;

/* The `flags_has_payload` group a source of codec_present_if_length_ref_t presents: the arm the value holds, which `kind`
 * names, each the members present while the gates hold its values
 * (docs/adr/0065 §6 R4). */
typedef struct {
    codec_present_if_length_ref_flags_has_payload_kind_t kind;
    union {
        codec_present_if_length_ref_flags_has_payload_has_payload_ref_t has_payload;
        codec_present_if_length_ref_flags_has_payload_not_has_payload_ref_t not_has_payload;
    } arm;
} codec_present_if_length_ref_flags_has_payload_ref_t;

/* The elements a value of codec_present_if_length_ref_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_present_if_length_ref_t is a source by the table
 * codec_present_if_length_ref_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*flags)(const void *ctx);
    codec_present_if_length_ref_flags_has_payload_ref_t (*flags_has_payload)(const void *ctx);
} codec_present_if_length_ref_source_t;

/* A source of codec_present_if_length_ref_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_present_if_length_ref_source_t *table;
    const void *ctx;
} codec_present_if_length_ref_source_ref_t;

/* A source of codec_present_if_length_ref_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_present_if_length_ref_source_ref_optional, codec_present_if_length_ref_source_ref_t)

/* The `flags` of the codec_present_if_length_ref_t at `ctx`. */
static inline uint8_t codec_present_if_length_ref_value_flags(const void *ctx) {
    return ((const codec_present_if_length_ref_t *)ctx)->flags;
}

/* The `flags_has_payload` group of the codec_present_if_length_ref_t at `ctx`, each member presented
 * where the value holds it. */
static inline codec_present_if_length_ref_flags_has_payload_ref_t codec_present_if_length_ref_value_flags_has_payload(const void *ctx) {
    const codec_present_if_length_ref_t *self = (const codec_present_if_length_ref_t *)ctx;
    codec_present_if_length_ref_flags_has_payload_ref_t ref;
    memset(&ref, 0, sizeof ref);
    ref.kind = self->flags_has_payload.kind;
    if (self->flags_has_payload.kind == CODEC_PRESENT_IF_LENGTH_REF_FLAGS_HAS_PAYLOAD_HAS_PAYLOAD) {
        ref.arm.has_payload.payload.data = self->flags_has_payload.arm.has_payload.payload.data;
        ref.arm.has_payload.payload.len = self->flags_has_payload.arm.has_payload.payload.len;
    }
    if (self->flags_has_payload.kind == CODEC_PRESENT_IF_LENGTH_REF_FLAGS_HAS_PAYLOAD_NOT_HAS_PAYLOAD) {
        ref.arm.not_has_payload.payload_size = self->flags_has_payload.arm.not_has_payload.payload_size;
    }
    return ref;
}

/* The table that presents a value of codec_present_if_length_ref_t as a source of it, its
 * context a `const codec_present_if_length_ref_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_present_if_length_ref_source_t *codec_present_if_length_ref_value_source(void) {
    static const codec_present_if_length_ref_source_t table = {
        codec_present_if_length_ref_value_flags,
        codec_present_if_length_ref_value_flags_has_payload,
    };
    return &table;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_present_if_length_ref_encode_source(const codec_present_if_length_ref_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    const uint8_t _e_flags = src->flags(ctx);
    const codec_present_if_length_ref_flags_has_payload_ref_t _e_flags_has_payload = src->flags_has_payload(ctx);
    const codec_present_if_length_ref_payload_bytes_optional_t _e_payload = (_e_flags_has_payload.kind == CODEC_PRESENT_IF_LENGTH_REF_FLAGS_HAS_PAYLOAD_HAS_PAYLOAD ? codec_present_if_length_ref_payload_bytes_optional_some(_e_flags_has_payload.arm.has_payload.payload) : codec_present_if_length_ref_payload_bytes_optional_none());
    const sce_optional_uint8_t _e_payload_size = (_e_flags_has_payload.kind == CODEC_PRESENT_IF_LENGTH_REF_FLAGS_HAS_PAYLOAD_NOT_HAS_PAYLOAD ? sce_optional_uint8_some(_e_flags_has_payload.arm.not_has_payload.payload_size) : sce_optional_uint8_none());
    const uint8_t _d_payload_size = _e_payload.present ? (uint8_t)(_e_payload.value.len) : (_e_payload_size.present ? _e_payload_size.value : (uint8_t)0);
    uint64_t _f_flags = (uint64_t)_e_flags;
    _f_flags = (_f_flags & ~0x1ull) | (uint64_t)(_e_payload.present);
    const uint8_t _c_flags = (uint8_t)_f_flags;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_flags));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _d_payload_size));
    if (_e_payload.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, _e_payload.value.data, _e_payload.value.len));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Whether `self` is a value of codec_present_if_length_ref_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_present_if_length_ref_check_value(const codec_present_if_length_ref_t *self) {
    if (self->flags_has_payload.kind == CODEC_PRESENT_IF_LENGTH_REF_FLAGS_HAS_PAYLOAD_HAS_PAYLOAD) {
        if (!sce_forge_length_within_bound(self->flags_has_payload.arm.has_payload.payload.len, 32)) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_PRESENT_IF_LENGTH_REF_MAX_BYTES bytes and use
 * `codec_present_if_length_ref_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_present_if_length_ref_encode(const codec_present_if_length_ref_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_present_if_length_ref_check_value(self));
    return codec_present_if_length_ref_encode_source(codec_present_if_length_ref_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_PRESENT_IF_LENGTH_REF_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_PRESENT_IF_LENGTH_REF_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_present_if_length_ref_encode_to_buf(const codec_present_if_length_ref_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_present_if_length_ref_encode(self, &_w);
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
static inline bool codec_present_if_length_ref_has_payload(const codec_present_if_length_ref_t *self) {
    return (self->flags & 0x01) != 0;
}

static inline void codec_present_if_length_ref_set_has_payload(codec_present_if_length_ref_t *self, bool v) {
    if (v) {
        self->flags = (uint8_t)(self->flags | 0x01);
    } else {
        self->flags = (uint8_t)(self->flags & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_PRESENT_IF_LENGTH_REF_H */
