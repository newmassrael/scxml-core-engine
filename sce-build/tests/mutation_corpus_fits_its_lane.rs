// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! No mutation round is scheduled into a job that cannot finish it.
//!
//! `mutation-rounds.yml` gives each job a 330-minute ceiling, and a job that
//! crosses it is `cancelled` — neither green nor red, which is the corpus's
//! worst outcome: an absent verdict that reads as nothing at all. Measured on
//! dispatch 32803356117 (2026-08-25), when the lane ran one job per CASEFILE,
//! EIGHT of its twenty-four rounds ended exactly at that ceiling. Forty-four
//! hours of machine time said nothing about any case.
//!
//! ## What the previous shape of this test got wrong
//!
//! It counted cases, and capped a casefile at 21 of them. A case is not a unit
//! of time, and the same dispatch measured how far apart the units are:
//!
//! ```text
//!     host_processor_declaration      16 cargo cases     73 min   finished
//!     error_cascade_is_bounded_ctest  16 ctest cases   >330 min   cancelled
//! ```
//!
//! Same count, same ceiling, opposite outcomes. This file's own header had
//! even named both files and asserted the ORDER between them — that
//! `mutation_rounds_selection`'s 21 cheap cases were "closest to the ceiling"
//! — and the measurement reversed it: the cheap 21 finished in 4h16m and the
//! expensive 16 did not finish at all. A proxy nobody could check had been
//! standing in for an arithmetic nobody had done.
//!
//! ## What binds instead
//!
//! A round is a rebuild and a test run PER CASE, so a job's cost is
//! `setup + cases × per-case`, and `per-case` is a property of the RUNNER. The
//! casefile therefore stops being the unit that has to fit: the lane expands a
//! casefile into `ceil(cases / cases-per-job)` jobs and each runs
//! `scripts/mutate --shard I/N` over its own slice.
//!
//! This test is the arithmetic tying that slice size to the ceiling. It reads
//! the ceiling out of the workflow and the cost model out of the gate, so
//! neither is restated here — a constant with two spellings is one that can be
//! changed in the wrong one — and then checks that a full slice fits.
//!
//! ## Why the cost model is the gate's and not this file's
//!
//! It used to live here, as `(setup, per-case)` minutes measured on 2026-08-25,
//! while the gate held a slice size chosen against it. The tree grew and the
//! constant did not: on runs 39782655402 and 39948952108 (2026-10-22/23) a C++
//! round spent up to 98 minutes before its baseline and up to 176 on one case,
//! so the three-case slice this file still certified was 626 minutes, and every
//! multi-case C++ casefile was cancelled. The arithmetic was right; its input
//! was stale, and nothing that ran could notice. The gate now derives the slice
//! from the cost and checks every job's duration against what the cost
//! promised it, so a stale model turns a job red while the job still finishes.
//!
//! The remedy when this fires is to make a case cheaper or re-measure the cost
//! in `scripts/gates/mutation-rounds.sh`, not to raise the workflow's ceiling:
//! 330 sits 30 minutes below the platform's own so that a timeout arrives with
//! this lane's name on it, and raising it spends that margin to move the
//! cancellation later.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod common;
use common::gate_selectors::gate_shell;

/// The cost model and its arithmetic, sourced by the gate.
const COST_MODEL: &str = "scripts/lib/mutation_rounds_cost.sh";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The `timeout-minutes:` the rounds job declares.
///
/// Read as text and not through a YAML parser for the same reason the rest of
/// this repository's workflow tests are: what is asserted is that a particular
/// line says a particular thing, and a parser would happily answer from a
/// default the file does not contain.
///
/// From the `rounds:` job on, and not from the top of the file: the selection
/// job above it declares a ceiling of its own, and the first one in the file is
/// that one.
fn lane_timeout_minutes(workflow: &str) -> u32 {
    workflow
        .lines()
        .skip_while(|line| *line != "  rounds:")
        .filter_map(|line| line.trim().strip_prefix("timeout-minutes:"))
        .filter_map(|value| value.trim().parse().ok())
        .next()
        .unwrap_or_else(|| {
            panic!(
                "no `timeout-minutes:` in .github/workflows/mutation-rounds.yml. \
                 The key moved or was removed — and a job with no declared ceiling \
                 inherits the platform's 360, which is the number this lane made \
                 explicit so that a timeout would arrive with its own name on it"
            )
        })
}

