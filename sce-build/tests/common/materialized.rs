// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A declared tree as its actions write it (docs/adr/0097).
//!
//! A tree the checkout does not commit (`committed = false` in
//! `tools/codegen/trees/<tree>.toml`) is a build output, so a suite that reads
//! one reads it where the generator under test writes it: `sce-codegen
//! materialize <tree> --into <dir> --replace`, run once per tree and per test
//! binary, into a directory of this binary under Cargo's scratch directory.
//! The files lie at the paths they have under the checkout's root, so a path a
//! suite named in the checkout names the same file under [`tree`]'s answer.
//!
//! The run goes through this machine's action cache, which answers an action
//! only for the generator whose sources the run was built from, so what is
//! read is what this generator writes.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// The directory `tree` was materialized into by `codegen` (the
/// `sce-codegen` binary of the suite, `env!("CARGO_BIN_EXE_sce-codegen")`),
/// written the first time a test of this binary asks for it and reused
/// after. Panics, with what the generator said, when the tree is not
/// written.
pub fn tree(codegen: &str, name: &str) -> PathBuf {
    static WRITTEN: OnceLock<Mutex<BTreeMap<String, PathBuf>>> = OnceLock::new();
    let written = WRITTEN.get_or_init(|| Mutex::new(BTreeMap::new()));
    // A test that panicked while holding the lock left the map as it was, and
    // a tree is in it only once it was written whole.
    let mut written = written.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(dir) = written.get(name) {
        return dir.clone();
    }
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("materialized")
        .join(env!("CARGO_CRATE_NAME"))
        .join(name);
    let out = Command::new(codegen)
        .arg("materialize")
        .arg(name)
        .arg("--into")
        .arg(&dir)
        .arg("--replace")
        .current_dir(super::repository::root())
        .output()
        .unwrap_or_else(|e| panic!("run {codegen}: {e}"));
    assert!(
        out.status.success(),
        "`sce-codegen materialize {name}` failed, so no suite can read the tree:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    written.insert(name.to_string(), dir.clone());
    dir
}

/// The file at `path`, a path under the checkout's root, in `tree` as
/// [`tree`] materialized it.
pub fn file(codegen: &str, name: &str, path: &str) -> PathBuf {
    tree(codegen, name).join(path)
}

/// Every file named `file_name` at or below `dir`, in path order: the
/// sidecars of a materialized tree, for one.
pub fn files_named(dir: &Path, file_name: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = std::fs::read_dir(&current)
            .unwrap_or_else(|e| panic!("read {}: {e}", current.display()));
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.file_name().is_some_and(|n| n == file_name) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}
