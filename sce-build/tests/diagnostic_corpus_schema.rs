// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Instance-validation gate for the diagnostic wire surface.
//
// `schemas/sce-diagnostic.v1.schema.json` is the contract an external
// consumer validates SCE's stderr against. Everything else guarding it
// compares one declaration to another: the enum-drift guard compares
// the schema's `code` list to `ALL_DIAGNOSTIC_CODES`, the status guard
// compares a header string to a Rust constant. Neither runs a record
// through a validator, so the producer is free to drift out of its own
// published schema with every one of them green.
//
// `diagnostic.rs::tests::every_golden_record_validates_against_the_wire_schema`
// closes half of that: it validates the golden table, which
// `every_code_has_a_golden` proves reaches every `DiagnosticCode`.
// It cannot close this half. Goldens are string literals authored by
// hand, so they say nothing about framing — whether stderr is one JSON
// object per line — and nothing about the values the pipeline actually
// fills in, as opposed to the ones a test author typed. Measured: the
// records this sweep collects share no `id` with any golden.
//
// This target is separate from `error_format_json` because its input
// set is the whole fixture tree, which makes it a tree-wide gate; as
// its own target the unfiltered workflow runs one test rather than
// that file's whole suite. See `workflow_trigger_coverage.rs`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::language_wire_name;
use sce_build::generator::Language;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// Tracked fixture documents, as absolute paths.
///
/// Tracked rather than walked: the fixture tree accumulates build
/// output, and a document nobody committed is not part of the corpus
/// this gate speaks for.
fn tracked_fixture_documents() -> Vec<PathBuf> {
    let root = repo_root();
    let out = Command::new("git")
        .args(["ls-files", "-z", "sce-build/tests/fixtures"])
        .current_dir(&root)
        .output()
        .expect("git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    out.stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .filter(|rel| rel.ends_with(".scxml"))
        .map(|rel| root.join(rel))
        .collect()
}

/// Lower bounds. A sweep that reaches nothing satisfies every
/// per-record assertion vacuously, so the corpus size, the record
/// count, and the spread of codes are all asserted rather than assumed.
const MIN_CORPUS_DOCUMENTS: usize = 60;
const MIN_CORPUS_RECORDS: usize = 100;
const MIN_CORPUS_DISTINCT_CODES: usize = 10;

/// Every diagnostic the CLI emits over the fixture corpus is valid
/// against `schemas/sce-diagnostic.v1.schema.json`.
///
/// The sweep runs `check`, which drives the full pipeline and writes
/// nothing, so the corpus can be swept in every backend without
/// materialising one artifact.
#[test]
fn every_cli_diagnostic_in_the_fixture_corpus_validates_against_the_schema() {
    let schema_value: serde_json::Value =
        serde_json::from_str(include_str!("../../schemas/sce-diagnostic.v1.schema.json"))
            .expect("diagnostic schema is valid JSON");
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema_value)
        .expect("diagnostic schema compiles as draft-07");

    let documents = tracked_fixture_documents();
    assert!(
        documents.len() >= MIN_CORPUS_DOCUMENTS,
        "corpus holds only {} documents; expected at least \
         {MIN_CORPUS_DOCUMENTS}. A sweep over nothing certifies nothing.",
        documents.len(),
    );

    let root = repo_root();
    let mut violations: Vec<String> = Vec::new();
    let mut records = 0usize;
    let mut codes: BTreeSet<String> = BTreeSet::new();

    for doc in &documents {
        // Every backend, from the one list of them: a seventh backend
        // widens this sweep without an edit here.
        for lang in Language::ALL {
            let wire = language_wire_name(*lang);
            let out = Command::new(sce_codegen_bin())
                .arg("check")
                .arg(doc)
                .arg("-l")
                .arg(wire)
                .arg("--error-format")
                .arg("json")
                .current_dir(&root)
                .output()
                .expect("invoke sce-codegen check");
            let stderr = String::from_utf8_lossy(&out.stderr);
            for line in stderr.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                // Framing first: one record per line. A pretty-printed
                // record fails here, and no golden could.
                let instance: serde_json::Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(e) => {
                        violations.push(format!(
                            "\n[{} / {wire}] stderr line is not one JSON object ({e}):\n  {line}",
                            doc.display(),
                        ));
                        continue;
                    }
                };
                records += 1;
                if let Some(code) = instance.get("code").and_then(|c| c.as_str()) {
                    codes.insert(code.to_string());
                }
                // Drained into owned strings inside the match so the
                // borrow the error iterator holds on `instance` ends
                // before `instance` leaves scope.
                let msgs: Vec<String> = match validator.validate(&instance) {
                    Ok(()) => Vec::new(),
                    Err(errors) => errors.map(|e| e.to_string()).collect(),
                };
                if !msgs.is_empty() {
                    violations.push(format!("\n[{} / {wire}] {msgs:?}\n  {line}", doc.display()));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "{} CLI diagnostics violate schemas/sce-diagnostic.v1.schema.json \
         (of {records} records over {} documents):{}",
        violations.len(),
        documents.len(),
        violations
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join(""),
    );
    assert!(
        records >= MIN_CORPUS_RECORDS,
        "swept only {records} diagnostics; expected at least \
         {MIN_CORPUS_RECORDS}. The corpus stopped producing errors, \
         which makes this gate vacuous.",
    );
    assert!(
        codes.len() >= MIN_CORPUS_DISTINCT_CODES,
        "swept only {} distinct codes ({codes:?}); expected at least \
         {MIN_CORPUS_DISTINCT_CODES}",
        codes.len(),
    );
}
