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
/// Asserted as equality between the two mappings rather than against a list
/// written here, so an input added later is covered the day it is added.
#[test]
fn the_selection_and_the_rounds_are_handed_the_same_inputs() {
    let workflow = fs::read_to_string(repo_root().join(".github/workflows/mutation-rounds.yml"))
        .expect("read the mutation-rounds workflow");

    let select = step_env(&workflow, "Which casefiles does this change reach");
    let run = step_env(&workflow, "Run the rounds");

    assert!(
        !run.is_empty(),
        "⚠ the rounds step declares no `env:` at all, so the comparison below \
         would hold only by both sides being empty"
    );
    assert_eq!(
        select, run,
        "⚠ the two steps are handed different inputs. A key on only one side \
         is the asymmetry itself: reaching only the selection makes the lane \
         install a CMake tree for a set it will not run, and reaching only the \
         rounds makes it run a set it never prepared for. Whichever key it is, \
         it belongs in both `env:` blocks or in neither."
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
    let out = Command::new("bash")
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
