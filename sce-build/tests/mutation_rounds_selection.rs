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
//
// One of these runs the lane's own selection step rather than the gate — see
// `the_lane_configures_a_cmake_tree_when_the_selection_needs_one`. The gate
// answering correctly is only half of the property, and the half that was
// already true when the lane had been wrong for 34 commits.

use std::collections::{BTreeMap, BTreeSet};
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
fn selection_without_a_cmake_tree(changed: &[&str]) -> (bool, BTreeMap<String, String>, String) {
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
        report(&String::from_utf8_lossy(&out.stdout)),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// Parse the gate's dry-run report: one `casefile<TAB>runner` line per
/// selected casefile.
///
/// The runner shares the line because the caller has to act on it before the
/// rounds — the workflow installs CMake and configures a tree only when the
/// selection contains a ctest round — and the one alternative to reading it
/// here is deriving it a second time somewhere else. That second derivation
/// existed in the lane's shell and was wrong on every push that needed it.
///
/// Split rather than tolerated: a line without the column is a report this
/// test cannot answer for, and accepting it would make every assertion below
/// hold vacuously the day the column is dropped.
fn report(stdout: &str) -> BTreeMap<String, String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            let (casefile, runner) = line.split_once('\t').unwrap_or_else(|| {
                panic!(
                    "⚠ every dry-run line must be `casefile<TAB>runner`; the workflow reads \
                     the second column to decide whether to configure a CMake tree. Got: {line:?}"
                )
            });
            (casefile.to_string(), runner.to_string())
        })
        .collect()
}

/// Run the gate in dry-run mode over a change set, and return what it chose.
///
/// `SCE_GATE_CHANGED_FILE` is the same channel `scripts/gate` fills from
/// `--changed-from`, so this drives the gate through the interface the push
/// hook uses rather than through one built for the test.
fn selection_for(changed: &[&str]) -> (bool, BTreeMap<String, String>, String) {
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

    (
        out.status.success(),
        report(&String::from_utf8_lossy(&out.stdout)),
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
        chosen.keys().cloned().collect::<BTreeSet<_>>(),
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
        assert_eq!(
            chosen.get(&casefile).map(String::as_str),
            Some("ctest"),
            "⚠ {target} must select {casefile} AND report that its round runs \
             through ctest. Selecting it without saying so leaves the lane to \
             work the runner out for itself, which is the derivation that was \
             wrong for 34 commits. Chose: {chosen:?}\n{log}"
        );
    }
}

/// The indentation width of a line, in characters.
fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The `run: |` body of the workflow step carrying the given `id:`, dedented
/// so it can be executed the way the runner executes it.
///
/// Lifted rather than restated: a test that carried its own copy of the
/// lane's shell would keep passing while the lane it describes went wrong,
/// which is the exact failure this file now exists to prevent.
fn run_body(workflow: &str, step_id: &str) -> String {
    let lines: Vec<&str> = workflow.lines().collect();
    let id_at = lines
        .iter()
        .position(|line| line.trim() == format!("id: {step_id}"))
        .unwrap_or_else(|| panic!("the workflow has no step with `id: {step_id}`"));
    let run_at = lines[id_at..]
        .iter()
        .position(|line| line.trim() == "run: |")
        .map(|offset| id_at + offset)
        .unwrap_or_else(|| panic!("step `{step_id}` has no `run: |` block"));

    let outer = indent_of(lines[run_at]);
    let body: Vec<&str> = lines[run_at + 1..]
        .iter()
        .take_while(|line| line.trim().is_empty() || indent_of(line) > outer)
        .copied()
        .collect();
    let pad = body
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| indent_of(line))
        .min()
        .expect("the block is not empty");
    body.iter()
        .map(|line| line.get(pad..).unwrap_or("").to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `if:` condition of the workflow step carrying the given `name:`.
fn step_condition(workflow: &str, step_name: &str) -> String {
    let lines: Vec<&str> = workflow.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.trim() == format!("- name: {step_name}"))
        .unwrap_or_else(|| panic!("the workflow has no step named {step_name:?}"));
    lines[at + 1..]
        .iter()
        .take_while(|line| !line.trim().starts_with("- name:"))
        .find_map(|line| line.trim().strip_prefix("if: "))
        .unwrap_or_else(|| panic!("step {step_name:?} declares no `if:` condition"))
        .to_string()
}

/// Run the workflow's selection step over a change set and return the outputs
/// it recorded, plus its combined log.
fn lane_selection(script: &str, changed: &str) -> (BTreeMap<String, String>, String) {
    let dir = tempdir().expect("tempdir");
    let changed_file = dir.path().join("changed.txt");
    fs::write(&changed_file, format!("{changed}\n")).expect("write change set");
    let step = dir.path().join("step.sh");
    fs::write(&step, script).expect("write the step script");
    let github_output = dir.path().join("github_output");
    fs::write(&github_output, "").expect("create the outputs file");
    let runner_temp = dir.path().join("runner_temp");
    fs::create_dir(&runner_temp).expect("create the runner temp directory");

    // `bash -e <file>` is how the runner invokes a `run:` block
    // (`shell: /usr/bin/bash -e {0}`), and the difference matters: a step
    // whose script only works when sourced by an interactive shell is not the
    // step CI runs.
    let out = Command::new("bash")
        .arg("-e")
        .arg(&step)
        .current_dir(repo_root())
        // The step-level `env:` of that job, supplied here because the test
        // executes the body rather than the YAML around it.
        .env("SCE_GATE_CHANGED_FILE", &changed_file)
        .env("GITHUB_OUTPUT", &github_output)
        .env("RUNNER_TEMP", &runner_temp)
        .env_remove("SCE_MUTATION_ROUNDS")
        .env_remove("SCE_MUTATION_ROUNDS_DRY_RUN")
        .output()
        .expect("run the workflow's selection step");

    let log = format!(
        "--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "the lane's selection step failed for change set `{changed}`:\n{log}"
    );
    let recorded = fs::read_to_string(&github_output).expect("read the outputs file");
    let outputs = recorded
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    (outputs, log)
}

