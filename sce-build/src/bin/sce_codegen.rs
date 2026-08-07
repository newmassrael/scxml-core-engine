// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// sce-codegen — Unified SCXML code generator CLI.
//
// Replaces Python scripts: codegen.py, generate_kotlin_w3c.py,
// fix_scxml_name.py, read_test_metadata.py
//
// Subcommands:
//   generate       — Single SCXML → code (replaces codegen.py)
//   generate-w3c   — Batch W3C test generation (replaces generate_kotlin_w3c.py)
//   fix-scxml-name — Fix SCXML name attribute (replaces fix_scxml_name.py)
//   read-metadata  — Extract metadata description (replaces read_test_metadata.py)

use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use sce_build::analyzer;
use sce_build::cli_error::CliError;
use sce_build::filters;
use sce_build::forge::diagnostic::{Diagnostic, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located};

use sce_build::generator::with_trailing_newline;

/// Write `contents` to `path` or emit a structured `WriteOutput`
/// diagnostic and terminate. Centralising the write+exit pattern
/// keeps every file-writing call-site one line and guarantees
/// `--error-format=json` is honoured uniformly.
fn write_or_exit<P: AsRef<std::path::Path>, C: AsRef<[u8]>>(
    fmt: ErrorFormat,
    path: P,
    contents: C,
) {
    let path = path.as_ref();
    if let Err(e) = fs::write(path, contents) {
        fmt.emit_and_exit(
            &CliError::WriteOutput {
                path: path.display().to_string(),
                source: e,
            },
            "",
        );
    }
}

/// Write a `Diagnostic` as a single NDJSON record to stderr.
///
/// The wire contract is *one JSON object per line*. If the diagnostic
/// itself fails to serialize — only possible under an OOM-class
/// failure given our schema has no floats or non-UTF-8 bytes — fall
/// back to `Diagnostic::meta_failure`, which still flows through
/// serde. Hand-built JSON literals are forbidden: they bypass the
/// schema and drift silently when the struct changes.
fn emit_ndjson(diag: &Diagnostic) {
    match serde_json::to_string(diag) {
        Ok(line) => eprintln!("{line}"),
        Err(e) => {
            let meta = Diagnostic::meta_failure(format!("diagnostic serialization failed: {e}"));
            // Second serde pass on a schema-identical value. If this
            // also fails the process is terminally wedged; emit the
            // shortest legal NDJSON record so downstream parsers at
            // least advance past the line.
            let line = serde_json::to_string(&meta)
                .unwrap_or_else(|_| "{\"v\":1,\"id\":\"fnv1a:0\",\"code\":\"io/filesystem\",\"stage\":\"io\",\"message\":\"double serialization failure\"}".to_string());
            eprintln!("{line}");
        }
    }
}

/// Emit one NDJSON record per W3C batch forge failure, JSON mode only.
///
/// Batch mode does not terminate on the first failure — the summary at
/// the end of `generate_w3c_unified` is the contract — so failure sites
/// must emit non-terminally. Human mode keeps its existing final
/// summary line; only the machine-readable channel gains per-failure
/// records, matching the forge / mesh `--error-format=json` behaviour.
///
/// Takes `Located<ForgeError>` so the full diagnostic shape (`code` /
/// `stage` / `fix` / `location`) survives to NDJSON. Both call sites —
/// parser failures (location populated by the parser, with roxmltree
/// row/col when available) and codegen failures (wrapped at the call
/// site with scxml path as the file label) — share this one helper.
/// Pass-state detection is a batch post-condition, not a compiler
/// error, so it emits `CliError::ScxmlGenerate` inline at its single
/// site rather than reusing this helper.
fn emit_batch_failure_ndjson(err: &sce_build::forge::error::Located<ForgeError>) {
    if !matches!(current_error_format(), ErrorFormat::Json) {
        return;
    }
    for diag in err.to_diagnostics() {
        emit_ndjson(&diag);
    }
}
use sce_build::forge::drift;
use sce_build::generator::{GeneratedOutput, Language};
use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;

/// Spec §synth-6.2.6 drift-header context bundled at `cmd_*` entry and
/// threaded to every file-emitting helper. Two parts —
/// `source_hash` (sha256 over `**/*.scxml` under `input_root` +
/// optional `deploy.yaml`) and `template_hash` (sha256 over
/// `tools/codegen/templates/` + `Cargo.lock` of the SCE workspace) —
/// plus a `generated-at` timestamp that honours `SOURCE_DATE_EPOCH`
/// for deterministic regen.
///
/// Pre-computed once per CLI invocation so every write site through
/// `write_drift_aware` / `write_if_changed_drift_aware` shares the
/// same numbers; `sce-codegen verify` then recomputes from the same
/// inputs and matches each generated file's embedded header.
#[derive(Debug, Clone)]
struct DriftContext {
    hashes: drift::DriftHashes,
    generated_at: u64,
    /// The synth-6.2.6 source set that produced `hashes.source_hash`,
    /// carried so `write_depfile` can declare it.
    ///
    /// Kept on the context rather than re-collected at the depfile call
    /// site because the two would then be separate walks of the same
    /// tree, free to disagree about what the header describes. A depfile
    /// that names a different set than the hash covers is worse than one
    /// that names none: it reports the artefact as watched.
    sources: Vec<PathBuf>,
}

/// Routes a §synth-6.2.6 hash-walk failure onto the diagnostic that names
/// its repair.
///
/// The two failures repair differently and so must not share a code. An I/O
/// error points at the filesystem — permissions, a vanished path. The
/// descent ceiling points at the *layout* of the input: a directory link
/// naming a sibling multiplies the paths beneath it, and the fix is where
/// `--input-root` points, not what mode the files carry. Both used to
/// collapse onto `cli/read-input`, which sent a repair agent hunting
/// permissions for a tree whose only problem was aliasing.
fn drift_hash_failure(walked: &Path, axis: &str, err: drift::DriftHashError) -> CliError {
    match err {
        drift::DriftHashError::WalkLimitExceeded { root, limit } => {
            CliError::SourceHashWalkUnbounded {
                root: root.display().to_string(),
                limit,
            }
        }
        other => CliError::ReadInput {
            path: format!("{}: {axis} compute failed: {other}", walked.display()),
            source: std::io::Error::other("drift compute"),
        },
    }
}

impl DriftContext {
    /// Best-effort compute for the `template-hash` axis: failures along
    /// the workspace-probe path downgrade it to a zero hash and log a
    /// stderr note instead of aborting codegen. The spec invariant
    /// is "every emitted file carries a header" — a zero-hash header
    /// still satisfies that, and `sce-codegen verify` reports the
    /// mismatch when invoked against the real workspace.
    ///
    /// The `source-hash` axis does **not** get that latitude. Its fold is
    /// total over the collected set, so a walk that resolved to nothing
    /// still yields a well-formed sha256 (the empty-input digest) that
    /// reads on the wire exactly like a successful hash — a header a
    /// consumer cannot audit is worse than a refusal.
    ///
    /// `must_cover` names the document the root was **inferred** from, and
    /// is what raises the bar from "the set is non-empty" to "the set
    /// contains this document". Callers pass `None` when the root came
    /// from `--input-root` or when the entry point is a batch with no
    /// single named input: a root the caller declared is an assertion
    /// about where the sources live, not an inference to second-guess,
    /// and the fixture regen scripts legitimately generate from a staged
    /// derivative while hashing against the tracked location it came
    /// from. An empty set is refused either way — no declaration makes
    /// the empty-input digest a truthful description of an input.
    fn compute(input_root: &Path, deploy: Option<&Path>, must_cover: Option<&Path>) -> Self {
        let sources = drift::SourceSet::collect(input_root, deploy)
            .unwrap_or_else(|e| cli_exit(drift_hash_failure(input_root, "source-hash", e)));
        let undescribed = match must_cover {
            Some(input) => !sources.covers(input),
            None => sources.is_empty(),
        };
        if undescribed {
            cli_exit(CliError::SourceHashInputUncovered {
                input: must_cover.unwrap_or(input_root).display().to_string(),
                root: sources.root().display().to_string(),
                hashed: sources.len(),
            });
        }
        let source_hash = sources.digest();
        let source_paths = sources.contributing_paths();
        let explicit = current_workspace_root_override();
        let template_hash = match locate_workspace_root(explicit.as_deref()) {
            Some(ws) => {
                let tpl = ws.join("tools").join("codegen").join("templates");
                let lock = ws.join("Cargo.lock");
                drift::compute_template_hash(&tpl, &lock).unwrap_or_else(|e| {
                    eprintln!(
                        "sce-codegen: template-hash compute failed under {} ({e}); embedding zero hash",
                        ws.display(),
                    );
                    [0u8; 32]
                })
            }
            None => {
                eprintln!(
                    "sce-codegen: workspace root not detected (tried --workspace-root, \
                     $SCE_WORKSPACE_ROOT, CARGO_MANIFEST_DIR/.., cwd-walk); \
                     template-hash embedded as zero — pass --workspace-root <PATH> \
                     to fix",
                );
                [0u8; 32]
            }
        };
        Self {
            hashes: drift::DriftHashes {
                source_hash,
                template_hash,
            },
            generated_at: drift::now_utc_seconds(),
            sources: source_paths,
        }
    }
}

/// File extension predicate matching `cmd_verify`'s
/// `collect_generated_files` set. Header injection is skipped for
/// every other extension so `.scxml` stubs, `.txt` children lists,
/// CMake `.d` depfiles, and `.inl` C++ partials stay byte-stable.
fn is_drift_eligible_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("rs") | Some("cpp") | Some("h") | Some("kt") | Some("go") | Some("py") | Some("c"),
    )
}

/// Returns `content` with the §synth-6.2.6 header prepended (or refreshed
/// if already present) when `path`'s extension is in the drift-
/// eligible set; otherwise echoes the input.
fn apply_drift_header(content: &str, path: &Path, ctx: &DriftContext) -> String {
    if is_drift_eligible_path(path) {
        let prefix = drift::comment_prefix_for_path(path);
        drift::prepend_or_replace_header(content, &ctx.hashes, ctx.generated_at, prefix)
    } else {
        content.to_string()
    }
}

/// Drift-aware analogue of [`write_or_exit`]. Prepends the §synth-6.2.6
/// header for source-extension files (`.rs / .cpp / .h / .kt / .go /
/// .py / .c`) before writing; non-source files are written verbatim.
/// `sce-codegen verify` recomputes both hashes and rejects on
/// mismatch, fulfilling the spec invariant that every emitted file
/// carries a drift-detectable header.
fn write_drift_aware<P: AsRef<Path>>(fmt: ErrorFormat, path: P, content: &str, ctx: &DriftContext) {
    let path_ref = path.as_ref();
    let headered = apply_drift_header(content, path_ref, ctx);
    let final_content = with_trailing_newline(&headered);
    if let Err(e) = fs::write(path_ref, final_content.as_ref()) {
        fmt.emit_and_exit(
            &CliError::WriteOutput {
                path: path_ref.display().to_string(),
                source: e,
            },
            "",
        );
    }
}

/// Drift-aware analogue of [`write_if_changed`]. Compares against
/// the headered bytes so a regen with identical hashes is a no-op
/// (preserves the mtime contract `write_if_changed` exists for).
fn write_if_changed_drift_aware(path: &Path, content: &str, ctx: &DriftContext) -> bool {
    let final_content = apply_drift_header(content, path, ctx);
    write_if_changed(path, &final_content)
}

/// SCE Protocol-Synthesis RFC §synth-5-O — emit the per-machine sourcemap
/// JSON alongside the generated SM source. The output is
/// byte-identical across the 6 backends because:
///
///   - the symbol table is built from the SCXML model alone (no
///     backend-specific data),
///   - hash values come from the same `DriftContext` the §synth-6.2.6
///     header consumes (delegation guarantee, not duplication), and
///   - JSON key ordering rides BTreeMap so iteration is deterministic.
///
/// `sce_sourcemap.json` deliberately does NOT get the §synth-6.2.6 header
/// because (a) JSON does not have a `//` comment syntax, and (b) the
/// file's `source_hash` field IS the drift-detectable provenance.
/// `sce-codegen verify` skips JSON in `is_drift_eligible_path`, so
/// the file stays a plain JSON document.
fn emit_sourcemap_for_machine(model: &SCXMLModel, target_dir: &Path, drift_ctx: &DriftContext) {
    use sce_build::forge::sourcemap;
    use sce_build::forge::symbol_mangling;

    let symbols = match symbol_mangling::build_symbol_table(model, &[]) {
        Ok(table) => table,
        Err(_collision) => {
            // Collision detection fires `traceability/state-id-
            // collision` at the validate phase; reaching codegen
            // means the model passed the walker, so this branch is
            // unreachable in production. Defensive skip rather than
            // panic so a future surprise stays observable as an
            // empty sourcemap rather than a binary crash.
            return;
        }
    };
    let map = sourcemap::build(
        &symbols,
        drift_ctx.hashes.source_hex(),
        drift_ctx.hashes.template_hex(),
    );
    let json = match sourcemap::to_json(&map) {
        Ok(s) => s,
        Err(_e) => return,
    };
    let path = target_dir.join("sce_sourcemap.json");
    // Plain JSON write — bypass `write_if_changed_drift_aware` so the
    // file does not receive the `// SCE-GENERATED` comment header
    // (JSON has no line-comment syntax and the `source_hash` field
    // already provides drift detection).
    write_if_changed(&path, &json);
}

/// How diagnostics are rendered to stderr.
///
/// `Human` is the default and preserves existing CLI output verbatim.
/// `Json` is the machine-readable contract consumed by upstream agents
/// (LangGraph triage, IDE LSP bridges, CI bots). In JSON mode each
/// diagnostic is a single NDJSON line on stderr; stdout continues to
/// carry artifact paths and progress text so build systems that parse
/// stdout are unaffected.
#[derive(Clone, Copy, Debug, ValueEnum)]
enum ErrorFormat {
    Human,
    Json,
}

/// Process-wide error format, installed once in `main` and read by
/// every termination path. Using a `OnceLock` (instead of threading
/// `error_format` through every helper signature) keeps call-sites
/// one line and guarantees that no subcommand can forget to apply
/// the flag. Read-only after install — defensible for a one-shot
/// CLI binary.
static ERROR_FORMAT: OnceLock<ErrorFormat> = OnceLock::new();

/// Resolve the active error format. If `main` neglected to install
/// one (e.g. a helper ran before `Cli::parse`), fall back to human
/// so failures are still observable.
fn current_error_format() -> ErrorFormat {
    ERROR_FORMAT.get().copied().unwrap_or(ErrorFormat::Human)
}

/// Globally-resolved `--workspace-root` override. Mirrors
/// [`ERROR_FORMAT`]: installed once at the top of `main` so every
/// site that needs the SCE workspace location (DriftContext template
/// hashing, the `verify` subcommand) can consult one source of truth
/// without each call having to thread the flag through its
/// signature. Unset when the user neither passed `--workspace-root`
/// nor `SCE_WORKSPACE_ROOT` — in which case the resolution falls
/// through to `CARGO_MANIFEST_DIR`'s parent and cwd-walk per
/// [`locate_workspace_root`].
static WORKSPACE_ROOT_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

/// Root the `// From:` provenance path is expressed relative to, when the
/// invocation has one. Claimed by `--source-root` in `main` if given,
/// otherwise by [`find_project_root`] for the in-repo batch commands.
/// Single-document `generate` / `orchestrate` runs leave it unset and emit
/// the path as the caller named it — see
/// [`sce_build::header_source_path`] for why neither shape consults the
/// process working directory.
static SOURCE_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Read the installed provenance root, if any.
fn current_source_root() -> Option<PathBuf> {
    SOURCE_ROOT.get().cloned()
}

/// Read the installed override, if any. Callers pair this with
/// [`locate_workspace_root`] to honour the global flag without
/// having to plumb it through their signatures.
fn current_workspace_root_override() -> Option<PathBuf> {
    WORKSPACE_ROOT_OVERRIDE.get().cloned()
}

impl ErrorFormat {
    /// Emit any [`ToDiagnostics`] error and terminate with its exit
    /// code. Generic over the error family so ForgeError, MeshError,
    /// and CLI-level errors all funnel through the same code path —
    /// a subcommand cannot accidentally render JSON on stdout or
    /// swallow an exit code without failing compilation.
    fn emit_and_exit<E: ToDiagnostics + std::fmt::Display>(self, err: &E, human_prefix: &str) -> ! {
        match self {
            ErrorFormat::Human => {
                eprintln!("{human_prefix}{err}");
            }
            ErrorFormat::Json => {
                // One NDJSON line per diagnostic. Most errors produce
                // exactly one; XSD validation can produce many.
                for diag in err.to_diagnostics() {
                    emit_ndjson(&diag);
                }
            }
        }
        std::process::exit(err.exit_code());
    }

    /// Convenience shim for the most common case: a forge-pipeline
    /// error reported with the legacy "Forge codegen error: " banner.
    ///
    /// The library returns `Located<ForgeError>` directly — location
    /// is part of the error contract at the library boundary, not a
    /// CLI-local convention — so this wrapper just adds the banner
    /// and delegates. XSD errors special-case their own per-violation
    /// line data inside `expand_xsd_diagnostics`.
    fn emit_forge_and_exit(self, err: &sce_build::forge::error::Located<ForgeError>) -> ! {
        self.emit_and_exit(err, "Forge codegen error: ")
    }
}

/// Emit a CLI-level error under the currently-installed format and
/// exit. One-liner call-site for every raw-exit replacement.
fn cli_exit(err: CliError) -> ! {
    current_error_format().emit_and_exit(&err, "")
}

// ── Stdout manifest (success-path contract) ─────────────────────

/// Accumulator populated during a single `generate` run and serialised
/// once, at the end, by [`emit_generate_manifest`]. Lives as a plain
/// local on the stack — no globals — so concurrent future callers
/// (batch mode, daemon) get per-invocation isolation for free.
#[derive(Default)]
struct GenerateReport {
    artifacts: Vec<PathBuf>,
    needs_script_engine: Option<bool>,
    script_engine_causes: Vec<sce_build::script_engine_analyzer::ScriptEngineCauseRecord>,
    rejected: Option<RejectedDocument>,
    /// Descriptive deploy declarations, present only on a `--deploy`
    /// run. `None` for every deploy-unaware invocation, which is what
    /// keeps the manifest byte-identical for them.
    deploy_facts: Option<sce_build::DeployFacts>,
}

struct RejectedDocument {
    spec: &'static str,
    name: String,
}

/// Wire contract for `sce-codegen generate` stdout. Matches
/// SCE_ERROR_CONTRACT.md §10.
///
/// * `v` — schema version, pinned at 1; bumped only on breaking shape
///   changes per the same policy that governs the error contract.
/// * `kind` — which subcommand produced the record. Constrains agent
///   dispatch when future subcommands (e.g. `generate-w3c`) emit
///   their own manifest shapes on the same stream.
/// * `artifacts` — every file written during the run. Each entry is
///   an object (not a bare string) so the schema can grow additively
///   (size, hash, kind-of-artifact) without a v-bump.
/// * `needs_script_engine` — whether the compiled machine needs a
///   runtime script engine.
/// * `script_engine_causes` — why. Present exactly when
///   `needs_script_engine` is `true`, naming every construct that
///   forced the engine in. A consumer that gates its build on
///   `needs_script_engine == false` (an MCU target with no engine to
///   embed, a deployment that must stay deterministic) can now report
///   which `cond` / `<assign>` / `<invoke>` cost it the pure-static
///   lowering instead of failing with a bare boolean.
/// * `rejected` — present only when the document was rejected by a
///   W3C-spec rule (e.g. §scxml-5.8) and stub files were written
///   in its place. Absence means clean generation.
///
/// `script_engine_causes` and `rejected` are both non-fatal degradation
/// reports: generation succeeded, but the output is weaker than the
/// author may have intended. The manifest is where SCE says so —
/// diagnostics on stderr carry no severity and are rejections by
/// construction (SCE_ERROR_CONTRACT.md §1), and a clean run is pinned to
/// an empty stderr.
#[derive(Serialize)]
struct GenerateManifest<'a> {
    v: u32,
    kind: &'static str,
    /// Commit of the generator that produced these artifacts, or
    /// `"unknown"` on a build with no git checkout to read. Present so a
    /// build system capturing this manifest attributes its output without
    /// a second invocation and without maintaining a version sidecar by
    /// hand — the bookkeeping that leaves a committed tree traceable to no
    /// single commit once it drifts.
    generator: &'static str,
    artifacts: Vec<ArtifactEntry>,
    needs_script_engine: bool,
    /// Omitted (not `[]`) on a pure-static machine, so a pure-static
    /// manifest stays byte-identical to the shape that predates this
    /// field.
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    script_engine_causes: &'a [sce_build::script_engine_analyzer::ScriptEngineCauseRecord],
    #[serde(skip_serializing_if = "Option::is_none")]
    rejected: Option<RejectedInfo<'a>>,
    /// Descriptive declarations read out of `--deploy`. Omitted whole
    /// when the run had no deploy or the deploy declared none of them,
    /// so every manifest that could be produced before this field stays
    /// byte-identical.
    #[serde(skip_serializing_if = "Option::is_none")]
    deploy: Option<DeployInfo>,
}

#[derive(Serialize)]
struct ArtifactEntry {
    path: String,
}

#[derive(Serialize)]
struct RejectedInfo<'a> {
    spec: &'a str,
    name: &'a str,
}

/// Deploy declarations SCE records without acting on them.
///
/// An object rather than a flat `static_analyzer` key for the same
/// reason `artifacts` holds objects: the spec has further descriptive
/// build-environment axes, and they belong beside this one rather than
/// each claiming a top-level manifest key.
#[derive(Serialize)]
struct DeployInfo {
    /// Which commercial analyzer the deployment declares it relies on
    /// (SCE Protocol-Synthesis RFC, synth-5-E lines 1409-1429). Omitted when
    /// unstated. SCE never verifies the claim — carrying it here is the
    /// whole of SCE's part, and is what lets deploy review read it off
    /// the build rather than off the author's word.
    #[serde(skip_serializing_if = "Option::is_none")]
    static_analyzer: Option<&'static str>,
}

impl DeployInfo {
    /// `None` when nothing was declared, so the manifest omits the key
    /// entirely rather than carrying an empty object.
    fn from_facts(facts: Option<&sce_build::DeployFacts>) -> Option<Self> {
        let analyzer = facts?.static_analyzer?;
        Some(DeployInfo {
            static_analyzer: Some(analyzer.as_str()),
        })
    }
}

/// Serialise `report` and write it as a single JSON line to stdout.
fn emit_generate_manifest(report: &GenerateReport) {
    let manifest = GenerateManifest {
        v: 1,
        kind: "generate",
        generator: env!("SCE_GIT_COMMIT"),
        artifacts: report
            .artifacts
            .iter()
            .map(|p| ArtifactEntry {
                path: p.display().to_string(),
            })
            .collect(),
        needs_script_engine: report.needs_script_engine.unwrap_or(false),
        script_engine_causes: &report.script_engine_causes,
        rejected: report.rejected.as_ref().map(|rd| RejectedInfo {
            spec: rd.spec,
            name: rd.name.as_str(),
        }),
        deploy: DeployInfo::from_facts(report.deploy_facts.as_ref()),
    };
    let line = serde_json::to_string(&manifest)
        .expect("GenerateManifest serialises; fields are all owned");
    println!("{line}");
}

// ── CLI Definition ──────────────────────────────────────────────

/// Generator identity: crate version plus the commit it was built from.
///
/// The crate version is frozen pre-1.0 and identifies nothing on its own,
/// so a consumer pinning this binary needs the commit to attribute a
/// generated artifact to the generator that produced it — otherwise the
/// attribution has to be recorded by hand, which drifts. `unknown` when
/// the build had no git checkout to read (vendored crate, release
/// tarball); see `build.rs` for the resolution and its limits.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("SCE_GIT_COMMIT"), ")");

#[derive(Parser)]
#[command(
    name = "sce-codegen",
    about = "SCE SCXML Code Generator",
    version = VERSION,
)]
struct Cli {
    /// Override the SCE workspace root. The workspace root is the
    /// directory carrying `tools/codegen/templates/` and the
    /// `Cargo.lock` that feed the §synth-6.2.6 `template-hash`. Resolution
    /// priority: this flag → `SCE_WORKSPACE_ROOT` env var →
    /// `CARGO_MANIFEST_DIR/..` (compile-time, used for vendored
    /// builds where cwd lives in the consumer workspace) → walk up
    /// from cwd. Set this when an automated build (vendored or
    /// otherwise) cannot rely on the default resolution and you want
    /// the embedded `template-hash` to be the real one rather than
    /// the zero fallback. Global — applies to every subcommand.
    #[arg(long, global = true, value_name = "PATH")]
    workspace_root: Option<PathBuf>,

    /// Root the `// From:` provenance line in every generated file is
    /// expressed relative to. Without it the path is emitted exactly as
    /// named on the command line; with it, inputs under the root are
    /// re-expressed relative to the root and inputs outside it fall back
    /// to the path as given. Set this when generated output is committed
    /// or compared byte-for-byte and you want provenance that resolves on
    /// a machine that never ran the generator. Never relative to the
    /// process working directory — an artifact that varies with the
    /// invoking directory cannot be reproduced without reproducing that
    /// directory. Global — applies to every subcommand.
    #[arg(long, global = true, value_name = "PATH")]
    source_root: Option<PathBuf>,

