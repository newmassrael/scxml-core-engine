// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// docs/SCE_TTCN3_LANE.md §12, §13.1 S2: every trace of the corpus replays, through
// its suite's runner program, to its expected log, byte for byte. Each suite's
// runner is the `main` of its generated unit (§4), made a program by a target of
// one line under `src/bin/`; the suites are the ones sce-codegen writes into
// `src/integration/static_datamodel/` (regen:
// `scripts/regen_static_datamodel_rust.sh`). runtime_suite's own traces are
// replayed by `a_test_suite_runs_through_its_runner.rs`, beside the rest of what
// that suite is held to.
//
// The expected logs are the canonical logs a reviewed run of the Rust runner wrote
// for each trace, written by `scripts/regen_corpus_expected_logs.sh` and committed;
// the other backends are held to the same files.

use std::path::{Path, PathBuf};

/// A suite of the corpus and how its runner is run over its traces.
struct Replayed {
    /// The suite's unit name, which names the directory of its runs.
    suite: &'static str,
    /// Its runner program.
    runner: &'static str,
    /// The world that selects every test case, as `--world` takes it.
    world: &'static str,
    /// What the runner prints on its standard output: each test case its world
    /// selects, in the suite's order, with its verdict.
    verdicts: &'static str,
}

/// docs/SCE_TTCN3_LANE.md §12.2: signed and 64-bit integers in a message, the
/// verdicts `fail` and `none`, and the two ways to `error` a document leads a
/// run to, a test case that can never end and a dynamic error.
const BOUNDARY: Replayed = Replayed {
    suite: "boundary_suite",
    runner: env!("CARGO_BIN_EXE_boundary_suite_runner"),
    world: "",
    verdicts: "exchanges_signed_numbers pass\nanswers_another_job fail\nsets_no_verdict none\n\
               never_ends error\nsends_to_a_removed_ptc error\n",
};

/// The directory of the runs of `suite`: its `traces/` and `expected_logs/`.
fn runs_dir(suite: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../sce-build/tests/fixtures/test_profile/runs")
        .join(suite)
}

/// The name of every file in `dir`, sorted.
fn file_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("the directory {} is read: {e}", dir.display()))
        .map(|entry| {
            entry
                .expect("an entry of the directory is read")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// A directory of the runner's logs for `suite`, made empty.
fn log_dir(suite: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("sce-corpus-runner-{}-{suite}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("an earlier run's logs are removed");
    }
    dir
}

/// Runs the suite's runner over every trace of its directory, in the world that
/// selects every test case, and holds what it prints to the verdicts it expects,
/// every log it writes to its expected log, and the set of logs to the set of
/// expected logs.
fn replays(replayed: &Replayed) {
    let runs = runs_dir(replayed.suite);
    let logs = log_dir(replayed.suite);
    let output = std::process::Command::new(replayed.runner)
        .arg("--trace-dir")
        .arg(runs.join("traces"))
        .arg("--log-dir")
        .arg(&logs)
        .arg("--world")
        .arg(replayed.world)
        .output()
        .expect("the runner program runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        replayed.verdicts,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let expected_dir = runs.join("expected_logs");
    let expected: Vec<String> = file_names(&expected_dir)
        .iter()
        .map(|name| {
            name.strip_suffix(".json")
                .unwrap_or_else(|| panic!("the expected log {name} is a .json file"))
                .to_string()
        })
        .collect();
    let written = file_names(&logs);
    for name in &written {
        let case = name
            .strip_suffix(".log.json")
            .unwrap_or_else(|| panic!("the runner wrote {name}, which is no log of a case"));
        let path = expected_dir.join(format!("{case}.json"));
        let want = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "the case {case} has no expected log {}: {e}",
                path.display()
            )
        });
        let log = std::fs::read_to_string(logs.join(name)).expect("the written log is read");
        assert_eq!(
            log,
            want.strip_suffix('\n')
                .unwrap_or_else(|| panic!("the expected log of {case} ends in one newline")),
            "the log of {case} of {} is not its expected log",
            replayed.suite
        );
    }
    let written: Vec<String> = written
        .iter()
        .map(|name| name.trim_end_matches(".log.json").to_string())
        .collect();
    assert_eq!(
        written, expected,
        "the runner wrote a log of each case that has an expected log, and of no other"
    );
    std::fs::remove_dir_all(&logs).expect("the logs are removed");
}

#[test]
fn the_boundary_suite_replays_to_its_expected_logs() {
    replays(&BOUNDARY);
}
