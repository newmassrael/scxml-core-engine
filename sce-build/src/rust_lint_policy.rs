// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The lints a consumer that ships generated Rust on a path where a panic is a
//! stopped device denies on the crate that holds the file, outside
//! `#[cfg(test)]`: no index or slice, no arithmetic operator, and no
//! `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!` or
//! `unimplemented!`.
//!
//! The generated Rust of an algorithm (SCE_FORGE.md §3.4.1) and of a buffer
//! pool is held to them by tests that run `clippy-driver` over it with each one
//! denied, so a consumer compiles the file as it is, with no `allow` of its own.
//! What is left of a failure is the runtime's: an algorithm's operations the
//! interval analysis proved are printed through `sce_forge_runtime::algorithm::proven`,
//! and a pool reaches its tables through an index type with no value out of
//! range.

/// The denied lints, as `clippy-driver -D` takes them.
pub const PANIC_FREE_LINTS: [&str; 8] = [
    "clippy::indexing_slicing",
    "clippy::unwrap_used",
    "clippy::expect_used",
    "clippy::panic",
    "clippy::unreachable",
    "clippy::todo",
    "clippy::unimplemented",
    "clippy::arithmetic_side_effects",
];
