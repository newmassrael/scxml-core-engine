// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The Swift backend lowers a statechart of `datamodel="ecmascript"` whose only uses of a script
//! engine are `<data>` initialisers that are expressions and transition `cond`s: both are lowered to
//! Lua at build time and run by the `SceLua` engine (docs/SCE_LUA_TRANSLATION_SEAM.md), the engine
//! every other Lua backend runs the same Lua on. Every other reason a document needs an engine is
//! refused by name and not written without its engine call.
//!
//! `EcmaScriptMachinesTests` holds what the machines do under their `Engine`; this file holds the
//! text the generator writes and the line it refuses by name.
//!
//! # What was wrong
//!
//! The Swift generator refused `datamodel="ecmascript"` whole ("would need a script engine"), which
//! is the default data model, so a document that omitted the attribute was refused too, and 247 of
//! the 252 W3C documents could not be lowered. The engine existed in every other backend and in
//! Swift only after the `SceLua` module.

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

/// A document: the attributes of `<scxml>` beyond the fixed ones, and the elements inside it.
fn document(attributes: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s0" name="probe" {attributes}>
{body}
</scxml>
"#
    )
}

/// What `sce-codegen generate -l swift` made of `text`, written as `probe.scxml` beside the files of
/// `extra`: its exit status, the text of the machine it wrote (empty when it wrote none), and
/// everything it said.
fn generate(text: &str, extra: &[(&str, &str)]) -> (Option<i32>, String, String) {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("probe.scxml"), text).expect("write the document");
    for (name, content) in extra {
        std::fs::write(dir.path().join(name), content).expect("write a file beside it");
    }
    let out_dir = tempdir().expect("tempdir");
    let out = Command::new(sce_codegen_bin())
        .arg("--workspace-root")
        .arg(repo_root())
        .args(["generate", "-l", "swift"])
        .arg(dir.path().join("probe.scxml"))
        .arg("-o")
        .arg(out_dir.path())
        .output()
        .expect("invoke sce-codegen");
    let machine =
        std::fs::read_to_string(out_dir.path().join("probe_sm.swift")).unwrap_or_default();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), machine, said)
}

const GUARDED: &str = r#"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0"><transition cond="n == 1" target="done"/></state>
  <final id="done"/>"#;

