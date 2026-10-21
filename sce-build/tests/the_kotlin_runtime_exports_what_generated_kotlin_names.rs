// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A project that depends on the Kotlin runtime alone compiles the Kotlin the generator
//! writes: every library generated Kotlin names is one the runtime exports (`api`), not
//! one it only uses (`implementation`).
//!
//! # What was wrong
//!
//! Measured 2026-10-21 by a project outside this repository that compiled a generated
//! machine against `sce-kotlin-runtime`: a machine's public `snapshot` is a
//! `kotlinx.coroutines.flow.StateFlow`, and the runtime declared coroutines as an
//! `implementation` dependency, so the generated file did not compile ("Unresolved
//! reference 'kotlinx'"). A `sce-static` machine names the forge runtime's
//! `SceChecked` too, which the runtime did not depend on at all. This repository's own
//! test module declared both itself, which hid it.
//!
//! The names are read from what generated Kotlin is made of: the Kotlin templates of
//! the statechart and of the forge kinds (the conformance harness and the test
//! templates aside, which render test code into a test source set), and the committed
//! generated trees, which hold what the lowering writes in Rust code rather than in a
//! template. Each package a name is in is mapped to the library that provides it; a
//! package this test does not know fails, so a template that starts naming a new
//! library is held here the day it does.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// What provides a package generated Kotlin names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Provider {
    /// The runtime itself.
    Runtime,
    /// Code the generator writes beside the machine (a forge kind's package, a
    /// machine's own).
    Generated,
    /// A library the runtime must export, spelled as its build file names it.
    Library(&'static str),
}

/// Each package root generated Kotlin may name, and what provides it. The longest
/// root a name begins with decides.
const PACKAGES: [(&str, Provider); 6] = [
    ("com.sce.runtime", Provider::Runtime),
    ("com.sce.generated", Provider::Generated),
    ("com.sce.integration", Provider::Generated),
    (
        "com.sce.forge.runtime",
        Provider::Library("project(\":sce-forge-runtime-kotlin\")"),
    ),
    (
        "kotlinx.coroutines",
        Provider::Library("libs.kotlinx.coroutines.core"),
    ),
    (
        "kotlinx.serialization",
        Provider::Library("libs.kotlinx.serialization.json"),
    ),
];

/// The roots a name of a library is looked for under: everything SCE publishes and
/// everything of the Kotlin libraries outside the standard library.
const ROOTS: [&str; 2] = ["com.sce.", "kotlinx."];

/// Every file under `dir`, recursively, but for the directories named in `skip`.
fn files(dir: &Path, skip: &[&str], found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !skip.contains(&name) {
                files(&path, skip, found);
            }
        } else {
            found.push(path);
        }
    }
}

/// The dotted names under [`ROOTS`] that `text` holds, each the longest run of
/// identifier characters and dots from where a root starts, without a trailing dot.
fn qualified_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    for root in ROOTS {
        let mut from = 0;
        while let Some(at) = text[from..].find(root) {
            let start = from + at;
            let before = text[..start].chars().next_back();
            from = start + root.len();
            // `xcom.sce.` or `my_kotlinx.` is not the root.
            if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
                continue;
            }
            let end = text[start..]
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
                .map_or(text.len(), |len| start + len);
            names.push(text[start..end].trim_end_matches('.').to_string());
        }
    }
    names
}

/// What provides `name`, by the longest root of [`PACKAGES`] it is in.
fn provider(name: &str) -> Option<Provider> {
    PACKAGES
        .iter()
        .filter(|(root, _)| name == *root || name.starts_with(&format!("{root}.")))
        .max_by_key(|(root, _)| root.len())
        .map(|(_, provider)| *provider)
}

/// The files generated Kotlin is made of.
fn sources() -> Vec<PathBuf> {
    let root = repo_root();
    let mut found = Vec::new();
    for templates in [
        "tools/codegen/templates/kotlin",
        "tools/codegen/templates/forge/kotlin",
    ] {
        files(&root.join(templates), &["conformance"], &mut found);
    }
    // A test template renders a test class into a test source set, which the
    // consumer's own test dependencies compile.
    found.retain(|path| {
        !path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.contains("_test."))
    });
    for generated in [
        "backends/kotlin/tests/src/main/kotlin/com/sce/integration",
        "backends/kotlin/tests/src/main/kotlin/com/sce/generated",
    ] {
        files(&root.join(generated), &[], &mut found);
    }
    found
}

/// Every library generated Kotlin names, with one file that names it.
fn libraries_named() -> BTreeMap<&'static str, (String, PathBuf)> {
    let mut libraries = BTreeMap::new();
    let mut unknown = Vec::new();
    let sources = sources();
    assert!(
        sources.len() > 100,
        "read only {} files of generated Kotlin: the walk stopped finding them",
        sources.len()
    );
    for path in sources {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for name in qualified_names(&text) {
            match provider(&name) {
                Some(Provider::Library(library)) => {
                    libraries.entry(library).or_insert((name, path.clone()));
                }
                Some(Provider::Runtime | Provider::Generated) => {}
                None => unknown.push(format!("{name} in {}", path.display())),
            }
        }
    }
    assert!(
        unknown.is_empty(),
        "generated Kotlin names packages this test does not know the provider of; add each \
         to PACKAGES, and a library to the runtime's `api`:\n{}",
        unknown.join("\n")
    );
    libraries
}

/// The `commonMain` dependencies of the runtime's build file, as written.
fn runtime_build() -> String {
    std::fs::read_to_string(repo_root().join("backends/kotlin/runtime/build.gradle.kts"))
        .expect("read the runtime's build file")
}

#[test]
fn every_library_generated_kotlin_names_is_exported_by_the_runtime() {
    let build = runtime_build();
    let named = libraries_named();
    assert!(
        named.contains_key("libs.kotlinx.coroutines.core"),
        "a machine's `snapshot` is a coroutines StateFlow, and the walk did not find it"
    );
    for (library, (name, path)) in &named {
        assert!(
            build.contains(&format!("api({library})")),
            "generated Kotlin names `{name}` ({}), which {library} provides, and the runtime \
             does not export it: a project that depends on the runtime alone cannot compile \
             it. Declare `api({library})` in backends/kotlin/runtime/build.gradle.kts.",
            path.display()
        );
        assert!(
            !build.contains(&format!("implementation({library})")),
            "{library} is declared as an `implementation` of the runtime beside its `api`"
        );
    }
}

/// The control for the reading: a name is taken whole, a root inside another word is
/// not one, and the longest root decides.
#[test]
fn a_name_is_read_whole_and_by_its_longest_root() {
    assert_eq!(
        qualified_names("val s: kotlinx.coroutines.flow.StateFlow<S> // xkotlinx.no"),
        ["kotlinx.coroutines.flow.StateFlow"]
    );
    assert_eq!(
        qualified_names("import com.sce.forge.runtime.*\nimport com.sce.runtime."),
        ["com.sce.forge.runtime", "com.sce.runtime"]
    );
    assert_eq!(
        provider("com.sce.forge.runtime.SceChecked"),
        Some(Provider::Library("project(\":sce-forge-runtime-kotlin\")"))
    );
    assert_eq!(
        provider("com.sce.runtime.SendHelper"),
        Some(Provider::Runtime)
    );
    assert_eq!(provider("com.sce.mesh.Router"), None);
}
