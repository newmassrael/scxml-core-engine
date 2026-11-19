// SCE-MAP: codec_zenoh_interest_body:56 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_INTEREST_BODY_H
#define SCE_FORGE_CODEC_ZENOH_INTEREST_BODY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */
#include "codec_zenoh_wireexpr.h"

#define CODEC_ZENOH_INTEREST_BODY_MIN_BYTES 1
#define CODEC_ZENOH_INTEREST_BODY_MAX_BYTES 147

typedef struct {
    uint8_t header;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    struct { bool present; codec_zenoh_wireexpr_t value; } keyexpr;
} codec_zenoh_interest_body_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_interest_body_decode(sce_forge_cursor_t *cursor, codec_zenoh_interest_body_t *out) {
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
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    memset(&out->keyexpr, 0, sizeof out->keyexpr);
    if ((out->header & 0x10) != 0) {
        sce_forge_codec_status_t _st = sce_forge_codec_note_capacity(codec_zenoh_wireexpr_decode(cursor, &out->keyexpr.value, (uint8_t)((out->header >> 5) & 0x1)), &_over);
        if (_st != SCE_FORGE_CODEC_OK) return _st;
        out->keyexpr.present = true;
    }
    if (_over) return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_zenoh_interest_body_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_interest_body_t is a source by the table
 * codec_zenoh_interest_body_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*header)(const void *ctx);
    codec_zenoh_wireexpr_source_ref_optional_t (*keyexpr)(const void *ctx);
} codec_zenoh_interest_body_source_t;

/* A source of codec_zenoh_interest_body_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_interest_body_source_t *table;
    const void *ctx;
} codec_zenoh_interest_body_source_ref_t;

/* A source of codec_zenoh_interest_body_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_interest_body_source_ref_optional, codec_zenoh_interest_body_source_ref_t)

/* The `header` of the codec_zenoh_interest_body_t at `ctx`. */
static inline uint8_t codec_zenoh_interest_body_value_header(const void *ctx) {
    return ((const codec_zenoh_interest_body_t *)ctx)->header;
}

/* The `keyexpr` of the codec_zenoh_interest_body_t at `ctx`. */
static inline codec_zenoh_wireexpr_source_ref_optional_t codec_zenoh_interest_body_value_keyexpr(const void *ctx) {
    const codec_zenoh_interest_body_t *self = (const codec_zenoh_interest_body_t *)ctx;
    if (!self->keyexpr.present) {
        return codec_zenoh_wireexpr_source_ref_optional_none();
    }
    codec_zenoh_wireexpr_source_ref_t body;
    body.table = codec_zenoh_wireexpr_value_source();
    body.ctx = &self->keyexpr.value;
    return codec_zenoh_wireexpr_source_ref_optional_some(body);
}

/* The table that presents a value of codec_zenoh_interest_body_t as a source of it, its
 * context a `const codec_zenoh_interest_body_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_interest_body_source_t *codec_zenoh_interest_body_value_source(void) {
    static const codec_zenoh_interest_body_source_t table = {
        codec_zenoh_interest_body_value_header,
        codec_zenoh_interest_body_value_keyexpr,
    };
    return &table;
}

/* One-bit flags of codec_zenoh_interest_body_t a parent may set in place of the ones a value
 * presents, such as the flag a chain whose entries are codec_zenoh_interest_body_t ends them
 * on, which the chain derives from where each entry stands. A flag absent
 * here is the value's own. */
