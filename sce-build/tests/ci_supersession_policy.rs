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
//! Every number below is re-derivable, and the case named
//! `the_recipe_names_every_field_its_columns_need` holds these commands to the
//! columns they have to fill — a recipe that cannot produce a column is how a
//! table stops being a measurement.
//!
//! ```sh
//! # 1. how often `main` is actually PUSHED -> median 17.6 min (26 gaps over 27
//! #    pushes, 2026-08-30T14:44 .. 2026-09-02T08:46, read 2026-09-02T09:24Z).
//! #    Derive it from the distinct SHAs runs were created for, NOT from
//! #    `git log`: several commits ride one push, so counting COMMIT gaps
//! #    invents gaps no push ever made. Measured 2026-09-02 over one window,
//! #    `git log -40 main` says 21.0 and the pushed SHAs say 17.6 -- the three
//! #    commits at 2026-08-30T23:43 alone contribute two 0.3-minute "gaps".
//! gh run list --limit 300 --json createdAt,headSha,headBranch,event
//! #    keep event=="push" && headBranch=="main", take each SHA's EARLIEST
//! #    createdAt, then the median of adjacent differences
//!
//! # 2. every column of [`LANES`] except the file name, over ITS OWN last 25
//! #    runs -- not a global window, see the warning below.
//! #      cancelled  = conclusion == "cancelled"
//! #      successes  = conclusion == "success"
//! #      median     = successes only, over updatedAt - createdAt
//! #      unfinished = conclusion is null AND status is queued|pending|in_progress
//! #    The last one is why the field list carries `status`: a run that never
//! #    started and one that was skipped both report a null conclusion.
//! for f in .github/workflows/*.yml; do
//!     gh run list --workflow="$(basename "$f")" --limit 25 \
//!         --json conclusion,status,createdAt,updatedAt
//! done
//!
//! # 3. how long each lane goes WITHOUT ANSWERING -- the longest run of
//! #    consecutive pushes for which no run of that lane reached `success` or
//! #    `failure`. Sort the same listing by `createdAt` and take the longest
//! #    streak of runs that are neither. This is the column that separates a
//! #    lane losing verdicts from one deferring them; the cancellation count
//! #    does not, see the warning below.
//! ```
//!
//! A lane whose median successful run exceeds the median push gap cannot be
//! expected to finish between two pushes, so for that lane supersession is not
//! deferral. Measured 2026-09-02 the two populations still separate, but less
//! comfortably than they did that morning: eight lanes from 17.9 to 79.1
//! minutes, and the next one down at 15.2. [`LANES`] carries every figure.
//!
//! ⚠ The eighth arrived by the table's own instruction. `deploy-visualizer.yml`
//! carried a comment naming itself "the one to re-measure first" at 15.9; it
//! was re-measured a few hours later and read 17.9, which is past the line.
//! A row that says what would move it is the only kind that gets moved.
//!
//! ⚠ **A cancellation COUNT is not evidence of the defect, and must not be read
//! as one.** `cancel-in-progress: false` still cancels a PENDING run, so a
//! repaired lane goes on reporting cancellations forever. Measured 2026-09-02,
//! `cpp-suite.yml` -- repaired at `74f83197b7` and long since declaring `false`
//! -- shows 12 cancellations in its last 25 runs, and its eight newest
//! cancelled NO JOB AT ALL: every one was still entirely pending. That is the
//! setting working, not failing.
//!
//! What that reading separates is `true` from `false`, per job rather than per
//! run:
//!
//! ```sh
//! gh run view <id> --json jobs   # a cancelled job with startedAt < completedAt
//! ```
//!
//! ⚠⚠ **And that is ALL it separates -- an earlier version of this paragraph
//! offered it as the discriminator for whether a lane is losing answers, which
//! it is not.** Probed across the whole directory on 2026-09-02, every
//! cancellation of every lane declaring `true` killed started work, down to
//! `sce-forge-codec-clippy.yml` at 1.4 minutes; every cancellation of a lane
//! declaring `false` killed none. Of course: killing the run in flight is what
//! `true` MEANS. A reading that is a restatement of the setting cannot be
//! evidence about the setting.
//!
//! What does discriminate is how long the lane goes without answering --
//! consecutive pushes for which no run reached `success` or `failure`.
//! Measured over each lane's own last 25 runs: `deploy-visualizer` 1,
//! `rust-workspace-tests` 1, `forge-conformance` 2, `regen-reproduces` 5 over
//! 42 min, `cpp-suite` 5, `tree-hygiene` 10, `w3c-tests` 12 over 317 min,
//! `mutation-rounds` 14 over 342 min.
//!
//! ⚠⚠⚠ Read that column and the doctrine below stops being sufficient. The
//! four worst lanes are all already repaired, three by `false` and one by a
//! per-commit key. `false` protects the run IN FLIGHT, and under a saturated
//! runner pool no run reaches flight to be protected -- `w3c-tests`'s eight
//! newest cancellations were all still pending. The lever this file asserts is
//! set correctly on every one of them and no longer controls the outcome. What
//! remains is runners, which no assertion here can buy.
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
//! holds its key.
//!
//! ⚠ That key also settles THIS file's question, and the rule reads it. A group
//! carrying `github.sha` puts every push in a group of its own, so a later push
//! cannot supersede the run whatever `cancel-in-progress` says. A long lane
//! therefore passes on `false` OR on a per-commit group -- a disjunction, not
//! an exemption list, because it is read off the mechanism in the file rather
//! than from a roster of names. Reading only the flag was a defect: it judged
//! `mutation-rounds.yml` by a property that lane does not depend on, and the
//! judgement happened to agree only while its median sat below the gap.
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
/// command 1 in this module's docs -- from PUSHED SHAs, not from `git log`.
///
/// ⚠ This one number classifies every row in [`LANES`], so moving it moves the
/// whole table at once and is never a local edit. It carried 18.9 until
/// 2026-09-02, taken over 2026-08-28/29 by a `git log` recipe that counted
/// commit gaps; re-derived from pushes over one window it is 17.6. Re-take it
/// and the rows TOGETHER, from a single window, or the table ends up comparing
/// lanes measured in one week against a threshold measured in another.
///
/// ⚠⚠ The `--limit` in that recipe is PART of the measurement, not an
/// incidental of it. Re-derived 2026-09-02T09:35Z from the one command at
/// three widths: 12.8 over 20 pushes at `--limit 200`, 17.6 over 27 at 300,
/// and 21.8 over 33 at 400. A wider window reaches back into quieter
/// stretches and the figure climbs with it, so a re-deriver who reads a
/// different number should check the width before concluding this constant
/// has drifted. That conclusion was reached once already, off a recipe that
/// had changed what it measured without saying so.
///
/// ⚠⚠⚠ Which direction that error runs in is the part to keep:
/// [`must_not_supersede`] fires only ABOVE this number, so a LARGER gap
/// classifies fewer lanes as long and is the PERMISSIVE reading. Widening the
/// window loosens the rule, and a re-derivation that widens it has to say so.
const MEDIAN_PUSH_GAP_MINUTES: f64 = 17.6;

