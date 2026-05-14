// SCE-MAP: codec_tlv_chain_present_if_basic:37

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec") */
/* Runtime: none */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CODEC_TLV_CHAIN_PRESENT_IF_BASIC_H
#define SCE_FORGE_CODEC_TLV_CHAIN_PRESENT_IF_BASIC_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>

#include "sce/forge/codec.h"
#include "codec_tlv_entry.h"

#define CODEC_TLV_CHAIN_PRESENT_IF_BASIC_MIN_BYTES 1
#define CODEC_TLV_CHAIN_PRESENT_IF_BASIC_MAX_BYTES 137

typedef struct {
    uint8_t carrier;
    /* RFC §5.B B3 tlv-chain: fixed array of codec_tlv_entry_t entries (max-depth 4, on-overflow=reject) */
    codec_tlv_entry_t entries[4];
    size_t  entries_len;
} codec_tlv_chain_present_if_basic_t;

typedef struct {
    uint8_t bytes[CODEC_TLV_CHAIN_PRESENT_IF_BASIC_MAX_BYTES];
    size_t  len;
} codec_tlv_chain_present_if_basic_encoded_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_tlv_chain_present_if_basic_decode(sce_forge_cursor_t *cursor, codec_tlv_chain_present_if_basic_t *out) {
    /* RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
     * advances the cursor per field. C11 has no nullable wrapper so
     * the gated field's storage stays as plain `T` (with `_len = 0`
     * for absent bytes payloads); the carrier's flag bit is the
     * source of truth for presence. B2-β extends gating to Tail /
     * LengthRef / Vle bit-sizes via dispatch inside the helper.
     * Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified streaming path. */
    {
        const uint8_t *raw = sce_forge_cursor_peek(cursor, 1);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        out->carrier = raw[0];
        if (!sce_forge_cursor_advance(cursor, 1)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    }
    out->entries_len = 0;
        if ((out->carrier & 0x01) != 0) {
            for (size_t _i = 0; _i < 4; ++_i) {
                if (sce_forge_cursor_remaining(cursor) == 0) break;
                sce_forge_codec_status_t _st = codec_tlv_entry_decode(cursor, &out->entries[out->entries_len]);
                if (_st != SCE_FORGE_CODEC_OK) return _st;
                out->entries_len++;
            }
            if (sce_forge_cursor_remaining(cursor) > 0) return SCE_FORGE_CODEC_TLV_CHAIN_OVERFLOW;
        }
    return SCE_FORGE_CODEC_OK;
}

static inline codec_tlv_chain_present_if_basic_encoded_t codec_tlv_chain_present_if_basic_encode(const codec_tlv_chain_present_if_basic_t *self) {
    codec_tlv_chain_present_if_basic_encoded_t r;
    /* RFC §5.B B1-δ + B2-β present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    r.len = 0;
    r.bytes[r.len++] = self->carrier;
    if ((self->carrier & 0x01) != 0) {
        for (size_t _ti = 0; _ti < self->entries_len; ++_ti) {
            codec_tlv_entry_encoded_t _sub = codec_tlv_entry_encode(&self->entries[_ti]);
            if (r.len + _sub.len <= sizeof(r.bytes)) {
                for (size_t _tj = 0; _tj < _sub.len; ++_tj) r.bytes[r.len + _tj] = _sub.bytes[_tj];
                r.len += _sub.len;
            }
        }
    }
    return r;
}

/* RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
 * the carrier field. Single-bit (width=1) reads as bool; multi-bit
 * (width>=2) reads as the smallest unsigned C11 integer type that fits
 * (uint8_t / uint16_t / uint32_t / uint64_t). Setters mask + shift on
 * the way in so out-of-range callers can't corrupt sibling bits. The
 * accessor name is `<struct_snake>_<flag_name>` so multiple codecs
 * carrying same-named flags coexist in a single translation unit. Wire
 * layout is unchanged — the carrier still occupies its declared bytes. */
static inline bool codec_tlv_chain_present_if_basic_has_chain(const codec_tlv_chain_present_if_basic_t *self) {
    return (self->carrier & 0x01) != 0;
}

static inline void codec_tlv_chain_present_if_basic_set_has_chain(codec_tlv_chain_present_if_basic_t *self, bool v) {
    if (v) {
        self->carrier = (uint8_t)(self->carrier | 0x01);
    } else {
        self->carrier = (uint8_t)(self->carrier & (uint8_t)(~(uint8_t)0x01));
    }
}

#endif  /* SCE_FORGE_CODEC_TLV_CHAIN_PRESENT_IF_BASIC_H */
