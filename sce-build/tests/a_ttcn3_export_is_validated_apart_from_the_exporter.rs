// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Every TTCN-3 export of the fixtures is held by the validator of ADR 0091
//! answer 12 (docs/SCE_TEST_PROFILE.md §9.4), which shares no code with the
//! exporter: it reads the suite's documents, the two modules and the witness
//! with readers of its own and finds no violation of its three checks, the
//! places and entries, the form of each place, and the order of each `alt`
//! and block with the exit and entry sets it computes itself. An export
//! changed in one place, in its modules or in its witness, is found, and
//! named by the code of what changed.

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

/// The suite whose component nests its states: blocks that exit and enter
/// more than one state, an internal transition, a targetless branch and a
/// default entry with content.
fn nested_suite() -> PathBuf {
    manifest_dir().join("tests/fixtures/ttcn3_export/nested_suite.scxml")
}

/// Every suite of the fixtures the export is held to
/// (`a_test_suite_is_exported_to_ttcn3`), the nested suite, and the boundary
/// suite of the lane's corpus (docs/SCE_TTCN3_LANE.md §12), which the lane
/// exports as it exports every suite of the corpus.
fn suites() -> Vec<PathBuf> {
    vec![
        job_suite(),
        manifest_dir().join("tests/fixtures/test_profile/runtime/runtime_suite.scxml"),
        kind_example_suite(),
        manifest_dir().join("tests/fixtures/ttcn3_export/orders_suite.scxml"),
        nested_suite(),
        manifest_dir().join("tests/fixtures/test_profile/boundary/boundary_suite.scxml"),
    ]
}

/// The function of the job suite's tester, and the place of the `alt` of its
/// state `requesting`, whose alternatives are its receive branch and then its
/// timeout branch.
const TESTER: &str = "f_job_tester_in_tester_and_helper";

fn requesting_alt() -> serde_json::Value {
    serde_json::json!([{"function": TESTER}, {"label": "requesting"}, {"statement": 1}])
}

/// `text` with the alternative that begins with `first` and the one after it,
/// which begins with `second`, in the other order. Each alternative of an
/// `alt` at the top of a label stands four spaces in and ends with its own
/// closing brace there.
fn swapped_alternatives(text: &str, first: &str, second: &str) -> String {
    const END: &str = "\n    }\n";
    let start = text.find(first).expect("the first alternative");
    let middle = start + text[start..].find(second).expect("the second alternative");
    let end = middle + text[middle..].find(END).expect("the end of the second") + END.len();
    format!(
        "{}{}{}{}",
        &text[..start],
        &text[middle..end],
        &text[start..middle],
        &text[end..]
    )
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

/// A statement deleted from a block moves the statements after it up one:
/// the last place of the block the witness names is gone, each of the four
/// places after the deleted one holds the statement the witness names the
/// next place by, and the block ends before the statements it owes do.
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
    assert_eq!(
        codes(&found),
        [
            "block-order",
            "form",
            "form",
            "form",
            "form",
            "location-dangling"
        ],
        "{}",
        lines(&found)
    );
}

/// Two alternatives swapped in the text, the witness unchanged: each place
/// holds the alternative of the other branch, which is not the form of the
/// branch its entry names. The order the witness gives is P3's, so it is no
/// `alt-order`.
#[test]
fn alternatives_swapped_in_the_text_are_each_of_another_form() {
    let suite = job_suite();
    let export = exported(&suite);
    let swapped = swapped_alternatives(
        &export.behaviour_module,
        "    [] sut.receive(",
        "    [] deadline.timeout",
    );
    assert_ne!(swapped, export.behaviour_module);
    let found = violations(&suite, &export.types_module, &swapped, &export.witness);
    let codes = codes(&found);
    assert!(codes.contains(&"form"), "{}", lines(&found));
    assert!(!codes.contains(&"alt-order"), "{}", lines(&found));
}

/// Two alternatives swapped in the text and in the witness alike, as an
/// export that wrote its parent's branch first would be: every place has its
/// form, and the order of the alternatives is not P3's.
#[test]
fn alternatives_out_of_the_order_of_p3_are_found() {
    let suite = job_suite();
    let export = exported(&suite);
    let swapped = swapped_alternatives(
        &export.behaviour_module,
        "    [] sut.receive(",
        "    [] deadline.timeout",
    );
    let mut witness = witness_of(&export);
    let alt = requesting_alt();
    let prefix = alt.as_array().expect("a path");
    for entry in witness["entries"].as_array_mut().expect("entries") {
        for location in entry["produced"].as_array_mut().expect("produced") {
            let path = location["path"].as_array_mut().expect("a path");
            if path.len() > prefix.len() && path[..prefix.len()] == prefix[..] {
                let position = path[prefix.len()]["alternative"]
                    .as_u64()
                    .expect("an alternative");
                path[prefix.len()] = serde_json::json!({"alternative": 1 - position});
            }
        }
    }
    let text = serde_json::to_string_pretty(&witness).expect("a witness is JSON");
    let found = violations(&suite, &export.types_module, &swapped, &text);
    assert_eq!(codes(&found), ["alt-order"], "{}", lines(&found));
}

/// An onexit dropped from a block of the nested suite: the block ends before
/// the statements it owes do.
#[test]
fn an_onexit_dropped_from_a_block_is_found() {
    let suite = nested_suite();
    let export = exported(&suite);
    let dropped = export
        .behaviour_module
        .replacen("      log(\"leaving first\");\n", "", 1);
    assert_ne!(
        dropped, export.behaviour_module,
        "the branch of `first` to `second` exits `first`"
    );
    let found = violations(&suite, &export.types_module, &dropped, &export.witness);
    let codes = codes(&found);
    assert!(codes.contains(&"block-order"), "{}", lines(&found));
    assert!(
        codes
            .iter()
            .all(|code| ["block-order", "form", "location-dangling"].contains(code)),
        "{}",
        lines(&found)
    );
}

/// A timer started for another duration than the document's.
#[test]
fn a_timer_started_for_another_duration_is_found() {
    let suite = job_suite();
    let export = exported(&suite);
    let changed = export
        .behaviour_module
        .replacen("settle.start(0.5);", "settle.start(0.25);", 1);
    assert_ne!(
        changed, export.behaviour_module,
        "settling starts its timer for 500ms"
    );
    let found = violations(&suite, &export.types_module, &changed, &export.witness);
    assert_eq!(codes(&found), ["form"], "{}", lines(&found));
}

/// A branch whose witness claims it exits no state, where it exits
/// `requesting`.
#[test]
fn a_claim_of_the_wrong_exit_set_is_found() {
    let found = with_witness_changed(|witness| {
        let mut alternative = requesting_alt();
        alternative
            .as_array_mut()
            .expect("a path")
            .push(serde_json::json!({"alternative": 0}));
        let mut edited = 0;
        for entry in witness["entries"].as_array_mut().expect("entries") {
            for location in entry["produced"].as_array_mut().expect("produced") {
                if location["path"] == alternative {
                    location["claim"]["exited"] = serde_json::json!([]);
                    edited += 1;
                }
            }
        }
        assert_eq!(edited, 1, "the receive branch of `requesting` is one place");
    });
    assert_eq!(codes(&found), ["exit-set"], "{}", lines(&found));
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