/// One row per workflow in `.github/workflows/`, as MEASURED.
///
/// `(file, median successful run in minutes, runs cancelled, successful runs
/// the median was taken over, runs that never finished)`. The window asked for
/// is each lane's own last 25 runs; a lane younger than that reports all it
/// has — `ecma262-lowered-kotlin.yml` has 6, which is why the counts are read
/// alongside the sample size and not as a ratio out of 25.
///
/// ⚠⚠⚠ The fifth column arrived 2026-09-02, and what it replaced was a hole
/// rather than a number. The table recorded cancellations and successes, so a
/// run that did NEITHER — still queued, never started — was in no column at
/// all. `mutation-rounds.yml` is where that mattered: its `0` cancellations
/// read as a healthy lane while FOURTEEN of its last twenty-five runs sat
/// unstarted, the oldest for 5.7 hours. A per-commit group cannot record a
/// supersession by construction, so its cancellation column could never have
/// been anything but zero, and a column that cannot move is not evidence.
///
/// The remaining slack — `25 - (cancelled + successes + unfinished)` — is
/// verdicts of other kinds, a failure or a timeout. Those are ANSWERS, which
/// is why they are the one bucket this table can leave implicit: every run is
/// cancelled, unfinished, or answered.
///
/// The supersession policy is DERIVED from the median rather than written down
/// beside it, so a row cannot record one thing and demand another. Editing a
/// number is therefore the only way to move a lane, which is a visible act;
/// flipping the flag in the workflow is not, and that is what this file exists
/// to catch.
const LANES: &[(&str, f64, u32, u32, u32)] = &[
    ("clang-format-check.yml", 0.8, 0, 25, 0),
    ("clippy-check.yml", 7.7, 0, 22, 1),
    // 12 of its last 25 cancelled while declaring `false`, and that is the
    // setting WORKING: probed 2026-09-02, its eight newest cancellations
    // killed no job at all -- every one was still entirely pending. Its repair
    // (`74f83197b7`, 2026-08-29T00:53) sits BEFORE this window, so the count is
    // post-fix history.
    ("cpp-suite.yml", 79.1, 12, 11, 2),
    // ⚠⚠⚠ CROSSED THE LINE, 15.9 -> 17.9 against a 17.6 gap. The row it
    // replaces named itself "the one to re-measure first"; re-measured on
    // instruction 2026-09-02T10:2xZ, it had moved past the threshold, and the
    // rule now requires the `false` this lane carries.
    //
    // ⚠ Its OUTCOME did not deteriorate with it: 22 successes in the window
    // and a longest run of consecutive pushes without an answer of ONE. It is
    // healthier than every repaired lane in this table, and the reason is its
    // `paths:` filter -- most pushes never reach it, so the gap it actually
    // faces is wider than the one this table compares it against.
    //
    // That mismatch is deliberate and its DIRECTION is the argument: the
    // pushes reaching a filtered lane are a SUBSET of all pushes, and the gaps
    // between a subset are never smaller. So [`MEDIAN_PUSH_GAP_MINUTES`] is a
    // conservative stand-in for every filtered lane -- it can demand `false`
    // where it was not needed, never withhold it where it was. Queueing a
    // deploy nothing needed queued is the price of that direction, and it is
    // the cheap side.
    ("deploy-visualizer.yml", 17.9, 2, 22, 1),
    ("doc-check.yml", 0.6, 0, 24, 0),
    ("doc-content-gate.yml", 11.4, 1, 24, 0),
    ("drift-verify.yml", 7.2, 1, 23, 1),
    ("ecma262-lowered-cpp.yml", 23.7, 2, 16, 1),
    // Now measured from ITS OWN hosted runs. This row carried 4.8 borrowed
    // from the `Kotlin W3C Tests` job of `w3c-tests.yml` -- a strict superset
    // of its work -- because the lane had landed 2026-08-30 with no history to
    // take a median over, and the borrowing was recorded here with the
    // instruction to replace it once it had. It has six successes now, median
    // 6.3, so the stand-in is retired.
    //
    // The borrowed bound held: 6.3 is above the 4.8 it stood in for and still
    // far below the gap, so the substitution never changed a classification.
    ("ecma262-lowered-kotlin.yml", 6.3, 0, 6, 0),
    ("embed-vendor-smoke.yml", 15.2, 4, 15, 0),
    ("example-codegen.yml", 8.5, 1, 23, 1),
    ("fmt-check.yml", 8.2, 0, 24, 1),
    // Moved 8.9 -> 22.2 -> 25.5 over one day. Its cancellations killed work
    // rather than a queue slot, which is what `false` is for: `Rust Forge
    // Conformance` was 44.9 min in when the push at 15:29Z took it, and `Build
    // sce-codegen` 20.9 min in at 15:50Z. This lane fans out to five
    // languages, so one supersession discards all five.
    //
    // ⚠ A calmer day does not bring this row back down: measured at 22.2, the
    // six runs of the quietest day median 22.15 -- the same figure as the full
    // window. The heavy tail moved and the middle did not, so moving this row
    // needs a faster lane, not a quieter day.
    ("forge-conformance.yml", 25.5, 4, 19, 2),
    ("http-endpoint-ssot.yml", 6.0, 7, 17, 1),
    ("license-verify.yml", 0.3, 0, 3, 0),
    // ⚠⚠⚠ This row is why the fifth column exists. It is long, declares
    // `true`, and is nonetheless right -- its concurrency group carries
    // `github.sha`, so a later push lands in a different group and cannot
    // supersede it at all, which `group_is_per_commit` reads.
    //
    // ⚠⚠⚠ But its `0` cancellations were being read as health, and they are
    // not a measurement of health at all: a per-commit group CANNOT record a
    // supersession, so that column can never be anything but zero here. What
    // the zero was hiding is the fifth column -- 14 of its last 25 runs had
    // still not started, the oldest queued 5.7 hours. Nothing cancels them and
    // nothing runs them. The lever was traded, not the loss.
    ("mutation-rounds.yml", 49.8, 0, 10, 14),
    // ⚠ REFUTED, and the row it replaces said the opposite. That row called
    // this lane "the standing evidence that the typical case is not the whole
    // distribution" and implied a repair was owed. Measured 2026-09-02 over
    // its own last 25 runs, the longest run of consecutive pushes leaving it
    // without an answer is FIVE, spanning 42 minutes -- mid-pack, and better
    // than `cpp-suite` (5 / 38 min), `tree-hygiene` (10 / 87 min), `w3c-tests`
    // (12 / 317 min) and `mutation-rounds` (14 / 342 min), every one of which
    // is already repaired. Its deferral works: it answers, later.
    //
    // What made the old claim look strong was the count -- 12 cancellations,
    // all killing started work. That is not evidence: probed across the whole
    // directory, EVERY cancellation of a lane declaring `true` killed started
    // work, because that is what `true` means. The discriminator separates
    // `true` from `false`; it does not separate a lane that loses answers from
    // one that defers them. Consecutive silence does, and is what to measure.
    ("regen-reproduces.yml", 13.9, 12, 12, 1),
    ("rust-workspace-tests.yml", 41.6, 0, 17, 1),
    ("sce-forge-codec-clippy.yml", 1.4, 1, 24, 0),
    ("sce-forge-codec-no-alloc.yml", 1.1, 1, 24, 0),
    ("sce-rust-runtime-no-std.yml", 1.6, 0, 25, 0),
    ("spec-citations.yml", 9.0, 1, 23, 1),
    ("spec-snapshot-drift.yml", 0.3, 0, 24, 0),
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
    // The confirming count here is not a ratio, it is unanimous. All of those
    // cancellations killed a job that had ALREADY STARTED, 68s to 1264s of
    // work in -- so `false`, which saves the run in flight, is the setting
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
    // Re-taken in the common window at 20.5 with 11 cancellations over 9
    // successes: the repair pushed at 08:46Z added a success and slid the
    // oldest cancellation out of the 25-run window. Still long, still `false`.
    ("tree-hygiene.yml", 20.5, 11, 9, 1),
    // ⚠ `false` and still the second-worst answer rate in the table: 9
    // successes, and a longest consecutive silence of 12 pushes over 317
    // minutes. Its eight newest cancellations killed NO started work -- every
    // one was cancelled while still entirely pending. That is not the setting
    // failing to protect a run in flight; it is no run ever reaching flight.
    // `false` protects the run that started, and under a saturated runner pool
    // there is none to protect.
    ("w3c-tests.yml", 70.0, 14, 9, 2),
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

/// The `group:` expression of the TOP-LEVEL `concurrency:` block.
///
/// Read as text, and only from a block starting at column zero, for the same
/// reasons [`top_level_cancel_in_progress`] is. Returns `None` when the block
/// declares no `group:`.
fn top_level_concurrency_group(workflow: &str) -> Option<String> {
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
            if let Some(value) = trimmed.strip_prefix("group:") {
                return Some(value.trim().to_owned());
            }
        }
        return None;
    }
    None
}

