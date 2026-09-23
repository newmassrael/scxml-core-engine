// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Rust half of the forge conformance suite. Everything it verifies is
//! under `tests/`, compiled against code `build.rs` generates into
//! `OUT_DIR`; this library is empty because a package needs a target of its
//! own for those tests to belong to.
//!
//! It is a crate apart from `sce-forge-runtime` so that the generator
//! running at build time is a cost these tests pay, not one every consumer
//! of the runtime pays — see `build.rs`.
