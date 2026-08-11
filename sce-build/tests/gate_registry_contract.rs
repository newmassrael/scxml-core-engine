// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The gate registry's own cases, run from the test suite.
//
// `tools/git-hooks/gate_registry.py --self-test` holds the properties that
// keep the gate set honest: an unclassified path forces the full run, an
// always-on gate classifies nothing, every registered gate has a script and
// every script is registered, and the derived run order puts dependencies
// first and cheap gates ahead of expensive ones.
//
// The runner already refuses to start when those cases fail, but that only
// covers a developer who runs a gate. Nothing in CI executed them, so a
// registry edit that broke the selection could reach main and only show up
// as gates quietly not running — the same class of silence the registry
// exists to prevent. This test is the CI-side witness.
//
// It reads the whole `scripts/gates/` directory and every workflow, so it is
// registered in `workflow_trigger_coverage`'s tree-wide gate list and runs
// from an unfiltered workflow.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent directory")
        .to_path_buf()
}

#[test]
fn the_gate_registry_passes_its_own_cases() {
    let root = repo_root();
    let registry = root.join("tools/git-hooks/gate_registry.py");
    assert!(
        registry.is_file(),
        "no gate registry at {} — the hook and the runner both read it",
        registry.display()
    );

    let out = Command::new("python3")
        .arg(&registry)
        .arg("--self-test")
        .arg("--repo-root")
        .arg(&root)
        .output()
        .unwrap_or_else(|e| panic!("run {}: {}", registry.display(), e));

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "gate registry self-test failed:\n{stderr}"
    );

    // A self-test that reports zero cases passes by doing nothing. The
    // count is a floor, not a pin: adding cases is the point, and this only
    // fails if they stop running.
    let ran: usize = stderr
        .split_whitespace()
        .find_map(|t| t.parse::<usize>().ok())
        .unwrap_or(0);
    assert!(
        ran >= 26,
        "gate registry self-test reported {ran} case(s); it had 26 when this \
         floor was last raised, so the cases are not running:\n{stderr}"
    );
}

#[test]
fn the_runner_reports_its_measurements_back_to_the_registry() {
    // `cost_s` decides the run order and is updated by hand, so a stale value
    // steers every run while every gate still passes. The runner already times
    // each gate; feeding those timings back is what turns a run into a witness
    // against the declarations. Without this line the drift report exists and
    // nothing ever calls it.
    let runner =
        std::fs::read_to_string(repo_root().join("scripts/gate")).expect("read scripts/gate");
    let invocations: Vec<&str> = runner
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.starts_with('#'))
        .filter(|l| l.contains("--order-drift"))
        .filter(|l| {
            !(l.starts_with("printf") || l.starts_with("echo") || l.starts_with("log_step"))
        })
        .collect();
    assert!(
        !invocations.is_empty(),
        "the gate runner no longer reports its measurements to the registry, \
         so a stale cost_s is unobservable again"
    );
}

#[test]
fn every_workflow_the_registry_maps_exists() {
    // A registry entry naming a workflow that is not there does not fail
    // loudly: `workflow_paths` returns the catch-all for a file it cannot
    // read, so the gate quietly becomes always-on and every push pays for
    // it. That is a silent cost rather than a silent hole, which is why it
    // needs a check of its own — no run will ever report it.
    //
    // Reading the workflow directory is also what makes this test's input
    // set tree-wide, and it is not incidental: the registry derives every
    // trigger from these files, so a `paths:` edit anywhere here changes
    // what a push verifies.
    let root = repo_root();
    let registry = std::fs::read_to_string(root.join("tools/git-hooks/gate_registry.py"))
        .expect("read tools/git-hooks/gate_registry.py");

    let mut named: Vec<String> = Vec::new();
    for line in registry.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("\"workflows\":") else {
            continue;
        };
        for token in rest.split(['[', ']', ',']) {
            let name = token.trim().trim_matches('"');
            if name.ends_with(".yml") || name.ends_with(".yaml") {
                named.push(name.to_string());
            }
        }
    }
    assert!(
        named.len() > 5,
        "parsed only {} mapped workflow(s) out of the registry — the parse \
         is broken, not the mapping",
        named.len()
    );

    let dir = root.join(".github/workflows");
    let missing: Vec<&String> = named
        .iter()
        .filter(|name| !dir.join(name).is_file())
        .collect();
    assert!(
        missing.is_empty(),
        "the gate registry maps workflow(s) that do not exist: {missing:?}\n\
         Each one silently turns its gate always-on instead of failing."
    );
}

