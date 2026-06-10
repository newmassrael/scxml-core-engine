// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Package integration anchors SCE Go integration-fixture state
// machines.
//
// Every `<stem>/*_sm.go` and `<stem>/*_test.go` under
// `sce-go-tests/integration/<stem>/` is regenerated from a canonical
// fixture at `integration_resources/<stem>/<stem>.scxml` by
// `scripts/regen_donedata_local_invoke{,_kotlin,_go}.sh`. The
// `integration/` parent dir is the Go counterpart to Rust's
// `sce-rust-tests/src/integration/` and Kotlin's
// `com.sce.integration` package — its sibling
// `sce-go-tests/generated/` is reserved for the W3C IRP harness so
// the W3C and integration trees stay disjoint at the directory
// level. See `docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md` for the
// per-backend layout convention and the architectural-axis
// (committed-tree vs build-time, Interpreter vs AOT) rationale.
//
// This file is intentionally not §6.2.6 drift-gated: it is the only
// hand-authored file under `sce-go-tests/integration/` and has no
// `SCE-GENERATED` header, mirroring `sce-rust-tests/src/integration/mod.rs`
// and `sce-kotlin-tests/src/main/kotlin/com/sce/integration/package-info.kt`.
package integration
