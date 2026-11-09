// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The bridge of ADR 0097, landing 0: every tree a manifest of
// `tools/codegen/trees/` declares, materialized from its actions, is byte for
// byte the tree committed in this checkout, in both directions.
//
// It is `regen-reproduces` expressed through the actions. While a tree is
// still committed, it proves the manifest declares what the regeneration
// scripts do; once the tree leaves version control, it proves the removal
// lost nothing, because the commit that removes it is judged by a tree this
// test proved equal to the one removed.
//
// The binary is driven as a build hook drives it (`sce-codegen materialize`),
// with an action cache of the test's own, so that a result kept by an earlier
// run on this machine cannot stand in for one this generator produces.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::action::materialize::{compare, Difference};
use sce_build::action::tree::trees;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// `sce-codegen materialize <tree> --into <into>` with the action cache at
/// `cache`; the last line it wrote on stderr, which reports the run.
fn materialize(tree: &str, into: &Path, cache: &Path) -> String {
    let out = Command::new(sce_codegen_bin())
        .arg("materialize")
        .arg(tree)
        .arg("--into")
        .arg(into)
        .current_dir(repo_root())
        .env("SCE_ACTION_CACHE", cache)
        .output()
        .expect("sce-codegen runs");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "materialize {tree} failed:\n{stderr}");
    stderr.lines().last().unwrap_or_default().to_string()
}

#[test]
fn every_declared_tree_is_the_committed_tree() {
    let root = repo_root();
    let manifests = trees(&root).expect("the tree manifests read");
    assert!(!manifests.is_empty(), "no tree is declared");
    let cache = tempfile::tempdir().expect("a cache directory");
    let mut report = Vec::new();
    for manifest in &manifests {
        let name = &manifest.tree.name;
        let into = tempfile::tempdir().expect("a directory to materialize into");
        materialize(name, into.path(), cache.path());
        let expanded = manifest.expand(&root).expect("the manifest expands");
        let differences = compare(&expanded, &root, into.path()).expect("the trees compare");
        for difference in differences {
            report.push(format!("{name}: {difference}"));
        }
    }
    assert!(
        report.is_empty(),
        "the declared trees are not the committed ones ({} difference(s)):\n{}",
        report.len(),
        report.join("\n")
    );
}

/// The cache, end to end: a tree materialized a second time through the same
/// cache runs nothing, and writes the same bytes.
#[test]
fn a_tree_materialized_again_is_answered_by_the_cache() {
    let root = repo_root();
    let manifests = trees(&root).expect("the tree manifests read");
    let manifest = manifests.first().expect("a tree is declared");
    let name = &manifest.tree.name;
    let cache = tempfile::tempdir().expect("a cache directory");
    let first = tempfile::tempdir().expect("a directory");
    let second = tempfile::tempdir().expect("a directory");

    let ran = materialize(name, first.path(), cache.path());
    assert!(
        !ran.ends_with(" 0 run"),
        "a cold cache answered every action of {name}: {ran}"
    );
    let reused = materialize(name, second.path(), cache.path());
    assert!(
        reused.ends_with(" 0 run"),
        "a warm cache left actions of {name} to run: {reused}"
    );

    let expanded = manifest.expand(&root).expect("the manifest expands");
    let a = compare(&expanded, first.path(), second.path()).expect("compare");
    // `compare` reads its first tree as a checkout, where only files that say
    // they are generated count; a materialized tree holds nothing else.
    let unexpected: Vec<&Difference> = a.iter().collect();
    assert!(
        unexpected.is_empty(),
        "a tree read from the cache differs from the one that ran: {unexpected:?}"
    );
}
