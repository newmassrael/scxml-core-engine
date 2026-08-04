// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Trigger-coverage gate: a test whose input set is wider than any
// `paths:` filter could express must be run by a workflow that declares
// no filter at all.
//
// One direction of hook/CI agreement is already structural.
// `select_stages.py` reads the `paths:` filters out of the workflows
// instead of restating them, so a pre-push stage cannot drift from the CI
// trigger it mirrors. Nothing played that role for the other direction —
// whether a test's *inputs* stay inside the trigger of the workflow that
// runs it. Where they do not, the gate is enforced only by whoever
// happens to run `cargo test` locally, and CI reports green on changes it
// never examined.
//
// That failure is measured. `roadmap_marker_gate` enumerates every
// tracked file, but the workflow running it filters on `sce-build/**` and
// a few siblings. On 2026-08-04 a violation landed under
// `tools/git-hooks/`, outside that filter: no workflow started, main went
// green, and a local run caught it days later. Widening the filter by
// hand fixes that one path and leaves the next one open, because the
// mismatch is between a filter and a tree — no list of globs is both
// correct and narrower than "everything".
//
// So the arrangement is: such gates run in a workflow with no filter, and
// this test pins that they do. It reads all of `.github/workflows/`,
// which makes it one of the gates it describes; it is registered below
// and runs beside `roadmap_marker_gate` in the same unfiltered workflow.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Test targets whose input set outgrows any path filter.
///
/// `roadmap_marker_gate` reads every tracked file; this test reads every
/// workflow. Adding an entry is a claim that the target's inputs cannot
/// be enumerated as globs — and it obliges the unfiltered workflow to run
/// that target by name.
const UNFILTERABLE_GATES: &[&str] = &["roadmap_marker_gate", "workflow_trigger_coverage"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

/// Every workflow as `(file name, text)`, sorted for stable diagnostics.
fn workflows() -> Vec<(String, String)> {
    let dir = repo_root().join(".github/workflows");
    let mut out: Vec<(String, String)> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {}", dir.display(), e))
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x == "yml" || x == "yaml")
        })
        .map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let text = fs::read_to_string(e.path())
                .unwrap_or_else(|err| panic!("read {}: {}", e.path().display(), err));
            (name, text)
        })
        .collect();
    out.sort();
    assert!(
        out.len() > 5,
        "found only {} workflow file(s) — enumeration is broken",
        out.len()
    );
    out
}

/// The body of the top-level `on:` key: every line indented under it, up
/// to the next line that starts in column zero. A top-level comment ends
/// the block, so prose about triggers cannot be mistaken for one.
fn on_block(name: &str, text: &str) -> String {
    let mut lines = text.lines();
    let mut saw_on = false;
    for line in lines.by_ref() {
        if line.starts_with("on:") {
            assert_eq!(
                line.trim_end(),
                "on:",
                "{name}: `on:` carries an inline value; this gate reads the block form only"
            );
            saw_on = true;
            break;
        }
    }
    assert!(saw_on, "{name}: no top-level `on:` key");

    let mut block = String::new();
    for line in lines {
        let indented = line.starts_with(' ') || line.starts_with('\t');
        if !line.trim().is_empty() && !indented {
            break;
        }
        block.push_str(line);
        block.push('\n');
    }
    block
}

/// Whether the `on:` block narrows which changes start the workflow.
/// `paths-ignore` counts: it is the same filter with the sense flipped.
fn declares_path_filter(on: &str) -> bool {
    on.lines()
        .map(str::trim)
        .any(|l| l == "paths:" || l == "paths-ignore:")
}

/// Whether a workflow runs `target` **by name**.
///
/// A workspace sweep (`cargo test --workspace`) also executes the target,
/// but coverage acquired that way is bound to the sweep's own trigger —
/// which is exactly the filter this gate exists to escape. Counting only
/// `--test <name>` keeps the coverage declared rather than incidental.
fn runs_target_by_name(text: &str, target: &str) -> bool {
    // `run:` bodies wrap with trailing backslashes; join them so a
    // continuation does not hide the flag from a line-wise scan.
    let joined = text.replace("\\\n", " ");
    let flag = format!("--test {target}");
    joined
        .lines()
        .any(|l| l.contains("cargo test") && l.contains(&flag))
}

/// Test sources whose inputs reach past any glob list.
///
/// Two signatures, both of which have to be spelled at the call site to
/// do the thing they signal: shelling out to git's tracked-file
/// enumeration, and reading the workflow directory. The former is
/// assembled at compile time so this file does not match itself on a
/// signature it never uses.
fn inputs_outgrow_a_filter(src: &str) -> bool {
    let tracked_file_enumeration = concat!("ls-", "files");
    src.contains(tracked_file_enumeration) || src.contains(".github/workflows")
}

