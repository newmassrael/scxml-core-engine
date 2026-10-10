// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A casefile says how long its test binary may run, and the harness keeps to it.
//
// `scripts/mutate` bounds one test binary at 300 seconds, so that a mutant that
// never ends is a verdict and not a frozen round. The same clock bounds the
// BASELINE, and a suite that is slow by nature cannot beat it: measured
// 2026-10-10, a loom model of two producers racing to link a segment takes 547
// seconds on the build machine, and the casefile whose oracle it was had its
// baseline reported as "did not terminate within 300s" on every run, so not one
// of its cases was ever judged.
//
// `mutation_timeout <seconds>` is how a casefile asks for more. What this file
// holds the harness to:
//
//   - a casefile that declares it has the number printed by `--declares`, and
//     one that does not prints nothing new, so every existing caller reads the
//     output it always read;
//   - the number is a whole number of seconds, above zero and no more than an
//     hour, declared once: anything else is refused (exit 2) before a build,
//     because one case that never ends costs the whole bound and a typo must
//     not be able to turn a round into a day;
//   - the declared number is the clock the baseline and the cases run under,
//     and a value in the environment, which is a person asking for a short
//     round, wins over it.
//
// The last is measured by running real rounds over a probe crate whose oracle
// pauses for four seconds: a harness that parsed the word and kept the old
// clock would pass every other assertion here.

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

const MANIFEST: &str = "[package]\n\
     name = \"slowprobe\"\n\
     version = \"0.0.0\"\n\
     edition = \"2021\"\n\n\
     [workspace]\n";

/// The subject the one case mutates.
const LIB: &str = "pub fn value() -> u32 {\n    1\n}\n";

/// One test that pauses before it asserts, so that a bound shorter than the
/// pause ends the binary and a longer one lets it finish.
const ORACLE: &str = "#[test]\n\
     fn the_value_is_one_after_a_pause() {\n    \
     std::thread::sleep(std::time::Duration::from_secs(4));\n    \
     assert_eq!(slowprobe::value(), 1);\n}\n";

/// A probe crate under a gitignored path, since `scripts/mutate` derives its
/// root from `git rev-parse --show-toplevel` and the casefile names paths
/// relative to it. Outside the workspace (`[workspace]` in its own manifest), so
/// cargo builds it on its own and the repository's lockfile never sees it.
struct Probe {
    root: PathBuf,
    rel: String,
    casefile: PathBuf,
    ledger: TempDir,
}

impl Drop for Probe {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn probe(name: &str) -> Probe {
    let rel = format!("tmp/timeout-{name}");
    let root = repo_root().join(&rel);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(root.join("Cargo.toml"), MANIFEST).expect("write the manifest");
    fs::write(root.join("src/lib.rs"), LIB).expect("write the subject");
    fs::write(root.join("tests/oracle.rs"), ORACLE).expect("write the oracle");
    Probe {
        casefile: root.join("probe.cases"),
        root,
        rel,
        ledger: tempdir().expect("temp ledger"),
    }
}

impl Probe {
    /// The casefile: `directives` first, then the selector and one case that
    /// makes `value()` return 2, which the oracle turns red.
    fn write(&self, directives: &str) {
        let casefile = format!(
            "{directives}\n\
             mutation_tests --manifest-path {rel}/Cargo.toml --test oracle\n\
             mutation_targets {rel}/src/lib.rs\n\
             mutation_oracles {rel}/tests/oracle.rs\n\n\
             mutation_case \"the subject answers two\" <<'PY'\n\
             edit(\"{rel}/src/lib.rs\", \"    1\\n}}\", \"    2\\n}}\")\n\
             PY\n",
            rel = self.rel
        );
        fs::write(&self.casefile, casefile).expect("write the casefile");
    }

