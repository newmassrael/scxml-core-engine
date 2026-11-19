// SCE-MAP: codec_zenoh_declaration:54 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_DECLARATION_H
#define SCE_FORGE_CODEC_ZENOH_DECLARATION_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#include "sce/forge/codec.h"
#include "sce/forge/optional.h"  /* a source of the codec that may be absent, docs/adr/0081 */
#include "codec_zenoh_decl_kexpr.h"
#include "codec_zenoh_undecl_kexpr.h"
#include "codec_zenoh_decl_subscriber.h"
#include "codec_zenoh_undecl_subscriber.h"
#include "codec_zenoh_decl_queryable.h"
#include "codec_zenoh_undecl_queryable.h"
#include "codec_zenoh_decl_token.h"
#include "codec_zenoh_undecl_token.h"
#include "codec_zenoh_decl_final.h"

#define CODEC_ZENOH_DECLARATION_MIN_BYTES 1
#define CODEC_ZENOH_DECLARATION_MAX_BYTES 65571

/* RFC §synth-5-B variant primitive: tagged-union body for the codec's
 * tag-field suffix. `kind` discriminates the active arm; `default_tag`
 * preserves the runtime tag value when the default arm fires; the inner
 * union holds one body slot per arm (per-arm fields keep the template
 * straight when two arms share a body type). */
typedef enum {
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_KEXPR,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_KEXPR,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_SUBSCRIBER,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_SUBSCRIBER,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_QUERYABLE,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_QUERYABLE,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_TOKEN,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_TOKEN,
    CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_FINAL,
    CODEC_ZENOH_DECLARATION_BODY_KIND_DEFAULT,
} codec_zenoh_declaration_body_kind_t;

typedef struct {
    codec_zenoh_declaration_body_kind_t kind;
    uint8_t default_tag;  /* valid only when kind == ..._DEFAULT */
    union {
        codec_zenoh_decl_kexpr_t codec_zenoh_decl_kexpr;
        codec_zenoh_undecl_kexpr_t codec_zenoh_undecl_kexpr;
        codec_zenoh_decl_subscriber_t codec_zenoh_decl_subscriber;
        codec_zenoh_undecl_subscriber_t codec_zenoh_undecl_subscriber;
        codec_zenoh_decl_queryable_t codec_zenoh_decl_queryable;
        codec_zenoh_undecl_queryable_t codec_zenoh_undecl_queryable;
        codec_zenoh_decl_token_t codec_zenoh_decl_token;
        codec_zenoh_undecl_token_t codec_zenoh_undecl_token;
        codec_zenoh_decl_final_t codec_zenoh_decl_final;
        codec_zenoh_decl_final_t default_body;
    } arm;
} codec_zenoh_declaration_variant_t;

typedef struct {
    uint8_t header;
    codec_zenoh_declaration_variant_t body;
} codec_zenoh_declaration_t;

