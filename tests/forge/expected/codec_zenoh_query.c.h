// SCE-MAP: codec_zenoh_query:51

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_ZENOH_QUERY_H
#define SCE_FORGE_CODEC_ZENOH_QUERY_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_zenoh_ext_entry.h"

#define CODEC_ZENOH_QUERY_MIN_BYTES 2
#define CODEC_ZENOH_QUERY_MAX_BYTES 612

typedef struct {
    uint8_t header;
    uint8_t consolidation;
    uint64_t parameters_len;
    /* variable-length payload (sce:bit-size="length-ref", sce:max-size="256") */
    uint8_t parameters[256];
    /* RFC §5.B B3 tlv-chain: fixed array of codec_zenoh_ext_entry_t entries (max-depth 8, on-overflow=reject) */
    codec_zenoh_ext_entry_t extensions[8];
    size_t  extensions_len;
} codec_zenoh_query_t;

/* RFC variant-default-uniformity (C11): designated-initializer
 * macro carrying the codec's wire-MID-baked defaults. C has no Default
 * trait — round-trip safety (`codec_zenoh_query_t x = CODEC_ZENOH_QUERY_DEFAULT_INIT;
 * codec_zenoh_query_t_encode(&x)` decodes back to the same arm)
 * requires using this macro rather than the zero-initializer `{0}`,
 * which would leave the dispatch tag at zero and (for variant codecs)
 * land in the catch-all arm or a mismatched union slot. Unspecified
 * fields zero-initialize per C11 §6.7.9 ¶21, so the macro names only
 * the wire-MID-bearing members. */
#define CODEC_ZENOH_QUERY_DEFAULT_INIT { \
    .header = 0x03u, \
}

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_query_decode(sce_forge_cursor_t *cursor, codec_zenoh_query_t *out) {
    /* RFC §5.B present-if primitive: streaming decode
     * advances the cursor per field. C11 has no nullable wrapper so
     * the gated field's storage stays as plain `T` (with `_len = 0`
     * for absent bytes payloads); the carrier's flag bit is the
     * source of truth for presence. Gating extends to Tail /
     * LengthRef / Vle bit-sizes via dispatch inside the helper.
     * Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified streaming path. */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->header = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    if ((out->header & 0x20) != 0) {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->consolidation = (uint8_t)(raw[0]);
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->consolidation = 0;
    }
    if ((out->header & 0x40) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        out->parameters_len = _v;
    } else {
        out->parameters_len = 0;
    }
    if ((out->header & 0x40) != 0) {
        size_t _n = (size_t)out->parameters_len;
        if (_n > 256) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        memcpy(out->parameters, raw, _n);
        out->parameters_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->parameters_len = 0;
    }
    out->extensions_len = 0;
        if ((out->header & 0x80) != 0) {
            for (size_t _i = 0; _i < 8; ++_i) {
                if (sce_forge_cursor_remaining(cursor) == 0) break;
                sce_forge_codec_status_t _st = codec_zenoh_ext_entry_decode(cursor, &out->extensions[out->extensions_len]);
                if (_st != SCE_FORGE_CODEC_OK) return _st;
                out->extensions_len++;
            }
            if (sce_forge_cursor_remaining(cursor) > 0) return SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW;
        }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §5.B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_QUERY_MAX_BYTES bytes and use
 * `codec_zenoh_query_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_query_encode(const codec_zenoh_query_t *self, sce_forge_writer_t *w) {
    /* RFC §5.B present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->header));
    if ((self->header & 0x20) != 0) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, self->consolidation));
    }
    if ((self->header & 0x40) != 0) {
    {
        uint64_t _vle = (uint64_t)(self->parameters_len);
        while (_vle >= 0x80u) {
            SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_vle & 0x7Fu) | 0x80u)));
            _vle >>= 7;
        }
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)_vle));
    }
    }
    if ((self->header & 0x40) != 0) {
        size_t _n = self->parameters_len;
        if (_n > self->parameters_len) _n = self->parameters_len;
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, self->parameters, _n));
    }
    if ((self->header & 0x80) != 0) {
        for (size_t _ti = 0; _ti < self->extensions_len; ++_ti) {
            SCE_FORGE_TRY_WRITE(codec_zenoh_ext_entry_encode(&self->extensions[_ti], w));
        }
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_QUERY_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_QUERY_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_query_encode_to_buf(const codec_zenoh_query_t *self, uint8_t *buf, size_t cap, size_t *out_len) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_query_encode(self, &_w);
    *out_len = _w.pos;
    return _st;
}

/* RFC §5.B flags primitive: per-bit-range accessors over
 * the carrier field. Single-bit (width=1) reads as bool; multi-bit
 * (width>=2) reads as the smallest unsigned C11 integer type that fits
 * (uint8_t / uint16_t / uint32_t / uint64_t). Setters mask + shift on
 * the way in so out-of-range callers can't corrupt sibling bits. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline uint8_t codec_zenoh_query_mid(const codec_zenoh_query_t *self) {
    return (uint8_t)((self->header >> 0) & (uint8_t)0x1F);
}

static inline void codec_zenoh_query_set_mid(codec_zenoh_query_t *self, uint8_t v) {
    const uint8_t _shifted_mask = (uint8_t)((uint8_t)0x1F << 0);
    const uint8_t _val = (uint8_t)(((uint8_t)v & (uint8_t)0x1F) << 0);
    self->header = (uint8_t)((self->header & (uint8_t)~_shifted_mask) | _val);
}


static inline bool codec_zenoh_query_c(const codec_zenoh_query_t *self) {
    return (self->header & 0x20) != 0;
}

static inline void codec_zenoh_query_set_c(codec_zenoh_query_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x20);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x20));
    }
}

static inline bool codec_zenoh_query_p(const codec_zenoh_query_t *self) {
    return (self->header & 0x40) != 0;
}

static inline void codec_zenoh_query_set_p(codec_zenoh_query_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x40);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x40));
    }
}

static inline bool codec_zenoh_query_z(const codec_zenoh_query_t *self) {
    return (self->header & 0x80) != 0;
}

static inline void codec_zenoh_query_set_z(codec_zenoh_query_t *self, bool v) {
    if (v) {
        self->header = (uint8_t)(self->header | 0x80);
    } else {
        self->header = (uint8_t)(self->header & (uint8_t)(~(uint8_t)0x80));
    }
}

#endif  /* SCE_FORGE_CODEC_ZENOH_QUERY_H */
