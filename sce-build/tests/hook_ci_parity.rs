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

use std::collections::{BTreeMap, BTreeSet};
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

/// Everything a push actually runs: the hook plus every gate script it
/// delegates to.
///
/// The gates used to sit inline in the hook, so reading one file was the
/// whole surface. They now live one per file under `scripts/gates/` and the
/// hook only works out which of them a change needs. These parity tests ask
/// "does a push run what CI runs", so the surface they read has to follow
/// the commands rather than the filename — a test still pointed at the hook
/// alone would find no `cargo test` at all and pass by reading nothing.
fn hook_surface() -> String {
    let mut out = read("tools/git-hooks/pre-push");
    let dir = repo_root().join("scripts/gates");
    let mut scripts: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {}", dir.display(), e))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "sh"))
        .collect();
    assert!(
        !scripts.is_empty(),
        "no gate scripts under scripts/gates/ — the hook delegates to a \
         directory that is empty, so every parity test below would pass by \
         finding nothing"
    );
    scripts.sort();
    for path in scripts {
        out.push('\n');
        out.push_str(
            &fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e)),
        );
    }
    out
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
/// that is the gap this closes. `sce-build` declares test targets with
/// `required-features = ["cli"]` — how many is derived by
/// `cli_feature_gating`, which also asks whether the commands reaching
/// them enable the feature at all — and cargo excludes an unmet-features
/// target **silently**: it is never built and never reported as skipped.
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

    let hook = hook_surface();
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
        ("pre-push + scripts/gates/", hook_surface()),
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
    verification_invocations_depth(text, 0)
}