fn integration_test_sources() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut out: Vec<(String, String)> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {}", dir.display(), e))
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("rs"))
        .map(|e| {
            let stem = e
                .path()
                .file_stem()
                .expect("a .rs file has a stem")
                .to_string_lossy()
                .into_owned();
            let text = fs::read_to_string(e.path())
                .unwrap_or_else(|err| panic!("read {}: {}", e.path().display(), err));
            (stem, text)
        })
        .collect();
    out.sort();
    assert!(
        out.len() > 20,
        "found only {} integration test source(s) — enumeration is broken",
        out.len()
    );
    out
}

/// The workflows that run `gate` by name and filter no paths.
fn unfiltered_carriers(gate: &str) -> Vec<String> {
    workflows()
        .into_iter()
        .filter(|(name, text)| {
            runs_target_by_name(text, gate) && !declares_path_filter(&on_block(name, text))
        })
        .map(|(name, _)| name)
        .collect()
}

#[test]
fn every_unfilterable_gate_runs_in_a_workflow_without_a_path_filter() {
    let mut unbacked = Vec::new();
    for gate in UNFILTERABLE_GATES {
        if unfiltered_carriers(gate).is_empty() {
            let filtered: Vec<String> = workflows()
                .into_iter()
                .filter(|(_, text)| runs_target_by_name(text, gate))
                .map(|(name, _)| name)
                .collect();
            unbacked.push(format!(
                "  {gate}: no unfiltered workflow runs it by name (workflows naming it: {})",
                if filtered.is_empty() {
                    "none".to_string()
                } else {
                    filtered.join(", ")
                }
            ));
        }
    }

    assert!(
        unbacked.is_empty(),
        "gate(s) whose inputs span the tree are triggered by a narrower filter, \
         so a change outside it is judged by the gate but never starts CI:\n{}\n\
         Fix by running the target from a workflow with no `paths:` key \
         (.github/workflows/tree-hygiene.yml), not by widening a filter — \
         no glob list is both correct and narrower than the tree.",
        unbacked.join("\n")
    );
}

#[test]
fn the_pre_push_hook_runs_every_unfilterable_gate() {
    // Half one: the hook's stage table maps the carrying workflow, which is
    // how the selector derives "run this stage always" — an unfiltered
    // workflow yields the catch-all trigger.
    let selector_path = repo_root().join("tools/git-hooks/select_stages.py");
    let selector = fs::read_to_string(&selector_path)
        .unwrap_or_else(|e| panic!("read {}: {}", selector_path.display(), e));

    let mut carriers: BTreeSet<String> = BTreeSet::new();
    for gate in UNFILTERABLE_GATES {
        carriers.extend(unfiltered_carriers(gate));
    }
    assert!(
        !carriers.is_empty(),
        "no unfiltered workflow carries any registered gate — \
         the companion test explains this failure in full"
    );

    let unmapped: Vec<&String> = carriers.iter().filter(|w| !selector.contains(*w)).collect();
    assert!(
        unmapped.is_empty(),
        "workflow(s) carrying a tree-wide gate are absent from the pre-push \
         stage table, so the hook skips a gate CI runs on every push: {:?}\n\
         Add a stage in {} mapping the workflow; its missing `paths:` filter \
         makes the stage unconditional, which is the intent.",
        unmapped,
        selector_path.display()
    );

    // Half two: a mapped stage that runs nothing is a table entry, not a
    // gate. Selecting a stage and executing the target are separate edits,
    // so check the hook body names each target too.
    let hook_path = repo_root().join("tools/git-hooks/pre-push");
    let hook = fs::read_to_string(&hook_path)
        .unwrap_or_else(|e| panic!("read {}: {}", hook_path.display(), e));

    let unrun: Vec<&&str> = UNFILTERABLE_GATES
        .iter()
        .filter(|gate| !runs_target_by_name(&hook, gate))
        .collect();
    assert!(
        unrun.is_empty(),
        "the pre-push hook selects the tree-wide stage but never runs \
         target(s) {:?}, so the gate holds locally only by way of the full \
         workspace sweep — the coupling this arrangement exists to break.\n\
         Name each target in the stage 1d body of {}.",
        unrun,
        hook_path.display()
    );
}

#[test]
fn the_registry_lists_every_gate_whose_inputs_outgrow_a_filter() {
    let detected: BTreeSet<String> = integration_test_sources()
        .into_iter()
        .filter(|(_, src)| inputs_outgrow_a_filter(src))
        .map(|(stem, _)| stem)
        .collect();
    let registered: BTreeSet<String> = UNFILTERABLE_GATES.iter().map(|s| s.to_string()).collect();

    let unregistered: Vec<&String> = detected.difference(&registered).collect();
    assert!(
        unregistered.is_empty(),
        "test(s) read inputs no `paths:` filter can enumerate but are not \
         registered in UNFILTERABLE_GATES: {unregistered:?}\n\
         Register each one, then run it by name from the unfiltered \
         workflow — otherwise it judges changes that never start CI."
    );

    let stale: Vec<&String> = registered.difference(&detected).collect();
    assert!(
        stale.is_empty(),
        "UNFILTERABLE_GATES names test(s) that no longer read tree-wide \
         inputs: {stale:?}\n\
         Drop the entry so the unfiltered workflow stops carrying a target \
         that a normal path filter would now cover."
    );
}
