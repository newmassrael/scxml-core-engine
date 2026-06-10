// SCE-MAP: codec_zenoh_wireexpr:53

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

#define CODEC_ZENOH_WIREEXPR_MIN_BYTES 0
#define CODEC_ZENOH_WIREEXPR_MAX_BYTES 148

typedef struct {
    uint64_t id;
    uint64_t suffix_len;
    /* RFC §5.B sce:type="string" payload (sce:max-size="128").
     * `char[N] + size_t len` parallels the bytes pair (uint8_t[N] + len)
     * but the host-language type signals UTF-8 text storage; the C
     * string is NOT NUL-terminated — payloads of exactly `max_size`
     * bytes are valid wire input. */
    char    suffix[128];
} codec_zenoh_wireexpr_t;

/* Decode the next frame from `cursor`. Returns SCE_FORGE_CODEC_OK on
 * success and advances `cursor`; returns SCE_FORGE_CODEC_NEED_MORE_BYTES
 * (without advancing) when the cursor's tail is shorter than the
 * declared minimum frame (RFC §5.B L494-519). VLE codecs may also
 * return SCE_FORGE_CODEC_VLE_WIDTH_OVERFLOW. */
static inline sce_forge_codec_status_t codec_zenoh_wireexpr_decode(sce_forge_cursor_t *cursor, codec_zenoh_wireexpr_t *out, uint8_t n) {
    /* Declared-but-unconsumed flag inputs: defensive (void) suppress per declared
     * `<sce:flag-input>` so codecs that haven't consumed an input via
     * `present-if` yet compile cleanly under -Wunused-parameter. */
    (void)n;
    /* RFC §5.B present-if primitive: streaming decode
     * advances the cursor per field. C11 has no nullable wrapper so
     * the gated field's storage stays as plain `T` (with `_len = 0`
     * for absent bytes payloads); the carrier's flag bit is the
     * source of truth for presence. Gating extends to Tail /
     * LengthRef / Vle bit-sizes via dispatch inside the helper.
     * Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified streaming path. */
    uint64_t id;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &id);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
    out->id = id;
    if ((n & 0x01) != 0) {
        uint64_t _v;
    {
        sce_forge_codec_status_t _vle_st = sce_forge_cursor_read_vle_u64(cursor, &_v);
        if (_vle_st != SCE_FORGE_CODEC_OK) return _vle_st;
    }
        out->suffix_len = _v;
    } else {
        out->suffix_len = 0;
    }
    if ((n & 0x01) != 0) {
        size_t _n = (size_t)out->suffix_len;
        if (_n > 128) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        const uint8_t *raw = sce_forge_cursor_peek(cursor, _n);
        if (raw == NULL) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
        if (!sce_forge_is_valid_utf8(raw, _n)) return SCE_FORGE_CODEC_INVALID_UTF8;
        memcpy(out->suffix, raw, _n);
        out->suffix_len = _n;
        if (!sce_forge_cursor_advance(cursor, _n)) return SCE_FORGE_CODEC_NEED_MORE_BYTES;
    } else {
        out->suffix_len = 0;
    }
    return SCE_FORGE_CODEC_OK;
}

/* RFC §5.B encode-side primary: write `*self` into the caller-
 * owned `*w` writer. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when the writer ran out of capacity.
 * Callers either pre-reserve CODEC_ZENOH_WIREEXPR_MAX_BYTES bytes and use
 * `codec_zenoh_wireexpr_encode_to_buf` (below), or run the writer themselves
 * for coalesced-send paths. */
static inline sce_forge_codec_status_t codec_zenoh_wireexpr_encode(const codec_zenoh_wireexpr_t *self, sce_forge_writer_t *w, uint8_t n) {
    /* Declared-but-unconsumed flag inputs: see decode — same suppress per input. */
    (void)n;
    /* RFC §5.B present-if encode: per-field byte append.
     * Gated fields skip the append when the carrier's flag bit is
     * clear. Per-field `is_repeat` / `is_tlv_chain` route to dedicated
     * helpers. Branch fires before has_vle_fields so a codec mixing
     * VLE + present-if uses the unified encode path. */
    {
        uint64_t _vle = (uint64_t)(self->id);
        while (_vle >= 0x80u) {
            SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_vle & 0x7Fu) | 0x80u)));
            _vle >>= 7;
        }
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)_vle));
    }
    if ((n & 0x01) != 0) {
    {
        uint64_t _vle = (uint64_t)(self->suffix_len);
        while (_vle >= 0x80u) {
            SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)((_vle & 0x7Fu) | 0x80u)));
            _vle >>= 7;
        }
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_u8(w, (uint8_t)_vle));
    }
    }
    if ((n & 0x01) != 0) {
        SCE_FORGE_TRY_WRITE(sce_forge_writer_write_bytes(w, (const uint8_t*)self->suffix, self->suffix_len));
    }
    return SCE_FORGE_CODEC_OK;
}

/* Heap-free convenience facade: wrap the caller-owned `buf` + `cap`
 * in a writer, run the primary encode, and report the resulting byte
 * count via `*out_len`. Returns SCE_FORGE_CODEC_OK on success;
 * SCE_FORGE_CODEC_BUFFER_OVERFLOW when `cap < CODEC_ZENOH_WIREEXPR_MAX_BYTES`
 * was insufficient for this codec's wire bytes. Worst-case bound is
 * `CODEC_ZENOH_WIREEXPR_MAX_BYTES` — callers sizing `buf` accordingly never
 * see overflow. */
static inline sce_forge_codec_status_t codec_zenoh_wireexpr_encode_to_buf(const codec_zenoh_wireexpr_t *self, uint8_t *buf, size_t cap, size_t *out_len, uint8_t n) {
    sce_forge_writer_t _w = sce_forge_writer_init_buf(buf, cap);
    sce_forge_codec_status_t _st = codec_zenoh_wireexpr_encode(self, &_w, n);
    *out_len = _w.pos;
    return _st;
}

#endif  /* SCE_FORGE_CODEC_ZENOH_WIREEXPR_H */
