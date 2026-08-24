// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `check` must be askable for the interpretation `generate` will be
// given, and must reach the same verdict under it.
//
// `cli_orchestrate_check_parity.rs` pins the document-set half of this:
// a flag `check` accepts on that route must exist on `orchestrate`. This
// file pins the single-document half, which is the other direction — a
// flag `generate` reads INTO the model every backend renders must exist
// on `check`, or `check` answers about a document nobody builds.
//
// The defect that produced this file, measured on
// `examples/ai_loop/ai_loop.scxml` (nine `<send type="x-sce-host">`
// sites, built with `--host-processor x-sce-host` by three CMake
// targets):
//
//     check    -l {rust,cpp,c11,kotlin,go,python}        -> ok, all six
//     generate -l {c11,kotlin,go,python} --host-processor -> exit 7
//
// `--host-processor` and `--host-invoker` existed on `generate` and not
// on `check`, so the interpretation the build uses was unspellable to
// the command whose whole contract is "reaches the same verdict
// `generate` would". `check` reported six backends able to lower a
// document four of them refuse. Not a disagreement about a document — a
// verdict that was unreachable, the same shape the sibling gate found
// going the other way.
//
// The gate is asked of the binary, not of the source: clap's own help
// enumerates the flags, so a flag added to `generate` and forgotten here
// fails the classification test rather than passing silently. And the
// `Emission` classification is not taken on trust — every flag claiming
// it is probed, and a flag that moves a verdict while claiming to touch
// only what is written fails.

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
    fn path(&self) -> String {
        self.0.to_str().expect("utf-8 scratch path").to_string()
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

/// Long flags a subcommand declares, read off its own help.
fn long_flags(subcommand: &str) -> Vec<String> {
    let out = run(&[subcommand, "--help"]);
    assert!(
        out.code == 0 && out.stdout.len() > 200,
        "`{subcommand} --help` did not render (exit {})",
        out.code,
    );
    let re = regex::Regex::new(r"(?m)^\s+(?:-\w,\s+)?(--[a-z0-9-]+)").expect("flag pattern");
    let mut flags: Vec<String> = re
        .captures_iter(&out.stdout)
        .map(|c| c[1].to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    flags.sort();
    flags
}

/// What a `generate` flag reaches.
#[derive(PartialEq, Eq, Debug)]
enum Reach {
    /// Read into the model or the compile options every backend renders
    /// from: it can change whether a backend accepts the document, so
    /// `check` must be able to be told it.
    Verdict,
    /// Changes only what is WRITTEN — a path, a name, a formatting pass.
    /// `check` writes nothing by construction, so it needs no
    /// counterpart. Held to account by `an_emission_flag_does_not_move_a_verdict`.
    Emission,
    /// Meaningful only alongside `--deploy`, and on `check` a `--deploy`
    /// run is routed to the document-set path whose reference producer
    /// is `orchestrate` — which carries neither flag. The pair is
    /// consistent there, and the sibling gate is the one that watches
    /// that route.
    DeployRoute,
}

struct FlagFacts {
    flag: &'static str,
    reach: Reach,
    /// Arguments that exercise this flag, for the `Emission` probe.
    ///
    /// Two tokens are substituted by the probe, so a flag that needs a path
    /// can be exercised without a static one that would write into the tree:
    /// `{probe_dir}` becomes the run's scratch directory (which exists), and
    /// `{probe_file}` a path inside it. A flag needing a file to READ names a
    /// real one instead — `run` works from the repo root.
    ///
    /// `None` skips the probe and must say why in `why`. Only `--help` may
    /// do so, and `an_emission_flag_does_not_move_a_verdict` pins that list
    /// to exactly `--help` so a later flag cannot join it silently.
    probe: Option<&'static [&'static str]>,
    /// Backend the probe runs against — a prefix flag only reaches its
    /// own language's emitter.
    probe_lang: &'static str,
    why: &'static str,
}

const fn f(
    flag: &'static str,
    reach: Reach,
    probe: Option<&'static [&'static str]>,
    probe_lang: &'static str,
    why: &'static str,
) -> FlagFacts {
    FlagFacts {
        flag,
        reach,
        probe,
        probe_lang,
        why,
    }
}

