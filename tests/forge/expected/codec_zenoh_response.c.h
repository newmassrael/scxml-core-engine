// SCE-MAP: codec_zenoh_response:76 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_RESPONSE_H
#define SCE_FORGE_CODEC_ZENOH_RESPONSE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a field gated by `sce:present-if`, docs/adr/0055 */
#include "codec_zenoh_ext_entry.h"
#include "codec_zenoh_reply.h"
#include "codec_zenoh_err.h"

#define CODEC_ZENOH_RESPONSE_MIN_BYTES 1
#define CODEC_ZENOH_RESPONSE_MAX_BYTES 17179870035

/* RFC §synth-5-B variant primitive: tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_REPLY,
    CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_ERR,
    CODEC_ZENOH_RESPONSE_BODY_KIND_DEFAULT,
} codec_zenoh_response_body_kind_t;

typedef struct {
    codec_zenoh_response_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_zenoh_reply_t codec_zenoh_reply;
        codec_zenoh_err_t codec_zenoh_err;
        codec_zenoh_reply_t default_body;
    } arm;
} codec_zenoh_response_variant_t;

#ifndef SCE_OPTIONAL_BYTES_256
#define SCE_OPTIONAL_BYTES_256
#ifndef SCE_STATIC_BYTES_256
#define SCE_STATIC_BYTES_256
/* SCE Accepted Subset §2.15: a `bytes` of at most 256 bytes. */
typedef struct {
    uint8_t data[256];
    size_t len;
} sce_static_bytes_256_t;

static inline sce_static_bytes_256_t sce_static_bytes_256_of(sce_forge_bytes_view_t bytes) {
    sce_static_bytes_256_t value = { { 0 }, 0 };
    for (size_t i = 0; i < bytes.len && i < 256u; ++i) {
        value.data[i] = bytes.data[i];
    }
    value.len = bytes.len < 256u ? bytes.len : 256u;
    return value;
}
#endif
/* docs/adr/0055: a value of `sce_static_bytes_256_t` that may be absent. */
SCE_OPTIONAL_DECLARE_BYTES(256)
#endif

typedef struct {
    uint8_t header;
    uint64_t request_id;
    uint32_t key_id;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    sce_optional_bytes_256_t suffix;
    /* `sce:present-if`: an optional value (`present`, `value`), docs/adr/0055 step 14 */
    struct { bool present; struct { codec_zenoh_ext_entry_t data[4]; size_t len; } value; } extensions;
    codec_zenoh_response_variant_t body;
} codec_zenoh_response_t;

