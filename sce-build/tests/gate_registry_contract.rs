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

mod common;

use common::workflow::{job_text, split_workflow};
use std::collections::BTreeSet;
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
fn no_ci_lane_asks_the_runner_to_choose_an_order() {
    // `cost_s` is measured on a WARM tree, and the registry declares that as a
    // deliberate basis: a push happens on a tree the developer has just built.
    // The open question against it was whether a COLD profile would order the
    // gates differently — deferred, at the time, until CI ran the gates through
    // the runner, which would be the way to find out.
    //
    // CI does now, and the answer measured 2026-08-12 is that the question is
    // empty. `run_order` is applied to a SET of gates, and the only selectors
    // that produce a set are the pre-push hook's path-scoped selection and
    // `--all` — both local, both warm. Every CI step names exactly one slug, so
    // its position is authored in the workflow rather than derived, and a cold
    // profile orders nothing.
    //
    // That is held here rather than remembered, because it is load-bearing: a
    // lane that handed the runner a set would put a warm-derived order in
    // charge of a cold run, and nothing else would notice.
    let dir = repo_root().join(".github/workflows");
    // Anything the shell would end the command at. A slug never looks like one.
    const ENDS_THE_COMMAND: [&str; 6] = ["|", ";", "&&", "||", ">", "2>&1"];
    let mut invocations = 0usize;
    let mut violations: Vec<String> = Vec::new();

    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .collect();
    files.sort();

    for path in &files {
        let body = std::fs::read_to_string(path).expect("read workflow");
        for (n, line) in body.lines().enumerate() {
            // Comments first. These files carry prose about how the gates are
            // run — `cpp-suite.yml` records the `--all` run that produced its
            // measurements — and a scanner that reads a sentence as a command
            // reports the opposite of the truth. That is the same "a comment
            // is not a contract" defect this suite has had twice.
            if line.trim_start().starts_with('#') {
                continue;
            }
            let tokens: Vec<&str> = line.split_whitespace().collect();
            // The bare token only. A `paths:` entry spells it `'scripts/gate'`,
            // which is the file being watched, not a command being run.
            let Some(at) = tokens.iter().position(|t| *t == "scripts/gate") else {
                continue;
            };
            let args: Vec<&str> = tokens[at + 1..]
                .iter()
                .take_while(|t| !t.starts_with('#') && !ENDS_THE_COMMAND.contains(t))
                .copied()
                .collect();
            invocations += 1;
            let where_ = format!(
                "{}:{}",
                path.file_name().expect("named file").to_string_lossy(),
                n + 1
            );
            if args.len() != 1 || args[0].starts_with('-') {
                violations.push(format!("{where_}: scripts/gate {}", args.join(" ")));
            }
        }
    }

    // A parse that stopped matching would find nothing and report a clean
    // sweep. The floor is what a working parse saw, and it may only grow.
    assert!(
        invocations >= 25,
        "found only {invocations} runner invocation(s) across {} workflow(s) — \
         the scan is broken, not the lanes",
        files.len()
    );
    assert!(
        violations.is_empty(),
        "a CI lane hands the runner a gate SET, so a warm-measured cost_s \
         would decide the order of a cold run: {violations:?}"
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
fn the_reporting_switch_the_workspace_lane_sets_is_the_one_the_gate_reads() {
    // The lane kept its own copy of `cargo test` for one flag —
    // `--no-fail-fast`, which its own registry note called "a reporting
    // choice, not a different verification". A choice is a switch, so the gate
    // took the switch and the lane delegated. Both halves are asserted because
    // either one alone is silent: a lane that stops setting it loses the
    // reporting it wanted, and a gate that stops reading it ignores the lane.
    let root = repo_root();
    let gate = std::fs::read_to_string(root.join("scripts/gates/workspace-tests.sh"))
        .expect("read scripts/gates/workspace-tests.sh");
    let lane = std::fs::read_to_string(root.join(".github/workflows/rust-workspace-tests.yml"))
        .expect("read .github/workflows/rust-workspace-tests.yml");

    assert!(
        gate.lines().any(|l| {
            let t = l.trim();
            !t.starts_with('#') && t.contains("SCE_GATE_NO_FAIL_FAST")
        }),
        "the workspace gate no longer reads SCE_GATE_NO_FAIL_FAST, so the \
         lane's reporting choice reaches nothing"
    );
    assert!(
        gate.contains("--no-fail-fast"),
        "the workspace gate no longer passes --no-fail-fast under any setting"
    );
    assert!(
        lane.lines()
            .any(|l| l.trim().starts_with("SCE_GATE_NO_FAIL_FAST:")),
        "the workspace lane no longer sets SCE_GATE_NO_FAIL_FAST, so CI \
         reports only the first failure of a run nobody can iterate on"
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
fn a_gate_that_regenerates_a_tree_pins_the_timestamp() {
    // A gate must not break the gate after it.
    //
    // `sce-codegen generate-*` stamps `generated-at` into every file it
    // writes, from the wall clock unless SOURCE_DATE_EPOCH says otherwise.
    // Two of these trees are also read by `committed_trees_carry_a_pinned_
    // generated_at` in the workspace suite, which runs later in the same
    // push: the first gate regenerated, the fourth reported 451 unpinned
    // files, and the push failed on a defect the developer did not write.
    // `scripts/regen_all_committed_trees.sh` has exported the variable for
    // this reason all along — nothing required a gate to.
    let dir = repo_root().join("scripts/gates");
    let mut generating = 0usize;
    let mut offenders: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .expect("gates dir is readable")
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) != Some("sh") {
            continue;
        }
        let Ok(body) = std::fs::read_to_string(&path) else {
            continue;
        };
        let regenerates = body.lines().any(|l| {
            let l = l.trim();
            !l.starts_with('#')
                && (l.contains("generate-w3c") || l.contains("generate-integration"))
        });
        if !regenerates {
            continue;
        }
        generating += 1;
        if !body.contains("SOURCE_DATE_EPOCH") {
            offenders.push(
                path.file_name()
                    .expect("gate file name")
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }
    assert!(
        generating > 0,
        "no gate was seen regenerating a tree — the scan is broken, not the gates"
    );
    assert!(
        offenders.is_empty(),
        "gate(s) regenerate a tree without pinning SOURCE_DATE_EPOCH, so a later \
         gate in the same push sees wall-clock `generated-at` churn: {offenders:?}"
    );
}

/// The Kotlin arm covers every engine its name claims, and each one's verdict
/// reaches the gate's.
///
/// A generated conformance case cannot see which script engine it reached Pass
/// on — every one of the 226 asserts the same thing about a document. So the
/// only place "this lane covered QuickJS" can be checked is where the lane is
/// written, and until it was, the arm ran the default engine and said so in a
/// step name nobody diffs.
///
/// What makes the loop worth asserting rather than trusting is that narrowing
/// it back is invisible: a gate covering one engine passes exactly as a gate
/// covering two does. So does one whose per-engine floor stopped being a floor
/// — Gradle reports BUILD SUCCESSFUL for a test task it decided was UP-TO-DATE
/// — and so does one that prints an engine's failures and carries on.
///
/// Comments are stripped first. The reasons for each of these live in prose
/// directly above the lines they explain, and a scan that counted those would
/// pass on a gate that had deleted the code and kept the paragraph.
#[test]
fn the_kotlin_gate_runs_every_engine_it_claims() {
    let body = std::fs::read_to_string(repo_root().join("scripts/gates/w3c-kotlin.sh"))
        .expect("the Kotlin gate is readable");
    let code: String = body
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");

    let engines = code
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("KOTLIN_ENGINES=(")?
                .strip_suffix(')')
                .map(|inner| {
                    inner
                        .split_whitespace()
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
        })
        .expect(
            "the Kotlin gate must declare the engines it runs in one place, as \
             `KOTLIN_ENGINES=(…)`, so this check and the loop cannot disagree",
        );

    for required in ["rhino", "quickjs"] {
        assert!(
            engines.iter().any(|e| e == required),
            "⚠ the Kotlin gate runs {engines:?}, without `{required}`. The W3C \
             suite is the only place an engine is exercised against the \
             GENERATED machines — session lifecycle, setCurrentEvent, \
             executeForeach, the In() callback — and the shared ECMA-262 table \
             asks none of that. Dropping an engine here leaves that surface \
             covered on no lane."
        );
    }

    // The registry's one-line summary is what a reader sees in `scripts/gate
    // --list`, so it must not name a coverage the script stopped providing.
    let registry = std::fs::read_to_string(repo_root().join("tools/git-hooks/gate_registry.py"))
        .expect("the registry is readable");
    let summary = registry
        .lines()
        .skip_while(|line| !line.contains("\"w3c-kotlin\": {"))
        .find_map(|line| line.trim().strip_prefix("\"summary\": "))
        .expect("the w3c-kotlin entry carries a summary")
        .to_lowercase();
    for engine in &engines {
        assert!(
            summary.contains(engine.as_str()),
            "the registry summarises w3c-kotlin as {summary}, which does not \
             name `{engine}` — the gate runs it and the one-line description a \
             reader gets does not say so"
        );
    }

    // Per-engine, not once at the end: the reports directory holds whichever
    // engine ran last, so a floor or a verdict read outside the loop describes
    // one engine and vouches for all of them.
    assert!(
        code.contains("if (( cases < 200 )); then"),
        "⚠ the per-engine floor on case count is gone. Gradle reports BUILD \
         SUCCESSFUL for a test task that ran nothing, so without the floor an \
         engine's arm can report green over an empty run."
    );
    assert!(
        code.contains(r#"sce_gate_fail "Kotlin conformance on $engine"#),
        "⚠ an engine's failures must fail the gate rather than be printed. A \
         loop that reports each result and returns the last one passes whenever \
         the final engine passes — and that is the engine already covered."
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

/// Build a throwaway git repository with `body` staged at `rel`.
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
///
/// The staged paths are a parameter because the stage's coverage used to depend
/// on two lists at once — a tree list in the gate and an extension list in the
/// checker — and a fixture pinned to one `.rs` path under `tests/` sits inside
/// both, so it could not see either boundary move. Several files stage into one
/// fixture because the stage reads the whole staged set: one gate run covers
/// every boundary, so a path per fixture would spawn the gate once per path for
/// no added coverage.
fn staged_citation_fixture_files(dir: &Path, files: &[(&str, &str)]) {
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
    // Every regular file in the hook directory, not `pre-commit` alone. The
    // hook sources its siblings out of its OWN directory on purpose — its
    // comment says "so it resolves the same way when the hook is invoked
    // against another tree", which is exactly this fixture — so a copy naming
    // one file dies on the first `source` a later stage adds, before reaching
    // the stage under test. Measured: `ident-gate.sh` landed in `ca9210ea8b`
    // and left this fixture failing at `pre-commit` line 68 with "no such file
    // or directory", which reads as a defect in the citation stage it never
    // got to. A sweep cannot go stale the next time a stage grows a sibling.
    let hooks = root.join("tools/git-hooks");
    for entry in std::fs::read_dir(&hooks).expect("read tools/git-hooks") {
        let entry = entry.expect("read a tools/git-hooks entry");
        if !entry.file_type().expect("stat a hook entry").is_file() {
            continue;
        }
        let name = entry.file_name();
        std::fs::copy(entry.path(), dir.join("tools/git-hooks").join(&name))
            .unwrap_or_else(|e| panic!("copy tools/git-hooks/{}: {e}", name.to_string_lossy()));
    }
    assert!(
        dir.join("tools/git-hooks/pre-commit").is_file(),
        "the sweep copied no pre-commit, so every hook test below would fail \
         for the fixture's reason rather than the hook's"
    );
    // `.github` joins `docs` and the adoption tools because the stage has two
    // halves now: the existence sweep these fixtures exercise, and the binding
    // axes, which resolve the rev-pinned mnemosyne binary out of
    // `.github/workflows/spec-citations.yml`. A fixture without the pin would
    // make the stage report "no MNEMOSYNE_REV pin found" — and the tempting fix
    // (skip the binding half when the pin is absent) is a silent bypass: every
    // checkout that lost the workflow file would commit unjudged.
    #[cfg(unix)]
    for d in ["tools/mnemosyne-adoption", "docs", ".github"] {
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
    for (rel, body) in files {
        let target = dir.join(rel);
        // The fixture symlinks `tools/` and `docs/` at the real checkout, so a
        // fixture path under either one writes THROUGH the link into the
        // working tree this test is supposed to leave alone. Observed: a
        // `docs/probe.md` fixture landed in the repository, and `git add`
        // refusing it ("pathspec is beyond a symbolic link") came after the
        // file already existed. Refuse before writing rather than after.
        let mut walk = dir.to_path_buf();
        for part in Path::new(rel).components() {
            walk.push(part);
            assert!(
                !walk.is_symlink(),
                "fixture path {rel} traverses the symlink {}, so writing it \
                 would modify the real checkout",
                walk.display()
            );
        }
        std::fs::create_dir_all(target.parent().expect("fixture path has a parent"))
            .expect("mkdir fixture parent");
        std::fs::write(&target, body).expect("write fixture");
        run(&["add", rel]);
    }
}

/// The historical fixture path: a `.rs` file under `tests/`.
fn staged_citation_fixture(dir: &Path, body: &str) {
    staged_citation_fixture_files(dir, &[("tests/cite.rs", body)]);
}

/// The same fixture, but owning its ledger workspaces instead of borrowing them.
///
/// The base fixture symlinks `docs` at the real checkout, and the gate refuses
/// to judge a workspace that resolves outside the repository it was handed —
/// rightly, since a store in another checkout cannot answer for this one's
/// staged files. The consequence for a probe is that the binding loop skips all
/// five workspaces by name, so a test written against the base fixture never
/// reaches the code it means to exercise and reads as evidence anyway.
///
/// Copied, not created empty: the base fixture's own comment says the migrator
/// derives its repo root from `tools/` and so reads the real stores, and that
/// is measurably not what happens — with an empty `docs` the existence half
/// reported `.atomic/workspace.atomic.json` missing UNDER THE FIXTURE and
/// stopped the run before the binding half. 1.8M, which is what it costs to
/// have both halves see one repository.
fn staged_citation_fixture_owning_a_workspace(dir: &Path, files: &[(&str, &str)]) {
    staged_citation_fixture_files(dir, files);
    let docs = dir.join("docs");
    std::fs::remove_file(&docs).expect("drop the symlinked docs tree");
    let out = Command::new("cp")
        .arg("-R")
        .arg(repo_root().join("docs"))
        .arg(&docs)
        .output()
        .expect("copy the ledger workspaces into the fixture");
    assert!(out.status.success(), "cp -R docs: {out:?}");
}

/// The revision the gate pins, read from the file the gate reads it from.
///
/// Hardcoding it here would make this test the second reader of a fact that
/// moves, and the pin moves on its own schedule: a bump would leave the stub
/// announcing a revision the gate rejects, and the test would then pass for the
/// wrong reason — a refusal to run, dressed as the refusal under study.
fn pinned_mnemosyne_short_rev() -> String {
    let workflow =
        std::fs::read_to_string(repo_root().join(".github/workflows/spec-citations.yml"))
            .expect("read .github/workflows/spec-citations.yml");
    let rev = workflow
        .lines()
        .find_map(|l| l.trim().strip_prefix("MNEMOSYNE_REV:"))
        .map(|v| v.trim().trim_matches(['"', '\''].as_slice()).to_string())
        .expect("the workflow declares MNEMOSYNE_REV");
    assert_eq!(rev.len(), 40, "MNEMOSYNE_REV is a full sha: {rev}");
    rev[..8].to_string()
}

/// Write an executable stub for the pinned validator.
///
/// `tail` is the script body that runs for every subcommand other than
/// `--version`, which each stub has to answer with the pinned revision — the
/// gate rejects a binary that reports any other one, and would do so before
/// reaching the code under study.
///
/// A stub rather than the real binary because the condition under study is a
/// validator that exits non-zero for a reason that is not the author's text.
/// The real tool collapses every failure into exit 1 — measured 2026-08-12: a
/// missing `mnemosyne.toml`, a `--paths` value outside the workspace and an
/// unknown flag all return 1 — so the gate cannot read the cause off the
/// status, and a stub is the only way to hold that cause fixed and observe
/// what the gate does with it.
fn stub_mnemosyne_cli(dir: &Path, name: &str, tail: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(
        &path,
        format!(
            "#!/usr/bin/env bash\n\
             if [ \"${{1:-}}\" = \"--version\" ]; then\n\
             \x20   echo 'mnemosyne-cli 0.1.0 ({rev})'\n\
             \x20   exit 0\n\
             fi\n\
             {tail}\n",
            rev = pinned_mnemosyne_short_rev(),
        ),
    )
    .expect("write the stub binary");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("chmod the stub binary");
    }
    path
}

fn run_staged_gate(dir: &Path) -> std::process::Output {
    Command::new("bash")
        .arg(dir.join("scripts/gates/ledger-citations.sh"))
        .arg("--staged")
        .current_dir(dir)
        .output()
        .expect("run the staged citation gate")
}

/// The staged gate with `MNEMOSYNE_BIN` pointed somewhere of the test's choosing.
fn run_staged_gate_with_binary(dir: &Path, bin: &Path) -> std::process::Output {
    Command::new("bash")
        .arg(dir.join("scripts/gates/ledger-citations.sh"))
        .arg("--staged")
        .env("MNEMOSYNE_BIN", bin)
        .current_dir(dir)
        .output()
        .expect("run the staged citation gate")
}

/// Exit status 3 is `sce_gate_cannot_run`: the gate's own tooling is missing,
/// so it measured nothing about the tree.
///
/// Every "the gate accepts a real citation" assertion below has to tell that
/// apart from a rejection, because the two look identical through
/// `status.success()` and only one of them is about the author. On a machine
/// without the rev-pinned `mnemosyne-cli` this file used to report "the staged
/// gate rejected a real citation" — a verdict about the fixture for a fault in
/// the checker's own inputs.
fn assert_the_gate_accepted(out: &std::process::Output, what: &str) {
    if out.status.code() == Some(3) {
        // `what` describes the rejection this assertion is here to catch, so
        // it must not be printed on this branch: there was no rejection to
        // describe. Prefixing it produced "the staged gate rejected a real
        // citation: the gate could not run", which contradicts itself in one
        // line and puts the blaming half first.
        panic!(
            "the gate could not run — its own tooling is missing, so this \
             says nothing about the citation or the tree. Install the \
             rev-pinned binary the message below names, then re-run:\n{}",
            String::from_utf8_lossy(&out.stderr),
        );
    }
    assert!(out.status.success(), "{what}: {out:?}");
}

/// The verdict the gate must not reach when it did not measure it: that the
/// author's staged text carries a citation nothing binds.
fn asserts_the_author_is_at_fault(stderr: &str) -> bool {
    stderr.contains("binding does not hold")
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
    assert_the_gate_accepted(&good, "the staged gate rejected a real citation");
}

#[test]
fn the_staged_binding_half_blames_the_tool_when_the_tool_is_what_is_missing() {
    // Observed 2026-08-12, on this repository: the mnemosyne pin was raised and
    // the binary was not yet installed under the new revision's path, and the
    // gate reported "a staged file carries a citation whose binding does not
    // hold" — a verdict about the author's text, for a fault in the gate's own
    // inputs. The author's citations were fine.
    //
    // The mechanism was placement. `sce_citation_binary` fails loudly when the
    // binary is absent, but it was called from inside a command substitution
    // inside a subshell, where its `exit` ends the substitution and nothing
    // else: the substitution then expanded to the empty string, the empty
    // string ran as a command, and its 127 arrived at an `||` that had one
    // sentence to say. Resolution belongs in the shell that can act on it.
    //
    // The sibling block twelve lines up already stated this rule for the
    // existence half — "reporting that as a bad citation would blame the
    // author's text for a fault in the gate's own inputs" — and the binding
    // half did the thing that comment forbids.
    let dir = tempfile::tempdir().expect("tempdir");
    staged_citation_fixture_owning_a_workspace(
        dir.path(),
        &[("tests/cite.rs", "// probe: §synth-5-B is a section\n")],
    );
    let absent = dir.path().join("no-such-mnemosyne-cli");
    let out = run_staged_gate_with_binary(dir.path(), &absent);
    let err = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "a gate that cannot resolve its validator must not pass — an \
         unjudged commit that reads as judged is the failure this whole \
         stage exists to prevent: {out:?}"
    );
    assert!(
        err.contains("no rev-pinned mnemosyne-cli at"),
        "the gate did not name the missing tool, so the author is left to \
         guess at a fault that is not theirs: {err}"
    );
    assert!(
        !asserts_the_author_is_at_fault(&err),
        "the gate blamed the author's citations for its own missing \
         validator: {err}"
    );
    // Prose is what a human reads; the exit status is what every other caller
    // reads, and until this was distinct the two disagreed. The gate said "no
    // rev-pinned mnemosyne-cli at ..." and exited 1, so `assert_the_gate_
    // accepted` above — and `scripts/gate`, and CI — all saw the same status a
    // genuinely bad citation produces. Two tests in this very file then
    // reported the build machine's missing tool as "the staged gate rejected a
    // real citation".
    assert_eq!(
        out.status.code(),
        Some(3),
        "a gate whose own tooling is missing must exit 3 (`sce_gate_cannot_run`), \
         not 1: exit 1 is the status that says the tree is at fault, and a \
         caller cannot tell the two apart from prose alone: {out:?}"
    );
}

#[test]
fn the_staged_binding_half_reports_what_the_validator_said() {
    // The second half of the same defect, and the one no placement fixes: the
    // validator collapses every failure into exit 1. Measured 2026-08-12
    // against the pinned binary — a missing `mnemosyne.toml`, a `--paths` value
    // outside the workspace and an unknown flag all exit 1, exactly as a real
    // unbound citation does. A status of 1 therefore does not carry a cause,
    // and a gate that names one is stating something it did not measure.
    //
    // So the gate surfaces the validator's own report instead of paraphrasing
    // it. That is checked from behaviour: the stub fails with a message only a
    // broken *input* produces, and the gate has to pass it through rather than
    // translate it into a verdict about staged text.
    let dir = tempfile::tempdir().expect("tempdir");
    staged_citation_fixture_owning_a_workspace(
        dir.path(),
        &[("tests/cite.rs", "// probe: §synth-5-B is a section\n")],
    );
    const CANNOT_RUN: &str =
        "error: mnemosyne.toml not found — CWD or ancestor in config file required";
    // Through a file, so the message stays a fixture value rather than
    // something that has to survive two levels of shell quoting.
    let message = dir.path().join("stub-failure.txt");
    std::fs::write(&message, format!("{CANNOT_RUN}\n")).expect("write the stub's message");
    let stub = stub_mnemosyne_cli(
        dir.path(),
        "stub-cannot-run",
        &format!("cat {} >&2\nexit 1", message.display()),
    );
    let out = run_staged_gate_with_binary(dir.path(), &stub);
    let err = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "the gate passed although its validator refused to run: {out:?}"
    );
    assert!(
        err.contains(CANNOT_RUN),
        "the gate discarded the validator's report, which is the only place \
         the cause is visible once the exit status cannot carry it: {err}"
    );
    assert!(
        !asserts_the_author_is_at_fault(&err),
        "the gate turned a validator that could not run into a verdict about \
         the author's citations: {err}"
    );
}

#[test]
fn the_staged_binding_half_still_runs_the_validator_it_resolved() {
    // The paired direction. Every assertion above is satisfied by a gate that
    // fails unconditionally, and one that never invokes the validator at all
    // would satisfy them too — the fixture owns a workspace precisely so the
    // loop body is entered, and a gate that skipped it would look identical
    // from the outside. A stub that succeeds has to leave the gate green, and
    // it has to have been ASKED: the stub records its own invocation.
    let dir = tempfile::tempdir().expect("tempdir");
    staged_citation_fixture_owning_a_workspace(
        dir.path(),
        &[("tests/cite.rs", "// probe: §synth-5-B is a section\n")],
    );
    let witness = dir.path().join("stub-was-asked");
    let stub = stub_mnemosyne_cli(
        dir.path(),
        "stub-accepts",
        &format!("echo \"$@\" >> {}\nexit 0", witness.display()),
    );

    let out = run_staged_gate_with_binary(dir.path(), &stub);
    assert!(
        out.status.success(),
        "the gate rejected a staged file its validator accepted: {out:?}"
    );
    let asked = std::fs::read_to_string(&witness).unwrap_or_default();
    assert!(
        asked.contains("validate-code-refs"),
        "the gate never ran the binding axes, so the failure assertions above \
         would hold for a gate that checks nothing: asked={asked:?}"
    );
}

#[test]
fn the_push_run_names_the_validator_that_rejected() {
    // The push half ran its four validators as one `&&` chain, so every one of
    // them failed as "${ws} ledger validation" with its output sent to
    // /dev/null. Two things an author needs were missing from that: which axis
    // refused, and what it said. The chain reads as economical and costs a
    // whole diagnosis.
    let dir = tempfile::tempdir().expect("tempdir");
    staged_citation_fixture_owning_a_workspace(
        dir.path(),
        &[("tests/cite.rs", "// probe: §synth-5-B is a section\n")],
    );
    let message = dir.path().join("stub-failure.txt");
    std::fs::write(&message, "error: the store is unreadable\n").expect("write the stub's message");
    let stub = stub_mnemosyne_cli(
        dir.path(),
        "stub-cannot-run",
        &format!("cat {} >&2\nexit 1", message.display()),
    );
    let out = Command::new("bash")
        .arg(dir.path().join("scripts/gates/ledger-citations.sh"))
        .env("MNEMOSYNE_BIN", &stub)
        .current_dir(dir.path())
        .output()
        .expect("run the citation gate");
    let err = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "the gate passed on a refusal: {out:?}"
    );
    assert!(
        err.contains("validate-workspace"),
        "the failure does not name the validator that refused, so the author \
         is told a workspace is bad and not which axis said so: {err}"
    );
    assert!(
        err.contains("error: the store is unreadable"),
        "the validator's own report was discarded: {err}"
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

    // The gate script owns the citation scope; a second copy in the hook is how
    // the two ends drift apart. That used to be checked by reading the gate's
    // tree list and asserting the hook restated none of it — which stopped
    // meaning anything the moment the scope became "every tracked file", and
    // which never covered the OTHER list that decided coverage: the checker's
    // set of readable extensions. A text scan cannot tell this stage's file
    // list from the C/C++ stage's legitimate one either, so the property is
    // asserted from behaviour in
    // `the_staged_stage_scope_is_not_bounded_by_path_or_extension` and in the
    // end-to-end hook test, both of which stage a file no list ever named.
}

#[test]
fn the_staged_stage_scope_is_not_bounded_by_path_or_extension() {
    // Measured 2026-08-11: the gate named `web/` among the trees it swept and
    // read 0 of its 46 tracked files, because the checker only read extensions
    // it could REWRITE — so seven fabricated section numbers were "checked" at
    // every push and every commit. Both boundaries are asserted here from
    // behaviour: a fixture at a path no tree list mentioned, in a file type no
    // rewrite rule covers, has to be rejected all the same. One staged set per
    // direction, since the stage reads all of it in one run.
    const PATHS: [&str; 3] = ["web/probe.js", "notes.md", "probe.toml"];
    let fabricated = "probe: §synth-F4 is not a section\n";
    let real = "probe: §synth-5-B is a section\n";

    let dir = tempfile::tempdir().expect("tempdir");
    let bad_files: Vec<(&str, &str)> = PATHS.iter().map(|p| (*p, fabricated)).collect();
    staged_citation_fixture_files(dir.path(), &bad_files);
    let bad = run_staged_gate(dir.path());
    assert!(
        !bad.status.success(),
        "the staged gate accepted a fabricated citation: {bad:?}"
    );
    let err = String::from_utf8_lossy(&bad.stderr);
    for rel in PATHS {
        assert!(
            err.lines()
                .any(|l| l.trim_start().starts_with(&format!("{rel}:"))),
            "the report must name {rel}, the path the author has to open: {err}"
        );
    }

    // The paired direction: a real citation in those same file types passes, so
    // the assertions above are not satisfied by a stage that fails everything.
    let dir = tempfile::tempdir().expect("tempdir");
    let good_files: Vec<(&str, &str)> = PATHS.iter().map(|p| (*p, real)).collect();
    staged_citation_fixture_files(dir.path(), &good_files);
    let good = run_staged_gate(dir.path());
    assert_the_gate_accepted(&good, "the staged gate rejected a real citation");
}

/// A gate that builds must be able to say why the build failed.
///
/// `cmake --build … >/dev/null` is the shape this catches. Ninja prints the
/// compiler's own output on STDOUT, so that redirection makes a build failure
/// undiagnosable: the gate log then holds the gate's one-line verdict and
/// nothing else. Measured 2026-08-19 on the build machine, a GCC internal
/// compiler error inside a mesh translation unit took three separate probes to
/// name, because `cpp-suite` reported "main tree build" and the error itself
/// had gone to /dev/null.
///
/// The rule is not "never redirect" — a passing gate's log should stay short.
/// It is that the redirection belongs in `sce_gate_build`, which captures the
/// output and emits its tail when, and only when, the build fails.
#[test]
fn no_gate_discards_the_output_of_a_build_it_runs() {
    let root = repo_root();
    let mut through_helper = 0usize;
    let mut offenders: Vec<String> = Vec::new();

    let mut files: Vec<PathBuf> = std::fs::read_dir(root.join("scripts/gates"))
        .expect("read scripts/gates")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("sh"))
        .collect();
    files.push(root.join("scripts/prepare_ctest_tree.sh"));
    files.sort();

    for path in &files {
        let Ok(body) = std::fs::read_to_string(path) else {
            continue;
        };
        let is_lib = path.file_name().and_then(|n| n.to_str()) == Some("lib.sh");
        for (lineno, line) in body.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                continue;
            }
            if trimmed.starts_with("sce_gate_build ") {
                through_helper += 1;
            }
            // The helper's own definition is the one place the raw command
            // belongs; it is what every other site calls.
            if is_lib || !trimmed.contains("cmake --build") {
                continue;
            }
            if line.contains("/dev/null") {
                offenders.push(format!(
                    "{}:{}: {}",
                    path.strip_prefix(&root).unwrap_or(path).display(),
                    lineno + 1,
                    trimmed
                ));
            }
        }
    }

    // The lower bound is the half a scan like this usually forgets: a walk that
    // stops finding build sites reads exactly like a clean tree. Six sites
    // called the helper when it was introduced; the floor sits below that so an
    // intentional removal does not fail here, while a scan that finds almost
    // nothing does.
    assert!(
        through_helper >= 4,
        "only {through_helper} gate build site(s) call sce_gate_build — the scan is \
         reading the wrong tree, or the helper was removed. Either way the check \
         below would prove nothing"
    );
    assert!(
        offenders.is_empty(),
        "a gate discards the output of a build it runs ({} site(s)):\n  {}\n\n\
         Ninja prints compiler errors on stdout, so a failure at one of these \
         leaves the gate log with nothing but the gate's own verdict. Call \
         `sce_gate_build <dir> [args...]` instead: it stays quiet on success and \
         emits the tail of the build's output when the build fails.",
        offenders.len(),
        offenders.join("\n  ")
    );
}

/// The helper's contract, exercised rather than described: a failing build's
/// output reaches the caller's stream.
///
/// A stub `cmake` stands in for the real one, because the property under test
/// is what `sce_gate_build` does with the output, not what any particular
/// compiler prints. The stub writes its marker to STDOUT — the stream the old
/// `>/dev/null` sites threw away — so a regression to that shape fails here
/// rather than the next time a build breaks on a build machine.
#[test]
fn a_failing_build_reaches_the_gate_log_through_the_helper() {
    let root = repo_root();
    let dir = tempfile::tempdir().expect("tempdir");

    // The stub speaks ninja's shape: a `FAILED:` edge, the compiler's message
    // under it, and then a run of unrelated lines, because ninja keeps its
    // other edges going after one fails. The trailing noise is the point — a
    // helper that showed the tail would report the noise and drop the cause,
    // which is what the first version of this helper did.
    let stub = dir.path().join("cmake");
    let mut body = String::from(
        "#!/usr/bin/env bash\n\
         echo 'FAILED: some/object.o'\n\
         echo 'error: NEEDLE_FROM_THE_COMPILER'\n",
    );
    for i in 0..120 {
        body.push_str(&format!(
            "echo '[{i}/994] Generating something unrelated'\n"
        ));
    }
    body.push_str("exit 1\n");
    std::fs::write(&stub, body).expect("write stub cmake");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755))
            .expect("chmod stub cmake");
    }

    // `lib.sh` turns on `set -e`, and the slug it prints is one it assigns
    // itself — so the status is taken in a `||` list (which `set -e` does not
    // act on) and the slug is set after the source rather than before it.
    let script = format!(
        "source '{}/scripts/gates/lib.sh'\n\
         SCE_GATE_SLUG=probe\n\
         PATH='{}':$PATH\n\
         sce_gate_build somewhere --target whatever || echo \"HELPER_RC=$?\"\n",
        root.display(),
        dir.path().display()
    );

    let out = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run the helper under a stub cmake");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        stdout.contains("HELPER_RC=1"),
        "the helper must report the build's failure to its caller; it said:\n{stdout}\n{stderr}"
    );
    assert!(
        stderr.contains("NEEDLE_FROM_THE_COMPILER"),
        "the failing build's own output did not reach the caller's stream. That is \
         what this helper exists for — the compiler's message goes to stdout, and a \
         gate that sends it to /dev/null reports a failure it cannot explain.\n\
         stderr was:\n{stderr}\nstdout was:\n{stdout}"
    );
}