    /// Diagnostic output format on stderr. `human` (default) preserves
    /// the existing CLI text. `json` emits one NDJSON record per error
    /// for machine consumption; stdout output is unchanged. The flag
    /// is global — every subcommand routes failure through the same
    /// emitter (`cli_exit` / `ErrorFormat::emit_and_exit`), so agents
    /// see a uniform wire contract regardless of which subcommand ran.
    #[arg(
        long,
        value_enum,
        default_value_t = ErrorFormat::Human,
        global = true,
        long_help = "\
Diagnostic output format on stderr.

  human  (default) Human-readable prose; preserves existing CLI output.
  json             One NDJSON record per error, one object per line.
                   Shape is defined by the schema at
                   schemas/sce-diagnostic.v1.schema.json and documented
                   in SCE_ERROR_CONTRACT.md. Example stderr line:

    {\"v\":1,\"id\":\"fnv1a:1c56b923b2b2b87f\",\"code\":\"validation/missing-attribute\",\"stage\":\"validation\",\"message\":\"sce:field must have an 'id' attribute\",\"fix\":{\"kind\":\"add_attribute\",\"element\":\"sce:field\",\"attr\":\"id\"}}

                   Required fields: v, id, code, stage, message.
                   Optional fields (may be absent): spec, location,
                   expected, actual, fix. Consumers MUST ignore
                   unknown fields — purely additive schema changes
                   are backwards-compatible and do not bump `v`.

Stdout is unaffected — the artifact manifest (see
SCE_ERROR_CONTRACT.md §10) rides there regardless of this flag.
"
    )]
    error_format: ErrorFormat,

    #[command(subcommand)]
    command: Commands,
}

/// CLI arguments for the `generate` subcommand. Extracted into a
/// dedicated `Args` struct (rather than inline enum-variant fields) so
/// the wide `Generate` surface does not dwarf the other `Commands`
/// variants; the enum holds it boxed, keeping every variant pointer-sized
/// and the `Subcommand` derive free of a `large_enum_variant` allow.
#[derive(clap::Args)]
struct GenerateArgs {
    /// Input SCXML file path
    scxml: String,
    /// Target language (rust, cpp, kotlin, go, c11).
    #[arg(short, long, default_value = "cpp")]
    language: String,
    /// Output directory
    #[arg(short, long, default_value = ".")]
    output_dir: String,
    /// Generate as child state machine (C++ invoke support)
    #[arg(long)]
    as_child: bool,
    /// Parent machine stem for Kotlin/Go child emission. When supplied
    /// with `--as-child`, the child's `package com.sce.generated.<child>`
    /// (Kotlin) or `package <child>` (Go) header is rewritten to the
    /// parent's package, matching the layout `generate-w3c` produces
    /// via its internal `process_child` hook. Without this flag a
    /// single-file `--as-child` invocation leaves the child in its
    /// own derived package and the parent's unqualified reference to
    /// the child `StateMachine` class fails to resolve.
    ///
    /// C++ and Rust children already compile cleanly without the flag
    /// (C++ resolves via `-I`, Rust via the parent's `mod.rs`) — the
    /// flag is a no-op for those languages.
    #[arg(long)]
    parent_stem: Option<String>,
    /// Write CMake DEPFILE for incremental builds
    #[arg(long)]
    write_deps: Option<String>,
    /// Go module path hosting the generated forge packages, e.g.
    /// `github.com/acme/proj/generated`. When set, each Go
    /// `<sce:import>` emits `import "{prefix}/{snake}"` instead of
    /// the bare `"{snake}"` form (invalid outside GOPATH). Required
    /// for any Go crossfile fixture; ignored for other languages.
    #[arg(long)]
    go_module_prefix: Option<String>,
    /// Path to a .clang-format file for C++ output formatting.
    /// When omitted, the built-in default style is used.
    #[arg(long)]
    format_style: Option<String>,
    /// Disable clang-format post-processing on C++ output.
    #[arg(long)]
    no_format: bool,
    /// Path to deploy.yaml for SCE Mesh transport codegen.
    /// When provided, generates transport routing code alongside
    /// the state machine code. SM output remains identical.
    #[arg(long)]
    deploy: Option<String>,
    /// Emit only the mesh transport header — skip the state-machine
    /// backend code generation entirely. Requires `--deploy`. Used
    /// by CMake fixtures that want transport regeneration without
    /// the rebuild churn from re-emitting `<name>_sm.{h,rs,kt,go,py}`
    /// on every transport-touching change. The state-machine output
    /// must be produced by a separate `generate` invocation without
    /// `--transport-only` (typically a sibling `add_custom_command`).
    ///
    /// Closes mesh_open_issues.md Issue 2 — Pattern Realization
    /// completed 2026-04-16 (Session C), making the deferred
    /// rationale moot.
    #[arg(long, requires = "deploy")]
    transport_only: bool,
    /// Partition identity for `<parallel>` rule-12 role assignment
    /// (SCE_MESH.md §14 rule 12, §16.5). When supplied together
    /// with `--deploy`, the generated SM code branches per
    /// `<parallel>` on this partition's role (Root / NonRoot /
    /// SinglePartition). Ignored without `--deploy`. Omitting the
    /// flag preserves P0 behaviour — all parallels render via the
    /// legacy single-partition path.
    #[arg(long)]
    partition: Option<String>,
    /// RFC §synth-5-F build-time const-fold iteration budget.
    ///
    /// Caps the total iteration count across every `<sce:fold>`
    /// body in the document — every fold tick and every nested
    /// while/foreach iteration decrements one shared counter so
    /// pathological const-fold bodies cannot turn `sce-build`
    /// into a general-purpose compute platform. Omit to use the
    /// SSoT default (1_000_000); set higher for legitimate large
    /// tables, lower for tighter CI budgets.
    #[arg(long)]
    const_fold_budget: Option<u64>,
    /// SCE Protocol-Synthesis RFC §synth-5-J-2: target the
    /// `sce-rust-runtime` no_std variant.
    ///
    /// Only meaningful with `-l rust`; ignored for other
    /// language targets. When set, validates that the SCXML
    /// document does not use constructs that require a
    /// std-coupled runtime (script engines, BasicHTTP send) —
    /// the no_std variant cannot link Lua / QuickJS / tokio /
    /// reqwest. Author-side violations fire
    /// `codegen/no-std-script-not-supported` /
    /// `codegen/no-std-http-not-supported`.
    ///
    /// Emission: `--no-std` produces allocator-free code —
    /// `#![no_std]`, `core::time::Duration`, `NoOpHal` as the
    /// default `Hal`, and elision of the invoke/script-engine
    /// machinery (`session_id` / `invoke_id` / parent queue).
    /// Every owned collection names a profile-resolving runtime
    /// alias — `SceString` / `SceBytes` for payload fields,
    /// `StateChain` / `SceTransitionBuf` / `SceIndexBuf` /
    /// `SceDedupSet` for the microstep buffers — so the runtime
    /// owns the std-vs-heapless choice and ONE emission compiles
    /// against BOTH runtime profiles: the no_std runtime
    /// (`thumbv7em-none-eabihf`, no global allocator) and the std
    /// runtime (the AP profile). Parallel states and typed
    /// bytes/string payloads are included; the
    /// sce-portable-emit-probe crate gates both directions.
    #[arg(long)]
    no_std: bool,
    /// Override the directory used for the §synth-6.2.6 `source-hash`
    /// computation. Defaults to the SCXML file's parent
    /// directory (the typical `resources/<num>/` test layout
    /// where every input the codegen consumed lives next to the
    /// driver file).
    ///
    /// Authoring workflows that stage the input file into a
    /// temporary directory before generation (e.g.
    /// `scripts/regen_donedata_local_invoke.sh`, which copies a
    /// tracked fixture into `mktemp -d` so synth-invoke children
    /// don't pollute the source tree) must point this back at
    /// the canonical input root, otherwise the embedded
    /// `source-hash` reflects the transient staging directory
    /// and `sce-codegen verify` cannot reproduce it from the
    /// repo.
    #[arg(long)]
    input_root: Option<String>,
    /// Emit the parsed document as JSON to `<path>` before
    /// codegen runs. The envelope shape is `apis/forge-ast.v1.schema.json`;
    /// see `docs/SCE_FORGE_AST.md` for the consumer contract.
    ///
    /// Covers every kind in the v1 envelope: 15 forge kinds plus
    /// `statechart` (the `oneOf` discriminator distinguishes them
    /// via `ast.document.kind`). Statechart documents flow through
    /// the SCXML pipeline and are emitted post-analyzer, before
    /// any deploy-time mutations or codegen prep.
    ///
    /// Codegen still runs after the emit — `--emit-ast` is an
    /// addition to the pipeline, not a replacement. Documents
    /// rejected by §scxml-5.8 (`document_rejected`) skip the
    /// emit and continue to the existing rejection-stub codegen
    /// path; the absence of `<path>` is the consumer signal.
    #[arg(long)]
    emit_ast: Option<String>,
    /// Override the Kotlin `package` header prefix on emitted
    /// `*Sm.kt` files. Defaults to `com.sce.generated` (every W3C
    /// IRP fixture emits there). Non-W3C consumers — integration
    /// fixtures under `integration_resources/<stem>/`, custom test
    /// fixtures — pass a different prefix (e.g. `com.sce.integration`)
    /// so the emitted tree lives under
    /// `src/main/kotlin/com/sce/integration/<stem>/` rather than
    /// `src/main/kotlin/com/sce/generated/<stem>/`, keeping the W3C
    /// and integration directory trees disjoint at the package
    /// level.
    ///
    /// Only meaningful with `-l kotlin`; ignored for other
    /// language targets.
    #[arg(long)]
    kotlin_package_prefix: Option<String>,
    /// Nest the emitted C++ namespace under
    /// `SCE::Generated::<prefix>::<machine>` instead of the default
    /// `SCE::Generated::<machine>`. Lets a separate catalog (suite) reuse
    /// the same machine names as the in-tree catalog without an ODR clash
    /// when both link into one binary. Empty/unset = the historical
    /// un-namespaced shape (byte-identical). Only meaningful with `-l cpp`.
    #[arg(long)]
    cpp_namespace_prefix: Option<String>,
    /// Prefix every emitted C symbol with `<prefix>_` (C has no
    /// namespace, so the suite prefix nests each `<machine>_…` symbol —
    /// struct tag, enum/macro names, and child-machine refs included).
    /// Lets a separate catalog (suite) reuse the same machine names as
    /// the in-tree catalog without an ODR clash when both link into one
    /// binary. The C11 peer of `--cpp-namespace-prefix`. Empty/unset =
    /// the historical un-prefixed shape (byte-identical). Only
    /// meaningful with `-l c11`.
    #[arg(long)]
    c_symbol_prefix: Option<String>,
    /// Reject the build when the
    /// document carries any `<sce:unresolved>` placeholder
    /// (attribute or element form). Default builds let the
    /// marker survive in the model + the `sce-codegen
    /// unresolved` NDJSON report; this flag lifts the marker to
    /// `validation/unresolved-placeholder` so production CI
    /// gates cannot merge IR with open decisions.
    #[arg(long)]
    strict_unresolved: bool,
    /// Additional directories searched (in declaration order) to
    /// resolve `<xi:include href="...">` and
    /// `<sce:use template="...">` fragments by name. Tried after
    /// the including file's own directory and before the cwd
    /// fallback, so an explicit search path wins over the implicit
    /// cwd guess. Repeatable: `-I dir_a -I dir_b`. Decouples a
    /// fragment reference from the includer's directory depth and
    /// the fragment's on-disk location — a case file can write
    /// `<sce:use template="foo.sce-template.xml">` instead of
    /// `<sce:use template="../../_templates/foo.sce-template.xml">`.
    ///
    /// Applies to the SCXML preprocessing pipeline only; forge
    /// `<sce:import>` resolution is unaffected.
    #[arg(short = 'I', long = "include-dir", value_name = "DIR")]
    include_dir: Vec<String>,
}

/// CLI arguments for the `generate-w3c` subcommand. Extracted into an
/// `Args` struct (mirroring `GenerateArgs`) so `cmd_generate_w3c` takes
/// one parameter instead of eight, free of a `too_many_arguments` allow.
#[derive(clap::Args)]
struct GenerateW3cArgs {
    /// Target language (rust, cpp, kotlin, go, c11).
    #[arg(short, long)]
    language: String,
    /// Path to tests/CMakeLists.txt (test registry)
    #[arg(long)]
    registry: Option<String>,
    /// Path to resources directory
    #[arg(long)]
    resources: Option<String>,
    /// Generate single test by ID
    #[arg(short, long)]
    test: Option<String>,
    /// Remove all generated files
    #[arg(long)]
    clean: bool,
    /// List tests without generating
    #[arg(long)]
    list: bool,
    /// Path to a .clang-format file for C++ output formatting.
    /// When omitted, the built-in default style is used.
    #[arg(long)]
    format_style: Option<String>,
    /// Disable clang-format post-processing on C++ output.
    #[arg(long)]
    no_format: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate code from a single SCXML file
    Generate(Box<GenerateArgs>),
    /// Multi-doc generate with cross-doc registry — wires
    /// `validate_on_sample_link_references` into production
    /// (SCE Protocol-Synthesis RFC §synth-5-D).
    /// Use this when the build has multiple SCXML/forge docs that
    /// reference each other across files (`<sce:on-sample link>`,
    /// `<sce:outbox ref>`); single-file `Generate` does not
    /// build the cross-doc registry and silently skips cross-ref
    /// validation. Both lists may be empty (no-op).
    Orchestrate {
        /// Input SCXML file path (repeat for multiple files).
        #[arg(long = "scxml")]
        scxml: Vec<String>,
        /// Input forge file path (repeat for multiple files).
        #[arg(long = "forge")]
        forge: Vec<String>,
        /// Target language (rust, cpp, kotlin, go, c11).
        #[arg(short, long)]
        language: String,
        /// Output directory (one entry per input doc; sidecars travel
        /// with their primary in `GeneratedOutput::files`).
        #[arg(short, long)]
        output_dir: String,
        /// Optional path to deploy.yaml. When provided, the orchestrator
        /// runs SCE Protocol-Synthesis RFC §synth-5-K + §synth-5-M cross-doc validators that
        /// otherwise silent-skip:
        ///   - `validate_links_cross_doc` (§synth-5-K)
        ///   - `validate_links_burst_invariants` (§synth-5-K [lines 2489-2500])
        ///   - `validate_reassembly_cross_doc` (§synth-5-M [lines 2946-2995])
        ///
        /// Omit to keep the multi-doc orchestrator deploy-unaware
        /// (matching every pre-existing call site's silent-skip
        /// semantics).
        #[arg(long)]
        deploy: Option<String>,
        /// Directory to write per-doc AST envelopes into. One
        /// `<doc_stem>.ast.json` is emitted per `--forge` input AND
        /// per `--scxml` input — the v1 envelope's `oneOf` arm covers
        /// statechart documents (`ast.document.kind = "statechart"`)
        /// alongside the 15 forge kinds, so the orchestrate emit path
        /// is uniform across both classifier outputs. Documents
        /// rejected by §scxml-5.8 (`document_rejected`) skip emit
        /// silently — matching the single-doc `generate --emit-ast`
        /// contract.
        ///
        /// Envelope shape: `apis/forge-ast.v1.schema.json`. Consumer
        /// contract: `docs/SCE_FORGE_AST.md`. Useful for batch tools
        /// (sce-db-gen, sce-eventstore-adapter, sce-ui-gen) that
        /// consume IR across an entire multi-doc build without
        /// invoking SCE codegen per file.
        #[arg(long)]
        emit_ast_dir: Option<String>,
    },
    /// Batch generate W3C test state machines and test classes
    GenerateW3c(Box<GenerateW3cArgs>),
    /// Batch generate integration-fixture state machines for the four
    /// sce-codegen-driven backends (Rust / Kotlin / Go / Python) from
    /// canonical fixtures under `integration_resources/<stem>/<stem>.scxml`.
    ///
    /// Parallel to `generate-w3c` but reads from
    /// `integration_resources/` (separate §synth-6.2.6 input-root from W3C
    /// `resources/`) and emits into each backend's integration tree:
    ///   - Rust:    `backends/rust/tests/src/integration/<stem>/`            (committed)
    ///   - Kotlin:  `backends/kotlin/tests/src/main/kotlin/com/sce/integration/<stem>/` (committed)
    ///   - Go:      `backends/go/tests/integration/<stem>/`                  (committed)
    ///   - Python:  `backends/python/tests/integration/<stem>/`              (.gitignored, CI regen)
    ///
    /// Python mirrors the W3C IRP pattern: `generate-w3c -l python`
    /// is already `.gitignored` + CI-regenerated, and the integration
    /// counterpart follows the same model.
    ///
    /// cpp and C11 are intentionally not supported — they emit at
    /// CMake build time through `sce_generate_static_integration_test`
    /// (see `cmake/SCEStaticIntegrationFixture.cmake`) without an
    /// sce-codegen-driven regen pipeline.
    GenerateIntegration {
        /// Target language (rust, kotlin, go, python).
        #[arg(short, long)]
        language: String,
        /// Single fixture stem to regenerate. When omitted, every
        /// `integration_resources/<stem>/` directory is processed.
        #[arg(long)]
        stem: Option<String>,
    },
    /// Fix SCXML name attribute (ensure <scxml name="testXXX">)
    FixScxmlName {
        /// SCXML file path
        scxml: String,
        /// Desired name value
        name: String,
    },
    /// Read metadata.txt description field
    ReadMetadata {
        /// Path to metadata.txt
        metadata_file: String,
    },
    /// Build forge dependency manifest (JSON)
    Manifest {
        /// Directory containing forge SCXML files
        dir: String,
    },
    /// Emit `sce:req` requirement-coverage NDJSON for a single SCXML
    /// file. One JSON record per IR
    /// node carrying a non-empty `sce:req` attribute; empty output
    /// when the document has no `sce:req` annotations.
    Requirements {
        /// SCXML file path
        scxml: String,
    },
    /// Emit `<sce:unresolved>` placeholder NDJSON for a single SCXML
    /// file. One JSON record per
    /// marker (attribute form and element form both detected);
    /// empty output when the document has no unresolved markers.
    Unresolved {
        /// SCXML file path
        scxml: String,
    },
    /// Generate a cross-language numerical conformance test harness from a
    /// fixture catalog (single source of truth for all 5 languages).
    GenerateConformance {
        /// Target language (rust, cpp, kotlin, go, python)
        #[arg(short, long)]
        language: String,
        /// Path to fixture catalog JSON (tests/forge/conformance/fixtures.json)
        #[arg(short, long)]
        manifest: String,
        /// Output directory for the generated harness file
        #[arg(short, long)]
        output_dir: String,
        /// Write a Make-format depfile for the harness.
        ///
        /// Same contract as `generate --write-deps`. Without it the two
        /// CMake steps that render a harness were the only codegen
        /// invocations in the tree with no `DEPFILE`, and declared their
        /// inputs with a `file(GLOB ... CONFIGURE_DEPENDS)` that names
        /// the per-kind fragments the scaffold includes directly and
        /// nothing those fragments pull in.
        #[arg(long)]
        write_deps: Option<String>,
    },
    /// Expand preprocessors (XInclude + `sce:template`) on an SCXML
    /// file and print the post-expansion text to stdout.
    ///
    /// Introduced for the SSOT byte-equivalence parity
    /// harness (`tests/w3c_template_parity/`): the C++ test driver
    /// compares this subcommand's stdout against the pugixml
    /// runtime's `processXInclude` + `processSceTemplate` output.
    /// Both producers canonicalise through the same pugixml
    /// serialiser before diff.
    ///
    /// Calls [`sce_build::parser::expand_preprocessors`] — the same
    /// function [`sce_build::parser::SCXMLParser::parse_file`]
    /// uses — so no third-party caller can drift the subcommand's
    /// semantics away from the codegen pipeline's view of the same
    /// document.
    Expand {
        /// Input SCXML file path
        scxml: String,
        /// Additional directories searched (in declaration order) to
        /// resolve `<xi:include href="...">` and
        /// `<sce:use template="...">` fragments by name — same
        /// semantics as `generate --include-dir`. Lets the parity
        /// harness drive include-dir resolution through both engines
        /// with a matching search path.
        #[arg(short = 'I', long = "include-dir", value_name = "DIR")]
        include_dir: Vec<String>,
    },
    /// Print the conformance fixture name list from a manifest. Build
    /// systems consume this so they don't need a native JSON parser
    /// (CMake, Gradle, plain Bash) to enumerate fixtures.
    ListFixtures {
        /// Path to fixture catalog JSON (tests/forge/conformance/fixtures.json)
        #[arg(short, long)]
        manifest: String,
        /// Output format. `plain` (default) is one fixture name per line,
        /// suitable for `for fixture in $(sce-codegen list-fixtures ...)`.
        /// `cmake` emits a single semicolon-separated CMake list literal.
        /// `space` emits a single space-separated line.
        #[arg(short, long, default_value = "plain")]
        format: String,
        /// Optional language gate. When set to `c11`, applies the same
        /// `c11_supported_kind` filter that `generate-conformance` uses,
        /// so the c11 cmake harness can derive its fixture set from the
        /// single manifest source of truth. Other values (and the unset
        /// default) emit every fixture in the manifest unchanged.
        #[arg(short, long)]
        language: Option<String>,
        /// RFC §synth-5-B B2-test-vector: restrict the listing to fixtures
        /// whose SCXML carries at least one `<sce:test-vector>` element
        /// (algorithm kind only — codec test vectors defer to B5). The
        /// cmake harness uses this to declare the per-fixture sidecar
        /// header `<fixture>_test.h` as an additional OUTPUT of the
        /// generate custom_command without speculating which fixtures
        /// emit a sidecar. Requires `--resource-dir` to locate the
        /// per-fixture SCXML files.
        #[arg(long)]
        has_test_vectors: bool,
        /// Directory containing per-fixture `<name>.scxml` source files.
        /// Required when `--has-test-vectors` is set.
        #[arg(long)]
        resource_dir: Option<String>,
    },

    /// Verify generated-source drift per spec §synth-6.2.6.
    ///
    /// Scans `out_dir` for emitted files (.rs / .cpp / .h / .kt / .go /
    /// .py / .c), reads each file's `// SCE-GENERATED` header,
    /// recomputes `source-hash` + `template-hash` from the current
    /// source + template state, and fails on mismatch with
    /// `forge/source-hash-mismatch`. CI / pre-commit hooks invoke this
    /// to enforce the "manual edits to `out/` are forbidden" invariant.
    Verify {
        /// Generated output directory to verify (recursive).
        out_dir: String,
        /// Input SCXML root used at codegen time. Required so the
        /// `source-hash` recompute walks the same `**/*.scxml` set the
        /// original codegen consumed.
        #[arg(long)]
        input_root: String,
        /// Path to deploy.yaml if it was part of the original codegen
        /// input set. Optional — single-document codegen omits this.
        #[arg(long)]
        deploy: Option<String>,
        /// Override the template tree location used for the
        /// `template-hash` recompute. Defaults to
        /// `<workspace_root>/tools/codegen/templates`.
        #[arg(long)]
        template_root: Option<String>,
        /// Override the `Cargo.lock` location used for the
        /// `template-hash` recompute. Defaults to `<workspace_root>/Cargo.lock`.
        #[arg(long)]
        cargo_lock: Option<String>,
    },

