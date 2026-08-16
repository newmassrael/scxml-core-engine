// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The `mutation-rounds` gate runs a round for the casefiles a change reaches,
// and for no others.
//
// `mutation-cases` asks the cheap half of the mutation question — does every
// case still find an unambiguous anchor. It says in its own header that it
// answers no more than that: it does not build and does not run a test, so a
// case whose mutated tree stopped COMPILING, or one the suite no longer turns
// red for, reads as present and proves nothing. `mutation-rounds` is the
// other half, and the thing that makes it affordable on a push is that it
// runs a round only where a declared target actually moved.
//
// That selection is the whole gate. Get it wrong in one direction and pushes
// grow an hour; wrong in the other and the gate reports green for having run
// nothing — the failure mode the corpus already produced once, when
// `parallel_microstep_owns_exit_and_entry.cases` sat in the debt registry as
// CAUGHT 1/1 while it was in fact INCONCLUSIVE.
//
// So the selection is asserted here rather than the rounds. `SCE_MUTATION_
// ROUNDS_DRY_RUN=1` stops the gate once it has chosen, which is what makes
// the choosing observable in milliseconds instead of an hour. The rounds
// themselves belong to `scripts/mutate`, and `mutation_case_liveness.rs`
// covers that half.
//
// The change sets below are written into a temp file, never into the
// repository: what the gate reads is a list of paths, and a test that had to
// touch those paths to be selected would be editing the tree it is judging.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Run the gate in dry-run mode over a change set from a tree that has no
/// configured CMake build, and return what it chose.
///
/// The workflow's runner is such a tree: it asks this dry run which casefiles
/// a push reaches and configures CMake only when one of them says it needs
/// it. So the answer has to be available *before* the tree exists, and the
/// only way to assert that is to ask from somewhere the tree does not.
/// A copy under a fresh index rather than the checkout itself: the gate
/// enumerates its corpus with `git ls-files`, so the tree it runs in has to
/// be a repository, and the one thing this fixture must not have is the
/// `build/` the real checkout carries.
fn selection_without_a_cmake_tree(changed: &[&str]) -> (bool, BTreeSet<String>, String) {
    let dir = tempdir().expect("tempdir");
    for entry in ["scripts", "sce-build/tests/mutations"] {
        let dest = dir.path().join(entry);
        fs::create_dir_all(dest.parent().expect("a parent")).expect("create the fixture tree");
        copy_tree(&repo_root().join(entry), &dest);
    }
    for args in [vec!["init", "-q"], vec!["add", "-A"]] {
        let out = Command::new("git")
            .args(&args)
            .current_dir(dir.path())
            .output()
            .expect("prepare the fixture index");
        assert!(out.status.success(), "git {args:?} failed in the fixture");
    }
    assert!(
        !dir.path().join("build/CMakeCache.txt").exists(),
        "this tree was supposed to be the one without a CMake cache"
    );
    let changed_file = dir.path().join("changed.txt");
    fs::write(&changed_file, changed.join("\n") + "\n").expect("write change set");

    let out = Command::new("bash")
        .arg("scripts/gates/mutation-rounds.sh")
        .current_dir(dir.path())
        .env("SCE_GATE_CHANGED_FILE", &changed_file)
        .env("SCE_MUTATION_ROUNDS_DRY_RUN", "1")
        .env_remove("SCE_MUTATION_ROUNDS")
        .output()
        .expect("run the gate");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// Run the gate in dry-run mode over a change set, and return what it chose.
///
/// `SCE_GATE_CHANGED_FILE` is the same channel `scripts/gate` fills from
/// `--changed-from`, so this drives the gate through the interface the push
/// hook uses rather than through one built for the test.
fn selection_for(changed: &[&str]) -> (bool, BTreeSet<String>, String) {
    let dir = tempdir().expect("tempdir");
    let changed_file = dir.path().join("changed.txt");
    fs::write(&changed_file, changed.join("\n") + "\n").expect("write change set");

    let out = Command::new("bash")
        .arg("scripts/gates/mutation-rounds.sh")
        .current_dir(repo_root())
        .env("SCE_GATE_CHANGED_FILE", &changed_file)
        .env("SCE_MUTATION_ROUNDS_DRY_RUN", "1")
        // Cleared rather than left inherited: a developer running the suite
        // with a subset pinned in their environment would otherwise get that
        // subset here and a green test about nothing.
        .env_remove("SCE_MUTATION_ROUNDS")
        .output()
        .expect("run the gate");

    let chosen = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    (
        out.status.success(),
        chosen,
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// Every tracked casefile, and the targets it declares, asked of the harness
/// that owns the casefile vocabulary.
fn declared_targets(casefile: &str) -> Vec<String> {
    let out = Command::new("scripts/mutate")
        .args(["--declares", casefile])
        .current_dir(repo_root())
        .output()
        .expect("run scripts/mutate --declares");
    assert!(
        out.status.success(),
        "`scripts/mutate --declares {casefile}` failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("target\t"))
        .map(str::to_string)
        .collect()
}

/// Copy a file or directory recursively, preserving the executable bit —
/// `scripts/gates/*.sh` and `scripts/mutate` are run, not read.
fn copy_tree(from: &Path, to: &Path) {
    if from.is_dir() {
        fs::create_dir_all(to).expect("create directory");
        for entry in fs::read_dir(from).expect("read directory") {
            let entry = entry.expect("dir entry");
            copy_tree(&entry.path(), &to.join(entry.file_name()));
        }
    } else {
        fs::copy(from, to).expect("copy file");
    }
}

/// Whether a casefile drives its round through ctest — the same line the
/// workflow reads to decide whether to configure CMake.
fn declares_ctest(casefile: &str) -> bool {
    let out = Command::new("scripts/mutate")
        .args(["--declares", casefile])
        .current_dir(repo_root())
        .output()
        .expect("run scripts/mutate --declares");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .any(|line| line == "runner\tctest")
}

fn casefiles() -> Vec<String> {
    let out = Command::new("git")
        .args(["ls-files", "sce-build/tests/mutations/*.cases"])
        .current_dir(repo_root())
        .output()
        .expect("git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

/// A change that touches no declared target selects nothing, and says so.
///
/// The important half is the exit status: choosing nothing is a legitimate
/// outcome on most pushes, so it has to be a pass. The other half is that it
/// is a pass which SAYS what it did — a gate silent about having run nothing
/// is indistinguishable from one that ran everything.
#[test]
fn a_change_touching_no_declared_target_selects_nothing() {
    let (ok, chosen, log) = selection_for(&["README.md", "docs/nothing/at/all.md"]);

    assert!(ok, "selecting nothing must pass; the gate said:\n{log}");
    assert!(
        chosen.is_empty(),
        "no declared target was touched, so no round is owed: {chosen:?}"
    );
    assert!(
        log.contains("0 of "),
        "⚠ the gate must report the count it chose, including zero. Silence \
         about having examined nothing is what lets a gate read as a pass it \
         never earned. Log was:\n{log}"
    );
}

/// A change that touches a declared target selects that casefile, and only
/// the casefiles that declare the touched path.
///
/// Driven off the corpus rather than a fixture: the property is about the
/// real declarations, and a fixture casefile would assert that the gate can
/// match a string the test itself wrote on both sides.
#[test]
fn a_change_to_a_declared_target_selects_exactly_its_casefiles() {
    let corpus = casefiles();
    assert!(
        corpus.len() >= 20,
        "the sweep found only {} casefile(s), so this test is not measuring \
         the corpus it claims to",
        corpus.len()
    );

    // A target declared by exactly one casefile makes the "and only" half
    // observable. Picking it by measurement rather than by name keeps the
    // test from going stale when a casefile is renamed.
    let mut owners: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for casefile in &corpus {
        for target in declared_targets(casefile) {
            owners.entry(target).or_default().push(casefile.clone());
        }
    }
    let (target, owner) = owners
        .iter()
        .find(|(_, files)| files.len() == 1)
        .map(|(t, files)| (t.clone(), files[0].clone()))
        .expect("some declared target belongs to exactly one casefile");

    let (ok, chosen, log) = selection_for(&[&target]);

    assert!(ok, "the gate failed while choosing:\n{log}");
    assert_eq!(
        chosen,
        BTreeSet::from([owner.clone()]),
        "⚠ touching `{target}` must select `{owner}` and nothing else. Selecting \
         more turns every push into a mutation sweep; selecting less leaves the \
         case that guards `{target}` unverified on the one push that could have \
         broken it."
    );
}

/// A path that merely looks like a declared target does not select it.
///
/// A dry run answers even when what it selects would need a CMake tree.
///
/// This is the case the workflow asks about and the one the gate used to
/// refuse: the "no configured tree" precondition ran ahead of the dry-run
/// exit, so selecting a ctest casefile — the answer that means *build one* —
/// exited 3 before printing it. Measured on the first push to touch a C11
/// template: the selection step went red for having been asked.
///
/// Driven from a real ctest casefile's declared target rather than a written
/// path, so a corpus that stops declaring one fails here instead of passing
/// against a shape nothing has.
#[test]
fn a_dry_run_answers_for_a_casefile_that_would_need_a_cmake_tree() {
    let ctest_casefiles: Vec<String> = casefiles()
        .into_iter()
        .filter(|casefile| declares_ctest(casefile))
        .collect();
    assert!(
        !ctest_casefiles.is_empty(),
        "the corpus declares no ctest casefile, so this test asserts nothing"
    );

    for casefile in ctest_casefiles {
        let targets = declared_targets(&casefile);
        let target = targets
            .first()
            .unwrap_or_else(|| panic!("{casefile} declares no target"));
        let (ok, chosen, log) = selection_without_a_cmake_tree(&[target]);
        assert!(
            ok,
            "the gate refused to *choose* for want of a tree it would only \
             need in order to *run*:\n{log}"
        );
        assert!(
            chosen.contains(&casefile),
            "{target} did not select {casefile}: {chosen:?}\n{log}"
        );
    }
}

/// The intersection is exact because both sides are repository-relative paths
/// written by the same tooling. A prefix or substring match would pull in
/// every casefile under a directory somebody touched, which is how a
/// change-scoped gate quietly becomes an unconditional one.
#[test]
fn a_path_that_only_resembles_a_declared_target_selects_nothing() {
    let corpus = casefiles();
    let target = corpus
        .iter()
        .flat_map(|casefile| declared_targets(casefile))
        .next()
        .expect("the corpus declares at least one target");

    let parent = Path::new(&target)
        .parent()
        .expect("a declared target has a parent directory")
        .to_string_lossy()
        .to_string();
    let neighbour = format!("{target}.orig");

    let (ok, chosen, log) = selection_for(&[&parent, &neighbour]);

    assert!(ok, "the gate failed while choosing:\n{log}");
    assert!(
        chosen.is_empty(),
        "⚠ `{parent}` and `{neighbour}` are not `{target}`, and matching them \
         would make one edit anywhere under a directory pull in every casefile \
         that declares a file there: {chosen:?}"
    );
}
