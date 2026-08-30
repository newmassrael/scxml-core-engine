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
// Two of these run the lane's own shell rather than the gate — see
// `the_lane_configures_a_cmake_tree_when_the_selection_needs_one` and
// `the_lane_prepares_the_tree_the_rounds_judge`. The gate answering correctly
// is only half of the property, and the half that was already true when the
// lane had been wrong for 34 commits.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

mod common;
use common::gate_selectors::gate_shell;

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

    let out = gate_shell()
        .arg("scripts/gates/mutation-rounds.sh")
        .current_dir(dir.path())
        .env("SCE_GATE_CHANGED_FILE", &changed_file)
        .env("SCE_MUTATION_ROUNDS_DRY_RUN", "1")
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
            let mut columns = line.split('\t');
            let (casefile, runner, shards) = (columns.next(), columns.next(), columns.next());
            let (Some(casefile), Some(runner), Some(shards)) = (casefile, runner, shards) else {
                panic!(
                    "⚠ every dry-run line must be `casefile<TAB>runner<TAB>shards`; the \
                     workflow reads the second column to decide whether to configure a \
                     CMake tree and the third to decide how many jobs the casefile is \
                     worth. Got: {line:?}"
                )
            };
            // A shard count is what stops a round reaching the lane's ceiling,
            // so it has to be a number a job can be expanded from. Zero is the
            // dangerous spelling: `for (shard = 1; shard <= 0; ...)` emits no
            // job at all, and a casefile that quietly stops being run is the
            // absent verdict this whole lane exists to end.
            let count: usize = shards.parse().unwrap_or_else(|_| {
                panic!("⚠ the shard column of {casefile:?} is not a number: {shards:?}")
            });
            assert!(
                count >= 1,
                "⚠ {casefile} is worth {count} job(s), so the matrix would expand it \
                 into nothing and the lane would report green having run no round on it"
            );
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

    // Every selector the gate reads is cleared by `gate_shell`, so what
    // follows is this scenario's whole input rather than its input plus
    // whatever the caller's environment happened to hold.
    let out = gate_shell()
        .arg("scripts/gates/mutation-rounds.sh")
        .current_dir(repo_root())
        .env("SCE_GATE_CHANGED_FILE", &changed_file)
        .env("SCE_MUTATION_ROUNDS_DRY_RUN", "1")
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

/// The oracles a casefile resolves: the test whose assertions produced its
/// verdicts, plus the casefile itself.
///
/// The other half of what retires a verdict. `declared_targets` above answers
/// "what does a case break"; this answers "what noticed" — and a change to the
/// noticing side invalidates the last verdict exactly as a change to the broken
/// side does. Asked of the harness rather than derived here, for the reason the
/// harness's own header gives: a second reader of the casefile vocabulary is a
/// copy, and the copy is what goes stale.
fn declared_oracles(casefile: &str) -> Vec<String> {
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
        .filter_map(|line| line.strip_prefix("oracle\t"))
        .map(str::to_string)
        .collect()
}

