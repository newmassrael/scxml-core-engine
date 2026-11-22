// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `datamodel="sce-static"`: a list variable assigned what an imported algorithm
// returns as a list (docs/SCE_ACCEPTED_SUBSET.md §2.15).
//
// A list is filled by `<sce:append>`, one element at a time; a calendar screen
// that shows a run of days wants them all at once from the algorithm that
// computes them. The assignment replaces the list, so the list's bound is the
// one thing it must still hold: an algorithm that may return more than the
// variable's `sce:capacity` is refused when the document is built, because a
// machine holds the same list wherever it runs.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::{emitting, language_wire_name};
use sce_build::forge::model::ForgeKind;
use tempfile::{tempdir, TempDir};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// A statechart with a `list<uint8>` `shown` of the given `capacity`, a
/// `list<uint16>` `wide`, a `uint8` `n` and the given `body` in one transition,
/// beside the algorithm it imports.
fn machine(capacity: u32, body: &str) -> (TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    for algorithm in ["algorithm_day_run.scxml", "algorithm_day_repeat.scxml"] {
        std::fs::copy(
            repo_root()
                .join("sce-build/tests/fixtures/static_datamodel")
                .join(algorithm),
            dir.path().join(algorithm),
        )
        .expect("an algorithm of the fixtures");
    }
    // A statechart keeps no import it does not call, so each is imported by the
    // body that calls it.
    let imports: String = [
        ("Range", "algorithm_day_run.scxml"),
        ("Repeat", "algorithm_day_repeat.scxml"),
    ]
    .iter()
    .filter(|(alias, _)| body.contains(&format!("{alias}(")))
    .map(|(alias, src)| format!("<sce:import kind=\"algorithm\" src=\"{src}\" as=\"{alias}\"/>"))
    .collect();
    let path = dir.path().join("probe.scxml");
    std::fs::write(
        &path,
        format!(
            r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  {imports}
  <datamodel>
    <data id="shown" sce:type="list&lt;uint8&gt;" sce:capacity="{capacity}"/>
    <data id="wide" sce:type="list&lt;uint16&gt;" sce:capacity="8"/>
    <data id="n" sce:type="uint8" expr="3"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      {body}
    </transition>
  </state>
</scxml>
"##
        ),
    )
    .expect("write probe");
    (dir, path)
}

fn run(args: &[&str], capacity: u32, body: &str) -> (bool, String) {
    let (_dir, path) = machine(capacity, body);
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

#[test]
fn a_list_is_assigned_what_an_algorithm_returns_in_every_language() {
    let body = r#"<assign location="shown" expr="Range(n, 4)"/>"#;
    for args in [
        &["check"][..],
        &["check", "-l", "rust"],
        &["check", "-l", "kotlin"],
        &["check", "-l", "cpp"],
        &[
            "check",
            "-l",
            "go",
            "--go-module-prefix",
            "example.com/generated",
        ],
        &["check", "-l", "python"],
        &["check", "-l", "c11"],
    ] {
        let (ok, out) = run(args, 8, body);
        assert!(ok, "{args:?}: a list assigned from an algorithm:\n{out}");
    }
}

#[test]
fn another_algorithm_is_taken_by_the_bound_it_declares() {
    // `Repeat` holds four: a list of eight takes it, one of three cannot.
    let body = r#"<assign location="shown" expr="Repeat(n, 2)"/>"#;
    let (ok, out) = run(&["check"], 8, body);
    assert!(ok, "a list that can hold four:\n{out}");
    let (ok, out) = run(&["check"], 3, body);
    assert!(!ok, "expected a refusal, got:\n{out}");
    assert!(out.contains("at most 4"), "the refusal must say 4:\n{out}");
}

#[test]
fn only_an_algorithm_that_declares_may_fail_is_taken() {
    // The same algorithm with `may-fail` taken off returns a plain list on the
    // backends whose lists grow and a fallible one on the rest, so a statechart has
    // no one shape to receive it in: it stays a host's to call.
    let body = r#"<assign location="shown" expr="Repeat(n, 2)"/>"#;
    let (dir, path) = machine(8, body);
    let algorithm = dir.path().join("algorithm_day_repeat.scxml");
    let text = std::fs::read_to_string(&algorithm).expect("the algorithm");
    assert!(
        text.contains(" may-fail=\"true\""),
        "the fixture declares it"
    );
    std::fs::write(&algorithm, text.replace(" may-fail=\"true\"", "")).expect("rewrite");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["check"])
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    let said =
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout);
    assert!(!out.status.success(), "expected a refusal, got:\n{said}");
    // The refusal says which algorithm and which condition it fails: not that a list
    // "takes what an imported algorithm returns" when this call is exactly that.
    for needed in ["`Repeat`", "may-fail"] {
        assert!(
            said.contains(needed),
            "the refusal must name {needed:?}:\n{said}"
        );
    }
}

