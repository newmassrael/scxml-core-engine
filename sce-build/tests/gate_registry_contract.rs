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

use common::workflow::{gate_slugs_invoked, job_text, split_workflow, workflow_texts};
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
/// ⚠ The rows became `engine:language` PAIRS on 2026-08-30. This backend emits
/// machines for two script-engine languages and the committed tree holds one
/// of them, so an engine reading the other one needs its own tree generated —
/// which is what `generate-w3c --script-engine` and `-Psce.generated.overlay`
/// exist for. An engine alone stopped identifying a run at that point: `lua`
/// over ECMAScript machines and `lua` over lowered Lua machines are two
/// different code paths through the same engine.
///
/// This test keeps the ENGINE half — that the declared engines include the
/// ones with nowhere else to be covered, that a per-pair floor exists, and
/// that a pair's failure reaches the gate's verdict.
///
/// The population claim splits in two along the seam, and each half is
/// asserted where its answer lives. The GENERATOR side — every language this
/// backend can emit an artifact for has a row — is
/// `the_kotlin_gate_runs_every_language_the_generator_can_emit`, just below.
/// The ENGINE side — every language each engine will accept has a row — is
/// `GateEnginePairsTest`, on the JVM, because it is
/// `ScxmlScriptEngine.acceptsLanguage` that answers it. Neither half implies
/// the other: an emittable artifact nothing runs and a running engine nothing
/// emits for are different holes.
///
/// Comments are stripped first. The reasons for each of these live in prose
/// directly above the lines they explain, and a scan that counted those would
/// pass on a gate that had deleted the code and kept the paragraph.
/// The Kotlin gate with its comments removed.
///
/// Every check below reads code, never prose: a scan that counted the
/// paragraphs would pass on a gate that had deleted the loop and kept the
/// explanation of it.
fn kotlin_gate_code() -> String {
    let body = std::fs::read_to_string(repo_root().join("scripts/gates/w3c-kotlin.sh"))
        .expect("the Kotlin gate is readable");
    body.lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `engine:language` rows the gate declares.
///
/// Parsed once and shared, because two readers of one array are two chances
/// to read a different array than the loop runs.
fn kotlin_gate_pairs(code: &str) -> Vec<String> {
    code.lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("KOTLIN_ENGINE_PAIRS=(")?
                .strip_suffix(')')
                .map(|inner| {
                    inner
                        .split_whitespace()
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
        })
        .expect(
            "the Kotlin gate must declare the engine/language pairs it runs in \
             one place, as `KOTLIN_ENGINE_PAIRS=(…)`, so this check and the \
             loop cannot disagree",
        )
}

/// The body of the loop that runs one engine/language row.
///
/// Structural rather than textual, because "the gate mentions the verdict
/// somewhere" and "the gate asks for it once per row" are different claims and
/// only the second one is worth anything: `$REPORTS` is a single directory
/// rewritten by every row, so a check hoisted out of this loop reads whichever
/// run finished last and reports it as all four.
fn kotlin_gate_row_loop(code: &str) -> String {
    let lines: Vec<&str> = code.lines().collect();
    let gradle = lines
        .iter()
        .position(|line| line.contains("./gradlew --console=plain :sce-kotlin-tests:test"))
        .expect(
            "the Kotlin gate must run the suite through `./gradlew --console=plain \
             :sce-kotlin-tests:test`, which is the invocation every row is built around",
        );
    let start = lines[..gradle]
        .iter()
        .rposition(|line| line.starts_with("for pair in"))
        .expect("the gradle invocation sits inside a loop over the engine/language rows");
    let end = start
        + 1
        + lines[start + 1..]
            .iter()
            .position(|line| *line == "done")
            .expect("the row loop is closed");
    lines[start..=end].join("\n")
}

/// The language half of each row, refusing a row that is not a pair.
fn kotlin_gate_row_half(
    pairs: &[String],
    half: for<'a> fn((&'a str, &'a str)) -> &'a str,
) -> Vec<String> {
    pairs
        .iter()
        .map(|pair| {
            let split = pair.split_once(':').unwrap_or_else(|| {
                panic!(
                    "row `{pair}` is not `engine:language`. The engine alone \
                     does not say which artifact it was handed, and the two \
                     routes into the Lua engine differ by exactly that."
                )
            });
            half(split).to_string()
        })
        .collect()
}

#[test]
fn the_kotlin_gate_runs_every_engine_it_claims() {
    let code = kotlin_gate_code();
    let pairs = kotlin_gate_pairs(&code);

    let engines = kotlin_gate_row_half(&pairs, |(engine, _)| engine);

    // `lua` joined this list on 2026-08-30, and its absence had been the
    // gate's own standing note: the engine ships, `EngineFactory` offers it
    // beside the other two, and no lane measured it above the expression
    // level. What had blocked it was mechanical — the committed tree is
    // emitted for one language — and generating per language is what lifted
    // it.
    for required in ["rhino", "quickjs", "lua"] {
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

    // Per-pair, not once at the end: the reports directory holds whichever run
    // finished last, so a verdict read outside the loop describes one pair and
    // vouches for all of them.
    //
    // ⚠ THIS USED TO DEMAND A CASE-COUNT FLOOR (`if (( cases < 200 ))`), on
    // the premise that "Gradle reports BUILD SUCCESSFUL for a test task that
    // ran nothing, so without the floor a pair's arm can report green over an
    // empty run". The premise is TRUE and was measured rather than argued on
    // 2026-09-02 — one row's filter narrowed to a class that does not exist,
    // `BUILD SUCCESSFUL in 15s` over an executed `:sce-kotlin-tests:test`,
    // the report directory left present holding no `TEST-*.xml`. What the
    // measurement also showed is that the CONCLUSION does not follow —
    // `(( cases < 200 ))` refuses `cases=0` as loudly as anything else, and
    // the gate refused that arm without it, naming all 251 classes that never
    // reported.
    //
    // ⚠⚠ So the floor was retired rather than loosened, and what replaced it
    // is strictly stronger in the direction the floor was weak: a floor of 200
    // over a suite of 373 passes while 173 cases stop running, and the
    // comparison against the derived class set refuses the FIRST class that
    // stops reporting. Demanding the floor back would be demanding the weaker
    // of the two.
    //
    // ⚠⚠⚠ And the replacement is not held by this assertion reading a line of
    // bash. `scripts/gates/kotlin_coverage.py` is that verdict as a program,
    // and `sce-build/tests/kotlin_coverage_verdict.rs` hands it an empty arm
    // and requires the refusal — the first time this gate's run-time logic has
    // been reachable by anything but a hand-bought red
    // (`no_shell_runner_reaches_a_gates_own_logic`). What is left for this file
    // is the DELEGATION: that the row loop still asks, once per row.
    let coverage = repo_root().join("scripts/gates/kotlin_coverage.py");
    assert!(
        coverage.is_file(),
        "no coverage program at {} — the gate reads every row's verdict from \
         it, and the cases that measure that verdict have nothing to run",
        coverage.display()
    );
    assert!(
        repo_root()
            .join("sce-build/tests/kotlin_coverage_verdict.rs")
            .is_file(),
        "⚠ the cases that measure the coverage verdict are gone. The gate \
         delegates its per-row refusal to `scripts/gates/kotlin_coverage.py` \
         precisely so that refusal can be exercised without running Gradle; \
         without them the delegation asserted below points at a program \
         nothing measures."
    );
    assert!(
        code.contains("\"$KOTLIN_COVERAGE\" derive"),
        "⚠ the gate no longer derives the set of test classes that must run. \
         Every row's verdict is a comparison against that set, and without it \
         there is nothing for a row to be complete with respect to."
    );
    let rows = kotlin_gate_row_loop(&code);
    assert!(
        rows.contains("\"$KOTLIN_COVERAGE\" verdict"),
        "⚠ the per-row coverage verdict is not asked inside the loop over \
         KOTLIN_ENGINE_PAIRS. The reports directory holds whichever run \
         finished last, so a verdict asked once at the end describes one pair \
         and vouches for all of them — and a row that ran nothing would be \
         covered by whichever row ran everything."
    );
    assert!(
        code.contains(r#"sce_gate_fail "Kotlin conformance on $engine over $language machines"#),
        "⚠ a pair's failures must fail the gate rather than be printed. A loop \
         that reports each result and returns the last one passes whenever the \
         final pair passes — and that is a pair already covered."
    );

    // The language the committed machines hold is DERIVED, never written down.
    // A constant here would be correct until the day it matters most — the day
    // `Language::Kotlin.default_script_engine_target()` flips — and silently
    // wrong after it, which is the failure this whole pairing exists to
    // prevent.
    assert!(
        code.contains("COMMITTED_LANGUAGE=\"$("),
        "⚠ the gate no longer derives which language the committed Kotlin \
         machines are emitted for; a literal there is a second answer beside \
         the generator's, free to disagree with it exactly when the default \
         moves."
    );
    assert!(
        !code.contains("COMMITTED_LANGUAGE=ecmascript") && !code.contains("COMMITTED_LANGUAGE=lua"),
        "⚠ the gate assigns COMMITTED_LANGUAGE a literal. That is the constant \
         the derivation replaced."
    );

    // A generated tree byte-identical to the committed one means the
    // `--script-engine` selection never reached the templates, and the row
    // built on it would hand its engine the OTHER language under this one's
    // name — a green row measuring the route it exists to stop measuring.
    //
    // Asserted here because the refusal is the ONLY thing standing between
    // that and a pass. Every other guard in this gate is answered by
    // something downstream: a floor that goes missing shows up as an empty
    // run, a verdict that stops failing shows up as a failing pair. A
    // comparison that stops comparing shows up as nothing at all, since both
    // trees then run and both pass.
    assert!(
        code.contains(r#"if diff -rq "$COMMITTED_MACHINES" "$machines""#),
        "⚠ the gate no longer compares a generated tree against the committed \
         machines. Without that comparison, a `generate-w3c --script-engine` \
         that accepted the flag and emitted its default anyway would give this \
         gate a row running the committed language under the other one's name, \
         and every case in it would pass."
    );
}

/// Every artifact this backend can emit is run by some row of the gate.
///
/// The generator half of the population claim, and the half that can be
/// answered in Rust: `Language::Kotlin.supports_script_engine_target` is what
/// decides whether `generate-w3c --script-engine <lang>` produces a suite at
/// all, so the set of languages it says yes to is exactly the set of Kotlin
/// artifacts that exist to be run. A language in that set with no row is an
/// artifact SCE ships and no lane executes — the shape this gate carried as a
/// written-down debt for the whole time `lua` was missing from its array.
///
/// ⚠ Containment, not equality, and the asymmetry is the point. A row naming
/// a language the backend cannot emit fails LOUDLY the first time the gate
/// runs: `generate-w3c` refuses through the same `supports_script_engine_target`
/// call, so nothing needs asserting here. A language the array OMITS fails
/// nowhere at all, which is why it is the direction worth a test.
///
/// ⚠⚠ This is the assertion that survives the day the backend's default
/// flips. `default_script_engine_target()` moving to Lua changes which
/// artifact is committed and which one the gate generates, but not the SET
/// this reads — both languages stay supported, so both stay required, and the
/// row that would have been quietly dropped in the swap turns this red.
#[test]
fn the_kotlin_gate_runs_every_language_the_generator_can_emit() {
    use sce_build::generator::{Language, ScriptEngineTarget};

    let code = kotlin_gate_code();
    let pairs = kotlin_gate_pairs(&code);
    let declared: BTreeSet<String> = kotlin_gate_row_half(&pairs, |(_, language)| language)
        .into_iter()
        .collect();

    // Read off the enum's own population rather than a list restated here: a
    // third engine language reaches this sweep the moment it reaches the wire
    // vocabulary, without an edit that someone has to remember to make.
    let emittable: BTreeSet<String> = ScriptEngineTarget::ALL
        .iter()
        .filter(|target| Language::Kotlin.supports_script_engine_target(**target))
        .map(|target| target.wire_name().to_string())
        .collect();

    assert!(
        !emittable.is_empty(),
        "the Kotlin backend reports no emittable script-engine language at \
         all, which cannot be true while it generates anything — a backend \
         always supports its own default. An empty population would make the \
         containment below hold for an empty gate."
    );

    for language in &emittable {
        assert!(
            declared.contains(language),
            "⚠ `sce-codegen generate-w3c -l kotlin --script-engine {language}` \
             produces a conformance suite, and no row of KOTLIN_ENGINE_PAIRS \
             runs it. The gate declares {declared:?} against an emittable \
             {emittable:?}. An artifact SCE can emit and no lane executes is \
             the exact state the `lua` arm of this gate sat in while its \
             omission was a paragraph of prose above the array."
        );
    }
}

/// The Kotlin lane CLASSIFIES every refusal, in both directions.
///
/// ⚠ The measurement this holds did not exist before 2026-08-30. The seam
/// document had said for weeks that `EcmaScriptToLuaTransformer` is still the
/// fallback behind every lowering entry point — *"empty is not retired"* — and
/// nothing counted the fallback, so the distance from here to
/// `kotlin-retire-rewriter` was a sentence rather than a figure. Measured the
/// day it landed: the frontend answered **50667** expressions across the four
/// engine/language pairs and the rewriter **100**.
///
/// ⚠⚠ **THE COUNT THEN RETIRED WITH ITS SUBJECT, and this test had to move
/// with it.** `kotlin-retire-rewriter` closed the same week: the fallback is
/// deleted, so `rewriter=0` is now STRUCTURAL — no change to this tree can
/// raise or lower it — and a ceiling over a value nothing can move is a gate
/// that cannot fail. What those four call sites became is a REFUSAL, and
/// refusals cannot be capped either, because §scxml-5.9.1 makes some of them
/// correct behaviour. So the gate matches each refused TEXT against a declared
/// entry, both ways, and this test holds that pair of comparisons.
///
/// ⚠⚠⚠ THE FLOOR IS NOT DECORATION, and it is the half a reader is most likely
/// to delete as redundant. A census that never arrives reports zero refusals
/// in exactly the way a backend with nothing to refuse does. That is not
/// hypothetical: the first two attempts at this measurement wrote to
/// `System.err`, Gradle swallowed the test JVM's stderr, and BOTH runs
/// reported `0` over a run that took the fallback 100 times. Asserting the
/// frontend's successes is what separates "nothing left to refuse" from
/// "nobody measured".
#[test]
fn the_kotlin_lane_classifies_every_refusal() {
    let gate = std::fs::read_to_string(repo_root().join("scripts/gates/w3c-kotlin.sh"))
        .expect("the Kotlin W3C gate script is readable");
    let code: String = code_lines(&gate).collect::<Vec<_>>().join("\n");

    // ⚠ The COMPARISONS, not the names. Asserting that a constant OCCURS is
    // satisfied by the line that assigns it, so a gate that compared against a
    // literal million while the constant sat unused would pass — measured
    // 2026-08-30, exactly that mutation SURVIVED this test's first form. Both
    // directions are named separately because either one alone is a one-sided
    // list: without `undeclared` a new gap is silently absorbed, and without
    // `unseen` a declaration outlives the case it describes.
    assert!(
        code.contains("undeclared=\"$(comm -23"),
        "⚠ `scripts/gates/w3c-kotlin.sh` no longer compares the refusals this \
         run OBSERVED against the declared list. Without that comparison a \
         text the frontend newly refuses is absorbed in silence, and the whole \
         point of recording refusals is that each one is either a shape to \
         teach, a caller to re-tag, or the specification working."
    );
    assert!(
        code.contains("unseen=\"$(comm -13"),
        "⚠ the Kotlin gate checks that observed refusals are declared and no \
         longer that declared refusals are OBSERVED. A one-sided list rots: an \
         entry whose case has been repaired goes on claiming the engine still \
         refuses it, and a reader consults that file to decide whether their \
         document stays inside what this backend covers."
    );
    assert!(
        code.contains("frontend_hits < FRONTEND_FLOOR"),
        "⚠⚠ the Kotlin gate classifies refusals and no longer asserts the \
         FRONTEND's successes. Those two readings are what tell a backend with \
         nothing to refuse from a census that never happened — both report no \
         refusals. Measured 2026-08-30: a stderr-based probe reported 0 \
         fallbacks twice over a run that took the fallback 100 times, because \
         Gradle swallowed the stream."
    );
    assert!(
        code.contains("no lowering census at"),
        "⚠ the Kotlin gate no longer fails when the census FILE is absent. An \
         absent file is the loudest form of the same blindness the floor \
         guards against, and it is the one a `build.gradle.kts` edit produces."
    );
}

/// A declared refusal names PRODUCERS, and each one resolves to something the
/// tree still carries.
///
/// ⚠ Its own test rather than an assertion inside the one above, because the
/// two hold different things and neither implies the other. That one holds the
/// census against the declared list — observed against declared, both ways.
/// This one holds the declared list against the TREE, and a list can satisfy
/// either while failing the other: a text the engine still refuses goes on
/// being declared, correctly, by an entry whose stated reason points at a
/// fixture that was retired three rounds ago.
///
/// ⚠⚠ WHY IT HAD TO BECOME A PREDICATE. Every entry in
/// `tests/ecmascript/kotlin_frontend_refusals.json` ends with a sentence of
/// the shape *"THIS ENTRY DOES NOT LEAVE while test307 is registered"*. Until
/// 2026-08-30 that was prose, and prose is what this repository has twice
/// watched rot in this very file's neighbourhood — a per-call figure quoted
/// from a `/tmp` probe deleted when its round ended, and "159 of 382" lifted
/// out of a neighbouring measurement and reused as a lane's size. Both read as
/// measured. What separated them from a measurement was that nothing asked the
/// tree, which is exactly what `produced_by` now makes the gate do.
///
/// ⚠⚠⚠ AND IT NEEDS THIS ANCHOR RATHER THAN A BEHAVIOURAL WITNESS. Delete the
/// resolution from the gate and nothing downstream looks different: the census
/// still matches the declared texts, both `comm` directions still pass, the
/// ceiling is still zero, and the lane stays green over a file whose reasons
/// have quietly stopped being checkable. A guard whose removal changes nothing
/// visible is one that needs its own predicate — the rule this repository
/// arrived at when a `diff -rq` that had stopped comparing left both trees
/// running and both trees passing.
///
/// The two populations are named separately because they do not imply each
/// other: a reader that resolved only fixtures would accept `NoSuchTest`, and
/// one that resolved only classes would accept fixture `999`.
#[test]
fn a_declared_refusal_names_producers_that_exist() {
    let gate = std::fs::read_to_string(repo_root().join("scripts/gates/w3c-kotlin.sh"))
        .expect("the Kotlin W3C gate script is readable");
    let code: String = code_lines(&gate).collect::<Vec<_>>().join("\n");

    assert!(
        code.contains("entry.get(\"produced_by\")"),
        "⚠ `scripts/gates/w3c-kotlin.sh` no longer reads `produced_by`, so \
         every entry's stated reason is back to being prose. An entry may then \
         name a fixture that has been retired, and the lane goes on passing \
         while a reader follows the reason to nothing."
    );
    assert!(
        code.contains("name not in fixtures"),
        "⚠⚠ the Kotlin gate no longer resolves a numeric producer against the \
         conformance registry. Retiring a W3C fixture has to take the refusal \
         entries that rest on it, and this comparison is the only thing that \
         says so — nothing else in the lane reads the registry for this."
    );
    assert!(
        code.contains("name not in classes"),
        "⚠⚠ the Kotlin gate no longer resolves a named producer against the \
         Kotlin test sources. Renaming or deleting a test class would leave \
         its refusal entry pointing at a class that does not exist, and the \
         census cannot notice: it records the refused TEXT and not its caller."
    );
}

/// The Kotlin test task forwards the census property to the test JVM.
///
/// ⚠ A Gradle `Test` task FORKS. `-Dsce.lua.loweringCensus=…` handed to Gradle
/// reaches Gradle's own JVM and not the tests, so the property has to be
/// forwarded deliberately. Without the forward the engine's census helper
/// returns on its first line, no file is written, and the lane's ceiling
/// passes over a run that measured nothing — which is why the gate's
/// file-absent check and this one are a pair rather than a duplicate.
#[test]
fn the_kotlin_test_task_forwards_the_lowering_census() {
    let build = std::fs::read_to_string(repo_root().join("backends/kotlin/tests/build.gradle.kts"))
        .expect("the Kotlin test module's build file is readable");
    let code: String = build
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        code.contains("systemProperty(\"sce.lua.loweringCensus\""),
        "⚠ `backends/kotlin/tests/build.gradle.kts` no longer forwards \
         `sce.lua.loweringCensus` to the test JVM. The property named on the \
         Gradle command line then reaches Gradle and stops there, the engine \
         writes no census, and the lane's rewriter ceiling is satisfied by a \
         run that recorded nothing."
    );
    assert!(
        code.contains("gradleProperty(\"sce.lua.loweringCensus\")"),
        "⚠ the census property is set from something other than the Gradle \
         property the gate passes. A hard-coded path would write one run's \
         census over another's; the gate names a directory of its own per run."
    );
}

/// The Kotlin suite re-runs when the gate it reads changes.
///
/// `GateEnginePairsTest` holds `KOTLIN_ENGINE_PAIRS` to the routes this backend
/// actually has, and it reads the gate script off disk. Gradle knows nothing
/// about that read: the gate is not on the compile classpath, so without an
/// explicit input declaration the test task stays UP-TO-DATE across every edit
/// to the file the test is about.
///
/// Measured 2026-08-30, before the declaration existed: deleting the `lua:lua`
/// row — the row the change adding it exists to add — left
/// `./gradlew :sce-kotlin-tests:test --tests GateEnginePairsTest` reporting
/// BUILD SUCCESSFUL in 500ms with no test run. The same shape the shared
/// ECMA-262 tables already carry, and for the same reason: a gate nothing
/// re-reads is a gate that cannot be wrong.
///
/// Asserted here rather than in Kotlin because a suite cannot observe its own
/// staleness — the run that would report it is the run that does not happen.
#[test]
fn the_kotlin_suite_reruns_when_the_gate_it_reads_changes() {
    let build = std::fs::read_to_string(repo_root().join("backends/kotlin/tests/build.gradle.kts"))
        .expect("the Kotlin test build file is readable");
    let code: String = build
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        code.contains("scripts/gates/w3c-kotlin.sh"),
        "⚠ `backends/kotlin/tests/build.gradle.kts` no longer declares \
         scripts/gates/w3c-kotlin.sh as an input of the test task. \
         `GateEnginePairsTest` reads that file to check the gate covers every \
         engine route, and Gradle cannot see the read: editing the gate then \
         leaves the suite UP-TO-DATE and the check unrun."
    );
}

/// The Kotlin action dispatch never re-evaluates a guard.
///
/// ⚠ THE DEFECT THIS FORBIDS WAS REAL, and it was invisible to every suite in
/// the tree until an artifact was compiled and run. `transition_actions.kt.jinja2`
/// keyed each arm on the transition's `cond` and event pattern — the same
/// `cond` `process_event.kt.jinja2` had already evaluated to CHOOSE that
/// transition. A pure guard answers the same twice and nothing shows. A guard
/// with a side effect does not: measured 2026-08-30, `cond="++v == 2"` took its
/// transition on the first evaluation (`++v` -> 2) and then failed the second
/// (`++v` -> 3), so the machine ran the other arm's content.
///
/// The arms are keyed on `transitionIndex` now. This test is what keeps them
/// there: a scan for the shapes that would mean a guard is being decided again
/// at dispatch time.
///
/// ⚠⚠ Comments are stripped first. This file's own header explains the defect
/// at length and names `safeEvaluateGuard` while doing so — a raw `contains`
/// would read the explanation as the thing it warns about. The same reading
/// error `reach_of` made on 2026-08-30, one file over.
#[test]
fn the_kotlin_action_dispatch_switches_on_the_selection_it_was_given() {
    let template = std::fs::read_to_string(
        repo_root().join("tools/codegen/templates/kotlin/transition_actions.kt.jinja2"),
    )
    .expect("the Kotlin transition-actions template is readable");

    // Jinja comments span lines, so they are cut as a block; `#` line comments
    // never appear in this template's Kotlin output.
    let mut code = String::new();
    let mut rest = template.as_str();
    while let Some(open) = rest.find("{#") {
        code.push_str(&rest[..open]);
        rest = match rest[open..].find("#}") {
            Some(close) => &rest[open + close + 2..],
            None => "",
        };
    }
    code.push_str(rest);

    assert!(
        code.contains("when (transitionIndex)"),
        "⚠ the Kotlin action dispatch no longer switches on the transition the \
         SELECTION chose. Whatever it switches on instead has to be re-derived \
         at dispatch time, and the only thing there is to re-derive it from is \
         the guard — which is the defect (`++v == 2`, measured 2026-08-30)."
    );
    for forbidden in [
        "safeEvaluateGuard",
        "render_cond",
        "cond_kt",
        "cond_constant",
    ] {
        assert!(
            !code.contains(forbidden),
            "⚠ `{forbidden}` is back in tools/codegen/templates/kotlin/\
             transition_actions.kt.jinja2. Every one of these spellings is a way \
             of DECIDING AGAIN, at action-dispatch time, which transition ran — \
             and the selection has already decided it. A guard with a side \
             effect answers differently the second time, so the machine takes \
             one transition and runs another's content."
        );
    }
}

/// The lowered lane regenerates when the GENERATOR changes, not just the input.
///
/// ⚠ Measured 2026-08-30, and it is the lane's own subject turned against it.
/// `GenerateLoweredArtifact` declared the generator as `@get:Input
/// codegen: Property<String>` — the PATH. A `cargo build` that changed what the
/// generator emits therefore left the task UP-TO-DATE, and the suite compiled
/// and measured machines from the PREVIOUS generator while reporting on the
/// current tree.
///
/// It surfaced only by luck: the change under test had also moved a runtime
/// signature, so the stale Kotlin failed to compile. A change that altered
/// emitted BEHAVIOUR and nothing else would have gone green over the old
/// artifact — which is precisely the reading this lane refuses everywhere else
/// (`--rerun` on the test task, the census demand, the pair counts).
///
/// The fix is to declare the binary's CONTENT, and this holds it.
#[test]
fn the_lowered_lane_regenerates_when_the_generator_itself_changes() {
    let build = std::fs::read_to_string(
        repo_root().join("backends/kotlin/lowered-ecma262/build.gradle.kts"),
    )
    .expect("the lowered-artifact build file is readable");
    let code: String = build
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        code.contains("@get:InputFile\n    abstract val codegenBinary"),
        "⚠ `backends/kotlin/lowered-ecma262/build.gradle.kts` no longer \
         declares the generator BINARY as an input of the generation task. \
         With only its path declared, rebuilding `sce-codegen` leaves the \
         task UP-TO-DATE and this lane measures machines the previous \
         generator emitted — a green about a tree that is no longer the one \
         on disk."
    );
    assert!(
        code.contains("codegenBinary.set(File(it))"),
        "⚠ the generation task declares a `codegenBinary` input and never \
         sets it. An unset `@InputFile` is not a stricter check than no \
         input at all; it is a configuration failure or an ignored property, \
         and either way the staleness it was added to catch goes on passing."
    );
}

/// The Kotlin lowered-artifact lane proves its two artifacts are different.
///
/// The lane compiles ONE document twice, with `--script-engine lua` and with
/// `--script-engine ecmascript`, and runs both. Both answer the same 98 cases
/// and — with `kotlin_lua_divergences.json` empty on both routes — answer them
/// the same way, so the SUITE cannot tell a real pair from a subject compared
/// against itself. What can is the shape of the emitted machines, and the gate
/// is where that is read.
///
/// ⚠ The floor is asserted separately from the mirror equalities, because the
/// mirror alone is satisfied by the failure it exists to catch. A
/// `--script-engine lua` that accepted the flag and emitted the default anyway
/// produces two IDENTICAL machines — and identical machines mirror each other
/// perfectly, with both counts sitting down at the run-time helper's single
/// arm. Only the floor says the subject carries hundreds of lowered call
/// sites.
///
/// ⚠⚠ And the count is not `== 0` on the control, which was the first
/// spelling and was measured wrong the same day: a generated machine emits
/// BOTH arms of the helper that re-wraps a `ScriptSource` it was handed, so
/// one occurrence of each spelling appears in every machine whatever it was
/// generated for. `w3c-kotlin` recorded the same trap from the other side,
/// where counting one tree read 159 against 159.
#[test]
fn the_kotlin_lowered_gate_proves_its_control_is_not_its_subject() {
    let code: String =
        std::fs::read_to_string(repo_root().join("scripts/gates/ecma262-lowered-kotlin.sh"))
            .expect("the Kotlin lowered-artifact gate is readable")
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");

    assert!(
        code.contains("if (( lowered_lua < 50 )); then"),
        "⚠ the floor on the subject's lowered call sites is gone. Without it a \
         `--script-engine lua` that emitted the backend's default anyway would \
         give this lane two identical machines, which satisfy every mirror \
         equality below and pass every case — a control compared against itself."
    );
    assert!(
        code.contains("if (( lowered_lua != source_ecma )); then"),
        "⚠ the gate no longer checks that the subject's lowered call sites and \
         the control's source-passing ones are the SAME COUNT. Two selections \
         of one document carry one call site for one call site; without this \
         they need not even be the same document."
    );

    // The verdict is read from the run's OUTPUT, so the run has to happen.
    assert!(
        code.contains(":sce-kotlin-lowered-ecma262:test --rerun"),
        "⚠ the gate no longer forces the suite to run. Gradle answers an \
         unchanged test task UP-TO-DATE or FROM-CACHE and produces no output, \
         and this lane reads its census and its probe controls out of that \
         output — measured 2026-08-30 as `the suite printed no census line` \
         over an artifact that was perfectly fine."
    );
    // ⚠ The CHECK and its consequence, held ADJACENTLY, and the adjacency is
    // the whole point. This assertion was first written as
    // `code.contains("the suite printed no census line")` — the failure
    // MESSAGE alone — and a mutation round measured it blind on 2026-08-30:
    // appending `|| true` to the test leaves the message in the file, so the
    // guard was dead and the oracle was green. It is the shape this repository
    // has already been bitten by, and names elsewhere as "the code dies and the
    // short-circuit survives".
    // ⚠ The escape hatch this lane INTRODUCED, held from outside the file that
    // declares it. `kotlin_lowered_artifact_defects.json` excuses a case from
    // answering ECMA-262, which is the one thing here that can be green over a
    // wrong artifact. The suite has `MAX_DEFECTS`, but that constant sits in
    // the same file as the assertion reading it — raise it and the assertion
    // agrees. The gate keeps a second ceiling, in another language, over the
    // count the suite PRINTED, and the two do not imply each other: one counts
    // entries in the file, the other counts entries that actually excused a
    // case in the population.
    assert!(
        code.contains("if (( excused > DEFECT_CEILING )); then"),
        "⚠ the exclusion list may now grow without limit. Every entry in \
         tests/ecmascript/kotlin_lowered_artifact_defects.json excuses a case \
         from answering ECMA-262, so a list with no ceiling is a way to make \
         this lane green over an artifact that answers nothing — and the \
         suite's own MAX_DEFECTS cannot be the only ceiling, because it lives \
         in the file that reads it."
    );
    assert!(
        code.contains(
            "[ -n \"$census\" ] \\\n    || sce_gate_fail \"the suite printed no census line"
        ),
        "⚠ a run that produced no census is now accepted. That is the reading \
         a cached green produces, and it is the one state where this lane \
         reports on a run that did not happen. Note this holds the TEST and \
         its `sce_gate_fail` together: a message with no live check behind it \
         is what this assertion exists to refuse."
    );
}

/// Every gate that derives a JDK floor names a workflow that declares one.
///
/// `sce_gate_require_jdk` reads its floor out of a workflow's `java-version:`
/// and REFUSES when it finds none — which is the right runtime behaviour and
/// is not a check anyone runs before pushing. A workflow that stopped pinning
/// a version would leave the two Kotlin gates with no floor, and Gradle free to
/// run on whatever JVM the machine exports. On this fleet that has been a JDK 8
/// while `java --version` said 17, failing inside a build script with a message
/// that names no JDK.
///
/// ⚠ The population is DERIVED from the gate scripts rather than listed here,
/// so a third gate that starts deriving a floor is covered on the commit that
/// adds it. The floor on the population is what keeps that derivation honest:
/// a scan that silently matched nothing would assert nothing, and would pass.
#[test]
fn every_gate_that_derives_a_jdk_floor_names_a_workflow_that_pins_one() {
    let root = repo_root();
    let dir = root.join("scripts/gates");
    let mut pinned: Vec<(String, String)> = Vec::new();
    let mut offenders: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {}", dir.display(), e))
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|x| x.to_str()) != Some("sh") {
            continue;
        }
        let body = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        for line in body.lines() {
            let line = line.trim_start();
            // Comments describe the call; they are not one. Without this the
            // helper's own documentation in `lib.sh` would be read as a caller.
            if line.starts_with('#') {
                continue;
            }
            let Some(rest) = line.strip_prefix("sce_gate_require_jdk ") else {
                continue;
            };
            let workflow = rest.trim().trim_matches('"').replace("$SCE_REPO_ROOT/", "");
            pinned.push((
                path.file_name().unwrap().to_string_lossy().into_owned(),
                workflow,
            ));
        }
    }

    // Two gates drive Gradle today. A floor rather than an equality: a third
    // may be added and must not have to edit this number. What it refuses is
    // the scan matching NOTHING, which would make every assertion below
    // vacuous — the failure mode this repository calls a gate that reports on
    // a population it never built.
    assert!(
        pinned.len() >= 2,
        "the scan over scripts/gates/*.sh found {} caller(s) of \
         `sce_gate_require_jdk`, and there are at least two (w3c-kotlin and \
         ecma262-lowered-kotlin). A scan that matches nothing asserts nothing \
         and passes, so this refuses the empty reading rather than reporting \
         on it.",
        pinned.len()
    );

    for (gate, workflow) in &pinned {
        let path = root.join(workflow);
        let Ok(body) = std::fs::read_to_string(&path) else {
            offenders.push(format!(
                "{gate} derives its JDK floor from {workflow}, which is not readable"
            ));
            continue;
        };
        let pin = body.lines().find_map(|line| {
            let line = line.trim_start();
            let rest = line.strip_prefix("java-version:")?;
            let value = rest.trim().trim_matches('\'').trim_matches('"');
            value.chars().next().filter(char::is_ascii_digit)?;
            Some(value.to_string())
        });
        match pin {
            Some(_) => {}
            None => offenders.push(format!(
                "{gate} derives its JDK floor from {workflow}, which declares no \
                 numeric `java-version:`"
            )),
        }
    }

    assert!(
        offenders.is_empty(),
        "⚠ {} gate(s) derive a JDK floor from a workflow that no longer pins \
         one. `sce_gate_require_jdk` would refuse at run time, which is a \
         failed push rather than a caught edit:\n  {}",
        offenders.len(),
        offenders.join("\n  ")
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
/// The first address `tools/git-hooks/ident-gate.sh` accepts, asked OF the gate.
///
/// Sourced rather than copied. A second literal spelling of the allow-list
/// would go stale the moment that list changes, and it would go stale
/// SILENTLY: a fixture that stops satisfying a precondition fails as the stage
/// it never reached, which is the shape this whole helper exists to prevent.
fn allowed_ident_email(root: &Path) -> String {
    let gate = root.join("tools/git-hooks/ident-gate.sh");
    let out = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "source {}; printf '%s' \"${{SCE_ALLOWED_IDENT_EMAILS[0]}}\"",
            gate.display()
        ))
        .output()
        .unwrap_or_else(|e| panic!("source {}: {e}", gate.display()));
    assert!(
        out.status.success(),
        "could not read the allow-list out of {}: {out:?}",
        gate.display()
    );
    let email = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(
        email.contains('@'),
        "the gate's allow-list yielded no address ({email:?}), so every fixture \
         below would set an identity the gate refuses and fail as the stage it \
         never reached"
    );
    email
}

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
    // The IDENTITY precondition, fixed by the fixture instead of inherited
    // from whoever is watching.
    //
    // `pre-commit` sources `ident-gate.sh` before Stage 0 and refuses when it
    // cannot read the identity a commit would carry. A fresh fixture
    // repository has none of its own, so the gate fell through to the global
    // config: on a developer machine that answered and the hook ran, and on a
    // hosted runner with no identity `git var GIT_AUTHOR_IDENT` failed.
    // Measured 2026-08-26 on `6937e04f38` — green locally, red in BOTH `Rust
    // Workspace Tests` and `Tree Hygiene`, and red as "the hook did not reach
    // the citation stage", which reads as a defect in the stage under test.
    let allowed = allowed_ident_email(&root);
    run(&["config", "user.name", "sce fixture"]);
    run(&["config", "user.email", &allowed]);
    // A lower bound on the precondition itself, because "we wrote the config"
    // and "git will stamp this" are different claims: `GIT_AUTHOR_EMAIL` and
    // `GIT_COMMITTER_EMAIL` in the environment override the two lines above,
    // and `git var` is what the gate itself grades. Asserting here names the
    // environment as the cause; leaving it to the hook names the citation
    // stage, which is not what broke.
    for role in ["GIT_AUTHOR_IDENT", "GIT_COMMITTER_IDENT"] {
        let out = Command::new("git")
            .args(["var", role])
            .current_dir(dir)
            .output()
            .unwrap_or_else(|e| panic!("git var {role}: {e}"));
        let ident = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success() && ident.contains(&format!("<{allowed}>")),
            "the fixture repository would stamp {role} as {ident:?}, not \
             <{allowed}> — the hook's identity gate refuses that and the tests \
             below would fail as the stage they never reached. Check the \
             environment: GIT_AUTHOR_EMAIL / GIT_COMMITTER_EMAIL override the \
             config this fixture just wrote."
        );
    }
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
            //
            // ⚠ `code_lines`, not a raw `contains` over the body, and the
            // difference was measured on 2026-08-30. A gate script that merely
            // MENTIONS the corpus directory in a comment — `ecma262-lowered-
            // kotlin.sh`, explaining which casefile mutates it — was read as
            // running the whole corpus's oracles, and this suite then reported
            // that its lane must install the pinned validator. It does not run
            // them; it talks about them. The two carriers that really do
            // (`mutation-rounds.sh`, `mutation-cases.sh`) both assign the path
            // to `CORPUS=`, so the code reading keeps them and drops the
            // sentence. This repository has recorded the same lesson from the
            // other side: a scanner has to strip comments before it matches.
            if code_lines(&body).any(|l| l.contains("sce-build/tests/mutations")) {
                reach.named.extend(corpus_oracle_targets(root));
            }
        }
    }
    reach
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