    /// SCE Protocol-Synthesis RFC §synth-5-O — resolve a mangled symbol or
    /// PC offset back to its originating SCXML coordinates.
    ///
    /// `--symbol <NAME>`  Look up a mangled `<machine>__<state_path>__
    ///                     <artifact>` identifier in
    ///                     `<sourcemap>/sce_sourcemap.json`.
    /// `--pc <ADDR>`       Resolve an ELF program-counter address to a
    ///                     function symbol via DWARF + then look that
    ///                     symbol up in the sourcemap. Requires
    ///                     `--elf <path>`.
    /// `--hardfault`       Read newline-separated PC addresses from
    ///                     stdin, resolve each through the same path
    ///                     as `--pc`, emit one NDJSON record per
    ///                     resolved frame.
    ///
    /// Spec lines 3253-3278 fix the tool's resolution contract:
    /// PC → symbol → sourcemap → SCXML file:line + state_path. The
    /// per-symbol attribution data ships in the sourcemap, not the
    /// DWARF; addr2sce composes the two layers.
    #[command(name = "addr2sce")]
    Addr2Sce {
        /// Directory containing `sce_sourcemap.json` (per-machine
        /// output, e.g. `target/.../src/generated/test144/`).
        sourcemap_dir: String,
        /// Mangled symbol to look up directly (mutually exclusive
        /// with `--pc` / `--hardfault`).
        #[arg(long)]
        symbol: Option<String>,
        /// ELF program-counter address (hex with or without `0x`
        /// prefix). Requires `--elf`.
        #[arg(long)]
        pc: Option<String>,
        /// ELF binary path for DWARF lookup (required when `--pc` or
        /// `--hardfault` is used).
        #[arg(long)]
        elf: Option<String>,
        /// Read PC addresses from stdin (one per line) and resolve
        /// each as `--pc` would.
        #[arg(long, default_value_t = false)]
        hardfault: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let error_format = cli.error_format;
    // Install the format once so every termination helper can read
    // it without plumbing through function signatures. Tests that
    // launch the binary inherit this install via the normal CLI
    // parse; in-process helpers never run before this point.
    let _ = ERROR_FORMAT.set(error_format);
    // Same OnceLock pattern for the workspace-root override so
    // DriftContext::compute and cmd_verify can both consult one
    // source of truth without growing parallel `workspace_root: Option<&Path>`
    // params on every helper signature.
    if let Some(p) = cli.workspace_root {
        let _ = WORKSPACE_ROOT_OVERRIDE.set(p);
    }
    // Claimed before any subcommand runs so an explicit flag outranks the
    // repo root `find_project_root` would otherwise install.
    if let Some(p) = cli.source_root {
        let _ = SOURCE_ROOT.set(p);
    }
    match cli.command {
        Commands::Generate(args) => cmd_generate(*args, error_format),
        Commands::Orchestrate {
            scxml,
            forge,
            language,
            output_dir,
            deploy,
            emit_ast_dir,
        } => cmd_orchestrate(
            &scxml,
            &forge,
            &language,
            &output_dir,
            deploy.as_deref(),
            emit_ast_dir.as_deref(),
            error_format,
        ),
        Commands::GenerateW3c(args) => cmd_generate_w3c(*args),
        Commands::GenerateIntegration { language, stem } => {
            cmd_generate_integration(&language, stem.as_deref(), error_format)
        }
        Commands::FixScxmlName { scxml, name } => cmd_fix_scxml_name(&scxml, &name),
        Commands::ReadMetadata { metadata_file } => cmd_read_metadata(&metadata_file),
        Commands::Manifest { dir } => cmd_manifest(&dir),
        Commands::Requirements { scxml } => cmd_requirements(&scxml, error_format),
        Commands::Unresolved { scxml } => cmd_unresolved(&scxml, error_format),
        Commands::GenerateConformance {
            language,
            manifest,
            output_dir,
            write_deps,
        } => cmd_generate_conformance(&language, &manifest, &output_dir, write_deps.as_deref()),
        Commands::ListFixtures {
            manifest,
            format,
            language,
            has_test_vectors,
            resource_dir,
        } => cmd_list_fixtures(
            &manifest,
            &format,
            language.as_deref(),
            has_test_vectors,
            resource_dir.as_deref(),
        ),
        Commands::Expand { scxml, include_dir } => cmd_expand(&scxml, &include_dir),
        Commands::Verify {
            out_dir,
            input_root,
            deploy,
            template_root,
            cargo_lock,
        } => cmd_verify(
            &out_dir,
            &input_root,
            deploy.as_deref(),
            template_root.as_deref(),
            cargo_lock.as_deref(),
            error_format,
        ),
        Commands::Addr2Sce {
            sourcemap_dir,
            symbol,
            pc,
            elf,
            hardfault,
        } => cmd_addr2sce(
            &sourcemap_dir,
            symbol.as_deref(),
            pc.as_deref(),
            elf.as_deref(),
            hardfault,
            error_format,
        ),
    }
}

// ── Subcommand: orchestrate ─────────────────────────────────────
//
// SCE Protocol-Synthesis RFC §synth-5-D entry point —
// the production-side consumer that closes the silent
// hole on `validate_on_sample_link_references`. Authors that hold
// multi-doc builds (cross-file `<sce:on-sample link>` references, or
// the `<sce:outbox ref>` axis) switch to
// this subcommand to gain cross-doc registry construction + cross-ref
// validation that the single-file `Generate` cannot provide.

fn cmd_orchestrate(
    scxml_paths: &[String],
    forge_paths: &[String],
    language: &str,
    output_dir: &str,
    deploy_path: Option<&str>,
    emit_ast_dir: Option<&str>,
    error_format: ErrorFormat,
) {
    // Per-doc AST emit runs *before* the multi-doc compile so the AST
    // surface is independent of any cross-doc validation outcome. A
    // failing cross-doc validator must still produce ASTs for the
    // docs that parsed successfully — consumer tooling treats AST
    // emit as observation, not as compile commitment.
    if let Some(dir) = emit_ast_dir {
        emit_orchestrate_asts(scxml_paths, forge_paths, dir, error_format);
    }
    let lang: Language = language.parse().unwrap_or_else(|_| {
        error_format.emit_and_exit(
            &CliError::UnknownLanguage {
                lang: language.to_string(),
            },
            "",
        )
    });

    let scxml_path_bufs: Vec<std::path::PathBuf> =
        scxml_paths.iter().map(std::path::PathBuf::from).collect();
    let forge_path_bufs: Vec<std::path::PathBuf> =
        forge_paths.iter().map(std::path::PathBuf::from).collect();
    let scxml_refs: Vec<&Path> = scxml_path_bufs.iter().map(|p| p.as_path()).collect();
    let forge_refs: Vec<&Path> = forge_path_bufs.iter().map(|p| p.as_path()).collect();

    // Default options match `Generate`'s sentinel defaults. Future
    // CLI flags can grow the surface (format-style, const_fold_budget)
    // when consumer demand arrives; the minimal shape today keeps the
    // subcommand's wire footprint bounded.
    let options = sce_build::ForgeCompileOptions::default();

    let template_dir = sce_build::find_template_dir_for(lang);

    // C13 orchestrator wiring (`b501b18c`): parse the optional
    // deploy.yaml into a `DeployConfig` so the multi-doc compile path
    // can fire SCE Protocol-Synthesis RFC §synth-5-K + §synth-5-M cross-doc validators.
    // Errors during read/parse route through the same Located<ForgeError>
    // pipeline `compile_scxml_with_imports` uses — `ForgeError::Mesh`
    // wraps `MeshError::Deploy` so the wire JSON shape matches every
    // other deploy-side diagnostic. The diagnostic label points at the
    // user-supplied deploy.yaml path so CLI consumers see the file
    // they passed.
    let deploy_cfg: Option<sce_build::mesh::deploy::DeployConfig> = match deploy_path {
        Some(p) => {
            let content = fs::read_to_string(p).unwrap_or_else(|e| {
                error_format.emit_and_exit(
                    &CliError::ReadInput {
                        path: p.to_string(),
                        source: e,
                    },
                    "",
                )
            });
            match sce_build::mesh::deploy::parse_deploy_str(&content) {
                Ok(cfg) => Some(cfg),
                Err(e) => {
                    let forge_err: ForgeError = sce_build::mesh::error::MeshError::from(e).into();
                    let located = Located::new(forge_err, p, None, None);
                    error_format.emit_forge_and_exit(&located);
                }
            }
        }
        None => None,
    };

    let outputs = match sce_build::compile_scxml_with_imports(
        &scxml_refs,
        &forge_refs,
        &template_dir,
        lang,
        &options,
        deploy_cfg.as_ref(),
    ) {
        Ok(o) => o,
        Err(e) => error_format.emit_forge_and_exit(&e),
    };

    let out_root = Path::new(output_dir);
    if !out_root.exists() {
        if let Err(e) = fs::create_dir_all(out_root) {
            error_format.emit_and_exit(
                &CliError::WriteOutput {
                    path: output_dir.to_string(),
                    source: e,
                },
                "",
            );
        }
    }

    // Spec §synth-6.2.6 drift context — covers every output file written
    // below with a `// SCE-GENERATED` header that `sce-codegen verify`
    // can recompute and gate on. `input_root` defaults to the parent
    // of the first SCXML path so a typical batch (all docs in one
    // directory) hashes its whole input set; a flat fallback to "."
    // keeps multi-dir invocations functional even though their hash
    // is then the cwd recursive walk.
    let drift_input_root: std::path::PathBuf = scxml_path_bufs
        .first()
        .and_then(|p| p.parent().map(|x| x.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let drift_ctx = DriftContext::compute(
        &drift_input_root,
        deploy_path.map(Path::new),
        scxml_path_bufs.first().map(|p| p.as_path()),
    );

    for (basename, generated) in &outputs {
        for (file_name, code) in &generated.files {
            let path = out_root.join(file_name);
            write_drift_aware(error_format, &path, code, &drift_ctx);
        }
        let _ = basename; // basename is the input-doc label; outputs already self-name.
    }
}

/// Multi-doc AST emit helper for [`cmd_orchestrate`]. For every
/// `--forge` input that parses as a forge document, writes
/// `<doc_stem>.ast.json` under `dir`. `--scxml` inputs and any forge
/// document the parser classifies as statechart are silently
/// skipped (no envelope, no error) — statechart AST export is not
/// part of v1.
///
/// Failures during parse propagate through the same NDJSON
/// diagnostic channel `cmd_orchestrate` itself uses, so an
/// `--error-format=json` consumer sees consistent records across
/// the parse/emit/compile chain.
fn emit_orchestrate_asts(
    scxml_paths: &[String],
    forge_paths: &[String],
    dir: &str,
    error_format: ErrorFormat,
) {
    let dir_path = std::path::Path::new(dir);
    if let Err(e) = fs::create_dir_all(dir_path) {
        error_format.emit_and_exit(
            &CliError::WriteOutput {
                path: dir.to_string(),
                source: e,
            },
            "",
        );
    }

    // Statechart AST emit — parallel to the forge loop below. Each
    // `--scxml` input is parsed + analyzed (the SCXML pipeline's
    // post-analyzer step) and serialised as the
    // `statechart` arm of the v1 envelope. Skip-on-rejected:
    // documents the analyzer flags via W3C 5.8 (`document_rejected`)
    // skip emit and fall through silently — the absence of
    // `<stem>.ast.json` is the consumer signal, matching the
    // single-doc `generate --emit-ast` contract.
    for scxml_path_str in scxml_paths {
        let scxml_path = std::path::Path::new(scxml_path_str);
        let stem = scxml_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let mut parser = sce_build::parser::SCXMLParser::new();
        let mut model = match parser.parse_file(scxml_path_str) {
            Ok(m) => m,
            Err(e) => error_format.emit_and_exit(&e, ""),
        };
        sce_build::analyzer::analyze(&mut model, scxml_path_str);
        if model.document_rejected {
            continue;
        }
        let parsed = sce_build::forge::ast_export::statechart_parsed_forge(model);
        let out_path = dir_path.join(format!("{stem}.ast.json"));
        if let Err(e) = sce_build::forge::ast_export::write_envelope_to_path(&out_path, &parsed) {
            error_format.emit_and_exit(
                &CliError::WriteOutput {
                    path: out_path.display().to_string(),
                    source: e,
                },
                "",
            );
        }
    }

    for forge_path_str in forge_paths {
        let forge_path = std::path::Path::new(forge_path_str);
        let content = fs::read_to_string(forge_path).unwrap_or_else(|e| {
            error_format.emit_and_exit(
                &CliError::ReadInput {
                    path: forge_path_str.to_string(),
                    source: e,
                },
                "",
            )
        });
        let stem = forge_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let basename = forge_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(stem);
        let label = sce_build::DocumentLabel {
            identifier: stem,
            diagnostic_label: basename,
        };

        let parsed = match sce_build::forge::parser::parse_forge_with_imports(&content, label) {
            Ok(Some(p)) => p,
            // Statechart: silent skip per v1 contract.
            Ok(None) => continue,
            Err(e) => error_format.emit_forge_and_exit(&e),
        };

        let out_path = dir_path.join(format!("{stem}.ast.json"));
        if let Err(e) = sce_build::forge::ast_export::write_envelope_to_path(&out_path, &parsed) {
            error_format.emit_and_exit(
                &CliError::WriteOutput {
                    path: out_path.display().to_string(),
                    source: e,
                },
                "",
            );
        }
    }
}

// ── Subcommand: generate ────────────────────────────────────────

fn cmd_generate(args: GenerateArgs, error_format: ErrorFormat) {
    // Unpack the CLI args struct into the internal borrowed names the
    // body uses. `args` owns the data for the whole function, so the
    // `&` / `as_deref()` borrows below stay valid for every use.
    let GenerateArgs {
        scxml,
        language,
        output_dir,
        as_child,
        parent_stem,
        write_deps,
        go_module_prefix,
        format_style,
        no_format,
        deploy,
        transport_only,
        partition,
        const_fold_budget,
        no_std,
        input_root,
        emit_ast,
        kotlin_package_prefix,
        cpp_namespace_prefix,
        c_symbol_prefix,
        strict_unresolved,
        include_dir,
    } = args;
    let scxml_path: &str = &scxml;
    let language: &str = &language;
    let output_dir: &str = &output_dir;
    let parent_stem = parent_stem.as_deref();
    let depfile_path = write_deps.as_deref();
    let go_module_prefix = go_module_prefix.as_deref();
    let format_style = format_style.as_deref();
    let deploy_path = deploy.as_deref();
    let for_partition = partition.as_deref();
    let input_root_override = input_root.as_deref();
    let emit_ast_path = emit_ast.as_deref();
    let kotlin_package_prefix = kotlin_package_prefix.as_deref();
    let cpp_namespace_prefix = cpp_namespace_prefix.as_deref();
    let c_symbol_prefix = c_symbol_prefix.as_deref();
    let include_dirs: &[String] = &include_dir;

    let lang: Language = language.parse().unwrap_or_else(|_| {
        error_format.emit_and_exit(
            &CliError::UnknownLanguage {
                lang: language.to_string(),
            },
            "",
        )
    });

    // Every stdout artifact, the `needs_script_engine` verdict, and
    // any W3C rejection are collected here and flushed as a single
    // JSON manifest at each function exit. Stays local — no globals.
    let mut report = GenerateReport::default();

    // C++ formatter: created once and reused for all output files.
    let cpp_formatter = create_cpp_formatter(lang, format_style, no_format);

    // SCE Forge: detect non-statechart kind and route to forge pipeline.
    // Read the file once; the same content is reused for both detection and compilation.
    let scxml_content = fs::read_to_string(scxml_path).unwrap_or_else(|e| {
        error_format.emit_and_exit(
            &CliError::ReadInput {
                path: scxml_path.to_string(),
                source: e,
            },
            "",
        )
    });

    // Spec §synth-6.2.6 drift context — input root defaults to the SCXML
    // file's parent so the hash covers every `*.scxml` in that
    // directory (the common-case test layout under
    // `resources/<num>/`). The `--input-root` flag overrides the
    // default for staged-input workflows (e.g. the donedata regen
    // script copies its tracked fixture into a tmp dir but needs
    // the embedded source-hash to reproduce against the tracked
    // location, not the transient stage). Pre-computed once and
    // threaded into every file write below so a single invocation's
    // generated tree shares one source-hash / template-hash pair.
    let drift_input_root: std::path::PathBuf = match input_root_override {
        Some(s) => std::path::PathBuf::from(s),
        None => Path::new(scxml_path)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from(".")),
    };
    // Coverage is asserted only when the root was inferred above; an
    // explicit `--input-root` is the caller declaring the source set.
    let drift_ctx = DriftContext::compute(
        &drift_input_root,
        deploy_path.map(Path::new),
        input_root_override.is_none().then(|| Path::new(scxml_path)),
    );

    match sce_build::classify_document(&scxml_content) {
        sce_build::Pipeline::Forge => {
            let input_stem = Path::new(scxml_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            // `input_stem` is the parser's `name` — a pure symbol
            // identifier that flows through `to_snake_case` into Go
            // package names, C++ namespaces, Kotlin packages, etc. Any
            // `.scxml` extension folded into it would corrupt those
            // identifiers (`crossfile_procedure_codec_scxml`).
            //
            // `input_basename` is the diagnostic label — the filename
            // with extension, enough for downstream tooling to open
            // the file without guessing the suffix. Passed through the
            // library as the `diagnostic_label` role of
            // `DocumentLabel`, keeping the two concerns separate all
            // the way down to XSD `source_label` and every
            // `Located::file`.
            let input_basename = Path::new(scxml_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(input_stem);
            let doc_label = sce_build::DocumentLabel {
                identifier: input_stem,
                diagnostic_label: input_basename,
            };

            let base_dir = Path::new(scxml_path)
                .parent()
                .unwrap_or_else(|| Path::new("."));
            let forge_opts = sce_build::ForgeCompileOptions {
                go_module_prefix: go_module_prefix.map(str::to_owned),
                const_fold_budget,
                ..Default::default()
            };

            // Single-parse path: parse once, emit AST if requested,
            // then run codegen against the same `ParsedForge` value via
            // `compile_forge_from_parsed`. Avoids the previous
            // architectural mismatch where the CLI parsed for emit and
            // `compile_forge_with_imports` parsed again internally —
            // `ParsedForge` is a cacheable artefact, not throwaway work.
            let parsed =
                match sce_build::forge::parser::parse_forge_with_imports(&scxml_content, doc_label)
                {
                    Ok(Some(p)) => p,
                    Ok(None) => {
                        // Defensive — `classify_document` already routed
                        // Statechart to the Scxml arm. A future classifier
                        // refactor must not silently land here.
                        error_format.emit_forge_and_exit(&sce_build::forge::error::Located::new(
                            sce_build::forge::error::ValidationError::WrongPipeline {
                                kind: sce_build::forge::model::ForgeKind::Statechart,
                            }
                            .into(),
                            doc_label.diagnostic_label,
                            None,
                            None,
                        ));
                    }
                    Err(e) => error_format.emit_forge_and_exit(&e),
                };

            if let Some(ast_path) = emit_ast_path {
                let path = std::path::Path::new(ast_path);
                if let Err(e) = sce_build::forge::ast_export::write_envelope_to_path(path, &parsed)
                {
                    error_format.emit_and_exit(
                        &CliError::WriteOutput {
                            path: ast_path.to_string(),
                            source: e,
                        },
                        "",
                    );
                }
            }

            match sce_build::compile_forge_from_parsed(
                &parsed,
                doc_label,
                lang,
                base_dir,
                &forge_opts,
            ) {
                Ok(output) => {
                    let import_deps = output.deps;
                    let files = maybe_format_files(output.files, &cpp_formatter);
                    let out = Path::new(output_dir);
                    for (filename, code) in &files {
                        let path = out.join(filename);
                        write_drift_aware(error_format, &path, code, &drift_ctx);
                        report.artifacts.push(path.clone());
                    }
                    if let Some(dep_path) = depfile_path {
                        // Through `write_depfile`, not a local
                        // `format!`. The inline version this replaced
                        // named the input `.scxml` and nothing else — no
                        // templates on any of the six backends, and no
                        // `<sce:import>` targets — so editing
                        // `forge/cpp/codec.h.jinja2` or an imported
                        // document left the output stale while the build
                        // reported success.
                        //
                        // Imports belong here because the importing
                        // document's `source-hash` covers them: editing
                        // an imported document demonstrably changes this
                        // document's output.
                        //
                        // They come from `GeneratedOutput::deps` — what
                        // the compile read — rather than from
                        // `parsed.imports`, which is the *direct* imports
                        // only. Re-deriving them here was the same
                        // second-source defect one layer over: in an
                        // `algorithm → codec → codec` chain the leaf went
                        // undeclared while widening it still changed this
                        // document's `source-hash`.
                        let out = Path::new(output_dir);
                        let targets: Vec<PathBuf> =
                            files.iter().map(|(f, _)| out.join(f)).collect();
                        // The forge scope, not the statechart one:
                        // `forge::generator::generate_*` loads
                        // `templates/forge/<lang>`.
                        let forge_template_dir = sce_build::find_template_base()
                            .join("forge")
                            .join(lang.forge_template_subdir());
                        write_depfile(
                            dep_path,
                            DepfileInputs {
                                output_paths: &targets,
                                template_dir: &forge_template_dir,
                                lang,
                                scxml_input: Path::new(scxml_path),
                                preprocessor_deps: &import_deps,
                                source_set: &drift_ctx.sources,
                                self_written: &[],
                            },
                        );
                    }
                    report.needs_script_engine = Some(false);
                    emit_generate_manifest(&report);
                    return;
                }
                Err(e) => error_format.emit_forge_and_exit(&e),
            }
        }
        sce_build::Pipeline::Scxml => {}
    }

    let template_dir = sce_build::find_template_dir_for(lang);

    // The `--include-dir` search path lets `<xi:include>` / `<sce:use>`
    // resolve fragments by name; empty in the common case, so the
    // parser resolves exactly as `absolute → base → cwd` when no
    // include dirs are passed.
    let mut parser =
        SCXMLParser::new().with_include_dirs(include_dirs.iter().map(PathBuf::from).collect());
    // Typed parser failures (XML/XSD/validation) flow straight to the
    // unified diagnostic emitter — the old CliError::ScxmlParse
    // wrapper collapsed forge codes into cli/scxml-parse, losing the
    // xml/* / validation/* signal agents dispatch on.
    let mut model = match parser.parse_file(scxml_path) {
        Ok(m) => m,
        Err(e) => error_format.emit_and_exit(&e, ""),
    };

    // `--strict-unresolved` lifts the
    // model's `<sce:unresolved>` markers from silent metadata to a
    // build-failing rejection. Runs before any codegen so CI gates
    // see the `validation/unresolved-placeholder` diagnostic on the
    // wire instead of a downstream "why is my generated code blank
    // at this assign expression" surprise.
    if strict_unresolved {
        if let Err(e) = sce_build::unresolved_check::check_strict_unresolved(&model) {
            error_format.emit_and_exit(&e, "");
        }
    }

    // `has_parent_communication` carries two distinct meanings across
    // backends:
    //   - cpp/rust/kotlin/go/python: enables the `ParentStateMachine`
    //     class-template parameter (analyzer derives `needs_parent_template
    //     = has_parent_communication && !is_remote_invoke_target`). Every
    //     `--as-child` invocation must template against the parent type
    //     because the parent's invoke spawn site passes `self_` (parent
    //     pointer) into the child constructor — even when the child has
    //     no `<send target="#_parent">` of its own.
    //   - c11: gates which init entrypoint the child emits — `_init`
    //     when false, `_init_with_parent` when true. The c11 parent's
    //     `invoke_methods.jinja2` switches on the parser-derived
    //     `invoke_info.child_has_send_to_parent`, so forcing the child
    //     side to `true` while the parent calls `_init` produces a
    //     declaration/call mismatch (child header declares only
    //     `_init_with_parent`, parent's `.c` references `_init` →
    //     linker error). For c11 the parser-derived value is the
    //     correct gate — children that genuinely route to `#_parent`
    //     (test191/338) flip `has_parent_communication=true` on their
    //     own, and `<donedata>`-only children (donedata_local_invoke)
    //     compile cleanly under the parser-derived `false`.
    if as_child && lang != Language::C11 {
        model.has_parent_communication = true;
    }

    analyzer::analyze(&mut model, scxml_path);

    // AST export — emit the analyzed model BEFORE any deploy-time
    // mutations (resolve_source_path, inject_server_model_mutations,
    // inject_partition_context_for, populate_event_queue_capacity_
    // from_deploy) so the envelope captures the parser+analyzer IR
    // — the statechart parallel of `ParsedForge` for forge kinds.
    //
    // Skip-on-rejected: document_rejected is the W3C 5.8
    // structured rejection (unloadable external script). Skip the
    // emit and fall through to the rejection-stub codegen path
    // below; the absence of the envelope file is the consumer
    // signal, matching the Forge precedent "parse fails → no envelope".
    if let Some(ast_path) = emit_ast_path {
        if !model.document_rejected {
            let parsed = sce_build::forge::ast_export::statechart_parsed_forge(model.clone());
            let path = Path::new(ast_path);
            if let Err(e) = sce_build::forge::ast_export::write_envelope_to_path(path, &parsed) {
                error_format.emit_and_exit(
                    &CliError::WriteOutput {
                        path: ast_path.to_string(),
                        source: e,
                    },
                    "",
                );
            }
        }
    }

    // §scxml-5.8: Document rejected at parse time (e.g., unloadable external script)
    // Generate a language-appropriate rejection stub so AOT test reports PASS.
    if model.document_rejected {
        let input_stem = Path::new(scxml_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let out = Path::new(output_dir);
        let pascal = crate::filters::to_pascal_case(input_stem.to_string());

        // §synth-5-O traceability — every drift-headered file must carry an
        // `SCE-MAP:` marker, otherwise `validate_emitted_files_have_markers`
        // fires `traceability/meta-generated-source-line-marker-missing`
        // on the next codegen call in the same output dir. Rejection
        // stubs go through `write_drift_aware` (which prepends the
        // §synth-6.2.6 header), so they MUST include a marker line too. Use
        // the SCXML basename + line 1 — the document was rejected at
        // parse time, no finer location is available.
        let scxml_basename = Path::new(scxml_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown.scxml");

        match lang {
            Language::Cpp => {
                let header = format!(
                    "// W3C SCXML 5.8: Document rejected\n\
                     // SCE-MAP: {scxml_basename}:1\n\
                     #pragma once\n\
                     #define SCE_DOCUMENT_REJECTED 1\n\
                     namespace SCE::Generated::{name} {{\n\
                     struct {pascal} {{\n\
                     }};\n\
                     }}  // namespace SCE::Generated::{name}\n",
                    name = input_stem,
                    pascal = pascal
                );
                let inl = "// W3C SCXML 5.8: Document rejected\n";
                write_drift_aware(
                    error_format,
                    out.join(format!("{input_stem}_sm.h")),
                    &header,
                    &drift_ctx,
                );
                write_drift_aware(
                    error_format,
                    out.join(format!("{input_stem}_sm.inl")),
                    inl,
                    &drift_ctx,
                );
            }
            Language::Rust => {
                let stub = format!(
                    "// W3C SCXML 5.8: Document rejected\n\
                     // SCE-MAP: {scxml_basename}:1\n\
                     // This state machine was rejected at parse time.\n"
                );
                write_drift_aware(
                    error_format,
                    out.join(format!("{input_stem}_sm.rs")),
                    &stub,
                    &drift_ctx,
                );
            }
            Language::Kotlin => {
                let stub = format!(
                    "// W3C SCXML 5.8: Document rejected\n\
                     // SCE-MAP: {scxml_basename}:1\n\
                     package com.sce.generated.{name}\n",
                    name = input_stem
                );
                write_drift_aware(
                    error_format,
                    out.join(format!("{input_stem}Sm.kt")),
                    &stub,
                    &drift_ctx,
                );
            }
            Language::Go => {
                let stub = format!(
                    "// W3C SCXML 5.8: Document rejected\n\
                     // SCE-MAP: {scxml_basename}:1\n\
                     package {name}\n",
                    name = input_stem
                );
                write_drift_aware(
                    error_format,
                    out.join(format!("{input_stem}_sm.go")),
                    &stub,
                    &drift_ctx,
                );
            }
            Language::Python => {
                let stub = format!(
                    "# W3C SCXML 5.8: Document rejected\n\
                     # SCE-MAP: {scxml_basename}:1\n"
                );
                write_drift_aware(
                    error_format,
                    out.join(format!("{input_stem}_sm.py")),
                    &stub,
                    &drift_ctx,
                );
            }
            Language::C11 => {
                // C11 rejected-document sentinel: emit a header-only
                // sentinel matching the C++ shape so any downstream
                // consumer that includes the .h compiles to a no-op.
                // The body file
                // carries an `extern const int` definition so the
                // translation unit is non-empty (ISO C forbids empty
                // translation units, surfaces under
                // `-Wpedantic -Werror`); the symbol's name doubles as a
                // grep-able marker that links cleanly even if the host
                // never declares it.
                let guard = filters::to_snake_case(input_stem.to_string()).to_uppercase();
                let header = format!(
                    "/* W3C SCXML 5.8: Document rejected */\n\
                     // SCE-MAP: {scxml_basename}:1\n\
                     #ifndef SCE_GEN_{guard}_SM_H\n\
                     #define SCE_GEN_{guard}_SM_H\n\
                     #define SCE_DOCUMENT_REJECTED 1\n\
                     extern const int sce_document_rejected_{stem};\n\
                     #endif\n",
                    guard = guard,
                    stem = input_stem
                );
                let body = format!(
                    "/* W3C SCXML 5.8: Document rejected */\n\
                     // SCE-MAP: {scxml_basename}:1\n\
                     #include \"{input_stem}_sm.h\"\n\
                     const int sce_document_rejected_{stem} = 1;\n",
                    input_stem = input_stem,
                    stem = input_stem
                );
                write_drift_aware(
                    error_format,
                    out.join(format!("{input_stem}_sm.h")),
                    &header,
                    &drift_ctx,
                );
                write_drift_aware(
                    error_format,
                    out.join(format!("{input_stem}_sm.c")),
                    &body,
                    &drift_ctx,
                );
            }
        }
        report.rejected = Some(RejectedDocument {
            spec: "W3C SCXML 5.8",
            name: model.name.clone(),
        });
        report.needs_script_engine = Some(false);
        emit_generate_manifest(&report);
        return;
    }

    if let Err(err) = analyzer::can_generate_static(&model) {
        // §wire-W5 D3: `can_generate_static` returns the
        // correctly-classified ForgeError directly — `ScxmlSemanticError`
        // for hard semantic violations (top-level script rejected,
        // initial-state names undeclared) and `ValidationDynamicFeatures`
        // for genuine codegen limitations (no initial attribute).
        let located = sce_build::forge::error::Located::new(err, scxml_path, None, None);
        error_format.emit_and_exit(&located, "");
    }

    // SCE Protocol-Synthesis RFC §synth-5-J-2: Rust no_std variant
    // rejection. Only fires when `--no-std` is paired with `-l rust`
    // (the flag is a no-op for other language targets, mirroring how
    // `--go-module-prefix` is rust/kotlin-inert). Two axes:
    //   1. `<script>` — Lua/QuickJS need `alloc`, no_std forbids it.
    //   2. BasicHTTP send — tokio/reqwest are std-coupled.
    // The model already carries `needs_script_engine` /
    // `has_unresolved_external_script` / `needs_http_send` flags from
    // the parser + analyzer passes; this gate just reads them.
    if no_std && lang == Language::Rust {
        if let Err(err) = sce_build::validate_no_std_compatibility(&model, Path::new(scxml_path)) {
            let located = sce_build::forge::error::Located::new(err, scxml_path, None, None);
            error_format.emit_and_exit(&located, "");
        }
    }

    resolve_source_path(&mut model, Path::new(scxml_path));

    let input_stem = Path::new(scxml_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Typed generator failures route through `Located<ForgeError>` so
    // the NDJSON wire carries the real `generate/*` code (template-load,
    // template-render, invalid-config) instead of collapsing to
    // `cli/scxml-generate`. minijinja does not surface the failing
    // template's row/col through its public error surface, so `None`
    // line/col is honest — fabricating `(1, 1)` would misroute agent
    // repair loops to the top of the source (see
    // `feedback_correctness_before_features`).
    // out_path is needed early so synth-invoke SCXMLs can be
    // materialised to `-o` (and to `deploy_dir` when distinct) BEFORE
    // `inject_partition_context_for` runs. The mesh injector iterates
    // every machine declared in `deploy.yaml` (including synth-invoke
    // children) and opens each by name from `deploy_dir`; without a
    // file on disk the open fails with `Partition context injection
    // error: SCXML file not found`. The dir is also created here so
    // the synth write below has somewhere to land.
    let out_path = Path::new(output_dir);
    fs::create_dir_all(out_path).unwrap_or_else(|e| {
        error_format.emit_and_exit(
            &CliError::CreateOutputDir {
                path: out_path.display().to_string(),
                source: e,
            },
            "",
        )
    });

    // SCE_MESH.md §9.6.6: re-materialise every inline-`<content>`
    // synth child to `-o` (and `deploy_dir` when distinct) so
    // downstream consumers find a file on disk:
    // - `inject_partition_context_for` opens every declared machine
    //   SCXML when `--deploy` is set (reads from `deploy_dir`)
    // - CMake's stage-3 synth codegen reads from the parent's `-o`
    //   (`tests/CMakeLists.txt:2442-2456`)
    // - W3C `process_children_<N>.cmake` reads `${child}.scxml` from
    //   the parent's `-o` for the per-child `--as-child` pass
    // Pollution of the parent's *source* directory stays closed: the
    // write targets are caller-controlled (`-o`, `--deploy <path>`),
    // never `scxml_path`'s parent.
    // Every file this invocation writes that is not a declared
    // artefact. Collected as they are written, so the depfile filter
    // cannot fall behind a new side-effect write.
    let mut self_written: Vec<PathBuf> = Vec::new();
    let synth_scxml_writes: Vec<(String, String)> = model
        .iter_scxml_invokes()
        .filter_map(|inv| {
            inv.inline_child_xml.as_deref().and_then(|xml| {
                if inv.child_name.is_empty() {
                    None
                } else {
                    Some((inv.child_name.clone(), xml.to_string()))
                }
            })
        })
        .collect();
    for (stem, xml) in &synth_scxml_writes {
        // A synthesised child is an artefact like any other: it is read
        // back by the next codegen pass and by humans, so it carries
        // the same trailing-newline contract as the sources beside it.
        let xml = with_trailing_newline(xml);
        let xml = xml.as_ref();
        let dst = out_path.join(format!("{stem}.scxml"));
        if let Err(e) = fs::write(&dst, xml) {
            eprintln!(
                "Warning: Cannot write synth SCXML to -o: {}: {e}",
                dst.display()
            );
        }
        // Recorded so the depfile does not name it. `-o` is frequently
        // the directory the input was staged into, which puts this file
        // in the invocation's own source set.
        self_written.push(dst);
        if let Some(deploy_file) = deploy_path {
            let deploy_dir = Path::new(deploy_file).parent().unwrap_or(Path::new("."));
            if deploy_dir != out_path {
                let mirror = deploy_dir.join(format!("{stem}.scxml"));
                if let Err(e) = fs::write(&mirror, xml) {
                    eprintln!(
                        "Warning: Cannot write synth SCXML to deploy_dir: {}: {e}",
                        mirror.display()
                    );
                }
                self_written.push(mirror);
            }
        }
    }

    // SCE Mesh: every deploy-derived model mutation, before SM
    // generation. The SM generator must see the injected server-response
    // <send> actions to emit the raiseExternal calls that drive
    // handleServerResponse routing, and must see the partition context
    // (SCE_MESH.md §14 rule 12) to pick the right `<parallel>`-final
    // branch and the right `<send target="#_parent">` site.
    //
    // The sequence lives in `apply_deploy_model_mutations` rather than
    // here so that anything else compiling with a deploy in scope — the
    // library entry `compile_scxml_lang_with_deploy`, and the gates that
    // use it — applies the same mutations in the same order. When it sat
    // inline in this function it was, in practice, unavailable to
    // everything else, and the value gate over author `<param>` literals
    // silently compiled a different shape of machine as a result.
    if let Some(deploy_file) = deploy_path {
        match sce_build::apply_deploy_model_mutations(
            &mut model,
            Path::new(deploy_file),
            for_partition,
        ) {
            // The descriptive half of the deploy travels into the
            // manifest. Without a reader a declaration is
            // indistinguishable from a typo — the author believes they
            // configured something and nothing in the build says
            // otherwise.
            Ok(facts) => report.deploy_facts = Some(facts),
            Err(e) => error_format.emit_and_exit(&e.source, e.stage.error_prefix()),
        }
    }

    let locate_codegen = |e: sce_build::forge::error::GenerateError| -> Located<ForgeError> {
        Located::new(ForgeError::from(e), scxml_path, None, None)
    };

    // mesh_open_issues.md Issue 2: when --transport-only is set, skip
    // the state-machine backend emit + its side-product files
    // (sourcemap, children manifest, static-invoke copy, hybrid stubs).
    // The transport-only branch still threads `out_path` and the report
    // through to the mesh emit block below so depfile + sourcemap-marker
    // validation operate on the transport header alone. `out_path` +
    // `fs::create_dir_all` ran above (before the synth-SCXML emit + the
    // mesh injection chain) — the dir already exists at this point.
    let mut output_paths: Vec<PathBuf> = Vec::new();

    if !transport_only {
        // Closure-extracted per-language emit. Used for the parent model
        // first, then once per inline `<invoke><content>` child so the
        // synth-invoke `_sm.*` artefacts land next to the parent in
        // `output_dir`. Before this loop existed the standalone generate
        // skipped inline children entirely — downstream consumers had to
        // re-invoke `sce-codegen generate --as-child` per child after the
        // parser dropped a sibling `.scxml` next to the source. With the
        // parser purified (no disk side-effect), codegen emit is the
        // single materialization point for synth children.
        //
        // `<sce:capacity>` parsing, the deploy.yaml
        // `default_event_queue_capacity` populator, and the
        // `pub const EVENT_QUEUE_CAPACITY` template emission feed the
        // heapless event-queue bound. The `--no-std` CLI flag both
        // validates compatibility (script/HTTP rejection — consumed
        // earlier in `validate_no_std_compatibility`) and switches
        // the templates to the no_std emission: `#![no_std]`,
        // `core::time::Duration`, and the profile-resolving runtime
        // collection aliases across the sub-templates
        // (send.rs.jinja2 / process_transition.rs.jinja2 /
        // invoke_methods.rs.jinja2).
        let emit_for_model = |m: &SCXMLModel,
                              stem: &str,
                              m_as_child: bool,
                              m_parent_stem: Option<&str>|
         -> GeneratedOutput {
            match lang {
                Language::Rust => {
                    let code = sce_build::generator::generate(m, &template_dir, no_std)
                        .unwrap_or_else(|e| error_format.emit_forge_and_exit(&locate_codegen(e)));
                    GeneratedOutput {
                        files: vec![(format!("{stem}_sm.rs"), code)],
                        // CLI threads `Parser::preprocessor_deps()` directly to
                        // the depfile sink (see `--write-deps` handling below);
                        // populating `GeneratedOutput.deps` here would
                        // duplicate the channel without a consumer.
                        ..Default::default()
                    }
                }
                Language::Cpp => {
                    sce_build::generator::generate_cpp(m, &template_dir, stem, cpp_namespace_prefix)
                        .unwrap_or_else(|e| error_format.emit_forge_and_exit(&locate_codegen(e)))
                }
                Language::Kotlin => {
                    let mut code = sce_build::generator::generate_kotlin(
                        m,
                        &template_dir,
                        kotlin_package_prefix,
                    )
                    .unwrap_or_else(|e| error_format.emit_forge_and_exit(&locate_codegen(e)));
                    // Mirror `generate-w3c`'s KotlinBackend::process_child: the
                    // child's self-derived package (`<prefix>.{child}`) is
                    // rewritten to the parent's package so the parent's
                    // unqualified reference to the child `StateMachine` class
                    // resolves within one compilation unit. The `<prefix>` mirrors
                    // whatever `--kotlin-package-prefix` selected (defaults to
                    // `com.sce.generated` for W3C, `com.sce.integration` for the
                    // integration tree).
                    if m_as_child {
                        if let Some(parent) = m_parent_stem {
                            let prefix = kotlin_package_prefix.unwrap_or("com.sce.generated");
                            let child_pkg = sce_build::filters::to_snake_case(stem.to_string());
                            code = code.replace(
                                &format!("package {prefix}.{child_pkg}"),
                                &format!("package {prefix}.{parent}"),
                            );
                        }
                    }
                    GeneratedOutput {
                        files: vec![(format!("{stem}Sm.kt"), code)],
                        ..Default::default()
                    }
                }
                Language::Go => {
                    let mut code = sce_build::generator::generate_go(m, &template_dir)
                        .unwrap_or_else(|e| error_format.emit_forge_and_exit(&locate_codegen(e)));
                    // Same rewrite as Kotlin, for the Go `package <child>` header.
                    if m_as_child {
                        if let Some(parent) = m_parent_stem {
                            let child_pkg = sce_build::filters::to_snake_case(stem.to_string());
                            code = code.replace(
                                &format!("package {child_pkg}"),
                                &format!("package {parent}"),
                            );
                        }
                    }
                    GeneratedOutput {
                        files: vec![(format!("{stem}_sm.go"), code)],
                        ..Default::default()
                    }
                }
                Language::Python => {
                    let code = sce_build::generator::generate_python(m, &template_dir)
                        .unwrap_or_else(|e| error_format.emit_forge_and_exit(&locate_codegen(e)));
                    GeneratedOutput {
                        files: vec![(format!("{stem}_sm.py"), code)],
                        ..Default::default()
                    }
                }
                Language::C11 => {
                    sce_build::generator::generate_c11(m, &template_dir, stem, c_symbol_prefix)
                        .unwrap_or_else(|e| error_format.emit_forge_and_exit(&locate_codegen(e)))
                }
            }
        };

        let output = emit_for_model(&model, input_stem, as_child, parent_stem);

        let files = maybe_format_files(output.files, &cpp_formatter);
        for (filename, code) in &files {
            let file_path = out_path.join(filename);
            write_drift_aware(error_format, &file_path, code, &drift_ctx);
            report.artifacts.push(file_path.clone());
            output_paths.push(file_path);
        }

        // SCE Protocol-Synthesis RFC §synth-5-O — sourcemap JSON sidecar
        // alongside the per-language SM output. The single-SCXML codegen
        // path writes one sourcemap per emit; cross-backend byte-identity
        // is preserved because the symbol table + hashes are language-
        // agnostic.
        emit_sourcemap_for_machine(&model, out_path, &drift_ctx);

        // SCE_MESH.md §9.6.6: emit synth-invoke children alongside the
        // parent. Mirrors `generate_child_sms` (the W3C unified path) for
        // the standalone `generate` CLI so a consumer no longer drives a
        // separate `--as-child` pass per inline `<content>` child. C11
        // skips the `has_parent_communication=true` override that the
        // other backends use under `--as-child` because C11 child
        // templates auto-detect parent-communication from the model and
        // the override would break that detection.
        for invoke in model.iter_scxml_invokes() {
            let Some(child_box) = invoke.inline_child.as_deref() else {
                continue;
            };
            let child_stem = invoke.child_name.as_str();
            if child_stem.is_empty() {
                continue;
            }

            // Synth SCXML was already written to `out_path` (and
            // `deploy_dir` when distinct) earlier in this function,
            // before the mesh-injection chain that depends on the
            // file. No second emit here.

            let mut child_model = child_box.clone();
            if lang != Language::C11 {
                child_model.has_parent_communication = true;
            }
            let synthetic_path = Path::new(scxml_path)
                .parent()
                .unwrap_or(Path::new("."))
                .join(format!("{child_stem}.scxml"));
            analyzer::analyze(&mut child_model, &synthetic_path.to_string_lossy());
            if analyzer::can_generate_static(&child_model).is_err() {
                continue;
            }
            // `resolve_source_path` canonicalizes the path, which fails
            // on the synthetic (the file does not exist on disk —
            // inline children live in-memory). Set `scxml_source_path`
            // directly so the `// From:` license header + Forge AST
            // envelope match the byte-stable goldens from the prior
            // `--as-child` flow that staged the synth child on disk.
            child_model.scxml_source_path = synthetic_path.to_string_lossy().into_owned();
            let child_output = emit_for_model(&child_model, child_stem, true, Some(input_stem));
            let child_files = maybe_format_files(child_output.files, &cpp_formatter);
            for (filename, code) in &child_files {
                let file_path = out_path.join(filename);
                write_drift_aware(error_format, &file_path, code, &drift_ctx);
                report.artifacts.push(file_path.clone());
                output_paths.push(file_path);
            }
            emit_sourcemap_for_machine(&child_model, out_path, &drift_ctx);
        }

        report.needs_script_engine = Some(model.needs_script_engine);
        // Projected from the list the analyzer stored on the model in the
        // same statement that set the flag — not recomputed here, which
        // would re-derive it from a model later passes have since touched.
        // The flag and its explanation cannot disagree.
        report.script_engine_causes = model
            .script_engine_causes
            .iter()
            .map(|c| c.to_wire())
            .collect();

        // §scxml-6.4: Generate children metadata + hybrid SCXML stubs for all languages.
        // C++ uses _children.txt for CMake post-processing; all languages need hybrid stubs.
        let children = collect_invoke_child_names(&model);
        if lang == Language::Cpp && !children.is_empty() {
            let children_file = out_path.join(format!("{input_stem}_children.txt"));
            write_or_exit(error_format, &children_file, children.join("\n") + "\n");
        }
        // §scxml-6.4: Copy static invoke child SCXML files to the output
        // directory so CMake's post-processing script can find them next to the
        // parent. `process_static_invokes` extracts inline <scxml> content to
        // the *source* directory; the build system expects them in OUTPUT_DIR.
        copy_static_invoke_children(&model, Path::new(scxml_path), out_path);
        // §scxml-6.4 (test216/530): hybrid stub destination is backend-aware.
        // cpp's CMake harness drives child codegen from OUTPUT_DIR (its
        // `process_children_<N>.cmake` reads `<OUTPUT_DIR>/<child>.scxml`), so
        // hybrid stubs land alongside the parent's generated files. c11 discovers
        // children via RESOURCE_DIR GLOBs at CMake configure time — a stub
        // emitted only into OUTPUT_DIR is invisible to that GLOB on the first
        // build. Mirroring `process_static_invokes` for inline `<content>`,
        // the c11 stub is written to the SCXML source directory so the same
        // configure-time discovery flow picks it up.
        let hybrid_dest = if lang == Language::C11 {
            Path::new(scxml_path).parent().unwrap_or(Path::new("."))
        } else {
            out_path
        };
        self_written.extend(generate_hybrid_child_scxmls(&model, hybrid_dest));
    } // end of `if !transport_only` — mesh transport emit follows.

    // SCE Mesh: generate transport routing code when --deploy is provided.
    // Uses the public API (compile_mesh_transport) so CLI, tests, and build.rs
    // share the same entry point. Server-response injection ran above (pre-SM)
    // and is idempotent, so the re-run inside compile_mesh_transport is a no-op.
    if let Some(deploy_file) = deploy_path {
        match sce_build::compile_mesh_transport(&mut model, Path::new(deploy_file), lang) {
            Ok(result) => {
                for w in &result.dynamic_target_warnings {
                    eprintln!("Warning: {w}");
                }
                for n in &result.deadline_override_notices {
                    eprintln!("Notice: {n}");
                }
                for n in &result.subscription_lint_notices {
                    eprintln!("Lint: {n}");
                }
                // SCE_MESH.md §7.7 circular dependency detection. A ring
                // is reported once per build, by its lexicographically
                // smallest member, so repeating the machine name here
                // would be misleading — the cycle text names every
                // participant already.
                for c in &result.invoke_wait_cycles {
                    eprintln!("Warning: {c}");
                }
                // SCE_MESH.md §16.4 auto-merge notice stream: surfacing
                // the merge events is what keeps permissive mode out of
                // the silently-broken-hook pattern — an author who
                // wrote a split partition plan must see that the
                // analyzer collapsed it, otherwise the build behaves
                // differently from what the author requested.
                for n in &result.distributability_merge_notices {
                    eprintln!(
                        "Notice: machine '{}' <parallel id=\"{}\"> {:?} auto-merged — \
                         partitions {:?} absorbed into '{}' (SCE_MESH.md §16.4).",
                        n.machine, n.parallel_id, n.rule, n.absorbed, n.canonical,
                    );
                }
                // SCE_MESH.md §16.3 R3 snapshot-read notices: advisory
                // only; printed so authors see "entry-point sync
                // required" cues without a build error.
                for n in &result.distributability_snapshot_notices {
                    eprintln!(
                        "Notice: machine '{}' <parallel id=\"{}\"> region '{}' reads \
                         ancestor data '{}' that sibling region '{}' writes — \
                         snapshot captured at parallel entry (SCE_MESH.md §16.3 R3).",
                        n.machine, n.parallel_id, n.reader_region, n.location, n.writer_region,
                    );
                }
                let mesh_files = maybe_format_files(result.output.files, &cpp_formatter);
                for (filename, code) in &mesh_files {
                    let file_path = out_path.join(filename);
                    write_drift_aware(error_format, &file_path, code, &drift_ctx);
                    report.artifacts.push(file_path.clone());
                    output_paths.push(file_path);
                }
            }
            Err(e) => error_format.emit_and_exit(&e, "Mesh error: "),
        }
    }

    // Write DEPFILE for CMake incremental builds. Preprocessor deps
    // (xi:include targets, sce:use template fragments) come from the
    // parser instance — they are the actual external files
    // `parse_file` consumed during this invocation, so editing one
    // correctly invalidates `<case>_sm.{h,inl}` artifacts on the next
    // ninja/make run. tc8-harness's CMake glob workaround can be
    // retired once consumers pick up this flag.
    if let Some(depfile) = depfile_path {
        write_depfile(
            depfile,
            DepfileInputs {
                output_paths: &output_paths,
                template_dir: &template_dir,
                lang,
                scxml_input: Path::new(scxml_path),
                preprocessor_deps: parser.preprocessor_deps(),
                source_set: &drift_ctx.sources,
                self_written: &self_written,
            },
        );
    }

    // §synth-5-O ownership-boundary walker. Every
    // SCE-emitted file (one carrying a §synth-6.2.6 drift header) must
    // contain at least one `SCE-MAP:` marker per ARCHITECTURE.md
    // "Traceability Ownership Boundary". External meta-generator
    // output (no drift header) is silently out-of-scope. Fires
    // `traceability/meta-generated-source-line-marker-missing` on
    // codegen-internal regression — surfaces immediately rather than
    // letting a broken template ship.
    if let Err(err) =
        sce_build::forge::sourcemap::validate_emitted_files_have_markers(Path::new(output_dir))
    {
        error_format.emit_and_exit(&err, "");
    }

    emit_generate_manifest(&report);
}

/// Collect child SCXML names from model's static/hybrid invokes.
fn collect_invoke_child_names(model: &SCXMLModel) -> Vec<String> {
    let mut children = Vec::new();
    for invoke in model.iter_scxml_invokes() {
        if !invoke.child_name.is_empty() {
            children.push(invoke.child_name.clone());
        }
    }
    for invoke in model.iter_hybrid_invokes() {
        if !invoke.child_name.is_empty() {
            children.push(invoke.child_name.clone());
        }
    }
    children
}

/// §scxml-6.4: Copy external `src="…"` static invoke children from
/// source to output directory.
///
/// CMake post-processing for the W3C test corpus expects static child
/// `.scxml` files alongside their parent in the output tree. External
/// (`<invoke src="file.scxml"/>`) children live on disk in the parent's
/// source directory and are copied unchanged into `output_dir`.
///
/// Inline (`<invoke><content><scxml>…</scxml></content></invoke>`) children
/// are kept in-memory on [`ScxmlInvokeInfo::inline_child`] and never
/// materialise as parent-adjacent files (Mesh §9.6.6 naming is enforced by
/// codegen emit, not by a parser write). They are skipped here — there is
/// no source-side file to copy and downstream codegen reads them from the
/// parent model directly.
fn copy_static_invoke_children(model: &SCXMLModel, scxml_path: &Path, output_dir: &Path) {
    let source_dir = scxml_path.parent().unwrap_or(Path::new("."));

    for invoke in model.iter_scxml_invokes() {
        if invoke.child_name.is_empty() || invoke.inline_child.is_some() {
            continue;
        }
        let child_scxml = format!("{}.scxml", invoke.child_name);
        let src = source_dir.join(&child_scxml);
        let dest = output_dir.join(&child_scxml);

        if src.exists() && !dest.exists() {
            if let Err(e) = std::fs::copy(&src, &dest) {
                eprintln!(
                    "Warning: Cannot copy static invoke child {} to output: {e}",
                    child_scxml
                );
            }
        }
    }
}

/// §scxml-6.4: Generate SCXML files for hybrid invoke children (srcexpr/contentexpr).
///
/// Hybrid invokes resolve their target expression (`srcexpr` / `contentexpr`)
/// at runtime — the AOT backends that consume this stub (`emits_hybrid_child_stub
/// == true`: Rust/Go/C++) evaluate the expression purely for error classification
/// and then instantiate a pre-generated `_hybrid{idx}` policy. That policy's
/// compiled shape is the only runtime-observable contribution of this file, so
/// a trivial immediate-final stub produces the W3C-correct `done.invoke`
/// sequence regardless of what the original SCXML expression would have named.
///
/// The stub's `<scxml name=...>` is aligned with the synthesized child_name so
/// the parser emits matching PascalCase symbols without needing a post-parse
/// rename.
///
/// The stub is rewritten unconditionally (via `write_if_changed`) — the file
/// is codegen-owned, so keeping a stale copy on disk hides generator-logic
/// changes from the next incremental build. `write_if_changed` skips the
/// write when the bytes are identical, so CMake mtime-based dependency
/// tracking stays quiet on no-op runs.
fn generate_hybrid_child_scxmls(model: &SCXMLModel, output_dir: &Path) -> Vec<PathBuf> {
    let mut written = Vec::new();
    for invoke in model.iter_hybrid_invokes() {
        if invoke.child_name.is_empty() {
            continue;
        }
        let child_name = &invoke.child_name;
        let dest = output_dir.join(format!("{child_name}.scxml"));

        let stub = format!(
            "<?xml version=\"1.0\"?>\n\
             <scxml xmlns=\"http://www.w3.org/2005/07/scxml\" \
             name=\"{child_name}\" initial=\"final\" version=\"1.0\">\n\
             \x20 <final id=\"final\"/>\n\
             </scxml>\n"
        );
        write_if_changed(&dest, &stub);
        written.push(dest);
    }
    written
}

/// A path in the form two spellings of the same file compare equal in.
///
/// `canonicalize` when the file exists, the path as given otherwise —
/// which is the right fallback, since a prerequisite that does not exist
/// cannot be something this invocation just wrote.
fn depfile_identity(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Every family of file a depfile is written from.
///
/// Grouped rather than passed one by one because the families differ in
/// *why* they belong, not just in what they hold, and the distinctions
/// are what the axis keeps getting wrong: reachability (templates,
/// imports) versus coverage (the source set) versus production
/// (artefacts, side-effect writes), the last of which must be
/// subtracted rather than added.
struct DepfileInputs<'a> {
    /// Files this invocation declares as artefacts — the depfile's
    /// targets, and the first half of what it must not depend on.
    output_paths: &'a [PathBuf],
    /// Loader scope for the render, whose templates are prerequisites.
    template_dir: &'a Path,
    lang: Language,
    /// The document compiled, named as a prerequisite in its own right.
    scxml_input: &'a Path,
    /// External files the parser consumed: `<xi:include>` targets,
    /// `<sce:use>` fragments, `<sce:import>` closures, and whatever else
    /// a pipeline reports having read.
    preprocessor_deps: &'a [PathBuf],
    /// The synth-6.2.6 source set behind the embedded `source-hash`.
    source_set: &'a [PathBuf],
    /// Side-effect writes: synth children, hybrid stubs.
    self_written: &'a [PathBuf],
}

/// Write CMake DEPFILE (Makefile-format dependency file).
///
/// [`DepfileInputs::self_written`] names files this invocation produced
/// that are not artefacts — the §9.6.6 synth children and the
/// hybrid-invoke stubs, which land in `-o` (or beside the deploy file)
/// as a side effect. They are subtracted from the prerequisites below,
/// and the reason is not cosmetic: a synth child is written into the
/// directory it was generated from, so the *next* run's source set
/// covers it, and declaring it made the mesh §9.6.6 step depend on its
/// own output. Ninja rejects that outright —
/// `dependency cycle: parent_synth_inline__sce_synth_invoke__remote_inv
/// .scxml -> itself` — taking 131 of 378 tests down with it. A file this
/// run wrote cannot be a reason to re-run it.
fn write_depfile(depfile_path: &str, inputs: DepfileInputs<'_>) {
    let DepfileInputs {
        output_paths,
        template_dir,
        lang,
        scxml_input,
        preprocessor_deps,
        source_set,
        self_written,
    } = inputs;
    let mut deps = Vec::new();

    // Add the SCXML input file itself as a dependency
    deps.push(scxml_input.to_path_buf());

    // Add user-side preprocessor inputs (xi:include targets, sce:use
    // template fragments) collected by the parser. Without this slice
    // a fragment edit silently ships stale `_sm.{h,inl}` because
    // CMake/Ninja have no prerequisite to invalidate. See tc8-harness
    // feedback report.
    deps.extend(preprocessor_deps.iter().cloned());

    // The synth-6.2.6 source set behind the `source-hash` every emitted
    // file carries — taken from the `DriftContext` that computed the
    // hash, not re-walked here.
    //
    // These are prerequisites for a different reason than everything
    // else in this list, and that difference is why they were missing.
    // The two families above are reachability families: files the render
    // loaded, documents the compile imported. A source-set member need
    // be none of those — the fold is over every `**/*.scxml` beneath the
    // input root, so a document that is never parsed, never imported and
    // never mentioned still moves the header. Measured on all six
    // backends and both pipelines before this line existed: editing an
    // unrelated sibling changed the `source-hash:` line while nothing
    // declared it, so ninja reused an artefact whose embedded hash no
    // longer described its inputs — precisely what `sce-codegen verify`
    // rejects. Declaring the set is what keeps the freshness contract
    // the spec puts on that header enforceable by the build.
    deps.extend(source_set.iter().cloned());

    // Template dependencies, taken from the loader rather than from a
    // second walk of our own. `loader_template_files` is what
    // `load_templates` registers, so the depfile cannot name a smaller
    // set than the render can reach. Walking `template_dir` alone — the
    // shape this used to have — silently dropped the shared `_macros/`
    // family for every backend scoped to a subdirectory (rust, kotlin,
    // go, python): editing `_macros/sce_map_marker.jinja2` left their
    // output stale while the build reported success. C++ and C11 hid it,
    // their scope being the whole tree.
    //
    // Then minus the templates belonging to other backends. Over-
    // declaring only costs a spurious rebuild, but the cost is real: the
    // hand-kept list this replaced named only rust/kotlin/go, so a C++
    // build declared 18 templates it cannot render and a C11 build 65,
    // and one Rust template edit regenerated all 270 C11 outputs.
    // Removing them is safe because they are outside what this backend's
    // render reads — which `codegen_depfile_content` proves by pruning
    // every undeclared template and re-rendering.
    let foreign = lang.foreign_template_prefixes();
    deps.extend(
        sce_build::generator::loader_template_files(template_dir)
            .into_iter()
            .filter(|(name, _)| {
                // Matched against the loader-registered name, which is
                // relative to the template root — the frame the prefixes
                // are defined in. Matching the path on disk instead let
                // every component of the checkout prefix take part, so a
                // tree under `/home/go/…` or `/srv/c/…` classified its
                // own templates as another backend's and wrote an empty
                // depfile. `codegen_depfile_path_independence` holds the
                // two frames apart by rendering from a relocated tree.
                //
                // Component-wise within that name, not substring: a
                // directory named `go_helpers` must not read as `go`,
                // and `forge/rust/` is as foreign to C++ as `rust/` is —
                // a plain prefix test would keep the whole `forge/`
                // half of the other five backends.
                !name.split('/').any(|c| foreign.contains(&c))
            })
            .map(|(_, path)| path),
    );

    // Add sce-codegen binary as a dependency (rebuilds if binary itself changes,
    // which covers all Rust source changes). More precise than listing all .rs files.
    let exe_path = std::env::current_exe().ok();
    if let Some(ref exe) = exe_path {
        if exe.exists() {
            deps.push(exe.clone());
        }
    }

    // Drop anything this invocation wrote. Its own artefacts as well as
    // the side-effect writes: both are outputs of this build edge, and
    // an edge that lists its own output as a prerequisite is a cycle,
    // not a dependency.
    let produced: std::collections::HashSet<PathBuf> = output_paths
        .iter()
        .chain(self_written.iter())
        .map(|p| depfile_identity(p))
        .collect();
    deps.retain(|p| !produced.contains(&depfile_identity(p)));

    // Collapse duplicates while preserving first-seen order. The same
    // canonical path can land in `deps` more than once (e.g. a fragment
    // pulled in via both `<xi:include>` and `<sce:use>` — pipeline-level
    // de-dup is the sink's responsibility per parser comment).
    let mut seen = std::collections::HashSet::new();
    deps.retain(|p| seen.insert(p.clone()));

    if !output_paths.is_empty() {
        // List all outputs as targets (e.g., C++ produces both .h and .inl)
        let targets: String = output_paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let mut content = format!("{targets}: ");
        for (i, dep) in deps.iter().enumerate() {
            if i > 0 {
                content.push_str(" \\\n    ");
            }
            content.push_str(&dep.display().to_string());
        }
        content.push('\n');
        fs::write(depfile_path, content).unwrap_or_else(|e| {
            cli_exit(CliError::WriteOutput {
                path: depfile_path.to_string(),
                source: e,
            })
        });
    }
}

// ── Subcommand: generate-w3c ────────────────────────────────────

/// Test info parsed from CMakeLists.txt
#[derive(Debug, Clone)]
struct TestInfo {
    test_type: String,
    comment: String,
}

/// Metadata from resources/XXX/metadata.txt
#[derive(Debug, Default)]
struct TestMetadata {
    description: String,
    specnum: String,
}

/// The fixed per-test codegen surface passed to `generate_test_file` — a
/// parameter object grouping the test's id, stems, target state, feature
/// flags, and metadata so the trait method takes one argument instead of
/// eight (replacing the `too_many_arguments` allow).
#[derive(Clone, Copy)]
struct TestFileSpec<'a> {
    test_id: &'a str,
    input_stem: &'a str,
    machine_name: &'a str,
    pass_state: &'a str,
    needs_script: bool,
    uses_http: bool,
    test_type: &'a str,
    metadata: &'a TestMetadata,
}

// CLI command dispatcher for `generate-w3c`. Takes the parsed
// `GenerateW3cArgs` and unpacks it into the borrowed names the body uses.
fn cmd_generate_w3c(args: GenerateW3cArgs) {
    let GenerateW3cArgs {
        language,
        registry,
        resources,
        test,
        clean,
        list,
        format_style,
        no_format,
    } = args;
    let language: &str = &language;
    let registry = registry.as_deref();
    let resources = resources.as_deref();
    let single_test = test.as_deref();
    let format_style = format_style.as_deref();

    let lang: Language = language.parse().unwrap_or_else(|_| {
        cli_exit(CliError::UnknownLanguage {
            lang: language.to_string(),
        })
    });

    // Resolve project root
    let project_root = find_project_root();
    let resources_dir = resources
        .map(PathBuf::from)
        .unwrap_or_else(|| project_root.join("resources"));
    let cmake_file = registry
        .map(PathBuf::from)
        .unwrap_or_else(|| project_root.join("tests/CMakeLists.txt"));

    let backend: Box<dyn W3cBackend> = match lang {
        Language::Rust => Box::new(RustBackend::new(&project_root)),
        Language::Go => Box::new(GoBackend::new(&project_root)),
        Language::Kotlin => Box::new(KotlinBackend::new(&project_root)),
        Language::Cpp => Box::new(CppBackend::new(&project_root)),
        Language::Python => Box::new(PythonBackend::new(&project_root)),
        Language::C11 => cli_exit(CliError::UnsupportedLanguage {
            lang: "C11 W3C statechart emitter (RFC §5.J.1)".into(),
        }),
    };

    // C++ formatter: created once and reused for all generated tests.
    let cpp_formatter = create_cpp_formatter(lang, format_style, no_format);

    generate_w3c_unified(
        backend.as_ref(),
        &resources_dir,
        &cmake_file,
        single_test,
        clean,
        list,
        &cpp_formatter,
    );
}

fn find_project_root() -> PathBuf {
    // Try CARGO_MANIFEST_DIR ancestor, then CWD
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = crate_dir.join("..");
    let root = if candidate.join("tests/CMakeLists.txt").exists() {
        fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.to_path_buf())
    } else {
        let cwd = std::env::current_dir().expect("Cannot get CWD");
        if !cwd.join("tests/CMakeLists.txt").exists() {
            cli_exit(CliError::ProjectRootNotFound);
        }
        cwd
    };
    // Every in-repo batch command reaches its inputs through this root and
    // records `// From:` provenance relative to it — that is what the
    // committed trees carry, and the only spelling that stays meaningful
    // on a machine that never ran the generator. Installing it here means
    // a future batch entry point inherits the convention rather than
    // having to remember it. `main` installs an explicit `--source-root`
    // before any subcommand runs, so the flag still wins.
    let _ = SOURCE_ROOT.set(root.clone());
    root
}

/// Parse test registrations from CMakeLists.txt.
fn parse_cmake_tests(cmake_file: &Path) -> BTreeMap<String, TestInfo> {
    let content = fs::read_to_string(cmake_file).unwrap_or_else(|e| {
        cli_exit(CliError::ReadInput {
            path: cmake_file.display().to_string(),
            source: e,
        })
    });

    let re = regex::Regex::new(
        r"sce_generate_static_w3c_test\((\S+)\s+\$\{STATIC_W3C_OUTPUT_DIR\}(?:\s+TYPE\s+(\w+))?\)\s*#\s*(.*)"
    ).unwrap();

    let mut tests = BTreeMap::new();
    for line in content.lines() {
        if let Some(caps) = re.captures(line) {
            let test_id = caps[1].to_string();
            let test_type = caps
                .get(2)
                .map(|m| m.as_str())
                .unwrap_or("SIMPLE")
                .to_string();
            let comment = caps[3].trim().to_string();
            tests.insert(test_id, TestInfo { test_type, comment });
        }
    }
    tests
}

/// Read metadata.txt for a test.
fn read_metadata(resources_dir: &Path, test_id: &str) -> TestMetadata {
    let num_prefix = extract_num_prefix(test_id);
    let meta_file = resources_dir.join(&num_prefix).join("metadata.txt");
    if !meta_file.exists() {
        return TestMetadata::default();
    }

    let content = fs::read_to_string(&meta_file).unwrap_or_default();
    let mut metadata = TestMetadata::default();
    for line in content.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            match key.trim() {
                "description" => metadata.description = value.trim().to_string(),
                "specnum" => metadata.specnum = value.trim().to_string(),
                _ => {}
            }
        }
    }
    metadata
}

/// Find the SCXML file for a test ID.
fn find_scxml(resources_dir: &Path, test_id: &str) -> Option<PathBuf> {
    let num_prefix = extract_num_prefix(test_id);
    let scxml = resources_dir
        .join(&num_prefix)
        .join(format!("test{test_id}.scxml"));
    if scxml.exists() {
        Some(scxml)
    } else {
        None
    }
}

/// Extract numeric prefix from test ID (e.g., "144" -> "144", "553b" -> "553")
fn extract_num_prefix(test_id: &str) -> String {
    test_id.chars().take_while(|c| c.is_ascii_digit()).collect()
}

/// Convert to PascalCase (matches Python to_pascal_case and Rust filters::to_pascal_case)
fn to_pascal_case(name: &str) -> String {
    filters::to_pascal_case(name.to_string())
}

/// Detect pass state from analyzed model.
fn detect_pass_state(model: &SCXMLModel) -> Option<String> {
    let final_states: Vec<&str> = model
        .states
        .iter()
        .filter(|(_, s)| s.is_final)
        .map(|(id, _)| id.as_str())
        .collect();

    // Priority: pass > final > single non-fail final state
    for state in &final_states {
        if state.eq_ignore_ascii_case("pass") {
            return Some("Pass".to_string());
        }
    }
    for state in &final_states {
        if state.eq_ignore_ascii_case("final") {
            return Some("Final".to_string());
        }
    }
    let non_fail: Vec<_> = final_states
        .iter()
        .filter(|s| !s.eq_ignore_ascii_case("fail"))
        .collect();
    if non_fail.len() == 1 {
        return Some(to_pascal_case(non_fail[0]));
    }
    None
}

/// Check if model uses BasicHTTP send (performHttpSend in generated code).
fn model_uses_http_send(model: &SCXMLModel) -> bool {
    for state in model.states.values() {
        for trans in &state.transitions {
            if action_uses_http_send(&trans.actions) {
                return true;
            }
        }
        for block in state
            .on_entry_blocks
            .iter()
            .chain(state.on_exit_blocks.iter())
        {
            if action_uses_http_send(block) {
                return true;
            }
        }
    }
    false
}

fn action_uses_http_send(actions: &[sce_build::model::Action]) -> bool {
    for action in actions {
        if action.action_type == "send" && action.send_type.contains("BasicHTTPEventProcessor") {
            return true;
        }
        if action_uses_http_send(&action.then_actions)
            || action_uses_http_send(&action.else_actions)
            || action_uses_http_send(&action.actions)
        {
            return true;
        }
        for branch in &action.elseif_branches {
            if action_uses_http_send(&branch.actions) {
                return true;
            }
        }
    }
    false
}

// ── W3cBackend Trait ───────────────────────────────────────────

/// Trait abstracting language-specific W3C test generation behavior.
/// Each backend implements language-specific SM generation, test file creation,
/// child processing, and cleanup logic.
trait W3cBackend {
    /// Human-readable language name for summary output.
    fn language_name(&self) -> &str;

    /// Base directory for generated state machine code.
    fn sm_output_base(&self) -> &Path;

    /// Directory for generated test files (may differ from sm_output_base).
    fn test_output_dir(&self) -> &Path;

    /// Generate SM code. Returns Vec of (filename, code) pairs.
    /// C++ returns .h + .inl, others return one file.
    /// Produce (filename, code) pairs for the state machine. Errors are
    /// returned as `ForgeError` so the W3C batch loop can drive them
    /// through `ToDiagnostics::to_diagnostics()` — preserving the
    /// structured `code` / `stage` / `fix` / `location` signal all the
    /// way to NDJSON output. Stringifying here would collapse every
    /// failure into `cli/scxml-generate` and lose the repair routing.
    fn generate_sm(
        &self,
        model: &SCXMLModel,
        input_stem: &str,
    ) -> Result<Vec<(String, String)>, ForgeError>;

    /// Hook after writing parent SM (e.g. Rust writes mod.rs).
    fn post_write_parent(
        &self,
        _test_id: &str,
        _test_mod_dir: &Path,
        _input_stem: &str,
        _drift_ctx: &DriftContext,
    ) {
    }

    /// Process a successfully generated child SM: fix package, register module, etc.
    fn process_child(
        &self,
        test_id: &str,
        child_name: &str,
        code: String,
        test_mod_dir: &Path,
        drift_ctx: &DriftContext,
    );

    /// Handle a child that failed codegen (Kotlin generates stubs, others skip).
    fn process_child_failure(
        &self,
        _test_id: &str,
        _child_name: &str,
        _test_mod_dir: &Path,
        _drift_ctx: &DriftContext,
    ) {
    }

    /// Generate test file content. Default returns empty (C++ uses CMake-managed test headers).
    // Trait method shared by every backend; the per-test codegen surface
    // is grouped into `TestFileSpec`. Default is a no-op — backends that
    // emit test files override it.
    fn generate_test_file(&self, _spec: &TestFileSpec) -> String {
        String::new()
    }

    /// Test filename (relative to test_output_dir or test_mod_dir).
    /// Default returns empty (backends that generate test files must override).
    fn test_filename(&self, _test_id: &str, _input_stem: &str) -> String {
        String::new()
    }

    /// Test file lives in test_mod_dir (Go) vs test_output_dir (Rust, Kotlin).
    fn test_in_sm_dir(&self) -> bool {
        false
    }

    /// Whether this backend writes SM files into per-test subdirectories (testNNN/).
    /// C++ writes to a flat output directory; others use subdirectories.
    fn uses_per_test_subdirs(&self) -> bool {
        true
    }

    /// §scxml-6.4: Whether this backend's parent template constructs a
    /// generated child class for hybrid (`srcexpr` / `contentexpr`)
    /// invokes. Rust / Go / C++ instantiate the stub by name
    /// (`Test{N}Hybrid{M}Policy` etc.), so the child SM must be emitted.
    /// Kotlin resolves hybrid invokes through `ScxmlRuntimeInterpreter`
    /// at runtime and never imports the generated class, so emitting
    /// the stub would be dead code — Kotlin overrides to `false`.
    /// Static `src=` / inline `<content>` invokes always get a stub
    /// because every backend's template references the child class
    /// by name and there is no runtime fallback.
    fn emits_hybrid_child_stub(&self) -> bool {
        true
    }

    /// Whether this backend generates test files alongside SM code.
    /// C++ test headers are managed by CMake, not by sce-codegen.
    fn generates_test_files(&self) -> bool {
        true
    }

    /// Called after main loop to write module indices (Rust writes root mod.rs).
    fn finalize(&self, _generated_ids: &[String], _drift_ctx: &DriftContext) {}

    /// Clean all generated files.
    fn clean(&self);

    /// Clean stale generated files for tests no longer in registry.
    /// Returns number of stale entries removed. Default is no-op.
    fn clean_stale(&self, _valid_ids: &BTreeSet<String>) -> usize {
        0
    }
}

// ── Shared Utilities for W3C Generation ────────────────────────

/// Unified child SM generation for all backends.
///
/// Enumerates children directly from the parent model's invoke lists
/// (`iter_scxml_invokes` + `iter_hybrid_invokes`). The parser
/// authoritatively populates `child_name` for both kinds: static invokes
/// resolve to the inline-extracted or `src=`-derived stem (parser §6.4),
/// and hybrid invokes synthesize `{model.name}_hybrid{idx}` to match the
/// stub written by `generate_hybrid_child_scxmls`. Static children live
/// in the parent's source directory; hybrid stubs live in the
/// per-test output directory. No filesystem scanning, no parent-code
/// substring heuristics, and no naming-convention filtering — the
/// backend trait carries no child-discovery hooks anymore.
fn generate_child_sms(
    backend: &dyn W3cBackend,
    test_id: &str,
    model: &SCXMLModel,
    scxml_path: &Path,
    test_mod_dir: &Path,
    drift_ctx: &DriftContext,
) {
    let resource_dir = scxml_path.parent().unwrap_or(Path::new("."));
    let mut seen = BTreeSet::new();

    // ChildSource captures where the child SCXMLModel comes from: an
    // inline `<content>` parser kept the model in-memory on the parent
    // invoke (no disk read needed), an external `src="…"` child must be
    // re-parsed from `resource_dir`, or a hybrid stub from `test_mod_dir`.
    // Replaces the prior unconditional disk read which was a leftover
    // from the parser-writes-synth-child era.
    enum ChildSource<'a> {
        Inline(&'a SCXMLModel),
        Disk(&'a Path),
    }

    let scxml_children: Vec<(&str, ChildSource<'_>)> = model
        .iter_scxml_invokes()
        .map(|inv| {
            let source = match inv.inline_child.as_deref() {
                Some(m) => ChildSource::Inline(m),
                None => ChildSource::Disk(resource_dir),
            };
            (inv.child_name.as_str(), source)
        })
        .collect();
    let hybrid_children: Vec<(&str, ChildSource<'_>)> = if backend.emits_hybrid_child_stub() {
        model
            .iter_hybrid_invokes()
            .map(|inv| (inv.child_name.as_str(), ChildSource::Disk(test_mod_dir)))
            .collect()
    } else {
        Vec::new()
    };

    for (child_name, child_source) in scxml_children.into_iter().chain(hybrid_children) {
        if child_name.is_empty() || !seen.insert(child_name.to_string()) {
            continue;
        }

        // Resolve the child model + the diagnostic label used by the
        // analyzer / source-path resolver. Inline children synthesize an
        // "as if on disk" path inside `resource_dir` so
        // `analyzer::compute_scxml_base_path` derives the same base_path
        // the on-disk-synth era did (the file itself does not exist;
        // `resolve_source_path` is skipped below — it would fail on
        // canonicalize). Disk children use the absolute path.
        let (mut child_model, label, child_path_for_resolver) = match child_source {
            ChildSource::Inline(m) => {
                let synthetic = resource_dir.join(format!("{child_name}.scxml"));
                (m.clone(), synthetic.to_string_lossy().into_owned(), None)
            }
            ChildSource::Disk(source_dir) => {
                let child_path = source_dir.join(format!("{child_name}.scxml"));
                if !child_path.exists() {
                    backend.process_child_failure(test_id, child_name, test_mod_dir, drift_ctx);
                    continue;
                }
                let child_str = child_path.to_string_lossy().into_owned();
                match SCXMLParser::new().parse_file(&child_str) {
                    Ok(parsed) => (parsed, child_str, Some(child_path)),
                    Err(_) => {
                        backend.process_child_failure(test_id, child_name, test_mod_dir, drift_ctx);
                        continue;
                    }
                }
            }
        };

        analyzer::analyze(&mut child_model, &label);

        if analyzer::can_generate_static(&child_model).is_err() {
            backend.process_child_failure(test_id, child_name, test_mod_dir, drift_ctx);
            continue;
        }

        if let Some(child_path) = child_path_for_resolver.as_deref() {
            resolve_source_path(&mut child_model, child_path);
        }

        // Templates derive generated PascalCase symbols from
        // `model.name`; `generate_hybrid_child_scxmls` writes the
        // synthesized `<scxml name="test{NUM}_hybrid{idx}">` so the
        // parser already produces the matching name. The guard
        // below is a defensive no-op for hybrid stubs and still
        // covers static children whose source SCXML carries a
        // different `<scxml name=...>` than the invoke `src=` stem.
        if child_model.name != child_name {
            child_model.name = child_name.to_string();
        }

        match backend.generate_sm(&child_model, child_name) {
            Ok(files) => {
                if let Some((_, code)) = files.into_iter().next() {
                    backend.process_child(test_id, child_name, code, test_mod_dir, drift_ctx);
                }
            }
            Err(_) => {
                backend.process_child_failure(test_id, child_name, test_mod_dir, drift_ctx);
            }
        }
    }
}

/// The single unified W3C test generation loop shared by all backends.
fn generate_w3c_unified(
    backend: &dyn W3cBackend,
    resources_dir: &Path,
    cmake_file: &Path,
    single_test: Option<&str>,
    clean: bool,
    list: bool,
    cpp_formatter: &Option<sce_build::formatter::CppFormatter>,
) {
    if clean {
        backend.clean();
        return;
    }

    // Spec §synth-6.2.6 drift context — input root is the W3C resources
    // tree; one hash pair covers every emitted parent SM + child SM
    // + test harness across all 202 tests in this invocation.
    let drift_ctx = DriftContext::compute(resources_dir, None, None);

    let cmake_tests = parse_cmake_tests(cmake_file);
    println!("C++ test registry: {} tests", cmake_tests.len());

    if list {
        for (tid, info) in &cmake_tests {
            let scxml = find_scxml(resources_dir, tid);
            let status = if scxml.is_some() { "OK" } else { "MISSING" };
            let comment_trunc: String = info.comment.chars().take(70).collect();
            println!(
                "  {tid:6} [{:9}] {status} -- {comment_trunc}",
                info.type_str()
            );
        }
        return;
    }

    let test_ids: Vec<String> = if let Some(tid) = single_test {
        vec![tid.to_string()]
    } else {
        let mut ids: Vec<String> = cmake_tests.keys().cloned().collect();
        ids.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
        ids
    };

    let mut generated_static = Vec::new();
    let mut generated_script = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    for test_id in &test_ids {
        let scxml_path = match find_scxml(resources_dir, test_id) {
            Some(p) => p,
            None => {
                skipped.push((test_id.clone(), "SCXML not found".to_string()));
                continue;
            }
        };

        let mut parser = SCXMLParser::new();
        let scxml_str = scxml_path.to_str().unwrap_or("");
        match parser.parse_file(scxml_str) {
            Ok(mut model) => {
                analyzer::analyze(&mut model, scxml_str);

                // §scxml-5.8: document_rejected models have initial->pass already
                // redirected by the parser, so they CAN be generated. Only skip
                // truly dynamic models.
                if analyzer::can_generate_static(&model).is_err() && !model.document_rejected {
                    skipped.push((test_id.clone(), "dynamic features".to_string()));
                    continue;
                }

                resolve_source_path(&mut model, &scxml_path);

                let input_stem = scxml_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");

                match backend.generate_sm(&model, input_stem) {
                    Ok(files) => {
                        // Format C++ output before writing to disk
                        let files = maybe_format_files(files, cpp_formatter);

                        // Determine write directory: per-test subdir or flat output dir
                        let test_mod_dir = if backend.uses_per_test_subdirs() {
                            backend.sm_output_base().join(format!("test{test_id}"))
                        } else {
                            backend.sm_output_base().to_path_buf()
                        };
                        fs::create_dir_all(&test_mod_dir).unwrap_or_else(|e| {
                            cli_exit(CliError::CreateOutputDir {
                                path: test_mod_dir.display().to_string(),
                                source: e,
                            })
                        });

                        // Write SM files
                        for (filename, code) in &files {
                            let file_path = test_mod_dir.join(filename);
                            write_if_changed_drift_aware(&file_path, code, &drift_ctx);
                        }

                        // Post-write hook (e.g. Rust writes initial mod.rs)
                        backend.post_write_parent(test_id, &test_mod_dir, input_stem, &drift_ctx);

                        // SCE Protocol-Synthesis RFC §synth-5-O — sourcemap
                        // JSON sidecar. Byte-identical across backends
                        // for the same SCXML input.
                        emit_sourcemap_for_machine(&model, &test_mod_dir, &drift_ctx);

                        // §scxml-6.4: Generate hybrid SCXML stubs + child state machines
                        // (only for backends that use per-test subdirs; C++ handles children via CMake)
                        if backend.uses_per_test_subdirs() {
                            generate_hybrid_child_scxmls(&model, &test_mod_dir);
                            generate_child_sms(
                                backend,
                                test_id,
                                &model,
                                &scxml_path,
                                &test_mod_dir,
                                &drift_ctx,
                            );
                        }

                        // Detect pass state and generate test file (if backend supports it)
                        if backend.generates_test_files() {
                            let pass_state = detect_pass_state(&model);
                            if let Some(ref pass) = pass_state {
                                let machine = to_pascal_case(input_stem);
                                let metadata = read_metadata(resources_dir, test_id);
                                let test_type = cmake_tests
                                    .get(test_id.as_str())
                                    .map(|i| i.test_type.as_str())
                                    .unwrap_or("SIMPLE");
                                let uses_http = model_uses_http_send(&model);
                                let needs_script = model.needs_script_engine;

                                let test_code = backend.generate_test_file(&TestFileSpec {
                                    test_id,
                                    input_stem,
                                    machine_name: &machine,
                                    pass_state: pass,
                                    needs_script,
                                    uses_http,
                                    test_type,
                                    metadata: &metadata,
                                });
                                let test_filename = backend.test_filename(test_id, input_stem);
                                let test_file_dir = if backend.test_in_sm_dir() {
                                    &test_mod_dir
                                } else {
                                    backend.test_output_dir()
                                };
                                let test_file = test_file_dir.join(&test_filename);
                                fs::create_dir_all(test_file_dir).ok();
                                write_if_changed_drift_aware(&test_file, &test_code, &drift_ctx);

                                if needs_script {
                                    generated_script.push(test_id.clone());
                                } else {
                                    generated_static.push(test_id.clone());
                                }
                            } else {
                                // Pass-state detection is a batch
                                // post-condition, not a compiler
                                // pipeline stage, so it emits inline
                                // as CliError::ScxmlGenerate rather
                                // than through the shared typed
                                // emitter (which only carries
                                // ForgeError codes).
                                let reason = "pass state not detected";
                                if matches!(current_error_format(), ErrorFormat::Json) {
                                    let cli_err = CliError::ScxmlGenerate {
                                        stage: "pass-state-detection",
                                        detail: format!("{test_id}: {reason}"),
                                    };
                                    for diag in cli_err.to_diagnostics() {
                                        emit_ndjson(&diag);
                                    }
                                }
                                failed.push((test_id.clone(), reason.to_string()));
                            }
                        } else {
                            // Backend doesn't generate test files (C++); count as generated
                            if model.needs_script_engine {
                                generated_script.push(test_id.clone());
                            } else {
                                generated_static.push(test_id.clone());
                            }
                        }
                    }
                    Err(e) => {
                        // Codegen produces bare `ForgeError` (minijinja
                        // unwinds without row/col), so wrap at the call
                        // site with the scxml path as the file label
                        // and leave line/col None — fabricating `(1,1)`
                        // would mislead the repair loop.
                        let located = sce_build::forge::error::Located::new(
                            e,
                            scxml_path.display().to_string(),
                            None,
                            None,
                        );
                        let reason = format!("codegen failed: {}", located.error);
                        emit_batch_failure_ndjson(&located);
                        failed.push((test_id.clone(), reason));
                    }
                }
            }
            Err(e) => {
                // Parser already populated `Located` with file +
                // (when available) row/col, so emit straight through.
                let reason = format!("parse error: {e}");
                emit_batch_failure_ndjson(&e);
                failed.push((test_id.clone(), reason));
            }
        }
    }

    // Clean stale files (backends that override clean_stale, e.g. Kotlin)
    let mut stale_removed = 0;
    if single_test.is_none() {
        let valid_ids: BTreeSet<String> = generated_static
            .iter()
            .chain(generated_script.iter())
            .cloned()
            .collect();
        if !valid_ids.is_empty() {
            stale_removed = backend.clean_stale(&valid_ids);
        }
    }

    // Finalize (Rust writes root mod.rs)
    if single_test.is_none() {
        let mut all_ids: Vec<String> = generated_static
            .iter()
            .chain(generated_script.iter())
            .cloned()
            .collect();
        all_ids.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
        backend.finalize(&all_ids, &drift_ctx);
    }

    // Summary
    let total_generated = generated_static.len() + generated_script.len();
    println!("\n{}", "=".repeat(60));
    println!("{} W3C Test Generation Summary", backend.language_name());
    println!("{}", "=".repeat(60));
    println!("  Generated (pure static):    {}", generated_static.len());
    println!("  Generated (script engine):  {}", generated_script.len());
    println!("  Generated (total):          {total_generated}");
    println!("  Skipped:                    {}", skipped.len());
    println!("  Failed:                     {}", failed.len());
    if stale_removed > 0 {
        println!("  Stale removed:              {stale_removed}");
    }
    println!("  Total:                      {}", test_ids.len());

    if !skipped.is_empty() {
        println!("\nSkipped:");
        for (tid, reason) in &skipped {
            println!("  {tid}: {reason}");
        }
    }

    if !failed.is_empty() {
        println!("\nFailed tests:");
        for (tid, reason) in &failed {
            println!("  {tid}: {reason}");
        }
    }

    if total_generated > 0 {
        println!(
            "\nGenerated SM classes: {}",
            backend.sm_output_base().display()
        );
        if !backend.test_in_sm_dir() {
            println!(
                "Generated test classes: {}",
                backend.test_output_dir().display()
            );
        }
        println!(
            "\nGenerated test IDs (static): {}",
            generated_static.join(" ")
        );
        if !generated_script.is_empty() {
            println!(
                "Generated test IDs (script): {}",
                generated_script.join(" ")
            );
        }
    }

    if !failed.is_empty() {
        // Per-test failures are already printed above in human mode.
        // In JSON mode we still emit a single structured summary so
        // agents see a record (not just a silent non-zero exit) and
        // can dedup via the id — the failed list is part of the key.
        cli_exit(CliError::ScxmlGenerate {
            stage: "w3c-batch",
            detail: format!(
                "{} of {} tests failed: {}",
                failed.len(),
                test_ids.len(),
                failed
                    .iter()
                    .map(|(id, _)| id.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        });
    }

    // §synth-5-O ownership-boundary walker. Mirrors
    // the cmd_generate hook: every drift-headered file under either
    // the SM output base or the per-test harness directory must
    // carry an `SCE-MAP:` marker. Non-drift-headered files (external
    // meta-generator output, hand-authored sources) are silently
    // skipped per ARCHITECTURE.md "Traceability Ownership Boundary".
    for root in [backend.sm_output_base(), backend.test_output_dir()] {
        if let Err(err) = sce_build::forge::sourcemap::validate_emitted_files_have_markers(root) {
            current_error_format().emit_and_exit(&err, "");
        }
    }
}

// ── RustBackend ────────────────────────────────────────────────

struct RustBackend {
    sm_base: PathBuf,
    test_dir: PathBuf,
    tmpl_dir: PathBuf,
}

impl RustBackend {
    fn new(project_root: &Path) -> Self {
        let tests_crate = project_root.join("backends/rust/tests");
        Self {
            sm_base: tests_crate.join("src/generated"),
            test_dir: tests_crate.join("tests"),
            tmpl_dir: sce_build::find_template_dir_for(Language::Rust),
        }
    }
}

impl W3cBackend for RustBackend {
    fn language_name(&self) -> &str {
        "Rust"
    }
    fn sm_output_base(&self) -> &Path {
        &self.sm_base
    }
    fn test_output_dir(&self) -> &Path {
        &self.test_dir
    }

    fn generate_sm(
        &self,
        model: &SCXMLModel,
        input_stem: &str,
    ) -> Result<Vec<(String, String)>, ForgeError> {
        // W3C SCXML W3C-test runner always emits std-coupled code:
        // the 202-fixture AOT suite exercises std-backed engine paths
        // (HTTP, script engines, multi-thread Arc<Mutex> external
        // queue). `--no-std` is a CLI-only profile; W3C
        // fixtures stay byte-identical to the std emission.
        let code = sce_build::generator::generate(model, &self.tmpl_dir, false)?;
        Ok(vec![(format!("{input_stem}_sm.rs"), code)])
    }

    fn post_write_parent(
        &self,
        _test_id: &str,
        test_mod_dir: &Path,
        input_stem: &str,
        drift_ctx: &DriftContext,
    ) {
        // Suppressions live on the generated `*_sm.rs` itself (see
        // `state_machine.rs.jinja2` header comment); the parent mod.rs no
        // longer needs to redundantly wrap the declaration in `#[allow(...)]`.
        //
        // §synth-5-O traceability — `write_if_changed_drift_aware` prepends the
        // §synth-6.2.6 header, so the ownership-boundary walker requires this
        // file to carry at least one `SCE-MAP:` marker line. The mod.rs
        // is the entry point for the test's generated module; point the
        // marker at the source SCXML so addr2sce traces back to it.
        let mod_content = format!(
            "// GENERATED -- DO NOT EDIT (sce-codegen)\n\
             // SCE-MAP: {input_stem}.scxml:1\n\n\
             mod {input_stem}_sm;\n\
             pub use {input_stem}_sm::*;\n"
        );
        write_if_changed_drift_aware(&test_mod_dir.join("mod.rs"), &mod_content, drift_ctx);
    }

    fn process_child(
        &self,
        _test_id: &str,
        child_name: &str,
        code: String,
        test_mod_dir: &Path,
        drift_ctx: &DriftContext,
    ) {
        let child_sm_file = test_mod_dir.join(format!("{child_name}_sm.rs"));
        write_if_changed_drift_aware(&child_sm_file, &code, drift_ctx);

        // Add child module to the test's mod.rs. The existing file's
        // first 4 lines are the drift header from `post_write_parent`;
        // `write_if_changed_drift_aware`'s downstream
        // `prepend_or_replace_header` detects that banner and replaces
        // the 4-line block in place, so a plain string append below
        // the read content is safe — the headered output still has
        // exactly one §synth-6.2.6 header at the top.
        let mod_file = test_mod_dir.join("mod.rs");
        if let Ok(existing) = fs::read_to_string(&mod_file) {
            if !existing.contains(&format!("mod {child_name}_sm;")) {
                let addition = format!(
                    "mod {child_name}_sm;\n\
                     pub use {child_name}_sm::*;\n"
                );
                write_if_changed_drift_aware(
                    &mod_file,
                    &format!("{existing}{addition}"),
                    drift_ctx,
                );
            }
        }
    }

    fn generate_test_file(&self, spec: &TestFileSpec) -> String {
        let TestFileSpec {
            test_id,
            input_stem,
            machine_name,
            pass_state,
            needs_script,
            uses_http,
            test_type,
            ..
        } = *spec;
        let timeout_secs = if test_type == "SCHEDULED" || test_type == "HTTP" {
            5
        } else {
            3
        };
        // Engine DI parity: instantiate LuaEngine per-test and pass it
        // to `Policy::new(engine)` instead of registering a process-global singleton.
        let policy_ctor = if needs_script {
            "    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> = \
             std::sync::Arc::new(sce_rust_lua::LuaEngine::new());\n\
             \x20   {POLICY_BINDING} = sce_rust_tests::generated::test"
        } else {
            "    {POLICY_BINDING} = sce_rust_tests::generated::test"
        };
        let policy_args = if needs_script {
            "(script_engine)"
        } else {
            "()"
        };
        let is_http = test_type == "HTTP" && uses_http;
        let http_setup = if is_http {
            "    sce_rust_tests::harness::setup_http_test(&mut engine);\n"
        } else {
            ""
        };
        // §scxml-C-2-3: the harness owns the inbound listener, so it declares
        // the access URI the machine publishes in `_ioprocessors`. Documents
        // converted from the W3C corpus address their BasicHTTP sends through
        // that entry, so a machine that publishes nothing sends nowhere. The
        // declaration lands on the policy before the engine takes ownership,
        // because `_ioprocessors` is populated once at session setup.
        let policy_binding = if is_http && needs_script {
            "let mut policy"
        } else {
            "let policy"
        };
        let http_access_uri = if is_http && needs_script {
            "\x20   policy.set_basic_http_access_uri(sce_rust_tests::harness::HTTP_TEST_SERVER_URL);\n"
        } else {
            ""
        };
        let policy_ctor = policy_ctor.replace("{POLICY_BINDING}", policy_binding);
        let pass_variant = to_pascal_case(pass_state);

        format!(
            "// GENERATED -- DO NOT EDIT (sce-codegen)\n\
             // SCE-MAP: {input_stem}.scxml:1\n\
             use std::time::Duration;\n\
             \n\
             #[test]\n\
             fn test_{test_id}() {{\n\
             {policy_ctor}{test_id}::{machine_name}Policy::new{policy_args};\n\
             {http_access_uri}\
             \x20   let mut engine = sce_rust_runtime::Engine::new(policy);\n\
             {http_setup}\
             \x20   engine.initialize();\n\
             \x20   let completed = engine.run_until_completion(\n\
             \x20       Duration::from_secs({timeout_secs}),\n\
             \x20       Duration::from_millis(10),\n\
             \x20   );\n\
             \x20   assert!(completed, \"Test {test_id} timed out\");\n\
             \x20   assert_eq!(\n\
             \x20       engine.get_current_state(),\n\
             \x20       sce_rust_tests::generated::test{test_id}::{machine_name}State::{pass_variant},\n\
             \x20       \"Test {test_id} reached wrong final state\"\n\
             \x20   );\n\
             }}\n"
        )
    }

    fn test_filename(&self, test_id: &str, _input_stem: &str) -> String {
        format!("test_{test_id}.rs")
    }

    fn finalize(&self, generated_ids: &[String], drift_ctx: &DriftContext) {
        if generated_ids.is_empty() {
            return;
        }
        // §synth-5-O traceability — `write_if_changed_drift_aware` prepends the
        // §synth-6.2.6 header, so the ownership-boundary walker requires a
        // marker line. This aggregator mod.rs has no single source SCXML;
        // reference the first registered test as the index entry point
        // so addr2sce still maps back into the generated tree.
        //
        // Hand-curated non-W3C-IRP fixtures live under
        // `backends/rust/tests/src/integration/` with their own hand-authored
        // `mod.rs`, so this aggregator only owns the W3C suite — the
        // generated/ tree is "codegen output, full overwrite" and the
        // integration/ tree is "hand-authored mod.rs over codegen-
        // emitted bodies".
        let first_id = &generated_ids[0];
        let mut mod_lines = vec![
            "// GENERATED -- DO NOT EDIT (sce-codegen)".to_string(),
            format!("// SCE-MAP: test{first_id}.scxml:1"),
            format!(
                "//! Generated W3C SCXML conformance test state machines ({} tests).\n",
                generated_ids.len()
            ),
        ];
        for id in generated_ids {
            mod_lines.push(format!("pub mod test{id};"));
        }
        mod_lines.push(String::new());
        write_if_changed_drift_aware(
            &self.sm_base.join("mod.rs"),
            &mod_lines.join("\n"),
            drift_ctx,
        );
    }

    fn clean(&self) {
        // Remove generated dirs but not handcrafted files
        for entry in fs::read_dir(&self.sm_base).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir()
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("test"))
            {
                fs::remove_dir_all(&path).ok();
            }
        }
        for entry in fs::read_dir(&self.test_dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("test_") && name.ends_with(".rs") {
                    fs::remove_file(&path).ok();
                }
            }
        }
        println!("Cleaned Rust generated files");
    }
}

// ── GoBackend ──────────────────────────────────────────────────

struct GoBackend {
    sm_base: PathBuf,
    tmpl_dir: PathBuf,
}

impl GoBackend {
    fn new(project_root: &Path) -> Self {
        let tests_module = project_root.join("backends/go/tests");
        Self {
            sm_base: tests_module.join("generated"),
            tmpl_dir: sce_build::find_template_dir_for(Language::Go),
        }
    }
}

impl W3cBackend for GoBackend {
    fn language_name(&self) -> &str {
        "Go"
    }
    fn sm_output_base(&self) -> &Path {
        &self.sm_base
    }
    fn test_output_dir(&self) -> &Path {
        &self.sm_base
    } // Go tests live in sm dir

    fn generate_sm(
        &self,
        model: &SCXMLModel,
        input_stem: &str,
    ) -> Result<Vec<(String, String)>, ForgeError> {
        let code = sce_build::generator::generate_go(model, &self.tmpl_dir)?;
        Ok(vec![(format!("{input_stem}_sm.go"), code)])
    }

    fn process_child(
        &self,
        test_id: &str,
        child_name: &str,
        code: String,
        test_mod_dir: &Path,
        drift_ctx: &DriftContext,
    ) {
        let parent_package = format!("test{test_id}");
        let child_pkg = sce_build::filters::to_snake_case(child_name.to_string());
        let fixed_code = code.replace(
            &format!("package {child_pkg}"),
            &format!("package {parent_package}"),
        );
        let child_file = test_mod_dir.join(format!("{child_name}_sm.go"));
        write_if_changed_drift_aware(&child_file, &fixed_code, drift_ctx);
    }

    fn generate_test_file(&self, spec: &TestFileSpec) -> String {
        let TestFileSpec {
            test_id,
            input_stem,
            machine_name,
            pass_state,
            needs_script,
            uses_http,
            test_type,
            metadata,
        } = *spec;
        let is_http = test_type == "HTTP" && uses_http;
        let timeout = if test_type == "SCHEDULED" {
            "5 * time.Second"
        } else {
            "3 * time.Second"
        };

        let engine_setup = if needs_script {
            // Engine DI parity: each test owns its LuaEngine; the
            // process-global `RegisterLuaEngine` / `GetScriptEngine` singleton
            // pair was deleted in the step #6 cleanup.
            //
            // §scxml-C-2-3: for HTTP fixtures the harness owns the inbound
            // listener, so it declares the access URI the machine publishes in
            // _ioprocessors. The converted documents address their BasicHTTP
            // sends through that entry.
            let access_uri = if is_http {
                "\tpolicy.BasicHTTPAccessURI = scegotest.BasicHTTPAccessURI\n"
            } else {
                ""
            };
            format!(
                "\tpolicy := New{machine_name}Policy()\n\
                 \tpolicy.SessionID = sce.GenerateSessionID()\n\
                 \tpolicy.ScriptEngine = scegotest.NewLuaEngine()\n\
                 {access_uri}\
                 \tengine := sce.NewEngine[{machine_name}State, {machine_name}Event](&policy)"
            )
        } else {
            format!(
                "\tpolicy := New{machine_name}Policy()\n\
                 \tengine := sce.NewEngine[{machine_name}State, {machine_name}Event](&policy)"
            )
        };

        let http_setup = if is_http {
            "\n\tscegotest.SetupHTTPTest(engine)\n"
        } else {
            ""
        };

        format!(
            "// GENERATED -- DO NOT EDIT (sce-codegen)\n\
             // SCE-MAP: {input_stem}.scxml:1\n\
             // W3C SCXML {specnum}: {description}\n\
             package test{test_id}\n\
             \n\
             import (\n\
             \t\"testing\"\n\
             \t\"time\"\n\
             \n\
             \tsce \"github.com/newmassrael/sce-go-runtime\"\n\
             \tscegotest \"github.com/newmassrael/sce-go-tests/harness\"\n\
             )\n\
             \n\
             func TestW3C{test_id}(t *testing.T) {{\n\
             {engine_setup}{http_setup}\n\
             \tengine.Initialize()\n\
             \tcompleted := engine.RunUntilCompletion({timeout}, 10*time.Millisecond)\n\
             \tif !completed {{\n\
             \t\tt.Fatalf(\"Test {test_id} timed out\")\n\
             \t}}\n\
             \tscegotest.AssertFinalState(t, engine.GetCurrentState(), {machine_name}State{pass_state}, \"{test_id}\")\n\
             }}\n",
            specnum = metadata.specnum,
            description = metadata.description,
        )
    }

    fn test_filename(&self, _test_id: &str, input_stem: &str) -> String {
        format!("{input_stem}_test.go")
    }

    fn test_in_sm_dir(&self) -> bool {
        true
    }

    fn clean(&self) {
        if self.sm_base.exists() {
            fs::remove_dir_all(&self.sm_base).ok();
            println!("Cleaned: {}", self.sm_base.display());
        }
    }
}

// ── KotlinBackend ──────────────────────────────────────────────

struct KotlinBackend {
    sm_base: PathBuf,
    test_dir: PathBuf,
    tmpl_dir: PathBuf,
}

impl KotlinBackend {
    fn new(project_root: &Path) -> Self {
        let tests_module = project_root.join("backends/kotlin/tests");
        Self {
            sm_base: tests_module.join("src/main/kotlin/com/sce/generated"),
            test_dir: tests_module.join("src/test/kotlin/com/sce/w3c"),
            tmpl_dir: sce_build::find_template_dir_for(Language::Kotlin),
        }
    }
}

impl W3cBackend for KotlinBackend {
    fn language_name(&self) -> &str {
        "Kotlin"
    }
    fn sm_output_base(&self) -> &Path {
        &self.sm_base
    }
    fn test_output_dir(&self) -> &Path {
        &self.test_dir
    }

    fn generate_sm(
        &self,
        model: &SCXMLModel,
        input_stem: &str,
    ) -> Result<Vec<(String, String)>, ForgeError> {
        let code = sce_build::generator::generate_kotlin(model, &self.tmpl_dir, None)?;
        Ok(vec![(format!("{input_stem}Sm.kt"), code)])
    }

    fn process_child(
        &self,
        test_id: &str,
        child_name: &str,
        code: String,
        test_mod_dir: &Path,
        drift_ctx: &DriftContext,
    ) {
        let parent_package = format!("test{test_id}");
        let child_package = child_name.to_lowercase();
        let fixed_code = code.replace(
            &format!("package com.sce.generated.{child_package}"),
            &format!("package com.sce.generated.{parent_package}"),
        );
        let child_sm_file = test_mod_dir.join(format!("{child_name}Sm.kt"));
        write_if_changed_drift_aware(&child_sm_file, &fixed_code, drift_ctx);
    }

    /// Kotlin's parent template handles hybrid invokes via
    /// `ScxmlRuntimeInterpreter.fromFile/fromString` (see
    /// `entry_exit_actions.kt.jinja2` `inv.is_hybrid` branch) and never
    /// imports the generated `Test{N}Hybrid{M}StateMachine` class, so
    /// emitting that stub would be dead code. Static `src=` / inline
    /// `<content>` invokes still get a stub via the trait default
    /// because the parent template instantiates them by name.
    fn emits_hybrid_child_stub(&self) -> bool {
        false
    }

    fn process_child_failure(
        &self,
        test_id: &str,
        child_name: &str,
        test_mod_dir: &Path,
        drift_ctx: &DriftContext,
    ) {
        let parent_package = format!("test{test_id}");
        let child_class = to_pascal_case(child_name);
        let stub = format!(
            "// GENERATED STUB -- child codegen failed (no-op)\n\
             // SCE-MAP: {child_name}.scxml:1\n\
             package com.sce.generated.{parent_package}\n\n\
             import com.sce.runtime.*\n\n\
             sealed interface {child_class}State : State {{\n\
             \x20   data object Initial : {child_class}State\n\
             }}\n\
             sealed interface {child_class}Event : Event\n\n\
             class {child_class}StateMachine(\n\
             \x20   scriptEngine: ScxmlScriptEngine? = null\n\
             ) : StateMachineEngine<{child_class}State, {child_class}Event>(scriptEngine) {{\n\
             \x20   override val initialState = {child_class}State.Initial\n\
             \x20   override fun processEvent(state: {child_class}State, event: {child_class}Event) = TransitionResult.Ignored as TransitionResult<{child_class}State>\n\
             \x20   override fun onEntry(state: {child_class}State) {{}}\n\
             \x20   override fun onExit(state: {child_class}State) {{}}\n\
             \x20   override fun executeTransitionActions(source: {child_class}State, event: {child_class}Event?) {{}}\n\
             }}\n"
        );
        let child_sm_file = test_mod_dir.join(format!("{child_name}Sm.kt"));
        write_if_changed_drift_aware(&child_sm_file, &stub, drift_ctx);
    }

    fn generate_test_file(&self, spec: &TestFileSpec) -> String {
        let TestFileSpec {
            test_id,
            input_stem,
            pass_state,
            needs_script,
            uses_http,
            test_type,
            metadata,
            ..
        } = *spec;
        let sm_class = format!("Test{}", to_pascal_case(test_id));
        let sm_package = format!("test{test_id}");

        // §scxml-C-2: HTTP tests use W3CHttpTestBase only when SM actually uses performHttpSend()
        let is_http = test_type == "HTTP" && uses_http;
        let base_class = if is_http {
            "W3CHttpTestBase"
        } else {
            "W3CTestBase"
        };

        // §scxml-6.2: SCHEDULED tests need longer timeout
        let timeout_override = if test_type == "SCHEDULED" {
            "    override val timeoutMs: Long = 5000L\n"
        } else {
            ""
        };

        let create_sm = if needs_script {
            format!(
                "    override fun createStateMachine() = {sm_class}StateMachine(createEngine())\n"
            )
        } else {
            format!("    override fun createStateMachine() = {sm_class}StateMachine()\n")
        };

        format!(
            "// GENERATED -- DO NOT EDIT (sce-codegen)\n\
             // SCE-MAP: {input_stem}.scxml:1\n\
             package com.sce.w3c\n\
             \n\
             import com.sce.generated.{sm_package}.{sm_class}Event\n\
             import com.sce.generated.{sm_package}.{sm_class}State\n\
             import com.sce.generated.{sm_package}.{sm_class}StateMachine\n\
             import org.junit.jupiter.api.DisplayName\n\
             \n\
             // W3C SCXML {specnum}: {description}\n\
             @DisplayName(\"Test {test_id} -- W3C SCXML {specnum}\")\n\
             class {sm_class} : {base_class}<{sm_class}State, {sm_class}Event>() {{\n\
             {create_sm}\
             \x20   override val expectedPassState: {sm_class}State = {sm_class}State.{pass_state}\n\
             {timeout_override}\
             }}\n",
            specnum = metadata.specnum,
            description = metadata.description,
        )
    }

    fn test_filename(&self, test_id: &str, _input_stem: &str) -> String {
        format!("Test{}.kt", to_pascal_case(test_id))
    }

    fn clean(&self) {
        if self.sm_base.exists() {
            fs::remove_dir_all(&self.sm_base).ok();
            println!("Cleaned: {}", self.sm_base.display());
        }
        for entry in fs::read_dir(&self.test_dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("Test") && name.ends_with(".kt") {
                fs::remove_file(entry.path()).ok();
            }
        }
        println!("Cleaned test classes in: {}", self.test_dir.display());
    }

    fn clean_stale(&self, valid_ids: &BTreeSet<String>) -> usize {
        let mut removed = 0;

        // Clean stale SM directories
        if self.sm_base.exists() {
            for entry in fs::read_dir(&self.sm_base).into_iter().flatten().flatten() {
                if !entry.path().is_dir() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with("test") {
                    continue;
                }
                let dir_test_id = &name[4..];
                if !valid_ids.contains(dir_test_id) {
                    fs::remove_dir_all(entry.path()).ok();
                    println!("  Removed stale SM dir: {name}");
                    removed += 1;
                }
            }
        }

        // Clean stale test classes
        if self.test_dir.exists() {
            let valid_lower: BTreeSet<String> =
                valid_ids.iter().map(|s| s.to_lowercase()).collect();
            for entry in fs::read_dir(&self.test_dir).into_iter().flatten().flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with("Test") || !name.ends_with(".kt") {
                    continue;
                }
                if name == "W3CTestBase.kt" || name == "W3CHttpTestBase.kt" {
                    continue;
                }
                let stem = &name[..name.len() - 3];
                let file_test_id = &stem[4..];
                if !valid_ids.contains(file_test_id)
                    && !valid_lower.contains(&file_test_id.to_lowercase())
                {
                    fs::remove_file(entry.path()).ok();
                    println!("  Removed stale test: {name}");
                    removed += 1;
                }
            }
        }

        removed
    }
}

// ── CppBackend ─────────────────────────────────────────────────

struct CppBackend {
    output_dir: PathBuf,
    tmpl_dir: PathBuf,
}

impl CppBackend {
    fn new(project_root: &Path) -> Self {
        Self {
            output_dir: project_root.join("build/tests/w3c_static_generated"),
            tmpl_dir: sce_build::find_template_dir_for(Language::Cpp),
        }
    }
}

impl W3cBackend for CppBackend {
    fn language_name(&self) -> &str {
        "C++"
    }
    fn sm_output_base(&self) -> &Path {
        &self.output_dir
    }
    fn test_output_dir(&self) -> &Path {
        &self.output_dir
    }

    fn generate_sm(
        &self,
        model: &SCXMLModel,
        input_stem: &str,
    ) -> Result<Vec<(String, String)>, ForgeError> {
        let output = sce_build::generator::generate_cpp(model, &self.tmpl_dir, input_stem, None)?;
        Ok(output.files)
    }

    fn uses_per_test_subdirs(&self) -> bool {
        false
    }
    fn generates_test_files(&self) -> bool {
        false
    }

    fn process_child(
        &self,
        _test_id: &str,
        _child_name: &str,
        _code: String,
        _test_mod_dir: &Path,
        _drift_ctx: &DriftContext,
    ) {
        // C++ children are handled by CMake via _children.txt, not the W3C generator
    }

    fn clean(&self) {
        if self.output_dir.exists() {
            fs::remove_dir_all(&self.output_dir).ok();
            println!("Cleaned: {}", self.output_dir.display());
        }
    }
}

// ── PythonBackend ──────────────────────────────────────────────
//
// Emits AOT Python statechart modules through the existing
// `sce_build::generator::generate_python` pipeline.
// `<invoke>` is still reject-walled by `reject_python_unsupported_features`
// so any W3C fixture exercising invoke surfaces a clean InvalidConfig
// at this layer and is reported as a generation failure rather than a
// silent skip. Tests that pass the codegen filter land at
// `backends/python/tests/generated/test{id}/test{id}_sm.py` and can be
// driven by an external pytest harness. SCE emits the tests, not the
// runner: a wrapper would pin one invocation style for a backend
// whose users already have one.

struct PythonBackend {
    sm_base: PathBuf,
    tmpl_dir: PathBuf,
}

impl PythonBackend {
    fn new(project_root: &Path) -> Self {
        Self {
            sm_base: project_root.join("backends/python/tests/generated"),
            tmpl_dir: sce_build::find_template_dir_for(Language::Python),
        }
    }
}

impl W3cBackend for PythonBackend {
    fn language_name(&self) -> &str {
        "Python"
    }
    fn sm_output_base(&self) -> &Path {
        &self.sm_base
    }
    fn test_output_dir(&self) -> &Path {
        &self.sm_base
    }

    fn generate_sm(
        &self,
        model: &SCXMLModel,
        input_stem: &str,
    ) -> Result<Vec<(String, String)>, ForgeError> {
        let code = sce_build::generator::generate_python(model, &self.tmpl_dir)
            .map_err(ForgeError::from)?;
        Ok(vec![(format!("{input_stem}_sm.py"), code)])
    }

    fn process_child(
        &self,
        _test_id: &str,
        child_name: &str,
        code: String,
        test_mod_dir: &Path,
        drift_ctx: &DriftContext,
    ) {
        let child_file = test_mod_dir.join(format!("{child_name}_sm.py"));
        write_if_changed_drift_aware(&child_file, &code, drift_ctx);
    }

    // The pytest wrapper lives alongside the generated `*_sm.py`
    // in `backends/python/tests/generated/test{N}/test_w3c_{N}.py`. The
    // wrapper imports the SM module by relative path (using sys.path
    // insertion at the test's own parent), instantiates an engine via
    // the generated `create_engine()` factory, drives time forward
    // until `reached_final` (matching the Go / Rust harness's
    // `RunUntilCompletion` contract), and asserts the final state is
    // the W3C `pass` final. SCHEDULED tests advance virtual time up
    // to 6 s in 50 ms ticks so 5 s `<send delay>` fixtures resolve;
    // SIMPLE tests reach final on the macrostep right after
    // initialize, so the time-advance loop completes in zero ticks.
    fn generates_test_files(&self) -> bool {
        true
    }

    fn test_in_sm_dir(&self) -> bool {
        true
    }

    fn generate_test_file(&self, spec: &TestFileSpec) -> String {
        let TestFileSpec {
            test_id,
            input_stem,
            pass_state,
            uses_http,
            test_type,
            metadata,
            ..
        } = *spec;
        // §scxml-6.2 — `<send delay="…">` fixtures arm scheduled
        // events the engine drains only via `advance_time(ms)`. We
        // advance in 50 ms ticks (the tightest of the spec's
        // canonical delays — 5 s timeouts split into 100 slots of
        // 50 ms each) so the loop never overshoots a fired event
        // by more than one tick; the SIMPLE path exits immediately
        // because `engine.initialize()` already drove every eventless
        // transition to a stable configuration.
        let (max_ms, tick_ms) = if test_type == "SCHEDULED" {
            (6_000_i64, 50_i64)
        } else {
            (50_i64, 50_i64)
        };
        // `detect_pass_state` returns the variant name in PascalCase
        // ("Pass" / "Final") for the Rust / Kotlin / Go backends that
        // dispatch on enum variants. Python's generated State enum
        // overrides `__str__` to return the SCXML `<final id="…">`
        // text verbatim (so `str(State.PASS)` → `"pass"`); the
        // wrapper compares on that string, so we lowercase here.
        let pass_literal = pass_state.to_ascii_lowercase();
        let pass_literal = pass_literal.as_str();
        // §scxml-C-2 — documents that use BasicHTTP transport take
        // the `setup_http` fixture from backends/python/tests/conftest.py,
        // which spawns the W3C echo server (port 8080) and registers
        // the HTTP dispatch callback on the engine. Non-HTTP fixtures
        // omit the parameter so the server only starts when actually
        // needed.
        let test_signature = if uses_http {
            "test_w3c_{test_id}(setup_http) -> None:".to_string()
        } else {
            "test_w3c_{test_id}() -> None:".to_string()
        };
        let test_signature = test_signature.replace("{test_id}", test_id);
        let setup_call = if uses_http {
            "    setup_http(engine)\n"
        } else {
            ""
        };
        format!(
            "# GENERATED -- DO NOT EDIT (sce-codegen)\n\
             # W3C SCXML {specnum}: {description}\n\
             # SCE-MAP: {input_stem}.scxml:1\n\
             \"\"\"pytest wrapper for W3C SCXML test {test_id} (Python AOT).\n\n\
             Imports the sibling `{input_stem}_sm.py` generated by\n\
             `sce-codegen generate-w3c --language python`, instantiates\n\
             an engine, and drives it until the W3C `<final id=\"pass\">`\n\
             is reached (or `<final id=\"fail\">` for negative fixtures).\n\
             \"\"\"\n\
             from __future__ import annotations\n\
             \n\
             import sys\n\
             from pathlib import Path\n\
             \n\
             _HERE = Path(__file__).resolve().parent\n\
             sys.path.insert(0, str(_HERE))\n\
             sys.path.insert(0, str(_HERE.parents[2] / \"runtime\"))\n\
             \n\
             import {input_stem}_sm as _sm  # noqa: E402 — path inserted above\n\
             \n\
             \n\
             def {test_signature}\n\
             \x20   engine = _sm.create_engine()\n\
             {setup_call}\
             \x20   engine.initialize()\n\
             \x20   elapsed = 0\n\
             \x20   while not engine.reached_final and elapsed < {max_ms}:\n\
             \x20       engine.advance_time({tick_ms})\n\
             \x20       elapsed += {tick_ms}\n\
             \x20   assert engine.reached_final, (\n\
             \x20       f\"test {test_id} did not reach a top-level <final> within {max_ms} ms; \"\n\
             \x20       f\"last leaf={{engine.current_state}}\"\n\
             \x20   )\n\
             \x20   actual = str(engine.current_state)\n\
             \x20   assert actual == \"{pass_literal}\", (\n\
             \x20       f\"test {test_id} reached <final id={{actual!r}}>; W3C expected \\\"{pass_literal}\\\"\"\n\
             \x20   )\n",
            specnum = metadata.specnum,
            description = metadata.description,
        )
    }

    fn test_filename(&self, test_id: &str, _input_stem: &str) -> String {
        format!("test_w3c_{test_id}.py")
    }

    fn clean(&self) {
        if self.sm_base.exists() {
            fs::remove_dir_all(&self.sm_base).ok();
            println!("Cleaned: {}", self.sm_base.display());
        }
    }
}

// ── Subcommand: fix-scxml-name ──────────────────────────────────

fn cmd_fix_scxml_name(scxml_path: &str, name: &str) {
    let content = fs::read_to_string(scxml_path).unwrap_or_else(|e| {
        cli_exit(CliError::ReadInput {
            path: scxml_path.to_string(),
            source: e,
        })
    });

    // Find first <scxml...> tag
    let re_scxml = regex::Regex::new(r"(?s)<scxml[^>]*?>").unwrap();
    let fixed = match re_scxml.find(&content) {
        Some(m) => {
            let first_scxml = m.as_str();
            // Remove existing name attribute
            let re_name = regex::Regex::new(r#"\s+name="[^"]*""#).unwrap();
            let cleaned = re_name.replace(first_scxml, "").to_string();
            // Add new name attribute
            let with_name = cleaned.replacen("<scxml", &format!("<scxml name=\"{name}\""), 1);
            format!(
                "{}{}{}",
                &content[..m.start()],
                with_name,
                &content[m.end()..]
            )
        }
        None => cli_exit(CliError::NoScxmlTag {
            path: scxml_path.to_string(),
        }),
    };

    fs::write(scxml_path, fixed).unwrap_or_else(|e| {
        cli_exit(CliError::WriteOutput {
            path: scxml_path.to_string(),
            source: e,
        })
    });
}

// ── Subcommand: read-metadata ───────────────────────────────────

fn cmd_read_metadata(metadata_file: &str) {
    let content = fs::read_to_string(metadata_file).unwrap_or_else(|e| {
        cli_exit(CliError::ReadInput {
            path: metadata_file.to_string(),
            source: e,
        })
    });

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("description:") {
            let description = line.split_once(':').map(|(_, v)| v.trim()).unwrap_or("");
            println!("{description}");
            return;
        }
    }

    cli_exit(CliError::MissingMetadataField {
        path: metadata_file.to_string(),
    });
}

// ── Subcommand: manifest ───────────────────────────────────────

fn cmd_manifest(dir: &str) {
    let dir_path = Path::new(dir);
    if !dir_path.is_dir() {
        cli_exit(CliError::NotADirectory {
            path: dir.to_string(),
        });
    }

    let manifest = sce_build::build_forge_manifest(dir_path)
        .unwrap_or_else(|e| current_error_format().emit_and_exit(&e, "Forge codegen error: "));

    let json = serde_json::to_string_pretty(&manifest).unwrap_or_else(|e| {
        cli_exit(CliError::JsonSerialization {
            detail: e.to_string(),
        })
    });

    println!("{json}");
}

// ── Subcommand: requirements ──────────────────────────────────
//
// Emit per-IR-node `sce:req`
// NDJSON. Routes through the same parser SCE uses for codegen so
// the report sees exactly the same node walk the build does —
// drift between "what compiles" and "what the report claims is
// annotated" is structurally impossible.

fn cmd_requirements(scxml: &str, error_format: ErrorFormat) {
    let mut parser = sce_build::parser::SCXMLParser::new();
    let model = parser
        .parse_file(scxml)
        .unwrap_or_else(|e| error_format.emit_and_exit(&e, "SCXML parse error: "));
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    sce_build::requirements_report::emit_requirements_ndjson(&model, &mut handle).unwrap_or_else(
        |source| {
            cli_exit(CliError::WriteOutput {
                path: "stdout".to_string(),
                source,
            })
        },
    );
}

// ── Subcommand: unresolved ─────────────────────────────────────
//
// Emit per-marker `<sce:unresolved>`
// NDJSON. Same architecture as the `requirements` subcommand — parse
// through the production parser, walk the model, emit one record per
// detected marker.

fn cmd_unresolved(scxml: &str, error_format: ErrorFormat) {
    let mut parser = sce_build::parser::SCXMLParser::new();
    let model = parser
        .parse_file(scxml)
        .unwrap_or_else(|e| error_format.emit_and_exit(&e, "SCXML parse error: "));
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    sce_build::unresolved_check::emit_unresolved_ndjson(&model, &mut handle).unwrap_or_else(
        |source| {
            cli_exit(CliError::WriteOutput {
                path: "stdout".to_string(),
                source,
            })
        },
    );
}

// ── Subcommand: generate-integration ───────────────────────────

/// Batch integration-fixture regeneration for the three committed-tree
/// backends (Rust / Kotlin / Go). Parallel to `generate-w3c` but
/// scoped to `integration_resources/<stem>/<stem>.scxml` fixtures.
///
/// Each `<stem>` is dispatched to the matching backend's regen script
/// (`scripts/regen_<stem>{,_kotlin,_go}.sh`); those scripts already
/// encode the per-language TMP staging + `--input-root` override +
/// post-processing (Rust `mod.rs` synthesis, Kotlin `// Source:`
/// rewrite, Kotlin `--kotlin-package-prefix com.sce.integration`).
/// Routing through a single CLI entry point keeps
/// `scripts/regen_all_committed_trees.sh` backend-agnostic.
///
/// Build-time backends (cpp / c11 / pybind11 Python) are intentionally
/// not supported here — they emit at CMake / CI time without a
/// committed tree, so there is nothing for `generate-integration` to
/// drive.
fn cmd_generate_integration(language: &str, stem: Option<&str>, error_format: ErrorFormat) {
    let lang: Language = language.parse().unwrap_or_else(|_| {
        error_format.emit_and_exit(
            &CliError::UnknownLanguage {
                lang: language.to_string(),
            },
            "",
        )
    });

    let script_suffix = match lang {
        Language::Rust => "",
        Language::Kotlin => "_kotlin",
        Language::Go => "_go",
        Language::Python => "_python",
        _ => {
            eprintln!(
                "generate-integration: only rust/kotlin/go/python are supported (cpp / c11 \
                 emit at CMake time without an sce-codegen-driven pipeline)"
            );
            std::process::exit(2);
        }
    };

    let project_root = find_project_root();
    let integration_root = project_root.join("integration_resources");

    let stems: Vec<String> = match stem {
        Some(s) => vec![s.to_string()],
        None => {
            let mut v = Vec::new();
            let entries = std::fs::read_dir(&integration_root).unwrap_or_else(|e| {
                eprintln!(
                    "generate-integration: cannot read {}: {e}",
                    integration_root.display()
                );
                std::process::exit(1);
            });
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    if let Some(name) = entry.file_name().to_str() {
                        v.push(name.to_string());
                    }
                }
            }
            v.sort();
            v
        }
    };

    for stem in &stems {
        let script = project_root.join(format!("scripts/regen_{stem}{script_suffix}.sh"));
        if !script.exists() {
            eprintln!(
                "generate-integration: missing regen script {}",
                script.display()
            );
            std::process::exit(1);
        }
        let status = std::process::Command::new("bash")
            .arg(&script)
            .current_dir(&project_root)
            .status()
            .unwrap_or_else(|e| {
                eprintln!(
                    "generate-integration: cannot spawn {}: {e}",
                    script.display()
                );
                std::process::exit(1);
            });
        if !status.success() {
            eprintln!(
                "generate-integration: {} failed (exit {})",
                script.display(),
                status.code().unwrap_or(-1)
            );
            std::process::exit(1);
        }
    }
}

// ── Subcommand: generate-conformance ───────────────────────────

fn cmd_generate_conformance(
    language: &str,
    manifest_path: &str,
    output_dir: &str,
    depfile_path: Option<&str>,
) {
    let lang: Language = language.parse().unwrap_or_else(|_| {
        cli_exit(CliError::UnknownLanguage {
            lang: language.to_string(),
        })
    });

    let manifest =
        sce_build::conformance::Manifest::load(Path::new(manifest_path)).unwrap_or_else(|e| {
            cli_exit(CliError::ReadInput {
                path: manifest_path.to_string(),
                source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
            })
        });

    let template_base = sce_build::find_template_base();
    let resource_dir = Path::new(manifest_path)
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("resources"))
        .unwrap_or_else(|| {
            cli_exit(CliError::ReadInput {
                path: manifest_path.to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "cannot derive resource_dir from manifest path",
                ),
            })
        });
    let rendered =
        sce_build::conformance::render_harness(&manifest, lang, &template_base, &resource_dir)
            .unwrap_or_else(|e| {
                cli_exit(CliError::ScxmlGenerate {
                    stage: "conformance",
                    detail: e.to_string(),
                })
            });

    let out_dir = Path::new(output_dir);
    fs::create_dir_all(out_dir).unwrap_or_else(|e| {
        cli_exit(CliError::CreateOutputDir {
            path: out_dir.display().to_string(),
            source: e,
        })
    });
    let out_path = out_dir.join(sce_build::conformance::harness_filename(lang));

    // Spec §synth-6.2.6: input root for the conformance harness is the
    // sibling `resources/` of the manifest (mirrors
    // `cmd_list_fixtures`'s resolution), so the embedded source-hash
    // covers exactly the SCXML inputs the harness asserts against.
    let drift_input_root: std::path::PathBuf = Path::new(manifest_path)
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("resources"))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let drift_ctx = DriftContext::compute(&drift_input_root, None, None);
    write_drift_aware(
        current_error_format(),
        &out_path,
        &rendered.source,
        &drift_ctx,
    );

    // Same depfile contract the statechart and forge routes carry. This
    // subcommand had none, so its two CMake steps declared their inputs
    // with `DEPENDS` plus a `file(GLOB ... CONFIGURE_DEPENDS)` over the
    // per-kind fragments — which names what the scaffold includes
    // directly and not what those fragments pull in, and named no
    // fixture document at all even though the harness asserts against
    // them and folds every one into its `source-hash`.
    //
    // The template scope is the one `render_harness` loads, taken from
    // the same `harness_layout` rather than spelled again here: a second
    // spelling is free to drift into declaring a directory the render
    // does not read, or missing the one it does.
    if let Some(dep_path) = depfile_path {
        let harness_template_dir =
            template_base.join(sce_build::conformance::harness_layout(lang).template_subdir);
        write_depfile(
            dep_path,
            DepfileInputs {
                output_paths: std::slice::from_ref(&out_path),
                template_dir: &harness_template_dir,
                lang,
                scxml_input: Path::new(manifest_path),
                preprocessor_deps: &rendered.extra_inputs,
                source_set: &drift_ctx.sources,
                self_written: &[],
            },
        );
    }
    println!("Generated conformance harness: {}", out_path.display());
}

