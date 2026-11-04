// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The forge AST and the sourcemap carry each element's own and effective
//! condition (ADR 0057 §7): `requires`, what the element writes itself, and
//! `presence`, its own and those of the elements around it, both in canonical
//! form, the same `presence` the transition table carries for the same node.
//! A node present in every variant carries neither, and so does every node of
//! a variant's own map.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

use sce_build::forge::symbol_mangling::mangle;

const FEATURES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="feature-model" name="session_features">
  <sce:feature name="session-auth"/>
  <sce:feature name="session-unicast"/>
  <sce:feature name="session-unicast-accept" requires="session-unicast"/>
</scxml>
"#;

const STATUS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0" sce:kind="enum" name="status" sce:underlying-type="uint8">
  <datamodel>
    <data id="variants">
      <sce:variant name="open" value="2"/>
      <sce:variant name="done" value="7"/>
    </data>
  </datamodel>
</scxml>
"#;

/// A machine whose elements differ in what they write and what they are
/// present under: a gated state and the transition inside it, which writes
/// nothing; a gated `<onentry>`, whose actions take its condition, one of them
/// writing one of its own; and, when `import`, an `<sce:import>` under a
/// condition.
fn machine(import: bool) -> String {
    let gated_import = if import {
        r#"  <sce:import kind="enum" src="status.scxml" as="status" requires="session-auth"/>"#
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" name="m" initial="idle" datamodel="sce-static">
  <sce:import kind="feature-model" src="features.scxml" as="features"/>
{gated_import}
  <state id="idle">
    <onentry sce:requires="session-unicast">
      <sce:action name="greet" effect="notice"/>
      <sce:action name="secure" effect="notice" requires="session-auth"/>
    </onentry>
    <transition event="go" target="busy" sce:requires="session-unicast-accept"/>
    <transition event="stop" target="done"/>
  </state>
  <state id="busy" sce:requires="session-unicast-accept">
    <transition event="stop" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#
    )
}

fn product_line(import: bool) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("features.scxml"), FEATURES).unwrap();
    std::fs::write(dir.path().join("status.scxml"), STATUS).unwrap();
    let path = dir.path().join("m.scxml");
    std::fs::write(&path, machine(import)).unwrap();
    (dir, path)
}

/// `node`'s `presence` and `requires`, `null` where it carries none. A path
/// that names no node fails here rather than reading as a node with neither.
fn conditions(node: &Value) -> (Value, Value) {
    assert!(node.is_object(), "no node here: {node}");
    (
        node.get("presence").cloned().unwrap_or(Value::Null),
        node.get("requires").cloned().unwrap_or(Value::Null),
    )
}

#[test]
fn the_ast_carries_each_elements_effective_and_own_condition() {
    let (_dir, path) = product_line(true);
    let model = sce_build::parser::SCXMLParser::new()
        .parse_file(path.to_str().unwrap())
        .expect("the document parses");
    let parsed = sce_build::forge::ast_export::statechart_parsed_forge(model);
    let ast = serde_json::to_value(&parsed).expect("the AST serialises");
    let chart = &ast["document"];
    assert_eq!(chart["kind"], "statechart");
    let states = &chart["states"];
    let accept = json!([["session-unicast-accept"]]);
    let unicast = json!([["session-unicast"]]);
    let auth = json!([["session-auth"]]);

    assert_eq!(conditions(&states["idle"]), (Value::Null, Value::Null));
    assert_eq!(
        conditions(&states["busy"]),
        (accept.clone(), accept.clone())
    );
    assert_eq!(
        conditions(&states["busy"]["transitions"][0]),
        (accept.clone(), Value::Null),
        "a transition takes its state's condition and writes none"
    );
    let idle = &states["idle"];
    assert_eq!(
        conditions(&idle["transitions"][0]),
        (accept.clone(), accept)
    );
    assert_eq!(
        conditions(&idle["transitions"][1]),
        (Value::Null, Value::Null)
    );
    assert_eq!(
        conditions(&idle["on_entry_blocks"][0][0]),
        (unicast, Value::Null),
        "an action takes its gated block's condition"
    );
    assert_eq!(
        conditions(&idle["on_entry_blocks"][0][1]),
        (json!([["session-auth", "session-unicast"]]), auth.clone()),
        "an action's own condition meets its block's"
    );

    let imports = chart["forge_imports"].as_array().expect("the imports");
    let of = |alias: &str| {
        imports
            .iter()
            .find(|import| import["alias"] == alias)
            .unwrap_or_else(|| panic!("no import `{alias}`: {imports:?}"))
    };
    assert_eq!(conditions(of("status")), (auth.clone(), auth));
    assert_eq!(conditions(of("features")), (Value::Null, Value::Null));
}

