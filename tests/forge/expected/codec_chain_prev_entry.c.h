// SCE-MAP: codec_chain_prev_entry:19 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_CHAIN_PREV_ENTRY_H
#define SCE_FORGE_CODEC_CHAIN_PREV_ENTRY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */

#define CODEC_CHAIN_PREV_ENTRY_MIN_BYTES 5
#define CODEC_CHAIN_PREV_ENTRY_MAX_BYTES 5

/* The arms of the `after_marker` group of codec_chain_prev_entry_t (docs/adr/0065 §6 R4). */
typedef enum {
    CODEC_CHAIN_PREV_ENTRY_AFTER_MARKER_AFTER_MARKER,
    CODEC_CHAIN_PREV_ENTRY_AFTER_MARKER_NOT_AFTER_MARKER,
} codec_chain_prev_entry_after_marker_kind_t;

/* The `after_marker` group of codec_chain_prev_entry_t where `after_marker` holds: the members present
 * then. */
typedef struct {
    uint16_t long_value;
} codec_chain_prev_entry_after_marker_after_marker_t;

/* The `after_marker` group of codec_chain_prev_entry_t where `after_marker` does not hold: the members present
 * then. */
typedef struct {
    uint8_t short_value;
} codec_chain_prev_entry_after_marker_not_after_marker_t;

/* The `after_marker` group of codec_chain_prev_entry_t: the arm the value holds, which `kind`
 * names, each the members present while the gates hold its values
 * (docs/adr/0065 §6 R4). */
typedef struct {
    codec_chain_prev_entry_after_marker_kind_t kind;
    union {
        codec_chain_prev_entry_after_marker_after_marker_t after_marker;
        codec_chain_prev_entry_after_marker_not_after_marker_t not_after_marker;
    } arm;
} codec_chain_prev_entry_after_marker_t;


typedef struct {
    uint8_t header;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_uint8_t tag;
    codec_chain_prev_entry_after_marker_t after_marker;
} codec_chain_prev_entry_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_chain_prev_entry_decode(sce_forge_cursor_t *cursor, codec_chain_prev_entry_t *out, uint8_t after_marker, uint8_t tagged) {
    /* Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
     * `<sce:flag-input>` so codecs that haven't consumed an input via
     * `present-if` yet compile cleanly under -Wunused-parameter. */
    (void)after_marker;
    (void)tagged;
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
        out->header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint8_t _d_short_value = 0;
    if ((after_marker & 0x01) == 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_short_value = (uint8_t)(raw[0]);
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint16_t _d_long_value = 0;
    if ((after_marker & 0x01) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 2);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        _d_long_value = (uint16_t)(((uint16_t)raw[0] << 8) | raw[1]);
        if (!sce_forge_cursor_advance(cursor, 2)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((tagged & 0x01) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->tag = sce_optional_uint8_some((uint8_t)(raw[0]));
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->tag = sce_optional_uint8_none();
    }
    /* The `after_marker` group the gates the decode read select (docs/adr/0065
     * §6 R4). */
    memset(&out->after_marker, 0, sizeof out->after_marker);
    if ((after_marker != 0)) {
        out->after_marker.kind = CODEC_CHAIN_PREV_ENTRY_AFTER_MARKER_AFTER_MARKER;
        out->after_marker.arm.after_marker.long_value = _d_long_value;
    } else {
        out->after_marker.kind = CODEC_CHAIN_PREV_ENTRY_AFTER_MARKER_NOT_AFTER_MARKER;
        out->after_marker.arm.not_after_marker.short_value = _d_short_value;
    }
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_chain_prev_entry_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_chain_prev_entry_t is a source by the table
 * codec_chain_prev_entry_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*header)(const void *ctx);
    sce_optional_uint8_t (*tag)(const void *ctx);
    codec_chain_prev_entry_after_marker_t (*after_marker)(const void *ctx);
} codec_chain_prev_entry_source_t;

/* A source of codec_chain_prev_entry_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_chain_prev_entry_source_t *table;
    const void *ctx;
} codec_chain_prev_entry_source_ref_t;

/* A source of codec_chain_prev_entry_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_chain_prev_entry_source_ref_optional, codec_chain_prev_entry_source_ref_t)

/* The `header` of the codec_chain_prev_entry_t at `ctx`. */
static inline uint8_t codec_chain_prev_entry_value_header(const void *ctx) {
    return ((const codec_chain_prev_entry_t *)ctx)->header;
}

/* The `tag` of the codec_chain_prev_entry_t at `ctx`. */
static inline sce_optional_uint8_t codec_chain_prev_entry_value_tag(const void *ctx) {
    return ((const codec_chain_prev_entry_t *)ctx)->tag;
}

/* The `after_marker` group of the codec_chain_prev_entry_t at `ctx`. */
static inline codec_chain_prev_entry_after_marker_t codec_chain_prev_entry_value_after_marker(const void *ctx) {
    return ((const codec_chain_prev_entry_t *)ctx)->after_marker;
}

/* The table that presents a value of codec_chain_prev_entry_t as a source of it, its
 * context a `const codec_chain_prev_entry_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_chain_prev_entry_source_t *codec_chain_prev_entry_value_source(void) {
    static const codec_chain_prev_entry_source_t table = {
        codec_chain_prev_entry_value_header,
        codec_chain_prev_entry_value_tag,
        codec_chain_prev_entry_value_after_marker,
    };
    return &table;
}

/* One-bit flags of codec_chain_prev_entry_t a parent may set in place of the ones a value
 * presents, such as the flag a chain whose entries are codec_chain_prev_entry_t ends them
 * on, which the chain derives from where each entry stands. A flag absent
 * here is the value's own. */
typedef struct {
    /* The `Z` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_z;
} codec_chain_prev_entry_overrides_t;

/* The overrides that set no flag. */
static inline codec_chain_prev_entry_overrides_t codec_chain_prev_entry_overrides_none(void) {
    codec_chain_prev_entry_overrides_t none;
    none.header_z = sce_optional_bool_none();
    return none;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_chain_prev_entry_encode_source_overriding(const codec_chain_prev_entry_source_t *src, const void *ctx, sce_forge_writer_t *w, codec_chain_prev_entry_overrides_t overrides) {
    const uint8_t _e_header = src->header(ctx);
    const sce_optional_uint8_t _e_tag = src->tag(ctx);
    const codec_chain_prev_entry_after_marker_t _e_after_marker = src->after_marker(ctx);
    const sce_optional_uint16_t _e_long_value = (_e_after_marker.kind == CODEC_CHAIN_PREV_ENTRY_AFTER_MARKER_AFTER_MARKER ? sce_optional_uint16_some(_e_after_marker.arm.after_marker.long_value) : sce_optional_uint16_none());
    const sce_optional_uint8_t _e_short_value = (_e_after_marker.kind == CODEC_CHAIN_PREV_ENTRY_AFTER_MARKER_NOT_AFTER_MARKER ? sce_optional_uint8_some(_e_after_marker.arm.not_after_marker.short_value) : sce_optional_uint8_none());
    uint64_t _f_header = (uint64_t)_e_header;
    if (overrides.header_z.present) {
        _f_header = (_f_header & ~0x80ull) | ((uint64_t)overrides.header_z.value << 7);
    }
    const uint8_t _c_header = (uint8_t)_f_header;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_header));
    if (_e_short_value.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _e_short_value.value));
    }
    if (_e_long_value.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_e_long_value.value >> 8) & 0xFF)));
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)(_e_long_value.value & 0xFF)));
    }
    if (_e_tag.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _e_tag.value));
    }
    return SCE_FORGE_CODEC_OK;
}

