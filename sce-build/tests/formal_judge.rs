// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `sce-codegen formal` as a consumer runs it (docs/adr/0146, D6 and D7): on a
// manifest outside this repository's docs/formal, leaving through the exit
// statuses of SCE_ERROR_CONTRACT.md §6 with a `formal/*` record. These need
// no Java and no TLC: the declaration is judged before any tool is admitted,
// and a judge whose pinned tool is not there says so, with no verdict. The
// fixture's runs are judged with TLC by the formal-models gate.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent dir")
        .to_path_buf()
}

fn consumer_manifest() -> PathBuf {
    repo_root().join("sce-build/tests/fixtures/formal_consumer/formal/models.json")
}

/// Run `sce-codegen --error-format=json formal ...` with `TLA2TOOLS` naming
/// `tlc`, and answer its status and its one diagnostic record.
fn formal(args: &[&str], tlc: &Path) -> (Option<i32>, String, Option<serde_json::Value>) {
    let ran = Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .arg("--error-format=json")
        .arg("formal")
        .args(args)
        .env("TLA2TOOLS", tlc)
        .output()
        .expect("sce-codegen runs");
    let stdout = String::from_utf8_lossy(&ran.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&ran.stderr);
    let record = stderr
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find(|value| value.get("code").is_some());
    (ran.status.code(), stdout, record)
}

#[test]
fn the_consumer_fixture_is_declared_whole() {
    let manifest = consumer_manifest();
    let text = std::fs::read_to_string(&manifest).expect("the fixture's manifest");
    let parsed = sce_build::formal::models::parse(&text).expect("a manifest");
    let problems =
        sce_build::formal::models::declared(&parsed, manifest.parent().expect("its directory"));
    assert_eq!(problems, Vec::<String>::new());
}

#[test]
fn a_judge_whose_tool_is_not_there_says_so_with_exit_15() {
    let missing = repo_root().join("target/no-such-tla2tools.jar");
    let manifest = consumer_manifest();
    let (status, _, record) = formal(
        &["models", "--manifest", manifest.to_str().expect("a path")],
        &missing,
    );
    assert_eq!(
        status,
        Some(15),
        "no TLC is \"could not run\", not a verdict"
    );
    let record = record.expect("a diagnostic record");
    assert_eq!(record["code"], "formal/tool-unavailable", "{record}");
    assert_eq!(record["stage"], "formal", "{record}");
}

#[test]
fn a_manifest_not_as_the_tree_is_is_refused_with_exit_1() {
    let dir = std::env::temp_dir().join(format!("sce-formal-judge-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory");
    let fixture = consumer_manifest();
    let from = fixture.parent().expect("its directory");
    for name in ["Counter.tla", "Counter.cfg", "Counter_without_bound.cfg"] {
        std::fs::copy(from.join(name), dir.join(name)).expect("copied");
    }
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&fixture).expect("read")).expect("JSON");
    manifest["tlc"] = serde_json::json!({"version": "1.7.4"});
    std::fs::write(dir.join("models.json"), manifest.to_string()).expect("written");
    let missing = repo_root().join("target/no-such-tla2tools.jar");
    let (status, stdout, record) = formal(
        &[
            "models",
            "--manifest",
            dir.join("models.json").to_str().expect("a path"),
        ],
        &missing,
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        status,
        Some(1),
        "the declaration is judged before any tool is admitted"
    );
    assert!(
        stdout.contains("FAIL the manifest names \"tlc\""),
        "{stdout}"
    );
    let record = record.expect("a diagnostic record");
    assert_eq!(record["code"], "formal/declaration", "{record}");
}

#[test]
fn the_toolchain_printed_is_the_one_compiled_in() {
    let (status, stdout, _) = formal(&["toolchain"], Path::new(""));
    assert_eq!(status, Some(0));
    let printed: serde_json::Value = serde_json::from_str(&stdout).expect("JSON");
    let pinned: serde_json::Value =
        serde_json::from_str(sce_build::formal::toolchain::TOOLCHAIN_JSON).expect("JSON");
    assert_eq!(printed, pinned);
}

/// This repository's traces manifest (docs/adr/0115, D4 and D8): every recorded
/// run read, every model and configuration there, an altered run for every kind
/// of step a run holds, and every recorded run declared. Its runs are judged
/// with TLC by `sce-codegen formal traces`.
#[test]
fn this_repositorys_traces_manifest_is_declared_whole() {
    let manifest = repo_root().join("docs/formal/traces.json");
    let text = std::fs::read_to_string(&manifest).expect("the traces manifest");
    let parsed = sce_build::formal::traces::parse(&text).expect("a manifest");
    let problems =
        sce_build::formal::traces::declared(&parsed, manifest.parent().expect("its directory"));
    assert_eq!(problems, Vec::<String>::new());
}

#[test]
fn the_traces_judge_whose_tool_is_not_there_says_so_with_exit_15() {
    let missing = repo_root().join("target/no-such-tla2tools.jar");
    let manifest = repo_root().join("docs/formal/traces.json");
    let (status, _, record) = formal(
        &["traces", "--manifest", manifest.to_str().expect("a path")],
        &missing,
    );
    assert_eq!(
        status,
        Some(15),
        "no TLC is \"could not run\", not a verdict"
    );
    let record = record.expect("a diagnostic record");
    assert_eq!(record["code"], "formal/tool-unavailable", "{record}");
}

