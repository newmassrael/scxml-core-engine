// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * SCE Kotlin integration fixture state machines (RFC
 * `claudedocs/rfc-donedata-5-backend-layout.md` Q-1).
 *
 * Every `*Sm.kt` under `com.sce.integration.<stem>` is regenerated from
 * a canonical fixture at `integration_resources/<stem>/<stem>.scxml`
 * by `scripts/regen_donedata_local_invoke{,_kotlin,_go}.sh`. The
 * `com.sce.integration` package is the Kotlin counterpart to Rust's
 * `sce-rust-tests/src/integration/` and Go's `sce-go-tests/integration/`
 * — its sibling `com.sce.generated` is reserved for the W3C IRP
 * harness so the W3C and integration trees stay disjoint at the
 * package level. See `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md` for the
 * per-backend layout convention and the architectural-axis
 * (committed-tree vs build-time, Interpreter vs AOT) rationale.
 *
 * This file is intentionally not §6.2.6 drift-gated: it is the only
 * hand-authored file under `com.sce.integration` and has no
 * `SCE-GENERATED` header, mirroring `sce-rust-tests/src/integration/mod.rs`.
 */
package com.sce.integration
