// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: a `<foreach>` over a list variable
// (docs/SCE_ACCEPTED_SUBSET.md §2.15, W3C SCXML 4.6).
//
// The loop's variables are the body's own — the item typed as the list's
// element, the index a `uint32` — and the generated code declares them in
// every backend under the spelling that backend gives a local, the one every
// expression of the body reads them by, so each name has to be one a backend
// can declare and none in scope can already mean. Each refusal here is one a
// loop would otherwise have walked past into a generated file that does not
// compile, or compiles and means something else.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::emitting;
use sce_build::forge::model::ForgeKind;
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

/// A statechart with a `list<uint8>` `picked`, a `uint32` `total`, a `bool`
/// `flag` and the given `body` in one transition.
fn machine(body: &str) -> String {
    format!(
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <datamodel>
    <data id="picked" sce:type="list&lt;uint8&gt;" sce:capacity="4"/>
    <data id="total" sce:type="uint32" expr="0"/>
    <data id="flag" sce:type="bool" expr="false"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      {body}
    </transition>
  </state>
</scxml>
"##
    )
}

/// `sce-codegen <args…> probe.scxml` over `doc`, as `(succeeded, output)`.
fn run(args: &[&str], doc: &str) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(args)
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

fn refused(body: &str, because: &str) {
    let (ok, out) = run(&["check"], &machine(body));
    assert!(!ok, "expected a refusal ({because}), got:\n{out}");
    assert!(
        out.contains(because),
        "the refusal must say {because:?}:\n{out}"
    );
}

#[test]
fn a_loop_over_a_list_is_accepted_with_an_item_an_index_and_nesting() {
    let body = r#"
      <foreach array="picked" item="v">
        <assign location="total" expr="total + v"/>
      </foreach>
      <foreach array="picked" item="v" index="i">
        <assign location="total" expr="total + v * (i + 1)"/>
        <foreach array="picked" item="w">
          <assign location="total" expr="total + w"/>
        </foreach>
      </foreach>"#;
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
    ] {
        let (ok, out) = run(args, &machine(body));
        assert!(ok, "{args:?}: a <foreach> over a list variable:\n{out}");
    }
}

#[test]
fn a_loop_walks_a_list_variable_and_nothing_else() {
    refused(
        r#"<foreach array="total" item="v"/>"#,
        "walks a list variable",
    );
    refused(
        r#"<foreach array="nothing" item="v"/>"#,
        "walks a list variable",
    );
    refused(
        r#"<foreach array="picked" item=""/>"#,
        "names the variable that holds each element",
    );
}