// ── Subcommand: expand ─────────────────────────────────────────

fn cmd_expand(scxml_path: &str, include_dirs: &[String]) {
    let content = fs::read_to_string(scxml_path).unwrap_or_else(|e| {
        cli_exit(CliError::ReadInput {
            path: scxml_path.to_string(),
            source: e,
        })
    });
    let base_dir = Path::new(scxml_path).parent();
    let extra_dirs: Vec<PathBuf> = include_dirs.iter().map(PathBuf::from).collect();
    let (expanded, _map, _deps) =
        sce_build::parser::expand_preprocessors(&content, scxml_path, base_dir, &extra_dirs)
            .unwrap_or_else(|err| {
                current_error_format().emit_and_exit(&err, "Preprocessor error: ")
            });
    // Write raw bytes to stdout without trailing newline so the
    // template parity harness can byte-compare against the C++
    // pugixml canonicalisation without newline handling quirks.
    use std::io::Write;
    std::io::stdout()
        .write_all(expanded.as_bytes())
        .unwrap_or_else(|e| {
            cli_exit(CliError::WriteOutput {
                path: "<stdout>".to_string(),
                source: e,
            })
        });
}

// ── Subcommand: verify ─────────────────────────────────────────
//
// Spec §synth-6.2.6 generated-source drift detection. Recomputes
// `source-hash` + `template-hash` over the current source/template
// state and compares each generated file's embedded header against
// the recomputed values. First mismatch wins (deterministic ordering
// via BTreeMap-sorted file walk) and exits non-zero with
// `forge/source-hash-mismatch`.

