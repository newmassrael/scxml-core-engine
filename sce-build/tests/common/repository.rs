// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Which files the repository holds.
//!
//! Eight suites sweep every `.scxml` to hold a claim about the whole
//! corpus, and each used to find the documents by walking the directory
//! tree and skipping a hand-written list — `target`, `.git`,
//! `node_modules`, character for character the same in all eight. So each
//! read every `.scxml` any process had left under the root. Measured
//! 2026-09-22 that was 7396 documents where the repository holds 764:
//! 5938 under a path `.gitignore` excludes as scratch, and 638 under
//! `build/`, which the list never named. One of the eight went red on that
//! scratch and would have been green on a clean checkout of the same
//! commit — its verdict was a property of the disk, not of the tree.
//!
//! The population is what git says the repository holds: every tracked
//! file, plus every untracked one git does not ignore, because a document
//! written but not yet staged is about to be committed and must be judged
//! before it is. In CI the second set is empty and the two readings agree.
//! What git ignores is excluded by the repository's own declaration rather
//! than by a list here that has to guess it.
//!
//! ⚠ Calling this makes a suite tree-wide: a document added ANYWHERE
//! changes what it reads, which no `paths:` filter written over today's
//! tree can enumerate. `workflow_trigger_coverage` sees the call through
//! this module and requires the suite in `UNFILTERABLE_GATES` and in
//! `scripts/gates/tree-hygiene.sh` — which is the point of routing every
//! such read through one function.

#![allow(dead_code)]

use std::path::PathBuf;

/// The repository root: `sce-build`'s parent.
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build's parent is the repo root")
        .to_path_buf()
}

/// Every file the repository holds whose name ends in `.<extension>`, as
/// absolute paths, sorted.
///
/// A tracked path the working tree has deleted is left out: `--cached`
/// still lists it until the deletion is staged, and nobody can read it.
pub fn files_with_extension(extension: &str) -> Vec<PathBuf> {
    let root = root();
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(&root)
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
        ])
        .arg(format!("*.{extension}"))
        .output()
        .expect("git ls-files runs in the repository");
    assert!(
        out.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut files: Vec<PathBuf> = out
        .stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| root.join(String::from_utf8_lossy(s).as_ref()))
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    files.dedup();
    files
}
