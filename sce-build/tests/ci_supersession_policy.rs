// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A lane that cannot finish between two pushes must not be superseded by one.
//!
//! `cancel-in-progress: true` is the right default for a lane that answers in
//! a minute: the run it kills is re-taken by the run that killed it, seconds
//! later. It is the wrong setting for a lane that needs longer than the gap
//! between pushes, because then the superseding run is killed in its turn and
//! the lane reports only when the person stops pushing. `cpp-suite.yml` names
//! the shape: *"a lane whose verdict depends on when a person stops typing is
//! not measuring the tree, and a lane cancelled three times in four reads
//! exactly like a lane that passes."*
//!
//! ## The threshold is measured, not chosen
//!
//! Two numbers decide it, and both are re-derivable:
//!
//! ```sh
//! # 1. how often main is pushed  -> median 18.9 min (n=39 gaps,
//! #    2026-08-28T06:45 .. 2026-08-29T16:52; 27 of 39 gaps under 30 min)
//! git log --format='%H %cI' -40 main
//!
//! # 2. how long each lane takes, and how often it is cancelled, over ITS OWN
//! #    last 25 runs -- not a global window, see the warning below
//! for f in .github/workflows/*.yml; do
//!     gh run list --workflow="$(basename "$f")" --limit 25 \
//!         --json conclusion,createdAt,updatedAt
//! done
//! ```
//!
//! A lane whose median successful run exceeds the median push gap cannot be
//! expected to finish between two pushes, so for that lane supersession is not
//! deferral. Measured 2026-08-29 the two populations separate with room to
//! spare: four lanes at 22.6, 28.3, 32.4 and 58.7 minutes, and every other lane
//! in the directory at 13.8 minutes or less. [`LANES`] carries every figure.
//!
//! The cancellation counts are the CONFIRMING evidence rather than the rule:
//! the three long lanes with wide or absent `paths:` filters lost 44%, 52% and
//! 48% of their last 25 runs to cancellation, while no lane under 14 minutes
//! lost more than 20%. It is not the rule because a narrow filter hides the
//! defect rather than fixing it -- `ecma262-lowered-cpp.yml` shows 2 of 11,
//! which looks like a short lane's rate, and both of those were mid-session
//! pushes 17 and 14 minutes behind a run that needed 22.6.
//!
//! ⚠ **Take the numbers from a global run window and they say something else.**
//! Read first from `gh run list --limit 300`, the same lanes reported 0
//! cancellations for everything under 20 minutes, which made "ever cancelled"
//! look like a clean discriminator. It is not: over each lane's own history
//! even a 0.4-minute lane has been cancelled three times. A global window is
//! dominated by whichever lanes ran most recently and under-samples the rest,
//! so it is the per-workflow query above that this table is built from.
//!
//! ## What `false` does and does not buy
//!
//! With `cancel-in-progress: false` a newly queued run still cancels a PENDING
//! one -- only the run IN FLIGHT and the LATEST commit survive. That is the
//! correct trade here and not a shortfall: these lanes re-ask a fixed
//! population against whatever the tree now holds, so their answer is about a
//! BRANCH and an intermediate commit's verdict is deferred into the next run.
//!
//! A lane whose answer is about a COMMIT needs the stronger fix, a per-commit
//! concurrency key, because nothing re-asks its question afterwards.
//! `mutation-rounds.yml` is that lane and `mutation_round_survives_the_next_push`
//! holds its key; it is short (2.6 min median) so this file's rule is satisfied
//! by its `true`, and the two properties are independent.
//!
//! ## Why the table lists every workflow
//!
//! A policy that names only the lanes it repairs cannot tell a lane it has
//! never heard of from one it approved. The completeness case below compares
//! the table against the directory in BOTH directions, so a new workflow is red
//! until somebody measures it -- unclassified is not a pass.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The median gap between pushes to `main`, in minutes.
///
/// A lane slower than this cannot finish between two of them. Re-derive with
/// the `git log` command in this module's docs.
const MEDIAN_PUSH_GAP_MINUTES: f64 = 18.9;