/// Every long flag `generate` declares, and what it reaches.
///
/// `--help` is listed too: clap renders it like any other flag and the
/// coverage assertion compares whole sets, so leaving it out would make
/// the set comparison fail for a flag nobody wrote.
const GENERATE_FLAGS: &[FlagFacts] = &[
    f(
        "--as-child",
        Reach::Emission,
        Some(&["--as-child", "--parent-stem", "p"]),
        "rust",
        "names the artifact's role in an emitted file set (a synth-invoke \
         child of `--parent-stem`); the one model bit it sets widens what is \
         emitted and refuses nothing",
    ),
    f(
        "--c-symbol-prefix",
        Reach::Emission,
        Some(&["--c-symbol-prefix", "acme_"]),
        "c11",
        "prefixes emitted C symbols",
    ),
    f(
        "--const-fold-budget",
        Reach::Verdict,
        None,
        "rust",
        "a `<sce:fold>` exceeding the budget is refused — the sibling gate \
         measures exactly that on `algorithm_crc16_table.scxml`",
    ),
    f(
        "--cpp-namespace-prefix",
        Reach::Emission,
        Some(&["--cpp-namespace-prefix", "acme"]),
        "cpp",
        "prefixes the emitted C++ namespace",
    ),
    f(
        "--deploy",
        Reach::Verdict,
        None,
        "rust",
        "the deploy topology decides whether a machine's transports resolve; \
         present on `check`, where it also selects the producer being \
         mirrored",
    ),
    f(
        "--emit-ast",
        Reach::Emission,
        Some(&["--emit-ast", "{probe_file}"]),
        "rust",
        "writes the analyzed model to a FILE — `{probe_file}`, not \
         `{probe_dir}`, and the first draft of this probe got that wrong. \
         Handed a directory the command exits 20 (\"Is a directory\"), which \
         the probe reported as the flag moving a verdict. The binding is \
         called `emit_ast_dir` in `sce_codegen.rs`, so the name is what misled \
         it. `codegen_ast_envelope` measures what lands there; this measures \
         that asking for the emit does not move the verdict",
    ),
    f(
        "--error-format",
        Reach::Verdict,
        None,
        "rust",
        "selects how a diagnostic is rendered, not whether there is one; on \
         `check` regardless, since a consumer parsing one command's JSON \
         parses the other's",
    ),
    f(
        "--format-style",
        Reach::Emission,
        Some(&["--format-style", ".clang-format"]),
        "cpp",
        "points at a style file for the C++ post-pass. The probe names this \
         repository's own `.clang-format` — `run` sets the working directory \
         to the repo root, so a real style file is already on disk and the \
         probe needs no fixture of its own",
    ),
    f(
        "--go-module-prefix",
        Reach::Verdict,
        None,
        "go",
        "a Go `<sce:import>` without it is refused — measured by \
         `cli_orchestrate_check_parity`",
    ),
    f(
        "--help",
        Reach::Emission,
        None,
        "rust",
        "clap's own; prints and exits, reads no document",
    ),
    f(
        "--host-invoker",
        Reach::Verdict,
        None,
        "rust",
        "claims `<invoke type>` sites for the host and is refused by a backend \
         with no invoker registry",
    ),
    f(
        "--host-processor",
        Reach::Verdict,
        None,
        "rust",
        "claims `<send type>` sites for the host and is refused by a backend \
         with no `<send>` dispatch registry — measured below",
    ),
    f(
        "--include-dir",
        Reach::Verdict,
        None,
        "rust",
        "resolves `<xi:include>` / `<sce:use>` fragments by name; one that \
         resolves nowhere is a parse refusal",
    ),
    f(
        "--input-root",
        Reach::Emission,
        Some(&["--input-root", "."]),
        "rust",
        "roots the §synth-6.2.6 source-hash the emitted file carries",
    ),
    f(
        "--kotlin-package-prefix",
        Reach::Emission,
        Some(&["--kotlin-package-prefix", "acme"]),
        "kotlin",
        "prefixes the emitted Kotlin package",
    ),
    f(
        "--language",
        Reach::Verdict,
        None,
        "rust",
        "names the backend whose verdict is being asked for",
    ),
    f(
        "--lint",
        Reach::Verdict,
        None,
        "rust",
        "the design-time lints reject legal SCXML — `cli_lint_parity` measures \
         that the two commands run the same call",
    ),
    f(
        "--no-format",
        Reach::Emission,
        Some(&["--no-format"]),
        "cpp",
        "disables the clang-format post-pass on already-emitted C++",
    ),
    f(
        "--no-std",
        Reach::Verdict,
        None,
        "rust",
        "validates the document against the `no_std` runtime variant, which \
         refuses constructs the std one accepts",
    ),
    f(
        "--output-dir",
        Reach::Emission,
        Some(&[]),
        "rust",
        "where artifacts are written; `check` writes nothing, which is its \
         contract rather than a gap. The probe adds no argument BECAUSE the \
         comparison already varies exactly this flag: the harness spells \
         `--output-dir` for both runs and hands them different directories, \
         so equal exit codes are the Emission claim for this flag and nothing \
         else",
    ),
    f(
        "--parent-stem",
        Reach::Emission,
        Some(&["--as-child", "--parent-stem", "p"]),
        "rust",
        "names the parent an emitted child file belongs to",
    ),
    f(
        "--partition",
        Reach::DeployRoute,
        None,
        "rust",
        "selects one partition of a deploy topology",
    ),
    f(
        "--source-root",
        Reach::Emission,
        Some(&["--source-root", "."]),
        "rust",
        "roots the `// From:` provenance line; on `check` anyway as a global",
    ),
    f(
        "--strict-unresolved",
        Reach::Verdict,
        None,
        "rust",
        "turns an `<sce:unresolved>` placeholder into a refusal",
    ),
    f(
        "--transport-only",
        Reach::DeployRoute,
        None,
        "rust",
        "emits a deploy topology's transport layer without its machine",
    ),
    f(
        "--workspace-root",
        Reach::Verdict,
        None,
        "rust",
        "resolves the template tree every backend renders from, and the \
         `template-hash` the emission is pinned to",
    ),
    f(
        "--write-deps",
        Reach::Emission,
        Some(&["--write-deps", "{probe_file}"]),
        "rust",
        "writes a CMake depfile. `codegen_depfile_content` measures the \
         file's content; this measures that asking for it does not move the \
         verdict",
    ),
];

