// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A mutation round is not cancelled by the push that follows it.
//!
//! `mutation-rounds.yml` runs `cancel-in-progress: true`, which is the right
//! setting for a lane whose answer is about a BRANCH — a newer commit really
//! does supersede an older one's build. It is the wrong setting for a lane
//! whose answer is about a COMMIT, and this one's is: selection is by change
//! set, so a round judges the casefiles that one push's targets selected, and
//! nothing re-selects them afterwards unless a later push happens to touch the
//! same declared targets. A casefile is only judged when it changes.
//!
//! So a cancellation here is a verdict LOST, not deferred. Measured across 40
//! runs (2026-08-24T01:06 → 08-25T02:13): 25 never reached success and 24 of
//! those were `cancelled` — the dominant way this lane failed to answer, ahead
//! of the job timeout `mutation_corpus_fits_its_lane` guards. The workflow's
//! own comment had recorded the mechanism ("pushes keep sharing one, so a
//! newer push still cancels the round an older one started") without repaying
//! it, and the habit that grew around it — commit now, push later, if you need
//! the verdict — is the shape of a defect being worked around.
//!
//! The repair is one key: `github.sha` puts each push's round in a group of its
//! own. This holds that shut. It does NOT ask for `cancel-in-progress: false` —
//! superseding is still correct for `pull_request`, where a force-push destroys
//! the very commit the run was judging.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

/// The `group:` value of the workflow's top-level `concurrency:` block.
///
/// Read as text rather than through a YAML parser: the value is a GitHub
/// expression, and what this test is about is which contexts appear inside it.
fn concurrency_group(workflow: &str) -> String {
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
            // Left the block without meeting `group:`.
            if !body.starts_with(' ') {
                break;
            }
            if let Some(value) = trimmed.strip_prefix("group:") {
                return value.trim().to_string();
            }
        }
        break;
    }
    String::new()
}

#[test]
fn a_push_round_is_keyed_to_the_commit_it_judges() {
    let workflow_path = repo_root().join(".github/workflows/mutation-rounds.yml");
    let workflow = std::fs::read_to_string(&workflow_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", workflow_path.display()));

    let group = concurrency_group(&workflow);

    // A lower bound before any claim about the contents. A block that moved,
    // or a `group:` spelled some other way, would otherwise read as an empty
    // string that satisfies nothing and fails nothing — the shape where a
    // scan reports green because it found the file and not the thing.
    assert!(
        group.len() > 40 && group.contains("github.workflow"),
        "no usable `group:` found under `concurrency:` in {} — got {group:?}. \
         The block moved or the key was renamed, and this test cannot say \
         anything about a value it did not read",
        workflow_path.display()
    );

    assert!(
        group.contains("github.event_name == 'push'") && group.contains("github.sha"),
        "the concurrency group does not distinguish one push from the next: \
         {group}\n\
         Every push to a branch then lands in the same group, and \
         `cancel-in-progress: true` kills the round judging the previous \
         commit. Selection is by change set, so those casefiles are not \
         re-selected by the push that killed them — the verdict is lost, not \
         deferred. Measured: 24 of 25 non-success runs ended `cancelled`. Key \
         push runs on `github.sha`."
    );

    // The dispatch key is the same property, repaid earlier for the same
    // reason (run 32034288611: a dispatch cancelled the push run for its ref).
    // Named here so removing it reads as the regression it would be.
    assert!(
        group.contains("github.event_name == 'workflow_dispatch'")
            && group.contains("github.run_id"),
        "the concurrency group no longer gives a dispatch a key of its own: {group}\n\
         Asking for an extra round would again cancel the push round for the \
         same ref, which is the opposite of what a dispatch is for."
    );
}
