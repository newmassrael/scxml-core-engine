// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! # SCE Rust Tests — W3C SCXML 1.0 conformance test harness
//!
//! Contains:
//! - [`harness`]: generic test runner (`SimpleAotTest` trait, `run_simple_aot_test`
//!   function, `linkme`-based registrar) modeled on the C++
//!   `SimpleAotTest<Derived, Num>` pattern documented in `CLAUDE.md`.
//! - [`generated`]: the W3C state machines, the tree `rust-w3c`
//!   (`tools/codegen/trees/rust-w3c.toml`), which `build.rs` writes into
//!   `OUT_DIR` and declares in `generated.rs` there (docs/adr/0097).
//! - [`integration`]: hand-curated non-W3C-IRP fixtures, the trees
//!   `rust-integration` and `rust-static-datamodel`, written and declared
//!   the same way. Its `mod.rs` is hand-authored.
//! - [`transition_function_equivalence`]: the drive that holds a generated
//!   machine to the transition function generated beside it, which the
//!   drivers of the stems that declare one (`docs/SCE_TRANSITION_FUNCTION.md`)
//!   share.
//!
//! ## Running
//!
//! ```bash
//! cargo test -p sce-rust-tests --release
//! ```
//!
//! The W3C tests are one target, `tests/w3c.rs`, a module per test the
//! generator writes beside its machine: `cargo test -p sce-rust-tests --test
//! w3c test_144::` runs one.

pub mod harness;
pub mod integration;
pub mod transition_function_equivalence;

// The module `generated`, from the file the tree placed in OUT_DIR: `#[path]`
// cannot take `env!("OUT_DIR")`, so the build script writes the declaration.
include!(concat!(env!("OUT_DIR"), "/generated.rs"));
