// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A test binary that dies under a mutation has not let the mutation survive.
//
// `scripts/mutate` runs a cargo suite by resolving its test executables once
// and running each directly, and it read the verdict out of what the binary
// printed: the `running N tests` header for how many tests there were, and the
// `test result:` line for how many failed. A binary that ends without that
// second line has printed the first and nothing else, so a mutant that killed
// it read as N tests, none red: `SURVIVED (0/N red)`.
//
// Measured 2026-10-08, on the loom models of the one-producer queue. A model
// that finds a causality violation panics; the unwinding drops the queue,
// whose `Drop` touches the cell loom still holds in a read; a panic in a
// destructor aborts the process (`signal: 6, SIGABRT`), and two of the six
// mutations of the queue's memory orderings were reported as survivors while
// the models were failing as loudly as a test can. The mutation-rounds lane
// was red for it, and the reading it gave was the opposite of the truth.
//
// The run-count guard cannot see this. libtest prints the header before the
// first test starts, so a binary that dies half-way through still reports the
// baseline's count; the Go runner, which counts per-test verdict lines, is
// protected by the same guard and has no such hole.
//
// What the harness now reads, and what this file holds it to:
//
//   - a binary that exits non-zero with no failed test counted is red, named,
//     with how it ended (`killed by signal 6 (ABRT)`, `exited with status 3`),
//     because the baseline ran it to completion and the mutant did not;
//   - one that something outside the run ended (SIGKILL, SIGTERM) is NOT a
//     verdict either way: that says nothing about the mutation, so the case
//     is INCONCLUSIVE and says why;
//   - the same reading applies to the baseline, which used to count a binary
//     that crashed as a green one and so started a round over nothing.
//
// The behaviour is measured by running real rounds over a probe crate, because
// the decision is made in the runner's own loop and a parser test would only
// show that the words are spelled right.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{tempdir, TempDir};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

// ── The sentence ───────────────────────────────────────────────────

/// What `mutation_exit_account` says for an exit status.
fn exit_account(status: u32) -> String {
    let out = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "source scripts/lib/mutation_failures.sh; mutation_exit_account {status}"
        ))
        .current_dir(repo_root())
        .output()
        .expect("run the helper");
    assert!(
        out.status.success(),
        "mutation_exit_account {status} exited {}: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn a_status_above_128_is_a_signal_and_anything_else_is_a_status() {
    assert_eq!(exit_account(134), "killed by signal 6 (ABRT)");
    assert_eq!(exit_account(139), "killed by signal 11 (SEGV)");
    assert_eq!(exit_account(3), "exited with status 3");
    assert_eq!(exit_account(101), "exited with status 101");
}

// ── The behaviour ──────────────────────────────────────────────────

const MANIFEST: &str = "[package]\n\
     name = \"deathprobe\"\n\
     version = \"0.0.0\"\n\
     edition = \"2021\"\n\n\
     [workspace]\n";

/// The subject every case mutates.
const LIB: &str = "pub fn value() -> u32 {\n    1\n}\n";

/// The same subject, already dead: the baseline of the second round.
const LIB_THAT_ABORTS: &str = "pub fn value() -> u32 {\n    std::process::abort()\n}\n";

/// Two tests, so the `running 2 tests` header is a number a half-finished run
/// would still print.
const ORACLE: &str = "#[test]\n\
     fn the_value_is_one() {\n    assert_eq!(deathprobe::value(), 1);\n}\n\n\
     #[test]\n\
     fn a_second_test_so_the_header_counts_two() {}\n";

/// A probe crate under a gitignored path, since `scripts/mutate` derives its
/// root from `git rev-parse --show-toplevel` and the casefile names paths
/// relative to it. Outside the workspace (`[workspace]` in its own manifest),
/// so cargo builds it on its own and the repository's lockfile never sees it.
struct Probe {
    root: PathBuf,
    rel: String,
    casefile: PathBuf,
    _ledger: TempDir,
}

impl Drop for Probe {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn probe(name: &str, lib: &str) -> Probe {
    let rel = format!("tmp/dies-{name}");
    let root = repo_root().join(&rel);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(root.join("Cargo.toml"), MANIFEST).expect("write the manifest");
    fs::write(root.join("src/lib.rs"), lib).expect("write the subject");
    fs::write(root.join("tests/oracle.rs"), ORACLE).expect("write the oracle");