/* codec_chain_prev_entry_encode_source_overriding with no flag set in place of the
 * value's: the encode every caller but a parent that sets one calls. */
static inline sce_forge_codec_status_t codec_chain_prev_entry_encode_source(const codec_chain_prev_entry_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    return codec_chain_prev_entry_encode_source_overriding(src, ctx, w, codec_chain_prev_entry_overrides_none());
}

/* The flag input `after_marker` a source of codec_chain_prev_entry_t says, which its encode is
 * not handed (docs/adr/0065 §6 R4a). */
static inline uint8_t codec_chain_prev_entry_input_after_marker(const codec_chain_prev_entry_source_t *src, const void *ctx) {
    return (uint8_t)((src->after_marker(ctx).kind == CODEC_CHAIN_PREV_ENTRY_AFTER_MARKER_AFTER_MARKER) ? 1u : 0u);
}

/* The flag input `tagged` a source of codec_chain_prev_entry_t says, which its encode is
 * not handed (docs/adr/0065 §6 R4a). */
static inline uint8_t codec_chain_prev_entry_input_tagged(const codec_chain_prev_entry_source_t *src, const void *ctx) {
    return (uint8_t)(src->tag(ctx).present ? 1u : 0u);
}

/* Whether `self` is a value of codec_chain_prev_entry_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_chain_prev_entry_check_value(const codec_chain_prev_entry_t *self) {
    (void)self;
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_CHAIN_PREV_ENTRY_MAX_BYTES bytes and use
 * `codec_chain_prev_entry_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_chain_prev_entry_encode(const codec_chain_prev_entry_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_chain_prev_entry_check_value(self));
    return codec_chain_prev_entry_encode_source(codec_chain_prev_entry_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_CHAIN_PREV_ENTRY_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_CHAIN_PREV_ENTRY_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_chain_prev_entry_encode_to_buf(const codec_chain_prev_entry_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_chain_prev_entry_encode(self, &_w);
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
static inline uint8_t codec_chain_prev_entry_ext_id(const codec_chain_prev_entry_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x0F);
}

static inline void codec_chain_prev_entry_set_ext_id(codec_chain_prev_entry_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_chain_prev_entry_z(const codec_chain_prev_entry_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_chain_prev_entry_set_z(codec_chain_prev_entry_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_CHAIN_PREV_ENTRY_H */