/// What one job costs, by runner, as `(setup, per-case)` minutes, read out of
/// the cost model's own `case` arms: `ctest) printf '100 180\n' ;;` and the
/// `*)` default.
///
/// The model is the one place this is decided — the gate derives the slice
/// from it and the workflow expands whatever the gate prints — so it is also
/// the one place to read it from.
fn cost_model(gate: &str) -> BTreeMap<String, (u32, u32)> {
    let mut found = BTreeMap::new();
    let mut inside = false;
    for line in gate.lines() {
        if line.starts_with("mutation_rounds_cost()") {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if line == "}" {
            break;
        }
        let trimmed = line.trim();
        let Some((label, body)) = trimmed.split_once(')') else {
            continue;
        };
        // `ctest) printf '100 180\n' ;;` — the two numbers between the quotes.
        let Some(rest) = body.trim().strip_prefix("printf '") else {
            continue;
        };
        let Some((numbers, _)) = rest.split_once('\\') else {
            continue;
        };
        let mut minutes = numbers.split_whitespace().map(str::parse::<u32>);
        if let (Some(Ok(setup)), Some(Ok(per_case)), None) =
            (minutes.next(), minutes.next(), minutes.next())
        {
            found.insert(label.trim().to_string(), (setup, per_case));
        }
    }
    found
}

/// The share of the ceiling the gate schedules a job into
/// (`MUTATION_ROUNDS_BUDGET_PERCENT=85`).
fn budget_percent(model: &str) -> u32 {
    model
        .lines()
        .find_map(|line| line.strip_prefix("MUTATION_ROUNDS_BUDGET_PERCENT="))
        .and_then(|value| value.trim().parse().ok())
        .expect(
            "no `MUTATION_ROUNDS_BUDGET_PERCENT=` in scripts/lib/mutation_rounds_cost.sh, \
             so the share of the ceiling a job is scheduled into cannot be read",
        )
}

/// What the model's own function prints for `call`, asked of bash with the
/// model sourced: the arithmetic under test is the shell's, so it is the shell
/// that answers.
fn model_says(call: &str) -> (bool, String) {
    let out = gate_shell()
        .arg("-c")
        .arg(format!("source {COST_MODEL} && {call}"))
        .current_dir(repo_root())
        .output()
        .expect("run bash");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    )
}

/// The slice the gate hands a job is what the budget leaves after setup,
/// divided by one case, for every runner and at any ceiling.
///
/// Asked of the model's own function over several ceilings rather than read
/// off the corpus: whether a wrong derivation moves a shard count depends on
/// which case counts the corpus happens to hold. Measured on run 40054201618,
/// a derivation that forgot the setup changed the cargo slice from 20 to 21
/// and no casefile's shard count with it, so the corpus-driven check above
/// read it as correct.
#[test]
fn the_slice_is_what_the_budget_leaves_after_setup() {
    let source =
        std::fs::read_to_string(repo_root().join(COST_MODEL)).expect("read the cost model");
    let model = cost_model(&source);
    for ceiling in [100u32, 120, 200, 330, 600] {
        let derived = cases_per_job(&source, ceiling);
        for (runner, label) in [("ctest", "ctest"), ("cargo", "*")] {
            let (setup, per_case) = model[label];
            let call = format!("mutation_rounds_cases_per_job {runner} {ceiling}");
            let (ok, said) = model_says(&call);
            assert!(ok, "`{call}` failed");
            assert_eq!(
                said,
                derived[label].to_string(),
                "`{call}` sized a job at {said} case(s); {}% of {ceiling} minutes \
                 less {setup} of setup, at {per_case} a case, is {}. A slice that \
                 forgets the setup holds more cases than the job can finish.",
                budget_percent(&source),
                derived[label]
            );
        }
    }
}