/// The script a CI job runs to put the rev-pinned validator on the runner.
const MNEMOSYNE_INSTALLER: &str = "scripts/install_mnemosyne_cli.sh";

/// The lines of `text` that are code, with whole-line comments dropped.
///
/// Both alphabets this file scans — Rust test sources and shell gate scripts
/// — start a whole-line comment the same way once `//` and `#` are both
/// counted, and both carry prose that names the very things being searched
/// for. A scanner that reads its own comments has been the defect twice in
/// this repository, so the stripping is not optional politeness.
fn code_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("//") && !l.starts_with('#'))
}

/// Cargo test targets named on a command line, as `--test <name>`.
fn named_test_targets(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in code_lines(text) {
        let words: Vec<&str> = line.split_whitespace().collect();
        for pair in words.windows(2) {
            if pair[0] != "--test" {
                continue;
            }
            let name = pair[1].trim_matches(['"', '\'', '\\'].as_slice());
            if !name.is_empty() {
                out.insert(name.to_string());
            }
        }
    }
    out
}

/// Whether a source resolves the rev-pinned validator at all.
///
/// The needles are assembled at compile time so this predicate does not match
/// the file it lives in on its own search terms — the same guard
/// `workflow_trigger_coverage` uses for its signatures, and for the same
/// reason: a scanner that matches itself measures nothing.
fn resolves_the_pinned_validator(text: &str) -> bool {
    let pin = concat!("MNEMOSYNE", "_REV");
    let install_root = concat!("mnemosyne", "-rev");
    let gate_script = concat!("ledger-citations", ".sh");
    code_lines(text).any(|l| l.contains(pin) || l.contains(install_root) || l.contains(gate_script))
}

