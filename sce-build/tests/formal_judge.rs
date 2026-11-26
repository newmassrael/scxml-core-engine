// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `sce-codegen formal` as a consumer runs it (docs/adr/0144, D6 and D7): on a
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