/// The promise a job is held to is its setup and its largest slice: a job of
/// one ctest case is promised setup plus one case, and a 21-case cargo
/// casefile split in two is promised setup plus eleven.
#[test]
fn a_job_is_promised_its_setup_and_its_largest_slice() {
    let model = cost_model(&std::fs::read_to_string(repo_root().join(COST_MODEL)).unwrap());
    let (ctest_setup, ctest_case) = model["ctest"];
    let (cargo_setup, cargo_case) = model["*"];
    for (call, expected) in [
        (
            "mutation_rounds_promise ctest 1 1".to_string(),
            ctest_setup + ctest_case,
        ),
        (
            "mutation_rounds_promise cargo 21 2".to_string(),
            cargo_setup + 11 * cargo_case,
        ),
    ] {
        let (ok, said) = model_says(&call);
        assert!(ok, "`{call}` failed");
        assert_eq!(
            said,
            expected.to_string(),
            "`{call}` promised {said} minutes; setup and the largest slice are \
             {expected}. A promise smaller than the job's real shape reads every \
             honest job as overrunning, and a larger one hides a stale model."
        );
    }
}

/// A job is judged against its promise in whole minutes rounded UP, so one
/// that overran by seconds is read as past it, and one inside it is not.
#[test]
fn a_job_past_its_promise_is_read_as_past_it() {
    let started = 1_000_000u64;
    for (ended, promised, overran) in [
        (started + 280 * 60, 280, false),
        (started + 280 * 60 + 1, 280, true),
        (started + 400 * 60, 280, true),
        (started + 10 * 60, 280, false),
    ] {
        let call = format!("mutation_rounds_overran {started} {ended} {promised}");
        let (said_overran, took) = model_says(&call);
        assert_eq!(
            said_overran, overran,
            "`{call}` (took {took} min) answered overran={said_overran}. A job \
             past its promise is the cost model falling behind the tree, and \
             reading it as inside the promise is the silence that scheduled C++ \
             jobs into a ceiling they could only be cancelled at."
        );
    }
}

/// The lane notes the job's start in its first step, and the gate judges every
/// job's duration after its round. Either half missing leaves the promise
/// unchecked, and the gate says "not judged" rather than failing a run by hand.
#[test]
fn every_lane_job_is_judged_against_its_promise() {
    let root = repo_root();
    let workflow = std::fs::read_to_string(root.join(".github/workflows/mutation-rounds.yml"))
        .expect("read the mutation-rounds workflow");
    let first_step = workflow
        .lines()
        .skip_while(|line| *line != "  rounds:")
        .skip_while(|line| line.trim() != "steps:")
        .find(|line| line.trim_start().starts_with("- name:"))
        .unwrap_or_default();
    assert_eq!(
        first_step.trim(),
        "- name: Note when the job started",
        "the rounds job's first step is not the one noting its start, so the \
         setup the cost model prices is not measured from the job's beginning"
    );
    assert!(
        workflow.contains("echo \"SCE_MUTATION_JOB_STARTED=$(date +%s)\" >> \"$GITHUB_ENV\""),
        "the rounds job does not export SCE_MUTATION_JOB_STARTED, so no job's \
         duration is ever judged against its promise"
    );
    let gate = std::fs::read_to_string(root.join("scripts/gates/mutation-rounds.sh"))
        .expect("read the mutation-rounds gate");
    assert!(
        gate.contains("    mutation_rounds_budget_check \"${SELECTED[0]}\" || over_budget=1")
            && gate.contains("(( over_budget == 0 )) || exit 1"),
        "the gate no longer judges a lane job against its promise, or no longer \
         turns an overrun red"
    );
}