#[test]
fn a_list_variable_must_hold_what_the_algorithm_may_return() {
    // The algorithm returns at most 8; a list of 4 cannot be assigned it.
    let (ok, out) = run(
        &["check"],
        4,
        r#"<assign location="shown" expr="Range(n, 4)"/>"#,
    );
    assert!(!ok, "expected a refusal, got:\n{out}");
    assert!(
        out.contains("at most 8"),
        "the refusal must say how many the algorithm may return:\n{out}"
    );
}

#[test]
fn only_a_list_of_the_algorithms_element_is_assigned_it() {
    let (ok, out) = run(
        &["check"],
        8,
        r#"<assign location="wide" expr="Range(n, 4)"/>"#,
    );
    assert!(!ok, "expected a refusal, got:\n{out}");
    assert!(
        out.contains("uint8") && out.contains("uint16"),
        "the refusal must name both element types:\n{out}"
    );
}

#[test]
fn a_returned_list_is_still_no_value_anywhere_but_a_whole_list_assignment() {
    for body in [
        r#"<assign location="n" expr="Range(n, 4)"/>"#,
        r#"<assign location="n" expr="len(Range(n, 4))"/>"#,
        r#"<assign location="n" expr="Range(n, 4)[0]"/>"#,
    ] {
        let (ok, out) = run(&["check"], 8, body);
        assert!(!ok, "expected a refusal for {body}, got:\n{out}");
    }
}

/// The lines of two entries, or none: a list of records, from a truth value alone.
const LINES: &str = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="algorithm" name="probe_lines">
  <sce:import kind="event-schema" src="schema_entry.scxml" as="Entry"/>
  <sce:signature>
    <sce:param name="some" type="bool"/>
    <sce:return type="list&lt;record:Entry&gt;" returns-max-size="8" may-fail="true"/>
  </sce:signature>
  <sce:body>
    <sce:var name="out" type="list&lt;record:Entry&gt;" capacity="8"/>
    <sce:if cond="some">
      <sce:var name="line" type="record:Entry">
        <sce:set name="id" expr="1"/>
        <sce:set name="amount" expr="2"/>
      </sce:var>
      <sce:append target="out" expr="line"/>
    </sce:if>
    <sce:return expr="out"/>
  </sce:body>
</scxml>
"##;

