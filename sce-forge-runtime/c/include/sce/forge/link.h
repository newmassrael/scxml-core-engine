/* SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael */

/*
 * SCE Forge — byte-stream link contract (C11).
 *
 * Mirrors the Rust trait surface at `sce-link-runtime/src/lib.rs`
 * (watching-zenoh RFC §5.C, B6-α). For C11 the polymorphic surface
 * uses the canonical Linux-kernel "trait in C" pattern: a per-instance
 * `sce_forge_link_t` carrying a pointer to a shared `const
 * sce_forge_link_ops_t` vtable plus the driver's per-instance state.
 * The const ops table can live in flash/ROM on MCU targets — only
 * the `void *self` and ops pointer cost RAM per instance.
 *
 * B6-β scope is the contract surface only (this header) plus a
 * generated wrapper struct per `<scxml sce:kind="link">` declaration.
 * Real per-platform impls (`sce_link_runtime_lwip` /
 * `sce_link_runtime_tokio` / QNX) live downstream in watching-zenoh
 * and supply concrete `sce_forge_link_ops_t` tables.
 *
 * Borrowed-slice lifetime (RFC §5.C / B6-α Q2.5=2.5a):
 *   The `data` pointer in `sce_forge_link_rx_frame_t` is owned by
 *   the impl and remains valid until the next call to `ops->rx`
 *   on the same instance. Callers must consume the bytes before
 *   the next poll. B7's pool-kind impl re-uses this contract by
 *   backing the slice with a slot reference instead of a `Vec`.
 */

#ifndef SCE_FORGE_LINK_H
#define SCE_FORGE_LINK_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* TX submission outcome. Mirrors the Rust `Result<(), LinkError>`
 * variants — `OK` is the success case, the others are the
 * `LinkError::Driver` / `LinkError::Backpressure` siblings. */
typedef enum {
    SCE_FORGE_LINK_OK = 0,
    /* Driver-level send failure (socket closed, ENOBUFS, ECONNRESET,
     * or platform-specific equivalents). */
    SCE_FORGE_LINK_ERR_DRIVER = 1,
    /* Outbound buffer is full and the link's `<sce:backpressure>`
     * policy is `block` or `drop` — the caller must retry or the
     * frame is dropped per policy. */
    SCE_FORGE_LINK_ERR_BACKPRESSURE = 2,
} sce_forge_link_status_t;

/* Borrowed-slice frame view returned by `ops->rx`. The `data`
 * pointer is owned by the impl — see the lifetime contract in the
 * file header. */
typedef struct {
    const uint8_t *data;
    size_t         len;
} sce_forge_link_rx_frame_t;

/* Borrowed-slice frame queued through `ops->tx`. Same lifetime
 * contract as RX: the slice must outlive the call. */
typedef struct {
    const uint8_t *data;
    size_t         len;
} sce_forge_link_tx_frame_t;

/* Pull the next decoded frame, if any is available without blocking.
 * Returns `true` and populates `*out` when a frame is ready; returns
 * `false` and leaves `*out` untouched when the underlying driver has
 * no pending bytes. The SCE-generated link wrapper polls in the
 * SCXML interpreter's idle slot. */
typedef bool (*sce_forge_link_rx_fn)(void *self, sce_forge_link_rx_frame_t *out);

/* Submit a frame for transmission. The implementation must honor
 * the link's `<sce:backpressure>` policy — `drop` returns
 * `SCE_FORGE_LINK_ERR_BACKPRESSURE` (the SCE-side wrapper ignores
 * it), `block` blocks until the driver accepts the frame. */
typedef sce_forge_link_status_t (*sce_forge_link_tx_fn)(void *self, sce_forge_link_tx_frame_t frame);

/* Shared, const-qualifiable vtable. Per RFC §5.J.1 + the watching-
 * zenoh MCU consumer, this lives in `.rodata` (flash/ROM on MCU)
 * so per-instance RAM cost is just `sizeof(void *) + sizeof(void *)`. */
typedef struct {
    sce_forge_link_rx_fn rx;
    sce_forge_link_tx_fn tx;
} sce_forge_link_ops_t;

/* Per-instance link handle. `ops` is the shared driver vtable;
 * `self` is the per-instance state (downstream `sce_link_runtime_<os>`
 * driver context). The generated `<snake>_link_t` wrapper composes
 * one of these by value. */
typedef struct {
    const sce_forge_link_ops_t *ops;
    void                       *self;
} sce_forge_link_t;

#ifdef __cplusplus
}  /* extern "C" */
#endif

#endif /* SCE_FORGE_LINK_H */
