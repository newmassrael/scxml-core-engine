// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `check`'s document-set route must not be able to reach a verdict its
// producer cannot reach.
//
// `cli_check_cross_doc.rs` pins that the two commands agree on the
// documents it feeds them. This file pins the property one level up: the
// *interpretations* they can be asked for. `check`'s own help says the
// reference producer is the one the invocation shape names — a lone
// document is checked against `generate`, a document set against
// `orchestrate` — and `CheckArgs` documents the flags "that change how
// the document is *read*", so that "a document that checks clean is one
// `generate` would accept under the same interpretation".
//
// Two flags broke that. `--go-module-prefix` and `--const-fold-budget`
// existed on `generate` and on `check` and not on `orchestrate`, which
// passed `ForgeCompileOptions::default()` unconditionally. The
// observable consequence, on a Go document set carrying `<sce:import>`:
//
//     check --scxml A --forge B -l go --go-module-prefix M   -> exit 0
//     orchestrate --scxml A --forge B -l go -o D             -> exit 7
//         "<sce:import> with language=go requires
//          ForgeCompileOptions.go_module_prefix"
//
// `check` said the system was valid under an interpretation the producer
// had no way to be given. Not a disagreement about a document — a
// verdict that was unreachable.
//
// The gate below is the general form, and it is asked of the binary
// rather than of the source: enumerate `check`'s flags, ask clap which
// of them survive on the document-set route, and require `orchestrate`
// to carry each survivor. A flag that is single-document-only excludes
// itself by conflicting with the route, which is how `--no-std` and
// `--strict-unresolved` stay out without anyone listing them here.
//
// One-directional on purpose. `orchestrate` may carry knobs `check` does
// not — `--emit-ast-dir` and `--output-dir` are both about *writing*,
// and `check` writes nothing by construction. The implication that has
// to hold is the other one: a knob that changes what `check` concludes
// must exist on the command whose conclusion it claims to predict.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("sce-build has a parent")
        .to_path_buf()
}

static SCRATCH_ID: AtomicU64 = AtomicU64::new(0);