/// The canonical §scxml-6.2.5 host-served fixture: one `<send
/// type="x-sce-host">` with a `<param>`, one ordinary `<send>` beside it
/// as the false-positive guard.
const HOST_FIXTURE: &str =
    "sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml";
const DECLARED_TYPE: &str = "x-sce-host";

/// Every backend `generate` and `check` both route to.
const BACKENDS: [&str; 6] = ["rust", "cpp", "c11", "kotlin", "go", "python"];

/// Enough flags must reach the comparison for a clean result to mean
/// anything. Measured when this gate was written: `generate` declares 27
/// long flags, 11 of them verdict-bearing.
const MIN_GENERATE_FLAGS: usize = 20;
const MIN_VERDICT_FLAGS: usize = 8;

#[test]
fn every_generate_flag_is_classified() {
    let declared = long_flags("generate");
    assert!(
        declared.len() >= MIN_GENERATE_FLAGS,
        "read only {} long flag(s) off `generate --help` (floor {}); the help \
         scraper is broken, not the CLI — a clean result would prove nothing",
        declared.len(),
        MIN_GENERATE_FLAGS,
    );

    let classified: std::collections::BTreeSet<&str> =
        GENERATE_FLAGS.iter().map(|f| f.flag).collect();
    let declared_set: std::collections::BTreeSet<&str> =
        declared.iter().map(String::as_str).collect();

    let unclassified: Vec<&&str> = declared_set.difference(&classified).collect();
    assert!(
        unclassified.is_empty(),
        "`generate` declares {unclassified:?}, which this file does not \
         classify. Decide what the flag reaches: `Verdict` if it is read into \
         the model or the compile options a backend renders from — then it \
         must exist on `check` — `Emission` if it only changes what is \
         written, `DeployRoute` if it is meaningful only under `--deploy`.",
    );

    let stale: Vec<&&str> = classified.difference(&declared_set).collect();
    assert!(
        stale.is_empty(),
        "this file classifies {stale:?}, which `generate` no longer declares; \
         a removed flag leaves a rule nothing enforces",
    );
}

