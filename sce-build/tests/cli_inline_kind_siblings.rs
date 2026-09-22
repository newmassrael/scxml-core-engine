// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! The COMMAND emits a sibling artifact for every kind a statechart
//! declares in place — on all six backends.
//!
//! `forge_conformance` asks the same question of the library entry, and
//! that is not the same question. Measured 2026-09-22: the library entry
//! emitted every sibling while `sce-codegen generate` emitted none, for
//! four hours, because the command carried its own copy of the backend
//! dispatch and the sibling emission had been wired into the library's.
//! Everything downstream — the CMake rules, the conformance harness, a
//! consumer's build — goes through the command, so the library passing
//! alone means the feature does not ship.
//!
//! The population is derived, never listed: the ids come out of the
//! fixture, and the languages out of `Language::ALL`. A list would
//! answer for the languages someone remembered to type.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The sce: extension namespace, as the fixtures and the parser spell it.
const SCE_EXT_NAMESPACE: &str = "http://sce.dev/ext";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build sits under the repository root")
        .to_path_buf()
}

/// Every `<data sce:kind>` id the fixture declares, in document order.
fn kinds_declared_in_place(fixture: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(fixture)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", fixture.display()));
    let doc = roxmltree::Document::parse(&text).expect("fixture parses as XML");
    doc.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "data")
        .filter(|n| n.attribute((SCE_EXT_NAMESPACE, "kind")).is_some())
        .map(|n| {
            n.attribute("id")
                .expect("a kind declared in place carries an id")
                .to_string()
        })
        .collect()
}

/// The artifact paths one `generate` run reports, read from the manifest
/// line it prints rather than from the directory: the manifest is what a
/// build system consumes, so a file written and not reported is a file
/// the build does not know about.
fn artifacts_of(stdout: &str) -> Vec<String> {
    let line = stdout
        .lines()
        .find(|l| l.starts_with('{'))
        .expect("generate prints one JSON manifest line");
    let manifest: serde_json::Value =
        serde_json::from_str(line).expect("the manifest line is JSON");
    manifest["artifacts"]
        .as_array()
        .expect("the manifest carries an artifacts array")
        .iter()
        .map(|a| {
            a["path"]
                .as_str()
                .expect("every artifact carries a path")
                .to_string()
        })
        .collect()
}

#[test]
fn the_command_emits_a_sibling_for_every_kind_declared_in_place() {
    let root = repo_root();
    let fixture = root.join("tests/forge/resources/inline_mixed.scxml");
    let declared = kinds_declared_in_place(&fixture);
    assert!(
        declared.len() >= 3,
        "the fixture is what gives this test its arity, and it now declares {} kind(s)",
        declared.len()
    );

    let out_root = tempfile::tempdir().expect("tempdir");
    for language in sce_build::generator::Language::ALL {
        let out_dir = out_root.path().join(format!("{language:?}"));
        std::fs::create_dir_all(&out_dir).expect("output dir");
        let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
            .arg("generate")
            .arg(&fixture)
            .arg("-o")
            .arg(&out_dir)
            .arg("-l")
            .arg(language.canonical_name())
            .current_dir(&root)
            .output()
            .expect("sce-codegen runs");
        assert!(
            run.status.success(),
            "generate ({language:?}) failed: {}",
            String::from_utf8_lossy(&run.stderr)
        );

        let artifacts = artifacts_of(&String::from_utf8_lossy(&run.stdout));
        for id in &declared {
            // `<machine>_<id>` is the contract; the extension and the
            // casing are the backend's, so the match is on the stem the
            // name is built from rather than on a per-language spelling
            // this test would have to keep a second copy of.
            let stem = sce_build::filters::to_snake_case(format!("inline_mixed_{id}"));
            let found = artifacts.iter().any(|path| {
                let file = Path::new(path)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();
                sce_build::filters::to_snake_case(file).starts_with(&stem)
            });
            assert!(
                found,
                "generate ({language:?}) reported no artifact for the kind declared as '{id}'. \
                 It reported: {artifacts:?}"
            );
            let on_disk = artifacts
                .iter()
                .filter(|path| Path::new(path).exists())
                .count();
            assert_eq!(
                on_disk,
                artifacts.len(),
                "generate ({language:?}) reported {} artifact(s) and {on_disk} of them exist",
                artifacts.len()
            );
        }
    }
}

#[test]
fn check_and_generate_refuse_an_invalid_inline_expression() {
    let root = repo_root();
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("invalid.scxml");
    std::fs::write(
        &input,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
        xmlns:sce="http://sce.dev/ext" initial="idle">
      <datamodel><data id="convert" sce:kind="transform"><datamodel>
        <data id="x" sce:type="int32" sce:direction="in"/>
        <data id="y" sce:type="int32" sce:direction="out" expr="missing + x"/>
      </datamodel></data></datamodel><state id="idle"/>
    </scxml>"#,
    )
    .unwrap();
    for language in sce_build::generator::Language::ALL {
        let mut refusals = Vec::new();
        for command in ["check", "generate"] {
            let mut run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"));
            run.current_dir(&root)
                .arg("--error-format=json")
                .arg(command)
                .arg(&input)
                .arg("-l")
                .arg(language.canonical_name());
            if command == "generate" {
                run.arg("-o").arg(dir.path().join("output"));
            }
            let result = run.output().unwrap();
            assert!(
                !result.status.success(),
                "{command} ({language:?}) accepted an undeclared inline name"
            );
            let stderr = String::from_utf8_lossy(&result.stderr);
            let diagnostic: serde_json::Value = serde_json::from_str(
                stderr
                    .lines()
                    .find(|s| s.starts_with('{'))
                    .expect("diagnostic"),
            )
            .unwrap();
            refusals.push((result.status.code(), diagnostic["code"].clone()));
        }
        assert_eq!(refusals[0], refusals[1], "{language:?}");
    }
}

