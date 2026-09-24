// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An `sce:unresolved` marker is reported where its author wrote it, and
//! every surface lists the markers the build refuses over.
//!
//! # What was wrong
//!
//! Measured 2026-09-24. A marker after an `<xi:include>` was reported on the
//! row of the EXPANDED text — a statechart whose marker sits on row 5 was
//! told row 9, by `sce-codegen unresolved` and by `--strict-unresolved`
//! alike, under the file's basename rather than the path it was named by.
//! The reader records a marker against the expanded text, and neither
//! surface moved it back. On the forge side `unresolved` read the file
//! unexpanded, so a marker an included fragment carried was not listed at
//! all, while `--strict-unresolved` — which reads the expanded text —
//! refused the build over it.
//!
//! # What is held
//!
//! Every fixture puts an include before its marker, so the expanded row and
//! the authored row differ; each expected position is read off the fixture
//! text, not restated. Documents are named by a path with a directory, so a
//! basename in `location.file` is visible as the wrong answer it is.

use std::path::{Path, PathBuf};
use std::process::Command;

fn codegen() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// The row and column `needle` starts at in `text`, counted as the
/// diagnostic contract counts them: from 1, a column per character.
fn written_at(text: &str, needle: &str) -> (u64, u64) {
    let offset = text.find(needle).expect("the fixture spells the needle");
    let before = &text[..offset];
    let row = before.matches('\n').count() as u64 + 1;
    let col = before[before.rfind('\n').map_or(0, |i| i + 1)..]
        .chars()
        .count() as u64
        + 1;
    (row, col)
}

/// `files` written into a fresh directory.
fn fixture(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, body) in files {
        std::fs::write(dir.path().join(name), body).expect("write fixture");
    }
    dir
}

/// Every record `unresolved` lists for `doc`.
fn listed(doc: &Path) -> Vec<serde_json::Value> {
    let run = codegen()
        .args(["unresolved", doc.to_str().unwrap()])
        .output()
        .expect("spawn sce-codegen");
    assert_eq!(
        run.status.code(),
        Some(0),
        "`unresolved` failed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("one JSON record per line"))
        .collect()
}

/// The `validation/unresolved-placeholder` record `route` refuses `doc`
/// with under `--strict-unresolved`.
fn refused(route: &str, doc: &Path, out: &Path) -> serde_json::Value {
    let mut cmd = codegen();
    cmd.args([
        "--error-format=json",
        route,
        doc.to_str().unwrap(),
        "-l",
        "rust",
        "--strict-unresolved",
    ]);
    if route == "generate" {
        cmd.args(["-o", out.to_str().unwrap()]);
    }
    let run = cmd.output().expect("spawn sce-codegen");
    let stderr = String::from_utf8_lossy(&run.stderr).into_owned();
    assert_ne!(
        run.status.code(),
        Some(0),
        "{route} built a marked document"
    );
    stderr
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('{'))
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("JSON record"))
        .find(|record| record["code"] == "validation/unresolved-placeholder")
        .unwrap_or_else(|| panic!("{route} gave no unresolved-placeholder record:\n{stderr}"))
}

fn assert_placed(record: &serde_json::Value, file: &str, at: (u64, u64), what: &str) {
    assert_eq!(record["location"]["file"], file, "{what}: {record}");
    assert_eq!(
        (
            record["location"]["line"].as_u64(),
            record["location"]["col"].as_u64()
        ),
        (Some(at.0), Some(at.1)),
        "{what}: {record}"
    );
}

const STATES_FRAGMENT: &str = r#"<wrap xmlns="http://www.w3.org/2005/07/scxml">
  <state id="a"/>
  <state id="b"/>
  <state id="c"/>
</wrap>
"#;

const STATECHART: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       xmlns:xi="http://www.w3.org/2001/XInclude" version="1.0" initial="s" datamodel="null">
  <xi:include href="states.xml"/>
  <state id="s" sce:unresolved="tbd" sce:unresolved-reason="the source is silent"/>