/* RFC variant-default-uniformity (C11): designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_zenoh_declaration_t x = CODEC_ZENOH_DECLARATION_DEFAULT_INIT;
 * codec_zenoh_declaration_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_ZENOH_DECLARATION_DEFAULT_INIT { \
    .body = { \
        .kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_FINAL, \
        .arm = { .codec_zenoh_decl_final = CODEC_ZENOH_DECL_FINAL_DEFAULT_INIT } \
    }, \
}

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §synth-5-B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_declaration_decode(sce_forge_cursor_t *cursor, codec_zenoh_declaration_t *out) {
    /* A field or a body past what this node holds: the frame is read on,
     * judged as one that holds it would be, and the capacity is answered
     * once every other refusal has had its turn (docs/adr/0077 §7). */
    bool _over = false;
    (void)_over;
    /* Decode fixed prefix (RFC §synth-5-B variant: fields before tag suffix). */
    const uint8_t *raw = sce_forge_cursor_peek(cursor, CODEC_ZENOH_DECLARATION_MIN_BYTES);
    if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    out->header = raw[0];
    if (!sce_forge_cursor_advance(cursor, CODEC_ZENOH_DECLARATION_MIN_BYTES)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    /* Dispatch on the tag field; each arm decodes its body codec from
     * the cursor. The default arm (when declared) carries the runtime
     * tag value so encode can round-trip it back onto the wire. */
    out->body.default_tag = 0;
    sce_forge_codec_status_t _arm_st;
    switch ((uint8_t)((out->header >> 0) & (uint8_t)0x1F)) {
        case 0:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_KEXPR;
            _arm_st = codec_zenoh_decl_kexpr_decode(cursor, &out->body.arm.codec_zenoh_decl_kexpr, (uint8_t)((out->header >> 5) & 0x1));
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 1:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_KEXPR;
            _arm_st = codec_zenoh_undecl_kexpr_decode(cursor, &out->body.arm.codec_zenoh_undecl_kexpr);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 2:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_SUBSCRIBER;
            _arm_st = codec_zenoh_decl_subscriber_decode(cursor, &out->body.arm.codec_zenoh_decl_subscriber, (uint8_t)((out->header >> 5) & 0x1));
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 3:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_SUBSCRIBER;
            _arm_st = codec_zenoh_undecl_subscriber_decode(cursor, &out->body.arm.codec_zenoh_undecl_subscriber, (uint8_t)((out->header >> 7) & 0x1));
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 4:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_QUERYABLE;
            _arm_st = codec_zenoh_decl_queryable_decode(cursor, &out->body.arm.codec_zenoh_decl_queryable, (uint8_t)((out->header >> 5) & 0x1), (uint8_t)((out->header >> 7) & 0x1));
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 5:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_QUERYABLE;
            _arm_st = codec_zenoh_undecl_queryable_decode(cursor, &out->body.arm.codec_zenoh_undecl_queryable, (uint8_t)((out->header >> 7) & 0x1));
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 6:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_TOKEN;
            _arm_st = codec_zenoh_decl_token_decode(cursor, &out->body.arm.codec_zenoh_decl_token, (uint8_t)((out->header >> 5) & 0x1));
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 7:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_TOKEN;
            _arm_st = codec_zenoh_undecl_token_decode(cursor, &out->body.arm.codec_zenoh_undecl_token, (uint8_t)((out->header >> 7) & 0x1));
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        case 26:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_FINAL;
            _arm_st = codec_zenoh_decl_final_decode(cursor, &out->body.arm.codec_zenoh_decl_final);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
        default:
            out->body.kind = CODEC_ZENOH_DECLARATION_BODY_KIND_DEFAULT;
            out->body.default_tag = (uint8_t)((out->header >> 0) & (uint8_t)0x1F);
            _arm_st = codec_zenoh_decl_final_decode(cursor, &out->body.arm.default_body);
            if (_arm_st != SCE_FORGE_CODEC_OK) return _arm_st;
            break;
    }
    if (_over) return SCE_FORGE_CODEC_CAPACITY_EXCEEDED;
    return SCE_FORGE_CODEC_OK;
}

/* The arm of the variant a source of codec_zenoh_declaration_t presents (docs/adr/0081
 * §2): which arm it is, the tag the catch-all holds, and the arm's body,
 * a source of the arm's codec in the member `kind` names. */
typedef struct {
    codec_zenoh_declaration_body_kind_t kind;
    uint8_t default_tag;  /* the catch-all's */
    union {
        codec_zenoh_decl_kexpr_source_ref_t codec_zenoh_decl_kexpr;
        codec_zenoh_undecl_kexpr_source_ref_t codec_zenoh_undecl_kexpr;
        codec_zenoh_decl_subscriber_source_ref_t codec_zenoh_decl_subscriber;
        codec_zenoh_undecl_subscriber_source_ref_t codec_zenoh_undecl_subscriber;
        codec_zenoh_decl_queryable_source_ref_t codec_zenoh_decl_queryable;
        codec_zenoh_undecl_queryable_source_ref_t codec_zenoh_undecl_queryable;
        codec_zenoh_decl_token_source_ref_t codec_zenoh_decl_token;
        codec_zenoh_undecl_token_source_ref_t codec_zenoh_undecl_token;
        codec_zenoh_decl_final_source_ref_t codec_zenoh_decl_final;
        codec_zenoh_decl_final_source_ref_t default_body;
    } arm;
} codec_zenoh_declaration_body_ref_t;

