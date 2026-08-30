// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A selector in the caller's environment does not become the test's scenario.
//
// The `mutation-rounds` gate reads four environment variables to decide WHAT
// TO RUN, and the suites that drive it run it as a subprocess — which inherits
// whatever the caller had. The caller is not hypothetical: the round job in
// `.github/workflows/mutation-rounds.yml` sets `SCE_MUTATION_ROUNDS` and
// `SCE_MUTATION_SHARD` on purpose, and everything that job starts gets both.
//
// Measured 2026-08-30, on the tree as it stood: with `SCE_MUTATION_SHARD=1/2`
// exported, `the_gate_starts_the_declared_service_for_that_round_and_no_other`
// failed with `names a slice, but 3 casefile(s) are selected`. Five other call
// sites survived, and that is the part worth stating plainly: NOT because they
// cleared the shard — none of them did — but because the gate checks the
// dry-run flag before it checks the shard, so a dry run returns above the
// guard. They were upstream of the door, not behind it.
//
// Each site had been carrying a hand-written subtraction list. Between the six
// of them they cleared `SCE_MUTATION_ROUNDS` six times, `SCE_GATE_CHANGED_FILE`
// five, the dry-run flag six — and `SCE_MUTATION_SHARD` zero. Six copies of a
// list is six chances to miss the next entry, and one had already been missed.
//
// So this file measures the repair from three sides, because each covers what
// the others cannot:
//
//   1. THE DERIVATION, including the branch the real gate cannot reach.
//   2. THE COMMAND `gate_shell` builds, read back off the object rather than
//      out of the source that built it.
//   3. THE CALL SITES, so a seventh one written next month cannot go back to a
//      bare `bash` — the population is DERIVED from which files name the gate,
//      not listed here.
//
// And one PROBE that is none of those three: the test re-executes itself with
// every selector exported, which is the failure mode in the shape it actually
// arrives in. That probe reads what the child process received, not what the
// gate concluded, so it stays true if the gate ever reorders its checks — the
// exact reordering that would turn the five "survivors" above into failures.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

mod common;
use common::gate_selectors::{gate_shell, mutation_rounds_selectors, selectors_in, GATE_SCRIPT};
use common::rust_source::code_only;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Set in the child of the leak probe, so the recursion terminates.
const LEAK_CHILD: &str = "SCE_SELECTOR_LEAK_CHILD";
/// The value the probe exports, distinctive enough to find in an environment
/// dump and impossible to confuse with a real selection.
const LEAKED: &str = "LEAKED-BY-THE-PROBE";

// ── 1. The derivation ──────────────────────────────────────────────

