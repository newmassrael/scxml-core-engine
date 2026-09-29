// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every example the kind catalog teaches is a document the product
//! accepts, as the kind it is the example of.
//!
//! # Why the examples are held here
//!
//! `sce-codegen kinds` is what an author outside this tree reads before
//! writing a document of a kind they have never seen. An example there is
//! copied, so an example `check` refuses teaches the refusal. The catalog
//! embeds `sce-build/kind-examples/<kind>.scxml` at build time, and nothing
//! about embedding a file says the product still accepts it; this test puts
//! each one through the same binary an author's document goes through:
//! `check --lint` must accept it and name its kind on the manifest, and
//! `pseudo` must render it, because the pseudocode page is what the owner
//! reviews.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn run(args: &[&str]) -> Output {
    Command::new(sce_codegen_bin())
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("run sce-codegen")
}

fn catalog(args: &[&str]) -> serde_json::Value {
    let out = run(&[&["kinds"], args].concat());
    assert!(
        out.status.success(),
        "kinds must succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("the catalog is one JSON line")
}

/// Accepted by `check --lint`, read as its own kind, and rendered by
/// `pseudo` — for every kind that has an example; and every kind without
/// one says why.
#[test]
fn every_example_checks_as_its_kind_and_renders() {
    let catalog = catalog(&[]);
    let kinds = catalog["kinds"].as_array().expect("kinds is an array");
    let mut checked = 0usize;
    for entry in kinds {
        let name = entry["name"].as_str().expect("name");
        let example = &entry["example"];
        let Some(document) = example.get("document") else {
            assert!(
                example["absent"]["because"]
                    .as_str()
                    .is_some_and(|s| !s.is_empty()),
                "{name}: an example is either a document or a reason: {example}"
            );
            continue;
        };
        let path = document["path"].as_str().expect("path");
        let on_disk = std::fs::read_to_string(repo_root().join(path))
            .unwrap_or_else(|e| panic!("{name}: {path}: {e}"));
        assert_eq!(
            document["text"].as_str(),
            Some(on_disk.as_str()),
            "{name}: the embedded text is not the file"
        );

        let check = run(&["--error-format", "json", "check", "--lint", path]);
        assert!(
            check.status.success(),
            "{name}: {path} is refused by check --lint: {}",
            String::from_utf8_lossy(&check.stderr)
        );
        let manifest: serde_json::Value =
            serde_json::from_slice(&check.stdout).expect("check writes one manifest line");
        assert_eq!(
            manifest["document_kind"]["name"].as_str(),
            Some(name),
            "{name}: {path} is read as another kind: {manifest}"
        );

        let pseudo = run(&["pseudo", path]);
        assert!(
            pseudo.status.success() && !pseudo.stdout.is_empty(),
            "{name}: {path} does not render as pseudocode: {}",
            String::from_utf8_lossy(&pseudo.stderr)
        );
        checked += 1;
    }
    // The catalog covers eighteen kinds and one of them has no example
    // today; a walk that checked far fewer found the catalog shrunk, not
    // the examples sound.
    assert!(checked >= 17, "only {checked} examples checked");
}

/// Every file in the example directory is the example of the kind it is
/// named for, so a document no entry embeds cannot sit there looking like
/// one the catalog vouches for.
#[test]
fn every_file_in_the_example_directory_is_a_kind_example() {
    let catalog = catalog(&[]);
    let claimed: std::collections::BTreeSet<String> = catalog["kinds"]
        .as_array()
        .expect("kinds")
        .iter()
        .filter_map(|entry| entry["example"]["document"]["path"].as_str())
        .map(str::to_string)
        .collect();
    let dir = "sce-build/kind-examples";
    let on_disk: std::collections::BTreeSet<String> = std::fs::read_dir(repo_root().join(dir))
        .expect("read the example directory")
        .map(|entry| {
            let name = entry.expect("directory entry").file_name();
            format!("{dir}/{}", name.to_string_lossy())
        })
        .collect();
    assert!(!on_disk.is_empty(), "the example directory is empty");
    assert_eq!(on_disk, claimed);
}

/// `--kind` narrows the catalog to one entry, and refuses a name the
/// parser does not accept rather than answering with nothing.
#[test]
fn one_kind_is_asked_for_by_name() {
    let one = catalog(&["--kind", "interpolation"]);
    let kinds = one["kinds"].as_array().expect("kinds");
    assert_eq!(kinds.len(), 1);
    assert_eq!(kinds[0]["name"], "interpolation");

    let refused = run(&["kinds", "--kind", "state-machine"]);
    assert!(!refused.status.success(), "an unknown kind must be refused");
    assert!(refused.stdout.is_empty(), "a refusal writes no catalog");
}
