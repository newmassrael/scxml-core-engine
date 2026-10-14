// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the specs tests share: the stand-ins of `sce_specs::testing`, and a folder of their own.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub use sce_specs::testing::*;

/// A fresh, empty folder under the build's temporary directory, named for the test that asked
/// and unique to this run.
pub fn scratch(label: &str) -> PathBuf {
    scratch_in(Path::new(env!("CARGO_TARGET_TMPDIR")), label)
}