/// Test targets that reach for the rev-pinned `mnemosyne-cli`.
///
/// Derived from the sources rather than listed, because a list is a second
/// place to remember: a new suite that resolves the pin would be added to the
/// tree and not to the list, and the gate would read green over it. The
/// signature is the resolution itself — the pin's name, the revision-keyed
/// install root, or the gate script that resolves it in shell — since a suite
/// cannot reach that binary without spelling one of them.
///
/// This file matches its own signature, and must: two of its tests drive the
/// staged citation stage as a subprocess, and both fail without the binary.
fn targets_that_need_the_pinned_validator() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut out: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {}", dir.display(), e))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("rs"))
        .filter(|p| {
            let text = std::fs::read_to_string(p)
                .unwrap_or_else(|e| panic!("read {}: {}", p.display(), e));
            resolves_the_pinned_validator(&text)
        })
        .map(|p| {
            p.file_stem()
                .expect("a .rs file has a stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    out.sort();
    out
}

/// What a run script can execute: the test targets it names, plus whether it
/// sweeps the workspace — which runs every target there is, named or not.
#[derive(Default)]
struct Reach {
    named: BTreeSet<String>,
    sweeps_the_workspace: bool,
}

impl Reach {
    fn runs(&self, target: &str) -> bool {
        self.sweeps_the_workspace || self.named.contains(target)
    }
}

/// Fold shell line continuations, so a command split over several lines is
/// read as the one command it is.
fn joined_commands(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        let trimmed = line.trim_end();
        if let Some(head) = trimmed.strip_suffix('\\') {
            out.push_str(head);
            out.push(' ');
        } else {
            out.push_str(trimmed);
            out.push('\n');
        }
    }
    out
}

/// Every target the mutation corpus can hand its runner, read from the
/// casefiles' own `mutation_tests` declarations.
///
/// The rounds gate takes one casefile per run and the corpus decides which,
/// so what the JOB can execute is the union — a lane provisioned for today's
/// selection is provisioned for none of the others.
fn corpus_oracle_targets(root: &Path) -> BTreeSet<String> {
    let dir = root.join("sce-build/tests/mutations");
    let mut out = BTreeSet::new();
    let entries =
        std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {}", dir.display(), e));
    let mut casefiles = 0usize;
    for path in entries.flatten().map(|e| e.path()) {
        if path.extension().and_then(|x| x.to_str()) != Some("cases") {
            continue;
        }
        casefiles += 1;
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        for line in code_lines(&text).filter(|l| l.starts_with("mutation_tests")) {
            out.extend(named_test_targets(line));
        }
    }
    assert!(
        casefiles > 20,
        "read only {casefiles} casefile(s) from {} — the enumeration is broken, \
         not the corpus",
        dir.display()
    );
    out
}

/// What the commands in `text` reach, following `scripts/gate <slug>` into the
/// script it runs.
///
/// Following it matters because no CI job spells a cargo invocation any more:
/// every lane delegates to the gate runner, so a reader that stopped at the
/// workflow would see a job that runs no tests at all.
fn reach_of(text: &str, root: &Path, depth: usize) -> Reach {
    let mut reach = Reach::default();
    if depth > 4 {
        return reach;
    }
    let commands = joined_commands(text);
    for line in code_lines(&commands) {
        reach.named.extend(named_test_targets(line));
        if line.contains("cargo test") && line.contains("--workspace") {
            reach.sweeps_the_workspace = true;
        }
        // A dry run enumerates what a real one would do and executes none of
        // it, so it needs nothing installed. Reading the switch rather than
        // the slug keeps this from becoming a list of blessed jobs.
        if line.contains("_DRY_RUN=1") {
            continue;
        }
        for slug in gate_slugs_invoked(line) {
            let script = root.join("scripts/gates").join(format!("{slug}.sh"));
            let Ok(body) = std::fs::read_to_string(&script) else {
                continue;
            };
            let inner = reach_of(&body, root, depth + 1);
            reach.named.extend(inner.named);
            reach.sweeps_the_workspace |= inner.sweeps_the_workspace;
            // The rounds gate resolves its targets from the corpus at run
            // time, so its script names none of them.
            if body.contains("sce-build/tests/mutations") {
                reach.named.extend(corpus_oracle_targets(root));
            }
        }
    }
    reach
}

/// The gate slugs a command line runs, as `scripts/gate <slug> [<slug> …]`.
fn gate_slugs_invoked(line: &str) -> Vec<String> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let Some(at) = words
        .iter()
        .position(|w| *w == "scripts/gate" || w.ends_with("/scripts/gate"))
    else {
        return Vec::new();
    };
    words[at + 1..]
        .iter()
        // A flag, a redirection or a pipe ends the slug list: what follows is
        // the runner's own vocabulary, not a gate's name.
        .take_while(|w| !w.starts_with(['-', '>', '|', '&']))
        .map(|w| w.trim_matches(['"', '\''].as_slice()).to_string())
        .collect()
}