#[test]
fn every_gate_script_declares_the_workflow_it_mirrors() {
    // The class this closes: a gate script's prose claimed to mirror one
    // workflow while the registry mapped it to another. `rustdoc-links` was
    // exactly that — its comment named doc-check.yml, the registry named the
    // no-std workflow, and the two share a trigger, so the mismatch never
    // showed as a missed run. Nothing read the claim, so nothing could
    // disagree with it.
    //
    // A rule that simply forbade naming another workflow would be wrong, and
    // measurably so: two scripts name one legitimately — a note recording the
    // mapping that was fixed, and a statement that another workflow starts the
    // same fixture server. So the CLAIM is what became structured, and the
    // prose stays free.
    let root = repo_root();
    // Ask the registry, do not scrape it: the source also carries the
    // self-test's synthetic tables, and a scraper reads those as gates.
    let out = Command::new("python3")
        .arg(root.join("tools/git-hooks/gate_registry.py"))
        .arg("--mapping")
        .arg("--repo-root")
        .arg(&root)
        .output()
        .expect("run gate_registry.py --mapping");
    assert!(
        out.status.success(),
        "gate_registry.py --mapping failed: {out:?}"
    );
    let table = String::from_utf8_lossy(&out.stdout);

    // One entry per top-level key; `"workflows": [...]` inside it.
    let mut mapped: Vec<(String, Vec<String>)> = Vec::new();
    for line in table.lines() {
        let indent = line.len() - line.trim_start().len();
        let t = line.trim();
        if indent == 1 && t.ends_with("{") {
            mapped.push((
                t.trim_matches(|c| c == '"' || c == ':' || c == ' ' || c == '{')
                    .trim_matches('"')
                    .to_string(),
                Vec::new(),
            ));
        }
        if t.ends_with(".yml\",") || t.ends_with(".yml\"") {
            if let Some(last) = mapped.last_mut() {
                last.1
                    .push(t.trim_matches(|c| c == '"' || c == ',').to_string());
            }
        }
    }
    assert!(
        mapped.len() > 10,
        "parsed only {} gate(s) out of the registry — the parse is broken",
        mapped.len()
    );

    let mut offenders: Vec<String> = Vec::new();
    for (slug, workflows) in &mapped {
        let path = root.join(format!("scripts/gates/{slug}.sh"));
        let Ok(script) = std::fs::read_to_string(&path) else {
            offenders.push(format!("{slug}: no script at {}", path.display()));
            continue;
        };
        let Some(claim) = script
            .lines()
            .find_map(|l| l.trim().strip_prefix("# Mirrors:"))
            .map(|c| c.trim().to_string())
        else {
            offenders.push(format!("{slug}: no `# Mirrors:` line"));
            continue;
        };
        let mut declared: Vec<&str> = claim.split_whitespace().collect();
        declared.sort_unstable();
        let mut want: Vec<&str> = workflows.iter().map(String::as_str).collect();
        want.sort_unstable();
        let want_text = if want.is_empty() { vec!["none"] } else { want };
        if declared != want_text {
            offenders.push(format!(
                "{slug}: script says `{}`, registry maps {want_text:?}",
                claim
            ));
        }
    }
    assert!(
        offenders.is_empty(),
        "gate script(s) disagree with the registry about the workflow they \
         mirror:\n  {}\n A gate that mirrors no workflow says `# Mirrors: none` \
         and carries its reason in the registry's no_ci_reason.",
        offenders.join("\n  ")
    );
}

#[test]
fn every_gate_script_is_executable() {
    // The runner refuses a non-executable gate, which turns a forgotten
    // `chmod +x` into a failed push rather than a skipped gate. Catching it
    // here names the file instead.
    let dir = repo_root().join("scripts/gates");
    let mut checked = 0;
    let mut offenders: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {}", dir.display(), e))
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) != Some("sh") {
            continue;
        }
        checked += 1;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = entry
                .metadata()
                .unwrap_or_else(|e| panic!("stat {}: {}", path.display(), e))
                .permissions()
                .mode();
            if mode & 0o111 == 0 {
                offenders.push(path.display().to_string());
            }
        }
    }
    assert!(
        checked > 0,
        "no gate scripts found under {} — this gate would pass by reading nothing",
        dir.display()
    );
    assert!(
        offenders.is_empty(),
        "gate script(s) are not executable: {offenders:?}"
    );
}