/// Every path git tracks, for asking whether a declaration names a real file
/// rather than one somebody moved.
fn tracked_files() -> BTreeSet<String> {
    let out = Command::new("git")
        .args(["ls-files"])
        .current_dir(repo_root())
        .output()
        .expect("git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    String::from_utf8_lossy(&out.stdout)
        .lines()
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

/// The services a casefile says its round needs running, asked of the harness
/// that owns the vocabulary.
fn declared_needs(casefile: &str) -> Vec<String> {
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
        .filter_map(|line| line.strip_prefix("needs\t"))
        .map(str::to_string)
        .collect()
}

/// Whether a casefile drives its round through ctest — the same line the
/// workflow reads to decide whether to configure CMake.
fn declares_ctest(casefile: &str) -> bool {
    declared_runner(casefile) == "ctest"
}

/// The runner a casefile declares, read from the harness that owns the
/// casefile vocabulary rather than by matching its text.
fn declared_runner(casefile: &str) -> String {
    let out = Command::new("scripts/mutate")
        .args(["--declares", casefile])
        .current_dir(repo_root())
        .output()
        .expect("run scripts/mutate --declares");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("runner\t").map(str::to_string))
        .unwrap_or_else(|| panic!("{casefile} declares no runner"))
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

/// Every runner the corpus declares is provisioned in the job that runs it.
///
/// The rounds job is one job per casefile and it installs by CONDITION —
/// `matrix.runner == 'ctest'` builds a CMake tree, and a cargo round pays for
/// none of it. That shape is right and it is also the shape of the defect this
/// repository has already paid for twice: a lane that did not install what a
/// round needed reported `baseline is not green` — a true sentence about a
/// missing tool, phrased as a defect in the tree — for every run the casefile
/// ever had. The rev-pinned validator cost two casefiles their rounds that way
/// (2026-08-25), and the same silence is one `mutation_go` away from
/// happening again with a toolchain nobody thought to add.
///
/// So this derives rather than lists: it reads the runner off every casefile
/// through `scripts/mutate --declares`, and asks the workflow whether some
/// step keys off it. Add a casefile in a new language and this fails until the
/// lane can run it — which is the moment it is cheapest to notice, rather than
/// after a round has reported a red baseline about a missing interpreter.
///
/// `cargo` is exempt and is the only exemption: the Rust toolchain is
/// installed unconditionally, because every job in this lane needs it to build
/// `sce-codegen`.
#[test]
fn every_runner_the_corpus_declares_is_provisioned_in_the_rounds_job() {
    let workflow = fs::read_to_string(repo_root().join(".github/workflows/mutation-rounds.yml"))
        .expect("read the mutation-rounds workflow");

    let corpus = casefiles();
    assert!(
        corpus.len() >= 5,
        "⚠ the corpus reports {} casefile(s); this test derives its answer from \
         them, and an empty sweep would pass by having nothing to check",
        corpus.len()
    );

    let mut runners: BTreeSet<String> = corpus.iter().map(|c| declared_runner(c)).collect();
    runners.remove("cargo");

    assert!(
        runners.len() >= 4,
        "⚠ the corpus declares {} non-cargo runner(s): {runners:?}. Four are \
         expected — ctest, go, gradle and pytest — and a smaller set means \
         either a runner was dropped or this derivation stopped reading them, \
         which would make the sweep below vacuous",
        runners.len()
    );

    for runner in &runners {
        let wanted = format!("matrix.runner == '{runner}'");
        assert!(
            workflow.contains(&wanted),
            "⚠ the corpus declares a `{runner}` runner and no step in \
             .github/workflows/mutation-rounds.yml is conditioned on \
             `{wanted}`. A round whose toolchain the job never installed does \
             not report a missing tool — it reports the baseline it could not \
             run, as though the tree were broken."
        );
    }
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
    let out = gate_shell()
        .arg("-e")
        .arg(&step)
        .current_dir(repo_root())
        // The step-level `env:` of that job, supplied here because the test
        // executes the body rather than the YAML around it. What the job does
        // NOT set is cleared by `gate_shell`, which is the same list this step
        // would inherit on a real runner.
        .env("SCE_GATE_CHANGED_FILE", &changed_file)
        .env("GITHUB_OUTPUT", &github_output)
        .env("RUNNER_TEMP", &runner_temp)
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
        "matrix.runner == 'ctest'",
        "⚠ the step that builds the tree must key off the runner the selection \
         step recorded for THIS casefile. A verdict nothing reads is the same \
         silence as no verdict at all, and a lane-wide answer would make every \
         cargo casefile in a mixed selection pay for a tree it never opens."
    );
    assert_eq!(
        run_body_named(&workflow, "Run the round").trim(),
        "scripts/gate mutation-rounds",
        "⚠ the round must be the gate, not a recipe restated in the lane."
    );
    assert!(
        workflow.contains("SCE_MUTATION_ROUNDS: ${{ matrix.casefile }}"),
        "⚠ the round must run the casefile the matrix named, through the gate's \
         own subset channel. Any second derivation here is the shape of defect \
         that had this lane answering for a different set than it selected."
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
    let ctest_matrix = matrix_of(&chose_ctest, &ctest_log);
    assert!(
        ctest_matrix.iter().any(|(_, runner)| runner == "ctest"),
        "⚠ touching `{ctest_target}` selects a casefile whose round runs through \
         ctest, so its job must configure a CMake tree. A matrix that says \
         `cargo` here does not skip a round — the round then refuses (exit 3) \
         for the want of that tree and the lane goes red.\n{ctest_log}"
    );
    assert_ne!(
        chose_ctest.get("count").map(String::as_str),
        Some("0"),
        "the ctest change set selected nothing, so the answer above says \
         nothing:\n{ctest_log}"
    );

    let (chose_cargo, cargo_log) = lane_selection(&script, &cargo_target);
    let cargo_matrix = matrix_of(&chose_cargo, &cargo_log);
    assert!(
        cargo_matrix.iter().all(|(_, runner)| runner != "ctest"),
        "⚠ touching `{cargo_target}` selects only cargo rounds, and paying for a \
         CMake configure-and-build on those jobs is what makes an unfiltered \
         workflow expensive enough to be switched off.\n{cargo_log}"
    );
    assert_ne!(
        chose_cargo.get("count").map(String::as_str),
        Some("0"),
        "the cargo change set selected nothing, so the answer above says \
         nothing:\n{cargo_log}"
    );
}

/// The `matrix` output, read the way `fromJSON` in the workflow reads it.
///
/// Parsed rather than pattern-matched: the value is expanded by the runner
/// into `strategy.matrix.include`, and a string that merely *looks* like an
/// array fails there — at which point the lane reports a configuration error
/// and every round it was going to run silently does not happen. So the test
/// requires the same thing the runner does, on output the step really wrote.
fn matrix_of(outputs: &BTreeMap<String, String>, log: &str) -> Vec<(String, String)> {
    let raw = outputs
        .get("matrix")
        .unwrap_or_else(|| panic!("⚠ the selection step recorded no `matrix` output:\n{log}"));
    let parsed: serde_json::Value = serde_json::from_str(raw).unwrap_or_else(|error| {
        panic!("⚠ `matrix` is not JSON the runner could expand ({error}): {raw}\n{log}")
    });
    let entries = parsed
        .get("include")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| {
            panic!("⚠ `matrix` must carry an `include` array, which is the shape the runner expands a generated matrix from: {raw}\n{log}")
        });

    let tracked = casefiles().into_iter().collect::<BTreeSet<_>>();
    // Every shard each casefile was expanded into, so the check below can ask
    // whether the slices actually cover the file. A matrix that emitted
    // `1/6 … 5/6` would look entirely healthy — six jobs, all green — while
    // the sixth slice's cases were never measured by anything.
    let mut shards_of: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    let rounds = entries
        .iter()
        .map(|entry| {
            let casefile = entry
                .get("casefile")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| panic!("⚠ a matrix entry names no casefile: {entry}\n{log}"))
                .to_string();
            assert!(
                tracked.contains(&casefile),
                "⚠ the matrix names `{casefile}`, which is not a casefile in the \
                 corpus — a job that would fail on its own argument.\n{log}"
            );
            let runner = entry
                .get("runner")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| {
                    panic!("⚠ a matrix entry carries no runner, so its job cannot know whether to build a tree: {entry}\n{log}")
                })
                .to_string();
            let shard = entry
                .get("shard")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_else(|| {
                    panic!("⚠ a matrix entry carries no shard, so its job would run the whole casefile — the shape whose rounds reached the lane's 330-minute ceiling and were cancelled: {entry}\n{log}")
                });
            let (index, of) = shard.split_once('/').unwrap_or_else(|| {
                panic!("⚠ a matrix entry's shard is not `I/N`, which is what `scripts/mutate --shard` refuses: {shard}\n{log}")
            });
            let parse = |what: &str, raw: &str| -> usize {
                raw.parse().unwrap_or_else(|_| {
                    panic!("⚠ a matrix entry's shard {what} is not a number: {shard}\n{log}")
                })
            };
            shards_of
                .entry(casefile.clone())
                .or_default()
                .push((parse("index", index), parse("count", of)));
            (casefile, runner)
        })
        .collect::<Vec<_>>();

    // The slices have to cover the casefile exactly: every index from 1 to N
    // once, all agreeing on N. `scripts/mutate --shard` refuses an index
    // outside its casefile, so a duplicate or a gap does not go red on the
    // runner — it goes green having measured the same cases twice and some
    // cases never.
    for (casefile, mut shards) in shards_of.clone() {
        shards.sort_unstable();
        let of = shards[0].1;
        let expected: Vec<(usize, usize)> = (1..=of).map(|index| (index, of)).collect();
        assert_eq!(
            shards, expected,
            "⚠ the shards of `{casefile}` do not cover it: the matrix expanded \
             {shards:?} where {of} slice(s) numbered 1..{of} are what \
             `--shard` partitions the cases into. A missing index is a set of \
             cases no job runs, reported as a lane that passed.\n{log}"
        );
    }

    let count: usize = outputs
        .get("count")
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("⚠ the selection step recorded no usable `count`:\n{log}"));
    assert_eq!(
        shards_of.len(),
        count,
        "⚠ the matrix and the count answer for different sets: {count} casefile(s) \
         selected, {} appear in the matrix. One of them is what a reader \
         believes. `count` stays a count of CASEFILES — it is what decides \
         whether the rounds job runs at all — while the matrix is one entry \
         per JOB, which is a shard.\n{log}",
        shards_of.len()
    );

    rounds
}

