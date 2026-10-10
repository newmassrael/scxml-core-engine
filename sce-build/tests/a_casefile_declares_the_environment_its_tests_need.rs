// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A casefile says what environment its tests need, and the harness gives it to
// a round and to nothing else.
//
// Some oracles are cargo test under a different build. The queue runtime's
// real-thread tests are judged by ThreadSanitizer, which wants a nightly
// toolchain, and a toolchain is named by `RUSTUP_TOOLCHAIN` or by the `+nightly`
// the `rustup` proxy reads before cargo sees any argument: a casefile's selector
// is cargo's arguments and has nowhere to put it. `mutation_env NAME=VALUE` is
// where it goes. What this file holds the harness to:
//
//   - a casefile that declares it has each variable printed by `--declares` as
//     an `env` line, and one that does not prints nothing new, so every existing
//     caller reads the output it always read;
//   - a name that is not upper-case, a missing or empty value, a variable of the
//     harness's own, and one name with two values are refused (exit 2) before a
//     build, while the same value twice, which is one casefile read twice, is
//     not;
//   - the declaration reaches the test binary a full round runs, and replaces a
//     value the caller had, and the round says so;
//   - `--declares` and `--check` apply nothing: they run on every push with the
//     host's own toolchain, and a declared toolchain the host has not installed
//     must not fail them;
//   - a full round on a host without the declared toolchain refuses before a
//     build, naming the toolchain and the command that installs it.
//
// The behaviour is measured by running real rounds over a probe crate whose
// oracle reads the variable at run time: a harness that parsed the word and
// exported nothing would pass every other assertion here.

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
     name = \"envprobe\"\n\
     version = \"0.0.0\"\n\
     edition = \"2021\"\n\n\
     [workspace]\n";

/// The subject the one case mutates.
const LIB: &str = "pub fn value() -> u32 {\n    1\n}\n";

