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

/// Both lanes must invoke the workspace suite with the same feature set.
///
/// Profile parity was gated from the start; feature parity was not, and
/// that is the gap this closes. `sce-build` declares 15 test targets with
/// `required-features = ["cli"]`, and cargo excludes an unmet-features
/// target **silently** — it is never built and never reported as skipped.
/// A lane without the flag therefore runs a strictly smaller suite while
/// its step name still says `--workspace`, which is indistinguishable from
/// the outside from a lane that runs everything and passes.
///
/// The gate compares the two lanes to each other rather than to a pinned
/// string: adding a second feature is fine as long as both lanes get it,
/// and pinning `cli` here would just move the drift to the pin.
#[test]
fn the_workspace_suite_runs_with_the_same_features_in_hook_and_ci() {
    /// Feature names a `cargo test` command line enables, sorted.
    fn features(cmd: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let tokens: Vec<&str> = cmd.split_whitespace().collect();
        for (i, t) in tokens.iter().enumerate() {
            let list = if let Some(rest) = t.strip_prefix("--features=") {
                rest
            } else if *t == "--features" {
                match tokens.get(i + 1) {
                    Some(next) => next,
                    None => continue,
                }
            } else {
                continue;
            };
            // `log_step` / `fail_step` labels are deliberately part of the
            // scanned set (a label naming a feature the command below does
            // not pass is its own defect), so trim the shell quoting and
            // parenthesis they carry before comparing.
            out.extend(
                list.split(',')
                    .map(|f| {
                        f.trim()
                            .trim_matches(|c| c == '"' || c == ')' || c == '(')
                            .to_string()
                    })
                    .filter(|f| !f.is_empty()),
            );
        }
        if cmd.contains("--all-features") {
            out.push("*all*".to_string());
        }
        out.sort();
        out.dedup();
        out
    }

    let hook = read("tools/git-hooks/pre-push");
    let ci = read(".github/workflows/rust-workspace-tests.yml");

    let hook_cmds = workspace_suite_invocations(&hook);
    let ci_cmds = workspace_suite_invocations(&ci);
    assert!(
        !hook_cmds.is_empty() && !ci_cmds.is_empty(),
        "no `cargo test --workspace` invocation found in one of the lanes — \
         this gate is reading the wrong file or the suite moved"
    );

    let hook_features: Vec<Vec<String>> = hook_cmds.iter().map(|c| features(c)).collect();
    let ci_features: Vec<Vec<String>> = ci_cmds.iter().map(|c| features(c)).collect();

    for (label, sets) in [("pre-push", &hook_features), ("CI", &ci_features)] {
        if let Some(first) = sets.first() {
            assert!(
                sets.iter().all(|s| s == first),
                "{label} invokes the workspace suite with differing feature sets: {sets:?}"
            );
        }
    }

    assert_eq!(
        hook_features.first(),
        ci_features.first(),
        "the hook and CI run the workspace suite with different features.\n\
         hook: {hook_cmds:?}\n  CI: {ci_cmds:?}\n\n\
         A target whose `required-features` are unmet is dropped without a \
         warning, so the lane with fewer features runs a smaller suite under \
         the same name."
    );
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

/// One verification a body invokes, reduced to a key that survives both
/// spellings.
///
/// A workflow step and a hook line describe the same check differently:
/// flag order, quoting, a `./` prefix, a `cd` into a subdirectory, a
/// `| tee` on the end, a `2>&1`. The key keeps what identifies the
/// verification — the tool plus the argument naming its target — and
/// drops the rest.
///
/// Recognition deliberately over-approximates. A token that merely looks
/// like a check is better surfaced and then declared CI-only in
/// [`CI_ONLY`] than silently dropped, because a dropped one is exactly
/// the divergence this gate exists to catch. The previous version of
/// this function matched `scripts/*.sh` and nothing else, so
/// `cargo`, `go`, `ctest`, `gradlew` and every `python3 tools/...`
/// invocation were outside its field of view — the test asserted
/// "the hook runs every script the workflows run" and meant something
/// far narrower than its name.
fn verification_invocations(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in text.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        let tokens: Vec<&str> = line
            .split_whitespace()
            .map(|t| {
                t.trim_matches(|c| c == '"' || c == '\'' || c == ';' || c == ')')
                    .trim_start_matches("./")
            })
            .collect();

        for (i, token) in tokens.iter().enumerate() {
            let after = |n: usize| tokens.get(i + n).copied();
            match *token {
                // `scripts/foo.sh` — the original axis.
                t if t.starts_with("scripts/") && t.ends_with(".sh") => {
                    out.insert(t.to_string());
                }
                // `python3 tools/mnemosyne-adoption/check_spec_drift.py --mode integrity`
                // keys on basename + the flag that picks which check runs,
                // since one script can be two gates.
                t if t.ends_with(".py") => {
                    let name = t.rsplit('/').next().unwrap_or(t);
                    let mut key = name.to_string();
                    for j in 1..4 {
                        match after(j) {
                            Some("--mode") => {
                                if let Some(m) = after(j + 1) {
                                    key = format!("{name} --mode {m}");
                                }
                                break;
                            }
                            Some("--check") => {
                                key = format!("{name} --check");
                                break;
                            }
                            _ => {}
                        }
                    }
                    out.insert(key);
                }
                // `cargo test --release -p X --features Y --test Z` keys on
                // the test target; the profile is asserted separately by
                // `the_workspace_suite_runs_with_debug_assertions_in_hook_and_ci`.
                "cargo" if matches!(after(1), Some("test")) => {
                    for (j, t) in tokens.iter().enumerate() {
                        if *t == "--test" {
                            if let Some(target) = tokens.get(j + 1) {
                                out.insert(format!("cargo test --test {target}"));
                            }
                        }
                    }
                }
                // `go test ./conformance/ -count=1` — the package path is
                // what distinguishes the two Go arms.
                "go" if matches!(after(1), Some("test")) => {
                    if let Some(pkg) = after(2) {
                        out.insert(format!("go test ./{}", pkg.trim_start_matches("./")));
                    }
                }
                // `./gradlew :project:task`
                "gradlew" => {
                    for t in &tokens[i + 1..] {
                        if t.starts_with(':') {
                            out.insert(format!("gradlew {t}"));
                        }
                    }
                }
                // `ctest --test-dir <dir>` — the directory is a scratch path
                // in the hook and a fixed name in CI, so it is not part of
                // the key.
                "ctest" => {
                    out.insert("ctest".to_string());
                }
                // `python3 -m unittest discover -s <dir>` / `... tests.test_x`
                "unittest" => {
                    let mut key = "unittest".to_string();
                    if let Some(target) = tokens[i + 1..].iter().find(|t| !t.starts_with('-')) {
                        key = format!("unittest {target}");
                    }
                    if let Some(j) = tokens[i..].iter().position(|t| *t == "-s") {
                        if let Some(dir) = tokens.get(i + j + 1) {
                            key = format!("{key} -s {dir}");
                        }
                    }
                    out.insert(key);
                }
                _ => {}
            }
        }
    }
    out
}