#[test]
fn check_carries_every_verdict_bearing_generate_flag() {
    let check_flags = long_flags("check");
    let verdict_bearing: Vec<&FlagFacts> = GENERATE_FLAGS
        .iter()
        .filter(|f| f.reach == Reach::Verdict)
        .collect();
    assert!(
        verdict_bearing.len() >= MIN_VERDICT_FLAGS,
        "only {} flag(s) classified verdict-bearing (floor {}); the table has \
         been emptied rather than the CLI made consistent",
        verdict_bearing.len(),
        MIN_VERDICT_FLAGS,
    );

    let missing: Vec<String> = verdict_bearing
        .iter()
        .filter(|f| !check_flags.iter().any(|c| c == f.flag))
        .map(|f| format!("{} ({})", f.flag, f.why))
        .collect();
    assert!(
        missing.is_empty(),
        "`generate` reads these into the model every backend renders from, \
         and `check` cannot be told any of them:\n  {}\n`check`'s contract is \
         that it reaches the verdict `generate` would; a verdict it cannot be \
         asked for is one it cannot reach.\ncheck flags: {check_flags:?}",
        missing.join("\n  "),
    );
}

/// An `Emission` classification is a claim that the flag cannot move a
/// verdict. Every flag making it is asked to prove it.
#[test]
fn an_emission_flag_does_not_move_a_verdict() {
    // The one Emission flag allowed to skip this probe, pinned as a list so a
    // new `None` is a failure rather than a quiet subtraction from coverage.
    // `--help` is clap's own: it prints and exits without reading a document,
    // so there is no verdict to hold still. It gets its own probe instead —
    // `the_help_flag_reads_no_document_and_writes_nothing`.
    let unprobed: Vec<&str> = GENERATE_FLAGS
        .iter()
        .filter(|f| f.reach == Reach::Emission && f.probe.is_none())
        .map(|f| f.flag)
        .collect();
    assert_eq!(
        unprobed,
        ["--help"],
        "⚠ every Emission flag but `--help` must carry a probe. An Emission \
         classification is a CLAIM that the flag cannot move a verdict, and an \
         unprobed claim is the gap this test exists to close."
    );

    let mut probed = 0;
    for facts in GENERATE_FLAGS.iter().filter(|f| f.reach == Reach::Emission) {
        let Some(extra) = facts.probe else { continue };
        let base_out = ScratchDir::new("emit-base");
        let flag_out = ScratchDir::new("emit-flag");
        let base_dir = base_out.path();
        let flag_dir = flag_out.path();
        // Substituted rather than static: a flag that must be handed a path
        // would otherwise have to name one in the tree, and writing into the
        // tree to measure it is what kept these five unprobed.
        let probe_file = format!("{flag_dir}/probe.out");
        let substituted: Vec<String> = extra
            .iter()
            .map(|arg| {
                arg.replace("{probe_dir}", &flag_dir)
                    .replace("{probe_file}", &probe_file)
            })
            .collect();

        // The long form on purpose: spelling `-o` here would leave
        // `--output-dir` — an Emission flag in its own right — exercised by
        // nothing, which is how it came to be classified on trust.
        let baseline = run(&[
            "generate",
            HOST_FIXTURE,
            "-l",
            facts.probe_lang,
            "--output-dir",
            &base_dir,
        ]);
        let mut with: Vec<&str> = vec![
            "generate",
            HOST_FIXTURE,
            "-l",
            facts.probe_lang,
            "--output-dir",
            &flag_dir,
        ];
        with.extend(substituted.iter().map(String::as_str));
        let flagged = run(&with);

        assert_eq!(
            baseline.code, 0,
            "the probe's baseline `generate -l {}` already fails, so the \
             comparison below measures nothing: {}",
            facts.probe_lang, baseline.stderr,
        );
        assert_eq!(
            flagged.code, baseline.code,
            "`{}` is classified Emission ({}) but moved the verdict from {} \
             to {}: {}\nReclassify it Verdict and give `check` a counterpart, \
             or fix the flag.",
            facts.flag, facts.why, baseline.code, flagged.code, flagged.stderr,
        );
        probed += 1;
    }
    // Tied to the table rather than to a constant. A floor of "at least five"
    // was what let five more sit unprobed underneath it: the number was
    // satisfied while the coverage was not. Every Emission flag but the one
    // pinned exception must have been probed by the loop above.
    let emission = GENERATE_FLAGS
        .iter()
        .filter(|f| f.reach == Reach::Emission)
        .count();
    assert_eq!(
        probed,
        emission - 1,
        "{probed} of {emission} emission flag(s) were probed (one exception is \
         allowed, and it is `--help`); the rest are being taken on trust"
    );
    assert!(
        emission > 1,
        "the flag table came back with {emission} emission flag(s); the \
         comparison above would hold vacuously"
    );
}

