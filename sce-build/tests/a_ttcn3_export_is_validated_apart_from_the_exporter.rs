// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every TTCN-3 export of the fixtures is held by the validator of ADR 0091
//! answer 12 (docs/SCE_TEST_PROFILE.md §9.4), which shares no code with the
//! exporter: it reads the suite's documents, the two modules and the witness
//! with readers of its own and finds no violation of the first check. An
//! export changed in one place, in its modules or in its witness, is found,
//! and named by the code of what changed.

use std::path::{Path, PathBuf};

use sce_build::ttcn3_export::{export_suite, ExportError, Ttcn3Export};
use sce_ttcn3_validator::{validate, validate_directory, Export, Violation};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn job_suite() -> PathBuf {
    manifest_dir().join("tests/fixtures/test_profile/job_suite.scxml")
}

fn kind_example_suite() -> PathBuf {
    manifest_dir().join("kind-examples/test-suite.scxml")
}

/// Every suite of the fixtures the export is held to
/// (`a_test_suite_is_exported_to_ttcn3`).
fn suites() -> Vec<PathBuf> {
    vec![
        job_suite(),
        manifest_dir().join("tests/fixtures/test_profile/runtime/runtime_suite.scxml"),
        kind_example_suite(),
        manifest_dir().join("tests/fixtures/ttcn3_export/orders_suite.scxml"),
    ]
}

fn exported(path: &Path) -> Ttcn3Export {
    export_suite(path, &[]).unwrap_or_else(|refused| match refused {
        ExportError::Refused(e) => panic!("{} is refused: {e:?}", path.display()),
        ExportError::NotASuite(what) => panic!("{what}"),
    })
}

/// The violations of the export of `suite` whose texts are these.
fn violations(
    suite: &Path,
    types_module: &str,
    behaviour_module: &str,
    witness: &str,
) -> Vec<Violation> {
    validate(
        suite,
        &Export {
            types_module,
            behaviour_module,
            witness,
        },
    )
    .unwrap_or_else(|error| panic!("{} is not validated: {error}", suite.display()))
}

/// The code of each violation, in order of the codes.
fn codes(found: &[Violation]) -> Vec<&'static str> {
    let mut codes: Vec<&'static str> = found.iter().map(Violation::code).collect();
    codes.sort_unstable();
    codes
}

/// Each violation on its line, for a message.
fn lines(found: &[Violation]) -> String {
    found
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

fn witness_of(export: &Ttcn3Export) -> serde_json::Value {
    serde_json::from_str(&export.witness).expect("a witness is JSON")
}

/// The position of the entry of `node` and `rule` among the entries of
/// `witness`.
fn position(witness: &serde_json::Value, node: &str, rule: &str) -> usize {
    witness["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .position(|entry| entry["construct"]["node"] == node && entry["rule"] == rule)
        .unwrap_or_else(|| panic!("no entry ({node}, {rule})"))
}

/// The violations of the job suite's export with its witness changed by
/// `change`.
fn with_witness_changed(change: impl FnOnce(&mut serde_json::Value)) -> Vec<Violation> {
    let suite = job_suite();
    let export = exported(&suite);
    let mut witness = witness_of(&export);
    change(&mut witness);
    let text = serde_json::to_string_pretty(&witness).expect("a witness is JSON");
    violations(
        &suite,
        &export.types_module,
        &export.behaviour_module,
        &text,
    )
}

#[test]
fn every_export_of_the_fixtures_has_no_violation() {
    for suite in suites() {
        let export = exported(&suite);
        let found = violations(
            &suite,
            &export.types_module,
            &export.behaviour_module,
            &export.witness,
        );
        assert!(found.is_empty(), "{}:\n{}", suite.display(), lines(&found));
    }
}

/// The files the export writes are read from their directory, named by the
/// suite's unit name, a comment before each module the header a written
/// module carries.
#[test]
fn an_export_written_to_a_directory_is_validated_from_it() {
    let suite = kind_example_suite();
    let export = exported(&suite);
    let dir = tempfile::tempdir().expect("a scratch directory");
    for (name, text) in export.files() {
        let text = if name.ends_with(".ttcn") {
            format!("// A comment before the module.\n{text}\n")
        } else {
            text
        };
        std::fs::write(dir.path().join(&name), text)
            .unwrap_or_else(|e| panic!("write {name}: {e}"));
    }
    let found = validate_directory(&suite, dir.path())
        .unwrap_or_else(|error| panic!("{} is not validated: {error}", suite.display()));
    assert!(found.is_empty(), "{}", lines(&found));
}

/// A statement deleted from a block moves the statements after it up one,
/// so the last place of the block the witness names is gone.
#[test]
fn a_statement_deleted_from_the_behaviour_module_is_found() {
    let suite = job_suite();
    let export = exported(&suite);
    let deleted = export
        .behaviour_module
        .replacen("      deadline.stop;\n", "", 1);
    assert_ne!(
        deleted, export.behaviour_module,
        "the receive branch of `requesting` stops the deadline"
    );
    let found = violations(&suite, &export.types_module, &deleted, &export.witness);
    assert_eq!(codes(&found), ["location-dangling"], "{}", lines(&found));
}

/// A statement the export does not write is a parse violation of its
/// module.
#[test]
fn a_construct_outside_the_exported_subset_is_a_parse_violation() {
    let suite = job_suite();
    let export = exported(&suite);
    let changed = export
        .behaviour_module
        .replacen("goto settling;", "repeat;", 1);
    assert_ne!(changed, export.behaviour_module);
    let found = violations(&suite, &export.types_module, &changed, &export.witness);
    assert_eq!(codes(&found), ["parse"], "{}", lines(&found));
}

/// A location the witness names twice.
#[test]
fn a_place_the_witness_names_twice_is_found() {
    let found = with_witness_changed(|witness| {
        let at = position(witness, "state 'settling'", "E6");
        let produced = witness["entries"][at]["produced"]
            .as_array_mut()
            .expect("produced");
        let label = produced[0].clone();
        produced.push(label);
    });
    assert_eq!(codes(&found), ["place-named-twice"], "{}", lines(&found));
}

/// An entry dropped from the witness leaves its construct without it and
/// its place named by none.
#[test]
fn an_entry_dropped_from_the_witness_is_found() {
    let found = with_witness_changed(|witness| {
        let at = position(witness, "state 'settling'", "E6");
        witness["entries"]
            .as_array_mut()
            .expect("entries")
            .remove(at);
    });
    assert_eq!(
        codes(&found),
        ["construct-missing", "place-unnamed"],
        "{}",
        lines(&found)
    );
}

/// An entry that names a node the documents do not hold: the construct it
/// names is no construct, and the one it stood for has no entry.
#[test]
fn an_entry_whose_node_is_renamed_is_found() {
    let found = with_witness_changed(|witness| {
        let at = position(witness, "state 'settling'", "E7");
        witness["entries"][at]["construct"]["node"] = serde_json::json!("state 'settled'");
    });
    assert_eq!(
        codes(&found),
        ["construct-dangling", "construct-missing"],
        "{}",
        lines(&found)
    );
}

/// An entry written twice.
#[test]
fn an_entry_written_twice_is_found() {
    let found = with_witness_changed(|witness| {
        let at = position(witness, "sce:test-timer 'settle'", "E4");
        let entries = witness["entries"].as_array_mut().expect("entries");
        let entry = entries[at].clone();
        entries.insert(at, entry);
    });
    // The two entries name one place twice, as well as being one entry
    // twice.
    assert_eq!(
        codes(&found),
        ["entry-duplicate", "place-named-twice"],
        "{}",
        lines(&found)
    );
}
