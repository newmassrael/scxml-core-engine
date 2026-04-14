// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// End-to-end contract test for `sce-codegen --error-format=json`.
//
// These tests launch the real CLI binary (the one cargo builds for
// integration tests and exposes via `CARGO_BIN_EXE_*`) on a forge
// document crafted to fail validation. The assertions pin the wire
// contract consumed by upstream agents:
//
//   * stderr is NDJSON (one JSON object per line)
//   * each line carries a stable `code`, `stage`, and `id`
//   * the process exit code matches `ForgeError::exit_code()`
//   * stdout is not polluted by the diagnostic
//
// Human mode is covered by a negative assertion: its stderr must NOT
// start with `{`, so anything grepping for JSON in human output breaks
// loudly instead of silently mis-parsing.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// Resolve the integration binary cargo built for this crate.
///
/// `CARGO_BIN_EXE_<bin>` is populated automatically for integration
/// tests so we can launch the CLI without hardcoding a path. This
/// requires the `cli` feature to be enabled when tests run — enforced
/// by the `required-features = ["cli"]` manifest entry on this test.
fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

/// Per-process counter so parallel tests never share a scratch dir.
static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

/// Scoped scratch directory under `target/`. Dropping removes the
/// directory and any fixtures it held.
struct ScratchDir(PathBuf);
impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let dir = root.join(format!("{label}-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        ScratchDir(dir)
    }
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}
impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Write a lookup-kind forge document with no `<datamodel>` child.
/// This reliably enters the forge pipeline (`sce:kind="lookup"` is an
/// XSD-accepted value) and fails semantic validation with
/// `ValidationError::MissingElement`, whose mapping (`code =
/// "validation/missing-element"`, `stage = "validation"`) is the
/// invariant these tests pin.
fn write_missing_datamodel_fixture() -> (ScratchDir, PathBuf) {
    let dir = ScratchDir::new("error-format");
    let path = dir.path().join("bad.scxml");
    let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="lookup" name="bad_lookup">
</scxml>
"#;
    std::fs::write(&path, body).expect("write fixture");
    (dir, path)
}

fn run_generate(bin: &PathBuf, scxml: &PathBuf, error_format: &str) -> std::process::Output {
    Command::new(bin)
        .args([
            "--error-format",
            error_format,
            "generate",
            scxml.to_str().unwrap(),
            "--language",
            "rust",
            "--output-dir",
            scxml.parent().unwrap().to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen")
}

#[test]
fn json_mode_emits_single_ndjson_record_on_stderr() {
    let (_dir, scxml) = write_missing_datamodel_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");

    assert!(!out.status.success(), "process must fail on validation error");
    // ValidationError maps to stage=3 per ForgeError::exit_code().
    assert_eq!(out.status.code(), Some(3), "exit code must equal ForgeError::exit_code()");

    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let trimmed = stderr.trim_end();
    // NDJSON: exactly one line here (single error), each line a JSON object.
    assert!(
        !trimmed.is_empty(),
        "json mode must emit at least one diagnostic line"
    );
    for line in trimmed.lines() {
        assert!(line.starts_with('{'), "line must start with '{{': {line}");
        assert!(line.ends_with('}'), "line must end with '}}': {line}");
        let parsed: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line is not valid JSON ({e}): {line}"));
        let obj = parsed.as_object().expect("root must be an object");
        // Required fields per the diagnostic contract.
        for key in ["v", "id", "code", "stage", "message"] {
            assert!(obj.contains_key(key), "missing required field '{key}' in: {line}");
        }
        assert_eq!(
            obj["v"].as_u64(),
            Some(1),
            "schema version pinned at 1 for this release: {line}"
        );
        assert_eq!(
            obj["code"], "validation/missing-element",
            "unexpected code: {line}"
        );
        assert_eq!(obj["stage"], "validation", "unexpected stage: {line}");
        assert!(
            obj["id"].as_str().unwrap_or("").starts_with("fnv1a:"),
            "id must be content-hashed: {line}"
        );
    }
}

#[test]
fn json_mode_does_not_pollute_stdout_with_diagnostics() {
    let (_dir, scxml) = write_missing_datamodel_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "json");
    let stdout = String::from_utf8(out.stdout).expect("stdout utf8");
    // stdout is for artifact manifests / progress text; error payloads
    // must ride on stderr so agents can split streams by fd without
    // parsing.
    assert!(
        !stdout.contains("validation/missing-element"),
        "diagnostic code leaked to stdout: {stdout}"
    );
    assert!(
        !stdout.contains("\"code\""),
        "JSON diagnostic shape leaked to stdout: {stdout}"
    );
}

/// CLI-boundary failure (unknown --language) must ride the same wire
/// contract as forge / mesh errors. Pins the "flag is universal"
/// guarantee — if a new subcommand path forgets to route through
/// `cli_exit`, this test fails.
#[test]
fn json_mode_covers_cli_boundary_errors() {
    // Any path works — the language check fires before I/O. Using a
    // bogus path explicitly avoids accidentally invoking the forge
    // pipeline on a real fixture.
    let out = Command::new(sce_codegen_bin())
        .args([
            "--error-format",
            "json",
            "generate",
            "/does-not-exist.scxml",
            "--language",
            "wasm", // deliberately unsupported
            "--output-dir",
            "/tmp",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn sce-codegen");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    let line = stderr.trim_end();
    assert!(line.starts_with('{') && line.ends_with('}'), "not NDJSON: {line}");
    let parsed: serde_json::Value =
        serde_json::from_str(line).expect("CLI error line must be JSON");
    assert_eq!(parsed["code"], "cli/unknown-language");
    assert_eq!(parsed["stage"], "cli");
    assert_eq!(parsed["v"].as_u64(), Some(1));
    // `expected` lists the legal languages so an agent can fix
    // without parsing the message.
    assert!(
        parsed["expected"]
            .as_array()
            .map(|a| a.iter().any(|v| v == "rust"))
            .unwrap_or(false),
        "expected field missing or malformed: {line}"
    );
}

#[test]
fn human_mode_remains_plain_text() {
    let (_dir, scxml) = write_missing_datamodel_fixture();
    let out = run_generate(&sce_codegen_bin(), &scxml, "human");
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(3));
    let stderr = String::from_utf8(out.stderr).expect("stderr utf8");
    // Human mode preserves the pre-existing banner exactly — any
    // change here is a user-visible regression.
    assert!(
        stderr.contains("Forge codegen error:"),
        "human banner missing: {stderr}"
    );
    // And it MUST NOT be JSON — anything grepping for `{` in human
    // output would otherwise silently mis-parse.
    assert!(
        !stderr.trim_start().starts_with('{'),
        "human mode must not emit JSON: {stderr}"
    );
}
