// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The routes that write a machine, as the suites holding what a declaration
//! adds beside it name them.
//!
//! Two suites hold what a declaration adds beside a machine (the transition
//! function, the transition table) on every route that writes the machine:
//! `generate`, `orchestrate` and `generate-w3c`. Each names a route's
//! arguments and stages a conformance fixture the same way, so both are here
//! once rather than twice.
//!
//! A backend's suite places a test's machines in a directory of its own
//! layout (a crate module, a Go package, a Kotlin package directory, a
//! Python package, a flat build directory), and what the suites claim is what
//! is written beside a machine, not where the layout puts it: so a test's
//! directory is found from the machine written into it
//! ([`directory_holding`]) rather than spelled per backend.
//!
//! `sce-codegen` is not run here: `env!("CARGO_BIN_EXE_sce-codegen")` exists
//! only for a test built with the `cli` feature, and every test binary that
//! declares `mod common` compiles this file.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

/// The arguments of `<via> -l <lang> -o <out>` over the one document `doc`,
/// as `generate` (a positional document) and `orchestrate` (a set of one,
/// `--scxml`) name it.
pub fn route_args<'a>(via: &'a str, lang: &'a str, out: &'a Path, doc: &'a Path) -> Vec<&'a str> {
    let document = doc.to_str().unwrap();
    let mut args = vec![via, "-l", lang, "-o", out.to_str().unwrap()];
    match via {
        "generate" => args.push(document),
        "orchestrate" => args.extend(["--scxml", document]),
        other => panic!("no route {other} writes one document"),
    }
    args
}

/// A conformance suite of one fixture, `test<id>.scxml`, for `generate-w3c`.
pub struct W3cSuite {
    pub resources: PathBuf,
    pub registry: PathBuf,
    pub document: PathBuf,
}

impl W3cSuite {
    /// `text` as fixture `id` under `work`, with a registry naming it alone:
    /// the committed registry's harnesses and version, which are what the
    /// loader asks of one.
    pub fn stage(work: &Path, id: &str, text: &str) -> Self {
        let resources = work.join("resources");
        let fixture_dir = resources.join(id);
        std::fs::create_dir_all(&fixture_dir).expect("create the fixture directory");
        let document = fixture_dir.join(format!("test{id}.scxml"));
        std::fs::write(&document, text).expect("write the fixture");

        let committed = std::fs::read_to_string(
            repo_root().join(sce_build::w3c_registry::W3C_REGISTRY_RELATIVE_PATH),
        )
        .expect("read the committed registry");
        let mut registry: serde_json::Value =
            serde_json::from_str(&committed).expect("the registry is JSON");
        registry["fixtures"] = serde_json::json!([{
            "id": id,
            "harness": "simple",
            "summary": "a fixture declaring what a run's digest is bound to"
        }]);
        let registry_path = work.join("fixtures.json");
        std::fs::write(
            &registry_path,
            serde_json::to_string_pretty(&registry).expect("serialize the registry"),
        )
        .expect("write the registry");
        Self {
            resources,
            registry: registry_path,
            document,
        }
    }

    /// `generate-w3c -l <lang>` of this suite into `out`, records as JSON.
    pub fn args<'a>(&'a self, lang: &'a str, out: &'a Path) -> Vec<&'a str> {
        vec![
            "--error-format=json",
            "generate-w3c",
            "-l",
            lang,
            "--registry",
            self.registry.to_str().unwrap(),
            "--resources",
            self.resources.to_str().unwrap(),
            "--output-dir",
            out.to_str().unwrap(),
        ]
    }
}

/// The directory below `root`, at any depth, that holds a file named `name`;
/// `None` when no directory does.
pub fn directory_holding(root: &Path, name: &str) -> Option<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.file_name().and_then(|n| n.to_str()) == Some(name) {
                return Some(dir);
            }
        }
    }
    None
}

/// The package a generated source of `lang` declares, for the two languages
/// whose machines carry a package header (Go and Kotlin, `package <name>` as
/// its first such line); `None` for the others, which place a machine by
/// namespace, symbol prefix, module path or file (C++, C, Rust, Python,
/// Swift) and declare no package to agree on.
pub fn package_of(lang: &str, source: &str) -> Option<String> {
    match lang {
        "go" | "kotlin" => source
            .lines()
            .find_map(|line| line.trim().strip_prefix("package "))
            .map(|name| name.trim().to_string()),
        "rust" | "cpp" | "c11" | "python" | "swift" => None,
        other => panic!("no package reading for {other}"),
    }
}
