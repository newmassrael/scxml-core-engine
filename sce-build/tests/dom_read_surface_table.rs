// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What binds the shared DOM table's two spellings to each other.
//
// `tests/ecmascript/dom_read_surface.json` carries every case twice: the
// author's ECMAScript in `source`, and the Lua the frontend lowers it to
// in `lua`. Seven readers divide by which of the two they are handed —
// the engines that run ECMAScript evaluate `source`, and the four
// backends whose translation happens at build time only ever see `lua` —
// so the two have to be the same expression or the table is measuring
// two different things and calling them one.
//
// The frontend is the oracle for that, not a reader's opinion: this
// lowers `source` and asserts the answer IS `lua`. A case cannot claim a
// lowering the emitter does not produce, and a change to the emitter that
// moves the lowering fails here rather than in four backends at once.

use sce_build::ecmascript::{to_lua_value, DocumentScope};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The table, as `(source, lua, clause)` rows.
fn rows() -> Vec<(String, String, String)> {
    let path = repo_root().join("tests/ecmascript/dom_read_surface.json");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let table: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let cases = table
        .get("cases")
        .and_then(|c| c.as_array())
        .expect("the table has a `cases` array");
    cases
        .iter()
        .map(|case| {
            let read = |key: &str| {
                case.get(key)
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|| panic!("every case has a `{key}`: {case}"))
                    .to_string()
            };
            (read("source"), read("lua"), read("clause"))
        })
        .collect()
}

/// `var1` is the document the table binds; declaring it keeps these
/// probes about the lowering rather than about identifier resolution,
/// which `ecmascript_identifier_scope` owns.
fn scope() -> DocumentScope {
    DocumentScope::declaring(["var1"])
}

#[test]
fn every_case_lowers_to_the_lua_the_table_names() {
    let rows = rows();
    // A floor, not an equality: adding a case must not have to touch this
    // number, but a table that stopped being read must not pass either.
    assert!(
        rows.len() >= 30,
        "the shared DOM table produced only {} case(s), so this is not \
         measuring the surface it claims to",
        rows.len()
    );

    let mut disagreements = Vec::new();
    for (source, lua, clause) in &rows {
        match to_lua_value(source, &scope()) {
            Ok(lowered) if &lowered == lua => {}
            Ok(lowered) => disagreements.push(format!(
                "`{source}` lowers to `{lowered}`, the table says `{lua}` ({clause})"
            )),
            Err(refusal) => disagreements.push(format!(
                "`{source}` was refused by the frontend: {refusal} ({clause})"
            )),
        }
    }
    assert!(
        disagreements.is_empty(),
        "{} of {} rows disagree with the frontend's own lowering:\n{}",
        disagreements.len(),
        rows.len(),
        disagreements.join("\n")
    );
}

/// The table's documents are the ones the readers bind, so a reader that
/// cannot find them is reading a different file.
#[test]
fn the_table_names_the_documents_its_cases_use() {
    let path = repo_root().join("tests/ecmascript/dom_read_surface.json");
    let text = std::fs::read_to_string(&path).expect("read the table");
    let table: serde_json::Value = serde_json::from_str(&text).expect("parse the table");
    let documents = table
        .get("documents")
        .and_then(|d| d.as_object())
        .expect("the table has a `documents` object");
    assert!(
        documents.len() >= 2,
        "one document cannot hold both an element with whitespace between its \
         children and a CDATA section"
    );
    let cases = table
        .get("cases")
        .and_then(|c| c.as_array())
        .expect("cases");
    for case in cases {
        let named = case
            .get("document")
            .and_then(|d| d.as_str())
            .unwrap_or_else(|| panic!("every case names a document: {case}"));
        assert!(
            documents.contains_key(named),
            "case names document `{named}`, which the table does not define"
        );
    }
}