#[test]
fn the_push_hook_delegates_rather_than_carrying_gates() {
    // The property the split exists to hold: verification commands live in
    // `scripts/gates/`, and the hook works out what a push changes. A gate
    // added back into the hook would run in a push and be unreachable by
    // `scripts/gate <slug>`, which is exactly the arrangement that made
    // "run this before pushing" a written-down recipe instead of a command.
    let hook = std::fs::read_to_string(repo_root().join("tools/git-hooks/pre-push"))
        .expect("read tools/git-hooks/pre-push");

    let offenders: Vec<&str> = hook
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.starts_with('#'))
        .filter(|l| {
            l.starts_with("cargo ")
                || l.starts_with("cmake ")
                || l.starts_with("ctest ")
                || l.starts_with("go test")
                || l.starts_with("rustup target")
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "the push hook runs verification commands directly: {offenders:?}\n\
         Put them in a gate under scripts/gates/ and register it in \
         tools/git-hooks/gate_registry.py, so the check can also be run on \
         its own."
    );

    assert!(
        hook.contains("scripts/gate"),
        "the push hook no longer delegates to the gate runner"
    );
}

/// Build a throwaway git repository with `body` staged at `tests/cite.rs`.
///
/// The gate library resolves the repo root with `git rev-parse`, so the
/// scripts are copied in; `tools/` and `docs/` are symlinked because the
/// migrator derives its own repo root from its path and reads the ledger
/// stores from there.
///
/// The fixture is also shaped so the whole pre-commit hook can run against it:
/// a minimal cargo package satisfies Stage 0, a `.rs` fixture file leaves
/// Stage 1 (staged C/C++) with nothing to do, and Stage 2's audit hook is
/// absent, which that stage already treats as "skipped".
fn staged_citation_fixture(dir: &Path, body: &str) {
    let root = repo_root();
    let run = |args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap_or_else(|e| panic!("git {args:?}: {e}"));
        assert!(out.status.success(), "git {args:?}: {out:?}");
    };
    run(&["init", "-q"]);
    std::fs::create_dir_all(dir.join("scripts/gates")).expect("mkdir scripts/gates");
    for f in ["lib.sh", "ledger-citations.sh"] {
        std::fs::copy(
            root.join("scripts/gates").join(f),
            dir.join("scripts/gates").join(f),
        )
        .unwrap_or_else(|e| panic!("copy {f}: {e}"));
    }
    std::fs::create_dir_all(dir.join("tools/git-hooks")).expect("mkdir tools/git-hooks");
    std::fs::copy(
        root.join("tools/git-hooks/pre-commit"),
        dir.join("tools/git-hooks/pre-commit"),
    )
    .expect("copy pre-commit");
    #[cfg(unix)]
    for d in ["tools/mnemosyne-adoption", "docs"] {
        std::fs::create_dir_all(dir.join(d).parent().expect("has a parent")).ok();
        std::os::unix::fs::symlink(root.join(d), dir.join(d))
            .unwrap_or_else(|e| panic!("symlink {d}: {e}"));
    }
    std::fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"cite-fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .expect("write Cargo.toml");
    std::fs::create_dir_all(dir.join("src")).expect("mkdir src");
    // Not empty: rustfmt reports a diff on a zero-byte file, which would fail
    // Stage 0 and stop the hook before it reaches the stage under test.
    std::fs::write(dir.join("src/lib.rs"), "// fixture crate\n").expect("write lib.rs");
    std::fs::create_dir_all(dir.join("tests")).expect("mkdir tests");
    std::fs::write(dir.join("tests/cite.rs"), body).expect("write fixture");
    run(&["add", "tests/cite.rs"]);
}

fn run_staged_gate(dir: &Path) -> std::process::Output {
    Command::new("bash")
        .arg(dir.join("scripts/gates/ledger-citations.sh"))
        .arg("--staged")
        .current_dir(dir)
        .output()
        .expect("run the staged citation gate")
}

#[test]
fn the_commit_stage_rejects_a_fabricated_citation_in_staged_content() {
    // The property Q6 is about: a citation naming a section that does not
    // exist must fail at the commit that writes it. Before this stage the only
    // reader ran at push, so a fabricated `§synth-F4` was reported once for a
    // batch of sixteen commits with nothing saying which one carried it.
    //
    // Both halves are asserted from behaviour rather than from the hook's
    // text: a text scan would accept a stage that runs and checks nothing,
    // which is how the previous parity gate let two mutations through.
    let dir = tempfile::tempdir().expect("tempdir");
    staged_citation_fixture(dir.path(), "// probe: §synth-F4 is not a section\n");
    let bad = run_staged_gate(dir.path());
    assert!(
        !bad.status.success(),
        "the staged gate accepted a fabricated citation: {bad:?}"
    );
    let err = String::from_utf8_lossy(&bad.stderr);
    // Anchored: a report naming the materialised copy still *contains*
    // "tests/cite.rs" as a tail, so a substring test passes on the very output
    // this assertion exists to reject.
    assert!(
        err.lines()
            .any(|l| l.trim_start().starts_with("tests/cite.rs:")),
        "the report must name the path the author has to open, not the \
         materialised index: {err}"
    );

    let dir = tempfile::tempdir().expect("tempdir");
    staged_citation_fixture(dir.path(), "// probe: §synth-5-B is a section\n");
    let good = run_staged_gate(dir.path());
    assert!(
        good.status.success(),
        "the staged gate rejected a real citation: {good:?}"
    );
}

