// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Embeds the generator's own commit into the binary.
//!
//! The crate version is frozen at `0.1.0` under the pre-1.0 no-versioning
//! rule, so it identifies nothing. Without a second signal a consumer that
//! commits generated output has no way to ask a binary which generator it
//! is, and must hand-maintain a version sidecar instead — bookkeeping that
//! drifts silently and leaves a committed tree unattributable to any
//! single commit.
//!
//! Scope is deliberately the commit alone. A dirty-worktree flag cannot be
//! kept honest here: the rerun triggers below watch the ref, not the
//! worktree, so a flag computed at build time would go stale on the next
//! edit — and a stale "clean" claim is worse than no claim. The stamp
//! therefore names the committed state the build started from, which is
//! exactly what a pinned or hermetic build has. Uncommitted edits are the
//! job of the §synth-6.2.6 source/template hashes, which are recomputed
//! per run from the actual bytes.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let commit = git_commit().unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=SCE_GIT_COMMIT={commit}");
}

/// Short commit of `HEAD`, or `None` when this is not a git checkout (a
/// vendored crate, a release tarball, a Bazel-style fetched dependency).
/// Absence is a normal build shape, not a failure — the build must not
/// break just because provenance is unavailable.
fn git_commit() -> Option<String> {
    let git_dir = git_dir()?;
    // Rebuild when the ref moves, so the embedded value cannot go stale
    // within a working session. A stamp that silently describes the wrong
    // commit is the failure mode this whole surface exists to remove.
    watch(&git_dir.join("HEAD"));
    if let Some(reference) = head_ref(&git_dir) {
        watch(&git_dir.join(&reference));
        // Refs living in `packed-refs` have no file of their own.
        watch(&git_dir.join("packed-refs"));
    }

    let out = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let commit = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!commit.is_empty()).then_some(commit)
}

/// Resolve the `.git` directory, following the `gitdir:` indirection a
/// worktree or submodule checkout uses. The field report's own repro
/// pins a generator via `git worktree add`, so that shape has to work.
fn git_dir() -> Option<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let dir = PathBuf::from(String::from_utf8(out.stdout).ok()?.trim());
    Some(if dir.is_absolute() {
        dir
    } else {
        std::env::current_dir().ok()?.join(dir)
    })
}

/// Symbolic ref `HEAD` points at (`refs/heads/main`), if any. A detached
/// HEAD has none, and the `HEAD` watch alone covers that case.
fn head_ref(git_dir: &Path) -> Option<String> {
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    head.strip_prefix("ref: ").map(|r| r.trim().to_string())
}

fn watch(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
}
