// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A statechart imports the schema of an event or the enum a field is held in with
// `<sce:import>`, and the entry points that parse one document (`sce-codegen
// generate`, `check`) resolve the sibling it names before the typed path is
// judged. A sibling they could not resolve used to be skipped in silence, and the
// typed path then had no schema to read: the author was told that a field of a
// record "is not a member" of it (`expression/unknown-member`, "declared:
// <none>"), pointing at an expression that is right, while the defect was a file
// that was not there or a schema with a typo in it. Measured 2026-10-04.
//
// Each way an import fails now has the diagnostic the build orchestrator's import
// pass gives it, placed where the author can act: a file that is not there, a
// document that is not a Forge kind, or one of another kind than declared, on the
// `<sce:import>` line of the importing document; a sibling that does not parse, at
// its own line, in its own words.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

const SCHEMA_DAY: &str = "sce-build/tests/fixtures/static_datamodel/schema_day.scxml";
const ENUM_VIEW_MODE: &str = "sce-build/tests/fixtures/static_datamodel/enum_view_mode.scxml";

/// A `sce-static` machine holding a record of the schema `schema_day.scxml`, which
/// reads a field of it in an expression that is right.
const IMPORTING: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="showing" datamodel="sce-static" name="importing">
  <sce:import kind="event-schema" src="schema_day.scxml" as="Day"/>
  <datamodel>
    <data id="shown" sce:type="record:Day">
      <sce:set name="year" expr="2026"/>
      <sce:set name="month" expr="9"/>
      <sce:set name="dayOfMonth" expr="24"/>
    </data>
  </datamodel>
  <state id="showing">
    <transition event="next" cond="shown.dayOfMonth &lt; 28" type="internal">
      <assign location="shown.dayOfMonth" expr="shown.dayOfMonth + 1"/>
    </transition>
  </state>
</scxml>
"##;

/// The line (1-based) of the first line of `text` that contains `needle`.
fn line_of(text: &str, needle: &str) -> u64 {
    text.lines()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("`{needle}` is on no line of the text")) as u64
        + 1
}

/// `check -l rust` of IMPORTING with the sibling `schema_day.scxml` as given
/// (`None`: not there), and the first diagnostic it prints.
fn first_diagnostic(sibling: Option<&str>) -> serde_json::Value {
    let dir = tempdir().expect("tempdir");
    if let Some(text) = sibling {
        std::fs::write(dir.path().join("schema_day.scxml"), text).expect("write the sibling");
    }
    let path = dir.path().join("importing.scxml");
    std::fs::write(&path, IMPORTING).expect("write the probe");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["check", "-l", "rust"])
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    assert!(
        !out.status.success(),
        "an import that cannot be resolved must be refused"
    );
    let text =
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout);
    let line = text
        .lines()
        .find(|line| line.starts_with('{'))
        .unwrap_or_else(|| panic!("no diagnostic in:\n{text}"));
    serde_json::from_str(line).unwrap_or_else(|why| panic!("{why}: {line}"))
}

fn read(path: &str) -> String {
    std::fs::read_to_string(repo_root().join(path)).expect("a fixture")
}

#[test]
fn a_sibling_that_is_not_there_is_reported_on_the_import_line() {
    let diagnostic = first_diagnostic(None);
    assert_eq!(diagnostic["code"], "import/file-not-found", "{diagnostic}");
    assert_eq!(
        diagnostic["location"]["line"],
        line_of(IMPORTING, "<sce:import"),
        "{diagnostic}"
    );
    assert!(
        diagnostic["location"]["file"]
            .as_str()
            .is_some_and(|file| file.ends_with("importing.scxml")),
        "the diagnostic is placed in the importing document: {diagnostic}"
    );
}

#[test]
fn a_sibling_that_does_not_parse_is_reported_in_the_sibling() {
    // A typo in a field's type: the author's expression is right, and the file to
    // open is the schema.
    let schema = read(SCHEMA_DAY).replace(
        "id=\"month\" sce:type=\"uint8\"",
        "id=\"month\" sce:type=\"uint9\"",
    );
    let diagnostic = first_diagnostic(Some(&schema));
    assert_eq!(diagnostic["code"], "xml/schema-validation", "{diagnostic}");
    assert!(
        diagnostic["location"]["file"]
            .as_str()
            .is_some_and(|file| file.ends_with("schema_day.scxml")),
        "the diagnostic is placed in the sibling: {diagnostic}"
    );
    assert_eq!(
        diagnostic["location"]["line"],
        line_of(&schema, "uint9"),
        "{diagnostic}"
    );
}

#[test]
fn a_sibling_of_another_kind_than_declared_is_reported_on_the_import_line() {
    let diagnostic = first_diagnostic(Some(&read(ENUM_VIEW_MODE)));
    assert_eq!(diagnostic["code"], "import/kind-mismatch", "{diagnostic}");
    assert_eq!(
        diagnostic["location"]["line"],
        line_of(IMPORTING, "<sce:import"),
        "{diagnostic}"
    );
}

#[test]
fn a_sibling_that_is_no_forge_document_is_reported_on_the_import_line() {
    let plain = "<?xml version=\"1.0\"?>\n<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" version=\"1.0\" initial=\"s\">\n  <state id=\"s\"/>\n</scxml>\n";
    let diagnostic = first_diagnostic(Some(plain));
    assert_eq!(diagnostic["code"], "import/not-forge", "{diagnostic}");
    assert_eq!(
        diagnostic["location"]["line"],
        line_of(IMPORTING, "<sce:import"),
        "{diagnostic}"
    );
}
