// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A cargo suite can be judged by Miri, and the round reads Miri's verdict as it
// reads a test binary's.
//
// `scripts/mutate` ran a cargo suite by resolving its test executables and
// running each directly, so a tool that is not a test binary had no place in a
// round: the ADR measured by hand which of the broadcast log's orders Miri sees
// weakened (`docs/adr/0062-*`, "What the thread tests under Miri establish"),
// and the mutation round of CI could not repeat the measurement, because Miri is
// `cargo miri test` and the selector handed its arguments to `cargo test`.
// `mutation_under_miri` declares that the suite named by `mutation_tests` is
// executed by `cargo miri test` instead, on the nightly the casefile declares.
//
// What this file holds the runner to, without Miri (a probe crate stands under a
// stand-in `cargo-miri` that records its arguments and hands the run to
// `cargo test`, so the behaviour measured is the runner's own and does not depend
// on a nightly being installed):
//
//   - the baseline and every case run the suite through `cargo miri test`, once
//     each and with the selector's arguments, and not through the test binaries;
//   - a mutant that fails a test is CAUGHT, read from the libtest output Miri
//     prints exactly as the cargo runner reads it;
//   - a run that dies before it reports (Miri reports undefined behaviour and a
//     data race by ending the process, with no `test result:` line) is CAUGHT and
//     says how it ended, the reading the cargo runner gives a binary that dies,
//     because both go through one reader (`mutation_cargo_judge_output`);
//   - the two declarations that make the runner mean anything are refused where
//     the casefile is read, in every mode: another runner, and no nightly;
//   - a suite run under several seeds is counted over every seed, in the baseline
//     and in each case alike. Miri ends a run at its first failing seed unless it
//     is told to keep going, so a mutant it caught reported fewer tests than the
//     baseline and read INCONCLUSIVE (CI run 44107166637: 12 against 24).

use std::fs;
use std::os::unix::fs::PermissionsExt;
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
     name = \"miriprobe\"\n\
     version = \"0.0.0\"\n\
     edition = \"2021\"\n\n\
     [workspace]\n";

/// The subject every case mutates.
const LIB: &str = "pub fn value() -> u32 {\n    1\n}\n";

/// Two tests, so the `running 2 tests` header is a number a half-finished run
/// would still print.
const ORACLE: &str = "#[test]\n\
     fn the_value_is_one() {\n    assert_eq!(miriprobe::value(), 1);\n}\n\n\
     #[test]\n\
     fn a_second_test_so_the_header_counts_two() {}\n";

