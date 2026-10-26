// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A selection of features (ADR 0057 §6): `generate --features` generates the
//! variant it denotes, `check --features` judges it, `project` prints it, a
//! selection the model does not admit is refused, and a gated document is not
//! generated without one.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sce_build::forge::error::{ForgeError, ValidationError};
use sce_build::parser::SCXMLParser;

const FEATURES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="feature-model" name="session_features">
  <sce:feature name="session-unicast"/>
  <sce:feature name="session-unicast-accept" requires="session-unicast"/>
</scxml>
"#;

const MACHINE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" name="machine">
  <sce:import kind="feature-model" src="features.scxml" as="features"/>
  <state id="idle">
    <transition event="inbound.start" target="accepting" sce:requires="session-unicast-accept"/>
    <transition event="connect" target="open" sce:requires="session-unicast"/>
  </state>
  <state id="accepting" sce:requires="session-unicast-accept">
    <transition event="done" target="open"/>
  </state>
  <state id="open" sce:requires="session-unicast"/>
</scxml>
"#;

/// A directory holding the model and the machine; the machine's path.
fn product_line() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("features.scxml"), FEATURES).unwrap();
    let path = dir.path().join("machine.scxml");
    std::fs::write(&path, MACHINE).unwrap();
    (dir, path)
}

fn sce_codegen(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(args)
        .output()
        .expect("spawn sce-codegen")
}

fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// The `code` of the first JSON record on `output`'s stderr.
fn first_code(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let record = stderr
        .lines()
        .find(|line| line.starts_with('{'))
        .unwrap_or_else(|| panic!("no JSON record on stderr:\n{stderr}"));
    let value: serde_json::Value = serde_json::from_str(record).expect("a JSON record");
    value["code"].as_str().unwrap_or_default().to_string()
}

/// A backend that writes one variant at a time needs the variant named.
#[test]
fn a_gated_document_is_not_generated_without_a_selection() {
    let (dir, machine) = product_line();
    let out = dir.path().join("out");
    let run = sce_codegen(&[
        "--error-format",
        "json",
        "generate",
        path(&machine),
        "-l",
        "go",
        "-o",
        path(&out),
    ]);
    assert!(
        !run.status.success(),
        "a gated document was generated as written"
    );
    assert_eq!(first_code(&run), "codegen/feature-selection-required");
}

