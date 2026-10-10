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
//! the ceiling out of the workflow and the slice size out of the gate, so
//! neither number is restated here — a constant with two spellings is one that
//! can be changed in the wrong one — and then checks that a full slice fits.
//!
//! The remedy when it fires is to LOWER the cases-per-job in
//! `scripts/gates/mutation-rounds.sh`, not to raise the workflow's ceiling:
//! 330 sits 30 minutes below the platform's own so that a timeout arrives with
//! this lane's name on it, and raising it spends that margin to move the
//! cancellation later.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod common;
use common::gate_selectors::gate_shell;

/// What one case costs, in minutes, by runner: `(setup, per-case)`.
///
/// Measured from dispatch 32803356117's own job durations on `ubuntu-latest`,
/// rounded UP, because this is a ceiling check and an optimistic cost model
/// would license exactly the schedule that was cancelled.
///
/// * ctest — the single-case job `parallel_microstep_owns_exit_and_entry` took
///   95.8 minutes, and the marginal cost across the completed multi-case jobs
///   ranged from 27.9 to 51.4 minutes (5-case `targetless_…` at 301.4, 7-case
///   `discarded_…` at 300.6, 4-case `host_processor_dispatch_cpp` at 179.4).
///   It is that expensive because these mutations land in headers the whole
///   tree includes, so nearly everything is rebuilt for every case.
/// * everything else — the worst measured is `mutation_rounds_selection`, 21
///   cargo cases in 4h16m, about 12.2 minutes each, because its cases drive
///   whole gate scripts rather than a crate. The `go` and `pytest` runners are
///   carried by this same row: neither has a job in the measured window, and
///   the honest default for an unmeasured runner is the worst measured one.
const COST: [(&str, u32, u32); 2] = [("ctest", 100, 60), ("*", 15, 13)];

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

/// The cases-per-job the gate hands each runner, read out of its own `case`
/// arms: `ctest) printf '3\n' ;;` and the `*)` default.
///
/// The gate is the one place this is decided — the workflow expands whatever
/// it prints — so it is also the one place to read it from.
fn cases_per_job(gate: &str) -> BTreeMap<String, u32> {
    let mut found = BTreeMap::new();
    let mut inside = false;
    for line in gate.lines() {
        if line.starts_with("mutation_rounds_cases_per_job()") {
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
        // `ctest) printf '3\n' ;;` — the number between the quotes.
        let Some(rest) = body.trim().strip_prefix("printf '") else {
            continue;
        };
        let Some((digits, _)) = rest.split_once('\\') else {
            continue;
        };
        if let Ok(value) = digits.parse() {
            found.insert(label.trim().to_string(), value);
        }
    }
    found
}

#[test]
fn no_job_is_scheduled_with_more_cases_than_it_can_finish() {
    let root = repo_root();
    let workflow_path = root.join(".github/workflows/mutation-rounds.yml");
    let gate_path = root.join("scripts/gates/mutation-rounds.sh");
    let workflow = std::fs::read_to_string(&workflow_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", workflow_path.display()));
    let gate = std::fs::read_to_string(&gate_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", gate_path.display()));

    let ceiling = lane_timeout_minutes(&workflow);
    let per_job = cases_per_job(&gate);

    // A floor before any arithmetic. A parse that stopped finding the `case`
    // arms would leave an empty map, and an empty map satisfies every
    // inequality below without reading a single number — the shape where a
    // scan reports green because it found the file and not the thing.
    assert!(
        per_job.contains_key("ctest") && per_job.contains_key("*"),
        "read {per_job:?} out of `mutation_rounds_cases_per_job` in {}. The \
         function moved or its arms were respelled, and this test cannot check \
         an arithmetic whose terms it did not find",
        gate_path.display()
    );

    for (runner, setup, per_case) in COST {
        let cases = per_job
            .get(runner)
            .copied()
            .unwrap_or_else(|| panic!("no cases-per-job for `{runner}` in {per_job:?}"));
        let cost = setup + cases * per_case;
        assert!(
            cost <= ceiling,
            "a full `{runner}` job holds {cases} case(s) and is measured to cost \
             {setup} + {cases}x{per_case} = {cost} minutes against the lane's \
             {ceiling}-minute ceiling. A job that crosses it is `cancelled`, \
             which is neither green nor red: eight rounds of dispatch \
             32803356117 ended exactly that way and said nothing about any \
             case. Lower the cases-per-job for `{runner}` in {}; raising \
             `timeout-minutes` in {} only spends the 30-minute margin that \
             makes the failure arrive with this lane's name on it.",
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
    let gate = std::fs::read_to_string(root.join("scripts/gates/mutation-rounds.sh"))
        .expect("read the mutation-rounds gate");
    let per_job = cases_per_job(&gate);
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
/// minutes this file already measured for the runner. The harness caps one
/// declaration at an hour so that a typo cannot reach a day on its own; this is
/// the check that a declaration under that cap still fits the job it lands in,
/// and its remedy is to lower the bound or split the casefile, as the cases per
/// job are lowered for the same reason above.
#[test]
fn a_declared_bound_fits_the_job_it_is_scheduled_into() {
    let root = repo_root();
    let workflow = std::fs::read_to_string(root.join(".github/workflows/mutation-rounds.yml"))
        .expect("read the mutation-rounds workflow");
    let gate = std::fs::read_to_string(root.join("scripts/gates/mutation-rounds.sh"))
        .expect("read the mutation-rounds gate");
    let ceiling = lane_timeout_minutes(&workflow);
    let per_job = cases_per_job(&gate);
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
        let (_, setup, per_case) = COST
            .iter()
            .find(|(runner, _, _)| *runner == declared.runner)
            .or_else(|| COST.iter().find(|(runner, _, _)| *runner == "*"))
            .expect("the cost table has a default row");
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
