// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Cross-language numerical conformance harness entry point (Rust half).
//
// The actual test functions are generated at build time by build.rs from
// tools/codegen/templates/forge/rust/conformance/harness.rs.jinja2 and the
// fixture catalog at tests/forge/conformance/fixtures.json. This file is a
// one-line shim so cargo's test discovery picks up the generated code.
//
// To inspect the generated source: `cargo build -p sce-forge-runtime --tests`
// then read target/debug/build/sce-forge-runtime-*/out/numerical_conformance.rs.

include!(concat!(env!("OUT_DIR"), "/numerical_conformance.rs"));
