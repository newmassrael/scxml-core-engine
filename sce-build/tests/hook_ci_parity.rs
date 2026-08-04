// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Parity gate: where the pre-push hook claims to mirror a CI workflow, the
// command it runs must not be weaker than the command CI runs.
//
// `workflow_trigger_coverage` pins *whether* a gate runs. This pins *how*.
// A stage can fire on exactly the right changes and still miss what CI
// catches, because the cargo profile decides which checks are compiled in
// at all — `[profile.test]` leaves debug-assertions on, `--release` turns
// them off. An overflowing add panics under the first and silently wraps
// under the second.
//
// The hook ran the workspace suite with `--release` while
// rust-workspace-tests.yml ran it in the test profile, and the workflow's
// own header described itself as a mirror of the hook. That is wrong in
// both directions at once. Weaker: an arithmetic overflow reaching main
// could only ever fail on the runner, which is precisely the round-trip
// the hook exists to prevent. Slower: development builds are unoptimised,
// so a release sweep shares no artifacts with the tree it runs in and
// rebuilds the workspace on every push.
//
// This test reads all of `.github/workflows/`, so it is registered in
// `workflow_trigger_coverage`'s gate list and runs from the same
// unfiltered workflow.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e))
}

/// Command lines invoking the workspace test suite, whitespace-normalised.
///
/// Continuations are joined first, and comment lines dropped — the hook
/// header and the workflow prose both describe these commands, and a
/// description is not an invocation. A `log_step` label is neither: it is
/// kept deliberately, because a label that advertises a profile the
/// command below it does not use is its own defect.
fn workspace_suite_invocations(text: &str) -> Vec<String> {
    text.replace("\\\n", " ")
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| l.contains("cargo test") && l.contains("--workspace"))
        .collect()
}

fn selects_release(cmd: &str) -> bool {
    cmd.contains("--release") || cmd.contains("--profile release")
}

/// Whether a `[profile.*]` section turns debug assertions off.
fn disables_debug_assertions(manifest: &str, section: &str) -> bool {
    let header = format!("[{section}]");
    let mut inside = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == header;
            continue;
        }
        if !inside || !trimmed.starts_with("debug-assertions") {
            continue;
        }
        let (_, value) = trimmed.split_once('=').expect("a key line carries `=`");
        return value.split('#').next().unwrap_or("").trim() == "false";
    }
    false
}

#[test]
fn the_workspace_suite_runs_with_debug_assertions_in_hook_and_ci() {
    let sources = [
        ("tools/git-hooks/pre-push", read("tools/git-hooks/pre-push")),
        (
            ".github/workflows/rust-workspace-tests.yml",
            read(".github/workflows/rust-workspace-tests.yml"),
        ),
    ];

    let mut offenders = Vec::new();
    for (name, text) in &sources {
        let invocations = workspace_suite_invocations(text);
        assert!(
            !invocations.is_empty(),
            "{name}: no `cargo test --workspace` invocation found — this gate \
             is reading the wrong file or the suite moved"
        );
        for cmd in invocations {
            if selects_release(&cmd) {
                offenders.push(format!("  {name}: {cmd}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "the workspace suite is invoked in the release profile, where \
         debug-assertions are off and integer overflow wraps instead of \
         panicking:\n{}\n\
         Both sides run it in the default test profile. Release buys no \
         parity — CI never runs the suite optimised — and costs a full \
         rebuild, since a development tree's artifacts are unoptimised.",
        offenders.join("\n")
    );
}

/// A workflow's steps: everything outside the `on:` trigger block.
///
/// Trigger declarations name scripts too — `paths:` filters list the
/// scripts whose edits should start the workflow — and a path filter is
/// not an invocation. Dropping the block is what stops a filter entry
/// from reading as a step the hook must mirror.
///
/// `workflow_trigger_coverage` parses the same block for the opposite
/// reason (it wants the block, this wants everything else). The two stay
/// separate rather than sharing a helper module: a shared `tests/common/`
/// would sit in a directory the registry backstop's `tests/*.rs` scan
/// does not reach, so a tree-wide reader hidden there would go
/// unregistered. Six lines duplicated beats a hole in the backstop.
fn steps_only(text: &str) -> String {
    let mut out = String::new();
    let mut in_triggers = false;
    for line in text.lines() {
        if line.starts_with("on:") {
            in_triggers = true;
            continue;
        }
        if in_triggers {
            let indented = line.starts_with(' ') || line.starts_with('\t');
            if line.trim().is_empty() || indented {
                continue;
            }
            in_triggers = false;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Repo-relative `scripts/*.sh` paths a body invokes.
fn scripts_invoked(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        for token in line.split_whitespace() {
            let token = token
                .trim_matches(|c| c == '"' || c == '\'' || c == ';' || c == ')')
                .trim_start_matches("./");
            if token.starts_with("scripts/") && token.ends_with(".sh") {
                out.insert(token.to_string());
            }
        }
    }
    out
}

/// Workflows the hook declares it mirrors, read from the stage table so
/// this cannot drift from the selector's own mapping.
fn mirrored_workflows(selector: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in selector.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        let Some((_, rest)) = line.split_once("\"workflows\":") else {
            continue;
        };
        let Some((inside, _)) = rest
            .split_once('[')
            .and_then(|(_, tail)| tail.split_once(']'))
        else {
            continue;
        };
        for token in inside.split(',') {
            let name = token.trim().trim_matches('"');
            if name.ends_with(".yml") {
                out.insert(name.to_string());
            }
        }
    }
    out
}

#[test]
fn the_hook_runs_every_script_the_workflows_it_mirrors_run() {
    let selector = read("tools/git-hooks/select_stages.py");
    let mirrored = mirrored_workflows(&selector);
    assert!(
        mirrored.len() > 3,
        "read only {} mirrored workflow(s) from the stage table — the \
         parse is broken, not the mirroring",
        mirrored.len()
    );

    let hook_scripts = scripts_invoked(&read("tools/git-hooks/pre-push"));

    let mut missing = Vec::new();
    for workflow in &mirrored {
        let text = read(&format!(".github/workflows/{workflow}"));
        for script in scripts_invoked(&steps_only(&text)) {
            if !hook_scripts.contains(&script) {
                missing.push(format!("  {workflow} runs {script}; the hook does not"));
            }
        }
    }

    assert!(
        missing.is_empty(),
        "stage(s) mirror a workflow but skip part of what it runs, so the \
         hook reports green on a change CI will fail:\n{}\n\
         Either invoke the script from the mirroring stage, or stop \
         claiming the workflow in the stage table — a partial mirror is \
         the harder failure to notice, because the stage does fire.",
        missing.join("\n")
    );
}

#[test]
fn the_test_profile_keeps_debug_assertions_on() {
    let manifest = read("Cargo.toml");

    // `[profile.test]` inherits from `[profile.dev]`, so either one can
    // switch the checks off for the suite.
    let mut disabled = Vec::new();
    for section in ["profile.dev", "profile.test"] {
        if disables_debug_assertions(&manifest, section) {
            disabled.push(section);
        }
    }

    assert!(
        disabled.is_empty(),
        "Cargo.toml turns debug assertions off in {disabled:?}, which \
         removes the overflow checks the workspace suite relies on — the \
         same loss that running it with `--release` caused.\n\
         `[profile.test] opt-level` is free to change; this bit is not."
    );
}