/// The cases a job may hold for each runner, derived as the gate derives it:
/// what the budget leaves after setup, divided by one case.
fn cases_per_job(gate: &str, ceiling: u32) -> BTreeMap<String, u32> {
    let budget = ceiling * budget_percent(gate) / 100;
    cost_model(gate)
        .into_iter()
        .map(|(runner, (setup, per_case))| (runner, budget.saturating_sub(setup) / per_case.max(1)))
        .collect()
}

#[test]
fn no_job_is_scheduled_with_more_cases_than_it_can_finish() {
    let root = repo_root();
    let workflow_path = root.join(".github/workflows/mutation-rounds.yml");
    let gate_path = root.join(COST_MODEL);
    let workflow = std::fs::read_to_string(&workflow_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", workflow_path.display()));
    let gate = std::fs::read_to_string(&gate_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", gate_path.display()));

    let ceiling = lane_timeout_minutes(&workflow);
    let model = cost_model(&gate);
    let per_job = cases_per_job(&gate, ceiling);

    // A floor before any arithmetic. A parse that stopped finding the `case`
    // arms would leave an empty map, and an empty map satisfies every
    // inequality below without reading a single number — the shape where a
    // scan reports green because it found the file and not the thing.
    assert!(
        model.contains_key("ctest") && model.contains_key("*"),
        "read {model:?} out of `mutation_rounds_cost` in {}. The function moved \
         or its arms were respelled, and this test cannot check an arithmetic \
         whose terms it did not find",
        gate_path.display()
    );

    for (runner, &(setup, per_case)) in &model {
        let cases = per_job[runner];
        // At least one: a slice of none schedules jobs that run no case, and
        // the gate refuses such a model before it schedules anything.
        assert!(
            cases >= 1,
            "one `{runner}` case is measured at {setup} + {per_case} minutes, \
             which does not fit the {}% of the lane's {ceiling}-minute ceiling a \
             job is scheduled into. No slice size can help: make one case \
             cheaper (what it rebuilds, what setup builds) in the harness.",
            budget_percent(&gate)
        );
        let cost = setup + cases * per_case;
        assert!(
            cost <= ceiling,
            "a full `{runner}` job holds {cases} case(s) and is measured to cost \
             {setup} + {cases}x{per_case} = {cost} minutes against the lane's \
             {ceiling}-minute ceiling. A job that crosses it is `cancelled`, \
             which is neither green nor red: eight rounds of dispatch \
             32803356117 ended exactly that way and said nothing about any \
             case. The derivation in {} admitted a slice the model says cannot \
             finish; raising `timeout-minutes` in {} only spends the 30-minute \
             margin that makes the failure arrive with this lane's name on it.",
            gate_path.display(),
            workflow_path.display(),
        );
    }
}

/// Every runner the corpus declares is one the cost model has a row for.
///
/// Separate from the arithmetic above because it fails for a different reason:
/// a casefile landing on a runner nobody measured would be scheduled by the
/// gate's `*)` default and checked here against nothing. Naming it is the
/// whole point — the `*` row is a deliberate stand-in for `go` and `pytest`,
/// not a hole, and a NEW runner needs a decision rather than a fallthrough.
#[test]
fn the_gate_sizes_every_slice_it_hands_the_lane() {
    let root = repo_root();
    let model_source = std::fs::read_to_string(root.join(COST_MODEL)).expect("read the cost model");
    let workflow = std::fs::read_to_string(root.join(".github/workflows/mutation-rounds.yml"))
        .expect("read the mutation-rounds workflow");
    let per_job = cases_per_job(&model_source, lane_timeout_minutes(&workflow));
    let ctest_per_job = per_job["ctest"];
    let default_per_job = per_job["*"];

    // The gate's own answer, over the whole corpus, through the channel the
    // lane uses. Driving it rather than restating its arithmetic is the point:
    // what is checked below is that the shard count it PRINTS partitions the
    // casefile into slices no larger than the number this test just read.
    // `gate_shell` rather than a bare `bash`: this site declared neither
    // `SCE_GATE_CHANGED_FILE` nor `SCE_MUTATION_SHARD`, so "the whole corpus"
    // was the whole corpus only as long as nobody's environment said otherwise.
    let out = gate_shell()
        .arg("scripts/gates/mutation-rounds.sh")
        .current_dir(&root)
        .env("SCE_MUTATION_ROUNDS_DRY_RUN", "1")
        .env("SCE_MUTATION_ROUNDS", "all")
        .output()
        .expect("run the mutation-rounds gate");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "the gate refused to answer for the whole corpus:\n{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let mut casefiles = 0usize;
    let mut cases_total = 0usize;
    for line in stdout.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let columns: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            columns.len(),
            3,
            "every dry-run line is `casefile<TAB>runner<TAB>shards`; got {line:?}"
        );
        let (casefile, runner) = (columns[0], columns[1]);
        let shards: u32 = columns[2]
            .parse()
            .unwrap_or_else(|_| panic!("{casefile}: shard count {:?} is not a number", columns[2]));
        let text = std::fs::read_to_string(root.join(casefile))
            .unwrap_or_else(|e| panic!("read {casefile}: {e}"));
        let cases = text
            .lines()
            .filter(|l| l.starts_with("mutation_case "))
            .count() as u32;
        casefiles += 1;
        cases_total += cases as usize;

        let allowed = if runner == "ctest" {
            ctest_per_job
        } else {
            default_per_job
        };
        assert!(
            shards >= 1,
            "{casefile} is worth {shards} job(s), so the matrix expands it into \
             nothing and the lane reports green having run no round on it"
        );
        // Ceiling division, re-derived here rather than read from the gate:
        // the point of the check is that the two answers agree.
        let expected = cases.div_ceil(allowed);
        assert_eq!(
            shards, expected,
            "{casefile} declares {cases} `{runner}` case(s), which is {expected} \
             job(s) at {allowed} per job, but the gate scheduled {shards}. Too \
             few puts more cases in a job than it is measured to finish, which \
             is the cancellation this lane exists to have stopped; too many \
             makes `scripts/mutate --shard` refuse an index the casefile cannot \
             supply."
        );
        // What actually has to hold: the largest slice. `--shard` spreads the
        // remainder over the leading shards, so the biggest is the ceiling of
        // the division, and it is that number the cost model was checked
        // against.
        let largest = cases.div_ceil(shards.max(1));
        assert!(
            largest <= allowed,
            "{casefile}'s largest slice holds {largest} `{runner}` case(s), over \
             the {allowed} a job is measured to finish"
        );
    }

    // Floors, so a walk that stopped finding things cannot pass by reading
    // nothing. Both were counted, not guessed: 87 casefiles holding 586 cases
    // on 2026-08-27, and they only ever grow.
    assert!(
        casefiles >= 80 && cases_total >= 500,
        "the gate answered for {casefiles} casefile(s) holding {cases_total} \
         case(s), expected at least 80 and 500. The selection or the corpus \
         walk moved — a corpus that reads as empty satisfies every ceiling"
    );
}

