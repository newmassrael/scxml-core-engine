// SCE-MAP: codec_flags_basic:7 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_FLAGS_BASIC_H
#define SCE_FORGE_CODEC_FLAGS_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */

#define CODEC_FLAGS_BASIC_MIN_BYTES 1
#define CODEC_FLAGS_BASIC_MAX_BYTES 1

typedef struct {
    uint8_t header;
} codec_flags_basic_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_flags_basic_decode(sce_forge_cursor_t *cursor, codec_flags_basic_t *out) {
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_FLAGS_BASIC_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->header = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_FLAGS_BASIC_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    return SCE_FORGE_CODEC_OK;
}

/* The elements a value of codec_flags_basic_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_flags_basic_t is a source by the table
 * codec_flags_basic_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*header)(const void *ctx);
} codec_flags_basic_source_t;

/* A source of codec_flags_basic_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_flags_basic_source_t *table;
    const void *ctx;
} codec_flags_basic_source_ref_t;

/* A source of codec_flags_basic_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_flags_basic_source_ref_optional, codec_flags_basic_source_ref_t)

/* The `header` of the codec_flags_basic_t at `ctx`. */
static inline uint8_t codec_flags_basic_value_header(const void *ctx) {
    return ((const codec_flags_basic_t *)ctx)->header;
}

/* The table that presents a value of codec_flags_basic_t as a source of it, its
 * context a `const codec_flags_basic_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_flags_basic_source_t *codec_flags_basic_value_source(void) {
    static const codec_flags_basic_source_t table = {
        codec_flags_basic_value_header,
    };
    return &table;
}

/* One-bit flags of codec_flags_basic_t a parent may set in place of the ones a value
 * presents, such as the flag a chain whose entries are codec_flags_basic_t ends them
 * on, which the chain derives from where each entry stands. A flag absent
 * here is the value's own. */
typedef struct {
    /* The `reliable` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_reliable;
    /* The `more` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_more;
    /* The `drop` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_drop;
    /* The `first` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_first;
} codec_flags_basic_overrides_t;

/* The overrides that set no flag. */
static inline codec_flags_basic_overrides_t codec_flags_basic_overrides_none(void) {
    codec_flags_basic_overrides_t none;
    none.header_reliable = sce_optional_bool_none();
    none.header_more = sce_optional_bool_none();
    none.header_drop = sce_optional_bool_none();
    none.header_first = sce_optional_bool_none();
    return none;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_flags_basic_encode_source_overriding(const codec_flags_basic_source_t *src, const void *ctx, sce_forge_writer_t *w, codec_flags_basic_overrides_t overrides) {
    const uint8_t _e_header = src->header(ctx);
    uint64_t _f_header = (uint64_t)_e_header;
    if (overrides.header_reliable.present) {
        _f_header = (_f_header & ~0x80ull) | ((uint64_t)overrides.header_reliable.value << 7);
    }
    if (overrides.header_more.present) {
        _f_header = (_f_header & ~0x40ull) | ((uint64_t)overrides.header_more.value << 6);
    }
    if (overrides.header_drop.present) {
        _f_header = (_f_header & ~0x20ull) | ((uint64_t)overrides.header_drop.value << 5);
    }
    if (overrides.header_first.present) {
        _f_header = (_f_header & ~0x10ull) | ((uint64_t)overrides.header_first.value << 4);
    }
    const uint8_t _c_header = (uint8_t)_f_header;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_header));
    return SCE_FORGE_CODEC_OK;
}

/* codec_flags_basic_encode_source_overriding with no flag set in place of the
 * value's: the encode every caller but a parent that sets one calls. */
static inline sce_forge_codec_status_t codec_flags_basic_encode_source(const codec_flags_basic_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    return codec_flags_basic_encode_source_overriding(src, ctx, w, codec_flags_basic_overrides_none());
}

/* Whether `self` is a value of codec_flags_basic_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_flags_basic_check_value(const codec_flags_basic_t *self) {
    (void)self;
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_FLAGS_BASIC_MAX_BYTES bytes and use
 * `codec_flags_basic_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_flags_basic_encode(const codec_flags_basic_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_flags_basic_check_value(self));
    return codec_flags_basic_encode_source(codec_flags_basic_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_FLAGS_BASIC_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_FLAGS_BASIC_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_flags_basic_encode_to_buf(const codec_flags_basic_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_flags_basic_encode(self, &_w);
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
static inline bool codec_flags_basic_reliable(const codec_flags_basic_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_flags_basic_set_reliable(codec_flags_basic_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

static inline bool codec_flags_basic_more(const codec_flags_basic_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_flags_basic_set_more(codec_flags_basic_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_flags_basic_drop(const codec_flags_basic_t *self) {
    return (self->header & 0x20) != 0;
}

static inline void codec_flags_basic_set_drop(codec_flags_basic_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x20);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x20));
    }
}

static inline bool codec_flags_basic_first(const codec_flags_basic_t *self) {
    return (self->header & 0x10) != 0;
}

static inline void codec_flags_basic_set_first(codec_flags_basic_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x10);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x10));
    }
}

#endif  /* SCE_FORGE_CODEC_FLAGS_BASIC_H */