/// Generate `machine` for Rust into `out`, the variant of `features` when
/// given, and read the sidecar the run wrote.
fn sidecar(machine: &Path, out: &Path, features: Option<&[&str]>) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sce-codegen"));
    command
        .arg("generate")
        .arg(machine)
        .args(["-l", "rust", "-o"])
        .arg(out);
    if let Some(features) = features {
        command.arg(format!("--features={}", features.join(",")));
    }
    let run = command.output().expect("spawn sce-codegen");
    assert!(
        run.status.success(),
        "generate {features:?} failed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let text = std::fs::read_to_string(out.join("sce_sourcemap.json")).expect("a sidecar");
    serde_json::from_str(&text).expect("the sidecar is JSON")
}

fn violations(sidecar: &Value) -> Vec<String> {
    let schema: Value =
        serde_json::from_str(include_str!("../../schemas/sce-sourcemap.v1.schema.json"))
            .expect("the schema is JSON");
    let validator = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema)
        .expect("the schema compiles");
    let outcome = validator.validate(sidecar);
    match outcome {
        Ok(()) => Vec::new(),
        Err(errors) => errors.map(|e| e.to_string()).collect(),
    }
}

#[test]
fn the_sourcemap_carries_each_symbols_conditions_and_a_variants_carries_none() {
    let (dir, path) = product_line(false);
    let document = sidecar(&path, &dir.path().join("every-build"), None);
    assert_eq!(violations(&document), Vec::<String>::new());
    let symbol = |state: &str, artifact: &str| {
        let name = mangle("m", state, artifact);
        document["symbols"]
            .get(&name)
            .cloned()
            .unwrap_or_else(|| panic!("no symbol `{name}`: {}", document["symbols"]))
    };
    let accept = json!([["session-unicast-accept"]]);
    assert_eq!(
        conditions(&symbol("busy", "_state_body")),
        (accept.clone(), accept.clone())
    );
    assert_eq!(
        conditions(&symbol("busy", "_transition_0")),
        (accept.clone(), Value::Null)
    );
    assert_eq!(
        conditions(&symbol("idle", "_transition_0")),
        (accept.clone(), accept)
    );
    assert_eq!(
        conditions(&symbol("idle", "_on_entry_0_1")),
        (
            json!([["session-auth", "session-unicast"]]),
            json!([["session-auth"]])
        )
    );
    for (state, artifact) in [
        ("", "_machine"),
        ("idle", "_state_body"),
        ("idle", "_transition_1"),
    ] {
        assert_eq!(
            conditions(&symbol(state, artifact)),
            (Value::Null, Value::Null),
            "`{artifact}` of `{state}` is in every variant"
        );
    }

    let variant = sidecar(
        &path,
        &dir.path().join("variant"),
        Some(&["session-unicast", "session-unicast-accept"]),
    );
    assert_eq!(violations(&variant), Vec::<String>::new());
    for (name, symbol) in variant["symbols"].as_object().expect("the symbols") {
        assert_eq!(
            conditions(symbol),
            (Value::Null, Value::Null),
            "`{name}` of a variant's own map carries a condition"
        );
    }
}
