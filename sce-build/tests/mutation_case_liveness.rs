// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `scripts/mutate --check` refuses the shapes a dead mutation case takes.
//
// The gate `mutation-cases` runs that mode over every casefile in
// `sce-build/tests/mutations/` on every push, and a gate is only worth its
// runtime if it can turn red. This file is where that is established: each
// case below hands the harness a casefile built to be broken in exactly one
// way and requires a non-zero exit naming that way, plus one built to be
// sound and required to pass.
//
// The shapes are not invented. Every one of them was found in the corpus the
// first time the mode ran:
//
//   - an anchor that matches several places, so `edit()`'s "replace the
//     first" was aiming at whichever backend happened to appear earliest in
//     `sce_codegen.rs` — three cases in two files, one of them the pair whose
//     anchor closed two `format!` calls where the cases meant one each;
//   - a selector whose `--lib` this check first rejected outright, because
//     cargo reports a library target under its crate type (`rlib`) rather
//     than under the word `lib`. That was a defect in the checker, and it is
//     pinned below so the checker cannot reacquire it.
//
// The fixtures are self-contained: each declares a target inside its own
// temporary directory, so nothing here edits the repository, and a run that
// dies half way cannot leave a mutation in the tree.

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

/// The line every fixture uses to name a suite that exists.
///
/// A real selector, because the mode resolves it: `cargo metadata` has to
/// find the package, the feature and the target. A fixture naming an
/// imaginary one would fail for that reason instead of the reason it is
/// written to test.
const LIVE_SELECTOR: &str = "mutation_tests -p sce-build --features cli --lib";

struct Fixture {
    _dir: TempDir,
    casefile: PathBuf,
    target: PathBuf,
}

/// A casefile and the file it studies, in a directory of their own.
///
/// `target_body` is written verbatim, and `cases` is appended to the
/// declarations, so a test spells only what it is about. `TARGET` in a case
/// body stands for the subject's path — the fixture knows it and the test
/// should not have to.
fn fixture(target_body: &str, cases: &str) -> Fixture {
    fixture_with_selector(LIVE_SELECTOR, target_body, cases)
}

fn fixture_with_selector(selector: &str, target_body: &str, cases: &str) -> Fixture {
    let dir = tempdir().expect("temp dir");
    let target = dir.path().join("subject.txt");
    fs::write(&target, target_body).expect("write the subject");

    let path = target.display().to_string();
    let casefile = dir.path().join("fixture.cases");
    fs::write(
        &casefile,
        format!(
            "{selector}\nmutation_targets {path}\n\n{}",
            // `{:?}` on the path yields a quoted, escaped literal, which is
            // what a Python case body needs to name it.
            cases.replace("TARGET", &format!("{path:?}"))
        ),
    )
    .expect("write the casefile");

    Fixture {
        _dir: dir,
        casefile,
        target,
    }
}

/// Run the check mode over a fixture; return (success, combined output).
fn check(casefile: &Path) -> (bool, String) {
    let out = Command::new(repo_root().join("scripts/mutate"))
        .arg("--check")
        .arg(casefile)
        .current_dir(repo_root())
        .output()
        .expect("run scripts/mutate --check");

    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), text)
}

fn assert_rejected(fixture: &Fixture, expected: &str) {
    let (ok, output) = check(&fixture.casefile);
    assert!(
        !ok,
        "the check mode accepted a casefile that cannot test anything:\n{output}"
    );
    assert!(
        output.contains(expected),
        "rejected, but not for the reason under test — expected {expected:?} in:\n{output}"
    );
}

#[test]
fn a_case_whose_anchor_is_gone_is_refused() {
    let f = fixture(
        "fn keep(x: u8) -> u8 {\n    x + 1\n}\n",
        "mutation_case \"the anchor moved\" <<'PY'\n\
         edit(TARGET, \"x + 2\", \"x + 3\")\n\
         PY\n",
    );
    assert_rejected(&f, "anchor not found");
}

