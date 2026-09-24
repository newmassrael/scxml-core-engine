// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Generated C++ is formatted by one pinned tool, or the run stops.
//!
//! `sce-codegen` formats the C++ it writes with clang-format of
//! `formatter::CLANG_FORMAT_MAJOR`, and until 2026-09-24 it did so with
//! whichever clang-format a host happened to have — or, on a host with none,
//! wrote unformatted C++ behind a note on stderr. Measured on the 288 raw C++
//! forge goldens: clang-format 18 and 19 disagree on 17 files. So one document
//! produced three different sets of bytes across the three build machines
//! this repository uses, and nothing said so (docs/SCE_CODEGEN_DETERMINISM.md
//! §9).
//!
//! Each case here puts a stand-in clang-format in front of the binary through
//! `SCE_TOOL_CLANG_FORMAT` — the override is the one answer the resolver
//! takes, so the cases judge the binary's contract and not whatever the host
//! has installed. A stand-in either copies its input (formats nothing),
//! appends a marker line (so a file that passed through it is visibly
//! changed, while the traceability markers the run validates afterwards stay
//! intact), or refuses.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod common;

fn codegen() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

const DOOR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="closed" name="Door">
  <state id="closed"><transition event="open" target="opened"/></state>
  <state id="opened"><transition event="close" target="closed"/></state>
</scxml>
"#;

/// What a stand-in does with the code it is asked to format.
enum Body {
    /// Copies stdin to stdout: formats nothing.
    Copy,
    /// Appends [`MARK`], so a file that passed through it is visibly changed.
    Mark,
    /// Refuses the input with a message, as clang-format does on a hard error.
    Refuse,
}

/// The line [`Body::Mark`] appends to everything it formats.
const MARK: &str = "// passed through the stand-in formatter";

/// A stand-in clang-format that reports `version` and then does `body`.
fn stand_in(dir: &Path, version: &str, body: Body) -> PathBuf {
    let path = dir.join(format!("clang-format-stand-in-{version}"));
    let act = match body {
        Body::Copy => "cat".to_string(),
        Body::Mark => format!("cat; echo '{MARK}'"),
        Body::Refuse => "echo 'unterminated comment' >&2; exit 1".to_string(),
    };
    common::executable::install_executable(
        &path,
        format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'Ubuntu clang-format version {version} (test)'; exit 0; fi\n{act}\n"
        ),
    )
    .expect("install the stand-in");
    path
}

fn workspace() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("temp dir");
    fs::write(tmp.path().join("door.scxml"), DOOR).expect("write the document");
    tmp
}

/// `sce-codegen --error-format json <args>` with `clang_format` as the only
/// clang-format the resolver may take.
fn run(clang_format: &Path, args: &[&str]) -> Output {
    Command::new(codegen())
        .args(["--error-format", "json"])
        .args(args)
        .env("SCE_TOOL_CLANG_FORMAT", clang_format)
        .output()
        .expect("run sce-codegen")
}

