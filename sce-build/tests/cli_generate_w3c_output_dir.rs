// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `generate-w3c` writes where it is told.
//!
//! Inputs were retargetable long before outputs were: `--registry` and
//! `--resources` both take a path, but the generated trees went to
//! `<project root>/backends/<lang>/tests/...` with no way to move them.
//! A repository vendoring SCE could therefore enumerate and validate the
//! conformance suite and then had nowhere to put what it generated —
//! which is most of what a "run the fixtures without SCE's build system"
//! claim has to mean.
//!
//! Two properties make `--output-dir` worth having rather than merely
//! present, and both are asserted below: the run must write under the
//! named root, and it must write *only* there. The second is the one
//! that would rot silently — a backend that kept one path derived from
//! the project root would still look correct in the output directory
//! while quietly rewriting the repository's own tree.

use std::path::{Path, PathBuf};
use std::process::Command;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Three fixtures: the claim is about where output lands, not about how
/// much of it there is, so a small registry keeps the test from
/// regenerating 202 state machines. 216 earns its place by lowering a
/// hybrid child — the generator synthesises that child's SCXML into the
/// output tree, which is the only shape whose provenance line can pick
/// up the output root.
fn stage_registry(dir: &Path) -> PathBuf {
    let full = std::fs::read_to_string(
        repo_root().join(sce_build::w3c_registry::W3C_REGISTRY_RELATIVE_PATH),
    )
    .expect("read the committed registry");
    let mut doc: serde_json::Value = serde_json::from_str(&full).expect("registry is JSON");
    let kept: Vec<serde_json::Value> = doc["fixtures"]
        .as_array()
        .expect("fixtures array")
        .iter()
        .filter(|f| matches!(f["id"].as_str(), Some("144") | Some("147") | Some("216")))
        .cloned()
        .collect();
    assert_eq!(
        kept.len(),
        3,
        "every fixture must still be registered for this test to mean anything",
    );
    doc["fixtures"] = serde_json::Value::Array(kept);
    let path = dir.join("fixtures.json");
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&doc).expect("serialize"),
    )
    .expect("write registry");
    path
}

/// Every regular file under `root`, as paths relative to it.
fn relative_files(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                out.push(
                    path.strip_prefix(root)
                        .expect("under root")
                        .display()
                        .to_string(),
                );
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

#[test]
fn generate_w3c_writes_under_the_named_root_and_not_into_the_repository() {
    let staging = tempfile::tempdir().expect("tempdir");
    let out = tempfile::tempdir().expect("tempdir");
    let registry = stage_registry(staging.path());
    let root = repo_root();

    // The repository's own copy of one of the two fixtures. If any
    // backend path still derived from the project root, this run would
    // rewrite it.
    let in_repo = root.join("backends/rust/tests/src/generated/test144/test144_sm.rs");
    let before = std::fs::read(&in_repo).expect("the committed tree holds test144");

    let output = Command::new(sce_codegen_bin())
        .arg("generate-w3c")
        .arg("-l")
        .arg("rust")
        .arg("--registry")
        .arg(&registry)
        .arg("--resources")
        .arg(root.join("resources"))
        .arg("--output-dir")
        .arg(out.path())
        // Pin the stamp so the run cannot depend on the clock.
        .env("SOURCE_DATE_EPOCH", "0")
        .output()
        .expect("spawn sce-codegen generate-w3c");
    assert!(
        output.status.success(),
        "generation must succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr),
    );

    let written = relative_files(out.path());
    assert_eq!(
        written,
        vec![
            "backends/rust/tests/src/generated/mod.rs".to_string(),
            "backends/rust/tests/src/generated/test144/mod.rs".to_string(),
            // The sourcemap sidecar travels with the machine it
            // describes, so it has to follow the root too — a sidecar
            // left behind in the repository would describe symbols the
            // caller's tree does not contain.
            "backends/rust/tests/src/generated/test144/sce_sourcemap.json".to_string(),
            "backends/rust/tests/src/generated/test144/test144_sm.rs".to_string(),
            "backends/rust/tests/src/generated/test147/mod.rs".to_string(),
            "backends/rust/tests/src/generated/test147/sce_sourcemap.json".to_string(),
            "backends/rust/tests/src/generated/test147/test147_sm.rs".to_string(),
            "backends/rust/tests/src/generated/test216/mod.rs".to_string(),
            "backends/rust/tests/src/generated/test216/sce_sourcemap.json".to_string(),
            // 216 lowers a hybrid child: the generator synthesises the
            // child's SCXML into the output tree and then reads it back,
            // so this input is itself an artifact of the run and has to
            // land under the named root like everything else.
            "backends/rust/tests/src/generated/test216/test216_hybrid0.scxml".to_string(),
            "backends/rust/tests/src/generated/test216/test216_hybrid0_sm.rs".to_string(),
            "backends/rust/tests/src/generated/test216/test216_sm.rs".to_string(),
            "backends/rust/tests/tests/test_144.rs".to_string(),
            "backends/rust/tests/tests/test_147.rs".to_string(),
            "backends/rust/tests/tests/test_216.rs".to_string(),
        ],
        "the backend layout is preserved beneath the named root, and only \
         the registered fixtures are emitted",
    );

    let after = std::fs::read(&in_repo).expect("read the committed tree again");
    assert_eq!(
        before,
        after,
        "`--output-dir` must not write into the repository: \
         {} changed",
        in_repo.display(),
    );
}

/// The generated bytes do not depend on where they were written.
///
/// An absolute output path leaking into a generated file would make the
/// artifact unreproducible — two runs writing to different directories
/// would differ despite identical inputs — and would defeat the
/// `source-hash` drift gate, which is computed over inputs.
#[test]
fn generated_content_does_not_depend_on_the_output_root() {
    let staging = tempfile::tempdir().expect("tempdir");
    let first = tempfile::tempdir().expect("tempdir");
    let second = tempfile::tempdir().expect("tempdir");
    let registry = stage_registry(staging.path());
    let root = repo_root();

    let run = |out: &Path| {
        let output = Command::new(sce_codegen_bin())
            .arg("generate-w3c")
            .arg("-l")
            .arg("rust")
            .arg("--registry")
            .arg(&registry)
            .arg("--resources")
            .arg(root.join("resources"))
            .arg("--output-dir")
            .arg(out)
            .env("SOURCE_DATE_EPOCH", "0")
            .output()
            .expect("spawn sce-codegen generate-w3c");
        assert!(
            output.status.success(),
            "generation must succeed; stderr: {}",
            String::from_utf8_lossy(&output.stderr),
        );
    };
    run(first.path());
    run(second.path());

    let files = relative_files(first.path());
    assert!(
        !files.is_empty(),
        "the comparison is vacuous unless the runs wrote something",
    );
    assert_eq!(
        files,
        relative_files(second.path()),
        "both runs must emit the same file set",
    );
    for rel in &files {
        let a = std::fs::read(first.path().join(rel)).expect("read first");
        let b = std::fs::read(second.path().join(rel)).expect("read second");
        assert_eq!(
            a, b,
            "{rel} differs between two output roots, so its bytes carry the \
             directory it was written to",
        );
    }
}