/// Whether a lane's concurrency group changes with every commit.
///
/// A group keyed on `github.sha` puts each push in a group of its OWN, so a
/// later push cannot supersede this run whatever `cancel-in-progress` says --
/// the two runs are never in the same group to begin with.
///
/// This is not an exemption from the property this file asserts, it is the
/// STRONGER way of satisfying it, and the distinction matters: an exemption
/// list would let a lane escape by being named, while this reads the mechanism
/// off the file and is wrong only if the mechanism is absent.
fn group_is_per_commit(workflow: &str) -> bool {
    top_level_concurrency_group(workflow).is_some_and(|g| g.contains("github.sha"))
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

/// A row cannot report more outcomes than its window holds.
///
/// The window the docs prescribe is each lane's OWN last 25 runs, so the three
/// outcome columns summing above 25 is arithmetically impossible for that
/// query however plausible the duration looks. Nothing offline can tell a
/// stale row from a fresh one, but this does catch the shape a hand-edited or
/// half-updated row takes: one column re-taken and the others left behind.
///
/// ⚠ The sum includes `unfinished`, and has to. While the row was two columns
/// wide the check passed on `mutation-rounds.yml` reporting 11 of 25 outcomes,
/// because the fourteen it did not report were in no column to be counted.
#[test]
fn no_row_counts_more_runs_than_the_window_holds() {
    const WINDOW: u32 = 25;
    let mut checked = 0;

    for &(file, _, cancelled, successes, unfinished) in LANES {
        assert!(
            cancelled + successes + unfinished <= WINDOW,
            "{file} reports {cancelled} cancellation(s), {successes} \
             success(es) and {unfinished} unfinished -- {} outcomes out of a \
             {WINDOW}-run window. The columns came from different readings.",
            cancelled + successes + unfinished
        );
        checked += 1;
    }

    assert!(
        checked >= 20,
        "only {checked} row(s) examined -- `LANES` has lost its rows, and this \
         case passes trivially on an empty table"
    );
}

/// The rule, plus the invariant tying a backlog to the mechanism that makes one.
///
/// A lane slower than the push gap must not be superseded. Separately, a row
/// reporting more unfinished runs than successes must sit on a per-commit
/// group, because that is the only arrangement under which unstarted runs
/// accumulate instead of being cleared by the next push.
///
/// ⚠ That invariant carries no minimum-population floor, deliberately, and the
/// asymmetry is the point. Every other floor in this file refuses a population
/// that has emptied, because an empty one means the check stopped looking. Here
/// an empty one means the QUEUE DRAINED — the good outcome — so a floor would
/// red the tree for improving. What keeps it honest instead is its mutation
/// case, which constructs a violating row rather than waiting for the hosted
/// queue to misbehave.
#[test]
fn a_lane_slower_than_the_push_gap_is_not_superseded() {
    let mut long = 0;
    let mut short = 0;
    // How each long lane satisfies the rule, counted apart so that the
    // per-commit arm cannot quietly become the only one that ever fires.
    let mut long_by_false = 0;
    let mut long_by_key = 0;

    for &(file, median, cancelled, successes, unfinished) in LANES {
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

        // The second arm: a row whose median was taken over fewer runs than
        // never finished is describing the minority that found a runner, not
        // the lane.
        //
        // ⚠ It deliberately does NOT feed the long/short classification, and
        // the reason is that doing so would be provably redundant rather than
        // merely unnecessary. The assertion below forces any such row onto a
        // per-commit group, and a per-commit group already satisfies the long
        // arm -- so the classification could not reach a different verdict.
        // An arm that cannot change an outcome is one no mutation can turn
        // red, and this file does not keep those.
        let median_is_unrepresentative = unfinished > successes;

        // A backlog that size is only reachable one way, and the two readings
        // have to agree about which way. Under a group shared across commits
        // the next push cancels whatever is still pending, so the unfinished
        // column stays at one or two -- measured 2026-09-02, every lane with a
        // shared group sat at 0..2. Only a per-commit group lets runs pile up,
        // because nothing ever clears them. A row claiming a backlog on a
        // shared group therefore contradicts its own workflow, and one of the
        // two was read wrong.
        assert!(
            !median_is_unrepresentative || group_is_per_commit(&workflow),
            "{file} reports {unfinished} run(s) that never finished against \
             {successes} success(es), a backlog only a per-commit concurrency \
             group can accumulate -- but its group is shared across commits, \
             where a later push clears what is still pending. The row and the \
             workflow disagree; re-read whichever was taken longer ago."
        );

        if must_not_supersede(median) {
            long += 1;
            let per_commit = group_is_per_commit(&workflow);
            if per_commit {
                long_by_key += 1;
            } else if !declared {
                long_by_false += 1;
            }
            assert!(
                !declared || per_commit,
                "{file} runs {median} min on a branch pushed every \
                 {MEDIAN_PUSH_GAP_MINUTES} min, so a push that arrives while it \
                 is running kills a verdict that will not be re-taken before the \
                 next one arrives -- and it still declares \
                 `cancel-in-progress: true` under a group that is shared across \
                 commits. Measured: {cancelled} cancellation(s) and {unfinished} \
                 run(s) that never finished, median over {successes} successes. \
                 Set `cancel-in-progress: false`, or key the group on \
                 `github.sha` so a later push lands in a different group, or \
                 re-measure and move the row."
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

    // The per-commit arm is a way of PASSING the long-lane assertion, so a
    // reader that answered `true` for every file would satisfy it everywhere
    // and this case would assert nothing about `cancel-in-progress` at all.
    // Requiring one long lane to pass on the flag keeps that arm evaluated.
    assert!(
        long_by_false >= 1,
        "every long lane in `LANES` passes by having a per-commit concurrency \
         key ({long_by_key} of them), so the `cancel-in-progress: false` \
         requirement was never evaluated. Either the group reader has started \
         answering yes for everything, or the last lane relying on the flag \
         has gone -- both make this case vacuous."
    );
}

// ## The prose copies of these two numbers
//
// Both figures above are also written in English, in the workflow that each
// row is about and in two files that argue from them. Those copies are what a
// person reads when they open a lane and ask why it declares what it declares,
// and until 2026-09-02 nothing held them to the table: moving the constant
// from 18.9 to 17.6 left SEVEN copies behind, in five workflows, a design doc
// and `tools/git-hooks/gate_registry.py`, every one of them still naming a gap
// no measurement supported.
//
// A rationale nobody re-measures is worse than none, because it promises a
// derivation it no longer has. The three cases below give the copies an owner.

/// The phrase every prose copy of a measured minute figure is written in.
///
/// A claim is a NUMBER immediately in front of this phrase, so a mention of
/// the phrase without one -- this scanner's own source included -- is not a
/// claim and needs no exemption to be skipped.
const CLAIM_PHRASE: &str = "min median";

/// What separates a claim about the PUSH GAP from a claim about one lane.
const GAP_SUFFIX: &str = "gap between pushes";

/// How far a stated figure may sit from the measured one, in minutes.
///
/// The columns carry one decimal, so this admits a rounding difference and
/// nothing else.
const CLAIM_TOLERANCE: f64 = 0.05;

/// One prose statement of a measured minute figure.
struct Claim {
    minutes: f64,
    about_the_gap: bool,
}

/// Flatten a file to a single line so a claim wrapped across two lines still
/// reads as one sentence.
///
/// Leading comment markers go, and so do the three characters prose uses to
/// quote or emphasise: `*` for markdown bold, a backtick for a code span, and
/// `"` for a sentence split across adjacent string literals. Without that last
/// one the copy in `tools/git-hooks/gate_registry.py` -- whose sentence is
/// built from two literals on consecutive source lines -- reads as a lane
/// claim rather than the gap claim it is.
fn flatten(text: &str) -> String {
    let mut raw = String::new();
    for line in text.lines() {
        let mut trimmed = line.trim();
        for marker in ["///", "//", "#"] {
            if let Some(rest) = trimmed.strip_prefix(marker) {
                trimmed = rest.trim_start();
                break;
            }
        }
        raw.push_str(&trimmed.replace(['*', '`', '"'], " "));
        raw.push(' ');
    }
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Every stated minute figure in `text`, classified.
fn claims(text: &str) -> Vec<Claim> {
    let flat = flatten(text);
    let mut found = Vec::new();
    let mut from = 0;

    while let Some(at) = flat[from..].find(CLAIM_PHRASE) {
        let start = from + at;
        let token = flat[..start].trim_end().rsplit(' ').next().unwrap_or("");
        if let Ok(minutes) = token.parse::<f64>() {
            let after = flat[start + CLAIM_PHRASE.len()..].trim_start();
            found.push(Claim {
                minutes,
                about_the_gap: after.starts_with(GAP_SUFFIX),
            });
        }
        from = start + CLAIM_PHRASE.len();
    }

    found
}

/// Every file git tracks, as an absolute path.
fn tracked_files() -> Vec<PathBuf> {
    let root = repo_root();
    let out = std::process::Command::new("git")
        .args(["-C", &root.display().to_string(), "ls-files", "-z"])
        .output()
        .expect("git ls-files runs");
    assert!(out.status.success(), "git ls-files failed: {out:?}");
    out.stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| root.join(String::from_utf8_lossy(s).into_owned()))
        .collect()
}

/// Every `--json` field the recipe must ask for, and the column it fills.
///
/// A column is only measured if the command written above can produce it. That
/// stopped being true the moment `unfinished` was added: the recipe asked for
/// `conclusion` alone, and a run that never started reports the same null
/// conclusion as one that was skipped. The number went into the table and its
/// stated derivation could not have reached it — which is the exact defect
/// this file exists to refuse, committed by the round that added the column.
const RECIPE_FIELDS: &[(&str, &str)] = &[
    ("conclusion", "the `cancelled` and `successes` columns"),
    (
        "status",
        "the `unfinished` column, which a null conclusion cannot fill",
    ),
    ("createdAt", "the median's start, and the push-gap window"),
    ("updatedAt", "the median's end"),
    ("headSha", "the distinct pushes the gap is measured between"),
    ("headBranch", "restricting the gap to `main`"),
    ("event", "restricting the gap to pushes"),
];

/// The COMMANDS in the shell block of this module's own documentation.
///
/// Read from the file rather than from `include_str!` of a fragment, because
/// the thing under test is what a person opening this file actually reads.
///
/// ⚠ Shell comments are stripped, and the first version of this reader did not
/// strip them. Its mutation case survived because of it: removing `status`
/// from the `--json` list left the sentence *"`status` is in the field list
/// because ..."* standing three lines above, and the reader accepted the prose
/// as the field. A scanner that counts an explanation as the thing explained
/// certifies exactly the tree it was built to refuse.
fn recipe_block() -> String {
    let path = repo_root().join("sce-build/tests/ci_supersession_policy.rs");
    let src =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

    let mut inside = false;
    let mut block = String::new();
    for line in src.lines() {
        let Some(doc) = line.strip_prefix("//!") else {
            continue;
        };
        let doc = doc.strip_prefix(' ').unwrap_or(doc);
        if doc.trim() == "```sh" {
            inside = true;
            continue;
        }
        if inside && doc.trim() == "```" {
            break;
        }
        if inside {
            // Commands only. See this function's docs for what naming a field
            // in a comment was allowed to prove.
            if doc.trim_start().starts_with('#') {
                continue;
            }
            block.push_str(doc);
            block.push('\n');
        }
    }
    block
}

/// The recipe asks for every field the table's columns are derived from.
#[test]
fn the_recipe_names_every_field_its_columns_need() {
    let block = recipe_block();

    assert!(
        block.contains("--json"),
        "the module's shell block carries no `--json` invocation, so either \
         the recipe has gone or this reader stopped finding it -- and a reader \
         that finds nothing agrees that nothing is missing"
    );

    let mut missing = Vec::new();
    for &(field, fills) in RECIPE_FIELDS {
        if !block.contains(field) {
            missing.push(format!("  {field}: fills {fills}"));
        }
    }

    assert!(
        missing.is_empty(),
        "the recipe in this module's docs does not ask for field(s) the table \
         is built from:\n{}\n\
         A column whose stated derivation cannot produce it is not a \
         measurement. Add the field to the `gh run list --json` list above, or \
         drop the column it fills.",
        missing.join("\n")
    );
}

/// The rows this file classifies as unable to finish between two pushes.
fn long_lanes() -> Vec<(&'static str, f64)> {
    LANES
        .iter()
        .filter(|(_, median, _, _, _)| must_not_supersede(*median))
        .map(|(file, median, _, _, _)| (*file, *median))
        .collect()
}

/// No file states a push gap other than [`MEDIAN_PUSH_GAP_MINUTES`].
///
/// The scan is `git ls-files` rather than a list of the files that carry a
/// copy today, because the copy this exists to catch is the NEXT one, written
/// into a file that does not exist yet. A list of today's answers cannot name
/// it by construction.
///
/// ⚠ What this does NOT catch: a copy phrased some other way. The gate owns
/// the canonical sentence, so "a median gap of 17.6 min" evades it. That is
/// the limit of reading English as text, and it is stated here rather than
/// left for a reader to discover -- the answer to a copy in a new phrasing is
/// to rewrite it into this one, not to widen the scanner until it guesses.
#[test]
fn no_tracked_file_states_a_push_gap_other_than_the_constant() {
    let root = repo_root();
    let mut wrong = Vec::new();
    let mut seen = 0;

    for path in tracked_files() {
        // Not UTF-8 means no prose to read, which is not a failure.
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for claim in claims(&text).iter().filter(|c| c.about_the_gap) {
            seen += 1;
            if (claim.minutes - MEDIAN_PUSH_GAP_MINUTES).abs() > CLAIM_TOLERANCE {
                let shown = path.strip_prefix(&root).unwrap_or(&path);
                wrong.push(format!(
                    "  {}: states {} min",
                    shown.display(),
                    claim.minutes
                ));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "file(s) state a push gap that `MEDIAN_PUSH_GAP_MINUTES` \
         ({MEDIAN_PUSH_GAP_MINUTES}) does not:\n{}\n\
         The constant is the one authority. Moving it moves every copy in the \
         same commit -- a rationale left behind still reads as derived.",
        wrong.join("\n")
    );

    // Each long lane carries one copy, so the floor is read off the table
    // rather than written down. Zero here means the scanner stopped finding
    // claims, and a scanner that finds none agrees with everything.
    let floor = long_lanes().len();
    assert!(
        seen >= floor,
        "only {seen} push-gap claim(s) found where the {floor} long lane(s) in \
         `LANES` each carry one -- the scanner has stopped reading the phrase \
         it owns, and would pass a tree where every copy is stale."
    );
}

/// Every long lane states, in its own workflow, the two numbers that decide it.
///
/// Presence is required rather than merely checked, and that is the point: a
/// rule that only validates copies it happens to find is escaped by deleting
/// the sentence, which is the cheapest way to make a comment stop disagreeing
/// with a measurement.
///
/// A workflow may state its OWN median and no other lane's. A figure copied
/// from a neighbouring lane has nothing that would ever re-derive it.
#[test]
fn every_long_lane_states_its_measured_numbers_in_its_own_workflow() {
    let mut faults = Vec::new();
    let lanes = long_lanes();

    for (file, median) in &lanes {
        let found = claims(&read_workflow(file));
        let (gap, lane): (Vec<_>, Vec<_>) = found.iter().partition(|c| c.about_the_gap);

        if gap.is_empty() {
            faults.push(format!(
                "  {file}: states no push gap. Add \"against a \
                 {MEDIAN_PUSH_GAP_MINUTES} {CLAIM_PHRASE} {GAP_SUFFIX} to `main`\"."
            ));
        }
        if !lane
            .iter()
            .any(|c| (c.minutes - median).abs() <= CLAIM_TOLERANCE)
        {
            faults.push(format!(
                "  {file}: `LANES` measures it at {median} min and its own \
                 comments state {:?}.",
                lane.iter().map(|c| c.minutes).collect::<Vec<_>>()
            ));
        }
        for other in lane
            .iter()
            .filter(|c| (c.minutes - median).abs() > CLAIM_TOLERANCE)
        {
            faults.push(format!(
                "  {file}: states {} min, which is not its own {median} min \
                 row -- a lane speaks for itself or points at the table.",
                other.minutes
            ));
        }
    }

    assert!(
        faults.is_empty(),
        "long lane(s) whose comments disagree with `LANES`:\n{}",
        faults.join("\n")
    );

    // `a_lane_slower_than_the_push_gap_is_not_superseded` already refuses an
    // empty long population; this floor keeps THIS case from passing on one.
    assert!(
        !lanes.is_empty(),
        "no lane in `LANES` is slower than the push gap, so this case examined \
         nothing"
    );
}