fn cmd_verify(
    out_dir: &str,
    input_root: &str,
    deploy: Option<&str>,
    template_root: Option<&str>,
    cargo_lock: Option<&str>,
    error_format: ErrorFormat,
) {
    use sce_build::forge::drift::{
        compute_source_hash, compute_template_hash, parse_embedded_hashes, DriftHashes,
    };

    let out_path = Path::new(out_dir);
    let input_path = Path::new(input_root);
    let deploy_path = deploy.map(Path::new);

    // Defaults for template_root / cargo_lock: consult the same
    // resolution chain DriftContext uses (--workspace-root override
    // → SCE_WORKSPACE_ROOT → CARGO_MANIFEST_DIR/.. → cwd-walk) so
    // `sce-codegen verify` matches whatever workspace produced the
    // emit. Falls back to cwd only if every layer fails, mirroring
    // the pre-2026-05 forgiving behaviour for ad-hoc invocations.
    let explicit_root = current_workspace_root_override();
    let workspace_root = locate_workspace_root(explicit_root.as_deref()).unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    });
    let tpl_root_default = workspace_root
        .join("tools")
        .join("codegen")
        .join("templates");
    let lock_default = workspace_root.join("Cargo.lock");
    let tpl_root = template_root
        .map(std::path::PathBuf::from)
        .unwrap_or(tpl_root_default);
    let lock_path = cargo_lock
        .map(std::path::PathBuf::from)
        .unwrap_or(lock_default);

    let expected_source = match compute_source_hash(input_path, deploy_path) {
        Ok(h) => h,
        Err(e) => cli_exit(drift_hash_failure(input_path, "source-hash", e)),
    };
    let expected_template = match compute_template_hash(&tpl_root, &lock_path) {
        Ok(h) => h,
        Err(e) => cli_exit(drift_hash_failure(&tpl_root, "template-hash", e)),
    };
    let expected = DriftHashes {
        source_hash: expected_source,
        template_hash: expected_template,
    };

    let files = collect_generated_files(out_path);
    let mut headerless: Vec<std::path::PathBuf> = Vec::new();
    for file in &files {
        let Ok(content) = fs::read_to_string(file) else {
            continue;
        };
        let Some(embedded) = parse_embedded_hashes(&content) else {
            headerless.push(file.clone());
            continue;
        };
        if embedded.source_hash_hex != expected.source_hex() {
            error_format.emit_and_exit(
                &CliError::VerifySourceHashMismatch {
                    path: file.display().to_string(),
                    axis: "source",
                    expected_hex: expected.source_hex(),
                    actual_hex: embedded.source_hash_hex,
                },
                "",
            );
        }
        if embedded.template_hash_hex != expected.template_hex() {
            error_format.emit_and_exit(
                &CliError::VerifySourceHashMismatch {
                    path: file.display().to_string(),
                    axis: "template",
                    expected_hex: expected.template_hex(),
                    actual_hex: embedded.template_hash_hex,
                },
                "",
            );
        }
    }
    // Files without an SCE-GENERATED header are reported but do not
    // fail the verify pass on their own — the spec invariant is
    // `every emitted file carries a header`, but the verifier's job is
    // drift detection on files that DO have headers. Headerless files
    // surface as an informational note on stderr so authors notice
    // partial migration windows.
    if !headerless.is_empty() {
        eprintln!(
            "sce-codegen verify: {} headerless file(s) skipped (no SCE-GENERATED block); first: {}",
            headerless.len(),
            headerless[0].display()
        );
    }
}