/// The `run:` body of the workflow step carrying the given `name:`, whether it
/// is a `run: |` block or a single command on the `run:` line itself.
///
/// Both forms are executed here, because both are forms the runner executes.
/// A test that only understood the block would silently stop asserting the day
/// a step shrank to one line — which is the shape a step has once it stops
/// carrying a recipe of its own, and therefore exactly the shape this file
/// wants to keep.
fn run_body_named(workflow: &str, step_name: &str) -> String {
    let lines: Vec<&str> = workflow.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.trim() == format!("- name: {step_name}"))
        .unwrap_or_else(|| panic!("the workflow has no step named {step_name:?}"));
    let run_at = lines[at + 1..]
        .iter()
        .take_while(|line| !line.trim().starts_with("- name:"))
        .position(|line| line.trim().starts_with("run:"))
        .map(|offset| at + 1 + offset)
        .unwrap_or_else(|| panic!("step {step_name:?} has no `run:`"));

    let inline = lines[run_at]
        .trim()
        .strip_prefix("run:")
        .expect("a run: line")
        .trim();
    if inline != "|" {
        return inline.to_string();
    }

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

/// Every casefile names the test that caught it, and the name resolves.
///
/// A casefile declares a SELECTOR — `--test datamodel_read_accessor`, `-R
/// '^IntegrationTests$'` — and a selector is not a path, so nothing about it
/// tells the gate which file to watch. Until this existed the answer was
/// nothing: 19 of the 30 casefiles had no path on the oracle side at all, and
/// the remaining 11 were covered by the accident that their cases mutate their
/// own test file.
///
/// Two claims, and the second is what keeps the first from being decorative:
/// every casefile resolves at least one oracle beyond itself, and every path it
/// names is tracked. An oracle naming a file somebody moved is a watch on
/// nothing, and it would read exactly like a watch that works.
#[test]
fn every_casefile_names_the_test_that_catches_it() {
    let corpus = casefiles();
    assert!(
        corpus.len() >= 20,
        "the sweep found only {} casefile(s), so this test is not measuring the \
         corpus it claims to",
        corpus.len()
    );
    let tracked = tracked_files();

    for casefile in &corpus {
        let oracles = declared_oracles(casefile);
        assert!(
            oracles.contains(casefile),
            "⚠ {casefile} does not name ITSELF as an oracle. Its text decides \
             which edit is applied and where, so a change to it retires the \
             verdict as surely as a change to the source under study — and a \
             casefile edit selecting no round is measured, not hypothetical. \
             Got: {oracles:?}"
        );
        let tests: Vec<&String> = oracles.iter().filter(|path| *path != casefile).collect();
        assert!(
            !tests.is_empty(),
            "⚠ {casefile} names no test-side oracle. Weaken the assertion that \
             catches its cases and every one of them keeps the verdict it last \
             earned. A cargo selector resolves this from `--test`; a ctest one \
             cannot, and declares `mutation_oracles` instead — measured from \
             which tests the round actually reds, not guessed."
        );
        for path in tests {
            assert!(
                tracked.contains(path.as_str()),
                "⚠ {casefile} names the oracle `{path}`, which git does not \
                 track. A path that moved makes this a watch on nothing."
            );
        }
    }
}

/// A change to the test that catches a case selects that case's round.
///
/// The property K2 named and the corpus did not have. Driven off every
/// casefile's own declaration, so it covers the corpus rather than a sample,
/// and asserted in one gate invocation because the gate reads all thirty
/// declarations on every call.
#[test]
fn a_change_to_an_oracle_selects_the_round_it_belongs_to() {
    let corpus = casefiles();
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for casefile in &corpus {
        for oracle in declared_oracles(casefile) {
            owners.entry(oracle).or_default().push(casefile.clone());
        }
    }
    let every_oracle: Vec<&str> = owners.keys().map(String::as_str).collect();

    let (ok, chosen, log) = selection_for(&every_oracle);
    assert!(ok, "the gate failed while choosing:\n{log}");
    for casefile in &corpus {
        assert!(
            chosen.contains_key(casefile),
            "⚠ every oracle in the corpus was in the change set and {casefile} \
             was still not selected, so nothing re-proves its cases when the \
             test that catches them changes. Chose: {chosen:?}\n{log}"
        );
    }

    // And exactly, for an oracle that belongs to one casefile alone: selecting
    // everything on every change would be the other failure — a corpus sweep
    // on every push, which is the version of this gate that gets switched off.
    let (sole, owner) = owners
        .iter()
        .find(|(oracle, files)| files.len() == 1 && oracle.ends_with(".rs"))
        .map(|(oracle, files)| (oracle.clone(), files[0].clone()))
        .expect("some oracle belongs to exactly one casefile");
    let (ok, chosen, log) = selection_for(&[&sole]);
    assert!(ok, "the gate failed while choosing:\n{log}");
    assert_eq!(
        chosen.keys().cloned().collect::<BTreeSet<_>>(),
        BTreeSet::from([owner.clone()]),
        "⚠ `{sole}` is the oracle of `{owner}` and of nothing else, so it must \
         select that round and no other.\n{log}"
    );
}

/// The `env:` mapping of the workflow step carrying the given `name:`.
fn step_env(workflow: &str, step_name: &str) -> BTreeMap<String, String> {
    let lines: Vec<&str> = workflow.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.trim() == format!("- name: {step_name}"))
        .unwrap_or_else(|| panic!("the workflow has no step named {step_name:?}"));
    let step: Vec<&str> = lines[at + 1..]
        .iter()
        .take_while(|line| !line.trim().starts_with("- name:"))
        .copied()
        .collect();
    let env_at = match step.iter().position(|line| line.trim() == "env:") {
        Some(at) => at,
        None => return BTreeMap::new(),
    };
    let outer = indent_of(step[env_at]);
    step[env_at + 1..]
        .iter()
        .take_while(|line| line.trim().is_empty() || indent_of(line) > outer)
        .filter(|line| !line.trim().starts_with('#') && !line.trim().is_empty())
        .filter_map(|line| line.trim().split_once(": "))
        .map(|(key, value)| (key.to_string(), value.trim().to_string()))
        .collect()
}

