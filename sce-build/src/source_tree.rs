// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where the SCE source tree a run works in is, found when the run starts.
//!
//! A native generator reads parts of the tree from disk while it runs: the
//! template tree, the schemas, the conformance registry, and the sources
//! `verify-generator` holds the binary to. It finds them from where it runs,
//! never from where it was compiled. A path fixed at compile time is the build
//! machine's: it names a checkout the binary may never see again, it makes two
//! builds of one source differ in their bytes by the directory each was built
//! in, so that no cache can hand one build's output to the other, and it
//! resolved a generator built in one checkout and run in another to the first
//! (a regeneration run in a linked worktree rewrote files of the main
//! checkout).
//!
//! The search climbs from three directories, in this order, and takes the
//! first directory that holds what is looked for:
//!
//! 1. the working directory, the tree the run was started in;
//! 2. `CARGO_MANIFEST_DIR` as the run's environment has it, the package Cargo
//!    runs this process for (`cargo run`, `cargo test`, a build script),
//!    which is how a workspace that vendors SCE finds the vendored tree when it
//!    runs the generator through Cargo;
//! 3. the directory of the running executable, the tree a binary sits in, such
//!    as an output placed inside the checkout it was built for.
//!
//! What the compiler knows without naming a machine is where this crate sits
//! under the root it was given the crate's sources from: `file!()` is
//! `sce-build/src/source_tree.rs` in this repository and
//! `vendor/sce/sce-build/src/source_tree.rs` in a workspace that keeps SCE
//! under `vendor/sce`, since Cargo names the sources of a package under its
//! workspace relative to the workspace's root. Beside each directory the
//! search climbs through it also looks at that place, so a build script of a
//! vendoring workspace, run from its own package, finds the vendored tree.
//!
//! An installed generator, on a machine with no checkout, finds nothing here
//! and is told where the tree is: `SCE_TEMPLATE_DIR`, `SCE_SCHEMAS_DIR`,
//! `SCE_WORKSPACE_ROOT` or `--workspace-root`, which each caller reads before
//! it searches.

use std::path::{Path, PathBuf};

/// This file, as the compiler was given it.
const THIS_FILE: &str = file!();

/// Where the SCE tree sits under the root the compiler was given this crate's
/// sources from: [`THIS_FILE`] without `sce-build/src/source_tree.rs`. Empty
/// in this repository. Absolute when the compiler was given the file by an
/// absolute path (a package outside the workspace that builds it), which names
/// the tree itself. `None` when the file was named in no such shape.
fn tree_under_build_root() -> Option<&'static Path> {
    Path::new(THIS_FILE).parent()?.parent()?.parent()
}

/// The directories the search climbs from, in the order it takes them: the
/// working directory, the package Cargo runs this process for, and the
/// directory of the running executable. One the process cannot name is left
/// out.
pub fn search_starts() -> Vec<PathBuf> {
    let mut starts = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    if let Some(package) = std::env::var_os("CARGO_MANIFEST_DIR") {
        starts.push(PathBuf::from(package));
    }
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        starts.push(dir);
    }
    starts
}

/// The SCE tree this run works in: the first directory of the search that
/// holds `marker`, a path relative to the tree's root (`tools/codegen/templates`,
/// `schemas/sce-forge.xsd`).
pub fn find(marker: &Path) -> Option<PathBuf> {
    find_from(marker, &search_starts())
}

/// [`find`] from the directories `starts` instead of this process's own.
pub fn find_from(marker: &Path, starts: &[PathBuf]) -> Option<PathBuf> {
    search(marker, starts, tree_under_build_root())
}

/// The search itself: each start in turn, each of its ancestors from the
/// nearest, and at each the directory and the place `under` names beneath it.
fn search(marker: &Path, starts: &[PathBuf], under: Option<&Path>) -> Option<PathBuf> {
    let under = under.filter(|place| !place.as_os_str().is_empty());
    starts
        .iter()
        .flat_map(|start| start.ancestors())
        .find_map(|dir| {
            std::iter::once(dir.to_path_buf())
                .chain(under.map(|place| dir.join(place)))
                .find(|candidate| candidate.join(marker).exists())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    // A path no tree but the ones these tests make holds, so that a temporary
    // directory inside some checkout cannot answer for them.
    const MARKER: &str = "source-tree-search/marker";

    fn tree_at(root: &Path) {
        std::fs::create_dir_all(root.join(MARKER)).expect("make a tree");
    }

    /// This crate's place under the root its sources were given from: the
    /// root itself, since Cargo names a workspace member's files relative to
    /// the workspace.
    #[test]
    fn this_crate_sits_at_the_root_of_the_tree_that_builds_it() {
        assert_eq!(tree_under_build_root(), Some(Path::new("")));
    }

    /// A start inside a tree finds that tree, by climbing to it.
    #[test]
    fn a_start_inside_a_tree_finds_it() {
        let dir = tempfile::tempdir().expect("a directory");
        let root = dir.path().join("checkout");
        tree_at(&root);
        let deep = root.join("sce-build/src");
        std::fs::create_dir_all(&deep).expect("a directory inside the tree");
        assert_eq!(search(Path::new(MARKER), &[deep], None), Some(root));
    }

    /// The first start that reaches a tree decides, not the tree that is
    /// nearest to some later start: the working directory is taken before
    /// the package and the executable.
    #[test]
    fn an_earlier_start_is_taken_before_a_later_one() {
        let dir = tempfile::tempdir().expect("a directory");
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        tree_at(&first);
        tree_at(&second);
        let found = search(Path::new(MARKER), &[first.join("a"), second.clone()], None);
        assert_eq!(found, Some(first));
    }

    /// A workspace that keeps the tree beneath it, under the place this crate
    /// was built from, is found from a package of that workspace.
    #[test]
    fn a_tree_kept_beneath_a_workspace_is_found_from_its_package() {
        let dir = tempfile::tempdir().expect("a directory");
        let workspace = dir.path().join("consumer");
        let vendored = workspace.join("vendor/sce");
        tree_at(&vendored);
        let package = workspace.join("app");
        std::fs::create_dir_all(&package).expect("the package");
        let place = Path::new("vendor/sce");
        assert_eq!(
            search(
                Path::new(MARKER),
                std::slice::from_ref(&package),
                Some(place)
            ),
            Some(vendored),
        );
        assert_eq!(search(Path::new(MARKER), &[package], None), None);
    }

    /// No start reaches a tree: nothing is found, and nothing is made up.
    #[test]
    fn no_tree_above_any_start_is_none() {
        let dir = tempfile::tempdir().expect("a directory");
        let start = dir.path().join("nowhere");
        std::fs::create_dir_all(&start).expect("a directory");
        assert_eq!(
            search(Path::new(MARKER), &[start], Some(Path::new(""))),
            None
        );
    }
}