#[test]
fn the_commit_hook_fails_the_commit_when_the_citation_stage_fails() {
    // End to end, because the text arm cannot see the failure that matters
    // most here: a stage that runs the gate and then discards its status. The
    // hook is executed against a fixture repository whose staged content
    // carries a fabricated citation, and the commit has to be refused.
    let dir = tempfile::tempdir().expect("tempdir");
    staged_citation_fixture(dir.path(), "// probe: §synth-F4 is not a section\n");
    let out = Command::new("bash")
        .arg(dir.path().join("tools/git-hooks/pre-commit"))
        .current_dir(dir.path())
        .output()
        .expect("run the pre-commit hook");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("Stage 3/3"),
        "the hook did not reach the citation stage — the fixture no longer \
         satisfies an earlier stage, so this test would pass without \
         exercising anything: {err}"
    );
    assert!(
        !out.status.success(),
        "the hook allowed a commit whose staged content carries a fabricated \
         citation: {err}"
    );
}

#[test]
fn the_commit_stage_judges_staged_content_not_the_working_tree() {
    // A citation corrected in the working tree but not re-staged must not
    // clear the commit that still carries the bad one. The stage materialises
    // the index for exactly this reason.
    let dir = tempfile::tempdir().expect("tempdir");
    staged_citation_fixture(dir.path(), "// probe: §synth-F4 is not a section\n");
    let out = run_staged_gate(dir.path());
    assert!(!out.status.success(), "fixture precondition: {out:?}");

    std::fs::write(
        dir.path().join("tests/cite.rs"),
        "// probe: §synth-5-B is a section\n",
    )
    .expect("fix in the working tree only");
    let out = Command::new("bash")
        .arg(dir.path().join("scripts/gates/ledger-citations.sh"))
        .arg("--staged")
        .current_dir(dir.path())
        .output()
        .expect("re-run the staged citation gate");
    assert!(
        !out.status.success(),
        "an unstaged fix cleared a commit that would carry the fabricated \
         citation: {out:?}"
    );
}

#[test]
fn the_commit_hook_runs_the_staged_citation_stage() {
    // The behavioural tests above prove the gate arm works; this one proves it
    // is reachable from a commit at all, and that the hook does not restate
    // the covered trees — one list, so commit time and push time cannot come
    // to disagree about coverage.
    let root = repo_root();
    let hook = std::fs::read_to_string(root.join("tools/git-hooks/pre-commit"))
        .expect("read tools/git-hooks/pre-commit");
    // An INVOCATION, not a mention. The first draft of this assertion searched
    // the whole file for the two strings and stayed green after the call was
    // replaced with `true`, because the stage's own comment and progress
    // message still named it — the same "a comment is not a contract" defect
    // this repo's CI parity gate had. Comment lines and the lines that only
    // *print* a command are excluded.
    let invocations: Vec<&str> = hook
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.starts_with('#'))
        .filter(|l| l.contains("--staged"))
        .filter(|l| {
            !(l.starts_with("log_step") || l.starts_with("printf") || l.starts_with("echo"))
        })
        .collect();
    assert!(
        !invocations.is_empty(),
        "the commit hook no longer runs the staged citation stage — no line \
         invokes it (mentions in comments and progress messages do not count)"
    );
    assert!(
        hook.contains("ledger-citations.sh"),
        "the commit hook no longer names the citation gate script"
    );

    let gate = std::fs::read_to_string(root.join("scripts/gates/ledger-citations.sh"))
        .expect("read scripts/gates/ledger-citations.sh");
    let trees: Vec<&str> = gate
        .lines()
        .find(|l| l.trim_start().starts_with("PROSE_TREES=("))
        .map(|l| {
            l.trim()
                .trim_start_matches("PROSE_TREES=(")
                .trim_end_matches(')')
                .split_whitespace()
                .collect()
        })
        .expect("the gate declares PROSE_TREES");
    assert!(
        !trees.is_empty(),
        "PROSE_TREES is empty — the gate would read nothing"
    );
    let restated: Vec<&&str> = trees.iter().filter(|t| hook.contains(**t)).collect();
    assert!(
        restated.is_empty(),
        "the commit hook names covered tree(s) itself: {restated:?}\n\
         The gate script owns the scope; a second copy is how the two ends \
         drift apart."
    );
}
