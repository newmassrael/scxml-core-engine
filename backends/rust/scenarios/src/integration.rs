// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The committed `datamodel="sce-static"` machines, mounted from where they are
//! committed (`backends/rust/tests/src/integration/static_datamodel/`) under the
//! path `sce-rust-tests` gives them, and read by the compiler as they are:
//! nothing is copied and nothing is rewritten, so a regeneration
//! (`scripts/regen_static_datamodel_rust.sh`) reaches this crate with no step of
//! its own.

// The path is read from this file's directory (`src/`). In an inline module it
// would be read from the directory the module nests to, which does not exist.
#[allow(clippy::all, dead_code)]
#[path = "../../tests/src/integration/static_datamodel/mod.rs"]
pub mod static_datamodel;
