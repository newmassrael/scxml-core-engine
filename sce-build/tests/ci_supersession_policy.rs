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
//! deferral. Measured 2026-09-02 the two populations still separate: seven
//! lanes from 20.5 to 79.1 minutes, and the next one down at 15.9. [`LANES`]
//! carries every figure.
//!
//! ⚠ **A cancellation COUNT is not evidence of the defect, and must not be read
//! as one.** `cancel-in-progress: false` still cancels a PENDING run, so a
//! repaired lane goes on reporting cancellations forever. Measured 2026-09-02,
//! `cpp-suite.yml` -- repaired at `74f83197b7` and long since declaring `false`
//! -- shows 13 cancellations in its last 25 runs, and its five newest cancelled
//! NO JOB AT ALL: every one was still entirely pending. That is the setting
//! working, not failing.
//!
//! What discriminates is whether the cancellation killed work that had already
//! STARTED, which is a per-job question the run conclusion cannot answer:
//!
//! ```sh
//! gh run view <id> --json jobs   # a cancelled job with startedAt < completedAt
//! ```
//!
//! Read that way the counts confirm rather than mislead: `tree-hygiene.yml`
//! before its repair killed 1.1 to 17.0 minutes of started work per
//! cancellation, and `regen-reproduces.yml` still does.
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
    ("clang-format-check.yml", 0.8, 0, 25),
    ("clippy-check.yml", 5.9, 0, 23),
    // 13 of its last 25 cancelled while declaring `false`, and that is the
    // setting WORKING: its five newest cancellations killed no job at all --
    // every one was still entirely pending. Its repair (`74f83197b7`,
    // 2026-08-29T00:53) now sits BEFORE this window instead of inside it, so
    // unlike the row this replaces the count is post-fix history.
    ("cpp-suite.yml", 79.1, 13, 11),
    // The nearest lane below the line and the one to re-measure first: 15.9
    // against a 17.6 gap, with `embed-vendor-smoke.yml` next at 15.2.
    ("deploy-visualizer.yml", 15.9, 2, 22),
    ("doc-check.yml", 0.6, 0, 24),
    ("doc-content-gate.yml", 10.6, 1, 24),
    ("drift-verify.yml", 7.1, 1, 24),
    ("ecma262-lowered-cpp.yml", 23.7, 2, 16),
    // Now measured from ITS OWN hosted runs. This row carried 4.8 borrowed
    // from the `Kotlin W3C Tests` job of `w3c-tests.yml` -- a strict superset
    // of its work -- because the lane had landed 2026-08-30 with no history to
    // take a median over, and the borrowing was recorded here with the
    // instruction to replace it once it had. It has six successes now, median
    // 6.3, so the stand-in is retired.
    //
    // The borrowed bound held: 6.3 is above the 4.8 it stood in for and still
    // far below the gap, so the substitution never changed a classification.
    ("ecma262-lowered-kotlin.yml", 6.3, 0, 6),
    ("embed-vendor-smoke.yml", 15.2, 4, 15),
    ("example-codegen.yml", 9.3, 1, 24),
    ("fmt-check.yml", 7.2, 0, 25),
    // Moved 8.9 -> 22.2 earlier on 2026-09-02 and re-taken here at 25.5 in the
    // common window; the population lost one success to the 25-run edge. Its
    // 5 cancellations killed work rather than a queue slot, which is what
    // `false` is for: `Rust Forge Conformance` was 44.9 min in when the push at
    // 15:29Z took it, and `Build sce-codegen` 20.9 min in at 15:50Z. This lane
    // fans out to five languages, so one supersession discards all five.
    //
    // ⚠ A calmer day does not bring this row back down: measured at 22.2, the
    // six runs of the quietest day median 22.15 -- the same figure as the full
    // window. The heavy tail moved and the middle did not, so moving this row
    // needs a faster lane, not a quieter day.
    ("forge-conformance.yml", 25.5, 5, 19),
    ("http-endpoint-ssot.yml", 7.2, 7, 18),
    ("license-verify.yml", 0.3, 0, 3),
    // ⚠⚠⚠ 2.6 -> 60.4, the largest move in this table, and the row that made
    // reading `cancel-in-progress` ALONE untenable. This lane is long, declares
    // `true`, and is nonetheless right: its concurrency group carries
    // `github.sha`, so a later push lands in a different group and cannot
    // supersede it at all. `group_is_per_commit` is what reads that.
    //
    // Its 0 cancellations are that mechanism showing. The old row justified the
    // same `true` by calling the lane short, which this measurement refutes --
    // the conclusion was right and its stated basis was false.
    ("mutation-rounds.yml", 60.4, 0, 11),
    // ⚠ Below the line at 14.1, and so required to declare `true`, yet 12 of
    // its last 25 were cancelled and every one of the five newest killed
    // STARTED work, 1.1 to 17.2 minutes in.
    //
    // The median rule does not catch this and is not being bent to: 11 of the
    // 26 push gaps in the same window are under 13 minutes, so a 14.1-minute
    // lane loses to those while still finishing inside the MEDIAN gap.
    // Comparing medians answers "typically", and this row is the standing
    // evidence that the typical case is not the whole distribution.
    ("regen-reproduces.yml", 14.1, 12, 13),
    ("rust-workspace-tests.yml", 41.4, 0, 16),
    ("sce-forge-codec-clippy.yml", 1.4, 1, 24),
    ("sce-forge-codec-no-alloc.yml", 1.1, 1, 24),
    ("sce-rust-runtime-no-std.yml", 1.6, 0, 25),
    ("spec-citations.yml", 8.8, 1, 23),
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
    ("tree-hygiene.yml", 20.5, 11, 9),
    ("w3c-tests.yml", 72.4, 13, 10),
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
/// The window the docs prescribe is each lane's OWN last 25 runs, so
/// `cancelled + successes` above 25 is arithmetically impossible for that
/// query however plausible the duration looks. Nothing offline can tell a
/// stale row from a fresh one, but this does catch the shape a hand-edited or
/// half-updated row takes: one column re-taken and the other left behind.
#[test]
fn no_row_counts_more_runs_than_the_window_holds() {
    const WINDOW: u32 = 25;
    let mut checked = 0;

    for &(file, _, cancelled, successes) in LANES {
        assert!(
            cancelled + successes <= WINDOW,
            "{file} reports {cancelled} cancellation(s) and {successes} \
             success(es) -- {} outcomes out of a {WINDOW}-run window. The two \
             columns came from different readings.",
            cancelled + successes
        );
        checked += 1;
    }

    assert!(
        checked >= 20,
        "only {checked} row(s) examined -- `LANES` has lost its rows, and this \
         case passes trivially on an empty table"
    );
}

#[test]
fn a_lane_slower_than_the_push_gap_is_not_superseded() {
    let mut long = 0;
    let mut short = 0;
    // How each long lane satisfies the rule, counted apart so that the
    // per-commit arm cannot quietly become the only one that ever fires.
    let mut long_by_false = 0;
    let mut long_by_key = 0;

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
                 commits. Measured: {cancelled} cancellation(s) in its recent \
                 history, median over {successes} successes. Set \
                 `cancel-in-progress: false`, or key the group on `github.sha` \
                 so a later push lands in a different group, or re-measure and \
                 move the row."
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

/// The rows this file classifies as unable to finish between two pushes.
fn long_lanes() -> Vec<(&'static str, f64)> {
    LANES
        .iter()
        .filter(|(_, median, _, _)| must_not_supersede(*median))
        .map(|(file, median, _, _)| (*file, *median))
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
