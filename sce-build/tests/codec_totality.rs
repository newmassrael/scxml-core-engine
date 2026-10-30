// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Whether a codec's encode can refuse a value, held against every codec
//! document in the tree (docs/adr/0065, law 1).
//!
//! Each codec states it on its root (`sce:encode-may-fail`), and the build
//! holds the statement to the refusals it finds before any backend renders.
//! This test asks the same of every document at once, so a document whose
//! statement and refusals disagree is named here with every place it refuses,
//! not one at a time at the first backend that renders it.

use std::path::{Path, PathBuf};

use sce_build::{codec_totality_file, ForgeCompileOptions};

/// The directories that hold codec documents, from the repository root.
const ROOTS: &[&str] = &[
    "tests/forge/resources",
    "tests/forge/conformance/broadcast_log_context",
    "backends/rust/forge-conformance/fixtures",
    "stdlib/mesh",
    "sce-build/kind-examples",
];

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn documents() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = ROOTS
        .iter()
        .flat_map(|root| {
            std::fs::read_dir(repo().join(root))
                .unwrap_or_else(|err| panic!("{root}: {err}"))
                .map(|entry| entry.expect("an entry").path())
        })
        .filter(|path| path.extension().is_some_and(|ext| ext == "scxml"))
        .collect();
    paths.sort();
    paths
}

#[test]
fn every_codec_states_its_encode_as_the_build_finds_it() {
    let mut codecs = 0usize;
    let mut may_fail = 0usize;
    let mut disagreements: Vec<String> = Vec::new();
    for path in documents() {
        let Some((model, totality)) =
            codec_totality_file(&path, &[], &ForgeCompileOptions::default())
                .unwrap_or_else(|err| panic!("{}: {err:?}", path.display()))
        else {
            continue;
        };
        codecs += 1;
        if !totality.is_total() {
            may_fail += 1;
        }
        if model.encode_may_fail == totality.is_total() {
            let found = if totality.is_total() {
                "no refusal".to_string()
            } else {
                totality
                    .refusals()
                    .iter()
                    .map(|r| format!("\n    {r}"))
                    .collect::<String>()
            };
            disagreements.push(format!(
                "{}: declares sce:encode-may-fail=\"{}\", and the build finds {found}",
                path.strip_prefix(repo()).unwrap_or(&path).display(),
                model.encode_may_fail,
            ));
        }
    }
    assert!(
        codecs > 100,
        "the roots hold the codec corpus ({codecs} read)"
    );
    assert!(
        disagreements.is_empty(),
        "{} of {codecs} codecs state their encode otherwise than the build finds it \
         ({may_fail} can refuse):\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
}