/// What `scripts/mutate --declares` says, per casefile, about the three things
/// the check below needs: its runner, how many cases it holds and the bound it
/// declared, in seconds, if it did.
struct Declared {
    runner: String,
    cases: u32,
    timeout: Option<u32>,
}

/// The declarations of every tracked casefile, asked of the harness in ONE
/// invocation, which is how the rest of this corpus's tests ask.
fn declarations() -> BTreeMap<String, Declared> {
    let casefiles = common::repository::paths_git_tracks(&["sce-build/tests/mutations/*.cases"]);
    let out = std::process::Command::new(repo_root().join("scripts/mutate"))
        .arg("--declares")
        .args(&casefiles)
        .current_dir(repo_root())
        .output()
        .expect("run scripts/mutate --declares");
    assert!(
        out.status.success(),
        "`scripts/mutate --declares` over the corpus failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut found: BTreeMap<String, Declared> = BTreeMap::new();
    let mut current = String::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let (key, value) = line.split_once('\t').unwrap_or((line, ""));
        if key == "casefile" {
            current = value.to_string();
            found.insert(
                current.clone(),
                Declared {
                    runner: String::new(),
                    cases: 0,
                    timeout: None,
                },
            );
            continue;
        }
        let Some(declared) = found.get_mut(&current) else {
            continue;
        };
        match key {
            "runner" => declared.runner = value.to_string(),
            "cases" => declared.cases = value.parse().unwrap_or(0),
            "timeout" => declared.timeout = value.parse().ok(),
            _ => {}
        }
    }
    found
}