/* RFC variant-default-uniformity (C11): designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_zenoh_response_t x = CODEC_ZENOH_RESPONSE_DEFAULT_INIT;
 * codec_zenoh_response_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_ZENOH_RESPONSE_DEFAULT_INIT { \
    .header = 0x1bu, \
    .body = { \
        .kind = CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_REPLY, \
        .arm = { .codec_zenoh_reply = CODEC_ZENOH_REPLY_DEFAULT_INIT } \
    }, \
}

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_response_decode(sce_forge_cursor_t *cursor, codec_zenoh_response_t *out) {
    /* A field or a body past what this node holds: the frame is read on,
     * judged as one that holds it would be, and the capacity is answered
     * once every other refusal has had its turn (docs/adr/0077 §7). */
    bool _over = false;
    (void)_over;
    /* RFC §synth-5-B peek-byte / streaming-prefix:
     * streaming prefix decode (variable-length fields supported via
     * per-field present_if/tlv-chain/embed/repeat helpers). Peek-byte
     * mode additionally peeks the cursor's next byte for variant tag
     * without advancing — arm body decoder reads it as own header. */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    uint64_t request_id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &request_id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->request_id = request_id;
    uint32_t key_id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u32(cursor, &key_id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->key_id = key_id;
    uint64_t _d_suffix_len = 0;
    if ((out->header & 0x20) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        _d_suffix_len = _v;
    }
    if ((out->header & 0x20) != 0) {
        if (_d_suffix_len > 65535) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        size_t _n = (size_t)_d_suffix_len;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        if (!sce_forge_is_valid_utf8(raw, _n)) return SCE_FORGE_CODEC_INVALID_UTF8;
        out->suffix = sce_optional_bytes_256_none();
        out->suffix.value.len = sce_forge_codec_hold(out->suffix.value.data, 256U, raw, _n, &_over);
        out->suffix.present = true;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->suffix = sce_optional_bytes_256_none();
    }
    memset(&out->extensions, 0, sizeof out->extensions);
        if ((out->header & 0x80) != 0) {
            out->extensions.present = true;
            bool _more = false;
            for (;;) {
                size_t _just = out->extensions.value.len < 4U ? out->extensions.value.len : 4U - 1U;
                sce_forge_codec_status_t _st = sce_forge_codec_note_capacity(codec_zenoh_ext_entry_decode(cursor, &out->extensions.value.data[_just]), &_over);
                if (_st != SCE_FORGE_CODEC_OK) return _st;
                if (out->extensions.value.len < 4U) out->extensions.value.len++; else _over = true;
                _more = codec_zenoh_ext_entry_z(&out->extensions.value.data[_just]);
                if (!_more) break;
            }
            if (_more && sce_forge_cursor_remaining(cursor) == 0) { return SCE_FORGE_CODEC_NEED_MORE_BYTES; }
        }
    const uint8_t *_peek_raw = sce_forge_cursor_peek(cursor, 1);
    if (_peek_raw == NULL) {
        return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    const uint8_t _peek = _peek_raw[0];
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch ((uint8_t)((_peek >> 0) & (uint8_t)0x1F)) {
        case 4:
            out->body.kind = CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_REPLY;
            _arm_st = codec_zenoh_reply_decode(cursor, &out->body.arm.codec_zenoh_reply);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 5:
            out->body.kind = CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_ERR;
            _arm_st = codec_zenoh_err_decode(cursor, &out->body.arm.codec_zenoh_err);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_ZENOH_RESPONSE_BODY_KIND_DEFAULT;
            out->body.default_tag = (uint8_t)((_peek >> 0) & (uint8_t)0x1F);
            _arm_st = codec_zenoh_reply_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    if (_over) return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
    return SCE_FORGE_CODEC_OK;
}

/* The `suffix` element of codec_zenoh_response_t: a text of 0 to 65535 bytes of UTF-8, viewed where it
 * is held, which outlives the view. A view is made by codec_zenoh_response_suffix_text_make, which
 * refuses one outside the bound, so every view holds it (docs/adr/0081 §2)
 * and the encode over a source writes it whole, refusing nothing of it. */
typedef struct {
    const char *data;
    size_t len;
} codec_zenoh_response_suffix_text_t;

/* Make `*out` a view of the `len` bytes of UTF-8 at `data`: SCE_FORGE_CODEC_OK, or
 * the refusal of a text outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_zenoh_response_suffix_text_make(codec_zenoh_response_suffix_text_t *out, const char *data, size_t len) {
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

/* A view of codec_zenoh_response_suffix_text_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_response_suffix_text_optional, codec_zenoh_response_suffix_text_t)

/* The `extensions` element of codec_zenoh_response_t: a list of 1 or more entries, the protocol stating no most of codec_zenoh_ext_entry_t,
 * walked where it is held, which outlives the list. A list is made by
 * codec_zenoh_response_extensions_list_make, which walks it once, counts its entries and refuses one
 * outside the bound, so every list holds it (docs/adr/0081 §2) and the
 * encode over a source writes its `count` entries, refusing nothing of
 * the count. */
typedef struct {
    const sce_forge_walk_t *walk;
    const void *ctx;
    const codec_zenoh_ext_entry_source_t *entry;
    size_t count;
} codec_zenoh_response_extensions_list_t;

/* Make `*out` the list `walk` presents over `ctx`, each entry a source of
 * codec_zenoh_ext_entry_t by `entry`: SCE_FORGE_CODEC_OK, or the refusal of a list
 * outside its bound, `*out` then as it was. */
static inline sce_forge_codec_status_t codec_zenoh_response_extensions_list_make(codec_zenoh_response_extensions_list_t *out, const sce_forge_walk_t *walk, const void *ctx, const codec_zenoh_ext_entry_source_t *entry) {
    size_t count = 0;
    for (const void *at = walk->begin(ctx); at != NULL; at = walk->next(ctx, at)) {
        ++count;
    }
    if (count < 1u) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    out->walk = walk;
    out->ctx = ctx;
    out->entry = entry;
    out->count = count;
    return SCE_FORGE_CODEC_OK;
}

/* A list of codec_zenoh_response_extensions_list_t that may be absent: absent where the gate leaves the
 * element out. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_response_extensions_list_optional, codec_zenoh_response_extensions_list_t)

/* The arm of the variant a source of codec_zenoh_response_t presents (docs/adr/0081
 * §2): which arm it is, the tag the catch-all holds, and the arm's body,
 * a source of the arm's codec in the member `kind` names. */
typedef struct {
    codec_zenoh_response_body_kind_t kind;
    uint8_t default_tag;  /* the catch-all's */
    union {
        codec_zenoh_reply_source_ref_t codec_zenoh_reply;
        codec_zenoh_err_source_ref_t codec_zenoh_err;
        codec_zenoh_reply_source_ref_t default_body;
    } arm;
} codec_zenoh_response_body_ref_t;

/* The elements a value of codec_zenoh_response_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_response_t is a source by the table
 * codec_zenoh_response_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*header)(const void *ctx);
    uint64_t (*request_id)(const void *ctx);
    uint32_t (*key_id)(const void *ctx);
    codec_zenoh_response_suffix_text_optional_t (*suffix)(const void *ctx);
    codec_zenoh_response_extensions_list_optional_t (*extensions)(const void *ctx);
    codec_zenoh_response_body_ref_t (*body)(const void *ctx);
} codec_zenoh_response_source_t;

/* A source of codec_zenoh_response_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_response_source_t *table;
    const void *ctx;
} codec_zenoh_response_source_ref_t;

/* A source of codec_zenoh_response_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_response_source_ref_optional, codec_zenoh_response_source_ref_t)

/* The `header` of the codec_zenoh_response_t at `ctx`. */
static inline uint8_t codec_zenoh_response_value_header(const void *ctx) {
    return ((const codec_zenoh_response_t *)ctx)->header;
}

/* The `request_id` of the codec_zenoh_response_t at `ctx`. */
static inline uint64_t codec_zenoh_response_value_request_id(const void *ctx) {
    return ((const codec_zenoh_response_t *)ctx)->request_id;
}

/* The `key_id` of the codec_zenoh_response_t at `ctx`. */
static inline uint32_t codec_zenoh_response_value_key_id(const void *ctx) {
    return ((const codec_zenoh_response_t *)ctx)->key_id;
}

/* The `suffix` of the codec_zenoh_response_t at `ctx`. */
static inline codec_zenoh_response_suffix_text_optional_t codec_zenoh_response_value_suffix(const void *ctx) {
    const codec_zenoh_response_t *self = (const codec_zenoh_response_t *)ctx;
    if (!self->suffix.present) {
        return codec_zenoh_response_suffix_text_optional_none();
    }
    codec_zenoh_response_suffix_text_t view;
    view.data = (const char *)self->suffix.value.data;
    view.len = self->suffix.value.len;
    return codec_zenoh_response_suffix_text_optional_some(view);
}

/* The first entry of the `extensions` of the codec_zenoh_response_t at `ctx`, or NULL where
 * it holds none. */
static inline const void *codec_zenoh_response_value_extensions_begin(const void *ctx) {
    const codec_zenoh_response_t *self = (const codec_zenoh_response_t *)ctx;
    return self->extensions.value.len > 0u ? (const void *)&self->extensions.value.data[0] : NULL;
}

/* The entry after `entry` of the `extensions` of the codec_zenoh_response_t at `ctx`, or
 * NULL where `entry` is its last. */
static inline const void *codec_zenoh_response_value_extensions_next(const void *ctx, const void *entry) {
    const codec_zenoh_response_t *self = (const codec_zenoh_response_t *)ctx;
    const codec_zenoh_ext_entry_t *at = (const codec_zenoh_ext_entry_t *)entry + 1;
    return at < self->extensions.value.data + self->extensions.value.len ? (const void *)at : NULL;
}

/* The `extensions` of the codec_zenoh_response_t at `ctx`. */
static inline codec_zenoh_response_extensions_list_optional_t codec_zenoh_response_value_extensions(const void *ctx) {
    static const sce_forge_walk_t walk = {
        codec_zenoh_response_value_extensions_begin,
        codec_zenoh_response_value_extensions_next,
    };
    const codec_zenoh_response_t *self = (const codec_zenoh_response_t *)ctx;
    if (!self->extensions.present) {
        return codec_zenoh_response_extensions_list_optional_none();
    }
    codec_zenoh_response_extensions_list_t list;
    list.walk = &walk;
    list.ctx = ctx;
    list.entry = codec_zenoh_ext_entry_value_source();
    list.count = self->extensions.value.len;
    return codec_zenoh_response_extensions_list_optional_some(list);
}

/* The arm of the variant of the codec_zenoh_response_t at `ctx`: its kind, the tag the
 * catch-all holds, and its body as a source of the arm's codec. */
static inline codec_zenoh_response_body_ref_t codec_zenoh_response_value_body(const void *ctx) {
    const codec_zenoh_response_t *self = (const codec_zenoh_response_t *)ctx;
    codec_zenoh_response_body_ref_t ref;
    memset(&ref, 0, sizeof ref);
    ref.kind = self->body.kind;
    ref.default_tag = self->body.default_tag;
    switch (self->body.kind) {
        case CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_REPLY:
            ref.arm.codec_zenoh_reply.table = codec_zenoh_reply_value_source();
            ref.arm.codec_zenoh_reply.ctx = &self->body.arm.codec_zenoh_reply;
            break;
        case CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_ERR:
            ref.arm.codec_zenoh_err.table = codec_zenoh_err_value_source();
            ref.arm.codec_zenoh_err.ctx = &self->body.arm.codec_zenoh_err;
            break;
        case CODEC_ZENOH_RESPONSE_BODY_KIND_DEFAULT:
            ref.arm.default_body.table = codec_zenoh_reply_value_source();
            ref.arm.default_body.ctx = &self->body.arm.default_body;
            break;
        default:
            break;
    }
    return ref;
}

/* The table that presents a value of codec_zenoh_response_t as a source of it, its
 * context a `const codec_zenoh_response_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_response_source_t *codec_zenoh_response_value_source(void) {
    static const codec_zenoh_response_source_t table = {
        codec_zenoh_response_value_header,
        codec_zenoh_response_value_request_id,
        codec_zenoh_response_value_key_id,
        codec_zenoh_response_value_suffix,
        codec_zenoh_response_value_extensions,
        codec_zenoh_response_value_body,
    };
    return &table;
}

/* One-bit flags of codec_zenoh_response_t a parent may set in place of the ones a value
 * presents, such as the flag a chain whose entries are codec_zenoh_response_t ends them
 * on, which the chain derives from where each entry stands. A flag absent
 * here is the value's own. */
typedef struct {
    /* The `M` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_m;
} codec_zenoh_response_overrides_t;

/* The overrides that set no flag. */
static inline codec_zenoh_response_overrides_t codec_zenoh_response_overrides_none(void) {
    codec_zenoh_response_overrides_t none;
    none.header_m = sce_optional_bool_none();
    return none;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_response_encode_source_overriding(const codec_zenoh_response_source_t *src, const void *ctx, sce_forge_writer_t *w, codec_zenoh_response_overrides_t overrides) {
    const uint8_t _e_header = src->header(ctx);
    const uint64_t _e_request_id = src->request_id(ctx);
    const uint32_t _e_key_id = src->key_id(ctx);
    const codec_zenoh_response_suffix_text_optional_t _e_suffix = src->suffix(ctx);
    const codec_zenoh_response_extensions_list_optional_t _e_extensions = src->extensions(ctx);
    const codec_zenoh_response_body_ref_t _e_body = src->body(ctx);
    const uint64_t _d_suffix_len = _e_suffix.present ? (uint64_t)(_e_suffix.value.len) : (uint64_t)0;
    uint64_t _f_header = (uint64_t)_e_header;
    _f_header = (_f_header & ~0x20ull) | ((uint64_t)(_e_suffix.present) << 5);
    _f_header = (_f_header & ~0x80ull) | ((uint64_t)(_e_extensions.present) << 7);
    if (overrides.header_m.present) {
        _f_header = (_f_header & ~0x40ull) | ((uint64_t)overrides.header_m.value << 6);
    }
    const uint8_t _c_header = (uint8_t)_f_header;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_header));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_e_request_id)));
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u32(w, (uint32_t)(_e_key_id)));
    if ((((uint64_t)_c_header & 0x20ull) != 0)) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_vle_u64(w, (uint64_t)(_d_suffix_len)));
    }
    if (_e_suffix.present) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, (const uint8_t *)_e_suffix.value.data, _e_suffix.value.len));
    }
    if (_e_extensions.present) {
        {
            const void *_at = _e_extensions.value.walk->begin(_e_extensions.value.ctx);
            for (size_t _i = 0; _i < _e_extensions.value.count && _at != NULL; ++_i) {
                codec_zenoh_ext_entry_overrides_t _o = codec_zenoh_ext_entry_overrides_none();
                _o.header_z = sce_optional_bool_some(_i + 1u < _e_extensions.value.count);
                SCE_FORGE_TRY_WRITE(codec_zenoh_ext_entry_encode_source_overriding(_e_extensions.value.entry, _at, w, _o));
                _at = _e_extensions.value.walk->next(_e_extensions.value.ctx, _at);
            }
        }
    }
    switch (_e_body.kind) {
        case CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_REPLY:
            SCE_FORGE_TRY_WRITE(codec_zenoh_reply_encode_source(_e_body.arm.codec_zenoh_reply.table, _e_body.arm.codec_zenoh_reply.ctx, w));
            break;
        case CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_ERR:
            SCE_FORGE_TRY_WRITE(codec_zenoh_err_encode_source(_e_body.arm.codec_zenoh_err.table, _e_body.arm.codec_zenoh_err.ctx, w));
            break;
        case CODEC_ZENOH_RESPONSE_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_reply_encode_source(_e_body.arm.default_body.table, _e_body.arm.default_body.ctx, w));
            break;
        default:
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* codec_zenoh_response_encode_source_overriding with no flag set in place of the
 * value's: the encode every caller but a parent that sets one calls. */
