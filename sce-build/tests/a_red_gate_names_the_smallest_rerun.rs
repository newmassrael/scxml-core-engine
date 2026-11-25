// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A red gate ends with the smallest command that reproduces each thing it
// found, under one heading, and a hosted lane's step summary carries the same.
//
// Until 2026-11-25 a red gate said "run just this one with: scripts/gate
// <slug>", which builds and runs the whole gate or shard again: job
// 138951650414 failed one test of the workspace suite and was told to run 1363
// seconds of it. `scripts/lib/repro.py` reads what failed from each runner's
// own report, and `sce_gate_tested` in scripts/gates/lib.sh hands it the report
// when a command a gate runs fails.
//
// What is held here:
//   * the reader's own cases, on output recorded from real red jobs
//     (`sce-build/tests/fixtures/gate_repro/`), one runner each;
//   * a gate that fails ends with the heading and the one test, through the
//     library a gate sources, with a stand-in cargo printing a recorded log;
//   * the runner prints the heading on a red gate and writes the step summary
//     where a hosted lane names one;
//   * the mutation harness runs the one case it is asked for.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

const READER: &str = "scripts/lib/repro.py";
const FIXTURES: &str = "sce-build/tests/fixtures/gate_repro";
const HEADING_OF_PROBE: &str = "==> reproduce gate[probe]";

/// The reader's cases: cargo's libtest list and its selector, a build that did
/// not compile, ctest's record of what failed and its closure, ninja's failed
/// edge, go's events, pytest's and Gradle's and swift's reports, the W3C
/// runner's report, the mutation harness's verdicts and a regeneration's
/// changed files.
#[test]
fn the_rerun_reader_passes_its_own_cases() {
    let out = Command::new("python3")
        .arg(READER)
        .arg("--self-test")
        .arg(FIXTURES)
        .current_dir(repo_root())
        .output()
        .expect("python3 runs the reader");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{READER} --self-test failed:\n{stdout}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("checks passed"),
        "{READER} --self-test said nothing about its checks, so it may have run none:\n{stdout}"
    );
}

/// A gate that fails ends with the heading and the command that runs the one
/// test that failed, and the command keeps the run's features and drops the
/// selection that ran the rest.
///
/// The library a gate sources is sourced as a gate does, with no runner around
/// it, so it keeps the commands itself and prints them as the script ends red.
/// A stand-in cargo prints the log of job 138951650414 (one shard of the
/// workspace suite, one failing test) and fails as cargo did, and answers no
/// metadata, so the run's features are kept as given.
#[test]
fn a_failing_gate_ends_with_the_heading_and_the_one_test() {
    let root = repo_root();
    let dir = tempfile::tempdir().expect("tempdir");
    let stub = dir.path().join("cargo");
    let fixture = root.join(FIXTURES).join("cargo_test_workspace_shard.txt");
    let body = format!(
        "#!/usr/bin/env bash\n\
         if [ \"$1\" = metadata ]; then exit 1; fi\n\
         sed -E 's/^[0-9]{{4}}-[0-9]{{2}}-[0-9]{{2}}T[0-9:.]+Z ?//' '{}'\n\
         exit 101\n",
        fixture.display()
    );
    common::executable::install_executable(&stub, body).expect("install stub cargo");

    // `lib.sh` turns on `set -e` and names the slug after the script that
    // sourced it, so the slug is set after the source; the gate's verdict is
    // the `sce_gate_fail` a gate writes after the command.
    let script = format!(
        "source '{}/scripts/gates/lib.sh'\n\
         SCE_GATE_SLUG=probe\n\
         PATH='{}':$PATH\n\
         sce_gate_tested -- cargo test --workspace --features cli --no-fail-fast \\\n\
             || sce_gate_fail \"the workspace suite\"\n",
        root.display(),
        dir.path().display()
    );
    let out = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .current_dir(&root)
        .env_remove("SCE_GATE_REPRO_FILE")
        .env_remove("GITHUB_STEP_SUMMARY")
        .output()
        .expect("run a failing gate under a stub cargo");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "the gate passed over a cargo that failed:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("every_ledger_binding_names_what_its_file_holds ... FAILED"),
        "the suite's own output no longer reaches the gate's log as it runs:\n{stdout}"
    );
    let heading = stderr
        .find(HEADING_OF_PROBE)
        .unwrap_or_else(|| panic!("a red gate did not end with the heading:\n{stderr}"));
    let after = &stderr[heading..];
    let command = "cargo test --features cli -p sce-build --test \
                   every_ledger_binding_names_what_its_file_holds -- \
                   every_ledger_binding_names_what_its_file_holds --exact";
    assert!(
        after.contains(command),
        "the heading does not name the one test that failed with the run's features:\n{after}"
    );
    assert!(
        !after.contains("--workspace"),
        "the command still runs the whole workspace suite:\n{after}"
    );
}

