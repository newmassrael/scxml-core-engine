// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package conformance is the Go half of the cross-language SCE Forge
// numerical conformance harness. The actual test functions live in
// numerical_conformance_test.go, which is generated on demand by
// sce-codegen from the shared fixture catalog at
// tests/forge/conformance/fixtures.json and the per-kind Jinja2
// fragments under tools/codegen/templates/forge/go/conformance/.
//
// Workflow:
//
//	cargo build --bin sce-codegen --features cli --release -p sce-build
//	go generate ./sce-forge-runtime/go/conformance/...
//	go test    ./sce-forge-runtime/go/conformance/...
//
// CI must run `go generate` (or invoke generate.sh directly) before
// `go test` because Go discovers test functions at compile time and
// numerical_conformance_test.go is gitignored — committing it would
// allow drift between fixtures.json and the harness.
package conformance

//go:generate ./generate.sh