/// Every workflow as `(file name, text)`, sorted for stable diagnostics.
fn workflow_texts(root: &Path) -> Vec<(String, String)> {
    let dir = root.join(".github/workflows");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {}", dir.display(), e))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .map(|p| {
            let name = p
                .file_name()
                .expect("a workflow file has a name")
                .to_string_lossy()
                .into_owned();
            let text = std::fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("read {}: {}", p.display(), e));
            (name, text)
        })
        .collect();
    out.sort();
    assert!(
        out.len() > 5,
        "found {} workflow(s); the directory read is broken, not the tree",
        out.len()
    );
    out
}

#[test]
fn every_job_that_runs_the_pinned_validator_installs_it() {
    // Measured 2026-08-24 and again on 2026-08-25: `mutation-rounds.yml` ran
    // the corpus round whose oracle is this very suite, on a runner that had
    // no rev-pinned `mnemosyne-cli`. The staged citation stage refuses
    // without it — exit 3, "the gate could not run" — so the round stopped at
    // `baseline is not green (2 failing)` and said nothing about any of its
    // cases. Three casefiles share that oracle, so one missing install step
    // read as three rotten corners of the corpus.
    //
    // `rust-workspace-tests.yml` had carried the same defect and had it
    // repaired; the repair was not carried to the sibling lane, which is the
    // shape a per-lane memory always ends up in. Hence a derived rule.
    //
    // Per JOB and not per FILE, deliberately: each job gets its own runner,
    // so an install step in the neighbouring job provisions nothing here.
    // `mutation-rounds.yml` is exactly where that mistake is available — its
    // `select` job names the same gate, in a dry run that executes nothing.
    let root = repo_root();
    let needing: BTreeSet<String> = targets_that_need_the_pinned_validator()
        .into_iter()
        .collect();
    assert!(
        needing.len() >= 2,
        "the scan found {} suite(s) that resolve the rev-pinned validator; it \
         used to find two, so the signature has stopped matching and this \
         gate would pass by measuring nothing: {needing:?}",
        needing.len()
    );
    assert!(
        needing.contains("gate_registry_contract"),
        "the scan lost the suite it is running inside, which drives the staged \
         citation stage as a subprocess and demonstrably fails without the \
         binary. The signature is broken: {needing:?}"
    );

    let mut carriers: Vec<String> = Vec::new();
    let mut unprovisioned: Vec<String> = Vec::new();
    for (file, text) in workflow_texts(&root) {
        let (_, jobs) = split_workflow(&text);
        for job in &jobs {
            let body = job_text(job);
            let reach = reach_of(&body, &root, 0);
            let runs: Vec<&str> = needing
                .iter()
                .filter(|t| reach.runs(t))
                .map(String::as_str)
                .collect();
            if runs.is_empty() {
                continue;
            }
            carriers.push(format!("{file}:{}", job.id));
            if !body.contains(MNEMOSYNE_INSTALLER) {
                unprovisioned.push(format!(
                    "  {file}: job `{}` runs {} but never runs {MNEMOSYNE_INSTALLER}",
                    job.id,
                    runs.join(", ")
                ));
            }
        }
    }

    assert!(
        carriers.len() >= 3,
        "only {} job(s) were found to run a suite that needs the pinned \
         validator: {carriers:?}. Three lanes did — the workspace sweep, the \
         tree-wide gate and the mutation round — so a smaller number means \
         the reach derivation stopped following `scripts/gate`, not that CI \
         got simpler.",
        carriers.len()
    );
    assert!(
        unprovisioned.is_empty(),
        "CI job(s) run a suite that resolves the rev-pinned `mnemosyne-cli` on \
         a runner that never installs it:\n{}\n\
         Such a job cannot fail for a reason about the tree — the citation \
         gate exits 3, \"the gate could not run\", and every verdict built on \
         it is void. Add the cache and install steps that \
         `rust-workspace-tests.yml` carries to the job itself; an install in a \
         neighbouring job runs on a different machine.",
        unprovisioned.join("\n")
    );
}

