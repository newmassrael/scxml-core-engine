// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust half of the forge conformance suite. Almost everything it
//! verifies is under `tests/`, compiled against code `build.rs` generates
//! into `OUT_DIR`.
//!
//! The library itself holds nothing. The linearizability checker that judges
//! a queue run's observed history (SCE Protocol-Synthesis RFC §synth-5-P,
//! verification layer 2) is `sce_build::queue_history`, where
//! `sce-codegen check-queue-history` reaches it for the stress run of every
//! backend; the tests here use it from there.
//!
//! It is a crate apart from `sce-forge-runtime` so that the generator
//! running at build time is a cost these tests pay, not one every consumer
//! of the runtime pays — see `build.rs`.