typedef struct {
    /* The `keyexprs` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_keyexprs;
    /* The `subscribers` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_subscribers;
    /* The `queryables` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_queryables;
    /* The `tokens` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_tokens;
    /* The `M` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_m;
    /* The `aggregate` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_aggregate;
} codec_zenoh_interest_body_overrides_t;

/* The overrides that set no flag. */
static inline codec_zenoh_interest_body_overrides_t codec_zenoh_interest_body_overrides_none(void) {
    codec_zenoh_interest_body_overrides_t none;
    none.header_keyexprs = sce_optional_bool_none();
    none.header_subscribers = sce_optional_bool_none();
    none.header_queryables = sce_optional_bool_none();
    none.header_tokens = sce_optional_bool_none();
    none.header_m = sce_optional_bool_none();
    none.header_aggregate = sce_optional_bool_none();
    return none;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_interest_body_encode_source_overriding(const codec_zenoh_interest_body_source_t *src, const void *ctx, sce_forge_writer_t *w, codec_zenoh_interest_body_overrides_t overrides) {
    const uint8_t _e_header = src->header(ctx);
    const codec_zenoh_wireexpr_source_ref_optional_t _e_keyexpr = src->keyexpr(ctx);
    uint64_t _f_header = (uint64_t)_e_header;
    _f_header = (_f_header & ~0x10ull) | ((uint64_t)(_e_keyexpr.present) << 4);
    if (_e_keyexpr.present) {
        _f_header = (_f_header & ~0x20ull) | ((uint64_t)((size_t)codec_zenoh_wireexpr_input_n(_e_keyexpr.value.table, _e_keyexpr.value.ctx)) << 5);
    }
    if (overrides.header_keyexprs.present) {
        _f_header = (_f_header & ~0x1ull) | (uint64_t)overrides.header_keyexprs.value;
    }
    if (overrides.header_subscribers.present) {
        _f_header = (_f_header & ~0x2ull) | ((uint64_t)overrides.header_subscribers.value << 1);
    }
    if (overrides.header_queryables.present) {
        _f_header = (_f_header & ~0x4ull) | ((uint64_t)overrides.header_queryables.value << 2);
    }
    if (overrides.header_tokens.present) {
        _f_header = (_f_header & ~0x8ull) | ((uint64_t)overrides.header_tokens.value << 3);
    }
    if (overrides.header_m.present) {
        _f_header = (_f_header & ~0x40ull) | ((uint64_t)overrides.header_m.value << 6);
    }
    if (overrides.header_aggregate.present) {
        _f_header = (_f_header & ~0x80ull) | ((uint64_t)overrides.header_aggregate.value << 7);
    }
    const uint8_t _c_header = (uint8_t)_f_header;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_header));
    if (_e_keyexpr.present) {
        SCE_FORGE_TRY_WRITE(codec_zenoh_wireexpr_encode_source(_e_keyexpr.value.table, _e_keyexpr.value.ctx, w));
    }
    return SCE_FORGE_CODEC_OK;
}

/* codec_zenoh_interest_body_encode_source_overriding with no flag set in place of the
 * value's: the encode every caller but a parent that sets one calls. */
static inline sce_forge_codec_status_t codec_zenoh_interest_body_encode_source(const codec_zenoh_interest_body_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    return codec_zenoh_interest_body_encode_source_overriding(src, ctx, w, codec_zenoh_interest_body_overrides_none());
}

/* Whether `self` is a value of codec_zenoh_interest_body_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_interest_body_check_value(const codec_zenoh_interest_body_t *self) {
    if (self->keyexpr.present) {
        SCE_FORGE_TRY_WRITE(codec_zenoh_wireexpr_check_value(&self->keyexpr.value));
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_INTEREST_BODY_MAX_BYTES bytes and use
 * `codec_zenoh_interest_body_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_interest_body_encode(const codec_zenoh_interest_body_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_interest_body_check_value(self));
    return codec_zenoh_interest_body_encode_source(codec_zenoh_interest_body_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_INTEREST_BODY_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_INTEREST_BODY_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_interest_body_encode_to_buf(const codec_zenoh_interest_body_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_interest_body_encode(self, &_w);
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
static inline bool codec_zenoh_interest_body_keyexprs(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x01) != 0;
}

static inline void codec_zenoh_interest_body_set_keyexprs(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x01);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x01));
    }
}

static inline bool codec_zenoh_interest_body_subscribers(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x02) != 0;
}

static inline void codec_zenoh_interest_body_set_subscribers(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x02);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x02));
    }
}

static inline bool codec_zenoh_interest_body_queryables(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x04) != 0;
}

static inline void codec_zenoh_interest_body_set_queryables(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x04);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x04));
    }
}

static inline bool codec_zenoh_interest_body_tokens(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x08) != 0;
}

static inline void codec_zenoh_interest_body_set_tokens(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x08);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x08));
    }
}

static inline bool codec_zenoh_interest_body_restricted(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x10) != 0;
}

static inline void codec_zenoh_interest_body_set_restricted(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x10);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x10));
    }
}

static inline bool codec_zenoh_interest_body_n(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x20) != 0;
}

static inline void codec_zenoh_interest_body_set_n(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x20);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x20));
    }
}

static inline bool codec_zenoh_interest_body_m(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_zenoh_interest_body_set_m(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_zenoh_interest_body_aggregate(const codec_zenoh_interest_body_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_zenoh_interest_body_set_aggregate(codec_zenoh_interest_body_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_INTEREST_BODY_H */