/* The elements a value of codec_zenoh_declaration_t presents, which its encode reads
 * (docs/adr/0081 §2): one function for each, handed the context the
 * table is passed with. A value of codec_zenoh_declaration_t is a source by the table
 * codec_zenoh_declaration_value_source() answers, its context the value; any other type
 * that presents the elements is one by a table of its own. The encode
 * derives every other element of the frame. */
typedef struct {
    uint8_t (*header)(const void *ctx);
    codec_zenoh_declaration_body_ref_t (*body)(const void *ctx);
} codec_zenoh_declaration_source_t;

/* A source of codec_zenoh_declaration_t: the table of its elements and the context the
 * table is handed, as a codec that holds one presents it to its encode. */
typedef struct {
    const codec_zenoh_declaration_source_t *table;
    const void *ctx;
} codec_zenoh_declaration_source_ref_t;

/* A source of codec_zenoh_declaration_t that may be absent. */
SCE_OPTIONAL_DECLARE_NAMED(codec_zenoh_declaration_source_ref_optional, codec_zenoh_declaration_source_ref_t)

/* The `header` of the codec_zenoh_declaration_t at `ctx`. */
static inline uint8_t codec_zenoh_declaration_value_header(const void *ctx) {
    return ((const codec_zenoh_declaration_t *)ctx)->header;
}

/* The arm of the variant of the codec_zenoh_declaration_t at `ctx`: its kind, the tag the
 * catch-all holds, and its body as a source of the arm's codec. */
static inline codec_zenoh_declaration_body_ref_t codec_zenoh_declaration_value_body(const void *ctx) {
    const codec_zenoh_declaration_t *self = (const codec_zenoh_declaration_t *)ctx;
    codec_zenoh_declaration_body_ref_t ref;
    memset(&ref, 0, sizeof ref);
    ref.kind = self->body.kind;
    ref.default_tag = self->body.default_tag;
    switch (self->body.kind) {
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_KEXPR:
            ref.arm.codec_zenoh_decl_kexpr.table = codec_zenoh_decl_kexpr_value_source();
            ref.arm.codec_zenoh_decl_kexpr.ctx = &self->body.arm.codec_zenoh_decl_kexpr;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_KEXPR:
            ref.arm.codec_zenoh_undecl_kexpr.table = codec_zenoh_undecl_kexpr_value_source();
            ref.arm.codec_zenoh_undecl_kexpr.ctx = &self->body.arm.codec_zenoh_undecl_kexpr;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_SUBSCRIBER:
            ref.arm.codec_zenoh_decl_subscriber.table = codec_zenoh_decl_subscriber_value_source();
            ref.arm.codec_zenoh_decl_subscriber.ctx = &self->body.arm.codec_zenoh_decl_subscriber;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_SUBSCRIBER:
            ref.arm.codec_zenoh_undecl_subscriber.table = codec_zenoh_undecl_subscriber_value_source();
            ref.arm.codec_zenoh_undecl_subscriber.ctx = &self->body.arm.codec_zenoh_undecl_subscriber;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_QUERYABLE:
            ref.arm.codec_zenoh_decl_queryable.table = codec_zenoh_decl_queryable_value_source();
            ref.arm.codec_zenoh_decl_queryable.ctx = &self->body.arm.codec_zenoh_decl_queryable;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_QUERYABLE:
            ref.arm.codec_zenoh_undecl_queryable.table = codec_zenoh_undecl_queryable_value_source();
            ref.arm.codec_zenoh_undecl_queryable.ctx = &self->body.arm.codec_zenoh_undecl_queryable;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_TOKEN:
            ref.arm.codec_zenoh_decl_token.table = codec_zenoh_decl_token_value_source();
            ref.arm.codec_zenoh_decl_token.ctx = &self->body.arm.codec_zenoh_decl_token;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_TOKEN:
            ref.arm.codec_zenoh_undecl_token.table = codec_zenoh_undecl_token_value_source();
            ref.arm.codec_zenoh_undecl_token.ctx = &self->body.arm.codec_zenoh_undecl_token;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_FINAL:
            ref.arm.codec_zenoh_decl_final.table = codec_zenoh_decl_final_value_source();
            ref.arm.codec_zenoh_decl_final.ctx = &self->body.arm.codec_zenoh_decl_final;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_DEFAULT:
            ref.arm.default_body.table = codec_zenoh_decl_final_value_source();
            ref.arm.default_body.ctx = &self->body.arm.default_body;
            break;
        default:
            break;
    }
    return ref;
}

