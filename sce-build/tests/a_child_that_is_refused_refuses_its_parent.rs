// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A statechart reads the child an `<invoke src>` names when it is parsed, for
// what the child declares and whether it runs a script engine. A child it
// could not read was turned into a child that needs a script engine, and the
// child's own refusal was dropped (found 2026-11-13): the author of a child
// with a parse error, or an import that resolves nowhere, was told the child
// needs a script engine, and a parent that holds no engine was refused for
// that.
//
// A child that is there and is refused now refuses its parent with the child's
// own record: its code, at its own row in its own file, with the parent's
// `src` as the site it was reached from (SCE_ERROR_CONTRACT §2.4,
// `reached-from`). A child that is read and runs a script engine is still the
// cause it was.

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

/// A statechart that starts the child `child.scxml` in its first state.
const PARENT: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="running"
       datamodel="ecmascript" name="parent">
  <state id="running">
    <invoke id="kid" type="http://www.w3.org/TR/scxml/" src="child.scxml"/>
    <transition event="done.invoke.kid" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"##;

/// A child the XML grammar refuses: a comment stands above the XML
/// declaration, which has to be the first thing in the file.
const CHILD_WITH_A_SYNTAX_ERROR: &str = r##"<!-- a comment above the declaration -->
<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c">
  <final id="c"/>
</scxml>
"##;

/// A child that imports the schema `schema_day.scxml`, which is not beside it.
const CHILD_WITH_AN_UNRESOLVED_IMPORT: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="showing" datamodel="sce-static">
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

/// A child whose `<data>` is initialised by an expression its engine
/// evaluates: a child that is read and runs a script engine.
const CHILD_THAT_RUNS_AN_ENGINE: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="c"
       datamodel="ecmascript">
  <datamodel><data id="n" expr="1 + 1"/></datamodel>
  <final id="c"/>
</scxml>
"##;

/// The line (1-based) of the first line of `text` that contains `needle`.
fn line_of(text: &str, needle: &str) -> u64 {
    text.lines()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("`{needle}` is on no line of the text")) as u64
        + 1
}

/// What `check -l rust` of PARENT, beside `child`, ended with.
struct Checked {
    succeeded: bool,
    stdout: String,
    stderr: String,
}

impl Checked {
    /// Every diagnostic record on stderr.
    fn records(&self) -> Vec<serde_json::Value> {
        self.stderr
            .lines()
            .filter(|line| line.starts_with('{'))
            .map(|line| serde_json::from_str(line).unwrap_or_else(|why| panic!("{why}: {line}")))
            .collect()
    }
}

fn check(child: &str) -> Checked {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("child.scxml"), child).expect("write the child");
    let path = dir.path().join("parent.scxml");
    std::fs::write(&path, PARENT).expect("write the parent");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["check", "-l", "rust"])
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    Checked {
        succeeded: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// The parent's refusal for `child`: the run failed, wrote no manifest, said
/// nothing of a script engine, and printed one record, which is returned.
fn refusal(child: &str) -> serde_json::Value {
    let checked = check(child);
    assert!(
        !checked.succeeded,
        "a child that is refused refuses its parent:\n{}\n{}",
        checked.stdout, checked.stderr
    );
    assert!(
        checked.stdout.trim().is_empty(),
        "a refused run writes no manifest: {}",
        checked.stdout
    );
    assert!(
        !checked.stderr.to_lowercase().contains("script engine")
            && !checked.stderr.contains("child-invoke-needs-script-engine"),
        "the child's refusal is not a script-engine requirement: {}",
        checked.stderr
    );
    let mut records = checked.records();
    assert_eq!(records.len(), 1, "one refusal: {}", checked.stderr);
    records.remove(0)
}

/// `record` is placed in the child, on `line`, and names the parent's `src` as
/// the one site it was reached from.
fn assert_placed_in_the_child_and_reached_from_the_parent(record: &serde_json::Value, line: u64) {
    assert!(
        record["location"]["file"]
            .as_str()
            .is_some_and(|file| file.ends_with("child.scxml")),
        "the record is placed in the child: {record}"
    );
    assert_eq!(record["location"]["line"], line, "{record}");
    let related = record["related"]
        .as_array()
        .unwrap_or_else(|| panic!("the route rides along: {record}"));
    assert_eq!(related.len(), 1, "{record}");
    assert_eq!(related[0]["role"], "reached-from", "{record}");
    assert!(
        related[0]["location"]["file"]
            .as_str()
            .is_some_and(|file| file.ends_with("parent.scxml")),
        "reached from the parent: {record}"
    );
    assert_eq!(
        related[0]["location"]["line"],
        line_of(PARENT, "src=\"child.scxml\""),
        "at the parent's <invoke src>: {record}"
    );
    assert_eq!(related[0]["actual"], "child.scxml", "{record}");
}

#[test]
fn a_child_with_a_syntax_error_is_reported_in_the_child() {
    let record = refusal(CHILD_WITH_A_SYNTAX_ERROR);
    assert_eq!(record["code"], "xml/parse", "{record}");
    assert_placed_in_the_child_and_reached_from_the_parent(
        &record,
        line_of(CHILD_WITH_A_SYNTAX_ERROR, "<?xml"),
    );
}

#[test]
fn a_child_with_an_unresolved_import_is_reported_on_its_import_line() {
    let record = refusal(CHILD_WITH_AN_UNRESOLVED_IMPORT);
    assert_eq!(record["code"], "import/file-not-found", "{record}");
    assert_placed_in_the_child_and_reached_from_the_parent(
        &record,
        line_of(CHILD_WITH_AN_UNRESOLVED_IMPORT, "<sce:import"),
    );
}

/// The control: a child that is read and runs a script engine still makes its
/// parent need one, for that cause, at the parent's `<invoke>`.
#[test]
fn a_child_that_runs_a_script_engine_is_still_that_cause() {
    let checked = check(CHILD_THAT_RUNS_AN_ENGINE);
    assert!(
        checked.succeeded,
        "a child that runs an engine is no refusal:\n{}\n{}",
        checked.stdout, checked.stderr
    );
    assert!(
        checked.records().is_empty(),
        "nothing is refused: {}",
        checked.stderr
    );
    let line = checked
        .stdout
        .lines()
        .find(|line| line.trim_start().starts_with('{'))
        .unwrap_or_else(|| panic!("no manifest on stdout: {}", checked.stdout));
    let manifest: serde_json::Value =
        serde_json::from_str(line).unwrap_or_else(|why| panic!("{why}: {line}"));
    assert_eq!(manifest["needs_script_engine"], true, "{manifest}");
    let causes = manifest["script_engine_causes"]
        .as_array()
        .unwrap_or_else(|| panic!("the causes are listed: {manifest}"));
    assert!(
        causes
            .iter()
            .any(|cause| cause["kind"] == "child-invoke-needs-script-engine"
                && cause["invoke"] == "kid"
                && cause["location"]["line"] == line_of(PARENT, "<invoke")),
        "the child's engine is the parent's cause, at its <invoke>: {manifest}"
    );
}
