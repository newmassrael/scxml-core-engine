// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The W3C SCXML 1.0 conformance suite in Rust, one target: each test the
//! tree `rust-w3c` writes beside its machine (`tests/test_<n>.rs` under the
//! root, in OUT_DIR) is a module of it, so `cargo test -p sce-rust-tests
//! --test w3c test_144::` runs one (docs/adr/0097, section 4).

// The modules, from the files the tree placed in OUT_DIR: `#[path]` cannot
// take `env!("OUT_DIR")`, so the build script writes the declarations.
include!(concat!(env!("OUT_DIR"), "/w3c.rs"));