/// A list of records an algorithm returns is converted into the machine's own records
/// whatever the algorithm takes: one that took no record and no list was called by its
/// own name, as one that returns numbers is, and the check of C11 panicked on a value
/// with no type of its own (SCE_ACCEPTED_SUBSET.md §2.15).
#[test]
fn a_list_of_records_is_assigned_from_an_algorithm_that_takes_a_truth_value() {
    let dir = tempdir().expect("tempdir");
    std::fs::copy(
        repo_root().join("sce-build/tests/fixtures/static_datamodel/schema_entry.scxml"),
        dir.path().join("schema_entry.scxml"),
    )
    .expect("the schema of the fixtures");
    std::fs::write(dir.path().join("probe_lines.scxml"), LINES).expect("write the algorithm");
    let path = dir.path().join("probe.scxml");
    std::fs::write(
        &path,
        r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static">
  <sce:import kind="algorithm" src="probe_lines.scxml" as="Lines"/>
  <sce:import kind="event-schema" src="schema_entry.scxml" as="Entry"/>
  <datamodel>
    <data id="rows" sce:type="list&lt;record:Entry&gt;" sce:capacity="8"/>
    <data id="some" sce:type="bool" expr="true"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal"><assign location="rows" expr="Lines(some)"/></transition>
  </state>
</scxml>
"##,
    )
    .expect("write probe");
    let mut wrong = Vec::new();
    let wires = std::iter::once("").chain(emitting(ForgeKind::Algorithm).map(language_wire_name));
    for lang in wires {
        let mut command = Command::new(sce_codegen_bin());
        command
            .arg("--workspace-root")
            .arg(repo_root())
            .arg("--error-format=json")
            .arg("check");
        if !lang.is_empty() {
            command.args(["-l", lang, "--go-module-prefix", "example.com/generated"]);
        }
        let out = command.arg(&path).output().expect("invoke sce-codegen");
        if !out.status.success() {
            wrong.push(format!(
                "[{lang}] {}\n{}",
                String::from_utf8_lossy(&out.stderr),
                String::from_utf8_lossy(&out.stdout)
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// A machine that names an enum and holds no value of it declares its own type for the
/// enum: the adapter of a call that hands an algorithm a variant of it names the type,
/// and so does an expression that reads a variant's number. The generated Rust named
/// the type in both and defined it in neither, so it did not compile; what is generated
/// for the call is compiled and run in every lane by the scenario `static_record_call`.
#[test]
fn a_machine_declares_the_enum_it_only_names() {
    let colour = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="colour" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants"><sce:variant name="red" value="4"/><sce:variant name="blue" value="9"/></data>
  </datamodel>
</scxml>
"##;
    let is_red = r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="algorithm" name="is_red">
  <sce:import kind="enum" src="colour.scxml" as="Colour"/>
  <sce:signature>
    <sce:param name="c" type="enum:Colour"/><sce:param name="n" type="uint32"/>
    <sce:return type="bool"/>
  </sce:signature>
  <sce:body><sce:return expr="c === Colour.red &amp;&amp; n &gt; 0"/></sce:body>
</scxml>
"##;
    for (imports, assign) in [
        (
            r#"<sce:import kind="algorithm" src="is_red.scxml" as="IsRed"/>"#,
            r#"<assign location="b" expr="IsRed(Colour.red, n)"/>"#,
        ),
        (
            "",
            r#"<assign location="n" expr="Colour.toNumber(Colour.blue)"/>"#,
        ),
    ] {
        let dir = tempdir().expect("tempdir");
        std::fs::write(dir.path().join("colour.scxml"), colour).expect("write the enum");
        std::fs::write(dir.path().join("is_red.scxml"), is_red).expect("write the algorithm");
        let path = dir.path().join("painted.scxml");
        std::fs::write(
            &path,
            format!(
                r##"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="painted">
  <sce:import kind="enum" src="colour.scxml" as="Colour"/>
  {imports}
  <datamodel>
    <data id="n" sce:type="uint32" expr="1"/>
    <data id="b" sce:type="bool" expr="false" sce:direction="out"/>
  </datamodel>
  <state id="s"><transition event="go" type="internal">{assign}</transition></state>
</scxml>
"##
            ),
        )
        .expect("write the machine");
        let out_dir = dir.path().join("out");
        let out = Command::new(sce_codegen_bin())
            .arg("--workspace-root")
            .arg(repo_root())
            .arg("generate")
            .arg(&path)
            .args(["-l", "rust", "-o"])
            .arg(&out_dir)
            .output()
            .expect("invoke sce-codegen");
        assert!(
            out.status.success(),
            "{assign}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let unit = std::fs::read_dir(&out_dir)
            .expect("the generated files")
            .map(|entry| entry.expect("a file").path())
            .find(|file| file.ends_with("painted_sm.rs"))
            .map(|file| std::fs::read_to_string(file).expect("the machine"))
            .expect("the machine is generated");
        let defined = unit.matches("pub enum PaintedColourEnum").count();
        assert!(
            unit.contains("PaintedColourEnum::"),
            "{assign}: the machine names no variant of its enum:\n{unit}"
        );
        assert_eq!(defined, 1, "{assign}: the enum is defined {defined} times");
    }
}
