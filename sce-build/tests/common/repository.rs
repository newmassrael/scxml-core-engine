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
//! Every suite that asks git which files exist asks here, through
//! [`paths_git_tracks`] or [`paths_git_holds`], and the difference between
//! the two is a decision each caller makes out loud rather than an
//! argument list it happens to copy.
//!
//! ⚠ Calling any of these makes a suite tree-wide: a file added ANYWHERE
//! changes what it reads, which no `paths:` filter written over today's
//! tree can enumerate. `workflow_trigger_coverage` sees the call through
//! this module and requires the suite in `UNFILTERABLE_GATES` and in
//! `scripts/gates/tree-hygiene.sh` — which is the point of routing every
//! such read through one place. Their names are compounds no prose uses,
//! because that detector matches a helper's name as a word in code with
//! the string literals left in.

#![allow(dead_code)]

use std::path::PathBuf;

/// The repository root: `sce-build`'s parent.
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build's parent is the repo root")
        .to_path_buf()
}

/// Every path git TRACKS that matches one of `pathspecs` — every tracked
/// path when there are none — repository-relative, as git stores it.
///
/// This is the population a claim about "the tree as committed" wants: an
/// untracked file, even one about to be added, is not part of it. A suite
/// whose subject is the difference between tracked and untracked — a
/// sourced script that nobody committed — must read this one and never
/// [`paths_git_holds`].
pub fn paths_git_tracks(pathspecs: &[&str]) -> Vec<String> {
    run_ls_files(&[], pathspecs)
}

/// Every path the repository HOLDS that matches one of `pathspecs`: what
/// git tracks, plus every untracked path it does not ignore.
///
/// The second half is what a document written but not yet staged is, and
/// a corpus sweep has to judge it before it is committed rather than
/// after. In CI it is empty, so the two functions agree there.
pub fn paths_git_holds(pathspecs: &[&str]) -> Vec<String> {
    run_ls_files(&["--cached", "--others", "--exclude-standard"], pathspecs)
}

/// Every file the repository holds whose name ends in `.<extension>`, as
/// absolute paths, sorted.
///
/// A tracked path the working tree has deleted is left out: `--cached`
/// still lists it until the deletion is staged, and nobody can read it.
pub fn files_with_extension(extension: &str) -> Vec<PathBuf> {
    readable_files(paths_git_holds(&[&format!("*.{extension}")]))
}

/// Every file git TRACKS that matches one of `pathspecs`, as absolute
/// paths, sorted — [`paths_git_tracks`] for a caller that opens what it
/// lists.
///
/// For a claim about the tree as COMMITTED, where a document written but
/// not yet added must not change the verdict; a sweep that should judge
/// such a document before it is committed reads [`files_with_extension`].
/// A tracked path the working tree has deleted is left out, as there.
pub fn files_git_tracks(pathspecs: &[&str]) -> Vec<PathBuf> {
    readable_files(paths_git_tracks(pathspecs))
}

/// Repository-relative `paths` as absolute paths to files the working tree
/// has, sorted and each once.
fn readable_files(paths: Vec<String>) -> Vec<PathBuf> {
    let root = root();
    let mut files: Vec<PathBuf> = paths
        .into_iter()
        .map(|p| root.join(p))
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    files.dedup();
    files
}

/// `git ls-files -z`, always with `-z`.
///
/// ⚠ Without it, git's default `core.quotePath` writes a path holding any
/// byte outside printable ASCII as a quoted octal escape — `"\355\225..."`
/// — which names no file on disk, so a caller splitting the output on
/// newlines opens nothing and skips the file in silence. Measured
/// 2026-09-22: nine of eighteen per-suite copies of this read omitted it,
/// one of them the gate whose whole subject is non-Latin text. No tracked
/// path needed quoting that day, so nothing had yet been dropped.
fn run_ls_files(options: &[&str], pathspecs: &[&str]) -> Vec<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(root())
        .args(["ls-files", "-z"])
        .args(options)
        .arg("--")
        .args(pathspecs)
        .output()
        .expect("git ls-files runs in the repository");
    assert!(
        out.status.success(),
        "git ls-files failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}