    Probe {
        casefile: root.join("probe.cases"),
        root,
        rel,
        _ledger: tempdir().expect("temp ledger"),
    }
}

impl Probe {
    /// The casefile: the selector, then one case per `(label, body)`, each of
    /// which replaces the `1` in `value()` with `body`. The body is the text
    /// of a Python string literal, so a newline in it is written `\n` and a
    /// quote `\"`, as two characters each.
    fn write_cases(&self, cases: &[(&str, &str)]) {
        let mut casefile = format!(
            "mutation_tests --manifest-path {rel}/Cargo.toml --test oracle\n\
             mutation_targets {rel}/src/lib.rs\n\
             mutation_oracles {rel}/tests/oracle.rs\n\n",
            rel = self.rel
        );
        for (label, body) in cases {
            casefile.push_str(&format!(
                "mutation_case \"{label}\" <<'PY'\n\
                 edit(\"{rel}/src/lib.rs\", \"    1\\n}}\", \"{body}\\n}}\")\n\
                 PY\n\n",
                rel = self.rel
            ));
        }
        fs::write(&self.casefile, casefile).expect("write the casefile");
    }

    /// Run the round; stdout and stderr together, in the order they arrived.
    ///
    /// `SCE_MUTATION_LEDGER_DIR` keeps the round out of the real corpus.
    fn round(&self) -> String {
        let out = Command::new(repo_root().join("scripts/mutate"))
            .arg(&self.casefile)
            .current_dir(repo_root())
            .env("SCE_MUTATION_LEDGER_DIR", self._ledger.path())
            .output()
            .expect("run scripts/mutate");
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    }
}

/// The verdict line of the case named `label`, which starts with its verdict.
fn verdict_of<'a>(round: &'a str, label: &str) -> &'a str {
    round
        .lines()
        .find(|line| line.contains(label) && !line.trim_start().starts_with("red:"))
        .unwrap_or_else(|| panic!("the round printed no verdict for {label:?}:\n{round}"))
        .trim_start()
}

/// The round's account of the lines that follow a verdict, up to the next one.
fn account_after(round: &str, label: &str) -> String {
    let mut lines = round.lines().skip_while(|line| !line.contains(label));
    let _ = lines.next();
    lines
        .take_while(|line| {
            let word = line.split_whitespace().next().unwrap_or("");
            !matches!(word, "CAUGHT" | "SURVIVED" | "INCONCLUSIVE")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_mutant_that_kills_the_test_binary_is_caught_unless_something_outside_did() {
    if Command::new("kill")
        .arg("-l")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        eprintln!("SKIP: no kill(1) to end the probe's own process with");
        return;
    }

    let aborts = "the subject aborts";
    let exits = "the subject exits without a result";
    let killed = "the subject is killed from outside";

    let p = probe("round", LIB);
    p.write_cases(&[
        (aborts, "    std::process::abort()"),
        (exits, "    std::process::exit(3)"),
        (
            killed,
            "    let pid = std::process::id().to_string();\\n    \
             std::process::Command::new(\\\"kill\\\").args([\\\"-KILL\\\", pid.as_str()]).status().unwrap();\\n    1",
        ),
    ]);

    let round = p.round();
    assert!(
        round.contains("baseline: 2 tests, 0 failing"),
        "the probe's baseline did not run the two tests it should, so nothing below measures \
         what it claims to:\n{round}"
    );

    let line = verdict_of(&round, aborts);
    assert!(
        line.starts_with("CAUGHT"),
        "a mutant that aborts the test binary is a mutant the suite noticed, and was read as \
         {line:?}:\n{round}"
    );
    assert!(
        account_after(&round, aborts)
            .contains("killed by signal 6 (ABRT) before it reported a result"),
        "the verdict did not say how the binary died:\n{round}"
    );

    let line = verdict_of(&round, exits);
    assert!(
        line.starts_with("CAUGHT"),
        "a binary that exits before it reports is read as {line:?}:\n{round}"
    );
    assert!(
        account_after(&round, exits).contains("exited with status 3 before it reported a result"),
        "the verdict did not say which status the binary left with:\n{round}"
    );

    let line = verdict_of(&round, killed);
    assert!(
        line.starts_with("INCONCLUSIVE"),
        "a binary something outside the run killed says nothing about the mutation, and was \
         read as {line:?}:\n{round}"
    );
    assert!(
        round.contains("which says nothing about the code it ran"),
        "the INCONCLUSIVE verdict did not say why:\n{round}"
    );
}

#[test]
fn a_baseline_that_dies_is_not_green() {
    let p = probe("baseline", LIB_THAT_ABORTS);
    p.write_cases(&[("any mutation", "    2")]);

    let round = p.round();
    assert!(
        round.contains("baseline is not green (1 failing)"),
        "a baseline whose binary aborts must not start a round over nothing:\n{round}"
    );
    assert!(
        round.contains("killed by signal 6 (ABRT) before it reported a result"),
        "the refusal did not say how the baseline's binary died:\n{round}"
    );
}