/// The Rust backend keeps the variability (ADR 0057 §9): with no selection it
/// writes one machine for every build, each element under its presence, and
/// the generation has held that machine, build by build, to what each
/// variant generates alone. `check` reaches the same verdict.
#[test]
fn the_rust_backend_writes_one_machine_for_every_build() {
    let (dir, machine) = product_line();
    let out = dir.path().join("out");
    let run = sce_codegen(&["generate", path(&machine), "-l", "rust", "-o", path(&out)]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let code = std::fs::read_to_string(out.join("machine_sm.rs")).expect("the machine");
    assert!(
        code.contains(r#"#[cfg(feature = "session-unicast-accept")] Accepting,"#),
        "the gated state is a variant under its presence"
    );
    assert!(
        code.contains(r#"#[cfg(feature = "session-unicast")] Open,"#),
        "the gated state is a variant under its presence"
    );
    let check = sce_codegen(&["check", path(&machine), "-l", "rust"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
}

#[test]
fn a_selection_generates_the_variant_it_denotes() {
    let (dir, machine) = product_line();
    let out = dir.path().join("out");
    let run = sce_codegen(&[
        "generate",
        path(&machine),
        "-l",
        "rust",
        "-o",
        path(&out),
        "--features",
        "session-unicast",
    ]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let code = std::fs::read_to_string(out.join("machine_sm.rs")).expect("the machine");
    assert!(code.contains("Open"), "the variant keeps `open`");
    assert!(
        !code.contains("Accepting"),
        "the variant has no `accepting`"
    );
}

#[test]
fn project_prints_the_variant_the_generator_reads_with_every_position_kept() {
    let (_dir, machine) = product_line();
    let run = sce_codegen(&["project", path(&machine), "--features", "session-unicast"]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let variant = String::from_utf8(run.stdout).unwrap();
    assert_eq!(variant.lines().count(), MACHINE.lines().count());
    for (kept, original) in variant.lines().zip(MACHINE.lines()) {
        assert_eq!(kept.chars().count(), original.chars().count());
    }
    assert!(!variant.contains("accepting"));
    assert!(!variant.contains("requires"));
    assert!(variant.contains(r#"<state id="open""#));

    // The library's projection is the same text.
    let projected = SCXMLParser::new()
        .project_file(path(&machine), vec!["session-unicast".into()])
        .expect("the variant");
    assert_eq!(projected, variant);

    // `--features` alone is the variant with no feature on.
    let none = sce_codegen(&["project", path(&machine), "--features"]);
    assert!(
        none.status.success(),
        "{}",
        String::from_utf8_lossy(&none.stderr)
    );
    let none = String::from_utf8(none.stdout).unwrap();
    assert!(!none.contains(r#"<state id="open""#) && none.contains(r#"<state id="idle""#));
}

#[test]
fn check_judges_the_variant_a_selection_denotes() {
    let (_dir, machine) = product_line();
    let run = sce_codegen(&[
        "check",
        path(&machine),
        "-l",
        "rust",
        "--features",
        "session-unicast,session-unicast-accept",
    ]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    // A selection that leaves out a requirement is refused, not completed.
    let run = sce_codegen(&[
        "--error-format",
        "json",
        "check",
        path(&machine),
        "-l",
        "rust",
        "--features",
        "session-unicast-accept",
    ]);
    assert!(!run.status.success());
    assert_eq!(first_code(&run), "validation/feature-selection-incomplete");
}

#[test]
fn a_selection_is_held_to_the_model() {
    let (_dir, machine) = product_line();
    let refusal = |features: Vec<String>| {
        let err = SCXMLParser::new()
            .with_features(features)
            .parse_file(path(&machine))
            .expect_err("the selection is refused");
        match err.error {
            ForgeError::Validation(v) => *v,
            other => panic!("{other:?}"),
        }
    };
    assert!(matches!(
        refusal(vec!["session-unicst".into()]),
        ValidationError::FeatureSelectionUndeclared { feature, .. } if feature == "session-unicst"
    ));
    assert!(matches!(
        refusal(vec!["session-unicast-accept".into()]),
        ValidationError::FeatureSelectionIncomplete { feature, missing, .. }
            if feature == "session-unicast-accept" && missing == "session-unicast"
    ));
}

/// A variant's states keep the ranks and the generated ids the document gives
/// them (ADR 0057 §9): removing a gated state before an unnamed one renames
/// nothing and moves nothing, so every variant spells a state alike.
#[test]
fn a_variant_inherits_the_names_and_the_order_of_its_document() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("features.scxml"), FEATURES).unwrap();
    let machine = dir.path().join("machine.scxml");
    std::fs::write(
        &machine,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" name="machine">
  <sce:import kind="feature-model" src="features.scxml" as="features"/>
  <state id="idle">
    <transition event="go" target="last"/>
  </state>
  <state sce:requires="session-unicast"/>
  <state>
    <history>
      <transition target="inner"/>
    </history>
    <state id="inner"/>
  </state>
  <state id="last"/>
</scxml>
"#,
    )
    .unwrap();
    let document = SCXMLParser::new()
        .parse_file(path(&machine))
        .expect("the document parses");
    let variant = SCXMLParser::new()
        .with_features(Vec::new())
        .parse_file(path(&machine))
        .expect("the variant parses");
    let removed: Vec<&String> = document
        .states
        .keys()
        .filter(|id| !variant.states.contains_key(*id))
        .collect();
    let gated: Vec<&String> = document
        .states
        .iter()
        .filter(|(_, state)| state.presence.is_some())
        .map(|(id, _)| id)
        .collect();
    assert_eq!(removed, gated);
    assert_eq!(gated.len(), 1);
    for (id, state) in &variant.states {
        let theirs = document
            .states
            .get(id)
            .unwrap_or_else(|| panic!("the variant names `{id}`, which the document does not"));
        assert_eq!(state.document_order, theirs.document_order, "{id}");
    }
    let histories = |model: &sce_build::model::SCXMLModel| {
        model.history_states.keys().cloned().collect::<Vec<_>>()
    };
    assert_eq!(histories(&variant), histories(&document));
    assert_eq!(histories(&variant).len(), 1);
}

#[test]
fn a_selection_for_a_document_without_a_model_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("plain.scxml");
    std::fs::write(
        &plain,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a" name="plain">
  <state id="a"/>
</scxml>"#,
    )
    .unwrap();
    let run = sce_codegen(&[
        "--error-format",
        "json",
        "generate",
        path(&plain),
        "-l",
        "rust",
        "-o",
        path(&dir.path().join("out")),
        "--features",
        "session-unicast",
    ]);
    assert!(!run.status.success());
    assert_eq!(
        first_code(&run),
        "validation/feature-selection-without-model"
    );
}