/// Walks `out_dir` recursively and returns every file whose extension
/// matches the set SCE backends emit. Ordering is BTreeMap-sorted so
/// the first-mismatch-wins diagnostic is deterministic across
/// filesystem-readdir reorderings.
fn collect_generated_files(out_dir: &Path) -> Vec<std::path::PathBuf> {
    use std::collections::BTreeSet;
    fn walk(dir: &Path, out: &mut BTreeSet<std::path::PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "rs" | "cpp" | "h" | "kt" | "go" | "py" | "c") {
                    out.insert(path);
                }
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(out_dir, &mut out);
    out.into_iter().collect()
}

/// Locate the SCE workspace root — the directory carrying
/// `tools/codegen/templates/` + the workspace `Cargo.lock` that feed
/// the §synth-6.2.6 `template-hash`. Resolution priority (each layer must
/// validate the `tools/codegen/templates/` shape — paths failing the
/// check fall through to the next layer rather than silently
/// embedding the wrong root):
///
///   1. `explicit` argument — surfaced as `--workspace-root <PATH>`
///      on the CLI; takes precedence so vendored consumers can pin
///      the location from a build.rs / Makefile without relying on
///      env vars or process layout.
///   2. `SCE_WORKSPACE_ROOT` env var — mirrors the `SCE_TEMPLATE_DIR`
///      escape hatch already documented for [`sce_build::find_template_base`].
///   3. `CARGO_MANIFEST_DIR`'s parent — baked at compile time to the
///      `sce-build/` crate root inside whichever workspace built this
///      binary. For both in-repo and `path = "vendor/sce/sce-build"`
///      consumer builds this resolves to the SCE workspace that
///      shipped the matching template tree. This is the layer the
///      walk-up-from-cwd path (used pre-2026-05) missed for vendored
///      binaries — cwd lives in the consumer workspace while the
///      vendored SCE tree sits *below* it under `vendor/sce/`.
///   4. Walk upward from the current directory looking for a
///      `tools/codegen/templates/` directory. Last-resort path for
///      ad-hoc invocations from inside the SCE workspace tree.
///
/// Returns `None` only if every layer fails. Callers treat that as
/// either a recoverable warning (DriftContext template-hash falls
/// back to all-zero) or an error (`verify` surfaces an I/O failure).
fn locate_workspace_root(explicit: Option<&Path>) -> Option<std::path::PathBuf> {
    fn validates(candidate: &Path) -> bool {
        candidate
            .join("tools")
            .join("codegen")
            .join("templates")
            .exists()
    }

    if let Some(p) = explicit {
        if validates(p) {
            return Some(p.to_path_buf());
        }
        // Explicit override that does not validate is a user mistake
        // worth surfacing — the DriftContext call site already emits
        // a zero-hash warning when this function returns None.
        eprintln!(
            "sce-codegen: --workspace-root '{}' does not contain tools/codegen/templates/",
            p.display(),
        );
    }

    if let Ok(env_root) = std::env::var("SCE_WORKSPACE_ROOT") {
        let candidate = std::path::PathBuf::from(env_root);
        if validates(&candidate) {
            return Some(candidate);
        }
        eprintln!(
            "sce-codegen: SCE_WORKSPACE_ROOT '{}' does not contain tools/codegen/templates/",
            candidate.display(),
        );
    }

    // `CARGO_MANIFEST_DIR` is baked at compile time to the sce-build
    // crate's directory inside the workspace that built the binary —
    // including vendored builds where the consumer's
    // `path = "vendor/sce/sce-build"` dependency makes the parent
    // (`vendor/sce/`) the canonical SCE workspace root.
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    if let Some(parent) = crate_dir.parent() {
        if validates(parent) {
            return Some(parent.to_path_buf());
        }
    }

    // Last resort: walk upward from cwd. Kept for ad-hoc invocations
    // from inside the SCE workspace; vendored binaries normally hit
    // the `CARGO_MANIFEST_DIR` layer above before reaching here.
    let mut cursor = std::env::current_dir().ok()?;
    loop {
        if validates(&cursor) {
            return Some(cursor);
        }
        if !cursor.pop() {
            return None;
        }
    }
}

