// SCE-MAP: codec_zenoh_ext_entry:52 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_EXT_ENTRY_H
#define SCE_FORGE_CODEC_ZENOH_EXT_ENTRY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */
#include "codec_zenoh_ext_unit.h"
#include "codec_zenoh_ext_zint.h"
#include "codec_zenoh_ext_zbuf.h"

#define CODEC_ZENOH_EXT_ENTRY_MIN_BYTES 1
#define CODEC_ZENOH_EXT_ENTRY_MAX_BYTES 266

/* RFC §synth-5-B variant primitive: tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_UNIT,
    CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZINT,
    CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZBUF,
    CODEC_ZENOH_EXT_ENTRY_BODY_KIND_DEFAULT,
} codec_zenoh_ext_entry_body_kind_t;

typedef struct {
    codec_zenoh_ext_entry_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_zenoh_ext_unit_t codec_zenoh_ext_unit;
        codec_zenoh_ext_zint_t codec_zenoh_ext_zint;
        codec_zenoh_ext_zbuf_t codec_zenoh_ext_zbuf;
        codec_zenoh_ext_unit_t default_body;
    } arm;
} codec_zenoh_ext_entry_variant_t;

typedef struct {
    uint8_t header;
    codec_zenoh_ext_entry_variant_t body;
} codec_zenoh_ext_entry_t;

/* RFC variant-default-uniformity (C11): designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_zenoh_ext_entry_t x = CODEC_ZENOH_EXT_ENTRY_DEFAULT_INIT;
 * codec_zenoh_ext_entry_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_ZENOH_EXT_ENTRY_DEFAULT_INIT { \
    .body = { \
        .kind = CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_UNIT, \
        .arm = { .codec_zenoh_ext_unit = CODEC_ZENOH_EXT_UNIT_DEFAULT_INIT } \
    }, \
}

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_ext_entry_decode(sce_forge_cursor_t *cursor, codec_zenoh_ext_entry_t *out) {
    /* A field or a body past what this node holds: the frame is read on,
     * judged as one that holds it would be, and the capacity is answered
     * once every other refusal has had its turn (docs/adr/0077 §7). */
    bool _over = false;
    (void)_over;
    /* Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix). */
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_ZENOH_EXT_ENTRY_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->header = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_ZENOH_EXT_ENTRY_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch ((uint8_t)((out->header >> 5) & (uint8_t)0x03)) {
        case 0:
            out->body.kind = CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_UNIT;
            _arm_st = codec_zenoh_ext_unit_decode(cursor, &out->body.arm.codec_zenoh_ext_unit);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 1:
            out->body.kind = CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZINT;
            _arm_st = codec_zenoh_ext_zint_decode(cursor, &out->body.arm.codec_zenoh_ext_zint);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 2:
            out->body.kind = CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZBUF;
            _arm_st = codec_zenoh_ext_zbuf_decode(cursor, &out->body.arm.codec_zenoh_ext_zbuf);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_ZENOH_EXT_ENTRY_BODY_KIND_DEFAULT;
            out->body.default_tag = (uint8_t)((out->header >> 5) & (uint8_t)0x03);
            _arm_st = codec_zenoh_ext_unit_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    if (_over) return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
    return SCE_FORGE_CODEC_OK;
}

/* The arm of the variant a source of codec_zenoh_ext_entry_t presents (docs/adr/0081
 * §2): which arm it is, the tag the catch-all holds, and the arm's body,
 * a source of the arm's codec in the member `kind` names. */
typedef struct {
    codec_zenoh_ext_entry_body_kind_t kind;
    uint8_t default_tag;  /* the catch-all's */
    union {
        codec_zenoh_ext_unit_source_ref_t codec_zenoh_ext_unit;
        codec_zenoh_ext_zint_source_ref_t codec_zenoh_ext_zint;
        codec_zenoh_ext_zbuf_source_ref_t codec_zenoh_ext_zbuf;
        codec_zenoh_ext_unit_source_ref_t default_body;
    } arm;
} codec_zenoh_ext_entry_body_ref_t;