static inline sce_forge_codec_status_t codec_zenoh_response_encode_source(const codec_zenoh_response_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    return codec_zenoh_response_encode_source_overriding(src, ctx, w, codec_zenoh_response_overrides_none());
}

/* Whether `self` is a value of codec_zenoh_response_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_response_check_value(const codec_zenoh_response_t *self) {
    if (self->suffix.present && (!sce_forge_length_within_bound(self->suffix.value.len, 256))) {
        return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
    }
    if (self->suffix.present && (!sce_forge_is_valid_utf8(self->suffix.value.data, self->suffix.value.len))) {
        return SCE_FORGE_CODEC_INVALID_UTF8;
    }
    if (self->extensions.present) {
        if (!sce_forge_length_within_bound(self->extensions.value.len, 4)) {
            return SCE_FORGE_CODEC_TOO_MANY_ELEMENTS;
        }
        if (self->extensions.value.len < 1u) {
            return SCE_FORGE_CODEC_LENGTH_OUT_OF_RANGE;
        }
        for (size_t _i = 0; _i < self->extensions.value.len; ++_i) {
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_entry_check_value(&self->extensions.value.data[_i]));
        }
    }
    switch (self->body.kind) {
        case CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_REPLY:
            SCE_FORGE_TRY_WRITE(codec_zenoh_reply_check_value(&self->body.arm.codec_zenoh_reply));
            break;
        case CODEC_ZENOH_RESPONSE_BODY_KIND_CODEC_ZENOH_ERR:
            SCE_FORGE_TRY_WRITE(codec_zenoh_err_check_value(&self->body.arm.codec_zenoh_err));
            break;
        case CODEC_ZENOH_RESPONSE_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_reply_check_value(&self->body.arm.default_body));
            break;
        default:
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_RESPONSE_MAX_BYTES bytes and use
 * `codec_zenoh_response_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_response_encode(const codec_zenoh_response_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_response_check_value(self));
    return codec_zenoh_response_encode_source(codec_zenoh_response_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_RESPONSE_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_RESPONSE_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_response_encode_to_buf(const codec_zenoh_response_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_response_encode(self, &_w);
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
static inline uint8_t codec_zenoh_response_mid(const codec_zenoh_response_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x1F);
}

static inline void codec_zenoh_response_set_mid(codec_zenoh_response_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x1F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x1F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_response_n(const codec_zenoh_response_t *self) {
    return (self->header & 0x20) != 0;
}

static inline void codec_zenoh_response_set_n(codec_zenoh_response_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x20);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x20));
    }
}

static inline bool codec_zenoh_response_m(const codec_zenoh_response_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_zenoh_response_set_m(codec_zenoh_response_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_zenoh_response_z(const codec_zenoh_response_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_zenoh_response_set_z(codec_zenoh_response_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_RESPONSE_H */