/// `depth` bounds the one expansion this extractor performs:
/// `scripts/gate <slug>` runs the gate's own script, so the commands it
/// verifies live one file away. Without expanding, a workflow that
/// delegates contributes no verification at all and the parity check
/// below passes it vacuously — the delegation would remove the workflow
/// from the comparison rather than simplify it. `hook_surface` already
/// reads every gate body, so expanding here is what puts the two sides
/// back on the same footing.
fn verification_invocations_depth(text: &str, depth: usize) -> BTreeSet<String> {
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
            let before = || (i > 0).then(|| tokens[i - 1]);
            match *token {
                // `scripts/foo.sh` — the original axis.
                //
                // `source scripts/lib/foo.sh` is excluded: sourcing loads
                // function definitions into the current shell, so the
                // caller decides what runs on the next line, and that
                // call is what this extractor should see. Keying on the
                // preceding word rather than on a `scripts/lib/`
                // directory convention states the actual distinction —
                // how the file is used, not where it sits.
                //
                // The over-approximation the header describes still
                // holds for execution: only sourcing is dropped, so a
                // script that is run rather than loaded stays visible
                // whatever its path.
                t if t.starts_with("scripts/")
                    && t.ends_with(".sh")
                    && !matches!(before(), Some("source") | Some(".")) =>
                {
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
                // `scripts/gate <slug>` — delegation to the gate runner.
                // Expanded rather than tokenised: the point of the
                // parity check is which commands run, and a token
                // naming the runner would compare a spelling instead.
                "scripts/gate" if depth == 0 => {
                    if let Some(slug) = after(1) {
                        let gate = repo_root().join(format!("scripts/gates/{slug}.sh"));
                        if let Ok(body) = std::fs::read_to_string(&gate) {
                            out.extend(verification_invocations_depth(&body, depth + 1));
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
        "scripts/install_mesh_transports.sh",
        "provisions the runner rather than verifying the tree: it builds \
         vsomeip3, zenohcxx and CycloneDDS from source, which a push-time \
         hook must not do. A workstation carries them already — that is why \
         it registers 178 cases where a bare runner registers 130 — and \
         `cpp-suite`, which the hook does run, refuses rather than report on \
         the smaller set.",
    ),
    (
        "scripts/install_mnemosyne_cli.sh",
        "provisions the runner rather than verifying the tree: it is a \
         `cargo install` from the network, which a push-time hook must not \
         perform — a developer installs the pinned binary once, and the \
         citation gate's own refusal prints the command that does it. The \
         verification it enables is `ledger-citations`, which the hook does \
         run.",
    ),
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
    // `gradlew :sce-kotlin-tests:test` used to sit here for the same
    // reason as the forge-runtime task above. The reason was mechanical,
    // not structural: the task invoked the generator without
    // SOURCE_DATE_EPOCH, so it stamped the wall clock into 449 committed
    // headers. `backends/kotlin/tests/build.gradle.kts` pins it now, the
    // `w3c-kotlin` gate re-checks that the run left the tree clean, and
    // the lane delegates to that gate — so the Kotlin W3C arm is no
    // longer CI-only and an entry here would describe nothing.
    (
        "xml_to_html.py",
        "publication, not verification. The report job turns the C++ JUnit \
         XML into the HTML deployed to Pages; it reaches no verdict of its \
         own and runs `if: always()` after the suites that do. A hook that \
         ran it would render a page nobody reads.",
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
    let selector = read("tools/git-hooks/gate_registry.py");
    let mirrored = mirrored_workflows(&selector);
    assert!(
        mirrored.len() > 3,
        "read only {} mirrored workflow(s) from the stage table — the \
         parse is broken, not the mirroring",
        mirrored.len()
    );

    let hook = verification_invocations(&hook_surface());
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

// ── The audit hook must not change behaviour with the caller's cwd ──

/// Drive `.claude/hooks/commit_audit.sh` with a synthetic payload and
/// return `(stdout, stderr, exit_code)`.
fn run_commit_audit(cwd: &Path, home: &Path) -> (String, String, Option<i32>) {
    let payload = format!(
        r#"{{"cwd":{},"tool_input":{{"command":"git commit -m \"chore: probe the audit hook\""}}}}"#,
        serde_json_string(&cwd.display().to_string()),
    );
    let hook = repo_root().join(".claude/hooks/commit_audit.sh");
    let mut child = std::process::Command::new("bash")
        .arg(&hook)
        .current_dir(repo_root())
        .env("HOME", home)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", hook.display()));
    {
        use std::io::Write;
        child
            .stdin
            .as_mut()
            .expect("hook stdin")
            .write_all(payload.as_bytes())
            .expect("write payload");
    }
    let out = child.wait_with_output().expect("hook runs to completion");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

/// Minimal JSON string escaping — enough for a filesystem path.
fn serde_json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The section of the banner listing memory-lifecycle violations, or
/// empty when the validation never ran.
fn memo_section(text: &str) -> String {
    match text.find("Violations:") {
        Some(i) => text[i..].to_string(),
        None => String::new(),
    }
}

/// Committing from a linked worktree must produce the same audit as
/// committing from the main tree.
///
/// The memory-lifecycle validation keys off a slug derived from a
/// directory path. Deriving it from the *invoking* directory made a
/// linked worktree resolve to a memory directory that has never
/// existed, and the branch that reads it was also the only place its
/// accumulator was initialised — so under `set -u` the hook died on an
/// unbound variable rather than asking its five questions.
///
/// Initialising the accumulator alone would have been worse than the
/// crash: the validation would then skip in every worktree without
/// saying so, which is the silently-inert gate this repository forbids.
/// Both halves are pinned here, by requiring the two invocations to
/// agree rather than merely requiring neither to crash.
///
/// The memory tree is seeded into a private HOME rather than read from
/// the developer's own, so the probe does not pass on a machine that
/// happens to have memos and fail on a CI runner that has none. The
/// seeded memo deliberately violates the lifecycle contract: the
/// validation names the offending file, and a named file is the only
/// evidence that the validation ran at all. A conforming memo leaves
/// the hook silent from both callers, and two silences agree with each
/// other while proving nothing.
///
/// The observable moved once already. It used to be the self-audit
/// banner's plan-memo list, which was removed when the audit gate was
/// retired; the memory-lifecycle banner is the surviving reader of the
/// same tree.
#[test]
fn the_audit_hook_reads_the_same_memory_tree_from_a_linked_worktree() {
    let root = repo_root();
    let scratch =
        std::env::temp_dir().join(format!("sce-audit-hook-worktree-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    let branch = format!("audit-hook-probe-{}", std::process::id());

    // The hook slugs the MAIN worktree's path into a memory directory under
    // $HOME (commit_audit.sh: "The slug keys off the *main* worktree, not the
    // invoking directory"); seed exactly that directory so the validation has
    // something to report.
    //
    // Resolved the way the hook resolves it, not from `root`. `root` is
    // CARGO_MANIFEST_DIR's parent, which is the checkout the test is COMPILED
    // in — and when that is a linked worktree the two paths differ, so seeding
    // under `root` put the memo somewhere the hook never looks and the banner
    // never named it. Measured: this test passed from the main checkout and
    // failed from either of two linked worktrees carrying unrelated changes,
    // which is a test that cannot run where the repository is meant to be
    // workable rather than a defect in the hook.
    let main_worktree = {
        let out = std::process::Command::new("git")
            .current_dir(&root)
            .args(["worktree", "list", "--porcelain"])
            .output()
            .expect("git worktree list runs");
        let text = String::from_utf8_lossy(&out.stdout);
        text.lines()
            .find_map(|line| line.strip_prefix("worktree "))
            .expect("git worktree list names the main worktree first")
            .to_string()
    };
    let home = std::env::temp_dir().join(format!("sce-audit-hook-home-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let slug = main_worktree.replace('/', "-");
    let memory = home.join(".claude/projects").join(&slug).join("memory");
    fs::create_dir_all(&memory).expect("seed memory dir");
    // A memo that VIOLATES the lifecycle contract: `feedback_*.md` must
    // declare `status:feedback`. The validation names the offending file
    // in its banner, which is what makes "the same memory tree was read"
    // observable from both callers. A conforming memo would leave the
    // hook silent, and two silences agree with each other while proving
    // nothing.
    fs::write(
        memory.join("feedback_audit_hook_probe.md"),
        "---\nname: feedback_audit_hook_probe\ndescription: seeded probe memo\n\
         metadata:\n  type: project\n  status: open\n---\n\n\
         Seeded by the worktree audit probe; the hook must name this file \
         from either caller.\n",
    )
    .expect("seed lifecycle-violating memo");

    let add = std::process::Command::new("git")
        .current_dir(&root)
        .args(["worktree", "add", "--detach"])
        .arg(&scratch)
        .arg("HEAD")
        .output()
        .expect("git worktree add runs");
    assert!(
        add.status.success(),
        "git worktree add failed:\n{}",
        String::from_utf8_lossy(&add.stderr),
    );

    let (out_main, err_main, code_main) = run_commit_audit(&root, &home);
    let (out_wt, err_wt, code_wt) = run_commit_audit(&scratch, &home);

    // Clean up before asserting so a failure does not strand a worktree.
    let _ = std::process::Command::new("git")
        .current_dir(&root)
        .args(["worktree", "remove", "--force"])
        .arg(&scratch)
        .output();
    let _ = std::process::Command::new("git")
        .current_dir(&root)
        .args(["branch", "-D", &branch])
        .output();
    let _ = fs::remove_dir_all(&scratch);
    let _ = fs::remove_dir_all(&home);

    for (label, err) in [("main tree", &err_main), ("linked worktree", &err_wt)] {
        assert!(
            !err.contains("internal failure"),
            "commit_audit.sh failed internally when invoked from the {label}:\n{err}",
        );
        assert!(
            !err.contains("unbound variable") && !err.contains("바인딩 해제한 변수"),
            "commit_audit.sh read an uninitialised variable from the {label}:\n{err}",
        );
    }

    assert_eq!(
        code_main, code_wt,
        "the audit hook must reach the same verdict from either directory;\
         \nmain stderr:\n{err_main}\nworktree stderr:\n{err_wt}",
    );

    let memos_main = memo_section(&format!("{out_main}{err_main}"));
    let memos_wt = memo_section(&format!("{out_wt}{err_wt}"));
    // Reaching the memory validation is a precondition of the
    // comparison below. The banner prints this section only after the
    // validation has run, so its absence means the probe stopped
    // earlier — at the COMMIT_FORMAT gate, say — and two empty strings
    // would then agree with each other while proving nothing.
    assert!(
        memos_main.contains("feedback_audit_hook_probe.md"),
        "the probe never reached the memory validation from the main tree; \
         the banner did not name the violating memo seeded into its HOME:\nstdout:\n\
         {out_main}\nstderr:\n{err_main}",
    );
    assert_eq!(
        memos_main, memos_wt,
        "the audit hook listed different plan memos depending on the caller's \
         directory — the memory tree it validates must not depend on which \
         worktree the commit came from",
    );
}

// ── The requiring lane must supply every tool the harness can ask for ──

/// Every apt-sourced tool must be installed by the lane that requires
/// tools.
#[test]
fn every_apt_sourced_tool_is_installed_by_the_requiring_lane() {
    use sce_build::toolchain::{ToolSource, HARNESS_TOOLS};
    let workflow = read(".github/workflows/rust-workspace-tests.yml");
    assert!(
        workflow.contains("SCE_REQUIRE_TOOLS"),
        "this test is aimed at the lane that promotes a missing tool into a \
         failure; that lane no longer sets SCE_REQUIRE_TOOLS, so the pairing \
         has moved and this check needs re-aiming",
    );
    let installs: String = workflow
        .lines()
        .filter(|l| l.contains("apt-get install"))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        !installs.is_empty(),
        "no `apt-get install` step found in the requiring lane",
    );

    let mut checked = 0usize;
    let mut missing: Vec<String> = Vec::new();
    for (tool, source) in HARNESS_TOOLS {
        let ToolSource::AptPackage(pkg) = source else {
            continue;
        };
        checked += 1;
        if !installs.split_whitespace().any(|w| w == *pkg) {
            missing.push(format!("{tool} needs package `{pkg}`"));
        }
    }
    assert!(
        checked >= 2,
        "only {checked} apt-sourced tools checked; the table lost its \
         AptPackage entries and this gate would pass on anything",
    );
    assert!(
        missing.is_empty(),
        "the lane sets SCE_REQUIRE_TOOLS but does not install what the \
         harness asks for ({} tool(s)):\n  {}\napt line(s):\n  {installs}",
        missing.len(),
        missing.join("\n  "),
    );
}

/// A lane that runs a gate whose script can skip on a missing tool must
/// say so, and must supply the tool.
///
/// The shell half of the harness had no equivalent of the Rust
/// `SCE_REQUIRE_TOOLS` pairing above, and the gap was not theoretical:
/// `scripts/test_emit_manifest_fail_fast.sh` answered `exit 0` when
/// clang was absent, so `embed-manifest-failfast` reported green while
/// verifying nothing. The workflow already carried a comment saying an
/// uninstalled toolchain "would turn this job green without running the
/// assertion it exists for" — a correct sentence with nothing acting on
/// it, which is the shape this repository keeps finding.
///
/// Derived in both directions, so a future skip-capable gate is covered
/// without a second edit: the call sites are found by scanning the
/// scripts for the helper, the lanes by asking the gate registry which
/// workflows run that slug.
/// The YAML block of the job that contains `needle`.
///
/// Jobs sit at one indent level under `jobs:`, so a block runs from its
/// own `  <name>:` line to the next one. Enough structure to keep a
/// per-job claim per-job, without pulling in a YAML parser for two
/// string checks.
fn job_block_running<'a>(workflow: &'a str, needle: &str) -> String {
    let mut blocks: Vec<Vec<&'a str>> = Vec::new();
    for line in workflow.lines() {
        let is_job_header = line.starts_with("  ")
            && !line.starts_with("   ")
            && line.trim_end().ends_with(':')
            && !line.trim_start().starts_with('#');
        if is_job_header {
            blocks.push(Vec::new());
        }
        if let Some(current) = blocks.last_mut() {
            current.push(line);
        }
    }
    blocks
        .into_iter()
        .find(|b| b.iter().any(|l| l.contains(needle)))
        .map(|b| b.join("\n"))
        .unwrap_or_default()
}

#[test]
fn lane_running_a_skip_capable_gate_requires_its_tools() {
    // Every script that can skip on a missing tool, with what it needs.
    let mut skip_capable: Vec<(PathBuf, String, String)> = Vec::new();
    for dir in ["scripts", "scripts/gates"] {
        for entry in std::fs::read_dir(repo_root().join(dir)).expect("script dir is readable") {
            let path = entry.expect("dir entry").path();
            let Ok(body) = std::fs::read_to_string(&path) else {
                continue;
            };
            for line in body.lines() {
                let line = line.trim();
                if line.starts_with('#') {
                    continue;
                }
                // Found anywhere on the line, not only at its start. The
                // call is a command whose status decides what follows, so
                // `if sce_gate_requires_tool ...` and
                // `sce_gate_requires_tool ... || exit 0` are the same
                // statement in two shapes — and matching only the second
                // dropped a gate's two tool requirements out of this test
                // without failing anything. A collector that silently sees
                // less than it claims is the defect this file exists to
                // catch, so it must not be one.
                let Some(rest) = line
                    .split_once("sce_gate_requires_tool ")
                    .map(|(_, rest)| rest)
                else {
                    continue;
                };
                let mut parts = rest.split_whitespace();
                let (Some(bin), Some(pkg)) = (parts.next(), parts.next()) else {
                    panic!("`sce_gate_requires_tool` in {path:?} takes a binary and a package");
                };
                // `; then` is shell punctuation, not part of the package.
                let pkg = pkg.trim_end_matches(';');
                assert!(
                    !pkg.starts_with('"') && !pkg.starts_with('\''),
                    "{path:?} quotes the package argument of `sce_gate_requires_tool` \
                     ({pkg}). The package is compared against the words of the lane's \
                     install step, so a quoted or multi-word value can never match one \
                     and the pairing this test enforces would pass vacuously."
                );
                skip_capable.push((path.clone(), bin.to_string(), pkg.to_string()));
            }
        }
    }
    assert!(
        !skip_capable.is_empty(),
        "no `sce_gate_requires_tool` call site found — either the helper \
         was renamed or a skip went back to a bare `exit 0`, which is the \
         state this check exists to keep the tree out of",
    );

    // Gate slug that reaches each script: the script itself when it is a
    // gate, otherwise the gate whose body invokes it.
    let gates_dir = repo_root().join("scripts/gates");
    let mut violations: Vec<String> = Vec::new();
    for (script, _bin, package) in &skip_capable {
        let rel = script
            .strip_prefix(repo_root())
            .expect("script sits under the repo root")
            .display()
            .to_string();
        let mut slugs: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(&gates_dir).expect("gates dir is readable") {
            let gate = entry.expect("dir entry").path();
            let Ok(body) = std::fs::read_to_string(&gate) else {
                continue;
            };
            if gate == *script || body.contains(&rel) {
                slugs.push(
                    gate.file_stem()
                        .expect("gate file has a stem")
                        .to_string_lossy()
                        .to_string(),
                );
            }
        }
        if slugs.is_empty() {
            violations.push(format!("{rel}: no gate under scripts/gates/ runs it"));
            continue;
        }

        for slug in &slugs {
            let mut lanes = 0usize;
            for entry in
                std::fs::read_dir(repo_root().join(".github/workflows")).expect("workflows dir")
            {
                let wf = entry.expect("dir entry").path();
                let Ok(body) = std::fs::read_to_string(&wf) else {
                    continue;
                };
                if !body.contains(&format!("scripts/gate {slug}")) {
                    continue;
                }
                lanes += 1;
                let name = wf.file_name().expect("workflow file").to_string_lossy();

                // Scoped to the job that runs the gate. A file-wide
                // search passed a mutation that deleted this job's
                // `apt-get install`, because a sibling job in the same
                // workflow installs the same package — and a toolchain
                // installed in another job is not on this job's runner.
                let job = job_block_running(&body, &format!("scripts/gate {slug}"));

                // Both checks read the YAML, not the prose. An earlier
                // draft matched the strings anywhere and passed on a
                // mutation that deleted the `env:` entry outright: the
                // surrounding comment still named it. A comment
                // standing in for the thing it describes is the failure
                // this test exists to catch, so it must not be the way
                // this test passes.
                let sets_var = job
                    .lines()
                    .map(str::trim)
                    .any(|l| l.starts_with("SCE_REQUIRE_TOOLS:"));
                if !sets_var {
                    violations.push(format!(
                        "{name} runs `{slug}` but that job sets no SCE_REQUIRE_TOOLS key — \
                         a runner without the tool would skip the check and still report green"
                    ));
                }
                let installs = job
                    .lines()
                    .map(str::trim)
                    .filter(|l| !l.starts_with('#'))
                    .any(|l| {
                        l.split_whitespace()
                            .any(|w| w.trim_end_matches('\\') == package)
                    });
                if !installs {
                    violations.push(format!(
                        "{name} runs `{slug}` under SCE_REQUIRE_TOOLS but that job installs \
                         no `{package}`, so the lane fails on its own requirement"
                    ));
                }
            }

            assert!(
                lanes > 0,
                "no workflow runs `scripts/gate {slug}`, so the skip-capable gate has \
                 no lane claiming it ran — the local-only state T1 recorded"
            );
        }
    }
    assert!(
        violations.is_empty(),
        "skip-capable gate(s) whose lane does not claim the check ran:\n  {}",
        violations.join("\n  "),
    );
}

// ── A gate that builds against third_party/ says so BEFORE it builds ──

/// Every gate slug the registry knows, taken from the runner's own listing
/// rather than a list here.
fn all_gate_slugs() -> Vec<String> {
    let registry = repo_root().join("tools/git-hooks/gate_registry.py");
    let out = std::process::Command::new("python3")
        .arg(&registry)
        .arg("--list")
        .arg("--repo-root")
        .arg(repo_root())
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", registry.display()));
    assert!(
        out.status.success(),
        "gate_registry.py --list failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // `    0.0s  slug  description` — the slug is the second field.
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1).map(str::to_string))
        .collect()
}

/// `slug -> workflow file`, one pair per line, straight from the registry.
fn gate_workflow_pairs() -> Vec<(String, String)> {
    let registry = repo_root().join("tools/git-hooks/gate_registry.py");
    let program = format!(
        "import runpy\n\
         mod = runpy.run_path({})\n\
         for slug, entry in mod['GATES'].items():\n\
         \x20   for wf in entry.get('workflows', []):\n\
         \x20       print(slug + '\\t' + wf)\n",
        serde_json_string(&registry.display().to_string())
    );
    let out = std::process::Command::new("python3")
        .arg("-c")
        .arg(&program)
        .current_dir(repo_root())
        .output()
        .expect("python3 reads the registry");
    assert!(
        out.status.success(),
        "reading GATES failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let (slug, wf) = line.split_once('\t')?;
            Some((slug.to_string(), wf.to_string()))
        })
        .collect()
}

/// Does this YAML really ask `actions/checkout` for submodules?
///
/// Comment lines are stripped FIRST, and that is the whole subtlety:
/// `tree-hygiene.yml` explains its own exemption in the words it is exempt
/// from — "No `submodules: recursive`: the gate skips third_party/" — so a
/// substring search reads that comment as the opposite of what it says.
fn asks_for_submodules(text: &str) -> bool {
    text.lines().any(|line| {
        let stripped = line.trim();
        if stripped.starts_with('#') {
            return false;
        }
        let code = stripped.split('#').next().unwrap_or("");
        code.starts_with("submodules:") && code.contains("recursive")
    })
}

/// `(job name, job body)` for each job in a workflow.
///
/// A job is a two-space key under `jobs:`, which is the whole grammar this
/// needs: the answer being derived is one step's `with:` value. Deliberately
/// the same reading the preflight does, re-implemented rather than shelled
/// out to — this file's job is to answer independently and compare.
fn jobs_of(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut name: Option<String> = None;
    let mut body: Vec<&str> = Vec::new();
    let mut in_jobs = false;
    for line in text.lines() {
        if line.starts_with("jobs:") {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        let is_job_header = line.starts_with("  ")
            && !line.starts_with("   ")
            && line.trim_end().ends_with(':')
            && line
                .trim()
                .trim_end_matches(':')
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            && !line.trim().trim_end_matches(':').is_empty();
        if is_job_header {
            if let Some(previous) = name.take() {
                out.push((previous, body.join("\n")));
            }
            name = Some(line.trim().trim_end_matches(':').to_string());
            body = Vec::new();
            continue;
        }
        body.push(line);
    }
    if let Some(previous) = name {
        out.push((previous, body.join("\n")));
    }
    out
}

/// Does this job body invoke `scripts/gate <slug>`?
///
/// The trailing boundary is load-bearing: without it `w3c-python` matches the
/// job running `scripts/gate w3c-python-bindings`, and those two jobs check
/// out differently — which is exactly the confusion this axis exists to
/// resolve.
fn job_runs_slug(body: &str, slug: &str) -> bool {
    let needle = format!("gate {slug}");
    let mut from = 0usize;
    while let Some(at) = body[from..].find(&needle) {
        let start = from + at;
        let end = start + needle.len();
        let after_ok = body[end..]
            .chars()
            .next()
            .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'));
        let before_ok = start == 0
            || !body[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if after_ok && before_ok {
            return true;
        }
        from = end;
    }
    false
}

/// The same answer, re-derived here from the same two inputs.
///
/// Per JOB, not per file. `actions/checkout` is a step, so a multi-job
/// workflow answers this question several times: `w3c-tests.yml` has seven
/// jobs and three of them say `submodules: false` outright. Measured
/// 2026-08-24, the round after the preflight landed, a file-level answer
/// refused six gates INSIDE CI — in the very jobs that had been running them
/// green without submodules all along. A slug no job names falls back to the
/// file, because "cannot tell" should refuse.
fn expected_gates_needing_submodules() -> BTreeSet<String> {
    let mut by_slug: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (slug, wf) in gate_workflow_pairs() {
        by_slug.entry(slug).or_default().push(wf);
    }
    by_slug
        .into_iter()
        .filter(|(slug, workflows)| {
            let texts: Vec<String> = workflows
                .iter()
                .filter_map(|wf| {
                    fs::read_to_string(repo_root().join(".github/workflows").join(wf)).ok()
                })
                .collect();
            let running: Vec<String> = texts
                .iter()
                .flat_map(|text| jobs_of(text))
                .filter(|(_, body)| job_runs_slug(body, slug))
                .map(|(_, body)| body)
                .collect();
            if running.is_empty() {
                texts.iter().any(|text| asks_for_submodules(text))
            } else {
                running.iter().any(|body| asks_for_submodules(body))
            }
        })
        .map(|(slug, _)| slug)
        .collect()
}

/// What the preflight answers for those slugs.
fn gates_needing_submodules(slugs: &[String]) -> BTreeSet<String> {
    let script = repo_root().join("scripts/lib/require_submodules.sh");
    let out = std::process::Command::new("bash")
        .arg(&script)
        .arg("--which")
        .args(slugs)
        .current_dir(repo_root())
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", script.display()));
    assert!(
        out.status.success(),
        "`--which` reports, it never refuses: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

/// `git worktree add` does not populate submodules, and neither does a plain
/// clone. Measured 2026-08-24: a push from a fresh worktree spent every gate
/// ahead of `w3c-kotlin` and then died 27 seconds into a Gradle CMake
/// configure whose message named no submodule and no repair; the gates ahead
/// of it had to run again from the start on the next push.
///
/// What this pins is not "submodules exist" — the machine running the test
/// may legitimately have them — but that the preflight DERIVES which gates
/// need them from the workflows those gates mirror. The derivation is the
/// part that can silently rot: `tree-hygiene.yml` explains its own exemption
/// in the very words it is exempt from, so a substring search puts the two
/// gates mirroring it on exactly the wrong side.
#[test]
fn a_gate_that_builds_against_third_party_is_preflighted_for_submodules() {
    let gate = read("scripts/gate");
    assert!(
        gate.contains("require_submodules.sh"),
        "scripts/gate does not run the submodule preflight, so a tree missing \
         third_party/ still fails deep inside a build instead of at the top"
    );

    let slugs = all_gate_slugs();
    assert!(
        slugs.len() >= 25,
        "only {} gate slug(s) parsed out of the registry listing — the sweep is \
         not reaching the corpus, and an empty sweep reads as a pass",
        slugs.len()
    );
    let needing = gates_needing_submodules(&slugs);

    // The claim, in the only form that cannot be faked by a typed list: the
    // script's answer is what the registry and the workflow files say, for
    // EVERY gate. Re-derived here from the same two inputs rather than
    // compared against a list written down in either place — the moment a
    // workflow changes its mind, a hardcoded preflight and this test disagree.
    assert_eq!(
        needing,
        expected_gates_needing_submodules(),
        "the preflight's answer is not the one the workflows give; it has stopped \
         deriving and is carrying a list of its own"
    );

    // Measured against the push that produced this test: these reached a
    // verdict in a tree with no submodules, and `w3c-kotlin` was the one that
    // could not.
    for exempt in ["ledger-citations", "rustdoc-links", "rust-modrs-drift"] {
        assert!(
            !needing.contains(exempt),
            "`{exempt}` was reported as needing submodules, but it reached a verdict \
             without them; refusing it would block work that CI itself does not gate \
             on third_party/"
        );
    }
    for exempt in ["tree-hygiene", "mutation-cases"] {
        assert!(
            !needing.contains(exempt),
            "`{exempt}` mirrors tree-hygiene.yml, which checks out WITHOUT submodules \
             on purpose and says so in a comment — reading that comment as the \
             declaration is the trap this assertion exists for"
        );
    }
    for needed in ["w3c-kotlin", "cpp-suite", "workspace-tests"] {
        assert!(
            needing.contains(needed),
            "`{needed}` mirrors a workflow that asks CI for submodules, so a local \
             run without them cannot mirror it; the preflight did not say so"
        );
    }

    // Measured 2026-08-24: each of these runs in a CI job that checks out
    // WITHOUT submodules, and the preflight refused them there on the day it
    // landed — five red lanes, in checkouts that were correct. They are named
    // rather than counted because a count cannot say which six.
    for exempt in [
        "w3c-go",
        "w3c-python",
        "forge-go",
        "forge-python",
        "forge-rust",
        "embed-manifest-failfast",
    ] {
        assert!(
            !needing.contains(exempt),
            "`{exempt}` was reported as needing submodules, but the CI job that runs \
             it checks out without them — the derivation is reading the file instead \
             of the job again"
        );
    }

    // A floor, not a target: a derivation that suddenly finds nothing would
    // pass every assertion above that is phrased as an absence.
    //
    // 12 since the derivation became per-job (2026-08-24); 18 before, and
    // that 18 was the file-level over-count rather than a coverage this lost.
    // The floor's purpose is catching an empty sweep, which 10 still serves.
    assert!(
        needing.len() >= 10,
        "the derivation found only {} gate(s) needing submodules, against 12 when it \
         became per-job; it has stopped reading the workflows: {needing:?}",
        needing.len()
    );
}
