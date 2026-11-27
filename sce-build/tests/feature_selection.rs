// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A selection of features (ADR 0057 §6): `generate --features` generates the
//! variant it denotes, `check --features` judges it, `project` prints it, a
//! selection the model does not admit is refused, and a gated document is not
//! generated without one. Over a set of documents (`generate --with-imports`,
//! `orchestrate`, `check` over a set) the selection is the set's: each
//! statechart written against a model reads the variant, held to its own
//! model, every other document is read as written, and a selection no
//! document of the set reads is refused.

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
        code.contains(r#"#[cfg(feature = "session-unicast-accept")] Accepting ="#),
        "the gated state is a variant under its presence"
    );
    // A state's number is its place in the build's own states, so `open` is
    // written once for each number it has, each under the builds that give it.
    let open: Vec<&str> = code.lines().filter(|l| l.contains(" Open = ")).collect();
    assert!(
        open.len() == 2 && open.iter().all(|l| l.contains("#[cfg(")),
        "every declaration of the gated state is under its presence: {open:?}"
    );
    // The machine refuses a build that breaks the model's edge (ADR 0057 §6),
    // since its conditions leave the implied feature out.
    assert!(
        code.contains(
            "#[cfg(all(feature = \"session-unicast-accept\", not(feature = \"session-unicast\")))]\n\
             compile_error!(\"feature `session-unicast-accept` requires `session-unicast`, \
             which this build leaves out (feature model `features`)\");"
        ),
        "the model's edge is a refusal in the machine"
    );
    let check = sce_codegen(&["check", path(&machine), "-l", "rust"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
}

/// The C11 backend keeps the variability too (ADR 0057 §10): one source and
/// one header for every build, each member under a preprocessor conditional
/// over the features' macros, held build by build to each variant, and the
/// header refusing a build that breaks the model's edge.
#[test]
fn the_c11_backend_writes_one_machine_for_every_build() {
    let (dir, machine) = product_line();
    let out = dir.path().join("out");
    let run = sce_codegen(&["generate", path(&machine), "-l", "c11", "-o", path(&out)]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let source = std::fs::read_to_string(out.join("machine_sm.c")).expect("the machine");
    assert!(
        source.contains("#if defined(SCE_FEATURE_SESSION_UNICAST_ACCEPT)\n"),
        "the gated state's code is under its presence"
    );
    let header = std::fs::read_to_string(out.join("machine_sm.h")).expect("the header");
    assert!(
        header.contains(
            "#if (defined(SCE_FEATURE_SESSION_UNICAST_ACCEPT) && !defined(SCE_FEATURE_SESSION_UNICAST))\n\
             #error \"feature `session-unicast-accept` requires `session-unicast`, \
             which this build leaves out (feature model `features`)\"\n#endif\n"
        ),
        "the model's edge is a refusal in the header"
    );
    let check = sce_codegen(&["check", path(&machine), "-l", "c11"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
}

/// The C++ backend keeps the variability as C11 does (ADR 0057 §10), in its
/// header and in the method bodies the header includes. The header is
/// formatted after it is written, so the refusal is asked for by its
/// condition and its message, not by how its line is broken.
#[test]
fn the_cpp_backend_writes_one_machine_for_every_build() {
    let (dir, machine) = product_line();
    let out = dir.path().join("out");
    let run = sce_codegen(&["generate", path(&machine), "-l", "cpp", "-o", path(&out)]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let methods = std::fs::read_to_string(out.join("machine_sm.inl")).expect("the methods");
    assert!(
        methods.contains("#if defined(SCE_FEATURE_SESSION_UNICAST_ACCEPT)\n"),
        "the gated state's code is under its presence"
    );
    let header = std::fs::read_to_string(out.join("machine_sm.h")).expect("the header");
    assert!(
        header.contains(
            "#if (defined(SCE_FEATURE_SESSION_UNICAST_ACCEPT) && !defined(SCE_FEATURE_SESSION_UNICAST))\n\
             #error"
        ) && header.contains(
            "\"feature `session-unicast-accept` requires `session-unicast`, \
             which this build leaves out (feature model `features`)\""
        ),
        "the model's edge is a refusal in the header"
    );
    let check = sce_codegen(&["check", path(&machine), "-l", "cpp"]);
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
}

/// The Swift backend keeps the variability with Swift's conditional compilation
/// blocks (ADR 0057 §10), over the features' compilation conditions, and its
/// source refuses a build that breaks the model's edge with an `#error`.
#[test]
fn the_swift_backend_writes_one_machine_for_every_build() {
    let (dir, machine) = product_line();
    let out = dir.path().join("out");
    let run = sce_codegen(&["generate", path(&machine), "-l", "swift", "-o", path(&out)]);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let source = std::fs::read_to_string(out.join("machine_sm.swift")).expect("the machine");
    assert!(
        source.contains("#if SCE_FEATURE_SESSION_UNICAST_ACCEPT\n"),
        "the gated state's code is under its presence"
    );
    assert!(
        source.ends_with(
            "#if (SCE_FEATURE_SESSION_UNICAST_ACCEPT && !SCE_FEATURE_SESSION_UNICAST)\n\
             #error(\"feature `session-unicast-accept` requires `session-unicast`, \
             which this build leaves out (feature model `features`)\")\n#endif\n"
        ),
        "the model's edge is a refusal in the source"
    );
    let check = sce_codegen(&["check", path(&machine), "-l", "swift"]);
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

/// A statechart that imports no feature model.
const PLAIN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="running" name="plain">
  <state id="running">
    <transition event="stop" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// A second feature model: one feature the first declares, and one it does not.
const OTHER_FEATURES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="feature-model" name="other_features">
  <sce:feature name="session-unicast"/>
  <sce:feature name="telemetry"/>
</scxml>
"#;

/// A statechart written against the second model.
const OTHER_MACHINE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="quiet" name="other">
  <sce:import kind="feature-model" src="other_features.scxml" as="features"/>
  <state id="quiet">
    <transition event="report" target="reporting" sce:requires="telemetry"/>
  </state>
  <state id="reporting" sce:requires="telemetry"/>
</scxml>
"#;

/// A deploy whose one machine is the product line's statechart.
const DEPLOY: &str = r#"
version: "1.0"
topology:
  host:
    machines:
      machine:
        source: machine.scxml
"#;

/// A directory holding the product line (`features.scxml`, `machine.scxml`), a
/// statechart that imports no model (`plain.scxml`), a second product line
/// over a second model (`other_features.scxml`, `other.scxml`) and a deploy
/// of the first machine (`deploy.yaml`).
fn set_of_documents() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (file, text) in [
        ("features.scxml", FEATURES),
        ("machine.scxml", MACHINE),
        ("plain.scxml", PLAIN),
        ("other_features.scxml", OTHER_FEATURES),
        ("other.scxml", OTHER_MACHINE),
        ("deploy.yaml", DEPLOY),
    ] {
        std::fs::write(dir.path().join(file), text).unwrap();
    }
    dir
}

/// `generate --with-imports --features`: the statechart written against the
/// model is generated as the variant the selection denotes, and the model the
/// run reaches and a statechart that imports no model are generated as
/// written, though each named alone with the selection refuses it.
#[test]
fn a_set_generates_the_variant_of_each_statechart_written_against_a_model() {
    let dir = set_of_documents();
    let machine = dir.path().join("machine.scxml");
    let plain = dir.path().join("plain.scxml");
    let generate = |language: &str, out: &Path| {
        sce_codegen(&[
            "--error-format",
            "json",
            "generate",
            path(&machine),
            path(&plain),
            "--with-imports",
            "-l",
            language,
            "-o",
            path(out),
            "--features",
            "session-unicast",
        ])
    };

    let out = dir.path().join("rust");
    let run = generate("rust", &out);
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
    assert!(
        !code.contains("feature = \"session-unicast"),
        "one variant was written, not the machine for every build"
    );
    assert!(
        out.join("plain_sm.rs").is_file(),
        "the statechart that imports no model was not generated"
    );

    // C11 keeps the variability, so without a selection it writes the machine
    // for every build; with one it writes the variant, as Rust does.
    let out = dir.path().join("c11");
    let run = generate("c11", &out);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let source = std::fs::read_to_string(out.join("machine_sm.c")).expect("the machine");
    assert!(
        !source.contains("SCE_FEATURE_SESSION_UNICAST"),
        "one variant was written, not the machine for every build"
    );
}

/// `sce-codegen --error-format json <command> --document <machine>
/// --with-imports --deploy <deploy> -l rust`, then `extra`.
fn over_the_deployed_set(dir: &Path, command: &str, extra: &[&str]) -> Output {
    let machine = dir.join("machine.scxml");
    let deploy = dir.join("deploy.yaml");
    let mut args = vec![
        "--error-format",
        "json",
        command,
        "--document",
        path(&machine),
        "--with-imports",
        "--deploy",
        path(&deploy),
        "-l",
        "rust",
    ];
    args.extend_from_slice(extra);
    sce_codegen(&args)
}

/// `orchestrate --deploy --with-imports` generates a set holding a gated
/// statechart only under a selection, and with one writes the variant;
/// `check` over the same set reaches the same verdict both ways.
#[test]
fn a_deployed_set_is_generated_and_checked_under_its_selection() {
    let dir = set_of_documents();

    let out = dir.path().join("unselected");
    let orchestrated = over_the_deployed_set(dir.path(), "orchestrate", &["-o", path(&out)]);
    assert!(
        !orchestrated.status.success(),
        "a set holding a gated statechart was generated with no selection"
    );
    assert_eq!(
        first_code(&orchestrated),
        "codegen/feature-selection-required"
    );
    let checked = over_the_deployed_set(dir.path(), "check", &[]);
    assert!(
        !checked.status.success(),
        "check disagreed with orchestrate"
    );
    assert_eq!(first_code(&checked), "codegen/feature-selection-required");

    let out = dir.path().join("selected");
    let orchestrated = over_the_deployed_set(
        dir.path(),
        "orchestrate",
        &["-o", path(&out), "--features", "session-unicast"],
    );
    assert!(
        orchestrated.status.success(),
        "{}",
        String::from_utf8_lossy(&orchestrated.stderr)
    );
    let code = std::fs::read_to_string(out.join("machine_sm.rs")).expect("the machine");
    assert!(code.contains("Open"), "the variant keeps `open`");
    assert!(
        !code.contains("Accepting"),
        "the variant has no `accepting`"
    );
    let checked = over_the_deployed_set(dir.path(), "check", &["--features", "session-unicast"]);
    assert!(
        checked.status.success(),
        "check disagreed with orchestrate: {}",
        String::from_utf8_lossy(&checked.stderr)
    );
}

/// A selection that no document of the set reads selects nothing and is
/// refused, on every route over a set, before anything is written.
#[test]
fn a_selection_no_document_of_a_set_reads_is_refused() {
    let dir = set_of_documents();
    let plain = dir.path().join("plain.scxml");
    let model = dir.path().join("features.scxml");
    let out = dir.path().join("out");
    let runs = [
        (
            "generate --with-imports",
            sce_codegen(&[
                "--error-format",
                "json",
                "generate",
                path(&plain),
                "--with-imports",
                "-l",
                "rust",
                "-o",
                path(&out),
                "--features",
                "session-unicast",
            ]),
        ),
        // The model itself is no document written against a model.
        (
            "generate --with-imports of the model",
            sce_codegen(&[
                "--error-format",
                "json",
                "generate",
                path(&model),
                "--with-imports",
                "-l",
                "rust",
                "-o",
                path(&out),
                "--features",
                "session-unicast",
            ]),
        ),
        (
            "orchestrate",
            sce_codegen(&[
                "--error-format",
                "json",
                "orchestrate",
                "--document",
                path(&plain),
                "-l",
                "rust",
                "-o",
                path(&out),
                "--features",
                "session-unicast",
            ]),
        ),
        (
            "check",
            sce_codegen(&[
                "--error-format",
                "json",
                "check",
                "--document",
                path(&plain),
                "-l",
                "rust",
                "--features",
                "session-unicast",
            ]),
        ),
    ];
    for (command, run) in &runs {
        assert!(
            !run.status.success(),
            "{command}: a selection that selects nothing was accepted"
        );
        assert_eq!(
            first_code(run),
            "validation/feature-selection-without-model",
            "{command}"
        );
    }
    assert!(!out.exists(), "a refused run wrote");
}

/// The selection is one configuration of the build the set is generated for:
/// each statechart holds the whole of it to its own model. Two statecharts
/// written against two models are generated together under a selection both
/// models admit, and a name one model does not declare is refused for the
/// statechart written against it, never dropped for it.
#[test]
fn each_statechart_of_a_set_holds_the_selection_to_its_own_model() {
    let dir = set_of_documents();
    let machine = dir.path().join("machine.scxml");
    let other = dir.path().join("other.scxml");
    let generate = |features: &str, out: &Path| {
        sce_codegen(&[
            "--error-format",
            "json",
            "generate",
            path(&machine),
            path(&other),
            "--with-imports",
            "-l",
            "rust",
            "-o",
            path(out),
            "--features",
            features,
        ])
    };

    let out = dir.path().join("admitted");
    let run = generate("session-unicast", &out);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let code = std::fs::read_to_string(out.join("other_sm.rs")).expect("the other machine");
    assert!(code.contains("Quiet"), "the variant keeps `quiet`");
    assert!(
        !code.contains("Reporting"),
        "the variant has no `reporting`"
    );

    // `telemetry` is a feature of the second model and not of the first.
    let out = dir.path().join("refused");
    let run = generate("session-unicast,telemetry", &out);
    assert!(
        !run.status.success(),
        "a name the first model does not declare was dropped for its statechart"
    );
    assert_eq!(first_code(&run), "validation/feature-selection-undeclared");
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("telemetry"),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

/// Whether a document is written against a feature model is read off its root,
/// once, for every route over a set: the product line's statechart is, and the
/// model itself, a statechart that imports none and a text that is not XML are
/// not.
#[test]
fn a_document_is_written_against_a_model_when_its_root_imports_one() {
    use sce_build::product_line::imports_a_feature_model;
    assert!(imports_a_feature_model(MACHINE));
    assert!(!imports_a_feature_model(FEATURES));
    assert!(!imports_a_feature_model(PLAIN));
    assert!(!imports_a_feature_model("<scxml"));

    let dir = set_of_documents();
    let written = |file: &str| {
        sce_build::written_against_a_feature_model(&dir.path().join(file), &[])
            .expect("the document is read")
    };
    assert!(written("machine.scxml"));
    assert!(!written("features.scxml"));
    assert!(!written("plain.scxml"));
}