/// A name the registry does not carry is a FAILURE, not an empty run.
///
/// The runner has one exit path for "no gate applies", and it is exit 0
/// because that is the honest answer when a change set selects nothing. A
/// named invocation used to reach the same path by a completely different
/// route: `mapfile -t ORDER < <(python3 …)` reports `mapfile`'s status and
/// never the resolver's, so the resolver refused an unknown slug with exit 2,
/// left `ORDER` empty, and fell into "nothing to verify".
///
/// Measured 2026-08-29, by walking into it: `scripts/gate clang-format` — a
/// slug this tree has never had, because that lane is a workflow with no gate
/// script — printed `unknown slug(s)` on stderr and returned SUCCESS. A typo
/// in a lane, in a hook or in a person's hand was reported as a gate that
/// passed, by the runner that owns every gate in the tree.
///
/// The stderr check is not decoration. `scripts/gate` refuses for several
/// reasons before it ever resolves a name, and a non-zero status alone would
/// be satisfied by any of them — this case has to know it REACHED the
/// resolver, or it is asserting that the script can fail rather than that it
/// fails here.
#[test]
fn a_slug_the_registry_does_not_carry_is_not_a_quiet_pass() {
    let out = Command::new(repo_root().join("scripts/gate"))
        .arg("no-such-gate-this-tree-has-never-had")
        .current_dir(repo_root())
        .output()
        .expect("scripts/gate runs");

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown slug"),
        "`scripts/gate` did not reach its slug resolver — it refused for some \
         earlier reason, so this case says nothing about what an unknown slug \
         does.\nstderr: {stderr}"
    );
    assert!(
        !out.status.success(),
        "`scripts/gate` said `unknown slug` and exited SUCCESSFULLY. Every \
         caller — a hook, a lane, a person — reads the status, so a name with \
         a typo in it is reported as a gate that ran and passed."
    );
}