#[test]
fn a_loop_variable_is_a_name_nothing_in_scope_already_means() {
    for (written, name) in [
        (r#"item="total""#, "total"),
        (r#"item="picked""#, "picked"),
        (r#"item="v" index="total""#, "total"),
        (r#"item="v" index="v""#, "v"),
    ] {
        refused(&format!(r#"<foreach array="picked" {written}/>"#), name);
    }
    refused(
        r#"<foreach array="picked" item="v"><foreach array="picked" item="v"/></foreach>"#,
        "which this one would hide",
    );
}

#[test]
fn a_loop_variable_is_a_name_the_generated_code_can_declare() {
    refused(
        r#"<foreach array="picked" item="a-b"/>"#,
        "a name the generated code can spell",
    );
    refused(
        r#"<foreach array="picked" item="sce_v"/>"#,
        "does not begin `sce_`",
    );
    // A keyword in one backend is a name no document can use in any.
    for keyword in ["fun", "match", "class"] {
        refused(
            &format!(r#"<foreach array="picked" item="{keyword}"/>"#),
            "no backend reserves as a keyword",
        );
    }
}

#[test]
fn the_item_is_typed_as_the_lists_element_and_belongs_to_the_body() {
    // A `uint8` is not a `bool`: the item is typed, not left to any use.
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<foreach array="picked" item="v"><assign location="flag" expr="v"/></foreach>"#,
        ),
    );
    assert!(!ok, "a uint8 item stored in a bool:\n{out}");
    // Outside the loop the name means nothing.
    let (ok, out) = run(
        &["check"],
        &machine(
            r#"<foreach array="picked" item="v"/>
      <assign location="total" expr="total + v"/>"#,
        ),
    );
    assert!(!ok, "the item read after the loop:\n{out}");
}

// ── A loop variable is bound and read under one spelling ──
//
// A backend spells a local its own way (snake case in Rust, Python and C), and
// the loop once bound its variables as written while every expression of the
// body read them under that spelling: the standard shared list bound `laterAt`
// and compared `later_at > earlier_at`, which no Rust compiler found in scope.
// Names of one word agree under both rules, so only a name with a capital in it
// shows the split, in a condition, an assignment, a record item's string field
// and a record item handed to an algorithm.

/// The fixtures the loop probe imports, copied beside it.
const LOOP_SIBLINGS: [&str; 3] = [
    "schema_labelled.scxml",
    "schema_entry.scxml",
    "algorithm_ledger_amount.scxml",
];

/// Loops over a list of numbers (nested), a list of records with a string
/// field, and a list of records whose item is handed to an algorithm, each with
/// an item and an index whose names have a capital in them.
const CAMEL_LOOPS: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="camel_loops">
  <sce:import kind="event-schema" src="schema_labelled.scxml" as="Labelled"/>
  <sce:import kind="event-schema" src="schema_entry.scxml" as="Entry"/>
  <sce:import kind="algorithm" src="algorithm_ledger_amount.scxml" as="AmountOf"/>
  <datamodel>
    <data id="picked" sce:type="list&lt;uint8&gt;" sce:capacity="4"/>
    <data id="labels" sce:type="list&lt;record:Labelled&gt;" sce:capacity="3"/>
    <data id="entries" sce:type="list&lt;record:Entry&gt;" sce:capacity="3"/>
    <data id="note" sce:type="string" sce:capacity="16" expr="'none'"/>
    <data id="total" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      <foreach array="picked" item="outerItem" index="outerAt">
        <foreach array="picked" item="innerItem" index="innerAt">
          <if cond="innerAt &gt; outerAt &amp;&amp; innerItem !== outerItem">
            <assign location="total" expr="total + outerItem + innerItem + outerAt + innerAt"/>
          </if>
        </foreach>
      </foreach>
      <foreach array="labels" item="lastLabel" index="labelAt">
        <if cond="lastLabel.sensor !== 0 &amp;&amp; labelAt &lt; 3">
          <assign location="note" expr="lastLabel.label"/>
          <assign location="total" expr="total + lastLabel.sensor + labelAt"/>
        </if>
      </foreach>
      <foreach array="entries" item="lineItem" index="lineAt">
        <if cond="AmountOf(lineItem) &gt; 100">
          <assign location="total" expr="total + lineAt"/>
        </if>
      </foreach>
    </transition>
  </state>
</scxml>
"##;

/// The loop variables [`CAMEL_LOOPS`] names.
const LOOP_VARIABLES: [&str; 8] = [
    "outerItem",
    "outerAt",
    "innerItem",
    "innerAt",
    "lastLabel",
    "labelAt",
    "lineItem",
    "lineAt",
];

/// Every file under `dir`, read as text.
fn read_tree(dir: &Path, into: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).expect("read the output directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            read_tree(&path, into);
        } else if let Ok(text) = std::fs::read_to_string(&path) {
            into.push(text);
        }
    }
}

/// `text`, written in `language`, with its comments and its string and
/// character literals taken out. A comment and a failure message quote the
/// document as the author wrote it, whatever the code spells, so only what is
/// left is the code's own spelling of a name.
fn code_of(text: &str, language: &str) -> String {
    let python = language == "python";
    let chars: Vec<char> = text.chars().collect();
    let at = |i: usize, s: &str| {
        s.chars()
            .enumerate()
            .all(|(k, c)| chars.get(i + k) == Some(&c))
    };
    let mut code = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if (python && c == '#') || (!python && at(i, "//")) {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if !python && at(i, "/*") {
            i += 2;
            while i < chars.len() && !at(i, "*/") {
                i += 1;
            }
            i += 2;
            code.push(' ');
        } else if at(i, "\"\"\"") {
            i += 3;
            while i < chars.len() && !at(i, "\"\"\"") {
                i += 1;
            }
            i += 3;
            code.push_str("\"\"");
        } else if c == '"' || (python && c == '\'') || (language == "go" && c == '`') {
            i += 1;
            while i < chars.len() && chars[i] != c {
                if chars[i] == '\\' && c != '`' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            code.push_str("\"\"");
        } else if c == '\'' && chars.get(i + 1) == Some(&'\\') {
            // An escaped character literal, `'\0'` or `'\''`.
            i += 3;
            while i < chars.len() && chars[i] != '\'' {
                i += 1;
            }
            i += 1;
            code.push(' ');
        } else if c == '\'' && chars.get(i + 2) == Some(&'\'') {
            // A character literal, `'"'`; a Rust label (`'action_block`) is not one.
            i += 3;
            code.push(' ');
        } else {
            code.push(c);
            i += 1;
        }
    }
    code
}

/// `name` without its underscores and its case: what every spelling a backend
/// gives the name has in common.
fn folded(name: &str) -> String {
    name.chars()
        .filter(|c| *c != '_')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The code `sce-codegen generate -l <language>` writes for [`CAMEL_LOOPS`],
/// comments and literals taken out ([`code_of`]).
fn camel_loops_code(language: &str) -> String {
    let input = tempdir().expect("tempdir");
    let fixtures = repo_root().join("sce-build/tests/fixtures/static_datamodel");
    for name in LOOP_SIBLINGS {
        std::fs::copy(fixtures.join(name), input.path().join(name)).expect("copy a sibling");
    }
    let doc = input.path().join("camel_loops.scxml");
    std::fs::write(&doc, CAMEL_LOOPS).expect("write the probe");
    let output = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("generate")
        .arg(doc)
        .arg("-o")
        .arg(output.path())
        .arg("-l")
        .arg(language)
        .arg("--go-module-prefix")
        .arg("example.test/generated")
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "sce-codegen generate -l {language} refused the probe:\n{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let mut files = Vec::new();
    read_tree(output.path(), &mut files);
    files
        .iter()
        .map(|text| code_of(text, language))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_loop_variable_is_bound_and_read_under_one_spelling_in_every_backend() {
    for language in emitting(ForgeKind::Statechart).map(|language| language.canonical_name()) {
        let code = camel_loops_code(language);
        for name in LOOP_VARIABLES {
            let mut spellings: BTreeMap<&str, usize> = BTreeMap::new();
            for token in code
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .filter(|token| folded(token) == folded(name))
            {
                *spellings.entry(token).or_default() += 1;
            }
            assert_eq!(
                spellings.len(),
                1,
                "{language}: the loop variable `{name}` is written as {spellings:?} in the \
                 code, where the loop that binds it and the expressions that read it must \
                 spell one identifier"
            );
            let (spelled, uses) = spellings.iter().next().expect("one spelling");
            assert!(
                *uses >= 2,
                "{language}: `{name}` is written as `{spelled}` {uses} time(s); the loop binds \
                 it and the body reads it"
            );
        }
    }
}