#[test]
fn a_guard_and_a_data_initialiser_are_lowered_to_lua_and_run_by_the_engine() {
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, GUARDED), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(machine.contains("import SceLua"), "{machine}");
    assert!(
        machine.contains(r#"defineData("n", as: "1", engine: engine)"#),
        "the initialiser is lowered and handed to the session: {machine}"
    );
    assert!(
        machine.contains(r#"evaluateGuard("_scxml_eq(n, 1)", engine: engine)"#),
        "the cond is lowered to Lua and asked of the engine: {machine}"
    );
    assert!(
        machine.contains("public var needsScriptEngine: Bool { true }")
            && machine.contains("public var needsDataModelInit: Bool { true }"),
        "{machine}"
    );
}

/// The default data model is `ecmascript`, so a document that omits the attribute is lowered the same.
#[test]
fn a_document_that_omits_its_data_model_is_an_ecmascript_one() {
    let (status, machine, said) = generate(&document("", GUARDED), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(machine.contains("import SceLua"), "{machine}");
}

/// A machine that needs no engine does not link Lua: `SceRuntime` does not import `SceLua`, and the
/// machine imports it only when the document asks.
#[test]
fn a_machine_that_needs_no_engine_does_not_import_the_engine() {
    let body = r#"<state id="s0"><transition event="go" target="done"/></state><final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="null""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(!machine.contains("import SceLua"), "{machine}");
    assert!(!machine.contains("evaluateGuard"), "{machine}");
    assert!(
        machine.contains("public var needsScriptEngine: Bool { false }"),
        "{machine}"
    );
}

/// A cond the frontend decided is a value written into the machine; the engine is not asked what
/// the build already knows.
#[test]
fn a_cond_the_build_decided_is_not_sent_to_the_engine() {
    let body = r#"
  <datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0">
    <transition cond="true" target="done"/>
  </state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(!machine.contains("evaluateGuard(\""), "{machine}");
}

/// The name of a `<data>` crosses as a string: an XML name is not always a Lua identifier.
#[test]
fn the_name_of_a_data_variable_is_a_string_and_never_source() {
    let body = r#"
  <datamodel><data id="screen-rules" expr="(4)"/></datamodel>
  <state id="s0"><transition cond="true" target="done"/></state>
  <final id="done"/>"#;
    let (status, machine, said) = generate(&document(r#"datamodel="ecmascript""#, body), &[]);
    assert_eq!(status, Some(0), "{said}");
    assert!(
        machine.contains(r#"defineData("screen-rules", as: "#),
        "{machine}"
    );
}

/// Each reason a document needs the engine that the templates do not lower is refused by its name,
/// with the status that says it is an unsupported feature.
#[test]
fn what_the_templates_do_not_lower_is_refused_by_name() {
    struct Refusal {
        what: &'static str,
        attributes: &'static str,
        body: &'static str,
        extra: &'static [(&'static str, &'static str)],
        named: &'static str,
    }
    let refusals = [
        Refusal {
            what: "an <assign>",
            attributes: r#"datamodel="ecmascript""#,
            body: r#"<datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0"><onentry><assign location="n" expr="2"/></onentry></state>"#,
            extra: &[],
            named: "<assign>",
        },
        Refusal {
            what: "a <log expr>",
            attributes: r#"datamodel="ecmascript""#,
            body: r#"<state id="s0"><onentry><log expr="'x'"/></onentry></state>"#,
            extra: &[],
            named: "<log>",
        },
        Refusal {
            what: "a top-level <script>",
            attributes: r#"datamodel="ecmascript""#,
            body: r#"<script>var x = 1;</script><state id="s0"/>"#,
            extra: &[],
            named: "a top-level <script>",
        },
        Refusal {
            what: "a <data src>",
            attributes: r#"datamodel="ecmascript""#,
            body: r#"<datamodel><data id="x" src="probe.json"/></datamodel><state id="s0"/>"#,
            extra: &[("probe.json", r#"{"a":1}"#)],
            named: "a <data src>",
        },
        Refusal {
            what: "an inline <data> content",
            attributes: r#"datamodel="ecmascript""#,
            body: r#"<datamodel><data id="x">42</data></datamodel><state id="s0"/>"#,
            extra: &[],
            named: "an inline <data> content",
        },
        Refusal {
            what: "a function expression as an initialiser",
            attributes: r#"datamodel="ecmascript""#,
            body: r#"<datamodel><data id="f" expr="function() { return 1; }"/></datamodel><state id="s0"/>"#,
            extra: &[],
            named: "a function expression",
        },
        Refusal {
            what: "late binding of a state's own data",
            attributes: r#"datamodel="ecmascript" binding="late""#,
            body: r#"<state id="s0"><datamodel><data id="x" expr="1"/></datamodel></state>"#,
            extra: &[],
            named: r#"binding="late""#,
        },
        Refusal {
            what: "a <donedata> of a final state",
            attributes: r#"datamodel="ecmascript""#,
            body: r#"<datamodel><data id="n" expr="1"/></datamodel>
  <state id="s0"><transition target="done"/></state>
  <final id="done"><donedata><content expr="n"/></donedata></final>"#,
            extra: &[],
            named: "donedata",
        },
    ];
    for refusal in refusals {
        let (status, machine, said) =
            generate(&document(refusal.attributes, refusal.body), refusal.extra);
        assert_eq!(
            status,
            Some(7),
            "{} must be refused as an unsupported feature, not written: {said}",
            refusal.what
        );
        assert!(
            machine.is_empty(),
            "{} wrote a machine anyway: {machine}",
            refusal.what
        );
        assert!(
            said.contains(refusal.named) && said.contains("no Swift codegen path yet"),
            "{} is refused, but not by name ({}): {said}",
            refusal.what,
            refusal.named
        );
    }
}
