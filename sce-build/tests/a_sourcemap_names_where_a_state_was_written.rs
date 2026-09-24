// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The sourcemap names where each state was written, across an
//! `<xi:include>`.
//!
//! # What was wrong
//!
//! The symbol table the sourcemap is built from took each node's position
//! from the model, which records it against the EXPANDED text. A state an
//! include spliced in was recorded in the including file at the row the
//! splice put it on, and a state after the include rows past its own
//! (measured 2026-09-24: states written on rows 2-4 of the fragment were
//! recorded on rows 5-7 of the host, and the host's row-5 state on row 9).
//!
//! # What is held
//!
//! Every state's `scxml_file` and `line_range` are the file and row that
//! hold its start tag — read off the fixtures, not restated. The file is the
//! basename, as an artifact spells one.

use std::process::Command;

const FRAGMENT: &str = r#"<wrap xmlns="http://www.w3.org/2005/07/scxml">
  <state id="a"/>
  <state id="b"/>
  <state id="c"/>
</wrap>
"#;

const HOST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:xi="http://www.w3.org/2001/XInclude" version="1.0" initial="s" datamodel="null">
  <xi:include href="states.xml"/>
  <state id="s"/>
</scxml>
"#;

/// The 1-based row `needle` starts on in `text`.
fn row_of(text: &str, needle: &str) -> u64 {
    let offset = text.find(needle).expect("the fixture spells the needle");
    text[..offset].matches('\n').count() as u64 + 1
}

#[test]
fn every_state_is_mapped_to_the_file_and_row_that_wrote_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("states.xml"), FRAGMENT).expect("write fragment");
    let doc = dir.path().join("sc.scxml");
    std::fs::write(&doc, HOST).expect("write host");
    let out = dir.path().join("out");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args([
            "generate",
            doc.to_str().unwrap(),
            "-l",
            "rust",
            "-o",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(
        run.status.code(),
        Some(0),
        "generate failed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let sourcemap: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(out.join("sce_sourcemap.json")).expect("the sourcemap"),
    )
    .expect("the sourcemap is JSON");

    let expected = [
        ("a", "states.xml", row_of(FRAGMENT, "<state id=\"a\"")),
        ("b", "states.xml", row_of(FRAGMENT, "<state id=\"b\"")),
        ("c", "states.xml", row_of(FRAGMENT, "<state id=\"c\"")),
        ("s", "sc.scxml", row_of(HOST, "<state id=\"s\"")),
    ];
    let symbols = sourcemap["symbols"].as_object().expect("a symbol table");
    let mut wrong = Vec::new();
    for (state, file, row) in expected {
        let symbol = symbols
            .values()
            .find(|s| s["kind"] == "state" && s["scxml_state_path"] == state)
            .unwrap_or_else(|| panic!("no symbol for state {state}: {sourcemap}"));
        if symbol["scxml_file"] != file || symbol["line_range"] != serde_json::json!([row, row]) {
            wrong.push(format!("{state}: want {file}:{row}, got {symbol}"));
        }
    }
    let machine = symbols
        .values()
        .find(|s| s["kind"] == "machine")
        .expect("the machine symbol");
    let row = row_of(HOST, "<scxml ");
    if machine["scxml_file"] != "sc.scxml" || machine["line_range"] != serde_json::json!([row, row])
    {
        wrong.push(format!("machine: want sc.scxml:{row}, got {machine}"));
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