/* The table that presents a value of codec_zenoh_declaration_t as a source of it, its
 * context a `const codec_zenoh_declaration_t *`. One table for every value, in read-only
 * memory. */
static inline const codec_zenoh_declaration_source_t *codec_zenoh_declaration_value_source(void) {
    static const codec_zenoh_declaration_source_t table = {
        codec_zenoh_declaration_value_header,
        codec_zenoh_declaration_value_body,
    };
    return &table;
}

/* One-bit flags of codec_zenoh_declaration_t a parent may set in place of the ones a value
 * presents, such as the flag a chain whose entries are codec_zenoh_declaration_t ends them
 * on, which the chain derives from where each entry stands. A flag absent
 * here is the value's own. */
typedef struct {
    /* The `M` flag of `header`, where a parent sets it. */
    sce_optional_bool_t header_m;
} codec_zenoh_declaration_overrides_t;

/* The overrides that set no flag. */
static inline codec_zenoh_declaration_overrides_t codec_zenoh_declaration_overrides_none(void) {
    codec_zenoh_declaration_overrides_t none;
    none.header_m = sce_optional_bool_none();
    return none;
}

/* Write the source `src` presents over `ctx` into `w`, each element read
 * once and written where the codec's encode plan lays it out
 * (docs/adr/0081). It refuses no value: it returns SCE_FORGE_CODEC_OK, or
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW where `w` has no room left. */
static inline sce_forge_codec_status_t codec_zenoh_declaration_encode_source_overriding(const codec_zenoh_declaration_source_t *src, const void *ctx, sce_forge_writer_t *w, codec_zenoh_declaration_overrides_t overrides) {
    const uint8_t _e_header = src->header(ctx);
    const codec_zenoh_declaration_body_ref_t _e_body = src->body(ctx);
    uint64_t _t_body = 0;
    switch (_e_body.kind) {
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_KEXPR:
            _t_body = 0x0ull;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_KEXPR:
            _t_body = 0x1ull;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_SUBSCRIBER:
            _t_body = 0x2ull;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_SUBSCRIBER:
            _t_body = 0x3ull;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_QUERYABLE:
            _t_body = 0x4ull;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_QUERYABLE:
            _t_body = 0x5ull;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_TOKEN:
            _t_body = 0x6ull;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_TOKEN:
            _t_body = 0x7ull;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_FINAL:
            _t_body = 0x1Aull;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_DEFAULT:
            _t_body = (uint64_t)_e_body.default_tag;
            break;
        default:
            break;
    }
    size_t _s_header_n = 0;
    bool _s_header_n_there = false;
    switch (_e_body.kind) {
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_KEXPR:
            _s_header_n = (size_t)codec_zenoh_decl_kexpr_input_n(_e_body.arm.codec_zenoh_decl_kexpr.table, _e_body.arm.codec_zenoh_decl_kexpr.ctx);
            _s_header_n_there = true;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_SUBSCRIBER:
            _s_header_n = (size_t)codec_zenoh_decl_subscriber_input_n(_e_body.arm.codec_zenoh_decl_subscriber.table, _e_body.arm.codec_zenoh_decl_subscriber.ctx);
            _s_header_n_there = true;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_QUERYABLE:
            _s_header_n = (size_t)codec_zenoh_decl_queryable_input_n(_e_body.arm.codec_zenoh_decl_queryable.table, _e_body.arm.codec_zenoh_decl_queryable.ctx);
            _s_header_n_there = true;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_TOKEN:
            _s_header_n = (size_t)codec_zenoh_decl_token_input_n(_e_body.arm.codec_zenoh_decl_token.table, _e_body.arm.codec_zenoh_decl_token.ctx);
            _s_header_n_there = true;
            break;
        default:
            break;
    }
    size_t _s_header_z = 0;
    bool _s_header_z_there = false;
    switch (_e_body.kind) {
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_SUBSCRIBER:
            _s_header_z = (size_t)codec_zenoh_undecl_subscriber_input_z(_e_body.arm.codec_zenoh_undecl_subscriber.table, _e_body.arm.codec_zenoh_undecl_subscriber.ctx);
            _s_header_z_there = true;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_QUERYABLE:
            _s_header_z = (size_t)codec_zenoh_decl_queryable_input_z(_e_body.arm.codec_zenoh_decl_queryable.table, _e_body.arm.codec_zenoh_decl_queryable.ctx);
            _s_header_z_there = true;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_QUERYABLE:
            _s_header_z = (size_t)codec_zenoh_undecl_queryable_input_z(_e_body.arm.codec_zenoh_undecl_queryable.table, _e_body.arm.codec_zenoh_undecl_queryable.ctx);
            _s_header_z_there = true;
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_TOKEN:
            _s_header_z = (size_t)codec_zenoh_undecl_token_input_z(_e_body.arm.codec_zenoh_undecl_token.table, _e_body.arm.codec_zenoh_undecl_token.ctx);
            _s_header_z_there = true;
            break;
        default:
            break;
    }
    uint64_t _f_header = (uint64_t)_e_header;
    _f_header = (_f_header & ~0x1Full) | (_t_body & 0x1Full);
    if (_s_header_n_there) {
        _f_header = (_f_header & ~0x20ull) | ((uint64_t)(_s_header_n) << 5);
    }
    if (_s_header_z_there) {
        _f_header = (_f_header & ~0x80ull) | ((uint64_t)(_s_header_z) << 7);
    }
    if (overrides.header_m.present) {
        _f_header = (_f_header & ~0x40ull) | ((uint64_t)overrides.header_m.value << 6);
    }
    const uint8_t _c_header = (uint8_t)_f_header;
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, _c_header));
    switch (_e_body.kind) {
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_KEXPR:
            SCE_FORGE_TRY_WRITE(codec_zenoh_decl_kexpr_encode_source(_e_body.arm.codec_zenoh_decl_kexpr.table, _e_body.arm.codec_zenoh_decl_kexpr.ctx, w));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_KEXPR:
            SCE_FORGE_TRY_WRITE(codec_zenoh_undecl_kexpr_encode_source(_e_body.arm.codec_zenoh_undecl_kexpr.table, _e_body.arm.codec_zenoh_undecl_kexpr.ctx, w));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_SUBSCRIBER:
            SCE_FORGE_TRY_WRITE(codec_zenoh_decl_subscriber_encode_source(_e_body.arm.codec_zenoh_decl_subscriber.table, _e_body.arm.codec_zenoh_decl_subscriber.ctx, w));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_SUBSCRIBER:
            SCE_FORGE_TRY_WRITE(codec_zenoh_undecl_subscriber_encode_source(_e_body.arm.codec_zenoh_undecl_subscriber.table, _e_body.arm.codec_zenoh_undecl_subscriber.ctx, w));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_QUERYABLE:
            SCE_FORGE_TRY_WRITE(codec_zenoh_decl_queryable_encode_source(_e_body.arm.codec_zenoh_decl_queryable.table, _e_body.arm.codec_zenoh_decl_queryable.ctx, w));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_QUERYABLE:
            SCE_FORGE_TRY_WRITE(codec_zenoh_undecl_queryable_encode_source(_e_body.arm.codec_zenoh_undecl_queryable.table, _e_body.arm.codec_zenoh_undecl_queryable.ctx, w));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_TOKEN:
            SCE_FORGE_TRY_WRITE(codec_zenoh_decl_token_encode_source(_e_body.arm.codec_zenoh_decl_token.table, _e_body.arm.codec_zenoh_decl_token.ctx, w));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_TOKEN:
            SCE_FORGE_TRY_WRITE(codec_zenoh_undecl_token_encode_source(_e_body.arm.codec_zenoh_undecl_token.table, _e_body.arm.codec_zenoh_undecl_token.ctx, w));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_FINAL:
            SCE_FORGE_TRY_WRITE(codec_zenoh_decl_final_encode_source(_e_body.arm.codec_zenoh_decl_final.table, _e_body.arm.codec_zenoh_decl_final.ctx, w));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_decl_final_encode_source(_e_body.arm.default_body.table, _e_body.arm.default_body.ctx, w));
            break;
        default:
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* codec_zenoh_declaration_encode_source_overriding with no flag set in place of the
 * value's: the encode every caller but a parent that sets one calls. */