#[test]
fn a_case_whose_anchor_matches_twice_is_refused() {
    // The shape that cost three cases in the corpus: four backends spelled
    // one predicate identically, and the case mutated whichever came first.
    let f = fixture(
        "fn a() -> bool {\n    ready()\n}\n\nfn b() -> bool {\n    ready()\n}\n",
        "mutation_case \"which one does this mean\" <<'PY'\n\
         edit(TARGET, \"    ready()\", \"    false\")\n\
         PY\n",
    );
    assert_rejected(&f, "anchor matches 2 places");
}

#[test]
fn a_case_that_replaces_its_anchor_with_itself_is_refused() {
    let f = fixture(
        "fn keep(x: u8) -> u8 {\n    x + 1\n}\n",
        "mutation_case \"a mutation that mutates nothing\" <<'PY'\n\
         edit(TARGET, \"x + 1\", \"x + 1\")\n\
         PY\n",
    );
    assert_rejected(&f, "replaces its anchor with itself");
}

#[test]
fn a_casefile_with_no_cases_is_refused() {
    // The emptiest form of the failure: declares a suite and a target, runs
    // clean, tests nothing.
    let f = fixture("fn keep() {}\n", "");
    assert_rejected(&f, "declared no mutation_case");
}

#[test]
fn a_declared_target_that_does_not_exist_is_refused() {
    let f = fixture(
        "fn keep(x: u8) -> u8 {\n    x + 1\n}\n",
        "mutation_case \"studies a file that is gone\" <<'PY'\n\
         edit(TARGET, \"x + 1\", \"x + 2\")\n\
         PY\n",
    );
    fs::remove_file(&f.target).expect("remove the subject");
    assert_rejected(&f, "declared target does not exist");
}

#[test]
fn a_selector_naming_a_suite_that_does_not_exist_is_refused() {
    let f = fixture_with_selector(
        "mutation_tests -p sce-build --features cli --test no_such_suite_exists",
        "fn keep(x: u8) -> u8 {\n    x + 1\n}\n",
        "mutation_case \"the suite was renamed out from under it\" <<'PY'\n\
         edit(TARGET, \"x + 1\", \"x + 2\")\n\
         PY\n",
    );
    assert_rejected(&f, "names no test target");
}

#[test]
fn a_sound_case_passes_and_leaves_its_subject_where_it_found_it() {
    let body = "fn keep(x: u8) -> u8 {\n    x + 1\n}\n";
    let f = fixture(
        body,
        "mutation_case \"the arithmetic is wrong\" <<'PY'\n\
         edit(TARGET, \"x + 1\", \"x - 1\")\n\
         PY\n",
    );

    let (ok, output) = check(&f.casefile);
    assert!(ok, "the check mode refused a sound casefile:\n{output}");
    assert!(
        output.contains("1/1 case(s) still apply"),
        "a sound casefile did not report as applying:\n{output}"
    );
    // The mode applies each case for real — that is what makes it evidence
    // rather than a parse — so the restore is half of what it promises.
    assert_eq!(
        fs::read_to_string(&f.target).expect("read the subject back"),
        body,
        "the check mode left its subject mutated"
    );
}

#[test]
fn a_library_selector_resolves_against_the_crate_type_cargo_reports() {
    // `--lib` in `LIVE_SELECTOR` is the point of this test: cargo reports
    // sce-build's library target as `rlib`, and a checker matching the
    // literal string "lib" called every `--lib` casefile broken. Two in this
    // corpus use that selector, so the defect read as a dead corpus rather
    // than as a bug in the check.
    let f = fixture(
        "fn keep(x: u8) -> u8 {\n    x + 1\n}\n",
        "mutation_case \"a case behind a --lib selector\" <<'PY'\n\
         edit(TARGET, \"x + 1\", \"x - 1\")\n\
         PY\n",
    );
    let (ok, output) = check(&f.casefile);
    assert!(
        ok,
        "a `--lib` selector was rejected against a workspace whose library \
         target exists:\n{output}"
    );
}
