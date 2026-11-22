// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The generated machines the server process serves, the tree `rust-server-suite`
//! (docs/adr/0097), mounted as the generator wrote them: the build script writes the tree into
//! OUT_DIR and the declaration of its module beside it, so a change to a document or a template
//! reaches this crate with no step of its own.

// The machines are the generator's, and the suite calls only some of what each one offers.
#![allow(clippy::all, dead_code)]

// `#[path]` cannot take `env!("OUT_DIR")`, so the build script writes the declaration of
// `generated`, read from its `mod.rs` there.
include!(concat!(env!("OUT_DIR"), "/generated.rs"));