/// The step that chooses and the step that runs are handed the same inputs.
///
/// They are two invocations of one gate, and every input that narrows the
/// corpus has to reach both: the first decides whether a CMake tree is
/// configured, the second decides which rounds execute. An input that reaches
/// only one of them makes the lane install for a set it will not run, or run a
/// set it never prepared for — and this lane has already paid for that exact
/// asymmetry once, when the selection answered "no tree needed" for a push
/// whose rounds then needed one.
///
/// That was asserted as equality between the two `env:` mappings while both
/// steps lived in one job and the rounds re-applied the selection themselves.
/// They no longer do: the casefiles are a matrix, one job each, and the
/// selection is applied once — in the job that computed it. So the property
/// is now that there is exactly ONE channel between them, and the round takes
/// its subset from that channel and from nothing else.
///
/// Which is the same invariant stated where it now lives. A round that also
/// read a change set would be narrowing a second time, on an input its job
/// does not even have (`changed.txt` is written in the selection's workspace),
/// and a round that read `inputs.casefiles` would be answering the dispatch
/// again after the selection already had.
#[test]
fn the_selection_narrows_and_the_round_only_obeys() {
    let workflow = fs::read_to_string(repo_root().join(".github/workflows/mutation-rounds.yml"))
        .expect("read the mutation-rounds workflow");

    let select = step_env(&workflow, "Which casefiles does this change reach");
    assert_eq!(
        select.get("SCE_GATE_CHANGED_FILE").map(String::as_str),
        Some("${{ steps.changed.outputs.file }}"),
        "⚠ the selection must be handed the push's range, or it answers for the \
         whole corpus on every push"
    );
    assert_eq!(
        select.get("SCE_MUTATION_ROUNDS").map(String::as_str),
        Some(
            "${{ (github.event_name == 'schedule' || github.event_name == 'workflow_dispatch') \
             && (inputs.casefiles != '' && inputs.casefiles || 'all') || '' }}"
        ),
        "⚠ the selection must be handed the dispatch's subset, or asking for one \
         casefile runs all of them — and an event that means everything must be \
         handed `all`, or the sweep selects nothing. See \
         `an_event_that_means_everything_is_handed_the_whole_corpus`."
    );

    let run = step_env(&workflow, "Run the round");
    assert_eq!(
        run,
        BTreeMap::from([
            (
                "SCE_MUTATION_ROUNDS".to_string(),
                "${{ matrix.casefile }}".to_string()
            ),
            (
                "SCE_MUTATION_SHARD".to_string(),
                "${{ matrix.shard }}".to_string()
            ),
        ]),
        "⚠ the round is handed something other than exactly what its matrix \
         entry names. Both keys are the same property: the selection decided \
         which casefile and how many slices, and the round obeys. A key here \
         reading anything but `matrix.` is a second narrowing, made in a job \
         that does not hold the inputs the first one used — which is the \
         asymmetry this lane already paid for once, in the other direction. A \
         MISSING `SCE_MUTATION_SHARD` is the same defect the other way: every \
         job would run the whole casefile, which is the shape whose rounds \
         reached the 330-minute ceiling and were cancelled."
    );
    for (key, value) in &run {
        assert!(
            value.starts_with("${{ matrix.") && value.ends_with(" }}"),
            "⚠ `{key}` is handed `{value}`, which does not come from the matrix \
             entry this job was scheduled as. The selection is the only place \
             the subset is decided."
        );
    }
}

