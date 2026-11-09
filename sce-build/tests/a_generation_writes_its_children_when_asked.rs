// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `generate --children`: one run writes a document's machine and the machine
// of every child it writes a document for, which a build otherwise generates
// by runs of their own (`--plan --plan-children` names them). It is what makes
// a document and its children one action of the cache (ADR 0097).
//
// Held here, by the real binary:
//   - each child the run starts gets its machine, in the run's directory;
//   - a child the checkout holds (a candidate, a `src` child) is generated from
//     where its author wrote it, so its files name that document and not the
//     copy the run staged beside its parent;
//   - a child only the run wrote (a hybrid invoke's stub) is named by its path
//     in the output directory, never by where that directory is;
//   - a plan and a depfile, which describe one generation each, refuse it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn codegen() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// `sce-codegen <arguments>` from the checkout's root, where every path a
/// generated file names is relative to.
fn run(arguments: &[&str]) -> Output {
    Command::new(codegen())
        .args(arguments)
        .current_dir(repo_root())
        .output()
        .expect("run sce-codegen")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn from_line(file: &Path) -> String {
    let text =
        std::fs::read_to_string(file).unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
    text.lines()
        .find(|line| line.starts_with("// From: "))
        .unwrap_or_else(|| panic!("{} names no document", file.display()))
        .to_string()
}

const CANDIDATES: &str = "integration_resources/a_hybrid_invoke_carries_its_arguments/\
                          a_hybrid_invoke_carries_its_arguments.scxml";
const STUB: &str = "integration_resources/invoke_expression_failure_is_reported/\
                    invoke_expression_failure_is_reported.scxml";

#[test]
fn a_child_the_checkout_holds_names_the_document_its_author_wrote() {
    let out = tempfile::tempdir().expect("an output directory");
    let out_text = out.path().display().to_string();
    let ran = run(&[
        "generate",
        CANDIDATES,
        "-l",
        "rust",
        "-o",
        &out_text,
        "--children",
    ]);
    assert!(ran.status.success(), "{}", stderr(&ran));

    for child in ["keeper", "bare"] {
        let machine = out.path().join(format!("{child}_sm.rs"));
        assert!(
            machine.is_file(),
            "the run wrote no machine for the candidate {child}"
        );
        assert_eq!(
            from_line(&machine),
            format!(
                "// From: integration_resources/a_hybrid_invoke_carries_its_arguments/{child}.scxml"
            ),
            "a candidate's machine names the copy the run staged, not its document"
        );
    }
}

#[test]
fn a_child_only_the_run_wrote_is_named_by_its_path_in_the_output_directory() {
    let out = tempfile::tempdir().expect("an output directory");
    let out_text = out.path().display().to_string();
    let ran = run(&[
        "generate",
        STUB,
        "-l",
        "rust",
        "-o",
        &out_text,
        "--children",
    ]);
    assert!(ran.status.success(), "{}", stderr(&ran));

    let machine = out
        .path()
        .join("invoke_expression_failure_is_reported_hybrid0_sm.rs");
    assert!(machine.is_file(), "the run wrote no machine for its stub");
    assert_eq!(
        from_line(&machine),
        "// From: invoke_expression_failure_is_reported_hybrid0.scxml"
    );
    let text = std::fs::read_to_string(&machine).expect("read the stub's machine");
    assert!(
        !text.contains(&out_text),
        "the stub's machine names the directory it was written to"
    );
}

#[test]
fn a_plan_and_a_depfile_refuse_it() {
    let out = tempfile::tempdir().expect("an output directory");
    let out_text = out.path().display().to_string();
    let depfile = out.path().join("run.d").display().to_string();

    let planned = run(&[
        "--plan",
        "generate",
        STUB,
        "-l",
        "rust",
        "-o",
        &out_text,
        "--children",
    ]);
    assert!(!planned.status.success(), "a plan accepted --children");
    assert!(
        stderr(&planned).contains("--plan-children"),
        "the refusal does not name the plan's own flag: {}",
        stderr(&planned)
    );

    let with_depfile = run(&[
        "generate",
        STUB,
        "-l",
        "rust",
        "-o",
        &out_text,
        "--children",
        "--write-deps",
        &depfile,
    ]);
    assert!(
        !with_depfile.status.success(),
        "a depfile of one build step was accepted for several generations"
    );
    assert!(
        stderr(&with_depfile).contains("--reads"),
        "the refusal does not name the list that does cover the run: {}",
        stderr(&with_depfile)
    );
}