fn properties_fixture(name: &str) -> PathBuf {
    repo_root()
        .join("sce-build/tests/fixtures/formal_properties")
        .join(name)
}

fn shown(path: &Path) -> &str {
    path.to_str().expect("a path")
}

/// The properties judge reads its documents and writes the model of what they
/// say before it admits a tool (docs/adr/0115, D15): with no TLC it says so,
/// and judges nothing.
#[test]
fn the_properties_judge_whose_tool_is_not_there_says_so_with_exit_15() {
    let missing = repo_root().join("target/no-such-tla2tools.jar");
    let properties = properties_fixture("turnstile_properties.scxml");
    let exploration = properties_fixture("turnstile_exploration.scxml");
    let (status, _, record) = formal(
        &[
            "properties",
            "--properties",
            shown(&properties),
            "--exploration",
            shown(&exploration),
        ],
        &missing,
    );
    assert_eq!(
        status,
        Some(15),
        "no TLC is \"could not run\", not a verdict"
    );
    let record = record.expect("a diagnostic record");
    assert_eq!(record["code"], "formal/tool-unavailable", "{record}");
}

/// An assumption about the machine itself would excuse the machine of anything:
/// it is refused by its name before any search (docs/adr/0115, D15).
#[test]
fn an_assumption_about_the_machine_itself_is_refused_by_name_with_exit_1() {
    let missing = repo_root().join("target/no-such-tla2tools.jar");
    let properties = properties_fixture("turnstile_assumes_itself_properties.scxml");
    let exploration = properties_fixture("turnstile_exploration.scxml");
    let (status, _, record) = formal(
        &[
            "properties",
            "--properties",
            shown(&properties),
            "--exploration",
            shown(&exploration),
        ],
        &missing,
    );
    assert_eq!(status, Some(1));
    let record = record.expect("a diagnostic record");
    assert_eq!(record["code"], "formal/property-refused", "{record}");
    let message = record["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("`SELF-1` (<sce:always>): an assumption about the machine itself"),
        "{record}"
    );
}

/// A recorded judgement is reported for the documents and the judge it was
/// made of, and is none for any other: a verdict of another digest is not read
/// (docs/adr/0115, D15). Neither needs TLC.
#[test]
fn a_recorded_judgement_is_read_only_of_what_it_judged() {
    use sce_build::formal::verdicts::{
        Digests, Judgement, PropertyVerdict, Verdict, RECORD, VERSION,
    };

    let missing = repo_root().join("target/no-such-tla2tools.jar");
    let properties = properties_fixture("turnstile_properties.scxml");
    let exploration = properties_fixture("turnstile_exploration.scxml");
    let documents =
        sce_build::formal::properties::read(&properties, &exploration).expect("the fixture reads");
    let within = Verdict::HoldsWithin {
        bounds: vec!["the most events from outside (4)".to_string()],
    };
    let judgement = |digests: Digests| Judgement {
        record: RECORD.to_string(),
        v: VERSION,
        digests,
        macrosteps: within.clone(),
        not_discharged: vec!["ENV-1".to_string()],
        verdicts: [
            ("TURN-1", "always"),
            ("TURN-2", "step"),
            ("TURN-3", "leads-to"),
            ("TURN-4", "recurs"),
            ("TURN-5", "possible"),
        ]
        .into_iter()
        .map(|(req, form)| PropertyVerdict {
            req: req.to_string(),
            form: form.to_string(),
            verdict: within.clone(),
            witness: None,
        })
        .collect(),
        coverage: None,
    };
    let dir =
        std::env::temp_dir().join(format!("sce-formal-properties-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory");
    let current = Digests::of(&documents);
    let recorded = dir.join("current.json");
    std::fs::write(&recorded, judgement(current.clone()).to_text()).expect("written");
    let other = dir.join("other.json");
    let moved = Digests {
        machine: "0".repeat(64),
        ..current
    };
    std::fs::write(&other, judgement(moved).to_text()).expect("written");
    let run = |record: &Path| {
        formal(
            &[
                "properties",
                "--properties",
                shown(&properties),
                "--exploration",
                shown(&exploration),
                "--recorded",
                shown(record),
            ],
            &missing,
        )
    };
    let (status, stdout, _) = run(&recorded);
    let (other_status, _, other_record) = run(&other);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(status, Some(0), "{stdout}");
    for line in [
        "TURN-1 (<sce:always>): holds within: the most events from outside (4) reached",
        "assumed and not discharged: ENV-1",
        "every property holds as judged",
    ] {
        assert!(
            stdout.lines().any(|l| l == line),
            "missing {line:?} in\n{stdout}"
        );
    }
    assert_eq!(other_status, Some(1));
    let other_record = other_record.expect("a diagnostic record");
    assert_eq!(other_record["code"], "formal/declaration", "{other_record}");
    assert!(
        other_record["message"]
            .as_str()
            .is_some_and(|m| m.contains("a verdict of another digest is none")),
        "{other_record}"
    );
}