</scxml>
"#;

#[test]
fn a_statechart_marker_after_an_include_is_placed_where_it_was_written() {
    let dir = fixture(&[("states.xml", STATES_FRAGMENT), ("sc.scxml", STATECHART)]);
    let doc: PathBuf = dir.path().join("sc.scxml");
    let file = doc.to_str().unwrap();
    let at = written_at(STATECHART, "sce:unresolved=");

    let records = listed(&doc);
    assert_eq!(records.len(), 1, "{records:?}");
    assert_placed(&records[0], file, at, "unresolved");

    assert_placed(
        &refused("check", &doc, dir.path()),
        file,
        at,
        "check --strict-unresolved",
    );
}

const INPUTS_FRAGMENT: &str = r#"<fragment xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext">
  <data id="raw" sce:type="int32" sce:direction="in"/>
  <data id="offset" sce:type="int32" sce:direction="in"/>
</fragment>
"#;

const TRANSFORM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       xmlns:xi="http://www.w3.org/2001/XInclude" version="1.0" sce:kind="transform" name="marked">
  <datamodel>
    <xi:include href="inputs.xml"/>
    <data id="scaled" sce:type="int32" sce:direction="out" expr="raw * 2" sce:unresolved="scale-factor"/>
  </datamodel>
</scxml>
"#;

#[test]
fn a_forge_marker_after_an_include_is_placed_where_it_was_written() {
    let dir = fixture(&[("inputs.xml", INPUTS_FRAGMENT), ("marked.scxml", TRANSFORM)]);
    let doc = dir.path().join("marked.scxml");
    let file = doc.to_str().unwrap();
    let at = written_at(TRANSFORM, "sce:unresolved=");

    let records = listed(&doc);
    assert_eq!(records.len(), 1, "{records:?}");
    assert_placed(&records[0], file, at, "unresolved");

    for route in ["check", "generate"] {
        assert_placed(
            &refused(route, &doc, dir.path()),
            file,
            at,
            &format!("{route} --strict-unresolved"),
        );
    }
}

const MARKED_FRAGMENT: &str = r#"<fragment xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext">
  <data id="raw" sce:type="int32" sce:direction="in"/>
  <data id="gain" sce:type="int32" sce:direction="out" expr="raw" sce:unresolved="gain-source"/>
</fragment>
"#;

const TRANSFORM_INCLUDING_A_MARKER: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       xmlns:xi="http://www.w3.org/2001/XInclude" version="1.0" sce:kind="transform" name="included">
  <datamodel>
    <xi:include href="marked_fragment.xml"/>
  </datamodel>
</scxml>
"#;

/// A marker only an included fragment carries is listed — the build
/// refuses over it, so the list that answers "what is unresolved here"
/// names it — and placed in that fragment.
#[test]
fn a_forge_marker_an_include_carries_is_listed_in_its_fragment() {
    let dir = fixture(&[
        ("marked_fragment.xml", MARKED_FRAGMENT),
        ("included.scxml", TRANSFORM_INCLUDING_A_MARKER),
    ]);
    let doc = dir.path().join("included.scxml");
    let at = written_at(MARKED_FRAGMENT, "sce:unresolved=");

    let records = listed(&doc);
    assert_eq!(
        records.len(),
        1,
        "the included marker is not listed: {records:?}"
    );
    assert_eq!(records[0]["id"], "gain-source", "{records:?}");
    let file = records[0]["location"]["file"].as_str().unwrap_or_default();
    assert!(
        file.ends_with("marked_fragment.xml"),
        "the marker is not placed in the fragment: {records:?}"
    );
    assert_placed(&records[0], file, at, "unresolved");

    assert_eq!(
        refused("check", &doc, dir.path())["location"],
        records[0]["location"],
        "the refusal and the list place the one marker differently"
    );
}
