// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the workbench tests share: the stand-ins of `sce_works::testing`, a folder of their own,
//! the product's programs a live test runs, and the stand-ins of a model server.

#![allow(dead_code)]

/// The stand-ins a model server run is held against (a shell script is one of them).
#[cfg(unix)]
pub mod model_server;

use std::path::{Path, PathBuf};

pub use sce_works::testing::*;

/// A fresh, empty folder under the build's temporary directory, named for the test that asked
/// and unique to this run.
pub fn scratch(label: &str) -> PathBuf {
    scratch_in(Path::new(env!("CARGO_TARGET_TMPDIR")), label)
}

/// The `sce-work` a live test hands its client: the one the environment names (`SCE_WORK`), else
/// the repository's debug build, which has to be there. It is the works' command and not this
/// crate's, so it is not built for these tests.
pub fn sce_work() -> PathBuf {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let work = std::env::var_os("SCE_WORK")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("target/debug/sce-work"));
    assert!(
        work.is_file(),
        "{} is not built: cargo build -p sce-works --features cli --bin sce-work, or name it with \
         SCE_WORK",
        work.display()
    );
    work
}