/// `--help` is the one Emission flag with no verdict to hold still, so its
/// claim is measured differently rather than not at all.
///
/// The claim an `Emission` classification makes is "changes only what is
/// WRITTEN". For `--help` the honest reading is stronger — it writes nothing
/// and reads nothing — and both halves are checkable:
///
///   * it succeeds with NO document argument, which every other `generate`
///     invocation requires. That is what "reads no document" means
///     operationally, and it is why the verdict comparison cannot be used
///     here: there is no document for a verdict to be about.
///   * handed an output directory as well, it leaves it empty.
///
/// The second half is the one that would catch a real regression: a `--help`
/// that fell through to the codegen path would still exit 0 and still print
/// usage, and only the untouched directory says it stopped.
#[test]
fn the_help_flag_reads_no_document_and_writes_nothing() {
    let out_dir = ScratchDir::new("help-probe");
    let dir = out_dir.path();

    let helped = run(&["generate", "--help", "--output-dir", &dir]);
    assert_eq!(
        helped.code, 0,
        "`generate --help` must succeed with no document argument — that is \
         what makes it the one Emission flag no verdict comparison can reach: \
         {}",
        helped.stderr
    );
    assert!(
        helped.stdout.contains("--output-dir"),
        "`generate --help` did not render its own flag list, so this probe is \
         measuring something other than help output:\n{}",
        helped.stdout
    );

    let written: Vec<String> = std::fs::read_dir(&dir)
        .expect("read the scratch dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        written.is_empty(),
        "⚠ `generate --help` wrote {} entr(ies) into the output directory: \
         {written:?}. Help is classified Emission on the grounds that it \
         prints and exits; writing artifacts means it reached the codegen \
         path, and the classification is wrong.",
        written.len()
    );
}

