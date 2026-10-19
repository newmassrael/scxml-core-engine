// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust artifacts SCE writes from a deploy, compiled. `build.rs` runs the
//! generator and writes one module per artifact into `OUT_DIR`; this file only
//! includes them, so the tests under `tests/` call the code a deploy yields
//! and not a copy of it.
//!
//! It is a crate apart so that the generator running at build time is a cost
//! these tests pay, not one every consumer of the link runtime pays.

include!(concat!(env!("OUT_DIR"), "/deploy_artifacts.rs"));
