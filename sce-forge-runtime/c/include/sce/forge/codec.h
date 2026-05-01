/* SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael */

/*
 * SCE Forge — codec cursor + typed error contract (C11).
 *
 * Mirrors the Rust reference at `sce-forge-runtime/rust/src/codec.rs`
 * and the C++ header at `sce-forge-runtime/cpp/include/sce/forge/codec.h`.
 * RFC §5.B L494-519 pins a per-language cursor + need-more-bytes
 * contract on decode so a truncated input never aborts — the caller
 * resumes after additional bytes arrive (DMA boundary, fragmented
 * network read).
 *
 * Phase B1-prep: minimum API for fixed-width codec fixtures
 * (sce_forge_cursor_peek + advance + remaining). Streaming readers
 * (read_u8, read_vle_*, read_tag) land in B1-α/β with their first
 * consumer. Encode-side cursor + SCE_FORGE_CODEC_BUFFER_OVERFLOW lands
 * in B1-α.
 */

#ifndef SCE_FORGE_CODEC_H
#define SCE_FORGE_CODEC_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Typed decode error. NeedMoreBytes is the only reachable variant
 * while every codec field is fixed-width. B1-α adds VleWidthOverflow,
 * B1-β adds UnknownVariantTag. */
typedef enum {
    SCE_FORGE_CODEC_OK = 0,
    SCE_FORGE_CODEC_NEED_MORE_BYTES = 1,
} sce_forge_codec_status_t;

/* Read-only cursor over a borrowed input buffer. Decode bodies bind a
 * cursor at the call site, peek the minimum frame, then advance after
 * the construction succeeds.
 *
 * Fields are exposed by name (not opaque) so the generated decode
 * bodies can `cursor->data + cursor->pos` for positional reads at
 * fixed offsets without an accessor call per field. */
typedef struct {
    const uint8_t *data;
    size_t         len;
    size_t         pos;
} sce_forge_cursor_t;

static inline sce_forge_cursor_t sce_forge_cursor_init(const uint8_t *data, size_t len) {
    sce_forge_cursor_t c;
    c.data = data;
    c.len = len;
    c.pos = 0;
    return c;
}

static inline size_t sce_forge_cursor_remaining(const sce_forge_cursor_t *c) {
    return c->len - c->pos;
}

/* Peek the next `n` bytes without advancing. Returns NULL when the
 * cursor's tail is shorter than `n`. */
static inline const uint8_t *sce_forge_cursor_peek(const sce_forge_cursor_t *c, size_t n) {
    if (sce_forge_cursor_remaining(c) < n) return NULL;
    return c->data + c->pos;
}

/* Advance the cursor by `n` bytes. Returns false if `n` would
 * overrun the buffer. */
static inline bool sce_forge_cursor_advance(sce_forge_cursor_t *c, size_t n) {
    if (sce_forge_cursor_remaining(c) < n) return false;
    c->pos += n;
    return true;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_FORGE_CODEC_H */
