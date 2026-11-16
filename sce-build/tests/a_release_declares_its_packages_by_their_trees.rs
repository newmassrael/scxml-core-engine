// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The release pipeline (docs/adr/0097, section 7) publishes the packages
// tools/codegen/release.toml declares, each with the generated trees it
// compiles materialized inside it. A package is a directory, and a tree is
// its only when every root of the tree lies under that directory: a tree
// whose files fall outside would ship in no archive, and a tree in two
// packages would ship twice from one materialization. `scripts/release.py
// --check` holds the declaration to the tree manifests, and the tag to the
// workspace's version, before a release writes anything. It is driven here
// the way the release workflow drives it, so what is measured is the reader
// that runs in a release.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// `scripts/release.py --check --root <root>` and `extra`: whether it
/// passed, and what it said on stdout and stderr.
fn check(root: &Path, extra: &[&str]) -> (bool, String) {
    let out = Command::new("python3")
        .arg(repo_root().join("scripts/release.py"))
        .arg("--check")
        .arg("--root")
        .arg(root)
        .args(extra)
        .output()
        .expect("run python3");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !text.contains("Traceback"),
        "the reader died rather than answering:\n{text}"
    );
    (out.status.success(), text)
}

/// A repository of one workspace version, the tree manifests `trees` (name,
/// roots), the directories `dirs`, and `declaration` as its release.toml.
fn repository(trees: &[(&str, &[&str])], dirs: &[&str], declaration: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a repository");
    let write = |path: &str, text: &str| {
        let path = root.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    };
    write(
        "Cargo.toml",
        "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"1.2.3\"\n",
    );
    for (name, roots) in trees {
        let roots: Vec<String> = roots.iter().map(|r| format!("{r:?}")).collect();
        write(
            &format!("tools/codegen/trees/{name}.toml"),
            &format!(
                "[tree]\nname = \"{name}\"\nroots = [{}]\n",
                roots.join(", ")
            ),
        );
    }
    for dir in dirs {
        fs::create_dir_all(root.path().join(dir)).unwrap();
    }
    write("tools/codegen/release.toml", declaration);
    root
}

const ONE: &str = "[[package]]\nname = \"pkg\"\npath = \"lib/pkg\"\ntrees = [\"t\"]\n";

/// The repository's own declaration names packages that exist, with trees
/// whose roots lie under them, and its workspace version is a release tag.
#[test]
fn the_repository_declares_its_packages_by_their_trees() {
    let root = repo_root();
    let (passed, text) = check(&root, &[]);
    assert!(passed, "{text}");
    assert!(text.contains("package(s) declared"), "{text}");

    let manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let version = manifest
        .parse::<toml::Table>()
        .unwrap()
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .expect("the workspace declares its version")
        .to_string();
    let (passed, text) = check(&root, &["--tag", &format!("v{version}")]);
    assert!(passed, "{text}");
}

#[test]
fn a_package_whose_trees_lie_under_it_is_accepted() {
    let repo = repository(&[("t", &["lib/pkg/src/generated"])], &["lib/pkg"], ONE);
    let (passed, text) = check(repo.path(), &["--tag", "v1.2.3"]);
    assert!(passed, "{text}");
    assert!(text.contains("1 package(s) declared, 1 tree(s)"), "{text}");
}

/// A tree that writes outside its package would ship in no archive.
#[test]
fn a_tree_that_writes_outside_its_package_is_refused() {
    let repo = repository(
        &[("t", &["lib/pkg/src", "lib/other/gen"])],
        &["lib/pkg"],
        ONE,
    );
    let (passed, text) = check(repo.path(), &[]);
    assert!(!passed, "{text}");
    assert!(text.contains("outside lib/pkg"), "{text}");
    // A directory whose name the package's begins is not under it.
    let repo = repository(&[("t", &["lib/pkg2/src"])], &["lib/pkg"], ONE);
    let (passed, text) = check(repo.path(), &[]);
    assert!(!passed, "{text}");
}

#[test]
fn a_tree_in_two_packages_is_refused() {
    let declaration =
        format!("{ONE}\n[[package]]\nname = \"two\"\npath = \"lib\"\ntrees = [\"t\"]\n");
    let repo = repository(&[("t", &["lib/pkg/src"])], &["lib/pkg"], &declaration);
    let (passed, text) = check(repo.path(), &[]);
    assert!(!passed, "{text}");
    assert!(text.contains("is in both pkg and two"), "{text}");
}

#[test]
fn a_package_named_twice_or_of_no_directory_or_tree_is_refused() {
    let twice = format!("{ONE}\n{ONE}");
    let repo = repository(&[("t", &["lib/pkg/src"])], &["lib/pkg"], &twice);
    let (passed, text) = check(repo.path(), &[]);
    assert!(!passed && text.contains("declared twice"), "{text}");

    let repo = repository(&[("t", &["lib/pkg/src"])], &[], ONE);
    let (passed, text) = check(repo.path(), &[]);
    assert!(!passed && text.contains("no directory"), "{text}");

    let repo = repository(&[], &["lib/pkg"], ONE);
    let (passed, text) = check(repo.path(), &[]);
    assert!(!passed && text.contains("does not exist"), "{text}");

    let repo = repository(&[("t", &["lib/pkg/src"])], &["lib/pkg"], "");
    let (passed, text) = check(repo.path(), &[]);
    assert!(
        !passed && text.contains("declares no [[package]]"),
        "{text}"
    );
}

/// A tag releases the workspace's version and has the form a Go module and
/// SwiftPM resolve.
#[test]
fn a_tag_that_is_not_the_workspace_version_is_refused() {
    let repo = repository(&[("t", &["lib/pkg/src"])], &["lib/pkg"], ONE);
    let (passed, text) = check(repo.path(), &["--tag", "v1.2.4"]);
    assert!(
        !passed && text.contains("the workspace is at '1.2.3'"),
        "{text}"
    );
    for bad in ["1.2.3", "v1.2", "v01.2.3", "v1.2.3-rc1"] {
        let (passed, text) = check(repo.path(), &["--tag", bad]);
        assert!(!passed && text.contains("is not v<major>"), "{bad}: {text}");
    }
}
