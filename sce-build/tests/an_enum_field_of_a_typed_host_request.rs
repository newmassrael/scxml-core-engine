// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! An enum field of a host-run `<invoke>`'s request record (SCE Accepted Subset
//! §2.12; docs/adr/0109, step 4 of its order of work): under
//! `datamodel="sce-static"` the `<param>` that supplies it is held to a value of
//! the field's own enum where it is written; the value crosses to the host as
//! the name its enum declares for it; the Rust and C++ records hold it in the
//! machine's own enum and their adapters read it back by that name; and every
//! other lane refuses the request by name (docs/adr/0109, D0).
//!
//! Each case writes a statechart beside the enum and the two records it
//! imports, since the aliases resolve through sibling files, the path the CLI
//! takes. The accepted case comes first: without it, every refusal below could
//! be the document failing for a reason of its own.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::forge::codegen_matrix::emitting;
use sce_build::forge::diagnostic::ToDiagnostics;
use sce_build::forge::model::ForgeKind;
use tempfile::{tempdir, TempDir};

/// The enum the request's field holds.
const LEVEL: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="level" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="low" value="0"/>
      <sce:variant name="high" value="1"/>
    </data>
  </datamodel>
</scxml>"#;

/// Another enum, whose values are no values of [`LEVEL`].
const MODE: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="enum" name="mode" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="on" value="0"/>
      <sce:variant name="off" value="1"/>
    </data>
  </datamodel>
</scxml>"#;

const REQUEST: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       name="ask_request" sce:kind="event-schema" sce:event-name="ask.request">
  <sce:import kind="enum" src="level.scxml" as="Level"/>
  <datamodel>
    <data id="level" sce:type="enum:Level" sce:direction="in"/>
    <data id="text" sce:type="string[16]" sce:direction="in"/>
  </datamodel>
</scxml>"#;

const RESULT: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       name="ask_result" sce:kind="event-schema" sce:event-name="ask.result">
  <datamodel>
    <data id="ok" sce:type="bool" sce:direction="in"/>
  </datamodel>
</scxml>"#;

/// The type the statecharts' host runs, declared to every build.
const HOST_TYPE: &str = "x-app-host";

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// A statechart that imports the two records and the enums `imports` names,
/// declares the variables `data` holds beside the text `label`, and asks its
/// host with `level` for the record's enum field.
fn doc(imports: &str, data: &str, level: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="a" datamodel="sce-static" name="probe">
  <sce:import kind="event-schema" src="ask_request.scxml" as="AskRequest"/>
  <sce:import kind="event-schema" src="ask_result.scxml" as="AskResult"/>
{imports}
  <datamodel>
    <data id="label" sce:type="string[16]" expr="''"/>
{data}
    <data id="finished" sce:type="bool" expr="false" sce:direction="out"/>
  </datamodel>
  <state id="a">
    <transition event="go" target="b"/>
  </state>
  <state id="b">
    <invoke type="x-app-host" id="ask" sce:request="AskRequest" sce:result="AskResult">
      <param name="level" expr="{level}"/>
      <param name="text" expr="label"/>
    </invoke>
    <transition event="done.invoke.ask" target="a">
      <assign location="finished" expr="_event.data.ok"/>
    </transition>
  </state>
</scxml>
"#
    )
}

const IMPORTS_LEVEL: &str = r#"  <sce:import kind="enum" src="level.scxml" as="Level"/>"#;

const IMPORTS_BOTH: &str = r#"  <sce:import kind="enum" src="level.scxml" as="Level"/>
  <sce:import kind="enum" src="mode.scxml" as="Mode"/>"#;

const CHOSEN: &str = r#"    <data id="chosen" sce:type="enum:Level" expr="Level.high"/>"#;

const SWITCHED: &str = r#"    <data id="switched" sce:type="enum:Mode" expr="Mode.on"/>"#;

/// A fresh directory holding `doc` as `probe.scxml` beside the enums and the
/// records it imports.
fn written(doc: &str) -> (TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    for (name, text) in [
        ("level.scxml", LEVEL),
        ("mode.scxml", MODE),
        ("ask_request.scxml", REQUEST),
        ("ask_result.scxml", RESULT),
    ] {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
    let path = dir.path().join("probe.scxml");
    std::fs::write(&path, doc).expect("write probe");
    (dir, path)
}

/// `sce-codegen check <args> --host-invoker x-app-host <doc>` with JSON
/// diagnostics: whether it succeeded, and everything it wrote.
fn check(args: &[&str], doc: &str) -> (bool, String) {
    let (_dir, path) = written(doc);
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .arg("check")
        .args(args)
        .args(["--host-invoker", HOST_TYPE])
        .arg(&path)
        .output()
        .expect("invoke sce-codegen");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned() + &String::from_utf8_lossy(&out.stdout),
    )
}

/// `sce-codegen generate -l <language> --host-invoker x-app-host --no-format`
/// of `doc`: the text of every file it wrote named after the machine, `probe`,
/// as the generator spelled it (no formatter rewraps a line read below).
fn generate(language: &str, doc: &str) -> String {
    let (_dir, path) = written(doc);
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .arg("--error-format=json")
        .args(["generate", "-l", language])
        .args(["--host-invoker", HOST_TYPE])
        .arg(&path)
        .arg("-o")
        .arg(out_dir.path())
        .arg("--no-format")
        .output()
        .expect("invoke sce-codegen");
    assert!(
        out.status.success(),
        "{language} refused the machine: {}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let mut text = String::new();
    for entry in std::fs::read_dir(out_dir.path()).expect("the output directory") {
        let path = entry.expect("an entry").path();
        let of_machine = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().to_lowercase().contains("probe"));
        if path.is_file() && of_machine {
            text.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            text.push('\n');
        }
    }
    assert!(
        !text.is_empty(),
        "{language} wrote no file named after the machine: a test that read none would pass"
    );
    text
}

