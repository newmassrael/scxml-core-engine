// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The loom models of the bounded inbox's queue (docs/adr/0147, D5), the
//! module `loom_models` of `src/inbox/ring.rs`.
//!
//! The queue is the crate's own, so the models live beside it, and this
//! target builds that one file as a test under `--cfg loom`: there the file
//! takes loom's atomics and cells (its module `sync`), and the library, built
//! beside it without `test`, keeps `core`'s, so its constant constructors and
//! `BoundedInbox` stay what they are. The text the models explore is the text
//! that ships.
//!
//! The target has `test = false` and builds only under `--cfg loom`
//! (`backends/rust/runtime/Cargo.toml` gives the command). Built without it,
//! it refuses to compile: a model file that compiled to nothing would pass.

#[cfg(not(loom))]
compile_error!("the loom models run only under `--cfg loom`; see backends/rust/runtime/Cargo.toml");

#[cfg(loom)]
#[path = "../src/inbox/ring.rs"]
mod ring;