/* The elements a value of codec_zenoh_ext_entry_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_ext_entry_t is a source by the table
 * codec_zenoh_ext_entry_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*header)(const void *ctx);
    codec_zenoh_ext_entry_body_ref_t (*body)(const void *ctx);
} codec_zenoh_ext_entry_source_t;

/* A source of codec_zenoh_ext_entry_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_ext_entry_source_t *table;
    const void *ctx;
} codec_zenoh_ext_entry_source_ref_t;

/* A source of codec_zenoh_ext_entry_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_ext_entry_source_ref_optional, codec_zenoh_ext_entry_source_ref_t)

/* The `header` of the codec_zenoh_ext_entry_t at `ctx`. */
static inline uint8_t codec_zenoh_ext_entry_value_header(const void *ctx) {
    return ((const codec_zenoh_ext_entry_t *)ctx)->header;
}

/* The arm of the variant of the codec_zenoh_ext_entry_t at `ctx`: its kind, the tag the
 * catch-all holds, and its body as a source of the arm's codec. */
static inline codec_zenoh_ext_entry_body_ref_t codec_zenoh_ext_entry_value_body(const void *ctx) {
    const codec_zenoh_ext_entry_t *self = (const codec_zenoh_ext_entry_t *)ctx;
    codec_zenoh_ext_entry_body_ref_t ref;
    memset(&ref, 0, sizeof ref);
    ref.kind = self->body.kind;
    ref.default_tag = self->body.default_tag;
    switch (self->body.kind) {
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_UNIT:
            ref.arm.codec_zenoh_ext_unit.table = codec_zenoh_ext_unit_value_source();
            ref.arm.codec_zenoh_ext_unit.ctx = &self->body.arm.codec_zenoh_ext_unit;
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZINT:
            ref.arm.codec_zenoh_ext_zint.table = codec_zenoh_ext_zint_value_source();
            ref.arm.codec_zenoh_ext_zint.ctx = &self->body.arm.codec_zenoh_ext_zint;
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZBUF:
            ref.arm.codec_zenoh_ext_zbuf.table = codec_zenoh_ext_zbuf_value_source();
            ref.arm.codec_zenoh_ext_zbuf.ctx = &self->body.arm.codec_zenoh_ext_zbuf;
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_DEFAULT:
            ref.arm.default_body.table = codec_zenoh_ext_unit_value_source();
            ref.arm.default_body.ctx = &self->body.arm.default_body;
            break;
        default:
            break;
    }
    return ref;
}

/* The table that presents a value of codec_zenoh_ext_entry_t as a source of it, its
 * context a `const codec_zenoh_ext_entry_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_ext_entry_source_t *codec_zenoh_ext_entry_value_source(void) {
    static const codec_zenoh_ext_entry_source_t table = {
        codec_zenoh_ext_entry_value_header,
        codec_zenoh_ext_entry_value_body,
    };
    return &table;
}

/* One-bit flags of codec_zenoh_ext_entry_t a parent may set in place of the ones a value
 * presents, such as the flag a chain whose entries are codec_zenoh_ext_entry_t ends them
 * on, which the chain derives from where each entry stands. A flag absent
 * here is the value's own. */