/// The toolchain the casefile declares: the one this test runs under, so that
/// the probe needs nothing installed that the test run does not already have.
/// `None` when no `rustup` can run it, in which case the round would refuse for
/// a reason that says nothing about the runner and the test says so and returns.
fn declared_toolchain() -> Option<String> {
    // The one in the environment, else the one `rustup` says is active here: a
    // host does not have `stable` installed because a test would like it to.
    let toolchain = std::env::var("RUSTUP_TOOLCHAIN").ok().or_else(|| {
        let shown = Command::new("rustup")
            .args(["show", "active-toolchain"])
            .output()
            .ok()
            .filter(|o| o.status.success())?;
        String::from_utf8_lossy(&shown.stdout)
            .split_whitespace()
            .next()
            .map(str::to_string)
    })?;
    let runs = Command::new("rustup")
        .args(["run", &toolchain, "rustc", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    runs.then_some(toolchain)
}

/// A directory holding an executable `cargo-miri`, which cargo finds on `PATH`
/// when asked for `cargo miri`. It records every call (`<args>` as cargo passed
/// them, the first being `miri`) in `$PROBE_MIRI_LOG`, answers `--version`, and
/// hands `test` to `cargo test` with the arguments that follow.
///
/// It reads `MIRIFLAGS` for the two flags of Miri's seeds and keeps their
/// meaning: under `-Zmiri-many-seeds=<from>..<to>` the test binary is run once a
/// seed, each run printing its own libtest output, and the first seed that fails
/// ends the whole run with that seed's status unless
/// `-Zmiri-many-seeds-keep-going` is given, when every seed runs, the failing
/// ones are counted, and the status is the worst. Miri runs the seeds in
/// parallel and this runs them one after another; what a round reads is the
/// same in both: the seeds after a failing one never report, unless kept going.
fn stand_in_for_cargo_miri() -> TempDir {
    let dir = tempdir().expect("temp dir for the stand-in");
    let script = dir.path().join("cargo-miri");
    fs::write(
        &script,
        r#"#!/bin/sh
echo "$@" >> "$PROBE_MIRI_LOG"
shift
case "$1" in
--version) echo 'cargo-miri 0.0.0 (a stand-in for the probe)'; exit 0 ;;
test) shift ;;
*) exit 64 ;;
esac
from=
to=
keep_going=
for flag in $MIRIFLAGS; do
    case "$flag" in
    -Zmiri-many-seeds=*)
        range=${flag#-Zmiri-many-seeds=}
        from=${range%%..*}
        to=${range#*..}
        ;;
    -Zmiri-many-seeds-keep-going) keep_going=1 ;;
    esac
done
[ -n "$to" ] || exec cargo test "$@"
seed=${from:-0}
failed=0
worst=0
while [ "$seed" -lt "$to" ]; do
    echo "Trying seed: $seed" >&2
    status=0
    cargo test "$@" || status=$?
    if [ "$status" -ne 0 ]; then
        echo "FAILING SEED: $seed" >&2
        [ -n "$keep_going" ] || exit "$status"
        failed=$((failed + 1))
        [ "$status" -le "$worst" ] || worst=$status
    fi
    seed=$((seed + 1))
done
[ "$failed" -eq 0 ] || echo "$failed/$((to - ${from:-0})) SEEDS FAILED" >&2
exit "$worst"
"#,
    )
    .expect("write the stand-in");
    let mut mode = fs::metadata(&script)
        .expect("stat the stand-in")
        .permissions();
    mode.set_mode(0o755);
    fs::set_permissions(&script, mode).expect("make the stand-in executable");
    dir
}

/// A probe crate under a gitignored path, since `scripts/mutate` derives its
/// root from `git rev-parse --show-toplevel` and the casefile names paths
/// relative to it. Outside the workspace, so cargo builds it on its own.
struct Probe {
    root: PathBuf,
    rel: String,
    casefile: PathBuf,
    ledger: TempDir,
    miri: TempDir,
    log: PathBuf,
}