// ── Subcommand: list-fixtures ──────────────────────────────────

fn cmd_list_fixtures(
    manifest_path: &str,
    format: &str,
    language: Option<&str>,
    has_test_vectors_only: bool,
    resource_dir: Option<&str>,
) {
    let mut manifest = sce_build::conformance::Manifest::load(Path::new(manifest_path))
        .unwrap_or_else(|e| {
            cli_exit(CliError::ReadInput {
                path: manifest_path.to_string(),
                source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
            })
        });
    // RFC §synth-5-B B2-test-vector: enrich algorithm fixtures with their
    // derived `has_test_vectors` flag so `--has-test-vectors` filtering
    // (used by cmake harnesses to enumerate per-fixture sidecar
    // OUTPUTs) reads off the fixture spec without re-scanning each
    // SCXML at the call site. The flag is SCXML-derived rather than
    // carried in fixtures.json (manifest-side override is rejected by
    // `Manifest::validate`), so this enrichment requires the
    // per-fixture `<name>.scxml` to live under `--resource-dir`.
    // `--has-test-vectors` callers always need the flag; the
    // `--language` short-form invocation triggers enrichment too
    // because language harness scaffolds may emerge that read it.
    // The unset default skips enrichment to preserve the cheap path
    // for tools that do not want a per-fixture SCXML scan.
    if language.is_some() || has_test_vectors_only {
        // Resource directory resolution: explicit `--resource-dir`
        // wins; otherwise fall back to `<manifest_dir>/../resources/`
        // which mirrors the in-repo layout (`tests/forge/conformance/
        // fixtures.json` → `tests/forge/resources/`). The fallback
        // keeps short-form invocations like `--language cpp` working
        // without forcing every caller to pass the resource path.
        let resource_root: std::path::PathBuf = match resource_dir {
            Some(rd) => std::path::PathBuf::from(rd),
            None => {
                let manifest_dir = Path::new(manifest_path).parent().unwrap_or(Path::new("."));
                manifest_dir
                    .parent()
                    .map(|p| p.join("resources"))
                    .unwrap_or_else(|| manifest_dir.join("resources"))
            }
        };
        // Pre-resolve `--language` so the per-kind sidecar gate
        // (codec sidecar = Rust + C11 only) can be applied
        // alongside the SCXML scan in one pass. When `--language`
        // is unset the per-kind gate stays open for every backend
        // (matches the manifest-as-source-of-truth contract; the
        // listing is then a superset that downstream cmake configs
        // narrow with their own --language filter).
        let lang_for_enrich: Option<sce_build::generator::Language> =
            language.and_then(|s| s.parse::<sce_build::generator::Language>().ok());
        for f in manifest.fixtures.iter_mut() {
            let (has_tv_slot, kind_supports_sidecar) = match &mut f.spec {
                sce_build::conformance::FixtureSpec::Algorithm {
                    has_test_vectors, ..
                } => (Some(has_test_vectors), true),
                sce_build::conformance::FixtureSpec::Codec {
                    has_test_vectors, ..
                } => {
                    // RFC §synth-5-B codec test-vector sidecar: Rust +
                    // C11 only. Force the flag false on the
                    // 4 gated backends so the cmake `--has-test-
                    // vectors` listing matches what `render_codec_
                    // test_vector_sidecar` actually emits — otherwise
                    // those backends would declare a sidecar OUTPUT
                    // file that never gets generated. A backend
                    // joins this gate only together with its own
                    // sidecar template + golden.
                    let supports = match lang_for_enrich {
                        Some(sce_build::generator::Language::Rust)
                        | Some(sce_build::generator::Language::C11)
                        | None => true,
                        Some(_) => false,
                    };
                    (Some(has_test_vectors), supports)
                }
                _ => (None, false),
            };
            if let Some(slot) = has_tv_slot {
                if !kind_supports_sidecar {
                    *slot = false;
                    continue;
                }
                let scxml_path = resource_root.join(format!("{}.scxml", f.name));
                if scxml_path.exists() {
                    *slot = sce_build::conformance::has_test_vectors(&scxml_path).unwrap_or(false);
                }
            }
        }
    }
    // When `--language c11` is passed, mirror the per-kind
    // filter `generate-conformance` applies before harness rendering so
    // the c11 cmake harness sees identically-shaped fixture sets from
    // both subcommands. Unrecognised languages and the unset default
    // pass every manifest fixture through untouched, matching the prior
    // (filterless) contract every other backend already relies on.
    let lang_filter = language.map(|s| {
        s.parse::<Language>().unwrap_or_else(|_| {
            cli_exit(CliError::UnknownLanguage {
                lang: s.to_string(),
            })
        })
    });
    // RFC §synth-5-B B2-test-vector: when `--has-test-vectors` is passed,
    // restrict the listing to algorithm fixtures whose SCXML carries at
    // least one `<sce:test-vector>` element. The cmake harness uses
    // this to declare the per-fixture sidecar header as an additional
    // OUTPUT of the generate custom_command without speculating which
    // fixtures emit a sidecar. The enrichment block above already
    // populated `has_test_vectors` on each algorithm fixture (using
    // `--resource-dir` or the in-repo fallback), so the filter below
    // can read the flag straight off the fixture spec.
    // Resolve the resource-dir for the per-fixture MCU-only SCXML scan
    // that `lang_supports_fixture` performs. Same fallback as the
    // earlier sidecar enrichment block above.
    let resource_root_for_filter: std::path::PathBuf = match resource_dir {
        Some(rd) => std::path::PathBuf::from(rd),
        None => {
            let manifest_dir = Path::new(manifest_path).parent().unwrap_or(Path::new("."));
            manifest_dir
                .parent()
                .map(|p| p.join("resources"))
                .unwrap_or_else(|| manifest_dir.join("resources"))
        }
    };
    let names: Vec<&str> = manifest
        .fixtures
        .iter()
        .filter(|f| match lang_filter {
            // RFC §synth-5-J-4 single-source-of-truth gate: skip any fixture
            // whose product template hasn't shipped on the requested
            // language, or whose SCXML carries MCU-only features
            // (`<sce:dma-aligned>`) that the four non-MCU backends
            // reject at codegen time. Both checks live in
            // `lang_supports_fixture` so `cargo test` and `cmake --build`
            // see identical fixture sets — drift here would silently
            // schedule a per-fixture `add_custom_command` for a
            // generation that fails with `MCU-class kind`.
            Some(lang) => {
                sce_build::conformance::lang_supports_fixture(f, lang, &resource_root_for_filter)
                    .unwrap_or(false)
            }
            None => true,
        })
        .filter(|f| {
            if !has_test_vectors_only {
                return true;
            }
            matches!(
                &f.spec,
                sce_build::conformance::FixtureSpec::Algorithm {
                    has_test_vectors: true,
                    ..
                } | sce_build::conformance::FixtureSpec::Codec {
                    has_test_vectors: true,
                    ..
                }
            )
        })
        .map(|f| f.name.as_str())
        .collect();
    match format {
        "plain" => {
            for n in &names {
                println!("{n}");
            }
        }
        "cmake" => println!("{}", names.join(";")),
        "space" => println!("{}", names.join(" ")),
        other => cli_exit(CliError::InvalidFormatOption {
            value: other.to_string(),
            expected: "plain|cmake|space".into(),
        }),
    }
}

