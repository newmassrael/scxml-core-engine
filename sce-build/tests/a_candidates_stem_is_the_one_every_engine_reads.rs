// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A hybrid `<invoke>` starts the declared candidate its `srcexpr` value
//! names, matched by document stem (docs/SCE_ACCEPTED_SUBSET.md §2.13). The
//! build reads the stem of each declared candidate and a generated runtime
//! reads the stem of the value, so the two readers must agree on every
//! spelling of one document — a mismatch is a value that names a candidate
//! the runtime then fails to find.
//!
//! `tests/document_stem/document_stem.json` is the one table: each runtime
//! measures its `DocumentStem` against it, and this measures the build's.

use sce_build::model::{document_stem, InvokeCandidate};
use std::path::Path;

fn cases() -> Vec<(String, String, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .join("tests/document_stem/document_stem.json");
    let text = std::fs::read_to_string(&path).expect("the shared document-stem table");
    let table: serde_json::Value = serde_json::from_str(&text).expect("the table is JSON");
    let field = |case: &serde_json::Value, key: &str| {
        case[key]
            .as_str()
            .unwrap_or_else(|| panic!("a case's {key} is text: {case}"))
            .to_string()
    };
    table["cases"]
        .as_array()
        .expect("the table has cases")
        .iter()
        .map(|case| {
            (
                field(case, "name"),
                field(case, "value"),
                field(case, "stem"),
            )
        })
        .collect()
}

#[test]
fn the_build_reduces_a_value_to_the_one_stem_every_engine_reduces_it_to() {
    let cases = cases();
    assert!(cases.len() >= 15, "the table lost cases: {}", cases.len());
    for (name, value, stem) in cases {
        assert_eq!(document_stem(&value), stem, "{name}: {value:?}");
    }
}

#[test]
fn a_candidate_is_named_by_the_stem_a_value_of_its_path_reduces_to() {
    // Every entry of `sce:candidates` that names a document is the candidate
    // whose stem the table gives for it; one that names none is no candidate.
    for (name, value, stem) in cases() {
        match InvokeCandidate::from_path(&value) {
            Some(candidate) => {
                assert_eq!(candidate.stem, stem, "{name}: {value:?}");
                assert_eq!(candidate.path, value, "{name}: the path is kept as written");
            }
            None => assert!(
                stem.is_empty(),
                "{name}: {value:?} has the stem {stem:?}, so it names a document"
            ),
        }
    }
}

#[test]
fn an_entry_that_names_no_document_is_no_candidate() {
    for entry in ["", ".", "..", "/", "dir/", "file:", "..\\"] {
        assert!(
            InvokeCandidate::from_path(entry).is_none(),
            "{entry:?} names no document"
        );
    }
}