impl Drop for Probe {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn probe(name: &str) -> Probe {
    let rel = format!("tmp/miri-{name}");
    let root = repo_root().join(&rel);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::create_dir_all(root.join("tests")).expect("create tests");
    fs::write(root.join("Cargo.toml"), MANIFEST).expect("write the manifest");
    fs::write(root.join("src/lib.rs"), LIB).expect("write the subject");
    fs::write(root.join("tests/oracle.rs"), ORACLE).expect("write the oracle");
    let miri = stand_in_for_cargo_miri();
    let log = miri.path().join("calls.log");
    Probe {
        casefile: root.join("probe.cases"),
        root,
        rel,
        ledger: tempdir().expect("temp ledger"),
        miri,
        log,
    }
}

impl Probe {
    /// The casefile: the declarations given, then the selector and one case per
    /// `(label, body)`, each of which replaces the `1` in `value()` with `body`.
    /// The body is the text of a Python string literal, so a newline in it is
    /// written `\n` and a quote `\"`, as two characters each.
    fn write_cases(&self, declarations: &str, cases: &[(&str, &str)]) {
        let mut casefile = format!(
            "{declarations}\
             mutation_tests --manifest-path {rel}/Cargo.toml --test oracle\n\
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

    /// The command that runs `scripts/mutate` with the stand-in first on `PATH`.
    /// `SCE_MUTATION_LEDGER_DIR` keeps the round out of the real corpus.
    fn mutate(&self, mode: Option<&str>) -> Command {
        let path = std::env::var("PATH").unwrap_or_default();
        let mut command = Command::new(repo_root().join("scripts/mutate"));
        if let Some(mode) = mode {
            command.arg(mode);
        }
        command
            .arg(&self.casefile)
            .current_dir(repo_root())
            .env("SCE_MUTATION_LEDGER_DIR", self.ledger.path())
            .env("PROBE_MIRI_LOG", &self.log)
            .env("PATH", format!("{}:{path}", self.miri.path().display()));
        command
    }

    /// Run the round; stdout and stderr together.
    fn round(&self) -> String {
        let out = self.mutate(None).output().expect("run scripts/mutate");
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    }

    /// What the stand-in was asked, one line per call.
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(&self.log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
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

fn miri_header(toolchain: &str) -> String {
    format!("mutation_env RUSTUP_TOOLCHAIN={toolchain}\nmutation_under_miri\n")
}

#[test]
fn a_round_under_miri_runs_the_baseline_and_every_case_through_cargo_miri() {
    let Some(toolchain) = declared_toolchain() else {
        eprintln!("SKIP: no rustup that can run the toolchain the probe declares");
        return;
    };
    let changes = "the value is changed";
    let aborts = "the subject aborts";

    let p = probe("round");
    p.write_cases(
        &miri_header(&toolchain),
        &[(changes, "    2"), (aborts, "    std::process::abort()")],
    );

    let round = p.round();
    assert!(
        round.contains("baseline: 2 tests, 0 failing"),
        "the baseline did not read the two tests Miri's output reports, so nothing below \
         measures what it claims to:\n{round}"
    );

    // Once for the baseline and once per case, each with the selector's own
    // arguments: the run went through `cargo miri test` and not through the
    // binaries the build produced.
    let ran: Vec<String> = p
        .calls()
        .into_iter()
        .filter(|call| call.starts_with("miri test"))
        .collect();
    assert_eq!(
        ran.len(),
        3,
        "the baseline and two cases are three runs, each through `cargo miri test`:\n{ran:#?}\n{round}"
    );
    for call in &ran {
        assert!(
            call.contains(&format!(
                "--manifest-path {}/Cargo.toml --test oracle",
                p.rel
            )),
            "a run did not carry the selector's arguments: {call}"
        );
    }

    let line = verdict_of(&round, changes);
    assert!(
        line.starts_with("CAUGHT"),
        "a mutant that fails a test under Miri is a mutant the suite noticed, and was read as \
         {line:?}:\n{round}"
    );

    let line = verdict_of(&round, aborts);
    assert!(
        line.starts_with("CAUGHT"),
        "a run that dies under Miri before it reports is a mutant the suite noticed, and was \
         read as {line:?}:\n{round}"
    );
    assert!(
        account_after(&round, aborts)
            .contains("cargo miri test (exited with status 101 before it reported a result)"),
        "the verdict did not say how the Miri run ended:\n{round}"
    );
}

/// The seeds a casefile declares for Miri, as the broadcast log's casefiles do
/// with eight: three here, so the probe's two tests are six runs of a test.
const THREE_SEEDS: &str = "mutation_env MIRIFLAGS=-Zmiri-many-seeds=0..3\n";

#[test]
fn a_mutant_that_fails_its_first_seed_is_counted_over_every_seed_the_baseline_ran() {
    let Some(toolchain) = declared_toolchain() else {
        eprintln!("SKIP: no rustup that can run the toolchain the probe declares");
        return;
    };
    let changes = "the value is changed";

    let p = probe("seeds");
    p.write_cases(
        &format!("{}{THREE_SEEDS}", miri_header(&toolchain)),
        &[(changes, "    2")],
    );

    let round = p.round();
    assert!(
        round.contains("baseline: 6 tests, 0 failing"),
        "the baseline is two tests under each of three seeds, and did not read so, so the \
         seeds the casefile declares did not reach the run:\n{round}"
    );

    // The mutant fails on the first seed. Were the run to end there, it would
    // report two tests against the baseline's six and read INCONCLUSIVE; run to
    // the end, every seed reports, and each fails the one test that reads the
    // value.
    let line = verdict_of(&round, changes);
    assert!(
        line.starts_with("CAUGHT") && line.contains("(3/6 red)"),
        "a mutant that fails under every seed must be counted over the seeds the baseline \
         ran, and was read as {line:?}:\n{round}"
    );
}

#[test]
fn a_baseline_that_miri_does_not_pass_is_not_green() {
    let Some(toolchain) = declared_toolchain() else {
        eprintln!("SKIP: no rustup that can run the toolchain the probe declares");
        return;
    };
    let p = probe("baseline");
    // The subject is already wrong where the oracle reads it.
    fs::write(
        p.root.join("src/lib.rs"),
        "pub fn value() -> u32 {\n    9\n}\n",
    )
    .expect("write the broken subject");
    p.write_cases(&miri_header(&toolchain), &[("any mutation", "    2")]);

    let round = p.round();
    assert!(
        round.contains("baseline is not green (1 failing)"),
        "a baseline that fails under Miri must not start a round over nothing:\n{round}"
    );
}

#[test]
fn a_casefile_that_declares_miri_says_so_to_the_gate_and_one_that_does_not_does_not() {
    let p = probe("declares");
    p.write_cases(&miri_header("nightly-x"), &[("any mutation", "    2")]);
    let out = p
        .mutate(Some("--declares"))
        .output()
        .expect("run --declares");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "--declares refused a casefile that is well formed:\n{text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        text.lines().any(|line| line == "miri\t1"),
        "the gate that installs the toolchain is not told Miri is needed:\n{text}"
    );
    assert!(
        text.lines()
            .any(|line| line == "env\tRUSTUP_TOOLCHAIN=nightly-x"),
        "the toolchain is not declared beside it:\n{text}"
    );

    p.write_cases(
        "mutation_env RUSTUP_TOOLCHAIN=nightly-x\n",
        &[("any mutation", "    2")],
    );
    let out = p
        .mutate(Some("--declares"))
        .output()
        .expect("run --declares");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "--declares refused a plain casefile:\n{text}"
    );
    assert!(
        !text.lines().any(|line| line.starts_with("miri\t")),
        "a casefile that did not declare Miri is declared as needing it:\n{text}"
    );
}

#[test]
fn the_two_declarations_that_make_the_runner_mean_something_are_refused_where_the_casefile_is_read()
{
    let p = probe("refused");

    // No nightly: Miri is not on stable, and a round that fell back to whatever
    // `cargo` is on the host would run a different suite from the one it names.
    p.write_cases("mutation_under_miri\n", &[("any mutation", "    2")]);
    let out = p
        .mutate(Some("--declares"))
        .output()
        .expect("run --declares");
    assert_eq!(
        out.status.code(),
        Some(2),
        "a casefile under Miri with no declared nightly was accepted:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    // The harness says its refusals on stdout (`red`), so both streams are read.
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        said.contains("needs the nightly Miri runs on"),
        "the refusal did not say what was missing:\n{said}"
    );

    // Another runner: the declaration changes how a cargo suite is executed and
    // means nothing to a ctest one.
    let ctest = format!(
        "mutation_env RUSTUP_TOOLCHAIN=nightly-x\n\
         mutation_under_miri\n\
         mutation_ctest --test-dir build -R x\n\
         mutation_targets {rel}/src/lib.rs\n",
        rel = p.rel
    );
    fs::write(&p.casefile, ctest).expect("write the casefile");
    let out = p
        .mutate(Some("--declares"))
        .output()
        .expect("run --declares");
    assert_eq!(
        out.status.code(),
        Some(2),
        "mutation_under_miri was accepted beside another runner:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        said.contains("belongs with mutation_tests"),
        "the refusal did not name the runner it belongs with:\n{said}"
    );
}