/// The manifest field for a declared type, read off either command's
/// stdout.
fn manifest_host_facts(stdout: &str) -> (bool, usize, Vec<String>) {
    let line = stdout
        .lines()
        .find(|l| l.starts_with('{'))
        .unwrap_or_else(|| panic!("no manifest line in stdout:\n{stdout}"));
    let v: serde_json::Value = serde_json::from_str(line).expect("manifest is JSON");
    let needs = v["needs_host_processor"].as_bool().unwrap_or(false);
    let causes = v["host_processor_causes"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    let types = v["host_processor_types"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|t| t.as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default();
    (needs, causes, types)
}

#[test]
fn check_and_generate_agree_about_a_host_declaration() {
    let mut refused_with_declaration = 0;
    let mut refused_without = 0;

    for backend in BACKENDS {
        for declared in [false, true] {
            let out = ScratchDir::new("host-decl");
            let dir = out.path();
            let mut gen: Vec<&str> = vec![
                "generate",
                HOST_FIXTURE,
                "-l",
                backend,
                "-o",
                &dir,
                "--error-format",
                "json",
            ];
            let mut chk: Vec<&str> = vec![
                "check",
                HOST_FIXTURE,
                "-l",
                backend,
                "--error-format",
                "json",
            ];
            if declared {
                for a in ["--host-processor", DECLARED_TYPE] {
                    gen.push(a);
                    chk.push(a);
                }
            }
            let g = run(&gen);
            let c = run(&chk);

            assert_eq!(
                c.code,
                g.code,
                "`check -l {backend}`{} exits {} where `generate` exits {}. \
                 The two commands read one document differently.\ncheck: \
                 {}\ngenerate: {}",
                if declared { " --host-processor" } else { "" },
                c.code,
                g.code,
                c.stderr,
                g.stderr,
            );
            if g.code != 0 {
                assert_eq!(
                    c.stderr.trim(),
                    g.stderr.trim(),
                    "both refused `-l {backend}` but with different \
                     diagnostics; a consumer switching commands would read a \
                     different reason for one document",
                );
                if declared {
                    refused_with_declaration += 1;
                } else {
                    refused_without += 1;
                }
                continue;
            }

            // Accepted by both: the declaration has to have reached the
            // analyzer identically too, or the manifests disagree about
            // what the host owes this build.
            let (gn, gc, gt) = manifest_host_facts(&g.stdout);
            let (cn, cc, ct) = manifest_host_facts(&c.stdout);
            assert_eq!(
                (gn, gc, &gt),
                (cn, cc, &ct),
                "`-l {backend}`{}: manifests disagree — generate says \
                 needs={gn} causes={gc} types={gt:?}, check says needs={cn} \
                 causes={cc} types={ct:?}",
                if declared { " --host-processor" } else { "" },
            );
            if declared {
                assert_eq!(
                    ct,
                    vec![DECLARED_TYPE.to_string()],
                    "`-l {backend}` was given the declaration and neither \
                     command echoed it",
                );
                assert!(
                    !cn && cc == 0,
                    "`-l {backend}` was given the declaration and the \
                     `<send type>` is still reported as one nobody performs \
                     (needs={cn} causes={cc})",
                );
            } else {
                assert!(
                    cn && cc >= 1,
                    "`-l {backend}` without the declaration reports no \
                     unserved `<send type>`, so the fixture no longer carries \
                     one and the declaration below moves nothing",
                );
            }
        }
    }

    assert_eq!(
        refused_without, 0,
        "a backend refused the fixture with no declaration at all; the \
         document is being refused for something other than the host seam \
         and the comparison above is measuring that instead",
    );
    // Non-vacuity: the declaration has to reach the BACKEND, not only
    // the analyzer. While any backend lacks a host-processor registry
    // that shows as a refusal, and the refusal has to be the same one
    // from both commands — which is the exact divergence this file was
    // written for. When every backend has a registry this drops to zero;
    // by then the manifest agreement above and the per-backend dispatch
    // channels are what hold the seam.
    assert!(
        refused_with_declaration > 0 || BACKENDS.len() == count_backends_with_registry(),
        "no backend refused the declaration, and not every backend has a \
         registry either — the flag stopped reaching the backends",
    );
}

/// How many backends currently lower a host-served `<send>`. Read from
/// the binary rather than written down: the number is the thing that
/// changes as coverage lands, and a constant here would have to be
/// edited in lockstep with it.
fn count_backends_with_registry() -> usize {
    let out = ScratchDir::new("registry-count");
    let dir = out.path();
    BACKENDS
        .iter()
        .filter(|b| {
            run(&[
                "generate",
                HOST_FIXTURE,
                "-l",
                b,
                "-o",
                &dir,
                "--host-processor",
                DECLARED_TYPE,
            ])
            .code
                == 0
        })
        .count()
}
