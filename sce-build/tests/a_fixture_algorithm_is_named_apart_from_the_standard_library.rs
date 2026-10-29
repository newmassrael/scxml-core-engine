// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The unit a document is written as is named for the document (`name=`), in every
// language: a Swift module, a C++ namespace and a header, a Kotlin package. A fixture
// algorithm that shares its name with an algorithm of the standard library is the same
// unit under two documents. The backends that generate one set of documents never
// meet it, and one that generates every document it can into a single namespace (the
// SwiftPM target of backends/swift/tests) compiles a caller of the one against the
// other.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Every `.scxml` under `dir`, recursively.
fn documents(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a directory") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            documents(&path, out);
        } else if path.extension().is_some_and(|e| e == "scxml") {
            out.push(path);
        }
    }
}

/// The `name=` of each algorithm document under `dir`, with the path it is written in.
fn algorithm_names(dir: &Path) -> BTreeMap<String, Vec<PathBuf>> {
    const SCE: &str = "http://sce.dev/ext";
    let mut paths = Vec::new();
    documents(dir, &mut paths);
    let mut names: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for path in paths {
        let text = std::fs::read_to_string(&path).expect("a document");
        let Ok(document) = roxmltree::Document::parse(&text) else {
            continue;
        };
        let root = document.root_element();
        if root.attribute((SCE, "kind")) != Some("algorithm") {
            continue;
        }
        if let Some(name) = root.attribute("name") {
            names.entry(name.to_string()).or_default().push(path);
        }
    }
    names
}

#[test]
fn a_fixture_algorithm_is_named_apart_from_the_standard_library() {
    let root = repo_root();
    let fixtures = algorithm_names(&root.join("sce-build/tests/fixtures/static_datamodel"));
    let library = algorithm_names(&root.join("stdlib"));
    assert!(!fixtures.is_empty(), "no fixture algorithm to judge");
    assert!(!library.is_empty(), "no standard algorithm to judge");
    let shared: Vec<String> = fixtures
        .keys()
        .filter(|name| library.contains_key(*name))
        .map(|name| {
            format!(
                "{name}: {:?} and {:?}",
                fixtures[name]
                    .iter()
                    .map(|p| p.strip_prefix(&root).unwrap_or(p).display().to_string())
                    .collect::<Vec<_>>(),
                library[name]
                    .iter()
                    .map(|p| p.strip_prefix(&root).unwrap_or(p).display().to_string())
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    assert!(
        shared.is_empty(),
        "an algorithm of the fixtures and one of the standard library share a name, so \
         their units are one: give the fixture another `name=`:\n{}",
        shared.join("\n")
    );
}

#[test]
fn no_two_fixture_algorithms_share_a_name() {
    let fixtures = algorithm_names(&repo_root().join("sce-build/tests/fixtures/static_datamodel"));
    let twice: Vec<&String> = fixtures
        .iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(name, _)| name)
        .collect();
    assert!(
        twice.is_empty(),
        "two fixture algorithms share a name, so their units are one: {twice:?}"
    );
}
