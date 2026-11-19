// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A value the deployment supplies (docs/adr/0109, D12; docs/SCE_ACCEPTED_SUBSET.md
// §2.15, "A value the deployment supplies"): `<data sce:from="deployment">` declares a
// value of a type the deployment gives the machine before it starts. The machine
// reads it and never writes it, so every statement that would write it is refused on
// its own code and on the row of the element; its declaration takes no initial value
// of the document's; and Rust and C++ build the machine from it while every other
// backend, and the Interpreter's lowering, refuses it by name.

use std::path::{Path, PathBuf};
use std::process::Command;

use sce_build::generator::Language;
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

/// Run `sce-codegen <args…> probe.scxml` with JSON diagnostics, `siblings` written
/// beside the document where its imports resolve, and return `(exit_ok, output)`.
fn run(args: &[&str], doc: &str, siblings: &[(&str, &str)]) -> (bool, String) {
    let dir = tempdir().expect("tempdir");
    for (name, text) in siblings {
        std::fs::write(dir.path().join(name), text).expect("write sibling");
    }
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

/// The record the deployment's `limits` is a value of.
const LIMITS_SCHEMA: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="event-schema" name="limits" sce:event-name="limits">
  <datamodel>
    <data id="max_rows" sce:type="uint32" sce:direction="in"/>
    <data id="label" sce:type="string[16]" sce:direction="in"/>
  </datamodel>
</scxml>
"#;

/// A child whose own value the deployment would supply.
const DEPLOYED_CHILD: &str = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="run" datamodel="sce-static" name="child">
  <datamodel>
    <data id="limit" sce:type="uint32" sce:from="deployment"/>
  </datamodel>
  <state id="run"/>
</scxml>
"#;

fn siblings() -> Vec<(&'static str, &'static str)> {
    vec![
        ("limits.scxml", LIMITS_SCHEMA),
        ("child.scxml", DEPLOYED_CHILD),
    ]
}

/// A statechart under `datamodel` whose deployment supplies a record and a list, whose
/// `limits` declaration ends with `extra`, and whose `go` transition holds `body`. A
/// statement of the body stands on line [`BODY_LINE`]; the `limits` declaration on
/// line [`LIMITS_LINE`].
fn statechart(datamodel: &str, extra: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="{datamodel}" name="probe">
  <sce:import kind="event-schema" src="limits.scxml" as="Limits"/>
  <datamodel>
    <data id="limits" sce:type="record:Limits" sce:from="deployment"{extra}/>
    <data id="hosts" sce:type="list&lt;string[32]&gt;[4]" sce:from="deployment"/>
    <data id="rows" sce:type="uint32" expr="limits.max_rows"/>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
{body}
    </transition>
  </state>
</scxml>
"#
    )
}

const LIMITS_LINE: u32 = 6;
const BODY_LINE: u32 = 13;

/// A body that reads both values and writes neither.
const READS: &str =
    r#"      <assign location="count" expr="count + limits.max_rows + len(hosts)"/>"#;

fn check(args: &[&str], doc: &str) -> (bool, String) {
    run(args, doc, &siblings())
}

/// Assert a refusal carries `code` and is placed on `line`.
fn assert_refused_at(out: &str, code: &str, line: u32) {
    let record = out
        .lines()
        .find(|l| l.contains(&format!("\"code\":\"{code}\"")))
        .unwrap_or_else(|| panic!("expected {code}, got:\n{out}"));
    assert!(
        record.contains(&format!("\"line\":{line},"))
            || record.contains(&format!("\"line\":{line}}}")),
        "{code} must be placed on line {line}:\n{record}"
    );
}

/// A machine that reads what its deployment supplies, an initial value among the
/// reads, keeps every rule and is lowered by Rust and C++.
#[test]
fn a_value_the_deployment_supplies_is_read_and_lowered_by_rust_and_cpp() {
    let doc = statechart("sce-static", "", READS);
    let (ok, out) = check(&["check"], &doc);
    assert!(
        ok,
        "the judge takes a value the deployment supplies:\n{out}"
    );
    for language in ["rust", "cpp"] {
        let (ok, out) = check(&["check", "-l", language], &doc);
        assert!(
            ok,
            "{language} builds the machine from its deployment:\n{out}"
        );
    }
}

