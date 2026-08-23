// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Trigger-coverage gate: a test whose input set is wider than any
// `paths:` filter could express must be run by a workflow that declares
// no filter at all.
//
// One direction of hook/CI agreement is already structural.
// `gate_registry.py` reads the `paths:` filters out of the workflows
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
/// `roadmap_marker_gate` and `codegen_binary_resolution` read every
/// tracked file; this test and `hook_ci_parity` read every workflow;
/// `cmake_option_guard_scope` reads every tracked CMake file, and a
/// directory acquiring a `CMakeLists.txt` is exactly the change a glob
/// list written today would not cover.
/// Adding an entry is a claim that the target's inputs cannot be
/// enumerated as globs — and it obliges the unfiltered workflow to run
/// that target by name.
const UNFILTERABLE_GATES: &[&str] = &[
    // Sweeps every tracked file under `scripts/` for a second spelling of the
    // parallel-jobs rule, so a script added anywhere under that tree changes
    // what it reads — and a NEW script carrying its own copy is the case the
    // gate exists for, which is exactly what a `paths:` filter written over
    // today's script list would stop it seeing.
    "build_jobs_has_one_owner",
    "cmake_option_guard_scope",
    "codegen_binary_resolution",
    "committed_sourcemap_drift",
    // Sweeps every committed *.scxml for a typed `<data>` that arrived
    // without a script engine, so a document added anywhere changes what it
    // reads. A `paths:` filter over today's fixture trees would cover the
    // answer it already knows and miss the case it exists for.
    "datamodel_read_accessor",
    "diagnostic_corpus_schema",
    // Compiles every `cond` / `expr` / `<script>` in every committed
    // document through the ECMAScript frontend, so a document added
    // anywhere changes what it reads — and it is the gate that says a
    // rejection cannot fire on a document that used to build, which is
    // exactly the claim a stale `paths:` filter would stop checking.
    "ecmascript_semantics",
    // Renders every committed document through every backend that lowers
    // ECMAScript and compares the refusals the acceptance walker reports
    // against the raises the artifacts carry. A document added anywhere
    // changes both sides of that comparison, and the direction that
    // matters — a refusal the walker invents — is only visible on a
    // document that carries the construct.
    "ecmascript_acceptance_parity",
    // Sweeps every authored document for a refused expression, so a
    // document added under `examples/` or `integration_resources/`
    // changes what it reads. It is also the only CI-side copy of
    // `scripts/gates/example-codegen.sh`, which has no workflow.
    "cli_expression_refusal",
    // Parses every committed *.scxml and asserts that each guard a
    // backend emits without a data model carries a value decided at
    // build time, so a document added anywhere changes what it reads —
    // and a document carrying the shape it exists for is exactly what a
    // `paths:` filter over today's trees would stop it seeing.
    "cli_guard_emission",
    // Runs the CLI over every committed *.scxml and performs the repair
    // each rejection proposes, so a document added anywhere changes both
    // what it reads and what it replays.
    "diagnostic_fix_is_applicable",
    // Derives the kinds it checks from every committed *.scxml, so no
    // `paths:` filter enumerates its inputs — a document added anywhere
    // changes what it reads.
    "forge_document_name_is_the_stem",
    "gate_registry_contract",
    "hook_ci_parity",
    // Asks `git ls-files` for the mutation corpus and then asks the harness
    // what each casefile declares, so a casefile added anywhere — or one
    // that starts naming a new target — changes both what it reads and what
    // it expects. A `paths:` filter over `sce-build/tests/mutations/**`
    // would cover the corpus and still miss the half that matters: the
    // declared targets are files all over the tree, and it is a change to
    // one of THOSE that the selection has to keep getting right.
    "mutation_rounds_selection",
    // Asks `git ls-files` which trees cite the spec, so a citation added
    // to any file anywhere changes its verdict. A `paths:` filter over
    // the backend runtimes would cover today's answer and miss the one
    // case the test exists for: a NEW tree drifting outside the ledger's
    // symbol scan.
    "ledger_symbol_axis_reach",
    "roadmap_marker_gate",
    "scope_terminology",
    "sourced_scripts_are_tracked",
    "sourcemap_symbol_markers",
    // Reads every workflow to check that a test-running step can fail its
    // job. A `paths:` filter on `.github/workflows/**` would cover its
    // inputs today and stop covering them the moment a workflow moves.
    "test_result_gating",
    "workflow_trigger_coverage",
];

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
    if joined
        .lines()
        .any(|l| l.contains("cargo test") && l.contains(&flag))
    {
        return true;
    }

    // A workflow may delegate to the gate runner instead of restating
    // the target list, in which case the names live one file away. Not
    // following the delegation would read "no workflow runs this gate"
    // off a workflow that runs exactly it — the reading that made this
    // test red the first time a lane was converted.
    for line in joined.lines() {
        let Some(rest) = line.split("scripts/gate ").nth(1) else {
            continue;
        };
        for slug in rest.split_whitespace() {
            let script = repo_root().join(format!("scripts/gates/{slug}.sh"));
            if let Ok(body) = fs::read_to_string(&script) {
                let joined = body.replace("\\\n", " ");
                if joined
                    .lines()
                    .any(|l| l.contains("cargo test") && l.contains(&flag))
                {
                    return true;
                }
            }
        }
    }
    false
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
    // Half one: the gate registry maps the carrying workflow, which is
    // how the selector derives "run this stage always" — an unfiltered
    // workflow yields the catch-all trigger.
    let selector_path = repo_root().join("tools/git-hooks/gate_registry.py");
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
         gate registry, so the hook skips a gate CI runs on every push: {:?}\n\
         Add a gate in {} mapping the workflow; its missing `paths:` filter \
         makes the stage unconditional, which is the intent.",
        unmapped,
        selector_path.display()
    );

    // Half two: a registered gate that runs nothing is a table entry, not a
    // gate. Selecting a gate and executing the target are separate edits, so
    // check that some gate script names each target too. The body to read is
    // `scripts/gates/`, not the hook: the hook now only decides which gates a
    // change needs, and the commands live one per file there.
    let gates_dir = repo_root().join("scripts/gates");
    let mut gate_bodies = String::new();
    let mut scripts: Vec<PathBuf> = fs::read_dir(&gates_dir)
        .unwrap_or_else(|e| panic!("read {}: {}", gates_dir.display(), e))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "sh"))
        .collect();
    assert!(
        !scripts.is_empty(),
        "no gate scripts under {} — this half would pass by reading nothing",
        gates_dir.display()
    );
    scripts.sort();
    for path in scripts {
        gate_bodies.push('\n');
        gate_bodies.push_str(
            &fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e)),
        );
    }

    let unrun: Vec<&&str> = UNFILTERABLE_GATES
        .iter()
        .filter(|gate| !runs_target_by_name(&gate_bodies, gate))
        .collect();
    assert!(
        unrun.is_empty(),
        "a tree-wide gate is registered but no gate script runs \
         target(s) {:?}, so the gate holds locally only by way of the full \
         workspace sweep — the coupling this arrangement exists to break.\n\
         Name each target in {}/tree-hygiene.sh.",
        unrun,
        gates_dir.display()
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

/// Lower bound on the debug-only build steps this gate must observe.
///
/// Moves down when a lane stops spelling the build out — the no_std lane
/// now calls `scripts/gate nostd-mcu`, which reaches the binary through
/// `sce_codegen_require`. That helper drops the stale release binary
/// itself, so the property this test protects survives the delegation
/// rather than leaking out of view with the step. The floor guards the
/// scan, not the count: it fails when the walk stops finding workflows.
const MIN_CODEGEN_BUILD_STEPS: usize = 11;

/// A step that builds only the debug binary must first delete the
/// release one the cache may have restored.
///
/// The Cargo cache in these workflows restores all of `target`, while
/// every `Build sce-codegen` step builds the debug profile alone. A
/// release binary left by an older cache therefore survives into a run
/// that never produced it, and a debug-only build that has not yet
/// produced its own binary leaves that stale one as the only candidate
/// the locators can hand out — which is how CI once executed a binary
/// predating a filter its own templates use and failed with
/// "unknown filter: unsupported" while the source tree was consistent.
///
/// Nothing else can see that: the source is self-consistent, every
/// local run builds both profiles from the same tree, and the only
/// evidence is a binary older than the checkout that produced it.
///
/// The needle is the locator call rather than the `rm` it performs.
/// Which path holds a release binary is exactly the build-layout detail
/// `codegen_binary_resolution.rs` keeps out of every file but the four
/// locators, so a workflow spelling it out would satisfy this gate by
/// breaking that one.
/// The helper every delegating caller reaches the binary through drops
/// the stale release too.
///
/// Without this the workflow-side check above could be satisfied by
/// deleting the build step rather than by making it safe: a lane that
/// delegates has no `cargo build --bin sce-codegen` line for the scan to
/// find, and the risk moves into `sce_codegen_require` unobserved. That
/// is what happened when the no_std lane converted.
#[test]
fn the_codegen_helper_drops_the_stale_release_before_building() {
    let helper = fs::read_to_string(repo_root().join("scripts/lib/sce_codegen.sh"))
        .expect("scripts/lib/sce_codegen.sh is readable");
    let build = helper
        .find("cargo build --bin sce-codegen")
        .expect("the helper builds the binary when no profile holds one");
    let drop = helper
        .find("sce_codegen_drop_stale_release \"$root\"")
        .expect("the helper drops the stale release binary");
    assert!(
        drop < build,
        "`sce_codegen_require` must drop the stale release binary BEFORE it \
         builds the debug one, or a restored cache decides which binary a \
         delegating caller runs"
    );
}

#[test]
fn debug_only_codegen_builds_drop_the_stale_release_binary() {
    const BUILD: &str = "cargo build --bin sce-codegen";
    const DROP: &str = "sce_codegen_drop_stale_release";

    let mut builds = 0usize;
    let mut offenders: Vec<String> = Vec::new();

    for (name, text) in workflows() {
        let joined = text.replace("\\\n", " ");
        let n_build = joined.matches(BUILD).count();
        if n_build == 0 {
            continue;
        }
        builds += n_build;
        let n_drop = joined.matches(DROP).count();
        if n_drop < n_build {
            offenders.push(format!(
                "{name}: {n_build} debug-only build step(s) but only \
                 {n_drop} drop the stale release binary"
            ));
        }
    }

    assert!(
        offenders.is_empty(),
        "a build step that produces only the debug binary must call \
         `sce_codegen_drop_stale_release` first (source \
         scripts/lib/sce_codegen.sh), or the cache decides which binary \
         CI runs:\n  {}",
        offenders.join("\n  "),
    );
    assert!(
        builds >= MIN_CODEGEN_BUILD_STEPS,
        "found only {builds} sce-codegen build step(s); expected at \
         least {MIN_CODEGEN_BUILD_STEPS}. A scan that matches nothing \
         certifies nothing.",
    );
}
