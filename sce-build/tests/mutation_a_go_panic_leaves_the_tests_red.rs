// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A Go test binary that panics has failed every test it never reached.
//
// `scripts/mutate` counts a Go suite by the verdict line each test prints, and
// compares the count a mutant ran with the baseline's: a mutant that changes
// which tests exist is no verdict, because it has not been judged against the
// same suite. A panic ends the whole binary, not the test that raised it, so
// the tests after it print no verdict line at all, and a mutant that crashed
// the suite read as one that changed which tests exist — `INCONCLUSIVE, ran 1
// tests, baseline ran 76` — when the tests were as red as they can be.
//
// Measured 2026-10-14 on `a_static_go_child_is_built_with_the_host_its_parent_
// answers_go.cases`, whose two mutants build a child with a nil host and whose
// first act dereferences it.
//
// The runner now lists the binary's tests and counts each one it never started
// as run and failed, so the round's count is the baseline's and the verdict is
// CAUGHT, with the tests named as not reached. The control below is the other
// half: a mutant that fails one test and panics in none is counted as it was.
//
// The real `scripts/mutate` is run over a real Go module, because the decision
// is made in the runner's own loop.

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

const GO_MOD: &str = "module gopanicprobe\n\ngo 1.21\n";

/// The subject every case mutates.
const SUBJECT: &str = "package probe\n\nfunc Value() int {\n\treturn 1\n}\n";

/// Three tests, so a binary that dies in the first leaves two with no verdict.
const ORACLE: &str = "package probe\n\n\
     import \"testing\"\n\n\
     func TestFirst(t *testing.T) {\n\tif Value() != 1 {\n\t\tt.Fatal(\"not one\")\n\t}\n}\n\n\
     func TestSecond(t *testing.T) {\n\tif Value() != 1 {\n\t\tt.Fatal(\"not one\")\n\t}\n}\n\n\
     func TestThird(t *testing.T) {}\n";

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

fn probe() -> Probe {
    let rel = "tmp/go-panic-probe".to_string();
    let root = repo_root().join(&rel);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create the module");
    fs::write(root.join("go.mod"), GO_MOD).expect("write go.mod");
    fs::write(root.join("subject.go"), SUBJECT).expect("write the subject");
    fs::write(root.join("subject_test.go"), ORACLE).expect("write the oracle");
    Probe {
        casefile: root.join("probe.cases"),
        root,
        rel,
        _ledger: tempdir().expect("temp ledger"),
    }
}

impl Probe {
    /// The casefile: each case replaces `return 1` in `Value()` with `body`, the
    /// text of a Python string literal.
    fn write_cases(&self, cases: &[(&str, &str)]) {
        let mut casefile = format!(
            "mutation_go {rel} .\n\
             mutation_targets {rel}/subject.go\n\
             mutation_oracles {rel}/subject_test.go\n\n",
            rel = self.rel
        );
        for (label, body) in cases {
            casefile.push_str(&format!(
                "mutation_case \"{label}\" <<'PY'\n\
                 edit(\"{rel}/subject.go\", \"\\treturn 1\", \"{body}\")\n\
                 PY\n\n",
                rel = self.rel
            ));
        }
        fs::write(&self.casefile, casefile).expect("write the casefile");
    }

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

fn verdict_of<'a>(round: &'a str, label: &str) -> &'a str {
    round
        .lines()
        .find(|line| line.contains(label) && !line.trim_start().starts_with("red:"))
        .unwrap_or_else(|| panic!("the round printed no verdict for {label:?}:\n{round}"))
        .trim_start()
}

#[test]
fn a_mutant_that_panics_leaves_every_test_it_never_reached_red() {
    if Command::new("go").arg("version").output().is_err() {
        eprintln!("SKIP: no go toolchain to build the probe with");
        return;
    }
    let panics = "the subject panics";
    let fails_one = "the subject answers wrong";

    let p = probe();
    p.write_cases(&[
        (panics, "\\tpanic(\\\"gone\\\")"),
        (fails_one, "\\treturn 2"),
    ]);

    let round = p.round();
    assert!(
        round.contains("baseline: 3 tests, 0 failing"),
        "the probe's baseline did not run its three tests, so nothing below measures what it \
         claims to:\n{round}"
    );

    let line = verdict_of(&round, panics);
    assert!(
        line.starts_with("CAUGHT") && line.contains("3/3 red"),
        "a panic that ends the binary in its first test fails all three, and was read as \
         {line:?}:\n{round}"
    );
    assert!(
        round.contains("TestSecond (not reached: the binary died in a panic)")
            && round.contains("TestThird (not reached: the binary died in a panic)"),
        "the tests the panic kept from running were not named:\n{round}"
    );

    let line = verdict_of(&round, fails_one);
    assert!(
        line.starts_with("CAUGHT") && line.contains("2/3 red"),
        "a mutant that fails two tests and panics in none is counted as it was, and was read \
         as {line:?}:\n{round}"
    );
    let after = &round[round.find(fails_one).expect("its verdict")..];
    assert!(
        !after.contains("not reached"),
        "a test that ran was named as not reached:\n{round}"
    );
}
