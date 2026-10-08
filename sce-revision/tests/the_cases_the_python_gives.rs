// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The judgment of a revision is the Python's, case by case and sentence by sentence.
//!
//! `sce-build/tests/fixtures/revision_judgment/cases.json` is written by
//! `tools/authoring/eval/revision_judgment_cases.py` from the Python reference, and
//! `tools/authoring/tests/test_the_revision_judgment_cases_are_the_ones_python_gives.py` holds that
//! the file is what the Python gives now. This holds the other half: that this crate gives the same
//! result, or the same refusal in the same words, for every case. A refusal in other words has
//! changed what a person is told, so the sentence is compared and not only the fact of a refusal.

use sce_revision::{
    belongs_to, belongs_to_list, between, extends, join, parse, render, LineageError,
};
use serde_json::{json, Value};

const CASES: &str = include_str!("../../sce-build/tests/fixtures/revision_judgment/cases.json");

fn cases() -> Value {
    serde_json::from_str(CASES).expect("the cases are JSON")
}

/// A verdict that returns nothing, as the cases write it.
fn verdict(result: Result<(), LineageError>) -> Value {
    match result {
        Ok(()) => json!("ok"),
        Err(error) => json!({ "refused": error.message() }),
    }
}

/// A verdict that returns a value, as the cases write it.
fn outcome(result: Result<Value, LineageError>) -> Value {
    match result {
        Ok(value) => json!({ "ok": value }),
        Err(error) => json!({ "refused": error.message() }),
    }
}

/// Run `judge` on each case of `section` and say which gave another result than the Python.
fn held_to_the_python(section: &str, judge: impl Fn(&Value, &Value) -> Value) {
    let all = cases();
    let table = &all["lineages"];
    let section_cases = all[section].as_array().expect("a section is a list");
    assert!(!section_cases.is_empty(), "{section} has no case");
    let wrong: Vec<String> = section_cases
        .iter()
        .filter_map(|case| {
            let given = judge(case, table);
            (given != case["expect"]).then(|| {
                format!(
                    "{}: expected {}, got {given}",
                    case["name"].as_str().unwrap_or("?"),
                    case["expect"]
                )
            })
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "{section}: {} of {} case(s) differ from the Python\n{}",
        wrong.len(),
        section_cases.len(),
        wrong.join("\n")
    );
}

fn lineage<'a>(case: &'a Value, key: &str, table: &'a Value) -> &'a Value {
    let name = case[key].as_str().expect("a case names its lineage");
    assert!(!table[name].is_null(), "{name} is not in the lineage table");
    &table[name]
}

#[test]
fn reading_a_lineage_refuses_what_the_python_refuses_in_its_words() {
    held_to_the_python("parse", |case, _| {
        let text = case["text"].as_str().expect("a case has its text");
        verdict(parse(text).map(|_| ()))
    });
}

#[test]
fn a_lineage_that_does_not_continue_the_works_is_refused_in_the_pythons_words() {
    held_to_the_python("extends", |case, table| {
        verdict(extends(
            lineage(case, "previous", table),
            lineage(case, "following", table),
        ))
    });
}

#[test]
fn a_lineage_is_the_lineage_of_its_list_or_is_refused_in_the_pythons_words() {
    held_to_the_python("belongs_to_list", |case, table| {
        verdict(belongs_to_list(
            lineage(case, "lineage", table),
            case["manifest_text"].as_str().expect("a manifest text"),
            case["sidecar_text"].as_str(),
        ))
    });
}

#[test]
fn a_delta_belongs_to_the_record_of_its_revision_or_is_refused_in_the_pythons_words() {
    held_to_the_python("belongs_to", |case, _| {
        verdict(belongs_to(&case["delta"], &case["record"]))
    });
}

#[test]
fn the_words_joined_with_the_evidence_are_the_pythons_row_for_row() {
    held_to_the_python("join", |case, _| {
        outcome(join(&case["words"], &case["evidence"]))
    });
}

#[test]
fn the_page_an_owner_reads_is_the_pythons_character_for_character() {
    held_to_the_python("render", |case, _| {
        let sentences = Some(&case["sentences"]).filter(|sentences| !sentences.is_null());
        json!(render(
            &case["result"],
            sentences,
            case["title"].as_str().unwrap_or_default()
        ))
    });
}

#[test]
fn what_became_of_the_words_between_two_states_is_the_pythons() {
    held_to_the_python("between", |case, table| {
        outcome(between(
            lineage(case, "older", table),
            lineage(case, "newer", table),
        ))
    });
}