/// Scoped scratch directory under `target/`; removed on drop.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(label: &str) -> Self {
        let id = SCRATCH_ID.fetch_add(1, Ordering::SeqCst);
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{label}-{}-{id}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        Self(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run(args: &[&str]) -> Run {
    let out = Command::new(sce_codegen_bin())
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("sce-codegen runs");
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn help(subcommand: &str) -> String {
    let out = run(&[subcommand, "--help"]);
    assert!(
        out.code == 0 && out.stdout.len() > 200,
        "`{subcommand} --help` did not render (exit {})",
        out.code,
    );
    out.stdout
}

/// Long flags a subcommand declares, read off its own help.
fn long_flags(subcommand: &str) -> Vec<String> {
    let text = help(subcommand);
    let re = regex::Regex::new(r"(?m)^\s+(?:-\w,\s+)?(--[a-z0-9-]+)").expect("flag pattern");
    let mut flags: Vec<String> = re
        .captures_iter(&text)
        .map(|c| c[1].to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    flags.sort();
    flags
}

/// A forge document with a `<sce:fold>` whose iteration count is large
/// enough that a budget of 1 refuses it — the control that keeps the
/// budget assertions from passing over a document the budget never
/// reaches.
const FOLD_DOC: &str = "tests/forge/resources/algorithm_crc16_table.scxml";
const FOLD_DOC_SIBLING: &str = "tests/forge/resources/algorithm_const_fold_smoke.scxml";

/// A Go-lowerable statechart carrying `<sce:import>`, with the two
/// documents it imports.
const GO_SET_SCXML: &str =
    "sce-build/tests/fixtures/event_schema/positive_statechart_enum_strict_opt_out.scxml";
const GO_SET_FORGE: [&str; 2] = [
    "sce-build/tests/fixtures/event_schema/schema_uds_response_open.scxml",
    "sce-build/tests/fixtures/event_schema/uds_nrc_open.scxml",
];

/// Enough flags must reach the comparison for a clean result to mean
/// anything. Measured when this gate was written: `check` declares 10
/// long flags, 8 of which survive on the document-set route.
const MIN_CHECK_FLAGS: usize = 8;
const MIN_DOCUMENT_SET_FLAGS: usize = 6;

#[test]
fn orchestrate_carries_every_flag_check_takes_on_the_document_set_route() {
    let check_flags = long_flags("check");
    assert!(
        check_flags.len() >= MIN_CHECK_FLAGS,
        "read only {} long flag(s) off `check --help` (floor {}); the help \
         scraper is broken, not the CLI — a clean result would prove nothing",
        check_flags.len(),
        MIN_CHECK_FLAGS,
    );

    // Which flags survive the document-set route is asked of clap, not
    // read out of the source: a single-document-only flag declares
    // `conflicts_with_all` against the three ids that open the route, so
    // offering it alongside `--forge` produces clap's conflict error.
    // Nothing here has to know which flags those are.
    //
    // A flag that takes a value has to be given one. clap reports a
    // missing value before it validates conflicts, so probing bare
    // reports "a value is required" for every value-taking flag and
    // silently classifies all of them as route-compatible. The first
    // draft did exactly that and called `--include-dir`
    // document-set-capable while its own `conflicts_with_all` says
    // otherwise. The candidates below are tried until one is neither
    // missing nor ill-typed; a flag no candidate satisfies fails the
    // test rather than being guessed at.
    const VALUES: [&str; 3] = ["1", "rust", "human"];
    let mut document_set: Vec<String> = Vec::new();
    let mut single_document_only: Vec<String> = Vec::new();
    let mut unclassified: Vec<String> = Vec::new();
    for flag in &check_flags {
        let mut verdict = None;
        for extra in std::iter::once(None).chain(VALUES.iter().map(Some)) {
            let mut args = vec!["check", "--forge", FOLD_DOC, "--language", "rust", flag];
            if let Some(v) = extra {
                args.push(v);
            }
            let out = run(&args);
            if out.stderr.contains("a value is required") || out.stderr.contains("invalid value") {
                continue;
            }
            verdict = Some(out.stderr.contains("cannot be used with"));
            break;
        }
        match verdict {
            Some(true) => single_document_only.push(flag.clone()),
            Some(false) => document_set.push(flag.clone()),
            None => unclassified.push(flag.clone()),
        }
    }
    assert!(
        unclassified.is_empty(),
        "no candidate value satisfied {unclassified:?}, so the probe never \
         reached clap's conflict check for them — extend VALUES rather than \
         letting them fall to either side",
    );

    assert!(
        !single_document_only.is_empty(),
        "no flag conflicted with the document-set route, so the probe never \
         reached the condition it discriminates on — every flag would read as \
         document-set-capable and the assertion below would be vacuous",
    );
    assert!(
        document_set.len() >= MIN_DOCUMENT_SET_FLAGS,
        "only {} flag(s) survived the document-set route (floor {})",
        document_set.len(),
        MIN_DOCUMENT_SET_FLAGS,
    );

    let orchestrate_flags = long_flags("orchestrate");
    let missing: Vec<&String> = document_set
        .iter()
        .filter(|f| !orchestrate_flags.contains(f))
        .collect();
    assert!(
        missing.is_empty(),
        "`check` accepts {missing:?} on its document-set route, and \
         `orchestrate` — the producer that route names as the reference — \
         does not. A verdict `check` can be asked for is one the producer \
         cannot be asked for, so the two cannot agree by construction.\n\
         check document-set flags: {document_set:?}\n\
         single-document-only (excluded by their own conflict): \
         {single_document_only:?}\n\
         orchestrate flags: {orchestrate_flags:?}",
    );
}

#[test]
fn the_go_module_prefix_reaches_the_producer_that_check_predicts() {
    let out = ScratchDir::new("orch-go-prefix");
    let mut base: Vec<&str> = vec!["--scxml", GO_SET_SCXML];
    for f in GO_SET_FORGE {
        base.push("--forge");
        base.push(f);
    }

    // Control: without the prefix the producer refuses, so the success
    // below is the flag's doing and not the document's.
    let mut without = vec!["orchestrate"];
    without.extend_from_slice(&base);
    without.extend_from_slice(&["-l", "go", "-o", out.path().to_str().expect("utf-8")]);
    let refused = run(&without);
    assert_ne!(
        refused.code, 0,
        "orchestrate accepted a Go `<sce:import>` set with no module prefix; \
         the control no longer reaches the condition"
    );
    assert!(
        refused.stderr.contains("go_module_prefix"),
        "refusal was not the module-prefix one: {}",
        refused.stderr
    );

    let mut with = without.clone();
    with.extend_from_slice(&["--go-module-prefix", "example.com/m"]);
    let built = run(&with);
    assert_eq!(
        built.code, 0,
        "orchestrate refused the set even with --go-module-prefix: {}",
        built.stderr
    );

    // The flag has to reach the emitter, not merely be accepted.
    let mut emitted_imports = false;
    for entry in std::fs::read_dir(out.path()).expect("read output dir") {
        let path = entry.expect("dir entry").path();
        if std::fs::read_to_string(&path)
            .unwrap_or_default()
            .contains("example.com/m/")
        {
            emitted_imports = true;
        }
    }
    assert!(
        emitted_imports,
        "no generated file carries a `example.com/m/` import; the prefix was \
         accepted and dropped"
    );

    // And `check`, on the same set, reaches the same verdict.
    let mut checked = vec!["check"];
    checked.extend_from_slice(&base);
    checked.extend_from_slice(&["-l", "go", "--go-module-prefix", "example.com/m"]);
    assert_eq!(
        run(&checked).code,
        0,
        "check disagreed with the producer it mirrors"
    );
}

#[test]
fn the_const_fold_budget_reaches_the_producer_that_check_predicts() {
    // A budget of 1 must refuse and the default must accept, on both
    // commands, for the same forge-only document set. Asserting both
    // ends is what keeps this from passing on a document the budget
    // never constrains.
    for (budget, want_refusal) in [("1", true), ("1000000", false)] {
        let out = ScratchDir::new("orch-fold");
        let orchestrated = run(&[
            "orchestrate",
            "--forge",
            FOLD_DOC,
            "--forge",
            FOLD_DOC_SIBLING,
            "-l",
            "rust",
            "-o",
            out.path().to_str().expect("utf-8"),
            "--const-fold-budget",
            budget,
        ]);
        let checked = run(&[
            "check",
            "--forge",
            FOLD_DOC,
            "--forge",
            FOLD_DOC_SIBLING,
            "-l",
            "rust",
            "--const-fold-budget",
            budget,
        ]);
        assert_eq!(
            orchestrated.code != 0,
            want_refusal,
            "orchestrate at budget {budget}: exit {} (stderr: {})",
            orchestrated.code,
            orchestrated.stderr
        );
        assert_eq!(
            checked.code != 0,
            want_refusal,
            "check at budget {budget}: exit {} (stderr: {})",
            checked.code,
            checked.stderr
        );
    }
}

#[test]
fn a_forge_only_document_set_is_nameable_to_check() {
    // `orchestrate` builds a set of forge documents with no statechart in
    // it. `check`'s positional was exempted only by `--scxml`, so the
    // only spelling left for such a set put a forge document into the
    // statechart slot, where it was read as a statechart and refused for
    // having no initial state — a refusal about the invocation, reported
    // as a fact about the document.
    let out = ScratchDir::new("orch-forge-only");
    let orchestrated = run(&[
        "orchestrate",
        "--forge",
        FOLD_DOC,
        "--forge",
        FOLD_DOC_SIBLING,
        "-l",
        "rust",
        "-o",
        out.path().to_str().expect("utf-8"),
    ]);
    assert_eq!(
        orchestrated.code, 0,
        "the producer no longer accepts a forge-only set, so this test's \
         premise is gone: {}",
        orchestrated.stderr
    );

    let checked = run(&[
        "check",
        "--forge",
        FOLD_DOC,
        "--forge",
        FOLD_DOC_SIBLING,
        "-l",
        "rust",
    ]);
    assert_eq!(
        checked.code, 0,
        "check could not mirror a forge-only set: {}",
        checked.stderr
    );
    assert!(
        !checked.stderr.contains("no initial state"),
        "check read a forge document as a statechart: {}",
        checked.stderr
    );

    // The deploy-only shape opens the route by the same rule.
    let deploy_only = run(&[
        "check",
        "--deploy",
        "sce-build/tests/fixtures/event_schema/mesh_matched_deploy.yaml",
        "-l",
        "rust",
    ]);
    assert!(
        !deploy_only
            .stderr
            .contains("required arguments were not provided"),
        "a deploy-only set is still unnameable to check: {}",
        deploy_only.stderr
    );
}