/// One test that fails unless the round gave the binary the declared variable
/// and the subject answers one, so the baseline is green only with the
/// environment in place and the case, which changes the answer, is CAUGHT.
const ORACLE: &str = "#[test]\n\
     fn the_probe_sees_the_declared_environment() {\n    \
     assert_eq!(std::env::var(\"PROBE_MODE\").as_deref(), Ok(\"declared\"));\n    \
     assert_eq!(envprobe::value(), 1);\n}\n";

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
    let rel = format!("tmp/env-{name}");
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
    /// stdout and stderr together. The probe's own variable is removed first, so
    /// a value a developer's shell happens to carry cannot decide a result.
    fn run(&self, args: &[&str], env: &[(&str, &str)]) -> (Option<i32>, String) {
        let mut command = Command::new(repo_root().join("scripts/mutate"));
        command
            .args(args)
            .arg(&self.casefile)
            .current_dir(repo_root())
            .env("SCE_MUTATION_LEDGER_DIR", self.ledger.path())
            .env_remove("MUTATION_ARTIFACT_TIMEOUT")
            .env_remove("PROBE_MODE");
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
fn a_declared_variable_is_printed_and_an_undeclared_casefile_prints_nothing() {
    let p = probe("declares");

    p.write("mutation_env PROBE_MODE=declared OTHER_ONE=two");
    let (status, out) = p.run(&["--declares"], &[]);
    assert_eq!(
        status,
        Some(0),
        "--declares refused a valid casefile:\n{out}"
    );
    assert!(
        out.lines().any(|line| line == "env\tPROBE_MODE=declared"),
        "--declares did not print the first variable:\n{out}"
    );
    assert!(
        out.lines().any(|line| line == "env\tOTHER_ONE=two"),
        "--declares did not print the second variable:\n{out}"
    );

    p.write("");
    let (status, out) = p.run(&["--declares"], &[]);
    assert_eq!(status, Some(0), "--declares refused a casefile:\n{out}");
    assert!(
        !out.lines().any(|line| line.starts_with("env")),
        "a casefile that declared no environment printed an `env` line, and every caller that \
         read the output as it always was now reads a new one:\n{out}"
    );
}

#[test]
fn a_declaration_the_harness_cannot_keep_is_refused_before_a_build() {
    let p = probe("refused");
    let refused = [
        ("probe_mode=declared", "a lower-case name"),
        ("1PROBE=declared", "a name that starts with a digit"),
        ("PROBE_MODE", "no value"),
        ("PROBE_MODE=", "an empty value"),
        ("=declared", "no name"),
        ("MUTATION_ARTIFACT_TIMEOUT=1", "the harness's own variable"),
        ("SCE_MUTATION_LEDGER_DIR=/tmp", "the harness's own variable"),
    ];
    for (word, why) in refused {
        p.write(&format!("mutation_env {word}"));
        let (status, out) = p.run(&["--declares"], &[]);
        assert_eq!(
            status,
            Some(2),
            "`mutation_env {word}` ({why}) was not refused with exit 2:\n{out}"
        );
        assert!(
            out.contains("mutation_env"),
            "the refusal of `mutation_env {word}` ({why}) did not name the directive:\n{out}"
        );
    }

    p.write("mutation_env");
    let (status, out) = p.run(&["--declares"], &[]);
    assert_eq!(
        status,
        Some(2),
        "a declaration of nothing was accepted:\n{out}"
    );

    p.write("mutation_env PROBE_MODE=declared\nmutation_env PROBE_MODE=other");
    let (status, out) = p.run(&["--declares"], &[]);
    assert_eq!(
        status,
        Some(2),
        "one name with two values was accepted:\n{out}"
    );
    assert!(
        out.contains("twice"),
        "the refusal did not say the name was declared twice:\n{out}"
    );

    p.write("mutation_env PROBE_MODE=declared\nmutation_env PROBE_MODE=declared");
    let (status, out) = p.run(&["--declares"], &[]);
    assert_eq!(
        status,
        Some(0),
        "the same declaration twice is one casefile read twice and was refused:\n{out}"
    );
}

#[test]
fn the_declared_environment_reaches_the_test_binary_and_replaces_the_callers() {
    let p = probe("round");
    p.write("mutation_env PROBE_MODE=declared");

    let (status, out) = p.run(&[], &[]);
    assert_eq!(
        status,
        Some(0),
        "a round whose oracle needs the declared variable failed, so it did not reach the \
         binary:\n{out}"
    );
    assert!(
        out.contains("baseline: 1 tests, 0 failing"),
        "the baseline did not run the one test it should:\n{out}"
    );
    assert!(
        out.contains("CAUGHT") && out.contains("the subject answers two"),
        "the case was not judged once the baseline passed:\n{out}"
    );
    assert!(
        out.contains("mutation_env PROBE_MODE=declared") && !out.contains("replaces the caller's"),
        "the round did not say which variable it applied, or claimed to replace a value the \
         caller never had:\n{out}"
    );

    let (status, out) = p.run(&[], &[("PROBE_MODE", "caller")]);
    assert_eq!(
        status,
        Some(0),
        "a value in the caller's environment overruled the casefile's declaration:\n{out}"
    );
    assert!(
        out.contains("mutation_env PROBE_MODE=declared replaces the caller's caller"),
        "the round did not say it replaced the caller's value:\n{out}"
    );
}

#[test]
fn a_declared_toolchain_is_applied_by_neither_a_declaration_nor_a_check() {
    let p = probe("check");
    p.write("mutation_env RUSTUP_TOOLCHAIN=nightly-1999-01-01");

    let (status, out) = p.run(&["--declares"], &[]);
    assert_eq!(
        status,
        Some(0),
        "--declares applied a toolchain this host does not have:\n{out}"
    );
    assert!(
        out.lines()
            .any(|line| line == "env\tRUSTUP_TOOLCHAIN=nightly-1999-01-01"),
        "--declares did not print the declared toolchain, so the gate cannot install it:\n{out}"
    );

    // `--check` reads the WORKSPACE's manifests to see that the selector names
    // something that exists, and the probe crate is outside it by design, so the
    // casefile below selects a real package and keeps its one edit in the probe,
    // which is all `--check` applies and restores.
    let casefile = format!(
        "mutation_env RUSTUP_TOOLCHAIN=nightly-1999-01-01\n\
         mutation_tests -p sce-forge-runtime --lib\n\
         mutation_targets {rel}/src/lib.rs\n\
         mutation_oracles {rel}/tests/oracle.rs\n\n\
         mutation_case \"the subject answers two\" <<'PY'\n\
         edit(\"{rel}/src/lib.rs\", \"    1\\n}}\", \"    2\\n}}\")\n\
         PY\n",
        rel = p.rel
    );
    fs::write(&p.casefile, casefile).expect("write the casefile");
    let (status, out) = p.run(&["--check"], &[]);
    assert_eq!(
        status,
        Some(0),
        "--check applied a toolchain this host does not have, and every push runs it:\n{out}"
    );
    assert!(
        out.contains("1/1 case(s) still apply"),
        "--check did not get as far as applying the case, so it proves nothing about the \
         toolchain:\n{out}"
    );
}

#[test]
fn a_round_on_a_host_without_the_declared_toolchain_refuses_before_a_build() {
    let p = probe("toolchain");
    p.write("mutation_env RUSTUP_TOOLCHAIN=nightly-1999-01-01");

    let (status, out) = p.run(&[], &[]);
    assert_eq!(
        status,
        Some(2),
        "a round on a host without the toolchain was not refused with exit 2:\n{out}"
    );
    assert!(
        out.contains("nightly-1999-01-01")
            && out.contains("rustup toolchain install nightly-1999-01-01"),
        "the refusal did not name the toolchain and the command that installs it:\n{out}"
    );
    assert!(
        !out.contains("baseline:"),
        "the round built and ran a baseline under a toolchain it knew the host lacked:\n{out}"
    );
}
