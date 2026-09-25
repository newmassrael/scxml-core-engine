// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-5.3 — a typed reader never lands on a member the generated type
// already carries.
//
// `reader_names` reads the reserved names off the templates, and a template
// does not show every name it produces: a member built from a document id
// (`history_<state>`, `<machine>_on_entry_<state>`) is a prefix in the
// template and a whole name only in the output. So this renders real
// documents in every backend, scans each output for the members it defines,
// and holds every one of them to the reserved set. A member some template
// grows that the scan of the templates does not see fails here, rather than
// as a reader that duplicates it in somebody's generated code.
//
// And the half the reserved set exists for: a document whose `<data>` ids
// are the names that broke — keywords, members, ids that fold together —
// generates in every backend, with each refusal published in the manifest.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use sce_build::generator::Language;
use sce_build::reader_names::{rendered_members, reserved};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn out_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sce-reader-names-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// Generate `fixture` for `language` into a fresh directory; return the
/// manifest and the artifacts' paths.
fn generate(
    fixture: &Path,
    language: Language,
    extra: &[&str],
    tag: &str,
) -> (serde_json::Value, Vec<PathBuf>) {
    let dir = out_dir(tag);
    let out = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("generate")
        .arg(fixture)
        .args(["-l", language.canonical_name(), "-o"])
        .arg(&dir)
        .args(extra)
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{} {}: generation failed:\n{}",
        fixture.display(),
        language.canonical_name(),
        String::from_utf8_lossy(&out.stderr)
    );
    let manifest: serde_json::Value =
        serde_json::from_str(stdout.lines().last().expect("a manifest line"))
            .expect("the manifest is JSON");
    let artifacts = manifest["artifacts"]
        .as_array()
        .expect("artifacts")
        .iter()
        .map(|a| PathBuf::from(a["path"].as_str().expect("artifact path")))
        .collect();
    (manifest, artifacts)
}

/// The document's own model, analyzed as the generator analyzes it.
fn model(fixture: &Path) -> sce_build::model::SCXMLModel {
    let path = fixture.to_string_lossy();
    let mut model = sce_build::parser::SCXMLParser::new()
        .parse_file(&path)
        .expect("fixture parses");
    sce_build::analyzer::analyze(&mut model, &path);
    model
}

/// Documents whose outputs between them reach the members a generated type
/// can carry: parallel regions and history, a local `<invoke>` with
/// `<finalize>`, host-run invokes, and a large real machine.
fn fixtures() -> Vec<(PathBuf, Vec<&'static str>)> {
    let root = repo_root();
    vec![
        (root.join("sce-build/tests/fixtures/no_std/parallel_history_probe.scxml"), vec![]),
        (
            root.join("sce-build/tests/fixtures/host_processor/statechart_host_invoker.scxml"),
            vec!["--host-invoker", "x-sce-host"],
        ),
        (
            root.join(
                "integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location.scxml",
            ),
            vec![],
        ),
        (root.join("examples/ai_loop/ai_loop.scxml"), vec![]),
    ]
}

#[test]
fn every_member_a_generated_type_defines_is_reserved() {
    let mut unreserved: BTreeSet<String> = BTreeSet::new();
    for (index, (fixture, extra)) in fixtures().into_iter().enumerate() {
        let model = model(&fixture);
        for &language in Language::ALL {
            let readers: BTreeSet<String> = model
                .readable_variables
                .iter()
                .filter_map(|v| v.reader.as_ref())
                .map(|r| {
                    r.get(language)
                        .trim_start_matches("r#")
                        .trim_matches('`')
                        .to_string()
                })
                .collect();
            let (_, artifacts) = generate(
                &fixture,
                language,
                &extra,
                &format!("{index}-{}", language.canonical_name()),
            );
            // Every machine the run emitted — a local `<invoke>`'s child is
            // one too — is read with its own C11 symbol prefix.
            for artifact in artifacts {
                let source = std::fs::read_to_string(&artifact).expect("artifact reads");
                let stem = artifact.file_stem().unwrap_or_default().to_string_lossy();
                let machine = stem.strip_suffix("_sm").unwrap_or(&stem);
                for name in rendered_members(language, &source, &format!("{machine}_")) {
                    if !readers.contains(&name) && !reserved(language).covers(&name, machine) {
                        unreserved.insert(format!(
                            "{} `{name}` in {}",
                            language.canonical_name(),
                            artifact.file_name().unwrap_or_default().to_string_lossy()
                        ));
                    }
                }
            }
        }
    }
    assert!(
        unreserved.is_empty(),
        "generated output defines members `reader_names::reserved` does not know of, so a \
         reader could be given one of these names:\n  {}",
        unreserved.into_iter().collect::<Vec<_>>().join("\n  ")
    );
}

#[test]
fn names_that_broke_generation_now_generate_and_say_why_they_have_no_reader() {
    // The integration fixture every backend's driver also compiles and runs.
    let fixture =
        repo_root().join("integration_resources/typed_reader_names/typed_reader_names.scxml");
    let expected_refusals: BTreeSet<(&str, &str)> = [
        ("auto", "keyword"),
        ("self", "keyword"),
        ("new", "member-collision"),
        ("start", "member-collision"),
        ("t", "member-collision"),
        ("a-b", "duplicate-spelling"),
    ]
    .into_iter()
    .collect();
    for &language in Language::ALL {
        let (manifest, _) = generate(
            &fixture,
            language,
            &[],
            &format!("names-{}", language.canonical_name()),
        );
        let refused: BTreeSet<(String, String)> = manifest["unreadable_variables"]
            .as_array()
            .unwrap_or_else(|| panic!("no unreadable_variables: {manifest}"))
            .iter()
            .map(|u| {
                (
                    u["var"].as_str().unwrap_or("").to_string(),
                    u["reason"].as_str().unwrap_or("").to_string(),
                )
            })
            .collect();
        let expected: BTreeSet<(String, String)> = expected_refusals
            .iter()
            .map(|(v, r)| (v.to_string(), r.to_string()))
            .collect();
        assert_eq!(
            refused,
            expected,
            "{}: {manifest}",
            language.canonical_name()
        );
    }

    // The escaped keywords keep their readers, spelled each language's way.
    let model = model(&fixture);
    let reader = |id: &str| {
        model
            .readable_variables
            .iter()
            .find(|v| v.id == id)
            .and_then(|v| v.reader.clone())
            .unwrap_or_else(|| panic!("`{id}` has a reader"))
    };
    assert_eq!(reader("box").rust, "r#box");
    assert_eq!(reader("object").kotlin, "`object`");
    assert_eq!(reader("pass").python, "pass_");
    assert_eq!(reader("screen-rules").cpp, "screen_rules");
    assert_eq!(
        Language::from_str("c11")
            .map(|l| reader("a_b").get(l).to_string())
            .ok()
            .as_deref(),
        Some("a_b")
    );
}