/// Run the gate in dry-run mode with a named subset, and return what it chose.
///
/// The sibling of `selection_for`, through the gate's OTHER input: that one
/// hands it a change set, this one hands it `SCE_MUTATION_ROUNDS`. Both are
/// real channels the lane uses — a push fills the first, a dispatch and now a
/// scheduled run fill the second — so both are driven here rather than
/// asserted about.
fn selection_named(subset: &str) -> (bool, BTreeMap<String, String>, String) {
    // No change set at all, which is the state a scheduled run is in: the
    // workflow's `Resolve the change set` step writes no file for an event
    // that carries no range. `gate_shell` is what makes that absence real
    // rather than dependent on the caller's environment.
    let out = gate_shell()
        .arg("scripts/gates/mutation-rounds.sh")
        .current_dir(repo_root())
        .env("SCE_MUTATION_ROUNDS_DRY_RUN", "1")
        .env("SCE_MUTATION_ROUNDS", subset)
        .output()
        .expect("run the gate");

    (
        out.status.success(),
        report(&String::from_utf8_lossy(&out.stdout)),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// The two events that mean "everything" reach every casefile, and reach them
/// because they say so rather than by falling through.
///
/// Three things have to hold together, and the middle one is the one that is
/// easy to get wrong — twice, as it turned out:
///
///   1. the workflow actually has a `schedule:` trigger — routing without a
///      trigger is dead code;
///   2. `schedule` AND a `workflow_dispatch` with an empty input are handed
///      `all` EXPLICITLY. The tempting reading is that neither carries a
///      range, so the gate runs the corpus by itself. It does not. Handed no
///      change file the gate derives a range from the tracking ref, and at the
///      tip of `main` that range is empty: measured on this repository,
///      `0 of 84 casefile(s)` for both. A weekly lane that ran nothing would
///      PASS, which is the same absent-verdict-reading-as-green the trigger
///      exists to end — and the dispatch had been promising "Empty runs the
///      corpus" in its own input description while doing exactly that;
///   3. the value they are handed really does select the whole corpus, asked
///      of the gate rather than assumed of it.
///
/// The floor on (3) is the whole corpus and not a constant, so a casefile
/// added tomorrow is covered without editing this test — and the separate
/// assertion that the corpus is non-trivial is what stops an empty
/// `git ls-files` from making the comparison hold vacuously.
///
/// What is deliberately NOT asserted: what an EMPTY value selects. It reaches
/// the gate's tracking-ref derivation, whose answer depends on the checkout —
/// `0 of 84` here, the whole corpus on a clone with no upstream — so pinning
/// it would be pinning the environment rather than the routing.
#[test]
fn an_event_that_means_everything_is_handed_the_whole_corpus() {
    let workflow = fs::read_to_string(repo_root().join(".github/workflows/mutation-rounds.yml"))
        .expect("read the mutation-rounds workflow");

    // (1) The trigger exists, with a cron under it.
    let schedule_at = workflow
        .lines()
        .position(|line| line.trim() == "schedule:")
        .expect(
            "⚠ the workflow declares no `schedule:` trigger, so nothing ever runs the \
             corpus whole and a casefile whose targets go quiet is never judged again",
        );
    assert!(
        workflow
            .lines()
            .skip(schedule_at + 1)
            .take(3)
            .any(|line| line.trim().starts_with("- cron:")),
        "⚠ `schedule:` must carry a cron entry; a trigger with no schedule never fires"
    );

    // (2) The routing names the corpus for BOTH events that mean "everything",
    // and turns an empty subset into `all` rather than into silence.
    //
    // Three needles rather than one exact string, because the property is what
    // the expression decides and not how it is spelled: each names one event
    // that must be routed, and the third is the fallback an empty
    // `inputs.casefiles` lands on. A cosmetic rewrite keeps them; deleting a
    // branch does not.
    let select = step_env(&workflow, "Which casefiles does this change reach");
    let routed = select
        .get("SCE_MUTATION_ROUNDS")
        .map(String::as_str)
        .expect("the selection step must set SCE_MUTATION_ROUNDS");
    for needle in [
        "github.event_name == 'schedule'",
        "github.event_name == 'workflow_dispatch'",
        "|| 'all'",
    ] {
        assert!(
            routed.contains(needle),
            "⚠ an event that means the whole corpus must be handed `all` by \
             name, and {needle:?} is missing. Relying on the absent change set \
             is the trap both of these fell into: the gate reads an empty \
             `SCE_MUTATION_ROUNDS` as the ABSENCE of a request, derives a range \
             from the tracking ref instead, and answers `0 of 84`. Measured for \
             a scheduled run and for a dispatch with an empty input — the \
             second having promised \"Empty runs the corpus\" in its own \
             description the whole time. Got: {routed:?}"
        );
    }

    // (3) That value really does reach every casefile — asked, not assumed.
    let corpus = casefiles();
    assert!(
        corpus.len() > 1,
        "⚠ the corpus enumeration came back with {} entries; every comparison \
         below would hold vacuously",
        corpus.len()
    );
    let (ok, chosen, log) = selection_named("all");
    assert!(
        ok,
        "selecting the whole corpus must pass; the gate said:\n{log}"
    );
    let missing: Vec<&String> = corpus.iter().filter(|c| !chosen.contains_key(*c)).collect();
    assert!(
        missing.is_empty(),
        "⚠ `all` left {} of {} casefile(s) unselected, so the weekly sweep would \
         not reach them: {missing:?}",
        missing.len(),
        corpus.len()
    );
}

/// Run a shell body in a checkout that has neither a `target/` nor a `build/`,
/// with `cmake` and `cargo` replaced by shims that record their arguments, and
/// return the commands it ran.
///
/// The fixture is the environment CI has and a developer's machine does not.
/// Every workstation in this project carries a built generator under
/// `target/debug`, so a preparation that forgets to produce one still
/// configures here and fails only on a runner — which is precisely what
/// happened: the lane's ctest path was reached for the first time in 34
/// commits on 2026-08-17 and stopped at `SCEFindCodegen.cmake:55`, "sce-codegen
/// not found", before a single round ran.
///
/// The shims record rather than execute because what is under test is the
/// recipe, not CMake: a real configure-and-build is the half-hour this lane
/// pays in CI, and asserting over it here would buy nothing the recorded
/// argument lists do not already say.
fn prepared_by(body: &str) -> (Vec<String>, String) {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    copy_tree(&repo_root().join("scripts"), &root.join("scripts"));

    // `scripts/gates/lib.sh` resolves the repository root with git, so the
    // fixture has to be one. Nothing is committed: the preparation reads
    // files, not history.
    let init = Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .output()
        .expect("git init the fixture");
    assert!(init.status.success(), "git init failed in the fixture");

    let log = root.join("commands.log");
    let shims = root.join("shims");
    fs::create_dir(&shims).expect("create the shim directory");
    write_shim(
        &shims.join("cmake"),
        "printf 'cmake %s\\n' \"$*\" >> \"$SCE_SHIM_LOG\"\n",
    );
    // The cargo shim leaves a binary behind because the real one would, and
    // because the resolver asks again afterwards: a shim that only recorded
    // the build would make `sce_codegen_require` report "the build produced no
    // binary" and the preparation would fail for the fixture's reason rather
    // than the tree's.
    write_shim(
        &shims.join("cargo"),
        "printf 'cargo %s\\n' \"$*\" >> \"$SCE_SHIM_LOG\"\n\
         mkdir -p \"$SCE_SHIM_ROOT/target/debug\"\n\
         printf '#!/usr/bin/env bash\\nexit 0\\n' > \"$SCE_SHIM_ROOT/target/debug/sce-codegen\"\n\
         chmod +x \"$SCE_SHIM_ROOT/target/debug/sce-codegen\"\n",
    );

    let step = root.join("step.sh");
    fs::write(&step, body).expect("write the step script");

    // `bash -e <file>` is how the runner invokes a `run:` block
    // (`shell: /usr/bin/bash -e {0}`).
    //
    // Through `gate_shell` even though this step prepares a tree rather than
    // running the gate: it is a step of the SAME job, so on a real runner it
    // carries that job's selectors, and a step that starts calling the gate
    // tomorrow would otherwise reintroduce the leak silently.
    let out = gate_shell()
        .arg("-e")
        .arg(&step)
        .current_dir(root)
        .env(
            "PATH",
            format!(
                "{}:{}",
                shims.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("SCE_SHIM_LOG", &log)
        .env("SCE_SHIM_ROOT", root)
        // A developer with a build directory pinned in their environment would
        // otherwise have the fixture point at their tree.
        .env_remove("SCE_W3C_BUILD_DIR")
        .output()
        .expect("run the tree preparation");

    let report = format!(
        "--- stdout ---\n{}\n--- stderr ---\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success(),
        "the tree preparation failed in a checkout with no target/ and no \
         build/ — which is every runner:\n{report}"
    );
    let commands = fs::read_to_string(&log)
        .unwrap_or_default()
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    (commands, report)
}

fn write_shim(path: &Path, body: &str) {
    fs::write(path, format!("#!/usr/bin/env bash\n{body}")).expect("write a shim");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .expect("make the shim executable");
    }
}

/// A recorded `cmake` invocation that configures a tree rather than building
/// one.
fn is_configure(command: &str) -> bool {
    command.starts_with("cmake ") && !command.contains("--build")
}

/// The lane prepares the tree the rounds judge, by the same recipe the gates
/// judge it with.
///
/// Its sibling above asserts that the lane ASKS for a tree when the selection
/// needs one. This one asserts that what it then produces is a tree — and the
/// distinction is not academic: the lane got the asking right on 2026-08-17,
/// reached its own configure step for the first time in 34 commits, and went
/// red inside it.
///
/// Two independent defects were in that one step, and both are the same shape:
/// a recipe written out here instead of delegated.
///
///   * `cmake -S . -B build` with no generator built first. The configure
///     resolves `sce-codegen` and stops with a FATAL_ERROR without one
///     (`cmake/SCEFindCodegen.cmake`), so the step could not have worked on any
///     runner. Every sibling lane that configures this tree carries a "Build
///     sce-codegen" step; this one did not, and nothing said so because nothing
///     ran it.
///   * `-DCMAKE_BUILD_TYPE=Debug`, against the `RelWithDebInfo` every gate
///     requires — `sce_main_build_dir` refuses a tree configured any other way,
///     in as many words. The lane would have built a tree (with AddressSanitizer
///     on, per the configure's own log) that the gate it exists to serve would
///     then have rejected.
///
/// So the assertion is not "the step names a build type"; it is that the step's
/// configure IS the gates' configure, compared by running both. A recipe
/// restated here can drift from the one the gates use; a delegated one cannot.
#[test]
fn the_lane_prepares_the_tree_the_rounds_judge() {
    let workflow = fs::read_to_string(repo_root().join(".github/workflows/mutation-rounds.yml"))
        .expect("read the mutation-rounds workflow");
    let step = run_body_named(&workflow, "Configure and build the CMake tree");
    assert!(
        !step.contains("${{"),
        "⚠ the step interpolates a workflow expression, which bash sees \
         literally — this test would be running a different string than CI \
         does. Move the expression into the step's `env:`:\n{step}"
    );

    let (lane, lane_log) = prepared_by(&step);
    // What the gates do when they need this tree, run the same way. The
    // comparison is between two executions rather than against a written-down
    // expectation, so it keeps holding through a change to the configure and
    // fails the moment the lane stops making the same call.
    let (gate, gate_log) = prepared_by("source scripts/gates/lib.sh\nsce_main_build_dir\n");

    let first_cmake = lane
        .iter()
        .position(|command| command.starts_with("cmake "))
        .unwrap_or_else(|| {
            panic!("⚠ the step ran no cmake at all, so it prepares nothing:\n{lane_log}")
        });
    assert!(
        lane[..first_cmake]
            .iter()
            .any(|command| command.contains("--bin sce-codegen")),
        "⚠ the step configures before anything builds sce-codegen. A runner's \
         checkout has no `target/`, the configure resolves the generator and \
         stops with a FATAL_ERROR without it, and that is the red this lane \
         carried on its first ctest push. Commands were: {lane:?}\n{lane_log}"
    );

    let lane_configure = lane.iter().find(|command| is_configure(command));
    let gate_configure = gate.iter().find(|command| is_configure(command));
    assert!(
        gate_configure.is_some(),
        "⚠ `sce_main_build_dir` configured nothing in a tree that has no \
         `build/`, so the comparison below would hold vacuously:\n{gate_log}"
    );
    assert_eq!(
        lane_configure, gate_configure,
        "⚠ the lane must configure the tree through `sce_main_build_dir`, not \
         with a recipe of its own. The gate refuses a tree configured any other \
         way — build type, `BUILD_TESTS`, `SCE_ENABLE_MESH` — so a second \
         recipe here builds something the rounds then decline to judge.\n\
         lane: {lane:?}\ngate: {gate:?}"
    );

    assert!(
        lane.iter()
            .any(|command| command.starts_with("cmake --build")),
        "⚠ the lane must BUILD the tree, not only configure it. `scripts/mutate` \
         builds its own baseline with the output discarded, so a compile error \
         first met there is reported as a round that proves nothing rather than \
         as the build failure it is. Commands were: {lane:?}\n{lane_log}"
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

/// The one function in the Rust test harness that asserts the W3C BasicHTTP
/// fixture server is reachable, and panics with instructions when it is not
/// (`backends/rust/tests/src/harness.rs`). A test that calls it cannot pass
/// without a listener on the fixture endpoint, so a round over such a suite
/// needs one started for it.
///
/// Matched as a CALL — the name followed by its open paren — rather than as a
/// bare mention. The first shape of this scan looked for the name alone and
/// flagged `mutation_rounds_selection.cases`, whose oracle is THIS file, which
/// mentions the name in the line below. A scanner that cannot tell a call from
/// the string it searches for reports itself.
const HTTP_HARNESS_ENTRY: &str = "setup_http_test";

/// A suite that cannot pass without the W3C fixture server says so, so the gate
/// can start one.
///
/// The requirement is invisible in a selector. `--test send_namelist_over_http`
/// names a target, not a socket, and the only place the socket appears is
/// inside the test's own source — where nothing choosing rounds ever looks. So
/// `send_namelist_reaches_the_form.cases` was authored on a machine that had
/// the server up from an earlier gate, and went red in CI on every run it ever
/// had: `baseline is not green (1 failing)`, which is the harness reporting the
/// guard's panic as an ordinary failing test. Two CI runs and 34 hours before
/// anybody read past the verdict to the socket.
///
/// Asserted from the ORACLE side rather than from a list of casefile names,
/// because a list would be the same missing-declaration failure with an extra
/// place to forget it. The oracle is the test that noticed — the file the
/// harness already resolves through `cargo metadata` — and whether it calls the
/// harness entry above is a fact about that file, not about anybody's memory.
///
/// What it does not reach, stated rather than discovered later: a suite that
/// talks to the server from a module the selector pulls in rather than from the
/// oracle's own source. The floor below is what keeps that from silently
/// becoming "no casefile needs anything" — a scan that matches nothing is
/// indistinguishable from a corpus that is clean.
#[test]
fn a_casefile_whose_suite_needs_the_http_fixture_declares_it() {
    let mut needing: Vec<String> = Vec::new();
    let mut undeclared: Vec<(String, String)> = Vec::new();

    let call = format!("{HTTP_HARNESS_ENTRY}(");
    for casefile in casefiles() {
        let Some(oracle) = declared_oracles(&casefile).into_iter().find(|oracle| {
            oracle.ends_with(".rs")
                && fs::read_to_string(repo_root().join(oracle))
                    .map(|src| src.contains(&call))
                    .unwrap_or(false)
        }) else {
            continue;
        };
        needing.push(casefile.clone());
        if !declared_needs(&casefile)
            .iter()
            .any(|service| service == "http-fixture")
        {
            undeclared.push((casefile, oracle));
        }
    }

    assert!(
        !needing.is_empty(),
        "⚠ no casefile in the corpus resolves an oracle that calls \
         `{HTTP_HARNESS_ENTRY}`, so this test asserted nothing. Either the \
         harness entry was renamed — re-aim `HTTP_HARNESS_ENTRY` at whatever \
         asserts the server is reachable now — or `declared_oracles` stopped \
         resolving cargo oracles to paths, which would blind the change-set \
         intersection at the same stroke."
    );

    assert!(
        undeclared.is_empty(),
        "⚠ {} casefile(s) drive a suite that calls `{HTTP_HARNESS_ENTRY}` and do \
         not declare `mutation_needs http-fixture`: {undeclared:?}\n\
         Without the declaration nothing starts the W3C fixture server for the \
         round, the suite's own guard refuses for want of a socket, and the \
         round reports `baseline is not green` — a true verdict about the wrong \
         thing, which is what this casefile's first two CI runs both said.",
        undeclared.len()
    );
}

/// A stand-in for the W3C BasicHTTP fixture server, put where the gate looks
/// for the real one. It records that it is up, un-records it when stopped, and
/// binds no port.
///
/// The port is the reason it is a stand-in. This test runs inside `cargo test
/// --workspace`, which the `workspace-tests` gate runs with a real fixture
/// server already holding the endpoint: a second one would refuse to bind, and
/// one this test held would make the C++ ctest suite report 13 of its cases
/// Not Run — the shape `sce_gate_requires_free_http_port` refuses. What is
/// under test is whether the gate PROVISIONS a declared service around the
/// round that declared it, not the service's HTTP.
///
/// The marker is written at start and removed on the way out, which is what
/// makes "was the service up while THIS round ran" a fact the round reads
/// rather than a claim about the gate's source. The ceiling is what keeps a
/// stand-in the gate forgot to stop from outliving the tree it was written
/// into.
const STANDIN_SERVICE: &str = r#"
const fs = require('fs');
const http = require('http');
const marker = process.env.SCE_ROUND_SERVICE_MARKER;
const stop = () => {
    try { fs.unlinkSync(marker); } catch (err) { /* already stopped */ }
    process.exit(0);
};
process.on('SIGTERM', stop);
process.on('SIGINT', stop);
// ANY exit removes the marker, not just a signalled one. A stand-in that died
// on its own — losing a bind, say — used to leave the file behind, and the
// round after it then read "the service is still up": a true statement about
// a process that was already gone, and a diagnosis that costs an hour.
process.on('exit', () => { try { fs.unlinkSync(marker); } catch (err) { /* already gone */ } });
// Loudly, so the gate's log says what happened instead of the next round
// inheriting a mystery.
process.on('uncaughtException', (err) => { console.error(`standin: ${err.message}`); process.exit(1); });
// The marker BEFORE the listener, so a caller that waits for the port to
// accept has thereby waited for the marker too. The gate waits on the socket
// because that is the readiness the real server has; this file is what makes
// the two the same question.
fs.writeFileSync(marker, `${process.pid}\n`);
// A listener at all, because the gate now waits for one. Standing in for an
// HTTP server without opening a port made "ready" mean two different things
// on the two sides of this test, and the gate's fixed one-second settle was
// what papered over the difference.
// `'localhost'`, spelled exactly as the real server spells it: Node resolves
// that verbatim, so binding `'127.0.0.1'` here while the gate's probe asks for
// `localhost` would put the two on different address families on any host
// whose `localhost` answers `::1` first — the failure this whole wait was
// added to remove, reintroduced in the stand-in.
http.createServer((req, res) => { res.writeHead(200); res.end('standin'); })
    .listen(Number(process.argv[2] || 0), 'localhost');
setTimeout(stop, 60000);
"#;

/// What each round saw of the service the gate was meant to provision for it,
/// and whether anything was left running afterwards.
struct RoundsRun {
    ok: bool,
    /// One `(casefile, "up" | "down")` per round, in the order they ran.
    observed: Vec<(String, String)>,
    service_outlived_the_run: bool,
    log: String,
}

/// Run the gate over the named rounds and record, from INSIDE each round,
/// whether the declared service was up while that round ran.
///
/// The rounds are stood in for; the gate is not. A real round is a rebuild plus
/// a suite run — 139s for the cheapest cargo casefile in this corpus — and what
/// is under test is which rounds the gate wraps in a service, not what any
/// round concludes. Every DECLARATION query still reaches the real harness:
/// `needs` is read out of `scripts/mutate --declares`, and a stand-in answering
/// that would be this test writing both sides of the answer.
///
/// A copy under a fresh index rather than the checkout, for the reason
/// `selection_without_a_cmake_tree` gives: the gate enumerates its corpus with
/// `git ls-files`, so the tree has to be a repository — and this one must not
/// carry the `build/` that would let a ctest round past the precondition.
fn rounds_observed(rounds: &[&str]) -> RoundsRun {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();
    for entry in ["scripts", "sce-build/tests/mutations"] {
        let dest = root.join(entry);
        fs::create_dir_all(dest.parent().expect("a parent")).expect("create the fixture tree");
        copy_tree(&repo_root().join(entry), &dest);
    }
    assert!(
        !root.join("build/CMakeCache.txt").exists(),
        "this tree was supposed to be the one without a CMake cache"
    );

    // The real harness moved aside, with a stand-in in its place that answers a
    // ROUND and delegates every query. `--declares` is where the `needs` key is
    // produced, so it has to stay the real one: a case that stops emitting the
    // key must still reach the gate through this fixture.
    let harness = root.join("scripts/mutate-under-test");
    fs::rename(root.join("scripts/mutate"), &harness).expect("move the harness aside");
    let marker = root.join("service.pid");
    let observed = root.join("rounds.tsv");
    write_shim(
        &root.join("scripts/mutate"),
        &format!(
            "if [[ \"${{1:-}}\" == -* ]]; then exec '{harness}' \"$@\"; fi\n\
             if [[ -e \"$SCE_ROUND_SERVICE_MARKER\" ]]; then state=up; else state=down; fi\n\
             printf '%s\\t%s\\n' \"$1\" \"$state\" >> \"$SCE_ROUND_OBSERVED\"\n",
            harness = harness.display()
        ),
    );

    fs::create_dir_all(root.join("tests/w3c")).expect("create the fixture's service directory");
    fs::write(
        root.join("tests/w3c/standalone_http_server.js"),
        STANDIN_SERVICE,
    )
    .expect("write the stand-in service");
    // The REAL endpoint owner, not a stand-in. `sce_gate_http_fixture_server`
    // resolves where the listener answers by reading this header
    // (§scxml-C-2-3), and refuses rather than defaulting when it cannot — so a
    // fixture tree that carries the service but not the fact the service is
    // addressed by fails the gate before any round runs. Copied rather than
    // written here because a second spelling of the endpoint is the thing
    // `http-endpoint-ssot` exists to refuse.
    fs::copy(
        repo_root().join("tests/w3c/basic_http_test_endpoint.h"),
        root.join("tests/w3c/basic_http_test_endpoint.h"),
    )
    .expect("copy the endpoint owner");

    for args in [vec!["init", "-q"], vec!["add", "-A"]] {
        let out = Command::new("git")
            .args(&args)
            .current_dir(root)
            .output()
            .expect("prepare the fixture index");
        assert!(out.status.success(), "git {args:?} failed in the fixture");
    }

    let out = gate_shell()
        .arg("scripts/gates/mutation-rounds.sh")
        .current_dir(root)
        .env("SCE_MUTATION_ROUNDS", rounds.join(","))
        .env("SCE_ROUND_SERVICE_MARKER", &marker)
        .env("SCE_ROUND_OBSERVED", &observed)
        // A port of this test's OWN, because the endpoint header's default is
        // already held when this runs: `workspace-tests` starts a gate-wide
        // fixture server on it, and this test executes another gate inside
        // that one. The stand-in then loses the bind, dies before anything
        // signals it, and leaves its marker behind — which reads as "the
        // service was still up for the round after", a true statement about
        // the wrong thing. `sce_http_endpoint_port` honours this variable for
        // exactly this reason: the header holds the default, and the variable
        // is what moves the endpoint.
        .env("SCE_W3C_HTTP_PORT", "18080")
        // The selection is settled by the variable above; a dry run would stop
        // the gate before the loop this test is about, and a change set left in
        // a developer's environment would choose a different set of rounds.
        // Both are cleared by `gate_shell`, along with the shard — which is the
        // one this site was measured leaking on 2026-08-30.
        .output()
        .expect("run the gate");

    let service_outlived_the_run = marker.exists();
    if service_outlived_the_run {
        // Reaped here rather than left to the stand-in's own ceiling: the
        // assertion below is about to fail, and a failing test must not leave a
        // process behind to explain.
        if let Ok(pid) = fs::read_to_string(&marker) {
            let _ = Command::new("kill").arg(pid.trim()).status();
        }
    }

    RoundsRun {
        ok: out.status.success(),
        observed: fs::read_to_string(&observed)
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let (casefile, state) = line.split_once('\t').unwrap_or_else(|| {
                    panic!("⚠ every round line must be `casefile<TAB>state`; got {line:?}")
                });
                (casefile.to_string(), state.to_string())
            })
            .collect(),
        service_outlived_the_run,
        log: format!(
            "--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    }
}

/// The declaration reaches something that acts on it — measured by running the
/// gate and asking the rounds what was up while they ran.
///
/// The half above proves a casefile SAYS what it needs; this proves the gate
/// provides it, and provides it around one round rather than around the whole
/// run. Neither implies the other: a declaration nothing reads is a comment,
/// and a gate that started a server unconditionally would break the ctest
/// rounds beside it, whose C11 entries bring up their own listener on the same
/// port through the `w3c_c_http_server` CMake fixture and whose C++ W3C runner
/// binds it directly.
///
/// This was a source scan first — `lib.sh` defines the scoped helper, the gate
/// reads the `needs` key, the gate mentions the helper — and the corpus
/// measured what that was worth. `the gate reads the service and provisions
/// nothing` deletes the two lines that wrap the round, and the scan kept
/// passing: the helper's NAME survives the deletion, in the comment three lines
/// above it. SURVIVED 0/11 red, CI run 32540592390. A check that greps for a
/// name cannot tell a call from the prose describing it, and this file has now
/// been on both sides of that mistake — `HTTP_HARNESS_ENTRY` above matches a
/// call for the same reason.
///
/// So the observation is made from inside the rounds. Three of them: one before
/// the declaring round, the declaring round, one after. `down, up, down` is the
/// whole clause — the service is not up before the round that asked for it, is
/// up while that round runs, and is down again by the next — and no arrangement
/// of source text produces it without the gate actually starting and stopping a
/// process.
#[test]
fn the_gate_starts_the_declared_service_for_that_round_and_no_other() {
    let corpus = casefiles();
    assert!(
        corpus.len() >= 20,
        "the sweep found only {} casefile(s), so this test is not measuring \
         the corpus it claims to",
        corpus.len()
    );

    let needs_of: BTreeMap<String, Vec<String>> = corpus
        .iter()
        .map(|casefile| (casefile.clone(), declared_needs(casefile)))
        .collect();

    // Cargo rounds throughout: the fixture has no configured CMake tree on
    // purpose, and the gate refuses a ctest round for that before it reaches
    // the loop under test — a refusal is exit 3, not a verdict.
    let declaring = corpus
        .iter()
        .find(|casefile| {
            needs_of[*casefile]
                .iter()
                .any(|need| need == "http-fixture")
                && !declares_ctest(casefile)
        })
        .unwrap_or_else(|| {
            panic!(
                "⚠ no cargo casefile in the corpus declares `mutation_needs \
                 http-fixture`, so this test asserted nothing. Either the \
                 declaration was dropped — `send_namelist_reaches_the_form.cases` \
                 carried it — or the service vocabulary moved and this test has \
                 to be re-aimed at whatever names an outside service now."
            )
        })
        .clone();
    let controls: Vec<String> = corpus
        .iter()
        .filter(|casefile| needs_of[*casefile].is_empty() && !declares_ctest(casefile))
        .take(2)
        .cloned()
        .collect();
    assert_eq!(
        controls.len(),
        2,
        "⚠ the corpus has fewer than two cargo casefiles that declare no \
         service, so the rounds either side of the declaring one cannot be \
         chosen and the scoping half of this test would assert nothing."
    );

    let order = vec![
        controls[0].as_str(),
        declaring.as_str(),
        controls[1].as_str(),
    ];
    let run = rounds_observed(&order);

    assert!(
        run.ok,
        "⚠ the gate failed while running the rounds:\n{}",
        run.log
    );
    let ran: Vec<&str> = run
        .observed
        .iter()
        .map(|(casefile, _)| casefile.as_str())
        .collect();
    assert_eq!(
        ran, order,
        "⚠ the gate must run every round it was named, in order. Anything else \
         and the states below belong to rounds other than the ones this test \
         chose:\n{}",
        run.log
    );

    assert_eq!(
        run.observed[1].1, "up",
        "⚠ `{declaring}` declares `mutation_needs http-fixture` and its round \
         ran with the service down. That is the defect exactly: the requirement \
         is knowable everywhere and acted on nowhere. The round then reaches \
         its baseline, the suite's own guard refuses for want of a socket, and \
         the harness reports `baseline is not green (1 failing)` — a true \
         verdict about the wrong thing, which is what this casefile's first two \
         CI runs both said.\n{}",
        run.log
    );
    assert_eq!(
        run.observed[0].1, "down",
        "⚠ the service was already up for `{}`, which declares none. A gate-wide \
         `sce_gate_http_fixture_server` does that, and it is why the scoped form \
         exists: the ctest rounds beside these bind the endpoint themselves and \
         fail against a listener already held.\n{}",
        controls[0], run.log
    );
    assert_eq!(
        run.observed[2].1, "down",
        "⚠ the service was still up for `{}`, the round AFTER the one that \
         declared it. `sce_gate_with_http_fixture_server` must stop it — and \
         `wait` for it — before the loop's next turn, or the port is still held \
         when a ctest round tries to bind it.\n{}",
        controls[1], run.log
    );
    assert!(
        !run.service_outlived_the_run,
        "⚠ the gate exited with the service it started still running. A fixture \
         server that outlives its gate is what the next suite trips over, which \
         is why the start arms an exit cleanup as well as the scoped stop.\n{}",
        run.log
    );
}