/// The runner prints the heading for a red gate, hands it the whole gate (the
/// shard a lane ran by its variable included), writes the step summary when a
/// hosted lane names one, and prints nothing for a gate that could not run.
#[test]
fn the_runner_ends_a_red_gate_with_the_heading_and_the_summary() {
    let root = repo_root();
    let runner = std::fs::read_to_string(root.join("scripts/gate")).expect("read scripts/gate");
    let code: String = runner
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code.contains("if (( rc != 3 )); then\n            gate_repro_report \"$slug\""),
        "scripts/gate no longer reports the commands that reproduce a red gate, \
         or reports them for a gate that could not run"
    );
    assert!(
        code.contains("export SCE_GATE_REPRO_FILE=\"$GATE_REPRO_DIR/$slug.jsonl\""),
        "scripts/gate no longer hands each gate a file of its own for its commands"
    );
    assert!(
        code.contains("args+=(--summary \"$GITHUB_STEP_SUMMARY\")"),
        "scripts/gate no longer adds the commands to a hosted lane's step summary"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let recorded = dir.path().join("probe.jsonl");
    std::fs::write(
        &recorded,
        "{\"builds\": \"b\", \"command\": \"cargo test -p x --test y -- z --exact\", \"what\": \"z\"}\n",
    )
    .expect("write a recorded command");
    let summary = dir.path().join("summary.md");
    let out = Command::new("python3")
        .arg(READER)
        .args(["report", "--slug", "probe", "--file"])
        .arg(recorded.as_os_str())
        .args(["--fallback", "scripts/gate --shard 2/4 probe", "--summary"])
        .arg(&summary)
        .current_dir(&root)
        .output()
        .expect("python3 runs the reader");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "the report failed:\n{stderr}");
    assert!(
        stderr.contains(HEADING_OF_PROBE)
            && stderr.contains("cargo test -p x --test y -- z --exact")
            && stderr
                .trim_end()
                .ends_with("scripts/gate --shard 2/4 probe"),
        "the report is not the heading, the command and the whole gate last:\n{stderr}"
    );
    let written = std::fs::read_to_string(summary.as_path()).expect("the step summary was written");
    assert!(
        written.contains("cargo test -p x --test y -- z --exact")
            && written.contains("scripts/gate --shard 2/4 probe"),
        "the step summary does not carry the command and the whole gate:\n{written}"
    );
}

/// `scripts/mutate --case` runs one case, named by its label or its place, and
/// refuses a case the casefile does not hold and a case inside a shard.
/// `--check` builds nothing and edits copies, so this asks no build.
#[test]
fn the_mutation_harness_runs_the_one_case_it_is_asked_for() {
    let root = repo_root();
    let casefile = "sce-build/tests/mutations/an_unknown_gate_slug_is_not_a_pass.cases";
    let second = "the resolver refuses without saying which slug it did not know";
    let first = "the resolver's refusal is swallowed and the empty order exits 0";
    let run = |args: &[&str]| {
        Command::new(root.join("scripts/mutate"))
            .args(args)
            .current_dir(&root)
            .output()
            .expect("scripts/mutate runs")
    };
    for selector in ["2", second] {
        let out = run(&["--check", "--case", selector, casefile]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "--case {selector} did not check its case:\n{stdout}\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let applied: Vec<&str> = stdout
            .lines()
            .filter(|l| l.starts_with("applies"))
            .collect();
        assert_eq!(
            applied.len(),
            1,
            "--case {selector} checked {} case(s), not the one it named:\n{stdout}",
            applied.len()
        );
        assert!(
            applied[0].contains(second) && !stdout.contains(first),
            "--case {selector} checked another case than the second:\n{stdout}"
        );
    }
    for args in [
        ["--check", "--case", "3", casefile],
        ["--check", "--case", "a label no case carries", casefile],
    ] {
        let out = run(&args);
        assert_eq!(
            out.status.code(),
            Some(2),
            "scripts/mutate {args:?} was not refused as a caller's error:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let out = run(&["--check", "--case", "1", "--shard", "1/2", casefile]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "--case inside a shard was not refused:\n{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