/// Verifications the mirrored workflows run that the hook deliberately
/// does not, each with the reason it stays behind.
///
/// This is the honest statement of what a green hook does not cover. Its
/// absence was the real defect: the hook skipped eight CI commands and
/// nothing said so, because saying so required diffing two files by hand.
///
/// In the manifest design this list disappears — `runs-in: ci` becomes a
/// field on the gate itself, and the set is derived rather than
/// maintained. Until then the test below pins it in both directions, so
/// an entry cannot go stale and a new skip cannot go undeclared.
const CI_ONLY: &[(&str, &str)] = &[
    (
        "check_spec_drift.py --mode upstream",
        "fetches the upstream spec over the network. A push-time gate must \
         not depend on an external host being reachable — in CI a fetch \
         failure is a retry, at push time it is a blocked push. CI runs it \
         on a schedule for the same reason.",
    ),
    (
        "gradlew :sce-forge-runtime-kotlin:jvmTest",
        "the Gradle task rewrites the committed trees' `generated-at` pins \
         as a side effect, so running it from the hook would dirty the very \
         tree being pushed. Startup cost is the secondary objection; the \
         side effect is the disqualifying one.",
    ),
];

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
fn the_hook_runs_every_verification_the_workflows_it_mirrors_run() {
    let selector = read("tools/git-hooks/select_stages.py");
    let mirrored = mirrored_workflows(&selector);
    assert!(
        mirrored.len() > 3,
        "read only {} mirrored workflow(s) from the stage table — the \
         parse is broken, not the mirroring",
        mirrored.len()
    );

    let hook = verification_invocations(&read("tools/git-hooks/pre-push"));
    assert!(
        hook.len() > 5,
        "read only {} verification(s) from the hook — the extractor is \
         broken, not the hook",
        hook.len()
    );

    // What the mirrored workflows run that the hook does not.
    let mut unmirrored: BTreeSet<(String, String)> = BTreeSet::new();
    for workflow in &mirrored {
        let text = read(&format!(".github/workflows/{workflow}"));
        for v in verification_invocations(&steps_only(&text)) {
            if !hook.contains(&v) {
                unmirrored.insert((v, workflow.clone()));
            }
        }
    }

    let declared: BTreeSet<&str> = CI_ONLY.iter().map(|(k, _)| *k).collect();
    let found: BTreeSet<&str> = unmirrored.iter().map(|(v, _)| v.as_str()).collect();

    // Direction 1 — a skip nobody declared. This is the silent-divergence
    // case: the stage fires, reports green, and CI fails on something the
    // hook never looked at.
    let undeclared: Vec<String> = unmirrored
        .iter()
        .filter(|(v, _)| !declared.contains(v.as_str()))
        .map(|(v, w)| format!("  {w} runs `{v}`; the hook does not, and CI_ONLY does not say why"))
        .collect();
    assert!(
        undeclared.is_empty(),
        "a mirrored workflow runs a verification the hook skips without \
         declaring it:\n{}\n\
         Either invoke it from the mirroring stage, or add it to CI_ONLY \
         with the reason it stays behind. A partial mirror is the harder \
         failure to notice, because the stage does fire.",
        undeclared.join("\n")
    );

    // Direction 2 — a declaration that no longer describes anything. Dead
    // exemptions accumulate silently and each one widens the gap between
    // what the list claims to document and what it does.
    let stale: Vec<&&str> = declared.difference(&found).collect();
    assert!(
        stale.is_empty(),
        "CI_ONLY declares verification(s) that the mirrored workflows no \
         longer skip: {stale:?}\n\
         Either the hook now runs them (remove the entry) or CI stopped \
         running them (remove it too). An exemption that describes nothing \
         is worse than none: it reads as coverage of a decision that has \
         already been reversed."
    );

    // Every declaration carries a reason someone can act on.
    for (key, why) in CI_ONLY {
        assert!(
            why.len() > 60,
            "CI_ONLY entry `{key}` needs a reason a reader can weigh, not \
             a label; got {} chars",
            why.len()
        );
    }
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