/// The lane configures a CMake tree exactly when the selection needs one.
///
/// Every other test in this file asks the gate. This one asks the *lane*, and
/// that distinction is the finding it records rather than a stylistic one.
///
/// The gate was never wrong here: it reads every casefile's runner in order to
/// refuse a ctest round without a configured tree. What was wrong was the
/// lane's own second derivation of that same fact,
///
/// ```text
/// scripts/mutate --declares "$casefile" | grep -qx "runner<TAB>ctest"
/// ```
///
/// which under `set -o pipefail` reports the pipeline's last failing member:
/// `grep -q` leaves at the first match, the writer takes SIGPIPE, and the
/// status is 141. Measured 20/20 on this repository — deterministic, not a
/// race. So the lane answered "no tree needed" precisely when one was, skipped
/// its configure step, and the rounds refused for the want of the tree the
/// step had just declined to build. `ai_loop_history_cpp`,
/// `c11_datamodel_reader` and the three `parallel_*` casefiles went 34 commits
/// without one CI round because of it — including
/// `parallel_microstep_owns_exit_and_entry`, the case whose silent
/// INCONCLUSIVE is the reason this gate was written.
///
/// The oracle asks `scripts/mutate --declares`, which is where a runner is
/// declared. Asking the gate would be asking the thing under test to grade
/// itself.
#[test]
fn the_lane_configures_a_cmake_tree_when_the_selection_needs_one() {
    let workflow = fs::read_to_string(repo_root().join(".github/workflows/mutation-rounds.yml"))
        .expect("read the mutation-rounds workflow");
    let script = run_body(&workflow, "select");

    assert!(
        script.contains("scripts/gate mutation-rounds"),
        "⚠ the selection step no longer runs the gate, so this test would be \
         asserting over a script that decides nothing:\n{script}"
    );
    assert!(
        !script.contains("${{"),
        "⚠ the selection step now interpolates a workflow expression, which \
         bash sees literally — this test would be running a different string \
         than CI does. Move the expression into the step's `env:`:\n{script}"
    );
    assert_eq!(
        step_condition(&workflow, "Configure and build the CMake tree").trim(),
        "steps.select.outputs.count != '0' && steps.select.outputs.needs_ctest == 'yes'",
        "⚠ the step that builds the tree must key off the output the selection \
         step records. A verdict nothing reads is the same silence as no \
         verdict at all."
    );

    // Which runner each casefile drives its round through, from the
    // declaration rather than from the gate.
    let corpus = casefiles();
    let mut ctest_target = None;
    let mut cargo_target = None;
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for casefile in &corpus {
        for target in declared_targets(casefile) {
            owners.entry(target).or_default().push(casefile.clone());
        }
    }
    for (target, casefiles) in &owners {
        if casefiles.iter().any(|casefile| declares_ctest(casefile)) {
            ctest_target.get_or_insert_with(|| target.clone());
        } else {
            cargo_target.get_or_insert_with(|| target.clone());
        }
    }
    let ctest_target = ctest_target.expect(
        "the corpus declares no ctest casefile, so this test cannot observe the \
         answer it exists to assert",
    );
    let cargo_target = cargo_target
        .expect("the corpus declares no cargo-only target, so the negative half is vacuous");

    let (chose_ctest, ctest_log) = lane_selection(&script, &ctest_target);
    assert_eq!(
        chose_ctest.get("needs_ctest").map(String::as_str),
        Some("yes"),
        "⚠ touching `{ctest_target}` selects a casefile whose round runs through \
         ctest, so the lane must configure a CMake tree. Answering `no` here is \
         not a skipped round — the rounds step then refuses (exit 3) for the \
         want of that tree and the lane goes red.\n{ctest_log}"
    );
    assert_ne!(
        chose_ctest.get("count").map(String::as_str),
        Some("0"),
        "the ctest change set selected nothing, so the answer above says \
         nothing:\n{ctest_log}"
    );

    let (chose_cargo, cargo_log) = lane_selection(&script, &cargo_target);
    assert_eq!(
        chose_cargo.get("needs_ctest").map(String::as_str),
        Some("no"),
        "⚠ touching `{cargo_target}` selects only cargo rounds, and paying for a \
         CMake configure-and-build on those pushes is what makes an unfiltered \
         workflow expensive enough to be switched off.\n{cargo_log}"
    );
    assert_ne!(
        chose_cargo.get("count").map(String::as_str),
        Some("0"),
        "the cargo change set selected nothing, so the answer above says \
         nothing:\n{cargo_log}"
    );
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