#[test]
fn the_selectors_are_derived_from_the_gate_and_there_are_four() {
    let selectors = mutation_rounds_selectors();
    // The floor first: a derivation that finds nothing reports no leak for
    // every call site, which reads exactly like a tree with no leak.
    assert!(
        !selectors.is_empty(),
        "no selector derived from {GATE_SCRIPT} — the derivation has lost its subject"
    );
    let expected: BTreeSet<String> = [
        "SCE_GATE_CHANGED_FILE",
        "SCE_MUTATION_ROUNDS",
        "SCE_MUTATION_ROUNDS_DRY_RUN",
        "SCE_MUTATION_SHARD",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    // Stated as a set rather than a count, and as the answer this test expects
    // rather than one it reads back out of the thing it measures. A FIFTH
    // selector added to the gate is meant to fail here — the failure is the
    // notice that every call site now clears something new, which is the whole
    // point of deriving the list.
    assert_eq!(
        selectors, expected,
        "the gate's selector set moved; every call site now clears a different \
         list, so this expectation is the place to confirm that is intended"
    );
}

#[test]
fn a_name_the_script_assigns_is_a_local_and_not_a_selector() {
    // The branch the real gate cannot reach, because it assigns no `SCE_*`
    // name today. Without this the rule is kept alive only by its comment —
    // and a rule nothing exercises is one a refactor deletes for free.
    let derived = selectors_in(
        "SCE_ROUND_TMP=\"$(mktemp -d)\"\n\
         echo \"${SCE_MUTATION_ROUNDS:-}\" > \"$SCE_ROUND_TMP/x\"\n\
         export SCE_EXPORTED_LOCAL=1\n\
         : \"${SCE_EXPORTED_LOCAL}\"\n",
    );
    assert!(
        derived.contains("SCE_MUTATION_ROUNDS"),
        "a name only ever read must be derived as a selector: {derived:?}"
    );
    assert!(
        !derived.contains("SCE_ROUND_TMP"),
        "a name the script assigns is its own local, not something a caller \
         supplies: {derived:?}"
    );
    assert!(
        !derived.contains("SCE_EXPORTED_LOCAL"),
        "`export NAME=value` is an assignment too: {derived:?}"
    );
}

#[test]
fn a_name_that_survives_only_in_a_comment_is_not_a_selector() {
    // Comments are stripped before the scan, which cuts both ways and is meant
    // to. A scanner that read prose would clear a variable the gate stopped
    // reading years ago — a rule kept alive by its own explanation.
    let derived = selectors_in(
        "# SCE_RETIRED_SELECTOR used to choose the lane; it no longer does.\n\
         echo \"${SCE_MUTATION_SHARD:-}\"\n",
    );
    assert_eq!(
        derived,
        ["SCE_MUTATION_SHARD".to_string()].into_iter().collect(),
        "only the name the CODE reads is a selector: {derived:?}"
    );
}

#[test]
fn a_longer_identifier_is_not_split_into_a_selector() {
    // The scan walks bytes rather than words, so the name boundary has to be
    // checked explicitly: without it `MY_SCE_MUTATION_ROUNDS` would contribute
    // a selector no caller ever sets, and every site would clear a ghost.
    let derived = selectors_in("echo \"$MY_SCE_MUTATION_ROUNDS\"\necho \"$SCE_MUTATION_SHARD\"\n");
    assert_eq!(
        derived,
        ["SCE_MUTATION_SHARD".to_string()].into_iter().collect(),
        "a name embedded in a longer identifier is not a selector: {derived:?}"
    );
}

// ── 2. The command ─────────────────────────────────────────────────

#[test]
fn the_gate_shell_clears_every_derived_selector_and_nothing_else() {
    let cmd = gate_shell();
    // Read off the built object, not out of the source that built it: what a
    // call site inherits is decided by this map and by nothing else.
    let removed: BTreeSet<String> = cmd
        .get_envs()
        .filter(|(_, value)| value.is_none())
        .map(|(key, _)| key.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        removed,
        mutation_rounds_selectors(),
        "gate_shell must remove exactly the selectors the gate reads"
    );
    // And it must not have reached for `env_clear`: the gate needs `PATH`,
    // `HOME` and the rest to run at all, and a helper that emptied the
    // environment would pass every assertion above while breaking every
    // caller.
    assert!(
        cmd.get_envs().all(|(_, value)| value.is_none()),
        "gate_shell sets nothing of its own; the scenario's own .env() calls do"
    );
    assert_eq!(
        cmd.get_program(),
        "bash",
        "the sites this replaces all invoke bash"
    );
}

#[test]
fn a_scenario_can_still_set_a_selector_it_means() {
    // The opposite failure: a helper that cleared the selectors so thoroughly
    // that a test could no longer choose one would make every scenario the
    // empty scenario, and the suite would go green measuring nothing.
    let mut cmd = gate_shell();
    cmd.env("SCE_MUTATION_ROUNDS", "chosen");
    let set: Vec<String> = cmd
        .get_envs()
        .filter(|(key, value)| *key == "SCE_MUTATION_ROUNDS" && value.is_some())
        .map(|(_, value)| value.expect("checked").to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        set,
        vec!["chosen".to_string()],
        "a later .env() must win over the constructor's removal"
    );
}

// ── 3. The call sites ──────────────────────────────────────────────

/// The test files that RUN the gate: the ones passing its script to a command.
///
/// Derived rather than listed, so a seventh suite written next month is
/// measured on the day it lands. Only the top level of `tests/` — the helper
/// itself lives under `tests/common/` and is the one place a bare `bash` is
/// the right answer.
///
/// Two narrowings, and both were forced by a measurement rather than chosen.
///
/// COMMENTS ARE STRIPPED, through `common::rust_source` rather than here.
/// `workflow_trigger_coverage.rs` names the gate script in a comment and runs
/// nothing; on the first draft of this scan it landed in the population and
/// failed for having no `gate_shell`, which is a demand it can never satisfy —
/// a population holding a role that cannot reach zero. That same file's own
/// scan then made the mirror-image mistake against THIS file, which is why
/// the stripping is one shared answer and not a line in each scan.
///
/// AND THE NAME MUST BE AN ARGUMENT. Reading the gate's text is not running
/// it: `mutation_corpus_fits_its_lane` parses the script for its per-job
/// arithmetic, and a suite that only did that would be in the same position.
///
/// ⚠ The residue, stated rather than hidden: a suite that ran a mutation-rounds
/// WORKFLOW STEP without ever naming the gate script would not be found here.
/// None does today — `mutation_rounds_selection` does both — and the leak probe
/// below is what would still catch such a site if it ever drove the gate.
/// The second element is the file's CODE — prose removed, line numbering
/// untouched, so the line a caller reports is still the file's own.
fn files_that_drive_the_gate() -> Vec<(PathBuf, String)> {
    let dir = repo_root().join("sce-build/tests");
    let mut found: Vec<(PathBuf, String)> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .filter_map(|path| {
            let code = code_only(&fs::read_to_string(&path).ok()?);
            let runs_it = code
                .lines()
                .any(|line| line.contains("arg(") && line.contains(GATE_SCRIPT));
            runs_it.then_some((path, code))
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

#[test]
fn every_suite_that_drives_the_gate_builds_its_shell_through_the_helper() {
    let files = files_that_drive_the_gate();
    // The floor: an empty population passes vacuously, and this scan's
    // population is a directory listing plus a substring — both of which can
    // go empty for reasons that have nothing to do with the property.
    assert!(
        files.len() >= 2,
        "expected the suites that drive {GATE_SCRIPT}, found {:?}",
        files.iter().map(|(p, _)| p).collect::<Vec<_>>()
    );
    for (path, body) in &files {
        let bare: Vec<usize> = body
            .lines()
            .enumerate()
            .filter(|(_, line)| line.contains("Command::new(\"bash\")"))
            .map(|(i, _)| i + 1)
            .collect();
        assert!(
            bare.is_empty(),
            "{} builds a bare bash at line(s) {bare:?} — a command built that \
             way inherits whatever selectors the caller had, which is the leak \
             this file exists to hold shut. Use `gate_shell()`.",
            path.display()
        );
        assert!(
            body.contains("gate_shell"),
            "{} names the gate but never builds a shell through the helper — \
             either it drives the gate some other way, in which case this scan \
             has lost its population, or it is a site that was missed",
            path.display()
        );
    }
}

// ── The probe ──────────────────────────────────────────────────────

#[test]
fn a_selector_in_the_callers_environment_does_not_reach_the_gate() {
    if env::var_os(LEAK_CHILD).is_some() {
        the_child_half_of_the_probe();
        return;
    }

    // The parent half: re-run this very test with every selector exported,
    // which is the environment `mutation-rounds.yml` hands its round job.
    let exe = env::current_exe().expect("this test binary's own path");
    let mut cmd = Command::new(exe);
    cmd.arg("--exact")
        .arg("a_selector_in_the_callers_environment_does_not_reach_the_gate")
        .arg("--nocapture")
        .env(LEAK_CHILD, "1");
    let selectors = mutation_rounds_selectors();
    assert!(
        !selectors.is_empty(),
        "nothing to leak — see the floor above"
    );
    for name in &selectors {
        cmd.env(name, LEAKED);
    }
    let out = cmd
        .output()
        .expect("re-run this test with the selectors set");
    assert!(
        out.status.success(),
        "with every selector exported, the child half failed:\n{}",
        // INDENTED, and that is not cosmetic: the child is a libtest binary,
        // so its output carries `running 1 test` and `test … FAILED` at the
        // start of a line — the exact shapes `scripts/mutate` counts a run
        // with. Measured 2026-08-30: quoting it flush left made a mutation
        // round read `ran 9 tests, baseline ran 8` and report INCONCLUSIVE
        // for a case the suite had in fact caught.
        indented(&format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ))
    );
}

/// Quote a child process's output without letting its line starts read as this
/// process's own.
fn indented(text: &str) -> String {
    text.lines().map(|line| format!("    | {line}\n")).collect()
}

/// Runs inside a process whose environment holds every selector.
///
/// What it reads is the ENVIRONMENT the child command received, not the
/// verdict the gate reached. That distinction is the whole value of this
/// probe: five of the six call sites passed the 2026-08-30 measurement because
/// the gate returns for a dry run before it looks at the shard, so a probe
/// written against the gate's verdict would have called them clean.
fn the_child_half_of_the_probe() {
    let selectors = mutation_rounds_selectors();
    // The premise, asserted rather than assumed: if the parent failed to
    // export these, everything below passes while measuring nothing.
    for name in &selectors {
        assert_eq!(
            env::var(name).ok().as_deref(),
            Some(LEAKED),
            "the probe's own environment is missing {name}; there is nothing \
             here to leak and the assertions below would be vacuous"
        );
    }

    let out = gate_shell()
        .arg("-c")
        .arg("printenv")
        .output()
        .expect("dump the environment a gate_shell command receives");
    assert!(
        out.status.success(),
        "printenv failed under gate_shell: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let received = String::from_utf8_lossy(&out.stdout);

    for name in &selectors {
        assert!(
            !received.lines().any(|l| l.starts_with(&format!("{name}="))),
            "{name} reached a gate_shell child from the caller's environment:\n{received}"
        );
    }
    // The helper clears selectors, not the environment. A gate started with an
    // empty one cannot find `bash`'s own tools, so this is the other half of
    // "cleared exactly what it should".
    assert!(
        received.lines().any(|l| l.starts_with("PATH=")),
        "gate_shell emptied the environment instead of clearing the selectors:\n{received}"
    );
}
