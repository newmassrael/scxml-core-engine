// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * SCE Kotlin integration fixture state machines.
 *
 * Every `*Sm.kt` under `com.sce.integration.<stem>` is generated from
 * a canonical fixture at `integration_resources/<stem>/<stem>.scxml`,
 * or from a document `tools/codegen/trees/kotlin-integration.toml` names
 * elsewhere, as that manifest declares (`scripts/materialize.sh
 * kotlin-integration`). The
 * `com.sce.integration` package is the Kotlin counterpart to Rust's
 * `backends/rust/tests/src/integration/` and Go's `backends/go/tests/integration/`
 * — its sibling `com.sce.generated` is reserved for the W3C IRP
 * harness so the W3C and integration trees stay disjoint at the
 * package level. See `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md` for the
 * per-backend layout convention and the architectural-axis
 * (committed-tree vs build-time, Interpreter vs AOT) rationale.
 *
 * The `datamodel="sce-static"` machines are a second generated tree and
 * have a package of their own, `com.sce.integration.static_datamodel.<machine>`,
 * written by `scripts/regen_static_datamodel_kotlin.sh` from
 * `sce-build/tests/fixtures/static_datamodel/`, as Rust's
 * `integration::static_datamodel` and Go's `integration/static_datamodel`
 * are: two trees in one directory could not each own it.
 *
 * This file is intentionally not §6.2.6 drift-gated: it is the only
 * hand-authored file under `com.sce.integration` and has no
 * `SCE-GENERATED` header, mirroring `backends/rust/tests/src/integration/mod.rs`.
 */
package com.sce.integration