typedef struct {
    /* The `M` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_m;
    /* The `Z` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_z;
} codec_zenoh_ext_entry_overrides_t;

/* The overrides that set no flag. */
static inline codec_zenoh_ext_entry_overrides_t codec_zenoh_ext_entry_overrides_none(void) {
    codec_zenoh_ext_entry_overrides_t none;
    none.header_m = sce_optional_bool_none();
    none.header_z = sce_optional_bool_none();
    return none;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_ext_entry_encode_source_overriding(const codec_zenoh_ext_entry_source_t *src, const void *ctx, sce_forge_writer_t *w, codec_zenoh_ext_entry_overrides_t overrides) {
    const uint8_t _e_header = src->header(ctx);
    const codec_zenoh_ext_entry_body_ref_t _e_body = src->body(ctx);
    uint64_t _t_body = 0;
    switch (_e_body.kind) {
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_UNIT:
            _t_body = 0x0ull;
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZINT:
            _t_body = 0x1ull;
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZBUF:
            _t_body = 0x2ull;
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_DEFAULT:
            _t_body = (uint64_t)_e_body.default_tag;
            break;
        default:
            break;
    }
    uint64_t _f_header = (uint64_t)_e_header;
    _f_header = (_f_header & ~0x60ull) | ((_t_body & 0x3ull) << 5);
    if (overrides.header_m.present) {
        _f_header = (_f_header & ~0x10ull) | ((uint64_t)overrides.header_m.value << 4);
    }
    if (overrides.header_z.present) {
        _f_header = (_f_header & ~0x80ull) | ((uint64_t)overrides.header_z.value << 7);
    }
    const uint8_t _c_header = (uint8_t)_f_header;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_header));
    switch (_e_body.kind) {
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_UNIT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_unit_encode_source(_e_body.arm.codec_zenoh_ext_unit.table, _e_body.arm.codec_zenoh_ext_unit.ctx, w));
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZINT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_zint_encode_source(_e_body.arm.codec_zenoh_ext_zint.table, _e_body.arm.codec_zenoh_ext_zint.ctx, w));
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZBUF:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_zbuf_encode_source(_e_body.arm.codec_zenoh_ext_zbuf.table, _e_body.arm.codec_zenoh_ext_zbuf.ctx, w));
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_unit_encode_source(_e_body.arm.default_body.table, _e_body.arm.default_body.ctx, w));
            break;
        default:
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* codec_zenoh_ext_entry_encode_source_overriding with no flag set in place of the
 * value's: the encode every caller but a parent that sets one calls. */
static inline sce_forge_codec_status_t codec_zenoh_ext_entry_encode_source(const codec_zenoh_ext_entry_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    return codec_zenoh_ext_entry_encode_source_overriding(src, ctx, w, codec_zenoh_ext_entry_overrides_none());
}

/* Whether `self` is a value of codec_zenoh_ext_entry_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_ext_entry_check_value(const codec_zenoh_ext_entry_t *self) {
    switch (self->body.kind) {
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_UNIT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_unit_check_value(&self->body.arm.codec_zenoh_ext_unit));
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZINT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_zint_check_value(&self->body.arm.codec_zenoh_ext_zint));
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_CODEC_ZENOH_EXT_ZBUF:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_zbuf_check_value(&self->body.arm.codec_zenoh_ext_zbuf));
            break;
        case CODEC_ZENOH_EXT_ENTRY_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_unit_check_value(&self->body.arm.default_body));
            break;
        default:
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_EXT_ENTRY_MAX_BYTES bytes and use
 * `codec_zenoh_ext_entry_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_ext_entry_encode(const codec_zenoh_ext_entry_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_ext_entry_check_value(self));
    return codec_zenoh_ext_entry_encode_source(codec_zenoh_ext_entry_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_EXT_ENTRY_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_EXT_ENTRY_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_ext_entry_encode_to_buf(const codec_zenoh_ext_entry_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_ext_entry_encode(self, &_w);
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
static inline uint8_t codec_zenoh_ext_entry_ext_id(const codec_zenoh_ext_entry_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x0F);
}

static inline void codec_zenoh_ext_entry_set_ext_id(codec_zenoh_ext_entry_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x0F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x0F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_ext_entry_m(const codec_zenoh_ext_entry_t *self) {
    return (self->header & 0x10) != 0;
}

static inline void codec_zenoh_ext_entry_set_m(codec_zenoh_ext_entry_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x10);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x10));
    }
}

static inline uint8_t codec_zenoh_ext_entry_enc(const codec_zenoh_ext_entry_t *self) {
    return (uint8_t)((self->header >> 5) & (uint8_t)0x03);
}

static inline void codec_zenoh_ext_entry_set_enc(codec_zenoh_ext_entry_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x03 << 5);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x03) << 5);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_ext_entry_z(const codec_zenoh_ext_entry_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_zenoh_ext_entry_set_z(codec_zenoh_ext_entry_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_EXT_ENTRY_H */
