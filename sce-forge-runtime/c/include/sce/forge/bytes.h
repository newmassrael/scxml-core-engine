/* SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael */

/*
 * SCE Forge — shared byte-buffer types (C11 backend).
 *
 * Single source of truth for the two byte representations the forge
 * runtime hands to generated code:
 *
 *   - `sce_forge_bytes_t`      owned, fixed-cap stack-only buffer
 *                              (`SCE_FORGE_BYTES_DEFAULT_MAX` bytes,
 *                              copied by value). Use when a value must
 *                              be retained past the borrow that produced
 *                              it (e.g. `<send sce:payload>` slots,
 *                              service request/response payloads).
 *   - `sce_forge_bytes_view_t` borrowed, zero-copy `{data, len}` view
 *                              over a buffer the caller owns. Use for
 *                              read-only pure-function inputs — the
 *                              owned buffer would force a 256-byte copy
 *                              per call, which is unacceptable on
 *                              once-per-row hot paths such as keyexpr
 *                              matching over a bounded-collection.
 *
 * Borrowed-by-default for algorithm-kind `bytes`/`String` parameters:
 * a pure function reads its input, so it takes the view, never the
 * owned copy. The owned form is reserved for slots that genuinely
 * retain ownership.
 *
 * Memory policy: no heap, no threads, no globals. The
 * owned buffer's cap is the contract; the view borrows the caller's
 * lifetime (the view MUST NOT outlive the buffer it points at).
 */

#ifndef SCE_FORGE_BYTES_H
#define SCE_FORGE_BYTES_H

#include <stddef.h>
#include <stdint.h>

#include "sce/forge/limits.h"

/* Owned bytes container — fixed-cap stack-only buffer, copied by value. */
typedef struct {
    uint8_t data[SCE_FORGE_BYTES_DEFAULT_MAX];
    size_t len;
} sce_forge_bytes_t;

/* Borrowed bytes view — zero-copy `{data, len}` over a caller-owned
 * buffer. The `data`/`len` field names match `sce_forge_bytes_t` so a
 * `.data[i]` / `.len` access body lowers identically over either type. */
typedef struct {
    const uint8_t *data;
    size_t len;
} sce_forge_bytes_view_t;

#endif  /* SCE_FORGE_BYTES_H */