static inline sce_forge_codec_status_t codec_zenoh_declaration_encode_source(const codec_zenoh_declaration_source_t *src, const void *ctx, sce_forge_writer_t *w) {
    return codec_zenoh_declaration_encode_source_overriding(src, ctx, w, codec_zenoh_declaration_overrides_none());
}

/* Whether `self` is a value of codec_zenoh_declaration_t: SCE_FORGE_CODEC_OK, or the
 * refusal of an array past the bound of its element's view or list or of a
 * text no UTF-8 holds, in it or in a body or an entry it holds
 * (docs/adr/0081 §2). */
static inline sce_forge_codec_status_t codec_zenoh_declaration_check_value(const codec_zenoh_declaration_t *self) {
    switch (self->body.kind) {
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_KEXPR:
            SCE_FORGE_TRY_WRITE(codec_zenoh_decl_kexpr_check_value(&self->body.arm.codec_zenoh_decl_kexpr));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_KEXPR:
            SCE_FORGE_TRY_WRITE(codec_zenoh_undecl_kexpr_check_value(&self->body.arm.codec_zenoh_undecl_kexpr));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_SUBSCRIBER:
            SCE_FORGE_TRY_WRITE(codec_zenoh_decl_subscriber_check_value(&self->body.arm.codec_zenoh_decl_subscriber));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_SUBSCRIBER:
            SCE_FORGE_TRY_WRITE(codec_zenoh_undecl_subscriber_check_value(&self->body.arm.codec_zenoh_undecl_subscriber));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_QUERYABLE:
            SCE_FORGE_TRY_WRITE(codec_zenoh_decl_queryable_check_value(&self->body.arm.codec_zenoh_decl_queryable));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_QUERYABLE:
            SCE_FORGE_TRY_WRITE(codec_zenoh_undecl_queryable_check_value(&self->body.arm.codec_zenoh_undecl_queryable));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_TOKEN:
            SCE_FORGE_TRY_WRITE(codec_zenoh_decl_token_check_value(&self->body.arm.codec_zenoh_decl_token));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_UNDECL_TOKEN:
            SCE_FORGE_TRY_WRITE(codec_zenoh_undecl_token_check_value(&self->body.arm.codec_zenoh_undecl_token));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_CODEC_ZENOH_DECL_FINAL:
            SCE_FORGE_TRY_WRITE(codec_zenoh_decl_final_check_value(&self->body.arm.codec_zenoh_decl_final));
            break;
        case CODEC_ZENOH_DECLARATION_BODY_KIND_DEFAULT:
            SCE_FORGE_TRY_WRITE(codec_zenoh_decl_final_check_value(&self->body.arm.default_body));
            break;
        default:
            break;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §synth-5-B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_DECLARATION_MAX_BYTES bytes and use
 * `codec_zenoh_declaration_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. It takes the flag inputs the value does not
 * say: a codec printed from its plan reads the others off the value
 * (docs/adr/0065 §6 R4a), and any other takes every input. */
static inline sce_forge_codec_status_t codec_zenoh_declaration_encode(const codec_zenoh_declaration_t *self, sce_forge_writer_t *w) {
    /* The value within its bounds before the first byte (docs/adr/0081 §2). */
    SCE_FORGE_TRY_WRITE(codec_zenoh_declaration_check_value(self));
    return codec_zenoh_declaration_encode_source(codec_zenoh_declaration_value_source(), self, w);
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_DECLARATION_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_DECLARATION_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_declaration_encode_to_buf(const codec_zenoh_declaration_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_declaration_encode(self, &_w);
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
static inline uint8_t codec_zenoh_declaration_mid(const codec_zenoh_declaration_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x1F);
}

static inline void codec_zenoh_declaration_set_mid(codec_zenoh_declaration_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x1F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x1F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_declaration_n(const codec_zenoh_declaration_t *self) {
    return (self->header & 0x20) != 0;
}

static inline void codec_zenoh_declaration_set_n(codec_zenoh_declaration_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x20);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x20));
    }
}

static inline bool codec_zenoh_declaration_m(const codec_zenoh_declaration_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_zenoh_declaration_set_m(codec_zenoh_declaration_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_zenoh_declaration_z(const codec_zenoh_declaration_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_zenoh_declaration_set_z(codec_zenoh_declaration_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_DECLARATION_H */