/// A casefile that declares `mutation_timeout` has told the lane how long one
/// test binary may run, and the lane has to be able to pay for it.
///
/// The bound is a cost per case and not only for the baseline: a mutant whose
/// test binary never ends runs the whole bound before it is CAUGHT, and every
/// job runs its own baseline first. So the worst a job of `n` cases costs is
/// `setup + bound + n x (per-case + bound)`, with the setup and per-case
/// minutes the gate's cost model holds for the runner. The harness caps one
/// declaration at an hour so that a typo cannot reach a day on its own; this is
/// the check that a declaration under that cap still fits the job it lands in,
/// and its remedy is to lower the bound or split the casefile, as the cases per
/// job are lowered for the same reason above.
#[test]
fn a_declared_bound_fits_the_job_it_is_scheduled_into() {
    let root = repo_root();
    let workflow = std::fs::read_to_string(root.join(".github/workflows/mutation-rounds.yml"))
        .expect("read the mutation-rounds workflow");
    let model_source = std::fs::read_to_string(root.join(COST_MODEL)).expect("read the cost model");
    let ceiling = lane_timeout_minutes(&workflow);
    let model = cost_model(&model_source);
    let per_job = cases_per_job(&model_source, ceiling);
    let declarations = declarations();

    // A floor before any arithmetic, for the reason the test above has one: a
    // parse that stopped finding casefiles satisfies every inequality below.
    assert!(
        declarations.len() >= 80,
        "`--declares` answered for {} casefile(s), expected at least 80. The \
         output format or the corpus walk moved, and this check cannot judge \
         bounds it did not read",
        declarations.len()
    );
    let declaring: Vec<&String> = declarations
        .iter()
        .filter(|(_, declared)| declared.timeout.is_some())
        .map(|(path, _)| path)
        .collect();
    assert!(
        !declaring.is_empty(),
        "no casefile declares `mutation_timeout`, though the segmented queue's \
         race model needs one. Either the `timeout` line of `--declares` stopped \
         being read here, or the last declaration was removed on purpose, in \
         which case this floor goes with it"
    );

    for path in declaring {
        let declared = &declarations[path];
        let seconds = declared.timeout.expect("filtered to declared bounds");
        let bound = seconds.div_ceil(60);
        let &(setup, per_case) = model
            .get(&declared.runner)
            .or_else(|| model.get("*"))
            .expect("the cost model has a default row");
        let in_a_job = per_job
            .get(&declared.runner)
            .or_else(|| per_job.get("*"))
            .copied()
            .expect("the gate gives every runner a cases-per-job");
        let slice = declared.cases.min(in_a_job);
        let cost = setup + bound + slice * (per_case + bound);
        assert!(
            cost <= ceiling,
            "{path} declares mutation_timeout {seconds} ({bound} min). A job of \
             its {slice} case(s) on the `{}` runner costs, at worst, {setup} + \
             {bound} + {slice}x({per_case} + {bound}) = {cost} minutes against \
             the lane's {ceiling}-minute ceiling, and a job that crosses it is \
             `cancelled`, which is neither green nor red. Lower the bound, or \
             split the casefile.",
            declared.runner
        );
    }
}
