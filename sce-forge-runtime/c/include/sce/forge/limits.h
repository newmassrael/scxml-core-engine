/* SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael */

/*
 * SCE Forge — runtime cap constants for bytes-typed slots.
 *
 * Single source of truth for the default `bytes` slot capacity, applied
 * by every backend (cpp / rust / kotlin / go / python / c11) when the
 * SCXML author has not declared a per-slot `sce:max-size` annotation.
 *
 * The Rust mirror lives at `sce-build/src/forge/limits.rs`. A lockstep
 * test in that crate asserts the two constants stay in sync.
 *
 * Rationale (RFC `claudedocs/rfc-forge-bytes-bounded.md` §3 B2): 256
 * bytes covers the UDS / OBD-II / automotive diagnostic single-frame
 * payload max and the watching-zenoh MCU consumer's stack budget without
 * bloat. A fixture needing more (or less) declares its own
 * `sce:max-size` per slot.
 */

#ifndef SCE_FORGE_LIMITS_H
#define SCE_FORGE_LIMITS_H

#define SCE_FORGE_BYTES_DEFAULT_MAX 256u

#endif  /* SCE_FORGE_LIMITS_H */
