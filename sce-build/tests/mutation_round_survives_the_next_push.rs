// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A mutation round is not lost when a newer push replaces the one waiting.
//!
//! `mutation-rounds.yml` answers about a COMMIT, not about a branch: selection
//! is by change set, so a round judges the casefiles that one range's targets
//! selected, and nothing re-selects them afterwards unless a later push happens
//! to touch the same declared targets. A casefile is only judged when it
//! changes. For a lane like that a run that never happens is a verdict LOST,
//! not deferred.
//!
//! Until 2026-10-01 the repair was one key: `github.sha` put each push's round
//! in a group of its own, so no push could replace or cancel another's. It held
//! the verdict and grew the queue without bound -- measured that day, 55 of the
//! 123 runs active across the account were this workflow's. The owner asked
//! instead for the behaviour every other lane has: the round in flight is kept,
//! one round waits behind it, and a newer push replaces the waiting one, so only
//! the newest commit runs when the one in flight ends.
//!
//! That is safe for this lane only if the newest commit's range reaches back
//! over the commits the replaced round would have judged. So the range does not
//! start at the push's own `before`, which names only the previous push, but at
//! the commit of the last round of this workflow that reached a verdict. A round
//! that was cancelled, skipped or replaced has no verdict and is passed over, so
//! its commits fall inside the next range.
//!
//! This holds both halves shut. The key must be shared per ref and must not name
//! the commit, and the range must be built from the workflow's own run history.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

fn read_mutation_rounds() -> (PathBuf, String) {
    let path = repo_root().join(".github/workflows/mutation-rounds.yml");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    (path, text)
}

/// The value of `key` in the workflow's top-level `concurrency:` block.
///
/// Read as text rather than through a YAML parser: the value is a GitHub
/// expression, and what this test is about is which contexts appear inside it.
fn concurrency_value(workflow: &str, key: &str) -> String {
    let prefix = format!("{key}:");
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
                break;
            }
            if let Some(value) = trimmed.strip_prefix(&prefix) {
                return value.trim().to_string();
            }
        }
        break;
    }
    String::new()
}

#[test]
fn a_push_round_shares_a_group_per_ref_and_is_not_cancelled() {
    let (path, workflow) = read_mutation_rounds();
    let group = concurrency_value(&workflow, "group");

    // A lower bound before any claim about the contents. A block that moved,
    // or a `group:` spelled some other way, would otherwise read as an empty
    // string that satisfies nothing and fails nothing — the shape where a
    // scan reports green because it found the file and not the thing.
    assert!(
        group.len() > 40 && group.contains("github.workflow"),
        "no usable `group:` found under `concurrency:` in {} — got {group:?}. \
         The block moved or the key was renamed, and this test cannot say \
         anything about a value it did not read",
        path.display()
    );

    assert!(
        group.contains("github.ref"),
        "the concurrency group does not name the ref: {group}\n\
         Every branch and pull request would then share one group, and a push \
         to one would replace the waiting round of another."
    );

    assert!(
        !group.contains("github.sha"),
        "the concurrency group names the commit: {group}\n\
         Every push then lands in a group of its own, nothing replaces a \
         waiting round, and the queue grows without bound — measured \
         2026-10-01, 55 of the 123 runs active across the account. The owner \
         asked for the last commit only; the range below is what keeps that \
         from losing a verdict."
    );

    // The round in flight is kept. Reading the literal rather than the lane's
    // behaviour, for the reason the policy test gives.
    assert_eq!(
        concurrency_value(&workflow, "cancel-in-progress"),
        "false",
        "the round in flight is cancelled by the next push. Selection is by \
         change set, so a round that never finishes judges nothing and no \
         later round re-selects its casefiles — keep it, and let the waiting \
         slot hold the newest commit."
    );

    // The dispatch key is a separate property, repaid earlier for the same
    // reason (run 32034288611: a dispatch cancelled the push run for its ref),
    // and it now has a second job: a dispatch must not take a push round's
    // single waiting slot. Named here so removing it reads as the regression
    // it would be.
    assert!(
        group.contains("github.event_name == 'workflow_dispatch'")
            && group.contains("github.run_id"),
        "the concurrency group no longer gives a dispatch a key of its own: {group}\n\
         Asking for an extra round would take the push round's waiting slot, \
         which is the opposite of what a dispatch is for."
    );
}

/// The workflow's code lines, with comments dropped, so that prose describing
/// the mechanism cannot stand in for it.
fn code_lines(workflow: &str) -> Vec<&str> {
    workflow
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect()
}

#[test]
fn a_push_range_starts_at_the_last_round_that_reached_a_verdict() {
    let (path, workflow) = read_mutation_rounds();
    let code = code_lines(&workflow);
    let has = |needle: &str| code.iter().any(|l| l.contains(needle));

    // The listing of this workflow's own completed runs on this branch.
    assert!(
        has("actions/workflows/mutation-rounds.yml/runs?")
            && has("event=push")
            && has("status=completed"),
        "{} does not list its own completed push runs. A push range that \
         starts at `github.event.before` covers only the previous push, so a \
         round that replaces a waiting one drops the casefiles that round was \
         for, and nothing re-selects them.",
        path.display()
    );

    // Only a verdict advances the range. A `cancelled` or `skipped` round
    // judged nothing, and counting it would hide its commits from the next one.
    assert!(
        has(".conclusion == \"success\"") && has(".conclusion == \"failure\""),
        "{} does not filter the listing on `success` and `failure`. A round \
         that was cancelled has no verdict; if it advances the range, its \
         commits are in nobody's.",
        path.display()
    );

    // The step that fills the range needs `actions: read`, or the listing is
    // refused at run time and only a hosted run would say so.
    assert!(
        has("actions: read"),
        "{} reads the run listing without `actions: read`; the job's token \
         would be refused and the step fails only on the hosted runner.",
        path.display()
    );

    // The fallback exists, and is the ONLY use of `before`: a branch with no
    // earlier verdict still gets a range rather than the corpus.
    assert!(
        has("github.event.before"),
        "{} lost its fallback to the push's own range for a branch with no \
         earlier verdict.",
        path.display()
    );

    // A failed listing must fail the step. Falling back silently reintroduces
    // the loss this range exists to end.
    assert!(
        !code
            .iter()
            .any(|l| l.contains("gh api") && l.contains("|| true")),
        "{} swallows a failed run listing, which would fall back to the push's \
         own range without saying so.",
        path.display()
    );
}