// ── Utility functions ───────────────────────────────────────────

/// Record the `// From:` provenance path against the invocation's
/// [`SOURCE_ROOT`] (delegates to lib). Reading the root here rather than
/// threading it keeps every emit site on one source of truth, matching how
/// the error format and workspace-root override are already handled.
fn resolve_source_path(model: &mut SCXMLModel, scxml_path: &Path) {
    sce_build::resolve_source_path(
        model,
        scxml_path.to_str().unwrap_or(""),
        current_source_root().as_deref(),
    );
}

/// Create a C++ formatter if language is C++ and formatting is not disabled.
/// Returns `None` for non-C++ languages, when `--no-format` is set, or when
/// no `clang-format` binary can be located.
fn create_cpp_formatter(
    lang: Language,
    format_style: Option<&str>,
    no_format: bool,
) -> Option<sce_build::formatter::CppFormatter> {
    if lang != Language::Cpp || no_format {
        return None;
    }
    match sce_build::formatter::CppFormatter::new(format_style.map(Path::new)) {
        Ok(f) => Some(f),
        Err(sce_build::formatter::FormatError::NotFound) => {
            eprintln!(
                "  Note: no clang-format located, skipping C++ formatting \
                 (set SCE_TOOL_CLANG_FORMAT to point at one)"
            );
            None
        }
        Err(sce_build::formatter::FormatError::StyleNotFound(p)) => {
            cli_exit(CliError::FormatStyleNotFound { path: p });
        }
        Err(e) => {
            eprintln!("  Warning: formatter init failed: {e}");
            None
        }
    }
}

/// Format generated file contents through the C++ formatter, if available.
/// Non-C++ files (by extension) pass through unchanged.
fn maybe_format_files(
    files: Vec<(String, String)>,
    formatter: &Option<sce_build::formatter::CppFormatter>,
) -> Vec<(String, String)> {
    let Some(fmt) = formatter else {
        return files;
    };
    fmt.format_output(files)
}

/// Write file only if content differs (preserves timestamps).
///
/// Normalisation runs before the comparison so a rerun that changes
/// nothing stays a no-op, preserving the mtime contract this function
/// exists for.
fn write_if_changed(path: &Path, content: &str) -> bool {
    let content = with_trailing_newline(content);
    let content = content.as_ref();
    if path.exists() {
        if let Ok(existing) = fs::read_to_string(path) {
            if existing == content {
                return false;
            }
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(path, content).unwrap_or_else(|e| {
        cli_exit(CliError::WriteOutput {
            path: path.display().to_string(),
            source: e,
        })
    });
    true
}

impl TestInfo {
    fn type_str(&self) -> &str {
        &self.test_type
    }
}

// ── Subcommand: addr2sce ───────────────────────────────────────
//
// SCE Protocol-Synthesis RFC §synth-5-O. Reverse-lookup from a mangled
// symbol or PC address back to SCXML coordinates (file + state path +
// line range).
//
// Three modes (spec lines 3253-3278):
//   `--symbol <NAME>` — direct sourcemap key lookup. No DWARF needed.
//   `--pc <ADDR>`     — ELF PC resolution. Deferred until a consumer
//                        materialises (per [[feedback-silently-broken-
//                        hooks]] we don't add the addr2line/gimli dep
//                        until an MCU consumer tests it end-to-end).
//   `--hardfault`     — bulk PC resolution from stdin. Same deferral
//                        as `--pc`.
//
// The `--symbol` path is live (the sourcemap-only path
// the foundation actually consumes — integration tests exercise it,
// the sourcemap-source-hash-mismatch diagnostic fires when the
// sidecar JSON drifts). `--pc` / `--hardfault` print a clear "deferred"
// message so authors that try those modes see a documented gap rather
// than an opaque crash.

fn cmd_addr2sce(
    sourcemap_dir: &str,
    symbol: Option<&str>,
    pc: Option<&str>,
    elf: Option<&str>,
    hardfault: bool,
    error_format: ErrorFormat,
) {
    let map_path = Path::new(sourcemap_dir).join("sce_sourcemap.json");
    let raw = match fs::read_to_string(&map_path) {
        Ok(s) => s,
        Err(e) => cli_exit(CliError::ReadInput {
            path: map_path.display().to_string(),
            source: e,
        }),
    };
    let map: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => cli_exit(CliError::ReadInput {
            path: map_path.display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
        }),
    };

    // Mode dispatch: exactly one of `--symbol` / `--pc` / `--hardfault`.
    match (symbol, pc, hardfault) {
        (Some(name), None, false) => addr2sce_resolve_symbol(&map, name, &map_path),
        (None, Some(_), false) | (None, None, true) => {
            let _ = elf;
            // Deferred — print a stable message so an automation
            // consumer can detect the deferral without parsing
            // human-readable prose. Exit non-zero so a CI gate that
            // depends on resolution does not silently green.
            eprintln!(
                "addr2sce: --pc / --hardfault modes are not implemented; \
                 --symbol is the supported mode. A consumer (e.g. an MCU JTAG \
                 debugger config) is needed to drive the addr2line / gimli dependency in."
            );
            std::process::exit(2);
        }
        _ => {
            let _ = error_format;
            eprintln!(
                "addr2sce: exactly one of --symbol / --pc / --hardfault required \
                 (use --symbol <NAME> for direct sourcemap lookup)"
            );
            std::process::exit(2);
        }
    }
}

/// Look `symbol` up in the loaded sourcemap and print the resolved
/// SCXML coordinates as a single JSON line on stdout. Returns
/// process exit 0 on a hit, 1 on a miss (so a CI gate using addr2sce
/// to verify symbol presence can fail loudly).
fn addr2sce_resolve_symbol(map: &serde_json::Value, symbol: &str, map_path: &Path) {
    let Some(symbols) = map.get("symbols").and_then(|v| v.as_object()) else {
        eprintln!(
            "addr2sce: malformed sourcemap at {} (no `symbols` object)",
            map_path.display()
        );
        std::process::exit(1);
    };
    let Some(entry) = symbols.get(symbol) else {
        eprintln!(
            "addr2sce: symbol '{symbol}' not found in {}",
            map_path.display()
        );
        std::process::exit(1);
    };
    // Echo the entry as a JSON line with the mangled symbol pinned.
    let out = serde_json::json!({
        "v": 1,
        "kind": "addr2sce",
        "symbol": symbol,
        "entry": entry,
    });
    println!("{}", out);
}
