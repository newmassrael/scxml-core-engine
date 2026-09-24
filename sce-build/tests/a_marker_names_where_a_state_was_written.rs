// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The SCE-MAP markers in generated code name where each state was
//! written, across an `<xi:include>`.
//!
//! # What was wrong
//!
//! The templates stamp a marker from the position each model node carries,
//! and the model records positions against the EXPANDED text. A state an
//! include spliced in was marked in the including file at the row the
//! splice put it on, and a state after the include rows past its own — the
//! same fault the sourcemap had (`a_sourcemap_names_where_a_state_was_written`),
//! reached through the copy every backend renders from.
//!
//! # What is held
//!
//! Every Rust marker that names a state names the file and row holding
//! that state's start tag — read off the fixtures, not restated. Each state
//! writes its transition on the same row, so a transition's marker is held
//! to that row too.

use std::process::Command;

// Each state carries entry actions and a transition with actions, because
// the Rust templates mark a state's body and a transition's arm only when
// there is code to put there; a bare state is marked at the machine alone.
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
fn row_of(text: &str, needle: &str) -> usize {
    let offset = text.find(needle).expect("the fixture spells the needle");
    text[..offset].matches('\n').count() + 1
}

#[test]
fn every_state_marker_names_the_file_and_row_that_wrote_it() {
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

    let mut markers = Vec::new();
    for entry in std::fs::read_dir(&out).expect("the output directory") {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).expect("a generated file");
            markers.extend(
                text.lines()
                    .filter_map(|line| line.trim().strip_prefix("// SCE-MAP: "))
                    .map(str::to_string),
            );
        }
    }

    let expected = [
        (
            "a",
            format!("states.xml:{}", row_of(FRAGMENT, "<state id=\"a\"")),
        ),
        (
            "b",
            format!("states.xml:{}", row_of(FRAGMENT, "<state id=\"b\"")),
        ),
        (
            "c",
            format!("states.xml:{}", row_of(FRAGMENT, "<state id=\"c\"")),
        ),
        ("s", format!("sc.scxml:{}", row_of(HOST, "<state id=\"s\""))),
    ];
    let mut wrong = Vec::new();
    for (state, place) in &expected {
        let naming: Vec<&String> = markers
            .iter()
            .filter(|m| m.contains(&format!(" :: {state} :: ")))
            .collect();
        // Floor: its body's marker and its transition's. A state with no
        // marker at all would pass the check below.
        if naming.len() < 2 {
            wrong.push(format!(
                "{state}: {} marker(s) name it, want its body's and its transition's",
                naming.len()
            ));
        }
        for marker in naming {
            if !marker.starts_with(&format!("{place} ")) {
                wrong.push(format!("{state}: want {place}, got `{marker}`"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{}\nall markers:\n{}",
        wrong.join("\n"),
        markers.join("\n")
    );
}
