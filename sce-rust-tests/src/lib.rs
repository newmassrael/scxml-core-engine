// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! # SCE Rust Tests — W3C SCXML 1.0 conformance test harness
//!
//! Contains:
//! - [`harness`]: generic test runner (`SimpleAotTest` trait, `run_simple_aot_test`
//!   function, `linkme`-based registrar) modeled on the C++
//!   `SimpleAotTest<Derived, Num>` pattern documented in `CLAUDE.md`.
//! - [`generated`]: per-test generated state machines produced by
//!   `sce-codegen generate -l rust` (via `build.rs`). Fully overwritten
//!   on every `sce-codegen generate-w3c -l rust` run.
//! - [`integration`]: hand-curated non-W3C-IRP fixtures regenerated via
//!   per-fixture scripts (e.g. `scripts/regen_donedata_local_invoke.sh`).
//!   The `mod.rs` is hand-authored so the W3C regen pipeline never
//!   touches it.
//!
//! ## Running
//!
//! ```bash
//! cargo test -p sce-rust-tests --release
//! ```
//!
//! Each individual W3C test lives as `tests/test_XXX.rs` integration test,
//! which the generator produces in Phase 2+. Rust's `#[test]` model compiles
//! each file as a separate binary, preserving per-test rebuild granularity.

pub mod generated;
pub mod harness;
pub mod integration;