    /// The harness over this casefile with `args` and `env`; status, then
    /// stdout and stderr together.
    fn run(&self, args: &[&str], env: &[(&str, &str)]) -> (Option<i32>, String) {
        let mut command = Command::new(repo_root().join("scripts/mutate"));
        command
            .args(args)
            .arg(&self.casefile)
            .current_dir(repo_root())
            .env("SCE_MUTATION_LEDGER_DIR", self.ledger.path())
            .env_remove("MUTATION_ARTIFACT_TIMEOUT");
        for (key, value) in env {
            command.env(key, value);
        }
        let out = command.output().expect("run scripts/mutate");
        (
            out.status.code(),
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        )
    }
}

#[test]
fn a_declared_bound_is_printed_and_an_undeclared_one_prints_nothing() {
    let p = probe("declares");

    p.write("mutation_timeout 900");
    let (status, out) = p.run(&["--declares"], &[]);
    assert_eq!(status, Some(0), "--declares refused a valid bound:\n{out}");
    assert!(
        out.lines().any(|line| line == "timeout\t900"),
        "--declares did not print the bound the casefile declared:\n{out}"
    );

    p.write("");
    let (status, out) = p.run(&["--declares"], &[]);
    assert_eq!(status, Some(0), "--declares refused a casefile:\n{out}");
    assert!(
        !out.lines().any(|line| line.starts_with("timeout")),
        "a casefile that declared no bound printed one, and every caller that read the \
         output as it always was now reads a new line:\n{out}"
    );
}

#[test]
fn a_bound_that_is_not_a_whole_number_of_seconds_in_range_is_refused_before_a_build() {
    let p = probe("refused");
    let refused = [
        ("abc", "not a number"),
        ("0", "zero"),
        ("-5", "negative"),
        ("1.5", "fractional"),
        ("", "missing"),
        ("10 20", "two arguments"),
        ("3601", "above the hour"),
    ];
    for (argument, why) in refused {
        p.write(&format!("mutation_timeout {argument}"));
        let (status, out) = p.run(&["--declares"], &[]);
        assert_eq!(
            status,
            Some(2),
            "`mutation_timeout {argument}` ({why}) was not refused with exit 2:\n{out}"
        );
        assert!(
            out.contains("mutation_timeout"),
            "the refusal of `mutation_timeout {argument}` ({why}) did not name the directive:\n{out}"
        );
    }

    p.write("mutation_timeout 3600");
    let (status, out) = p.run(&["--declares"], &[]);
    assert_eq!(
        status,
        Some(0),
        "an hour is the limit and is allowed:\n{out}"
    );

    p.write("mutation_timeout 600\nmutation_timeout 900");
    let (status, out) = p.run(&["--declares"], &[]);
    assert_eq!(status, Some(2), "a second declaration was accepted:\n{out}");
    assert!(
        out.contains("twice"),
        "the refusal did not say the bound was declared twice:\n{out}"
    );
}

#[test]
fn the_declared_bound_is_the_clock_the_baseline_and_the_cases_run_under() {
    let p = probe("clock");

    // A bound shorter than the oracle's pause ends the baseline, and the round
    // says it was the clock that did.
    p.write("mutation_timeout 2");
    let (status, out) = p.run(&[], &[]);
    assert_ne!(
        status,
        Some(0),
        "a baseline the bound ends must not pass:\n{out}"
    );
    assert!(
        out.contains("did not terminate within 2s"),
        "the declared 2s was not the clock the baseline ran under:\n{out}"
    );

    // A bound longer than the pause lets the baseline finish, and the case that
    // follows is judged: without the declaration the default would have allowed
    // this too, so the line below is what shows the declaration was read.
    p.write("mutation_timeout 30");
    let (status, out) = p.run(&[], &[]);
    assert_eq!(
        status,
        Some(0),
        "a round within its declared bound failed:\n{out}"
    );
    assert!(
        out.contains("bounded at 30s, as the casefile declares"),
        "the round did not say which bound it ran under:\n{out}"
    );
    assert!(
        out.contains("baseline: 1 tests, 0 failing"),
        "the baseline did not run the one test it should:\n{out}"
    );
    assert!(
        out.contains("CAUGHT") && out.contains("the subject answers two"),
        "the case was not judged after the baseline finished:\n{out}"
    );

    // The caller's own value wins over the declaration.
    let (status, out) = p.run(&[], &[("MUTATION_ARTIFACT_TIMEOUT", "2")]);
    assert_ne!(
        status,
        Some(0),
        "an environment bound of 2s did not end the baseline:\n{out}"
    );
    assert!(
        out.contains("did not terminate within 2s"),
        "the casefile's declaration overruled the caller's own bound:\n{out}"
    );
    assert!(
        out.contains("by the environment (the casefile declares 30s)"),
        "the round did not say the environment overrode the declaration:\n{out}"
    );
}
