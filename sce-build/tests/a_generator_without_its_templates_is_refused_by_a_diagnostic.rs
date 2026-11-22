// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A binary installed on a machine that does not carry the source tree it was
// built from has no templates until SCE_TEMPLATE_DIR names them. It used to
// panic (exit 101, no diagnostic) on every command that renders, `check`
// included, so a consumer reading NDJSON got a Rust backtrace. It is refused as
// `generate/template-load`, naming what was looked for, with stdout empty and
// the exit code of the stage (SCE_ERROR_CONTRACT.md sections 1, 2 and 6).
//
// The variable naming a directory that is not there stands for the missing
// tree: the source tree is on the machine this test runs on, and a named
// directory is the one place looked.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const MISSING: &str = "/nonexistent/sce-templates";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(args)
        .env("SCE_TEMPLATE_DIR", MISSING)
        .output()
        .expect("sce-codegen runs")
}

fn scxml() -> String {
    let dir = std::env::temp_dir().join(format!("sce-no-templates-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let path = dir.join("plain.scxml");
    std::fs::write(
        &path,
        r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a" name="plain" datamodel="null"><state id="a"/></scxml>"#,
    )
    .expect("the document");
    path.display().to_string()
}

fn refused(args: &[&str]) {
    let output = run(args);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output.status.code();
    assert!(
        code.is_some_and(|code| code != 0 && code != 101),
        "{args:?} must be refused and not panic: {code:?}\n{stderr}"
    );
    assert!(output.stdout.is_empty(), "{args:?} wrote stdout on failure");
    let record = stderr
        .lines()
        .find_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .unwrap_or_else(|| panic!("{args:?} wrote no record: {stderr}"));
    assert_eq!(
        record["code"], "generate/template-load",
        "{args:?}: {stderr}"
    );
    let message = record["message"].as_str().expect("a message");
    assert!(
        message.contains(MISSING) && message.contains("SCE_TEMPLATE_DIR"),
        "{args:?} must name what was looked for: {message}"
    );
}

#[test]
fn check_is_refused_by_a_diagnostic() {
    refused(&["--error-format=json", "check", &scxml()]);
}

#[test]
fn generate_is_refused_by_a_diagnostic_on_both_pipelines() {
    let out = std::env::temp_dir().display().to_string();
    let forge = fixture("static_datamodel/algorithm_take_generation.scxml");
    for document in [scxml(), forge.display().to_string()] {
        refused(&[
            "--error-format=json",
            "generate",
            &document,
            "-o",
            &out,
            "-l",
            "rust",
        ]);
    }
}

#[test]
fn a_human_run_names_the_directory_too() {
    let output = run(&["check", &scxml()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_ne!(output.status.code(), Some(101), "{stderr}");
    assert!(stderr.contains(MISSING), "{stderr}");
}