/// One row per workflow in `.github/workflows/`, as MEASURED.
///
/// `(file, median successful run in minutes, runs cancelled, successful runs
/// the median was taken over)`. The window asked for is each lane's own last
/// 25 runs; a lane younger than that reports all it has —
/// `ecma262-lowered-cpp.yml` had 11, which is why the counts are read
/// alongside the sample size and not as a ratio out of 25.
///
/// The supersession policy is DERIVED from the median rather than written down
/// beside it, so a row cannot record one thing and demand another. Editing a
/// number is therefore the only way to move a lane, which is a visible act;
/// flipping the flag in the workflow is not, and that is what this file exists
/// to catch.
const LANES: &[(&str, f64, u32, u32)] = &[
    ("clang-format-check.yml", 0.6, 0, 25),
    ("clippy-check.yml", 1.4, 2, 21),
    // 12 of its last 25 cancelled, but that window STRADDLES its own repair
    // (`74f83197b7`, 2026-08-29T00:53) -- the count is pre-fix history and is
    // recorded here as such. The median is what this table reads.
    ("cpp-suite.yml", 58.7, 12, 12),
    ("deploy-visualizer.yml", 11.1, 2, 23),
    ("doc-check.yml", 0.6, 0, 24),
    ("doc-content-gate.yml", 2.5, 4, 21),
    ("drift-verify.yml", 3.7, 4, 21),
    ("ecma262-lowered-cpp.yml", 22.6, 2, 5),
    // ⚠ NO HOSTED HISTORY OF ITS OWN — the lane landed 2026-08-30 and the `0`
    // in the last column says so rather than hiding it.
    //
    // ⚠⚠ So the number is a CONSERVATIVE UPPER BOUND, and the direction
    // matters: `must_not_supersede` only fires ABOVE the push gap, so an
    // optimistic median is the permissive reading. A local wall clock is the
    // wrong stand-in for exactly that reason — this row first carried 1.4 from
    // `scripts/gate --measure` on the build machine, where the whole gate runs
    // in 11 seconds from clean, which says nothing about a cold hosted runner.
    //
    // What it carries instead is DERIVED FROM HOSTED RUNS, of the nearest lane
    // that exists: the `Kotlin W3C Tests` job of `w3c-tests.yml`, measured
    // 2026-08-30 over its last three successes at 3.4, 4.8 and 3.4 minutes.
    // That job is a strict SUPERSET of this one's work — it builds the same
    // Kotlin runtime and the same Lua JNI library, plus QuickJS and Rhino,
    // then runs 4 x 373 cases where this lane runs 98 — so its worst reading
    // bounds this lane from above. 4.8 is that reading.
    //
    //   gh run list --workflow=w3c-tests.yml --limit 3 --json databaseId
    //   gh run view <id> --json jobs      # startedAt/completedAt of that job
    //
    // Replace it with this lane's own median once it has runs to take one
    // over, and move the row if it grew past the push gap.
    ("ecma262-lowered-kotlin.yml", 4.8, 0, 0),
    // The nearest lane below the line, and the one to re-measure first: 13.8
    // minutes against an 18.9 minute gap, on only 12 successful runs.
    ("embed-vendor-smoke.yml", 13.8, 4, 12),
    ("example-codegen.yml", 1.2, 3, 22),
    ("fmt-check.yml", 0.4, 3, 22),
    ("forge-conformance.yml", 8.9, 5, 20),
    ("http-endpoint-ssot.yml", 0.2, 0, 25),
    ("license-verify.yml", 0.3, 0, 3),
    ("mutation-rounds.yml", 2.6, 0, 19),
    ("regen-reproduces.yml", 7.0, 3, 21),
    ("rust-workspace-tests.yml", 32.4, 11, 12),
    ("sce-forge-codec-clippy.yml", 1.4, 1, 24),
    ("sce-forge-codec-no-alloc.yml", 1.4, 1, 24),
    ("sce-rust-runtime-no-std.yml", 1.6, 0, 25),
    ("spec-citations.yml", 0.9, 1, 22),
    ("spec-snapshot-drift.yml", 0.3, 0, 24),
    // Re-measured 2026-09-02 and MOVED: 9.9 -> 20.1, cancellations 0 -> 12.
    // The row it replaces was true when it was taken -- bucket that same
    // query by day and 2026-08-28 and 2026-08-29 still median 9.9 and 10.2.
    //
    // What grew is not the work. The `tree-wide gates` job's own successful
    // compute is 8.5..12.4 min, median 11.1, unchanged. The extra ten minutes
    // are QUEUE, and this row measures `createdAt -> updatedAt` because that
    // is the right clock for the question: a run waiting for a runner is
    // exactly the run the next push takes away.
    //
    // The confirming count here is not a ratio, it is unanimous. All 12 of
    // those cancellations killed a job that had ALREADY STARTED, 68s to 1264s
    // of work in -- so `false`, which saves the run in flight, is the setting
    // every one of them needed. Ten are consecutive: every push from
    // `b59ed99b10` (04:28Z) through `9a16e970cf` (05:56Z), among them
    // `06ab3dcf44`, the commit repairing a red THIS job had raised at
    // `55620099a7`. The lane stopped answering at the moment its answer
    // mattered, and stayed stopped.
    //
    // Split the job verdicts out rather than reading the run conclusion --
    // check runs from other workflows land on this run and are not this lane:
    //
    //   gh run list --workflow=tree-hygiene.yml --limit 25 --json databaseId
    //   gh run view <id> --json jobs   # select .name == "tree-wide gates"
    //
    // The reason the drift ran for four days: this lane is where
    // `ci_supersession_policy` runs, so a stale row here silences the only
    // check that would move it.
    ("tree-hygiene.yml", 20.1, 12, 8),
    ("w3c-tests.yml", 28.3, 13, 11),
];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn workflow_dir() -> PathBuf {
    repo_root().join(".github/workflows")
}

