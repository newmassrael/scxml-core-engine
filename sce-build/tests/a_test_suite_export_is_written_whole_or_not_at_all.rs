// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! `sce-codegen export-ttcn3` writes a suite's types module, its manifest,
//! its behaviour module and the correspondence witness of the two modules,
//! the text the library exports, each module under the header every
//! generated source carries, or, when the suite or the document is refused,
//! no file at all (docs/SCE_TEST_PROFILE.md §9.4, §9.5).

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::profile_suite::write_suite_of_enum;

fn job_suite() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_profile/job_suite.scxml")
}

/// What `export-ttcn3` does with `suite`: `global` stands before the
/// subcommand (`--error-format`), and the files go to `out`.
fn export(global: &[&str], suite: &Path, out: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sce-codegen"))
        .args(global)
        .arg("export-ttcn3")
        .arg(suite)
        .arg("--output-dir")
        .arg(out)
        .output()
        .expect("invoke sce-codegen")
}

/// The names of the files in `dir`, sorted.
fn listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("read the output directory")
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The text of the module at `path` after the two lines of the header every
/// generated source carries, which it is held to open with.
fn below_the_header(path: &Path) -> String {
    let module = read(path);
    let mut lines = module.splitn(3, '\n');
    assert_eq!(
        lines.next(),
        Some(format!("// {}", sce_build::forge::drift::HEADER_BANNER).as_str()),
        "{}",
        path.display()
    );
    assert!(
        lines
            .next()
            .is_some_and(|line| line.starts_with("// source-hash: ")),
        "{module}"
    );
    lines.next().unwrap_or_default().to_string()
}

/// The files are the library's export, all four: each module after its two
/// header lines, and the manifest and the witness as they are, each ending
/// in a newline.
#[test]
fn a_suite_is_written_as_the_library_exports_it() {
    let out = tempfile::tempdir().expect("a scratch directory");
    let run = export(&[], &job_suite(), out.path());
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        listing(out.path()),
        [
            "job_suite.ttcn",
            "job_suite.witness.json",
            "job_suite_Types.manifest.json",
            "job_suite_Types.ttcn"
        ]
    );
    let library = sce_build::ttcn3_export::export_suite(&job_suite(), &[])
        .expect("the job suite is exported");

    assert_eq!(
        below_the_header(&out.path().join("job_suite_Types.ttcn")),
        format!("{}\n", library.types_module)
    );
    assert_eq!(
        below_the_header(&out.path().join("job_suite.ttcn")),
        format!("{}\n", library.behaviour_module)
    );
    assert_eq!(
        read(&out.path().join("job_suite_Types.manifest.json")),
        format!("{}\n", library.manifest)
    );
    assert_eq!(
        read(&out.path().join("job_suite.witness.json")),
        format!("{}\n", library.witness)
    );
}

/// A suite the export refuses is refused on the wire, and leaves no file.
#[test]
fn a_refused_suite_writes_nothing() {
    let documents = tempfile::tempdir().expect("a scratch directory");
    let suite = write_suite_of_enum(documents.path(), "inconc");
    let out = tempfile::tempdir().expect("a scratch directory");
    let run = export(&["--error-format=json"], &suite, out.path());
    assert!(!run.status.success(), "a refused suite is exported");
    let stderr = String::from_utf8_lossy(&run.stderr);
    let record: serde_json::Value = stderr
        .lines()
        .find_map(|line| serde_json::from_str(line.trim()).ok())
        .unwrap_or_else(|| panic!("a diagnostic record:\n{stderr}"));
    assert_eq!(record["code"], "test-profile/export-name", "{record}");
    assert_eq!(record["actual"], "inconc", "{record}");
    assert_eq!(listing(out.path()), Vec::<String>::new());
}

/// A document that is no test suite is refused by saying what the command
/// reads, and leaves no file.
#[test]
fn a_document_that_is_no_suite_writes_nothing() {
    let template = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/test_profile/job_report.scxml");
    let out = tempfile::tempdir().expect("a scratch directory");
    let run = export(&[], &template, out.path());
    assert!(!run.status.success(), "a template is exported as a suite");
    assert!(
        String::from_utf8_lossy(&run.stderr)
            .contains("export-ttcn3 reads a test suite (sce:kind=\"test-suite\")"),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(listing(out.path()), Vec::<String>::new());
}
