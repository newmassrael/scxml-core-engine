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
    // Reads every `.yml` in `.github/workflows/` and asks whether each lane's
    // supersession setting matches its measured duration, so a workflow added
    // anywhere changes what it reads — and an unclassified one is the case it
    // exists for. A `paths:` filter cannot carry it for a second reason
    // besides: a gate inherits its workflow's filter as its hook triggers, so
    // naming `.github/workflows/**` on the lane that runs the workspace suite
    // classified every workflow path for a `ci_only` gate the hook is never
    // offered, taking away the full run that editing an unfiltered lane's own
    // file is supposed to buy (`unfiltered-workflow-self` in
    // tools/git-hooks/gate_registry.py).
    "ci_supersession_policy",
    "cmake_option_guard_scope",
    "codegen_binary_resolution",
    "committed_sourcemap_drift",
    // Sweeps every committed *.scxml for a typed `<data>` that arrived
    // without a script engine, so a document added anywhere changes what it
    // reads. A `paths:` filter over today's fixture trees would cover the
    // answer it already knows and miss the case it exists for.
    "datamodel_read_accessor",
    "diagnostic_corpus_schema",
    // Asks `git ls-files` which tracked file BINDS the path of a
    // divergence list, because the one failure the list's own suites
    // cannot see is a list nothing opens any more — an unread list scores
    // its engine perfect in both directions. A reader is a compile
    // definition or a `const val`, and the next one will arrive in a file
    // that does not exist today, so a `paths:` filter written over the
    // tree as it stands enumerates the readers the gate already knows and
    // by construction cannot name the one whose arrival or departure it
    // exists to catch. It reads `ARCHITECTURE.md`'s engine matrix and the
    // two JSON files under `tests/ecmascript/` besides, and no workflow's
    // filter names any of the three.
    "ecma262_scoreboard_contract",
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
    // Lowers every expression in every committed document once per scope
    // stage, so a document added anywhere changes what it reads. The
    // sweep is the lesser half of the reason: its census is what says
    // whether a run-time lowering surface needs a scope handle at all,
    // and a zero there is an answer rather than a silence. What keeps
    // the census honest is a document that crosses a stage boundary —
    // exactly the input a `paths:` filter over today's trees cannot
    // name.
    "scope_obligation",
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
    // Asks `git ls-files` which backends own a `tools/codegen/templates/mesh/
    // <dir>/` tree, and that answer IS the gate: a directory appearing there
    // is what lifts the mesh-rpc refusal, so SCE_MESH.md §9.5's roster has to
    // flip with it. A `paths:` filter written over the tree as it stands
    // enumerates `mesh/cpp/` — the answer this gate already knows — and by
    // construction cannot name the directory whose arrival it exists to
    // catch. It then drives `sce-codegen` for every `--lang` to check the
    // roster against the binary, which widens the input set again.
    "mesh_rpc_backend_contract",
    // Asks `git ls-files` for the mutation corpus and then asks the harness
    // what each casefile declares, so a casefile added anywhere — or one
    // that starts naming a new target — changes both what it reads and what
    // it expects. A `paths:` filter over `sce-build/tests/mutations/**`
    // would cover the corpus and still miss the half that matters: the
    // declared targets are files all over the tree, and it is a change to
    // one of THOSE that the selection has to keep getting right.
    "mutation_rounds_selection",
    // Reads the lane's `timeout-minutes:` and then drives
    // `scripts/gates/mutation-rounds.sh` over the whole corpus, which asks
    // `git ls-files` which casefiles exist and `scripts/mutate --declares`
    // what each one holds — so a casefile added anywhere changes both the
    // schedule it reads and the case counts it checks that schedule against.
    // The half a filter could name is the lane's own two files, and naming
    // them is exactly what the suite carrying this test did not do:
    // `rust-workspace-tests.yml` lists its own workflow and not
    // `mutation-rounds.yml`, so an edit to the ceiling never started the
    // test that checks the ceiling.
    "mutation_corpus_fits_its_lane",
    // Judges the `concurrency:` key of `.github/workflows/mutation-rounds.yml`,
    // a property of the trigger machinery rather than of any source tree —
    // the input class a `paths:` list cannot be keyed to at all. Same
    // measurement as its neighbour above, and the same shape as the
    // 2026-08-04 failure this registry was written for: the only workflow
    // running it was `rust-workspace-tests.yml`, whose filter does not name
    // the lane file it reads, so every edit to that group was held to this
    // rule by a workflow the edit could not start.
    "mutation_round_survives_the_next_push",
    // Asks `git ls-files` which trees cite the spec, so a citation added
    // to any file anywhere changes its verdict. A `paths:` filter over
    // the backend runtimes would cover today's answer and miss the one
    // case the test exists for: a NEW tree drifting outside the ledger's
    // symbol scan.
    "ledger_symbol_axis_reach",
    // Reads every tracked CMake file to answer whether anything links a
    // Rust artifact yet, which is the premise the one OPEN row of the D1
    // ledger rests on. A directory can acquire a `CMakeLists.txt`
    // anywhere, and the link this watches for would arrive in a file
    // that does not exist today — so a `paths:` filter written over the
    // tree as it stands enumerates the answer the gate already knows and
    // by construction cannot name the file whose arrival it exists to
    // catch. It reads `docs/SCE_LUA_TRANSLATION_SEAM.md` and two
    // `sce-build/src/forge/` enums besides.
    "lowering_decision_ledger",
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

