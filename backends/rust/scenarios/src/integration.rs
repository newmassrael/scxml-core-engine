// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The `datamodel="sce-static"` machines, the tree `rust-static-datamodel`
//! (docs/adr/0097), mounted under the path `sce-rust-tests` gives them and read
//! by the compiler as the generator wrote them: the build script writes the
//! tree into OUT_DIR and the declaration of its module beside it, so a change
//! to a fixture or a template reaches this crate with no step of its own.

// The machines are the generator's, and the replay calls only some of what
// each one offers.
#![allow(clippy::all, dead_code)]

// `#[path]` cannot take `env!("OUT_DIR")`, so the build script writes the
// declaration of `static_datamodel` and of `static_datamodel_large` (the
// signing and sealing machine built with its records' bounds moved,
// docs/adr/0109, D17), each read from its `mod.rs` there.
include!(concat!(env!("OUT_DIR"), "/static_datamodel.rs"));
