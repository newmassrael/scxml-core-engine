// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The `--emit-ast` envelope places each node where it was written, across
//! an `<xi:include>`.
//!
//! # What was wrong
//!
//! The envelope serialised the model as parsed, and the model records
//! positions against the EXPANDED text. A state an include spliced in was
//! placed in the including file at the row the splice put it on, and every
//! node after the include rows past its own (measured 2026-09-24: states
//! written on rows 2-4 of the fragment at `sc.scxml:5-7`, the host's row-5
//! state at row 9) — while `docs/SCE_FORGE_AST.md` §10 names the line and
//! column as the part an IDE consumer relies on.
//!
//! # What is held
//!
//! Every position in the envelope is found by its shape, not by a list of
//! fields, and each must name the row that writes it: a state and
//! everything inside it are written on one row, so each of those rows is
//! read off the fixtures rather than restated.

use std::process::Command;

const FRAGMENT: &str = r#"<wrap xmlns="http://www.w3.org/2005/07/scxml">
  <state id="a"><onentry><raise event="x"/></onentry><transition event="go" target="b"><raise event="y"/></transition></state>
  <state id="b"><onentry><raise event="x"/></onentry><transition event="go" target="c"><raise event="y"/></transition></state>
  <state id="c"><onentry><raise event="x"/></onentry><transition event="go" target="s"><raise event="y"/></transition></state>
</wrap>
"#;

const HOST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:xi="http://www.w3.org/2001/XInclude" version="1.0" initial="a" datamodel="null">
  <xi:include href="states.xml"/>
  <state id="s"><onentry><raise event="x"/></onentry><transition event="go" target="a"><raise event="y"/></transition></state>
</scxml>
"#;

/// The 1-based row `needle` starts on in `text`.
fn row_of(text: &str, needle: &str) -> u64 {
    let offset = text.find(needle).expect("the fixture spells the needle");
    text[..offset].matches('\n').count() as u64 + 1
}

/// Every `(json path, file, line)` in `value`, found by shape.
fn positions(value: &serde_json::Value, path: &str, out: &mut Vec<(String, String, u64)>) {
    match value {
        serde_json::Value::Object(map) => {
            if let (Some(file), Some(line)) = (
                map.get("file").and_then(|f| f.as_str()),
                map.get("line").and_then(|l| l.as_u64()),
            ) {
                out.push((path.to_string(), file.to_string(), line));
            }
            for (key, child) in map {
                positions(child, &format!("{path}.{key}"), out);
            }
        }
        serde_json::Value::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                positions(child, &format!("{path}[{i}]"), out);
            }
        }
        _ => {}
    }
}

#[test]
fn every_position_in_the_envelope_names_the_row_that_wrote_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("states.xml"), FRAGMENT).expect("write fragment");
    let doc = dir.path().join("sc.scxml");
    std::fs::write(&doc, HOST).expect("write host");
    let out = dir.path().join("out");
    let ast = dir.path().join("sc.ast.json");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args([
            "generate",
            doc.to_str().unwrap(),
            "-l",
            "rust",
            "-o",
            out.to_str().unwrap(),
            "--emit-ast",
            ast.to_str().unwrap(),
        ])
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(
        run.status.code(),
        Some(0),
        "generate failed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let envelope: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&ast).expect("the envelope"))
            .expect("the envelope is JSON");

    let mut found = Vec::new();
    positions(&envelope, "", &mut found);

    // Where each state's subtree was written, keyed by its JSON path.
    let places = [
        (
            ".states.a",
            "states.xml",
            row_of(FRAGMENT, "<state id=\"a\""),
        ),
        (
            ".states.b",
            "states.xml",
            row_of(FRAGMENT, "<state id=\"b\""),
        ),
        (
            ".states.c",
            "states.xml",
            row_of(FRAGMENT, "<state id=\"c\""),
        ),
        (".states.s", "sc.scxml", row_of(HOST, "<state id=\"s\"")),
    ];
    let root_row = row_of(HOST, "<scxml ");

    // Floor: each state, its entry action, its transition and the
    // transition's action. A walk that reached nothing passes nothing.
    for (key, _, _) in &places {
        let under = found
            .iter()
            .filter(|(path, _, _)| path.contains(&format!("{key}.")))
            .count();
        assert!(
            under >= 4,
            "only {under} position(s) under {key}: {found:?}"
        );
    }

    let wrong: Vec<_> = found
        .iter()
        .filter(|(path, file, line)| {
            match places
                .iter()
                .find(|(key, _, _)| path.contains(&format!("{key}.")))
            {
                Some((_, want_file, want_row)) => file != want_file || line != want_row,
                None => file != "sc.scxml" || *line != root_row,
            }
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "position(s) not where they were written: {wrong:?}"
    );
}