/// The floor under the committed-artefact population, so a probe that
/// stops finding artefacts fails instead of certifying an empty sweep.
///
/// Measured 2026-08-30: 1303 tracked files carry a §synth-6.2.6 drift
/// header. The floor sits below that rather than at it — trees do get
/// retired, and this number exists to catch a BROKEN probe, not to pin a
/// corpus size some other gate owns.
const MIN_COMMITTED_ARTEFACTS: usize = 1000;

/// Whether a tracked file is a committed §synth-6.2.6 artefact.
///
/// The drift header is the generator's own mark and it is emitted in
/// every backend language, so asking for it is asking the tree rather
/// than restating a directory list that the next backend invalidates.
/// Both halves are required: `SCE-GENERATED` alone appears in prose
/// about the marker, and the `template-hash` line is what makes the file
/// something `scripts/regen_all_committed_trees.sh` rewrites.
fn is_committed_artefact(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(400)]);
    head.contains("SCE-GENERATED") && head.contains("template-hash:")
}

/// Every tracked path, from git rather than from a directory walk, so
/// build output and ignored trees cannot enter the population.
fn tracked_files() -> Vec<String> {
    let out = std::process::Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(repo_root())
        .output()
        .expect("git ls-files runs in the repository");
    assert!(
        out.status.success(),
        "git ls-files failed: {:?}",
        out.status
    );
    String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether `path` matches one GitHub Actions `paths:` filter pattern.
///
/// The two wildcards differ in exactly the way that decides this gate's
/// answers: `**` crosses `/`, a single `*` does not. A matcher that
/// blurred them would read `com/sce/generated/**` as covering
/// `com/sce/integration/...` and report the very hole it exists to find
/// as covered.
fn filter_matches(pattern: &str, path: &str) -> bool {
    fn go(p: &[u8], s: &[u8]) -> bool {
        if p.is_empty() {
            return s.is_empty();
        }
        if p[0] == b'*' {
            if p.len() > 1 && p[1] == b'*' {
                // `**` crosses separators.
                let rest = &p[2..];
                for i in 0..=s.len() {
                    if go(rest, &s[i..]) {
                        return true;
                    }
                }
                return false;
            }
            // A single `*` stops at the next separator.
            let rest = &p[1..];
            for i in 0..=s.len() {
                if go(rest, &s[i..]) {
                    return true;
                }
                if s.get(i) == Some(&b'/') {
                    break;
                }
            }
            return false;
        }
        if p[0] == b'?' {
            return !s.is_empty() && s[0] != b'/' && go(&p[1..], &s[1..]);
        }
        !s.is_empty() && p[0] == s[0] && go(&p[1..], &s[1..])
    }
    go(pattern.as_bytes(), path.as_bytes())
}

/// The `paths:` / `paths-ignore:` patterns declared anywhere in an `on:`
/// block, as written.
fn declared_filter_patterns(on: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in on.lines() {
        let t = line.trim();
        if t == "paths:" || t == "paths-ignore:" {
            inside = true;
            continue;
        }
        if inside {
            if let Some(rest) = t.strip_prefix("- ") {
                out.push(rest.trim().trim_matches(['\'', '"']).to_owned());
                continue;
            }
            if !t.is_empty() && !t.starts_with('#') {
                inside = false;
            }
        }
    }
    out
}

/// The matcher separates the two wildcards, and says so about the exact
/// pair that produced the hole.
///
/// The gate above is only as strong as this: a matcher that let a single
/// `*` cross `/` would read `com/sce/generated/**` as covering
/// `com/sce/integration/...` and report the 538 uncovered artefacts as
/// covered — a green that means the opposite of what it says. That
/// failure is invisible from the gate itself, because with no filter
/// declared the matcher is never called at all, so it is asserted here
/// directly rather than left to a sweep that may not reach it.
#[test]
fn the_filter_matcher_separates_the_two_wildcards() {
    // `**` crosses separators; a single `*` stops at one.
    assert!(filter_matches("a/**", "a/b/c.kt"));
    assert!(filter_matches("a/*", "a/b.kt"));
    assert!(!filter_matches("a/*", "a/b/c.kt"));
    // `a/**` covers what is under `a`, not `a` itself.
    assert!(!filter_matches("a/**", "a"));
    // A wildcard inside a segment stays inside it.
    assert!(filter_matches(
        "scripts/regen_event_schema_native*.sh",
        "scripts/regen_event_schema_native_go.sh"
    ));
    assert!(!filter_matches(
        "scripts/regen_event_schema_native*.sh",
        "scripts/x/regen_event_schema_native_go.sh"
    ));
    // A literal pattern matches only itself.
    assert!(filter_matches(
        "tests/w3c/conformance/fixtures.json",
        "tests/w3c/conformance/fixtures.json"
    ));
    assert!(!filter_matches(
        "tests/w3c/conformance/fixtures.json",
        "tests/w3c/conformance/fixtures.json.bak"
    ));

    // The measured pair. The filter this lane used to carry named the
    // `generated` tree; the artefacts that went stale live in the
    // `integration` one beside it, and no reading of that pattern
    // reaches them.
    const OLD: &str = "backends/kotlin/tests/src/main/kotlin/com/sce/generated/**";
    assert!(filter_matches(
        OLD,
        "backends/kotlin/tests/src/main/kotlin/com/sce/generated/test150/test150Sm.kt"
    ));
    assert!(!filter_matches(
        OLD,
        "backends/kotlin/tests/src/main/kotlin/com/sce/integration/ai_loop/ai_loopSm.kt"
    ));
}

/// The lane that regenerates every committed tree is started by a change
/// to any artefact it regenerates.
///
/// A `paths:` filter on that lane is a claim that the changes able to
/// break it can be enumerated as globs. Measured 2026-08-30, the filter
/// it carried made that claim falsely: 538 of the 1303 committed
/// artefacts sat outside it — every Kotlin `com/sce/integration/**` and
/// `src/test/**` tree, every Rust `src/integration/**` tree and
/// `tests/test_*.rs`, and the Go forge round-trip codecs — because it
/// named the two `generated/**` roots and stopped.
///
/// What that costs is not a slower red but an UNCLEARABLE one. The round
/// that made lowered Lua the Kotlin backend's default artefact left 40
/// Kotlin integration files stale and this lane failing; the commit whose
/// whole content is those 40 regenerated files would not have started the
/// lane, so the red would have stayed on the dashboard while the tree
/// underneath it was already correct.
///
/// The lane therefore declares no filter, and this test is what keeps
/// that true. It does not forbid a filter — it requires that one cover
/// every artefact the lane reproduces, which is the property the removed
/// filter did not have.
#[test]
fn the_regen_lane_is_started_by_every_artefact_it_reproduces() {
    const LANE: &str = "regen-reproduces.yml";

    let root = repo_root();
    let artefacts: Vec<String> = tracked_files()
        .into_iter()
        .filter(|p| is_committed_artefact(&root.join(p)))
        .collect();

    // A sweep that found nothing would pass every assertion below while
    // measuring nothing at all.
    assert!(
        artefacts.len() >= MIN_COMMITTED_ARTEFACTS,
        "found only {} tracked file(s) carrying a §synth-6.2.6 drift \
         header; expected at least {MIN_COMMITTED_ARTEFACTS}. Either the \
         committed trees are gone or this probe stopped recognising them \
         — and a probe that matches nothing certifies nothing.",
        artefacts.len(),
    );

    let text = fs::read_to_string(root.join(".github/workflows").join(LANE))
        .unwrap_or_else(|e| panic!("read {LANE}: {e}"));
    let on = on_block(LANE, &text);
    if !declares_path_filter(&on) {
        return;
    }

    let patterns = declared_filter_patterns(&on);
    assert!(
        !patterns.is_empty(),
        "{LANE} declares `paths:` but this gate read no pattern out of it, \
         so it cannot say what the lane covers"
    );

    let uncovered: Vec<&String> = artefacts
        .iter()
        .filter(|a| !patterns.iter().any(|p| filter_matches(p, a)))
        .collect();

    assert!(
        uncovered.is_empty(),
        "{LANE} regenerates {} committed artefact(s) but its `paths:` \
         filter does not start on {} of them, so a commit that repairs \
         those files cannot clear the red they cause. Either cover them \
         or drop the filter — the lane's input set is the union of the \
         generator, the templates, every regeneration script, every \
         authored document they read and every artefact they write.\n  \
         first uncovered: {}\n  ({} more)",
        artefacts.len(),
        uncovered.len(),
        uncovered[0],
        uncovered.len().saturating_sub(1),
    );
}