/// Every backend that emits a statechart, by the identifier `-l` takes.
fn languages() -> Vec<&'static str> {
    emitting(ForgeKind::Statechart)
        .map(|language| language.canonical_name())
        .collect()
}

#[test]
fn a_value_of_the_fields_own_enum_is_accepted_on_rust_and_cpp() {
    // A variable of the enum, and a variant written where the field is
    // supplied, which no variable of the machine holds.
    for (data, level) in [(CHOSEN, "chosen"), ("", "Level.low")] {
        let accepted = doc(IMPORTS_LEVEL, data, level);
        for language in ["rust", "cpp"] {
            let (ok, text) = check(&["-l", language], &accepted);
            assert!(
                ok,
                "{language} refused `{level}` for the enum field: {text}"
            );
        }
    }
}

/// docs/adr/0109, D0: a lane that does not read an enum field of a typed
/// request refuses the machine by name, `generate/unsupported-feature`, naming
/// the field and the lanes that read one; never a machine whose host would be
/// handed a request its adapter cannot read.
#[test]
fn every_other_lane_refuses_an_enum_request_field_by_name() {
    let accepted = doc(IMPORTS_LEVEL, CHOSEN, "chosen");
    let lanes = languages();
    let others: Vec<&str> = lanes
        .iter()
        .copied()
        .filter(|language| !matches!(*language, "rust" | "cpp"))
        .collect();
    assert!(
        !others.is_empty(),
        "no emitting lane but rust and cpp: the refusal is held on none"
    );
    for &language in &others {
        let (ok, text) = check(&["-l", language], &accepted);
        assert!(
            !ok,
            "{language} took an enum field of a typed request: {text}"
        );
        assert!(
            text.contains("generate/unsupported-feature")
                && text.contains("'level'")
                && text.contains(&format!("the {language} lane does not read"))
                && text.contains("rust, cpp"),
            "{language}: {text}"
        );
    }
}

/// The Interpreter is a lane as a backend is, and a lowering for it is refused
/// by its name, `interpreter`.
#[test]
fn a_lowering_for_the_interpreter_refuses_an_enum_request_field_by_name() {
    let (_dir, path) = written(&doc(IMPORTS_LEVEL, CHOSEN, "chosen"));
    let refusal =
        sce_build::forge::static_js::lower_file_set(path.to_str().expect("utf-8 path"), Vec::new())
            .expect_err("the Interpreter took an enum field of a typed request");
    let diagnostic = &refusal.to_diagnostics()[0];
    assert_eq!(diagnostic.code.as_str(), "generate/unsupported-feature");
    assert!(
        diagnostic.message.contains(
            "the interpreter lane does not read an enum field of a typed host-run request"
        ),
        "{}",
        diagnostic.message
    );
}

/// A value that is no value of the field's own enum is refused where it is
/// written: the variant's name as a text literal, at parse, and a text, a value
/// of another enum and a value of an enum the document does not import, by the
/// judge of the data model.
#[test]
fn a_value_that_is_not_of_the_fields_enum_is_refused_where_it_is_written() {
    let (ok, text) = check(&[], &doc(IMPORTS_LEVEL, CHOSEN, "'high'"));
    assert!(!ok, "the name of a variant was taken as its value: {text}");
    assert!(text.contains("validation/typed-invoke-request"), "{text}");

    for (imports, data, level, says) in [
        (IMPORTS_LEVEL, CHOSEN, "label", "no value of an enum"),
        (IMPORTS_BOTH, SWITCHED, "switched", "of the enum `Mode`"),
        ("", "", "label", "does not import"),
    ] {
        let (ok, text) = check(&[], &doc(imports, data, level));
        assert!(!ok, "`{level}` was taken for the enum field: {text}");
        assert!(
            text.contains("scxml/static-datamodel-rule") && text.contains(says),
            "`{level}`: {text}"
        );
    }
}

/// The Rust record holds the field in the machine's own enum, with no default,
/// and its adapter reads it back through `request_field`, which parses the
/// name it crossed as through the enum's `FromStr`.
#[test]
fn the_rust_record_holds_the_machines_own_enum() {
    let text = generate("rust", &doc(IMPORTS_LEVEL, "", "Level.high"));
    for wanted in [
        "#[derive(Clone, Debug, PartialEq)]\npub struct ProbeAskRequest {\n    pub level: ProbeLevelEnum,\n",
        "level: ::sce_rust_runtime::request_field(&request, \"level\"),",
        "pub fn from_sce_name(name: &str) -> Option<Self> {",
        "impl ::core::str::FromStr for ProbeLevelEnum {",
    ] {
        assert!(text.contains(wanted), "no `{wanted}` in:\n{text}");
    }
}

/// The C++ record holds the field in the machine's own enum, and its adapter
/// reads it back from the name it crossed as through `<type>FromSceName`.
#[test]
fn the_cpp_record_holds_the_machines_own_enum() {
    let text = generate("cpp", &doc(IMPORTS_LEVEL, "", "Level.high"));
    for wanted in [
        "    ProbeLevelEnum level{};\n",
        "typed.level = ProbeLevelEnumFromSceName(::SCE::requestField<std::string>(start, \"level\")).value();",
        "inline std::optional<ProbeLevelEnum> ProbeLevelEnumFromSceName(const std::string &name) {",
    ] {
        assert!(text.contains(wanted), "no `{wanted}` in:\n{text}");
    }
}