/// Every `.yml` in the workflow directory, by file name.
fn workflows_on_disk() -> BTreeSet<String> {
    let dir = workflow_dir();
    let entries =
        std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));
    entries
        .map(|e| {
            e.expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|n| n.ends_with(".yml") || n.ends_with(".yaml"))
        .collect()
}

/// The `cancel-in-progress:` value of the TOP-LEVEL `concurrency:` block.
///
/// Read as text rather than through a YAML parser, for the reason
/// `mutation_round_survives_the_next_push` gives: what matters is the literal
/// the file carries. Only a block starting at column zero is read, so a
/// job-level `concurrency:` cannot answer for the workflow.
///
/// Returns `None` when the file declares no such block -- which the caller must
/// treat as a failure and not as a default, because a workflow with no
/// `concurrency:` at all supersedes nothing and queues nothing, and that is a
/// third state this table does not describe.
fn top_level_cancel_in_progress(workflow: &str) -> Option<bool> {
    let mut lines = workflow.lines();
    while let Some(line) = lines.next() {
        if line.trim_end() != "concurrency:" {
            continue;
        }
        for body in lines.by_ref() {
            let trimmed = body.trim_start();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                continue;
            }
            // Left the block without meeting the key.
            if !body.starts_with(' ') {
                return None;
            }
            if let Some(value) = trimmed.strip_prefix("cancel-in-progress:") {
                return match value.trim() {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => None,
                };
            }
        }
        return None;
    }
    None
}

fn read_workflow(name: &str) -> String {
    let path = workflow_dir().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Whether this lane is long enough that supersession loses its verdict.
fn must_not_supersede(median_minutes: f64) -> bool {
    median_minutes > MEDIAN_PUSH_GAP_MINUTES
}

#[test]
fn every_workflow_is_classified() {
    let on_disk = workflows_on_disk();
    let in_table: BTreeSet<String> = LANES.iter().map(|(f, ..)| (*f).to_owned()).collect();

    // Lower bound before either comparison: an empty directory read, or a
    // table that lost its rows, would satisfy a subset check in one direction
    // and prove nothing.
    assert!(
        on_disk.len() >= 20,
        "only {} workflow file(s) found under {} -- the directory moved or the \
         read failed, and this case cannot say anything about files it did not see",
        on_disk.len(),
        workflow_dir().display()
    );

    let unclassified: Vec<&String> = on_disk.difference(&in_table).collect();
    assert!(
        unclassified.is_empty(),
        "workflow(s) that no row in `LANES` measures: {unclassified:?}\n\
         A supersession policy that has not heard of a lane cannot tell it from \
         one it approved, so an unclassified lane is red and not a pass. Measure \
         it -- the two commands are in this module's docs -- and add its row.",
    );

    let stale: Vec<&String> = in_table.difference(&on_disk).collect();
    assert!(
        stale.is_empty(),
        "`LANES` names workflow(s) that are not in {}: {stale:?}\n\
         A row for a deleted lane is a measurement nothing can contradict.",
        workflow_dir().display()
    );
}

#[test]
fn a_lane_slower_than_the_push_gap_is_not_superseded() {
    let mut long = 0;
    let mut short = 0;

    for &(file, median, cancelled, successes) in LANES {
        let workflow = read_workflow(file);
        let declared = top_level_cancel_in_progress(&workflow);

        let Some(declared) = declared else {
            panic!(
                "{file} declares no top-level `concurrency:` with a \
                 `cancel-in-progress:` literal. That is a third state this \
                 table does not describe -- the lane neither supersedes nor \
                 queues -- so it cannot be judged against a measurement. Give \
                 it a `concurrency:` block."
            );
        };

        if must_not_supersede(median) {
            long += 1;
            assert!(
                !declared,
                "{file} runs {median} min on a branch pushed every \
                 {MEDIAN_PUSH_GAP_MINUTES} min, so a push that arrives while it \
                 is running kills a verdict that will not be re-taken before the \
                 next one arrives -- and it still declares \
                 `cancel-in-progress: true`. Measured: {cancelled} cancellation(s) \
                 in its recent history, median over {successes} successes. Set \
                 `cancel-in-progress: false`, or re-measure and move the row."
            );
        } else {
            short += 1;
            assert!(
                declared,
                "{file} runs {median} min, comfortably inside the \
                 {MEDIAN_PUSH_GAP_MINUTES} min push gap, so a superseded run is \
                 re-taken by the run that superseded it -- and it declares \
                 `cancel-in-progress: false`, which queues runs nothing needed \
                 queued. If the lane has grown, re-measure it and update its row \
                 in `LANES`; the table is what decides this, not the file."
            );
        }
    }

    // Both populations have to be non-empty or one of the two branches above
    // was never taken and this case measured half of what it claims.
    assert!(
        long >= 1,
        "no lane in `LANES` is slower than the push gap, so the assertion this \
         file exists for was never evaluated"
    );
    assert!(
        short >= 1,
        "every lane in `LANES` is slower than the push gap, so the short-lane \
         assertion was never evaluated"
    );
}