/// Whether a source asks whether tools are required.
///
/// Assembled at compile time for the same reason the pin's needles are: a
/// predicate that matches its own file measures nothing.
fn consults_the_require_tools_switch(text: &str) -> bool {
    let needle = concat!("tools_are", "_required");
    code_lines(text).any(|l| l.contains(needle))
}

/// Test targets that answer a missing tool with a note instead of a failure.
///
/// The signature is the predicate itself. A suite that consults
/// [`sce_build::toolchain::tools_are_required`] is one that has TWO answers
/// for an absent tool, and the lenient one returns `ok` while measuring
/// nothing — invisible from outside the process.
fn targets_that_degrade_without_their_tool() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut out: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {}", dir.display(), e))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("rs"))
        .filter(|p| {
            let text = std::fs::read_to_string(p)
                .unwrap_or_else(|e| panic!("read {}: {}", p.display(), e));
            consults_the_require_tools_switch(&text)
        })
        .map(|p| {
            p.file_stem()
                .expect("a .rs file has a stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    out.sort();
    out
}

#[test]
fn a_suite_that_can_skip_is_run_by_a_job_that_forbids_skipping() {
    // The shell half of this pairing already exists: `hook_ci_parity` holds
    // that a lane running a gate script which can skip must set
    // `SCE_REQUIRE_TOOLS` and install the package. The Rust half had no
    // equivalent, and `ledger_symbol_axis_reach` is what it costs — its gap
    // check returns early with a note on stderr when the rev-pinned validator
    // is absent, and a runner that lost the binary would have reported `ok`
    // for a suite that compared its whole UNREACHED list against nothing.
    //
    // Not "every job that runs it", deliberately. A round in the mutation
    // corpus runs the suite too, and its `env:` is held to exactly one key by
    // `the_selection_narrows_and_the_round_only_obeys` — a second narrowing
    // channel is what that invariant exists to refuse, and a strictness switch
    // smuggled in beside it would be the same edit. One lane that forbids
    // skipping is what makes the check measured; the rest may stay lenient.
    let root = repo_root();
    let degrading = targets_that_degrade_without_their_tool();
    assert!(
        !degrading.is_empty(),
        "no suite was found to consult the require-tools switch. Either the \
         predicate was renamed — in which case this gate is now measuring \
         nothing — or the last skip-capable suite became unconditional, in \
         which case delete this test rather than leaving it green over an \
         empty set."
    );

    let mut unmeasured: Vec<String> = Vec::new();
    for target in &degrading {
        let mut strict_lanes: Vec<String> = Vec::new();
        let mut lenient_lanes: Vec<String> = Vec::new();
        for (file, text) in workflow_texts(&root) {
            let (_, jobs) = split_workflow(&text);
            for job in &jobs {
                let body = job_text(job);
                if !reach_of(&body, &root, 0).runs(target) {
                    continue;
                }
                let lane = format!("{file}:{}", job.id);
                // The key, not the word: a comment naming the variable is how
                // the neighbouring shell-side rule was first written and how
                // it first passed on a mutation that deleted the `env:` entry.
                if body
                    .lines()
                    .map(str::trim)
                    .any(|l| l.starts_with("SCE_REQUIRE_TOOLS:"))
                {
                    strict_lanes.push(lane);
                } else {
                    lenient_lanes.push(lane);
                }
            }
        }
        if strict_lanes.is_empty() {
            unmeasured.push(format!(
                "  {target}: run by {} lane(s), none setting SCE_REQUIRE_TOOLS ({})",
                lenient_lanes.len(),
                if lenient_lanes.is_empty() {
                    "no lane runs it at all".to_string()
                } else {
                    lenient_lanes.join(", ")
                }
            ));
        }
    }

    assert!(
        unmeasured.is_empty(),
        "suite(s) that answer a missing tool with a note are run by no CI job \
         that promotes that note into a failure:\n{}\n\
         Such a suite reports `ok` on a runner that lost the tool, so the \
         checks it is named for silently stop running. Set \
         SCE_REQUIRE_TOOLS on one lane that also installs the tool.",
        unmeasured.join("\n")
    );
}