/// Every statement that writes a place, held to a place inside a deployment value:
/// each is refused as `validation/deployment-value-assigned`, on its own row, with the
/// place as written as the record's `actual`.
#[test]
fn every_statement_that_writes_a_deployment_value_is_refused() {
    let writes = [
        (
            r#"      <assign location="limits.max_rows" expr="1"/>"#,
            "limits.max_rows",
        ),
        (
            r#"      <assign location="limits" expr="limits"/>"#,
            "limits",
        ),
        (r#"      <sce:append target="hosts" expr="'h'"/>"#, "hosts"),
        (r#"      <sce:clear target="hosts"/>"#, "hosts"),
        (
            r#"      <foreach array="hosts" item="limits"><assign location="count" expr="1"/></foreach>"#,
            "limits",
        ),
        (
            r##"      <send event="go" target="#_internal" idlocation="limits"/>"##,
            "limits",
        ),
    ];
    for (body, place) in writes {
        let (ok, out) = check(&["check"], &statechart("sce-static", "", body));
        assert!(!ok, "a write of `{place}` is refused:\n{out}");
        assert_refused_at(&out, "validation/deployment-value-assigned", BODY_LINE);
        assert!(
            out.contains(&format!("\"actual\":\"{place}\"")),
            "the place written is the record's `actual`:\n{out}"
        );
    }
}

/// The declaration names the deployment as its one source, and takes nothing the
/// document would start it at; it is the host's input, so it is not published.
#[test]
fn a_declaration_of_a_deployment_value_holds_its_rules() {
    // An initial value of the document's, and a publication to the host.
    for extra in [r#" expr="limits""#, r#" sce:direction="out""#] {
        let (ok, out) = check(&["check"], &statechart("sce-static", extra, READS));
        assert!(!ok, "`{extra}` beside sce:from is refused:\n{out}");
        assert_refused_at(&out, "scxml/static-datamodel-rule", LIMITS_LINE);
    }
    // A source other than the deployment.
    let doc = statechart("sce-static", "", READS).replace(
        r#"sce:type="record:Limits" sce:from="deployment""#,
        r#"sce:type="record:Limits" sce:from="environment""#,
    );
    let (ok, out) = check(&["check"], &doc);
    assert!(!ok, "a source other than the deployment is refused:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", LIMITS_LINE);
    assert!(out.contains("environment"), "{out}");
    // A field built by `<sce:set>`, as a record variable of the document is.
    let doc = statechart("sce-static", "", READS).replace(
        r#"sce:type="record:Limits" sce:from="deployment"/>"#,
        r#"sce:type="record:Limits" sce:from="deployment"><sce:set name="max_rows" expr="1"/><sce:set name="label" expr="''"/></data>"#,
    );
    let (ok, out) = check(&["check"], &doc);
    assert!(
        !ok,
        "a deployment value built by <sce:set> is refused:\n{out}"
    );
    assert_refused_at(&out, "scxml/static-datamodel-rule", LIMITS_LINE);
}

/// Under any other data model the attribute is read by nothing, and refused so.
#[test]
fn a_deployment_value_under_another_data_model_is_unread() {
    let doc = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="ecmascript" name="probe">
  <datamodel>
    <data id="limit" expr="1" sce:from="deployment"/>
  </datamodel>
  <state id="s"/>
</scxml>
"#;
    let (ok, out) = check(&["check"], doc);
    assert!(!ok, "sce:from outside sce-static is refused:\n{out}");
    assert!(
        out.contains("\"code\":\"validation/sce-attribute-unread\""),
        "the attribute is unread there:\n{out}"
    );
}

/// A child session is started by its parent, which has no deployment value to give
/// it: an `<invoke>` of a child that declares one is refused at the invoke.
#[test]
fn a_child_that_declares_a_deployment_value_is_refused_at_its_invoke() {
    let doc = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="probe">
  <datamodel>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <invoke id="worker" type="scxml" src="child.scxml"/>
  </state>
</scxml>
"#;
    let (ok, out) = check(&["check"], doc);
    assert!(!ok, "a child with a deployment value is refused:\n{out}");
    assert_refused_at(&out, "scxml/static-datamodel-rule", 8);
    assert!(out.contains("limit"), "the child's value is named:\n{out}");
}

/// Rust and C++ read a value the deployment supplies (docs/adr/0109, D0); every other
/// backend, and the Interpreter's lowering, refuses it by name where the `<data>` is
/// walked, rather than starting the machine on a value no deployment gave.
#[test]
fn every_other_backend_refuses_a_deployment_value_by_name() {
    // A number alone, which every backend lowers as a variable, so the refusal is
    // the deployment value's and no other construct's.
    let doc = r#"<?xml version="1.0"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="s" datamodel="sce-static" name="probe">
  <datamodel>
    <data id="limit" sce:type="uint32" sce:from="deployment"/>
    <data id="count" sce:type="uint32" expr="0"/>
  </datamodel>
  <state id="s">
    <transition event="go" type="internal">
      <assign location="count" expr="count + limit"/>
    </transition>
  </state>
</scxml>
"#;
    for language in Language::ALL {
        let name = language.canonical_name();
        let (ok, out) = check(&["check", "-l", name], doc);
        match language {
            Language::Rust | Language::Cpp => {
                assert!(ok, "{name} reads a deployment value:\n{out}");
            }
            _ => {
                assert!(!ok, "{name} reads no deployment value yet:\n{out}");
                assert!(
                    out.contains("\"code\":\"generate/unsupported-feature\""),
                    "{name} refuses it by name:\n{out}"
                );
                assert!(
                    out.contains("reads no value a deployment supplies"),
                    "the refusal says what {name} does not read:\n{out}"
                );
            }
        }
    }
    let (ok, out) = check(&["lower"], doc);
    assert!(!ok, "the Interpreter reads no deployment value:\n{out}");
    assert!(
        out.contains("generate/unsupported-feature")
            && out.contains("reads no value a deployment supplies"),
        "the Interpreter refuses it by name:\n{out}"
    );
}