fn generate(dir: &Path, clang_format: &Path, language: &str, extra: &[&str]) -> Output {
    let doc = dir.join("door.scxml");
    let out = dir.join("out");
    let mut args = vec![
        "generate",
        doc.to_str().expect("utf-8 path"),
        "-o",
        out.to_str().expect("utf-8 path"),
        "-l",
        language,
    ];
    args.extend_from_slice(extra);
    run(clang_format, &args)
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The first NDJSON diagnostic record on stderr.
fn record(out: &Output) -> serde_json::Value {
    let text = stderr(out);
    let line = text
        .lines()
        .find(|l| l.starts_with('{'))
        .unwrap_or_else(|| panic!("no NDJSON record on stderr:\n{text}"));
    serde_json::from_str(line).unwrap_or_else(|e| panic!("record does not parse ({e}): {line}"))
}

/// The run manifest, the one JSON line on stdout.
fn manifest(out: &Output) -> serde_json::Value {
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text
        .lines()
        .find(|l| l.starts_with('{'))
        .unwrap_or_else(|| panic!("no manifest on stdout:\n{text}\nstderr:\n{}", stderr(out)));
    serde_json::from_str(line).unwrap_or_else(|e| panic!("manifest does not parse ({e}): {line}"))
}

fn header(dir: &Path) -> String {
    fs::read_to_string(dir.join("out").join("door_sm.h")).expect("read the generated header")
}

/// The host shape this exists for: a clang-format is present, and it is the
/// wrong major. The run used to format with it; now it stops and names it.
#[test]
fn a_host_with_only_another_major_is_refused_by_name() {
    let tmp = workspace();
    let eighteen = stand_in(tmp.path(), "18.1.3", Body::Copy);

    let out = generate(tmp.path(), &eighteen, "cpp", &[]);
    assert!(
        !out.status.success(),
        "a clang-format 18 must not format generated C++"
    );
    let rec = record(&out);
    assert_eq!(rec["code"], "cli/formatter-unavailable", "record: {rec}");
    let message = rec["message"].as_str().unwrap_or_default();
    assert!(
        message.contains("18.1.3"),
        "the refusal names what it found: {message}"
    );
    assert!(
        message.contains("--no-format"),
        "the refusal names the way out: {message}"
    );
    assert!(
        !tmp.path().join("out").join("door_sm.h").exists(),
        "a refused run writes no C++"
    );
}

#[test]
fn the_pinned_major_formats_and_the_manifest_names_it() {
    let tmp = workspace();
    let nineteen = stand_in(tmp.path(), "19.1.1", Body::Mark);

    let out = generate(tmp.path(), &nineteen, "cpp", &[]);
    assert!(
        out.status.success(),
        "clang-format 19 must serve: {}",
        stderr(&out)
    );
    assert_eq!(
        manifest(&out)["formatter"],
        serde_json::json!({"tool": "clang-format", "version": "19.1.1"}),
    );
    // What reached the disk is what the formatter returned.
    let h = header(tmp.path());
    assert!(
        h.contains(MARK),
        "the header is the formatter's output:\n{h}"
    );
}

/// `--no-format` is the opt-out: the formatter is neither asked for nor run,
/// even when the only one on offer would be refused.
#[test]
fn no_format_writes_the_templates_own_bytes_and_names_no_formatter() {
    let tmp = workspace();
    let eighteen = stand_in(tmp.path(), "18.1.3", Body::Mark);

    let out = generate(tmp.path(), &eighteen, "cpp", &["--no-format"]);
    assert!(
        out.status.success(),
        "--no-format needs no formatter: {}",
        stderr(&out)
    );
    assert!(
        manifest(&out).get("formatter").is_none(),
        "a run that formatted nothing names no formatter"
    );
    assert!(
        !header(tmp.path()).contains(MARK),
        "--no-format must not pass the output through any clang-format"
    );
}

/// The formatter is resolved at the first C++ there is to write, so a
/// document that fails before then is refused with its own diagnostic.
#[test]
fn a_document_that_fails_first_reports_its_own_diagnostic() {
    let tmp = workspace();
    fs::write(tmp.path().join("door.scxml"), "<scxml version=\"1.0\"")
        .expect("write a broken document");
    let eighteen = stand_in(tmp.path(), "18.1.3", Body::Copy);

    let out = generate(tmp.path(), &eighteen, "cpp", &[]);
    assert!(!out.status.success(), "a broken document is refused");
    let rec = record(&out);
    assert_ne!(
        rec["code"], "cli/formatter-unavailable",
        "the document's own diagnostic must not be masked by the formatter's: {rec}"
    );
}

#[test]
fn a_run_that_writes_no_cpp_needs_no_formatter() {
    let tmp = workspace();
    let eighteen = stand_in(tmp.path(), "18.1.3", Body::Mark);

    let out = generate(tmp.path(), &eighteen, "rust", &[]);
    assert!(
        out.status.success(),
        "Rust output needs no clang-format: {}",
        stderr(&out)
    );
    assert!(manifest(&out).get("formatter").is_none());
}

/// A document set is not shaped differently from the same documents
/// generated one at a time: `orchestrate` formats, and says so.
#[test]
fn orchestrate_formats_as_generate_does() {
    let tmp = workspace();
    let nineteen = stand_in(tmp.path(), "19.1.1", Body::Mark);
    let doc = tmp.path().join("door.scxml");
    let out_dir = tmp.path().join("out");

    let out = run(
        &nineteen,
        &[
            "orchestrate",
            "--scxml",
            doc.to_str().expect("utf-8 path"),
            "-l",
            "cpp",
            "-o",
            out_dir.to_str().expect("utf-8 path"),
        ],
    );
    assert!(out.status.success(), "orchestrate failed: {}", stderr(&out));
    assert_eq!(
        manifest(&out)["formatter"],
        serde_json::json!({"tool": "clang-format", "version": "19.1.1"}),
    );
    assert!(
        header(tmp.path()).contains(MARK),
        "orchestrate's C++ passed through the formatter"
    );
}

/// A file clang-format refuses stops the run; it is not written unformatted
/// behind a warning, which is what happened until 2026-09-24.
#[test]
fn a_file_clang_format_refuses_stops_the_run() {
    let tmp = workspace();
    let refusing = stand_in(tmp.path(), "19.1.1", Body::Refuse);

    let out = generate(tmp.path(), &refusing, "cpp", &[]);
    assert!(!out.status.success(), "a refused file must fail the run");
    let rec = record(&out);
    assert_eq!(rec["code"], "cli/format-failed", "record: {rec}");
    assert!(
        rec["message"]
            .as_str()
            .unwrap_or_default()
            .contains("unterminated comment"),
        "the refusal carries clang-format's own words: {rec}"
    );
    assert!(
        !tmp.path().join("out").join("door_sm.h").exists(),
        "the refused file is not written unformatted"
    );
}