#[test]
fn in_memory_compilation_keeps_the_inline_artifacts() {
    let fixture = repo_root().join("tests/forge/resources/inline_mixed.scxml");
    let source = std::fs::read_to_string(&fixture).unwrap();
    for &language in sce_build::generator::Language::ALL {
        let templates = sce_build::template_registry::embedded_templates_for(language);
        let memory = sce_build::compile_from_string_lang_typed(
            &source,
            "inline_mixed",
            &templates,
            language,
        )
        .unwrap();
        let disk = sce_build::compile_scxml_lang_typed(
            fixture.to_str().unwrap(),
            &sce_build::find_template_dir_for(language),
            language,
        )
        .unwrap();
        let names = |out: &sce_build::generator::GeneratedOutput| {
            out.files
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&memory), names(&disk), "{language:?}");
        for ((name, got), (_, want)) in
            memory
                .files
                .iter()
                .zip(&disk.files)
                .filter(|((name, _), _)| {
                    let name = sce_build::filters::to_snake_case(name.clone());
                    kinds_declared_in_place(&fixture).iter().any(|id| {
                        name.starts_with(&sce_build::filters::to_snake_case(format!(
                            "inline_mixed_{id}"
                        )))
                    })
                })
        {
            let body = |code: &str| {
                code.lines()
                    .filter(|line| !line.contains("SCE-MAP:"))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            assert_eq!(
                body(got),
                body(want),
                "{language:?}: embedded and file templates must emit the same sibling {name}"
            );
        }
    }
}

#[test]
fn an_inline_import_reaches_the_library_and_cli_dependency_channels() {
    let root = repo_root();
    let dir = tempfile::tempdir().unwrap();
    let imported = dir.path().join("session.scxml");
    std::fs::copy(
        root.join("tests/forge/resources/enum_session_state.scxml"),
        &imported,
    )
    .unwrap();
    let input = dir.path().join("machine.scxml");
    std::fs::write(
        &input,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
        xmlns:sce="http://sce.dev/ext" initial="idle">
      <sce:import src="session.scxml" kind="enum" as="session"/>
      <datamodel><data id="frame" sce:kind="codec"><datamodel>
        <sce:field id="state" sce:type="enum:session" sce:byte="0" sce:bit-size="8"/>
      </datamodel></data></datamodel><state id="idle"/>
    </scxml>"#,
    )
    .unwrap();
    let lang = sce_build::generator::Language::Cpp;
    let result = sce_build::compile_scxml_lang_typed(
        input.to_str().unwrap(),
        &sce_build::find_template_dir_for(lang),
        lang,
    )
    .unwrap();
    assert!(
        result.deps.contains(&imported.canonicalize().unwrap()),
        "inline import absent from deps: {:?}",
        result.deps
    );
    let depfile = dir.path().join("machine.d");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .current_dir(&root)
        .arg("generate")
        .arg(&input)
        .args(["-l", "cpp", "-o"])
        .arg(dir.path().join("out"))
        .arg("--write-deps")
        .arg(&depfile)
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(std::fs::read_to_string(depfile)
        .unwrap()
        .contains("session.scxml"));
}

#[test]
fn an_ineligible_kind_is_refused_instead_of_becoming_a_variable() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("invalid.scxml");
    for kind in ["timer", "unknown-kind"] {
        std::fs::write(
            &input,
            format!(
                r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
            xmlns:sce="http://sce.dev/ext" initial="idle">
          <datamodel><data id="value" sce:kind="{kind}"/></datamodel><state id="idle"/>
        </scxml>"#
            ),
        )
        .unwrap();
        let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
            .args(["--error-format=json", "check"])
            .arg(&input)
            .output()
            .unwrap();
        assert!(!run.status.success());
        let diagnostic: serde_json::Value = serde_json::from_slice(&run.stderr).unwrap();
        if kind == "timer" {
            assert_eq!(diagnostic["code"], "validation/kind-not-inline-eligible");
            assert_eq!(diagnostic["actual"], kind);
        } else {
            // The XSD can refuse an unknown kind before the typed parser.
            assert!(matches!(
                diagnostic["code"].as_str(),
                Some("xml/schema-validation" | "validation/kind-not-inline-eligible")
            ));
        }
    }
}

#[test]
fn an_inline_sibling_cannot_overwrite_the_machine() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("collision.scxml");
    std::fs::write(
        &input,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
        xmlns:sce="http://sce.dev/ext" initial="idle">
      <datamodel><data id="sm" sce:kind="transform"><datamodel>
        <data id="x" sce:type="int32" sce:direction="in"/>
        <data id="y" sce:type="int32" sce:direction="out" expr="x + 1"/>
      </datamodel></data></datamodel><state id="idle"/>
    </scxml>"#,
    )
    .unwrap();
    let output = dir.path().join("out");
    let run = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(["--error-format=json", "generate"])
        .arg(&input)
        .args(["-l", "rust", "-o"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(!run.status.success());
    let diagnostic: serde_json::Value = serde_json::from_slice(&run.stderr).unwrap();
    assert_eq!(diagnostic["code"], "validation/duplicate-id");
    assert!(!output.join("collision_sm.rs").exists());
}
