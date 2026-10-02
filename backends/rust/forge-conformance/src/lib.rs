// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust half of the forge conformance suite. Almost everything it
//! verifies is under `tests/`, compiled against code `build.rs` generates
//! into `OUT_DIR`.
//!
//! The library holds one thing the tests share: [`queue_history`], the
//! linearizability checker that judges a queue run's observed history (SCE
//! Protocol-Synthesis RFC §synth-5-P, verification layer 2). It lives here
//! rather than in a test file because it judges histories, not this crate,
//! and a history recorded by any backend is judged by the same search.
//!
//! It is a crate apart from `sce-forge-runtime` so that the generator
//! running at build time is a cost these tests pay, not one every consumer
//! of the runtime pays — see `build.rs`.

pub mod queue_history;
