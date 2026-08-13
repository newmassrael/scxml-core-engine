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
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use sce_build::analyzer;
use sce_build::cli_error::CliError;
use sce_build::cli_language::LanguageRoute;
use sce_build::filters;
use sce_build::forge::diagnostic::{Diagnostic, Stage, ToDiagnostics};
use sce_build::forge::error::{ForgeError, Located};
use sce_build::manifest::{
    ArtifactEntry, DeployInfo, LanguageVerdict, Manifest, ManifestKind, RejectedInfo,
};

use sce_build::generator::with_trailing_newline;
use sce_build::w3c_suite::{SuiteIdentity, SuiteIdentityError};

/// Where a conformance suite of the caller's own is rooted, and which
/// SCE checkout it depends on.
///
/// Present only when `--output-dir` names a root outside this
/// repository. The repository's own trees sit inside a Cargo
/// workspace / Gradle build / Go module that is hand-authored and
/// already correct; emitting the standalone shape over them would
/// replace working build files with ones that describe a package
/// standing alone, which this repository's is not.
struct StandaloneSuite {
    /// Directory the emitted package is rooted at — the one holding
    /// its build manifest (`<output-root>/backends/rust/tests`, ...).
    package_root: PathBuf,
    /// SCE checkout the emitted manifest points at for the runtime
    /// packages the suite depends on. Resolved from the same ladder
    /// that finds the registry, so it is a function of the run's
    /// inputs and never of where the output happened to land.
    sce_root: PathBuf,
}

/// Everything the emitted suite needs to know about itself, resolved
/// once per run and handed to whichever backend the run selected.
struct SuitePackaging {
    /// What the suite calls itself. `None` for the backends whose
    /// emitted code names no suite — see
    /// [`sce_build::w3c_suite::SuiteIdentity::for_language`].
    identity: Option<SuiteIdentity>,
    /// `Some` when the run writes a suite of the caller's own.
    standalone: Option<StandaloneSuite>,
}

impl SuitePackaging {
    /// The identity, for the three backends that always have one.
    ///
    /// Panics rather than defaulting: reaching here without an identity
    /// would mean a backend whose generated code names the suite was
    /// constructed for a language that
    /// [`SuiteIdentity::for_language`] refuses, and emitting some
    /// fallback name would put an unbuildable reference into every
    /// generated test instead of failing the run.
    fn identity(&self) -> &SuiteIdentity {
        self.identity
            .as_ref()
            .expect("this backend's generated code names the suite, so the run resolved one")
    }

    /// The standalone roots for a backend rooted at `relative` beneath
    /// the output root — `None` for an in-repo regeneration.
    fn standalone_at(&self, relative: &str) -> Option<StandaloneSuite> {
        self.standalone.as_ref().map(|s| StandaloneSuite {
            package_root: s.package_root.join(relative),
            sce_root: s.sce_root.clone(),
        })
    }
}

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
            // pre-serialized fallback record so downstream parsers at
            // least advance past the line. It lives on `Diagnostic`
            // because the tests there hold it to the schema — a
            // literal spelled here would be the one record no
            // validator ever sees.
            let line = serde_json::to_string(&meta)
                .unwrap_or_else(|_| Diagnostic::TERMINAL_FALLBACK_NDJSON.to_string());
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
/// collapse onto `cli/read-input`, which sent a repair consumer hunting
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
/// Accumulated symbol table for one invocation's output directory.
///
/// There is one `sce_sourcemap.json` per directory but an invocation
/// can emit several machines into it — a parent plus every
/// inline-`<content>` synth-invoke child. Each machine's symbols are
/// added here and the file is written once at the end.
///
/// Writing per machine, which is what this replaced, made the sidecar
/// describe only whichever machine happened to be emitted last: a
/// `generate` on a document with a synth-invoke child produced two
/// `_sm.*` artifacts and a sourcemap covering only the child, so every
/// parent symbol was unresolvable through `addr2sce` even though the
/// manifest listed the parent's artifact.
type SymbolAccumulator = BTreeMap<String, sce_build::forge::symbol_mangling::SymbolEntry>;

/// Add `model`'s symbols to `acc`.
fn collect_sourcemap_symbols(model: &SCXMLModel, acc: &mut SymbolAccumulator) {
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
    acc.extend(symbols);
}

/// Write the accumulated sourcemap for `target_dir`.
///
/// A no-op on an empty accumulator so a run that emitted no statechart
/// symbols (forge-only output, a rejected document) leaves no sidecar
/// rather than an empty one.
fn flush_sourcemap(acc: &SymbolAccumulator, target_dir: &Path, drift_ctx: &DriftContext) {
    use sce_build::forge::sourcemap;

    if acc.is_empty() {
        return;
    }
    let map = sourcemap::build(
        acc,
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
/// `Json` is the machine-readable contract consumed by upstream consumers
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

/// Root the generated trees were written under, when `--output-dir`
/// named one.
///
/// A second provenance root, not a replacement. Most inputs live under
/// [`SOURCE_ROOT`], but a hybrid child's SCXML is *synthesised into the
/// output tree* and read back from there, so under a foreign output root
/// it has no relative spelling beneath the project root — provenance
/// would fall back to the absolute path and the emitted bytes would then
/// carry the directory they were written to.
static OUTPUT_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Read the installed provenance root, if any.
fn current_source_root() -> Option<PathBuf> {
    SOURCE_ROOT.get().cloned()
}

/// Whether `path` resolves to somewhere beneath `root`.
///
/// Canonicalising both sides matches what `header_source_path` does when
/// it computes the relative spelling, so this predicate and that
/// computation cannot disagree about containment.
fn path_is_under(path: &Path, root: &Path) -> bool {
    match (fs::canonicalize(path), fs::canonicalize(root)) {
        (Ok(p), Ok(r)) => p.starts_with(&r),
        _ => false,
    }
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

// ── Path arithmetic ────────────────────────────────────────────
//
// `Path::parent` answers a lexical question, not a filesystem one:
// for a path carrying no separator it returns `Some("")`, and `""`
// names no directory — `read_dir("")` fails with `ENOENT`, and
// `create_dir_all("")` with it. Callers here want the *containing
// directory*, whose answer for a bare filename is the working
// directory.
//
// The gap opens on the shortest way there is to name a document:
//
//     cd resources && sce-codegen generate door.scxml -l rust -o out
//
// which walked `""` for the §synth-6.2.6 source set and refused with
// `cli/read-input` — carrying an empty path in the message, because
// the path that failed to read *was* the empty string. Naming the
// same document `./door.scxml`, or by any path with a separator in
// it, generated it. Every site below already carried an
// `unwrap_or(".")` written for this exact case; none of them fired,
// because the case is `Some("")` and not `None`.

/// The directory `path` lives in, as a directory that can be opened.
fn containing_dir(path: &Path) -> PathBuf {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        // Either `path` is a bare filename (`Some("")`) or it is a root
        // with nothing above it (`None`). In both cases the directory
        // holding it is the one the process is standing in.
        _ => PathBuf::from("."),
    }
}

/// The directory named `name` beside the directory `path` lives in.
///
/// `tests/forge/conformance/fixtures.json` + `resources` →
/// `tests/forge/resources`, the in-repo conformance layout.
///
/// Kept apart from [`containing_dir`] rather than composed from it
/// because the degenerate case resolves differently: the directory
/// above `.` has no name a `Path` can spell without `..`, while the
/// directory above a single relative component is the working
/// directory and needs no prefix at all.
fn sibling_of_containing_dir(path: &Path, name: &str) -> PathBuf {
    let dir = containing_dir(path);
    match dir.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(name),
        // `dir` is the working directory itself — reached from a bare
        // filename or a `./`-prefixed one. Its parent is spellable
        // only as `..`.
        _ if dir == Path::new(".") => Path::new("..").join(name),
        // `dir` is one relative component (`tests/`), so the directory
        // above it is the working directory and `name` alone names the
        // sibling.
        _ => PathBuf::from(name),
    }
}

// ── Stdout ─────────────────────────────────────────────────────
//
// Every byte this process puts on stdout leaves through here.
//
// A consumer that stops reading closes the pipe, and the next write
// fails with `BrokenPipe`. `println!` panics on that — exit 101, a
// panic message on stderr, no NDJSON record — which is a status §6
// does not define at all, reached by a condition the *consumer*
// chose rather than a fault of this run. §6 states the universal the
// other way round: "A non-zero exit with no NDJSON record is a
// contract violation."
//
// It surfaced on `list-fixtures` first because that is the
// subcommand whose help tells build systems to consume it without a
// JSON parser, and its output outruns the pipe buffer, so
// `list-fixtures … | head` lost the race about half the time. The
// other stdout writers differed only in printing too little to lose
// it — the panic was one long document away.
//
// `expand`, `requirements` and `unresolved` already carried the rule
// this restores: a stdout write failure is `cli/write-output` at exit
// 20, with a record naming the stream. Routing every writer through
// these two helpers makes that rule the only way to reach stdout, so
// a subcommand added later cannot reintroduce the panic by reaching
// for `println!`.

/// The path a stdout failure names. One spelling — the three sites
/// that already handled the failure used two.
const STDOUT_LABEL: &str = "<stdout>";

/// Hand locked stdout to `emit` and end the process on failure.
///
/// The one primitive. Streaming producers (the NDJSON reports) need a
/// writer rather than a finished string, so the writer — not the
/// bytes — is what this takes.
fn out_stream(emit: impl FnOnce(&mut dyn std::io::Write) -> std::io::Result<()>) {
    use std::io::Write;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    // Stdout is line-buffered, so output that ends without a newline
    // sits in the buffer until the implicit flush at exit — which
    // discards its error. Flushing here is what makes the failure
    // reportable at all.
    let result = emit(&mut handle).and_then(|()| handle.flush());
    if let Err(source) = result {
        drop(handle);
        cli_exit(CliError::WriteOutput {
            path: STDOUT_LABEL.to_string(),
            source,
        });
    }
}

/// Write `text` to stdout, followed by a newline.
fn out_line(text: &str) {
    out_stream(|w| {
        w.write_all(text.as_bytes())?;
        w.write_all(b"\n")
    })
}

/// Write `bytes` to stdout with nothing appended.
///
/// `expand` needs this: its output is compared byte-for-byte against
/// the C++ pugixml canonicalisation, which carries no trailing
/// newline.
fn out_bytes(bytes: &[u8]) {
    out_stream(|w| w.write_all(bytes))
}

/// `println!` for this binary's stdout: same call shape, but a closed
/// reader ends the process through [`out_line`]'s contract instead of
/// a panic.
macro_rules! outln {
    () => { out_line("") };
    ($($arg:tt)*) => { out_line(&format!($($arg)*)) };
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
    /// Whether any document in this run requires `Engine::tick()` rather
    /// than `step()`. Accumulated as a union alongside
    /// `needs_script_engine` because it answers the same kind of
    /// question about the same set.
    needs_event_scheduler: Option<bool>,
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

/// Build the stdout manifest for `report` under `kind`.
///
/// The shape lives in `sce_build::manifest` — see
/// SCE_ERROR_CONTRACT.md §10 for the prose contract and
/// `schemas/sce-manifest.v1.schema.json` for the wire schema. Both
/// `generate` and `check` funnel through here so the two subcommands
/// cannot drift into two shapes of the same record.
fn build_manifest<'a>(
    report: &'a GenerateReport,
    kind: ManifestKind,
    languages: Option<Vec<LanguageVerdict>>,
) -> Manifest<'a> {
    Manifest {
        v: sce_build::manifest::MANIFEST_SCHEMA_VERSION,
        kind: kind.as_str(),
        generator: sce_build::GENERATOR_COMMIT,
        artifacts: report
            .artifacts
            .iter()
            .map(|p| ArtifactEntry {
                path: p.display().to_string(),
            })
            .collect(),
        needs_script_engine: report.needs_script_engine.unwrap_or(false),
        script_engine_causes: &report.script_engine_causes,
        needs_event_scheduler: report.needs_event_scheduler.unwrap_or(false),
        rejected: report.rejected.as_ref().map(|rd| RejectedInfo {
            spec: rd.spec,
            name: rd.name.clone(),
        }),
        deploy: DeployInfo::from_facts(report.deploy_facts.as_ref()),
        languages,
    }
}

/// Serialise `report` and write it as a single JSON line to stdout.
fn emit_generate_manifest(report: &GenerateReport) {
    outln!(
        "{}",
        build_manifest(report, ManifestKind::Generate, None).to_line()
    );
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
    /// emitter (`cli_exit` / `ErrorFormat::emit_and_exit`), so consumers
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
    #[arg(
        short,
        long,
        default_value = "cpp",
        help = LanguageRoute::Generate.flag_summary("Target language"),
        long_help = LanguageRoute::Generate.flag_help("Target language"),
    )]
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
    /// Run the design-time statechart lints (`sce_build::lint_statechart`):
    /// graph reachability, event-set exhaustiveness, and guard
    /// analysis. Off by default because these reject **legal** SCXML —
    /// they assert design intent, not validity, and the W3C IRP corpus
    /// contains documents that deliberately declare unreachable states
    /// (`resources/278` hosts a datamodel read from outside its
    /// lexical scope; `resources/576` proves `initial` is honoured by
    /// leaving its first state unentered). Turn it on for authored
    /// documents, where an orphan region or a sibling missing an event
    /// handler is nearly always a mistake.
    #[arg(long)]
    lint: bool,
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

/// `--registry`'s long help, formatted from the constant the loader
/// resolves against the project root.
///
/// Written this way rather than as a doc comment because the two have
/// drifted before: the registry moved off `tests/CMakeLists.txt` and
/// the help kept naming it, so a caller doing exactly what the help
/// said got a JSON parse error on a build script. Formatting the one
/// constant into the one sentence makes that particular drift
/// unrepresentable; `cli_documented_catalog_paths` covers the rest.
fn w3c_registry_flag_help() -> String {
    format!(
        "Path to the W3C conformance registry — the JSON catalog at \
         `{}`, which declares every upstream test this build generates \
         and the harness each one needs.\n\n\
         Defaults to that path under the resolved project root. Name it \
         when the caller is not this repository: together with \
         `--resources` and `--output-dir`, that is what lets a vendoring \
         build drive the suite without owning SCE's directory layout.",
        sce_build::w3c_registry::W3C_REGISTRY_RELATIVE_PATH
    )
}

/// `--manifest`'s long help, shared by the two subcommands that read a
/// fixture catalog. Formatted from the constant for the same reason
/// [`w3c_registry_flag_help`] is.
fn forge_catalog_flag_help() -> String {
    format!(
        "Path to fixture catalog JSON — by default the numerical fixture \
         catalog at `{}`. Pass `--catalog` to read a different one.",
        sce_build::conformance::FORGE_CATALOG_RELATIVE_PATH
    )
}

/// `check --language`'s long help: the route's menu, then the part
/// specific to this subcommand — that the flag is repeatable and that
/// naming nothing means sweeping every backend.
///
/// The menu half is formatted from [`LanguageRoute`] like every other
/// route's, so the promise `check`'s own subcommand help makes — that
/// `check -l X` and `generate -l X` always agree — cannot be broken by
/// one of the two listing a backend the other omits. It was: `check`
/// listed all six and `generate` omitted `python`.
fn check_language_flag_help() -> String {
    format!(
        "{}\n\n\
         Repeatable. When omitted every backend is checked and the \
         per-backend verdict rides the manifest instead of the exit code \
         — see the subcommand's long help for the exit contract.",
        LanguageRoute::Check.flag_summary("Backend to check against")
    )
}

/// `list-fixtures --language`'s long help. The flag filters a listing
/// rather than selecting an output target, but it parses the same names
/// through the same [`Language`] `FromStr`, so it owes callers the same
/// menu — a name this route rejects is a name it must not have offered.
fn list_fixtures_language_flag_help() -> String {
    format!(
        "{}\n\n\
         When set to `c11`, applies the same `c11_supported_kind` filter \
         that `generate-conformance` uses, so the c11 cmake harness can \
         derive its fixture set from the single manifest source of truth. \
         Every other backend (and the unset default) emits every fixture \
         in the manifest unchanged.",
        LanguageRoute::ListFixtures.flag_summary("Optional language gate")
    )
}

/// `--catalog`'s long help. Both catalog locations come from their own
/// modules, so neither can be renamed out from under this sentence.
fn catalog_flag_help() -> String {
    format!(
        "Which catalog `--manifest` points at.\n\n\
         `forge` (default) is the numerical fixture catalog at `{}`; \
         `w3c` is the statechart conformance registry at `{}`. Named \
         rather than sniffed from the file's shape: the two catalogs \
         answer different questions, and a reader that guesses would \
         pick one silently on a malformed file.",
        sce_build::conformance::FORGE_CATALOG_RELATIVE_PATH,
        sce_build::w3c_registry::W3C_REGISTRY_RELATIVE_PATH
    )
}

/// CLI arguments for the `generate-w3c` subcommand. Extracted into an
/// `Args` struct (mirroring `GenerateArgs`) so `cmd_generate_w3c` takes
/// one parameter instead of eight, free of a `too_many_arguments` allow.
#[derive(clap::Args)]
struct GenerateW3cArgs {
    #[arg(
        short,
        long,
        help = LanguageRoute::GenerateW3c.flag_summary("Target language"),
        long_help = LanguageRoute::GenerateW3c.flag_help("Target language"),
    )]
    language: String,
    /// Path to the W3C conformance registry
    #[arg(long, long_help = w3c_registry_flag_help())]
    registry: Option<String>,
    /// Path to resources directory
    #[arg(long)]
    resources: Option<String>,
    /// Root the generated trees are written under.
    ///
    /// Each backend keeps its own layout beneath this root
    /// (`backends/rust/tests/`, `backends/kotlin/tests/`, ...) because
    /// the emitted code depends on it: the tests sit at a fixed place
    /// relative to the machines they exercise. Only the root moves.
    ///
    /// Defaults to the project root, which is what an in-repo
    /// regeneration wants. Naming a root outside this repository asks
    /// for a suite of the caller's own, and the run then also emits the
    /// files that make the tree buildable on its own — the build
    /// manifest, the module root, and the harness the generated tests
    /// call into. Together with `--registry`, `--resources` and
    /// `--suite-package`, that is what lets a vendoring build drive the
    /// conformance suite without owning SCE's directory layout or its
    /// build system.
    #[arg(long)]
    output_dir: Option<String>,
    /// Name the emitted conformance suite calls itself.
    ///
    /// Generated tests have to spell the suite they belong to: a Rust
    /// integration test lives outside the crate and names it
    /// (`sce_rust_tests::generated::test144`), a Go test imports
    /// `<module>/harness`, a Kotlin test imports the package root. Give
    /// the name in the target language's own idiom — a Cargo package
    /// name (`acme-conformance`), a Go module path
    /// (`github.com/acme/conformance`), a Kotlin package root
    /// (`com.acme.conformance`).
    ///
    /// Defaults to the name this repository's own suite carries, so an
    /// in-repo regeneration is unaffected. Only meaningful alongside an
    /// `--output-dir` outside this repository: renaming the suite while
    /// writing into the repository would desync the emitted sources
    /// from the committed build files, so that combination is refused.
    ///
    /// The Python and C++/C11 backends refuse it, because neither emits
    /// anything that names a suite — Python's tests import the machine
    /// beside them by path and take fixtures from pytest's
    /// directory-scoped conftest, and the C++ drivers are hand-authored
    /// headers that CMake configures.
    #[arg(long, value_name = "NAME")]
    suite_package: Option<String>,
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

/// CLI arguments for the `check` subcommand.
///
/// Deliberately narrower than [`GenerateArgs`]: every flag that only
/// shapes *output* (`--output-dir`, `--write-deps`, `--emit-ast`,
/// formatting, namespace prefixes, `--as-child`) is absent, because
/// `check` writes nothing for them to shape. What remains is the set
/// that changes how the document is *read* — include path, unresolved
/// strictness, forge import resolution, the const-fold budget, and the
/// no_std validation axis — so a document that checks clean is one
/// `generate` would accept under the same interpretation.
#[derive(clap::Args)]
struct CheckArgs {
    /// Input SCXML file path. Shorthand for a single-document run;
    /// `--scxml` names the rest of a set.
    ///
    /// Exempted by the same three ids that put the run on the
    /// document-set route and that the single-document-only flags below
    /// conflict with. Exempting `--scxml` alone made a forge-only or
    /// deploy-only set — both of which `orchestrate` accepts and builds
    /// — unnameable to `check`: the only spelling left forced a forge
    /// document into this statechart slot, where it was read as a
    /// statechart and refused for having no initial state.
    #[arg(required_unless_present_any = ["scxml_set", "forge", "deploy"])]
    scxml: Option<String>,
    /// Statechart document joining the set, mirroring `orchestrate`'s
    /// `--scxml`. Repeatable. Naming one puts the run on the
    /// document-set route.
    #[arg(long = "scxml", value_name = "PATH")]
    scxml_set: Vec<String>,
    /// Forge document joining the cross-doc registry, mirroring
    /// `orchestrate`'s `--forge`. Repeatable. Naming one puts the run on
    /// the document-set route, where cross-file `<sce:on-sample link>`
    /// and `<sce:outbox ref>` references resolve.
    #[arg(long = "forge", value_name = "PATH")]
    forge: Vec<String>,
    /// Path to `deploy.yaml`, mirroring `orchestrate`'s `--deploy`.
    /// Fires the deploy-aware cross-doc validators — link-vs-deploy
    /// declaration, burst absorption, reassembly — which otherwise
    /// silent-skip. Omit to keep the run deploy-unaware.
    #[arg(long = "deploy", value_name = "PATH")]
    deploy: Option<String>,
    #[arg(
        short,
        long = "language",
        value_name = "LANG",
        help = LanguageRoute::Check.flag_summary("Backend to check against"),
        long_help = check_language_flag_help(),
    )]
    language: Vec<String>,
    /// Additional directories searched (in declaration order) to
    /// resolve `<xi:include href="...">` and `<sce:use template="...">`
    /// fragments by name. Mirrors `generate`'s `-I`.
    ///
    /// Single-document runs only: the multi-doc compile entry point
    /// that `orchestrate` and the document-set route share resolves
    /// includes relative to each document, with no search path, so
    /// honouring this there would answer a question no producer can
    /// reproduce.
    #[arg(
        short = 'I',
        long = "include-dir",
        value_name = "DIR",
        conflicts_with_all = ["scxml_set", "forge", "deploy"]
    )]
    include_dir: Vec<String>,
    /// Reject the document when it carries any `<sce:unresolved>`
    /// placeholder. Mirrors `generate --strict-unresolved`.
    ///
    /// Single-document runs only — `orchestrate` has no counterpart, and
    /// a `check` stricter than the producer it mirrors would refuse
    /// document sets that build.
    #[arg(long, conflicts_with_all = ["scxml_set", "forge", "deploy"])]
    strict_unresolved: bool,
    /// Run the design-time statechart lints. Mirrors
    /// `generate --lint` — same `sce_build::lint_statechart` call, so
    /// `check --lint` and `generate --lint` cannot disagree about a
    /// document.
    #[arg(long, conflicts_with_all = ["scxml_set", "forge", "deploy"])]
    lint: bool,
    /// Go module path hosting the generated forge packages. Required to
    /// check any Go crossfile document; ignored for other backends.
    #[arg(long)]
    go_module_prefix: Option<String>,
    /// Build-time const-fold iteration budget. Mirrors `generate`.
    #[arg(long)]
    const_fold_budget: Option<u64>,
    /// Validate against the Rust `no_std` runtime variant. Only
    /// meaningful when `rust` is among the checked backends.
    ///
    /// Single-document runs only: the document-set route renders through
    /// the multi-doc compile entry point, which has no `no_std` variant
    /// to render, so the flag has no producer to agree with there.
    #[arg(long, conflicts_with_all = ["scxml_set", "forge", "deploy"])]
    no_std: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate code from a single SCXML file
    Generate(Box<GenerateArgs>),
    /// Validate a document without writing anything.
    ///
    /// Reaches the same verdict `generate` would — the same parse, the
    /// same validators, the same backend codegen — and then discards
    /// the result instead of writing it. Nothing is created: no
    /// artifacts, no sourcemap, no output directory. The manifest's
    /// `artifacts` array is empty by construction, which is the
    /// contract, not an accident of the input.
    ///
    /// Two refusal axes, and the exit code distinguishes them:
    ///
    /// * The **document** axis (`xml/*`, `validation/*`, `scxml/*`) —
    ///   the document is wrong under any backend. Always fatal: the
    ///   diagnostic goes to stderr, stdout stays empty, exit is
    ///   non-zero.
    /// * The **backend** axis (`generate/*`, `codegen/*`) — the
    ///   document is well-formed but some backend cannot lower a
    ///   construct it uses. With `--language` named, this is fatal too,
    ///   so `check -l X` and `generate -l X` always agree. With no
    ///   `--language`, every backend is swept and the per-backend
    ///   verdict rides the manifest's `languages` array with exit 0 —
    ///   "only Rust can lower this" is an answer, not a failure.
    ///
    /// The reference producer is the one the invocation shape names.
    /// A lone document is checked against `generate`; a document set —
    /// any `--scxml`, `--forge`, or `--deploy` — is checked against
    /// `orchestrate`, so the cross-doc registry is built and the deploy
    /// validators fire instead of silent-skipping. `orchestrate` has to
    /// be given an `--output-dir` and materialises the whole build into
    /// it, which makes "is this system valid?" cost a tree of artifacts;
    /// the document-set route answers it and writes nothing.
    Check(Box<CheckArgs>),
    /// Multi-doc generate with cross-doc registry — wires
    /// `validate_on_sample_link_references` into production
    /// (SCE Protocol-Synthesis RFC §synth-5-D).
    /// Use this when the build has multiple SCXML/forge docs that
    /// reference each other across files (`<sce:on-sample link>`,
    /// `<sce:outbox ref>`); single-file `Generate` does not
    /// build the cross-doc registry and silently skips cross-ref
    /// validation. Both lists may be empty (no-op).
    Orchestrate(Box<OrchestrateArgs>),
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
        #[arg(
            short,
            long,
            help = LanguageRoute::GenerateIntegration.flag_summary("Target language"),
            long_help = LanguageRoute::GenerateIntegration.flag_help("Target language"),
        )]
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
    /// Generate a cross-language numerical conformance test harness from
    /// a fixture catalog — one catalog is the single source of truth for
    /// every backend it serves, which `--language` names.
    ///
    /// The count used to be spelled out here ("all 5 languages") and was
    /// one short of what the dispatcher emitted; a sentence that counts
    /// the backends has to be corrected every time one lands, so this
    /// one does not count them.
    GenerateConformance {
        #[arg(
            short,
            long,
            help = LanguageRoute::GenerateConformance.flag_summary("Target language"),
            long_help = LanguageRoute::GenerateConformance.flag_help("Target language"),
        )]
        language: String,
        /// Path to fixture catalog JSON
        #[arg(short, long, long_help = forge_catalog_flag_help())]
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
        /// Path to fixture catalog JSON
        #[arg(short, long, long_help = forge_catalog_flag_help())]
        manifest: String,
        /// Output format. `plain` (default) is one fixture name per line,
        /// suitable for `for fixture in $(sce-codegen list-fixtures ...)`.
        /// `cmake` emits a single semicolon-separated CMake list literal.
        /// `space` emits a single space-separated line.
        #[arg(short, long, default_value = "plain")]
        format: String,
        #[arg(
            short,
            long,
            help = LanguageRoute::ListFixtures.flag_summary("Optional language gate"),
            long_help = list_fixtures_language_flag_help(),
        )]
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
        /// Which catalog `--manifest` points at
        #[arg(long, default_value = "forge", long_help = catalog_flag_help())]
        catalog: String,
        /// Restrict a `--catalog w3c` listing to fixtures naming this
        /// harness (`simple`, `scheduled`, `http`). This is how a build
        /// system reconstructs the per-harness registration groups
        /// without parsing JSON. Unset lists every registered fixture.
        #[arg(long)]
        harness: Option<String>,
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
    /// `--pc <ADDR>`       Resolve an ELF program-counter address to the
    ///                     function symbol containing it (`--elf`
    ///                     required), then look that symbol up.
    /// `--hardfault`       Read newline-separated PC addresses from
    ///                     stdin and resolve each as `--pc` would, one
    ///                     record per frame in the order given. Exits 1
    ///                     when any frame is unresolvable.
    ///
    /// Spec lines 3253-3278 fix the tool's resolution contract:
    /// PC → symbol → sourcemap → SCXML file:line + state_path. The
    /// per-symbol attribution data ships in the sourcemap, so the
    /// address→symbol hop reads the ELF symbol table rather than a
    /// DWARF line program — `.symtab` carries the ranges this needs and
    /// survives `--strip-debug`.
    ///
    /// On ARM the Thumb bit (bit 0) is cleared on both the symbol and
    /// the query address, so a PC taken from a Cortex-M exception frame
    /// resolves like its even neighbour.
    ///
    /// For the opposite direction — SCXML coordinates to the symbols
    /// they lowered to — see `sce2sym`.
    // The three modes and the `--elf` they need are expressed to the
    // argument parser rather than checked by hand after it. A
    // hand-rolled arity check is a second argument parser: it emitted
    // prose and exited 2, which SCE_ERROR_CONTRACT.md §6 assigns to
    // `xml/*` — so mistyping a flag reported a malformed *document*.
    // Declared here, the same mistakes leave as `cli/usage`.
    #[command(name = "addr2sce")]
    #[command(group = clap::ArgGroup::new("addr2sce_mode")
        .required(true)
        .multiple(false)
        .args(["symbol", "pc", "hardfault"]))]
    Addr2Sce {
        /// Directory containing `sce_sourcemap.json` (per-machine
        /// output, e.g. `target/.../src/generated/test144/`).
        sourcemap_dir: String,
        /// Mangled symbol to look up directly (mutually exclusive
        /// with `--pc` / `--hardfault`).
        #[arg(long)]
        symbol: Option<String>,
        /// ELF program-counter address, hexadecimal with or without a
        /// `0x` prefix (a bare value is read as hex — every stack dump
        /// prints hex). Requires `--elf`.
        #[arg(long, requires = "elf", value_parser = parse_pc_address)]
        pc: Option<u64>,
        /// ELF image whose symbol table maps an address to a function.
        ///
        /// Required by `--pc` / `--hardfault`: the sourcemap keys on
        /// symbol *names*, and only the image knows which name owns a
        /// given address.
        #[arg(long)]
        elf: Option<String>,
        /// Read PC addresses from stdin, one per line, and resolve each
        /// as `--pc` would. Blank lines are skipped so a pasted dump
        /// works verbatim. Requires `--elf`.
        #[arg(long, requires = "elf")]
        hardfault: bool,
    },
    /// Resolve SCXML coordinates to the symbols they lowered to — the
    /// reverse of `addr2sce`.
    ///
    /// `addr2sce` answers "which line of SCXML produced this symbol";
    /// this answers "which symbols did this line of SCXML produce".
    /// The two are not mirror images in shape: a mangled symbol is a
    /// map key, so the forward direction is one exact lookup, while a
    /// single SCXML coordinate legitimately lowers to several symbols
    /// (a state's body, its entry block, each of its transitions) and,
    /// across backends, to several sidecars. Output is therefore
    /// NDJSON — one record per hit — against
    /// `schemas/sce-symbol-lookup.v1.schema.json`.
    ///
    /// Pass more than one sourcemap directory to ask the same question
    /// of several backends at once; each record names the sidecar it
    /// came from.
    ///
    /// Every filter is optional and they intersect. With none, the
    /// whole table is listed — the useful default for "what did this
    /// document lower to", which a lookup that demanded a key could
    /// not express.
    // Named explicitly: clap's derived kebab-case renders this variant
    // as `sce2-sym`, which does not match the `addr2sce` it is the
    // reverse of. The pair reads as a pair only if both spell the
    // direction the same way.
    #[command(name = "sce2sym")]
    Sce2Sym {
        /// Directory containing `sce_sourcemap.json`. Repeat to query
        /// several backends in one invocation.
        #[arg(required = true)]
        sourcemap_dir: Vec<String>,
        /// Canonical state-hierarchy path, matched exactly (e.g.
        /// `s1/s1p1`). The machine-level symbol carries an empty path.
        #[arg(long)]
        state: Option<String>,
        /// 1-based source line that must fall inside the symbol's
        /// line range, inclusive at both ends.
        #[arg(long)]
        line: Option<u32>,
        /// IR node kind, matched exactly: `state`, `transition`,
        /// `on_entry`, `on_exit`, `forge_body`, `machine`.
        #[arg(long)]
        kind: Option<String>,
        /// Event name, matched exactly. Symbols carrying no event
        /// never match a constrained query.
        #[arg(long)]
        event: Option<String>,
        /// Author-facing SCXML path, matched exactly. Narrows a
        /// sourcemap that covers several documents.
        #[arg(long)]
        file: Option<String>,
    },
}

/// Multi-doc generate with cross-doc registry — wires
/// `validate_on_sample_link_references` into production
/// (SCE Protocol-Synthesis RFC §synth-5-D).
/// Use this when the build has multiple SCXML/forge docs that
/// reference each other across files (`<sce:on-sample link>`,
/// `<sce:outbox ref>`); single-file `Generate` does not
/// build the cross-doc registry and silently skips cross-ref
/// validation. Both lists may be empty (no-op).
#[derive(clap::Args)]
struct OrchestrateArgs {
    /// Input SCXML file path (repeat for multiple files).
    #[arg(long = "scxml")]
    scxml: Vec<String>,
    /// Input forge file path (repeat for multiple files).
    #[arg(long = "forge")]
    forge: Vec<String>,
    /// Additional directories searched (in declaration order) to
    /// resolve `<xi:include href="...">` and `<sce:use template="...">`
    /// fragments by name — same semantics as `generate --include-dir`.
    /// Without it, fragments resolve relative to the including document
    /// only, which is the whole search path a multi-doc build had.
    #[arg(short = 'I', long = "include-dir", value_name = "DIR")]
    include_dir: Vec<String>,
    #[arg(
        short,
        long,
        help = LanguageRoute::Orchestrate.flag_summary("Target language"),
        long_help = LanguageRoute::Orchestrate.flag_help("Target language"),
    )]
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
    /// Go module path hosting the generated forge packages.
    /// Required to build any Go document carrying `<sce:import>`;
    /// ignored for other backends.
    ///
    /// `check`'s document-set route takes this flag and names this
    /// subcommand as the producer whose verdict it mirrors. Without
    /// it here that mirror could not hold: `check --scxml A --forge
    /// B -l go --go-module-prefix M` returned exit 0 while the run
    /// it claimed to predict had no way to be given `M` and failed
    /// with `<sce:import> with language=go requires
    /// ForgeCompileOptions.go_module_prefix`.
    #[arg(long)]
    go_module_prefix: Option<String>,
    /// Build-time const-fold iteration budget, capping the total
    /// iteration count across every `<sce:fold>` body in the
    /// document. Mirrors `generate` and `check`; unset uses the
    /// RFC §synth-5-F default of 1_000_000.
    #[arg(long)]
    const_fold_budget: Option<u64>,
}

/// Recover `--error-format` from the raw argument vector.
///
/// Needed only on the parse-failure path: when [`Cli::try_parse`]
/// fails there is no parsed `--error-format` to read, yet the failure
/// still has to be rendered in the format the caller asked for. A
/// caller that mistyped the format flag itself falls back to human,
/// which is the right direction — the prose says what was wrong with
/// the flag.
fn peek_error_format<I: IntoIterator<Item = String>>(args: I) -> ErrorFormat {
    let mut want_value = false;
    for arg in args {
        if want_value {
            return if arg == "json" {
                ErrorFormat::Json
            } else {
                ErrorFormat::Human
            };
        }
        match arg.as_str() {
            "--error-format" => want_value = true,
            "--error-format=json" => return ErrorFormat::Json,
            _ => {}
        }
    }
    ErrorFormat::Human
}

/// Parse the command line, reporting a parse failure through the same
/// diagnostic pipeline as every other failure.
///
/// `clap`'s own `parse()` prints prose and exits 2 on a malformed
/// invocation. Both halves of that break the contract: exit 2 is the
/// status `SCE_ERROR_CONTRACT.md` §6 assigns to `xml/*`, so a caller
/// that mistyped a flag was told its document was malformed, and the
/// prose carries no `code` for it to read instead.
///
/// `--help` and `--version` keep clap's behaviour exactly — they are
/// successful requests for output, not failures, and they leave
/// through clap's own writer on stdout with status 0.
fn parse_cli() -> Cli {
    match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            use clap::error::ErrorKind;
            if matches!(
                err.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) {
                err.exit();
            }
            let _ = ERROR_FORMAT.set(peek_error_format(std::env::args()));
            // `render()` is the message without clap's ANSI styling —
            // §7 forbids escapes in JSON mode, and a human-mode reader
            // gains nothing from colour it did not ask for.
            cli_exit(CliError::Usage {
                detail: err.render().to_string().trim_end().to_string(),
            })
        }
    }
}

fn main() {
    let cli = parse_cli();
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
        Commands::Check(args) => cmd_check(*args, error_format),
        Commands::Orchestrate(args) => cmd_orchestrate(*args, error_format),
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
            catalog,
            harness,
        } => cmd_list_fixtures(
            &manifest,
            &format,
            language.as_deref(),
            has_test_vectors,
            resource_dir.as_deref(),
            &catalog,
            harness.as_deref(),
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
            pc,
            elf.as_deref(),
            hardfault,
            error_format,
        ),
        Commands::Sce2Sym {
            sourcemap_dir,
            state,
            line,
            kind,
            event,
            file,
        } => cmd_sce2sym(
            &sourcemap_dir,
            sce_build::forge::sourcemap::SymbolQuery {
                state_path: state.as_deref(),
                line,
                kind: kind.as_deref(),
                event: event.as_deref(),
                file: file.as_deref(),
            },
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

fn cmd_orchestrate(args: OrchestrateArgs, error_format: ErrorFormat) {
    // Destructured once, into the borrowed shapes the body below reads.
    // The arguments travel as a struct because `Generate`, `Check` and
    // `GenerateW3c` already do: a flat parameter list crossed clippy's
    // `too_many_arguments` the moment this subcommand gained the two
    // options `check` had all along.
    let OrchestrateArgs {
        scxml,
        forge,
        include_dir,
        language: language_arg,
        output_dir: output_dir_arg,
        deploy,
        emit_ast_dir: emit_ast_dir_arg,
        go_module_prefix,
        const_fold_budget,
    } = args;
    let include_dirs: Vec<PathBuf> = include_dir.iter().map(PathBuf::from).collect();
    let scxml_paths: &[String] = &scxml;
    let forge_paths: &[String] = &forge;
    let language: &str = &language_arg;
    let output_dir: &str = &output_dir_arg;
    let deploy_path: Option<&str> = deploy.as_deref();
    let emit_ast_dir: Option<&str> = emit_ast_dir_arg.as_deref();
    // Per-doc AST emit runs *before* the multi-doc compile so the AST
    // surface is independent of any cross-doc validation outcome. A
    // failing cross-doc validator must still produce ASTs for the
    // docs that parsed successfully — consumer tooling treats AST
    // emit as observation, not as compile commitment.
    if let Some(dir) = emit_ast_dir {
        emit_orchestrate_asts(scxml_paths, forge_paths, dir, &include_dirs, error_format);
    }
    let lang: Language = language.parse().unwrap_or_else(|_| {
        error_format.emit_and_exit(
            &CliError::UnknownLanguage {
                lang: language.to_string(),
                route: LanguageRoute::Orchestrate,
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

    // The two options `check`'s document-set route accepts, since that
    // route names this subcommand as the producer it mirrors: a verdict
    // check can reach has to be a verdict this run can reach too.
    // Everything else keeps `Generate`'s sentinel defaults.
    let options = sce_build::ForgeCompileOptions {
        go_module_prefix,
        const_fold_budget,
        include_dirs: include_dirs.clone(),
        ..Default::default()
    };

    let template_dir = sce_build::find_template_dir_for(lang);

    // C13 orchestrator wiring (`b501b18c`): parse the optional
    // deploy.yaml into a `DeployConfig` so the multi-doc compile path
    // can fire SCE Protocol-Synthesis RFC §synth-5-K + §synth-5-M cross-doc validators.
    // Shared with `check`'s document-set route, which enters the same
    // validators and so must refuse the same broken topologies.
    let deploy_cfg: Option<sce_build::mesh::deploy::DeployConfig> =
        deploy_path.map(|p| load_deploy_config(p, error_format));

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
    // can recompute and gate on. `input_root` defaults to the directory
    // holding the first SCXML path so a typical batch (all docs in one
    // directory) hashes its whole input set; a flat fallback to "."
    // keeps multi-dir invocations functional even though their hash
    // is then the cwd recursive walk.
    let drift_input_root: std::path::PathBuf = scxml_path_bufs
        .first()
        .map(|p| containing_dir(p))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let drift_ctx = DriftContext::compute(
        &drift_input_root,
        deploy_path.map(Path::new),
        scxml_path_bufs.first().map(|p| p.as_path()),
    );

    // §10 stdout manifest. Built from the same `GenerateReport` shape
    // and the same `build_manifest` construction point the other two
    // subcommands use, so the three cannot drift into three shapes of
    // one record. Without it this subcommand wrote a tree of artifacts
    // and told no one: `check` mirrors its verdict but reports
    // `artifacts: []` by contract, so nothing on the wire named the
    // files a build had just produced.
    //
    // `needs_script_engine` describes the **set** — the union over its
    // statechart inputs — which is the form the question takes for a
    // build deciding whether to link an engine, and the same form
    // `check`'s document-set route answers it in. Both read it through
    // `scxml_script_engine_facts`, so the two routes cannot disagree.
    let mut report = GenerateReport {
        deploy_facts: deploy_cfg.as_ref().map(|cfg| sce_build::DeployFacts {
            static_analyzer: cfg.build.as_ref().and_then(|b| b.static_analyzer),
        }),
        ..GenerateReport::default()
    };
    let mut needs_script_engine = false;
    let mut needs_event_scheduler = false;
    for path in &scxml_path_bufs {
        let Some(facts) = scxml_host_requirement_facts(&path.to_string_lossy()) else {
            continue;
        };
        needs_script_engine |= facts.needs_script_engine;
        needs_event_scheduler |= facts.needs_event_scheduler;
        for cause in facts.script_engine_causes {
            if !report.script_engine_causes.contains(&cause) {
                report.script_engine_causes.push(cause);
            }
        }
    }
    report.needs_script_engine = Some(needs_script_engine);
    report.needs_event_scheduler = Some(needs_event_scheduler);

    for (basename, generated) in &outputs {
        for (file_name, code) in &generated.files {
            let path = out_root.join(file_name);
            write_drift_aware(error_format, &path, code, &drift_ctx);
            // Recorded at the write, not from `outputs` — §10.1 defines
            // `artifacts` as every file written, so a path that never
            // reached the disk must never reach the manifest.
            report.artifacts.push(path);
        }
        let _ = basename; // basename is the input-doc label; outputs already self-name.
    }

    outln!(
        "{}",
        build_manifest(&report, ManifestKind::Orchestrate, None).to_line()
    );
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
    include_dirs: &[PathBuf],
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

        // Expand before parsing, as the statechart loop above does via
        // `parse_file`. An AST emitted from unexpanded source would
        // describe a document the author never wrote — every node a
        // `<sce:use>` was carrying would simply be absent from it.
        let content = match sce_build::parser::expand_preprocessors(
            &content,
            forge_path_str,
            forge_path.parent(),
            include_dirs,
        ) {
            Ok((expanded, _map, _deps)) => expanded,
            Err(e) => error_format.emit_forge_and_exit(&e),
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

/// Wire `code` of the first diagnostic an error expands to.
///
/// The per-backend verdict carries the same code a consumer would read
/// off stderr, so a refusal surfaced through the manifest routes to the
/// repair path the diagnostic wire already defines.
fn first_diagnostic_code<E: ToDiagnostics>(err: &E) -> String {
    err.to_diagnostics()
        .first()
        .map(|d| d.code.as_str().to_string())
        .unwrap_or_else(|| "generate/unknown".to_string())
}

/// Record one backend's outcome, or terminate.
///
/// `explicit` is the caller's `--language` choice: when the operator
/// named the backend, its refusal is the answer to the question they
/// asked and must behave exactly as `generate -l <lang>` does. When no
/// backend was named the sweep is exploratory, so a refusal is data —
/// but only a refusal that belongs to one backend. `axis` is how the
/// caller's route tells the two apart.
fn record_backend_outcome<E: ToDiagnostics>(
    verdicts: &mut Vec<LanguageVerdict>,
    lang: Language,
    explicit: bool,
    axis: SweepAxis,
    error_format: ErrorFormat,
    outcome: Result<(), E>,
) {
    let wire = sce_build::manifest::language_wire_name(lang);
    match outcome {
        Ok(()) => verdicts.push(LanguageVerdict::ok(wire)),
        Err(e) => {
            if explicit || !axis.is_one_backends_refusal(&e) {
                error_format.emit_and_exit(&e, "");
            }
            verdicts.push(LanguageVerdict::rejected(wire, first_diagnostic_code(&e)));
        }
    }
}

/// How a route's refusals are classified when no backend was named.
///
/// Only a refusal that belongs to one backend can ride the manifest's
/// `languages` array. A document, a cross-doc reference or a deploy
/// topology is wrong under every backend, so a sweep has nothing to
/// report about it and must fail the same way `--language` would —
/// otherwise `check` answers "valid" with exit 0 for a build no
/// producer can produce.
#[derive(Clone, Copy)]
enum SweepAxis {
    /// The route already ran its document validators and exited on
    /// them, so everything reaching the recorder is one backend's
    /// refusal by construction. The single-document route.
    BackendOnly,
    /// The route's compile call fuses document-set validation with
    /// per-backend rendering, so nothing about the call site says which
    /// axis a refusal came from. Read it off the diagnostic instead.
    ByStage,
}

impl SweepAxis {
    /// Stages that describe rendering for one backend. Every other
    /// stage — the document stages, and the mesh deploy/topology
    /// validators — reaches the same verdict whichever backend is
    /// named.
    fn is_one_backends_refusal<E: ToDiagnostics>(self, err: &E) -> bool {
        match self {
            SweepAxis::BackendOnly => true,
            SweepAxis::ByStage => err
                .to_diagnostics()
                .first()
                .is_some_and(|d| matches!(d.stage, Stage::Generate | Stage::MeshCodegen)),
        }
    }
}

/// Resolve `--language` into the backends to check and whether the
/// operator named them.
///
/// An empty list means "sweep every backend"; a non-empty one pins the
/// question to what was named, which is what makes a refusal fatal. Both
/// `check` routes resolve it here so the sweep-vs-named split cannot
/// come to mean two different things depending on the invocation shape.
fn requested_backends(language: &[String], error_format: ErrorFormat) -> (bool, Vec<Language>) {
    let explicit = !language.is_empty();
    let langs: Vec<Language> = if explicit {
        language
            .iter()
            .map(|name| {
                name.parse::<Language>().unwrap_or_else(|_| {
                    error_format.emit_and_exit(
                        &CliError::UnknownLanguage {
                            lang: name.to_string(),
                            route: LanguageRoute::Check,
                        },
                        "",
                    )
                })
            })
            .collect()
    } else {
        Language::ALL.to_vec()
    };
    (explicit, langs)
}

/// Read and parse a `deploy.yaml`, or terminate.
///
/// Both `orchestrate` and `check`'s document-set route enter the C13
/// cross-doc validators through the same `Option<&DeployConfig>`, so
/// they must also agree on what a broken topology does. Read failures
/// route through [`CliError`]; parse failures route through
/// `ForgeError::Mesh` so the wire JSON shape matches every other
/// deploy-side diagnostic, and the label points at the path the operator
/// passed rather than at whatever the parser was reading.
fn load_deploy_config(
    path: &str,
    error_format: ErrorFormat,
) -> sce_build::mesh::deploy::DeployConfig {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        error_format.emit_and_exit(
            &CliError::ReadInput {
                path: path.to_string(),
                source: e,
            },
            "",
        )
    });
    match sce_build::mesh::deploy::parse_deploy_str(&content) {
        Ok(cfg) => cfg,
        Err(e) => {
            let forge_err: ForgeError = sce_build::mesh::error::MeshError::from(e).into();
            let located = Located::new(forge_err, path, None, None);
            error_format.emit_forge_and_exit(&located)
        }
    }
}

/// Script-engine facts for one statechart, or `None` when the document
/// does not parse.
///
/// Report-only, and silent on failure by design: the compile pass that
/// follows reads the same file and is what reports a parse error, with
/// the diagnostic code and exit status the producer would give. Failing
/// here instead would put the first refusal on a pass whose job is to
/// describe the run, not to judge it.
fn scxml_host_requirement_facts(path: &str) -> Option<HostRequirements> {
    let mut parser = SCXMLParser::new();
    let mut model = parser.parse_file(path).ok()?;
    analyzer::analyze(&mut model, path);
    Some(HostRequirements {
        needs_script_engine: model.needs_script_engine,
        script_engine_causes: model
            .script_engine_causes
            .iter()
            .map(|c| c.to_wire())
            .collect(),
        needs_event_scheduler: model.needs_event_scheduler_driving(),
    })
}

/// What one statechart document asks of the host that will run it.
///
/// Carried together because the manifest reports them together and the
/// set-union routes accumulate them in one pass — splitting the walk
/// would let the two answers be derived from two different parses of
/// the same file.
struct HostRequirements {
    needs_script_engine: bool,
    script_engine_causes: Vec<sce_build::script_engine_analyzer::ScriptEngineCauseRecord>,
    needs_event_scheduler: bool,
}

/// `sce-codegen check` over a document set — every verdict
/// `orchestrate` would reach, nothing written.
///
/// This is [`cmd_orchestrate`] with its write loop absent. Both enter
/// [`sce_build::compile_scxml_with_imports`], which builds the cross-doc
/// registry, fires the SCE Protocol-Synthesis RFC §synth-5-K +
/// §synth-5-M deploy validators when a topology is supplied, and returns
/// every artifact in memory. Writing is the caller's step, so declining
/// to take it is the whole difference between the two subcommands —
/// there is no second validation path here that could drift out of
/// agreement with the producer.
///
/// The manifest reports the set, not a document: `needs_script_engine`
/// is the union over the statechart inputs, which is the question a
/// build system asks (does this build link an engine?) rather than a
/// per-file property it would have to re-aggregate itself.
fn cmd_check_document_set(args: CheckArgs, error_format: ErrorFormat) {
    let CheckArgs {
        scxml,
        scxml_set,
        forge,
        deploy,
        language,
        include_dir,
        strict_unresolved: _,
        lint: _,
        go_module_prefix,
        const_fold_budget,
        no_std: _,
        // The three flags this route cannot honour are declared to
        // conflict with every argument that reaches it, so clap refuses
        // the combination before dispatch rather than this function
        // accepting and ignoring them.
    } = args;

    let (explicit, langs) = requested_backends(&language, error_format);

    // The positional is shorthand for the first statechart, so a set may
    // be written either way round; order is input order in both cases.
    let scxml_paths: Vec<PathBuf> = scxml
        .into_iter()
        .chain(scxml_set)
        .map(PathBuf::from)
        .collect();
    let forge_paths: Vec<PathBuf> = forge.into_iter().map(PathBuf::from).collect();
    let scxml_refs: Vec<&Path> = scxml_paths.iter().map(|p| p.as_path()).collect();
    let forge_refs: Vec<&Path> = forge_paths.iter().map(|p| p.as_path()).collect();

    // Parsed once, before the per-backend loop: a malformed topology is
    // fatal regardless of backend, and `orchestrate` refuses it at the
    // same point.
    let deploy_cfg = deploy
        .as_deref()
        .map(|p| load_deploy_config(p, error_format));

    let mut report = GenerateReport {
        deploy_facts: deploy_cfg.as_ref().map(|cfg| sce_build::DeployFacts {
            static_analyzer: cfg.build.as_ref().and_then(|b| b.static_analyzer),
        }),
        ..GenerateReport::default()
    };
    let mut needs_script_engine = false;
    let mut needs_event_scheduler = false;
    for path in &scxml_paths {
        let Some(facts) = scxml_host_requirement_facts(&path.to_string_lossy()) else {
            continue;
        };
        needs_script_engine |= facts.needs_script_engine;
        needs_event_scheduler |= facts.needs_event_scheduler;
        for cause in facts.script_engine_causes {
            if !report.script_engine_causes.contains(&cause) {
                report.script_engine_causes.push(cause);
            }
        }
    }
    report.needs_script_engine = Some(needs_script_engine);
    report.needs_event_scheduler = Some(needs_event_scheduler);

    let options = sce_build::ForgeCompileOptions {
        go_module_prefix,
        const_fold_budget,
        include_dirs: include_dir.iter().map(PathBuf::from).collect(),
        ..Default::default()
    };

    let mut verdicts: Vec<LanguageVerdict> = Vec::new();
    for lang in &langs {
        let template_dir = sce_build::find_template_dir_for(*lang);
        let outcome = sce_build::compile_scxml_with_imports(
            &scxml_refs,
            &forge_refs,
            &template_dir,
            *lang,
            &options,
            deploy_cfg.as_ref(),
        )
        .map(|_| ());
        record_backend_outcome(
            &mut verdicts,
            *lang,
            explicit,
            SweepAxis::ByStage,
            error_format,
            outcome,
        );
    }

    outln!(
        "{}",
        build_manifest(&report, ManifestKind::Check, Some(verdicts)).to_line()
    );
}

/// `sce-codegen check` — every verdict `generate` would reach, nothing
/// written.
///
/// Runs the real pipeline rather than a validation subset: the same
/// parser, the same validators, and the same per-backend codegen, whose
/// in-memory `GeneratedOutput` is then dropped instead of written. A
/// subset would answer a different question than the one the operator
/// asked — "does this parse" instead of "would this generate" — and the
/// two diverge exactly where template rendering does.
///
/// Writing nothing is structural, not a policy this function enforces
/// by being careful: every file `generate` produces is written by the
/// CLI from a `GeneratedOutput` the library returned, so a code path
/// that never calls a write helper cannot emit. The library's compile
/// entry points touch no filesystem.
///
/// The same holds one level up. A document set is compiled by
/// [`sce_build::compile_scxml_with_imports`], the entry point
/// `orchestrate` calls, which returns every artifact in memory and
/// leaves the writes to its caller — so [`cmd_check_document_set`] is
/// `cmd_orchestrate` with the write loop absent rather than a second
/// validation path that has to be kept in agreement with it.
fn cmd_check(args: CheckArgs, error_format: ErrorFormat) {
    // A lone document is checked against `generate`; anything that
    // needs a cross-doc registry or a deploy topology is checked
    // against `orchestrate`. Routing on the invocation shape keeps each
    // route mirroring exactly one producer — a single route would have
    // to agree with both, and the two producers do not agree with each
    // other about what a document set means.
    if !args.scxml_set.is_empty() || !args.forge.is_empty() || args.deploy.is_some() {
        cmd_check_document_set(args, error_format);
        return;
    }
    let CheckArgs {
        scxml,
        scxml_set: _,
        forge: _,
        deploy: _,
        language,
        include_dir,
        strict_unresolved,
        lint,
        go_module_prefix,
        const_fold_budget,
        no_std,
    } = args;
    // The router above leaves only the single-document shape here, and
    // clap's `required_unless_present` guarantees the positional is the
    // one that carries it.
    let scxml_path: &str = scxml
        .as_deref()
        .expect("clap requires the positional when no --scxml is given");

    let (explicit, langs) = requested_backends(&language, error_format);

    let scxml_content = fs::read_to_string(scxml_path).unwrap_or_else(|e| {
        error_format.emit_and_exit(
            &CliError::ReadInput {
                path: scxml_path.to_string(),
                source: e,
            },
            "",
        )
    });

    let mut report = GenerateReport::default();
    let mut verdicts: Vec<LanguageVerdict> = Vec::new();

    match sce_build::classify_document(&scxml_content) {
        sce_build::Pipeline::Forge => {
            let input_stem = Path::new(scxml_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
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

            // Expand first. The statechart arm reaches the expander
            // through `Parser::parse_file`; this arm had no equivalent,
            // so the same command answered differently about whether
            // templates exist depending on the document's kind.
            let scxml_content = match sce_build::parser::expand_preprocessors(
                &scxml_content,
                scxml_path,
                Some(base_dir),
                &include_dir.iter().map(PathBuf::from).collect::<Vec<_>>(),
            ) {
                Ok((expanded, _map, _deps)) => expanded,
                Err(e) => error_format.emit_forge_and_exit(&e),
            };

            // Document axis: a parse failure is fatal regardless of
            // which backends were asked for.
            let parsed =
                match sce_build::forge::parser::parse_forge_with_imports(&scxml_content, doc_label)
                {
                    Ok(Some(p)) => p,
                    Ok(None) => {
                        error_format.emit_forge_and_exit(&sce_build::forge::error::Located::new(
                            sce_build::forge::error::ValidationError::WrongPipeline {
                                kind: sce_build::forge::model::ForgeKind::Statechart,
                                pipeline: sce_build::Pipeline::Forge,
                            }
                            .into(),
                            doc_label.diagnostic_label,
                            None,
                            None,
                        ));
                    }
                    Err(e) => error_format.emit_forge_and_exit(&e),
                };

            for lang in &langs {
                let forge_opts = sce_build::ForgeCompileOptions {
                    go_module_prefix: go_module_prefix.clone(),
                    const_fold_budget,
                    ..Default::default()
                };
                let outcome = sce_build::compile_forge_from_parsed(
                    &parsed,
                    doc_label,
                    *lang,
                    base_dir,
                    &forge_opts,
                )
                .map(|_| ());
                record_backend_outcome(
                    &mut verdicts,
                    *lang,
                    explicit,
                    SweepAxis::BackendOnly,
                    error_format,
                    outcome,
                );
            }
            // Forge kinds are stateless by construction — no script
            // engine is reachable from them, and nothing schedules or
            // drives a child session, so `step()` suffices.
            report.needs_script_engine = Some(false);
            report.needs_event_scheduler = Some(false);
        }
        sce_build::Pipeline::Scxml => {
            let mut parser = SCXMLParser::new()
                .with_include_dirs(include_dir.iter().map(PathBuf::from).collect());
            let mut model = match parser.parse_file(scxml_path) {
                Ok(m) => m,
                Err(e) => error_format.emit_and_exit(&e, ""),
            };

            if strict_unresolved {
                if let Err(e) = sce_build::unresolved_check::check_strict_unresolved(&model) {
                    error_format.emit_and_exit(&e, "");
                }
            }

            analyzer::analyze(&mut model, scxml_path);

            // §scxml-5.8: a rejected document is a successful run that
            // produced stubs. `check` reports the rejection the same way
            // `generate` does — the difference is only that no stub was
            // written.
            if model.document_rejected {
                report.rejected = Some(RejectedDocument {
                    spec: "W3C SCXML 5.8",
                    name: model.name.clone(),
                });
                report.needs_script_engine = Some(false);
                report.needs_event_scheduler = Some(false);
                for lang in &langs {
                    verdicts.push(LanguageVerdict::ok(
                        sce_build::manifest::language_wire_name(*lang),
                    ));
                }
                outln!(
                    "{}",
                    build_manifest(&report, ManifestKind::Check, Some(verdicts)).to_line()
                );
                return;
            }

            if let Err(located) = analyzer::can_generate_static(&model, scxml_path) {
                error_format.emit_forge_and_exit(&located);
            }

            // Document axis, opt-in: the design-time lints reject legal
            // SCXML, so the operator asks for them. Same call the
            // library entry points make, so a document cannot pass here
            // and fail there.
            if lint {
                if let Err(e) = sce_build::lint_statechart(&model, scxml_path) {
                    error_format.emit_forge_and_exit(&e);
                }
            }

            // SCE Protocol-Synthesis RFC §synth-5-O — not a lint: an IR
            // node reaching codegen without a source coordinate emits a
            // silent SCE-MAP marker, and `check` is contracted to reach
            // the same verdict `generate` does. Always on, both paths.
            if let Err(e) =
                sce_build::forge::provenance::validate_emission_provenance(&model, scxml_path)
            {
                error_format.emit_forge_and_exit(&e);
            }

            if no_std && langs.contains(&Language::Rust) {
                if let Err(err) =
                    sce_build::validate_no_std_compatibility(&model, Path::new(scxml_path))
                {
                    let located =
                        sce_build::forge::error::Located::new(err, scxml_path, None, None);
                    error_format.emit_forge_and_exit(&located);
                }
            }

            resolve_source_path(&mut model, Path::new(scxml_path));

            let input_stem = Path::new(scxml_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            report.needs_script_engine = Some(model.needs_script_engine);
            report.needs_event_scheduler = Some(model.needs_event_scheduler_driving());
            report.script_engine_causes = model
                .script_engine_causes
                .iter()
                .map(|c| c.to_wire())
                .collect();

            for lang in &langs {
                let template_dir = sce_build::find_template_dir_for(*lang);
                let outcome = match lang {
                    Language::Rust => {
                        sce_build::generator::generate(&model, &template_dir, no_std).map(|_| ())
                    }
                    Language::Cpp => {
                        sce_build::generator::generate_cpp(&model, &template_dir, input_stem, None)
                            .map(|_| ())
                    }
                    Language::Kotlin => {
                        sce_build::generator::generate_kotlin(&model, &template_dir, None)
                            .map(|_| ())
                    }
                    Language::Go => {
                        sce_build::generator::generate_go(&model, &template_dir).map(|_| ())
                    }
                    Language::Python => {
                        sce_build::generator::generate_python(&model, &template_dir).map(|_| ())
                    }
                    Language::C11 => {
                        sce_build::generator::generate_c11(&model, &template_dir, input_stem, None)
                            .map(|_| ())
                    }
                }
                .map_err(|e| {
                    sce_build::forge::error::Located::new(
                        ForgeError::from(e),
                        scxml_path,
                        None,
                        None,
                    )
                });
                record_backend_outcome(
                    &mut verdicts,
                    *lang,
                    explicit,
                    SweepAxis::BackendOnly,
                    error_format,
                    outcome,
                );
            }
        }
    }

    outln!(
        "{}",
        build_manifest(&report, ManifestKind::Check, Some(verdicts)).to_line()
    );
}

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
        lint,
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
                route: LanguageRoute::Generate,
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
        None => containing_dir(Path::new(scxml_path)),
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

            // Expand before parsing — see the `generate` arm for why
            // this cannot be left to the caller.
            let (scxml_content, preprocessor_deps) = match sce_build::parser::expand_preprocessors(
                &scxml_content,
                scxml_path,
                Some(base_dir),
                &include_dirs.iter().map(PathBuf::from).collect::<Vec<_>>(),
            ) {
                Ok((expanded, _map, deps)) => (expanded, deps),
                Err(e) => error_format.emit_forge_and_exit(&e),
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
                                pipeline: sce_build::Pipeline::Forge,
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
                    // Preprocessor inputs ahead of the import closure.
                    // Both are files this compile read, and a template
                    // edit has to trigger a rebuild exactly as an edit to
                    // an imported document does — otherwise the output
                    // goes stale while the build reports success, which
                    // is the staleness the import entry above prevents.
                    let mut import_deps = preprocessor_deps;
                    import_deps.extend(output.deps);
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
                    report.needs_event_scheduler = Some(false);
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
    // xml/* / validation/* signal consumers dispatch on.
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
        fs::create_dir_all(out).unwrap_or_else(|e| {
            error_format.emit_and_exit(
                &CliError::CreateOutputDir {
                    path: out.display().to_string(),
                    source: e,
                },
                "",
            )
        });
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

        // Each backend names its stub files and their contents; the
        // write, the manifest bookkeeping, and the sourcemap emit are
        // shared below. Previously each arm wrote its own files and
        // none of them recorded what it wrote, so the manifest claimed
        // `"artifacts":[]` for a run that had just written a stub —
        // contradicting SCE_ERROR_CONTRACT.md §10.1, which defines the
        // field as every file written during the run.
        let stubs: Vec<(String, String)> = match lang {
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
                vec![
                    (format!("{input_stem}_sm.h"), header),
                    (
                        format!("{input_stem}_sm.inl"),
                        "// W3C SCXML 5.8: Document rejected\n".to_string(),
                    ),
                ]
            }
            Language::Rust => vec![(
                format!("{input_stem}_sm.rs"),
                format!(
                    "// W3C SCXML 5.8: Document rejected\n\
                     // SCE-MAP: {scxml_basename}:1\n\
                     // This state machine was rejected at parse time.\n"
                ),
            )],
            Language::Kotlin => vec![(
                format!("{input_stem}Sm.kt"),
                format!(
                    "// W3C SCXML 5.8: Document rejected\n\
                     // SCE-MAP: {scxml_basename}:1\n\
                     package com.sce.generated.{name}\n",
                    name = input_stem
                ),
            )],
            Language::Go => vec![(
                format!("{input_stem}_sm.go"),
                format!(
                    "// W3C SCXML 5.8: Document rejected\n\
                     // SCE-MAP: {scxml_basename}:1\n\
                     package {name}\n",
                    name = input_stem
                ),
            )],
            Language::Python => vec![(
                format!("{input_stem}_sm.py"),
                format!(
                    "# W3C SCXML 5.8: Document rejected\n\
                     # SCE-MAP: {scxml_basename}:1\n"
                ),
            )],
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
                vec![
                    (format!("{input_stem}_sm.h"), header),
                    (format!("{input_stem}_sm.c"), body),
                ]
            }
        };

        for (filename, content) in &stubs {
            let path = out.join(filename);
            write_drift_aware(error_format, &path, content, &drift_ctx);
            report.artifacts.push(path);
        }

        // The document parsed — §5.8 refuses to *generate* it, not to
        // read it — so its symbols resolve and the sidecar is emitted
        // exactly as the `generate-w3c` path already did. Without this
        // the two paths disagreed: the committed W3C trees carry a
        // sourcemap for a rejected document while a standalone
        // `generate` on the same input produced none.
        let mut sourcemap_acc = SymbolAccumulator::new();
        collect_sourcemap_symbols(&model, &mut sourcemap_acc);
        flush_sourcemap(&sourcemap_acc, out, &drift_ctx);

        report.rejected = Some(RejectedDocument {
            spec: "W3C SCXML 5.8",
            name: model.name.clone(),
        });
        report.needs_script_engine = Some(false);
        report.needs_event_scheduler = Some(false);
        emit_generate_manifest(&report);
        return;
    }

    if let Err(located) = analyzer::can_generate_static(&model, scxml_path) {
        // §wire-W5 D3: `can_generate_static` returns the
        // correctly-classified ForgeError directly — `ScxmlSemanticError`
        // for hard semantic violations (top-level script rejected,
        // initial-state names undeclared) and `ValidationDynamicFeatures`
        // for genuine codegen limitations (no initial attribute).
        // It anchors the record itself: the rejections that belong to a
        // node carry that node's line, and only the document-scoped
        // ones arrive with the file alone.
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

    // Document axis, opt-in — see `CheckArgs::lint`. Placed on the same
    // side of `can_generate_static` as the `check` call site so the two
    // subcommands report the same diagnostic for the same document.
    if lint {
        if let Err(e) = sce_build::lint_statechart(&model, scxml_path) {
            error_format.emit_forge_and_exit(&e);
        }
    }

    // SCE Protocol-Synthesis RFC §synth-5-O — see the `check` call site.
    // Runs before `resolve_source_path` so a `None` cannot leak into the
    // marker-emitting templates, matching `lib.rs::compile_model`.
    if let Err(e) = sce_build::forge::provenance::validate_emission_provenance(&model, scxml_path) {
        error_format.emit_forge_and_exit(&e);
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
    // line/col is honest — fabricating `(1, 1)` would misroute consumer
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
        // alongside the per-language SM output. Cross-backend
        // byte-identity is preserved because the symbol table + hashes
        // are language-agnostic.
        //
        // Accumulated rather than written here: the synth-invoke loop
        // below emits further machines into the same directory, and
        // there is one sidecar for all of them.
        let mut sourcemap_acc = SymbolAccumulator::new();
        collect_sourcemap_symbols(&model, &mut sourcemap_acc);

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
            if analyzer::can_generate_static(&child_model, &synthetic_path.to_string_lossy())
                .is_err()
            {
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
            collect_sourcemap_symbols(&child_model, &mut sourcemap_acc);
        }

        // Every machine this invocation emitted into `out_path` is now
        // in the table, so the sidecar covers the whole directory
        // rather than whichever machine was written last.
        flush_sourcemap(&sourcemap_acc, out_path, &drift_ctx);

        report.needs_script_engine = Some(model.needs_script_engine);
        report.needs_event_scheduler = Some(model.needs_event_scheduler_driving());
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
        output_dir,
        suite_package,
        test,
        clean,
        list,
        format_style,
        no_format,
    } = args;
    let language: &str = &language;
    let registry = registry.as_deref();
    let resources = resources.as_deref();
    let output_dir = output_dir.as_deref();
    let single_test = test.as_deref();
    let format_style = format_style.as_deref();

    let lang: Language = language.parse().unwrap_or_else(|_| {
        cli_exit(CliError::UnknownLanguage {
            lang: language.to_string(),
            route: LanguageRoute::GenerateW3c,
        })
    });

    // Resolve project root
    let project_root = find_project_root();
    let resources_dir = resources
        .map(PathBuf::from)
        .unwrap_or_else(|| project_root.join("resources"));
    let registry_file = registry
        .map(PathBuf::from)
        .unwrap_or_else(|| project_root.join(sce_build::w3c_registry::W3C_REGISTRY_RELATIVE_PATH));

    // Inputs were already retargetable through `--registry` and
    // `--resources`; the output root was not, so a caller outside this
    // repository could enumerate and validate the suite but had nowhere
    // to put what it generated.
    let output_root = output_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| project_root.clone());
    if output_dir.is_some() {
        // Only when the caller named one: with the default root the two
        // are the same directory and the extra candidate would say
        // nothing.
        let _ = OUTPUT_ROOT.set(output_root.clone());
    }

    let packaging = resolve_suite_packaging(
        lang,
        suite_package.as_deref(),
        &project_root,
        &output_root,
        output_dir.is_some(),
    );

    let backend: Box<dyn W3cBackend> = match lang {
        Language::Rust => Box::new(RustBackend::new(&output_root, &packaging)),
        Language::Go => Box::new(GoBackend::new(&output_root, &packaging)),
        Language::Kotlin => Box::new(KotlinBackend::new(&output_root, &packaging)),
        Language::Cpp => Box::new(CppBackend::new(&output_root)),
        Language::Python => Box::new(PythonBackend::new(&output_root, &packaging)),
        // `lang` holds the backend the caller named, not prose about
        // it: `actual` on the wire then means the same kind of value
        // here as on every other language refusal, and the reason the
        // route excludes it rides the route's own exclusion note.
        Language::C11 => cli_exit(CliError::UnsupportedLanguage {
            lang: Language::C11.canonical_name().to_string(),
            route: LanguageRoute::GenerateW3c,
        }),
    };

    // C++ formatter: created once and reused for all generated tests.
    let cpp_formatter = create_cpp_formatter(lang, format_style, no_format);

    generate_w3c_unified(
        backend.as_ref(),
        &resources_dir,
        &registry_file,
        single_test,
        clean,
        list,
        &cpp_formatter,
    );
}

/// Decide what the emitted suite calls itself and whether it has to
/// arrive buildable.
///
/// Two facts come out of this, and keeping them together is deliberate
/// — every refusal below is about their combination. A run writing
/// into this repository is a regeneration of the committed trees,
/// whose build files are hand-authored and fix the name; a run writing
/// anywhere else is producing a package that has to stand on its own.
fn resolve_suite_packaging(
    lang: Language,
    suite_package: Option<&str>,
    project_root: &Path,
    output_root: &Path,
    output_dir_named: bool,
) -> SuitePackaging {
    // Compared after realising the directory, because an output root
    // that does not exist yet canonicalises to nothing and would
    // compare unequal to every path including itself.
    let standalone = output_dir_named && {
        if let Err(e) = fs::create_dir_all(output_root) {
            cli_exit(CliError::CreateOutputDir {
                path: output_root.display().to_string(),
                source: e,
            });
        }
        let resolved = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        resolved(output_root) != resolved(project_root)
    };

    let refuse = |detail: String| -> ! { cli_exit(CliError::InvalidSuitePackage { detail }) };

    let identity = match suite_package {
        Some(name) => {
            // Applicability before spelling: a caller naming a suite for
            // a backend that has none should be told that, not told
            // their perfectly good name is malformed.
            let parsed = SuiteIdentity::parse(lang, name)
                .unwrap_or_else(|e: SuiteIdentityError| refuse(e.to_string()));
            if !standalone {
                refuse(format!(
                    "naming a suite only applies to a tree written outside this repository. \
                     This run writes into {}, whose build files are committed and already fix \
                     the name to '{}'; renaming the emitted sources would leave them naming a \
                     package that does not exist. Pass --output-dir <DIR> to emit a suite of \
                     your own.",
                    project_root.display(),
                    SuiteIdentity::for_language(lang)
                        .expect("applicability established above")
                        .name(),
                ));
            }
            Some(parsed)
        }
        // Silence is right when the caller did not ask: the backends
        // that name no suite simply have nothing to resolve.
        None => SuiteIdentity::for_language(lang).ok(),
    };

    SuitePackaging {
        identity,
        standalone: standalone.then(|| StandaloneSuite {
            package_root: output_root.to_path_buf(),
            sce_root: project_root.to_path_buf(),
        }),
    }
}

fn find_project_root() -> PathBuf {
    // `SCE_WORKSPACE_ROOT` first, for the same reason
    // [`locate_workspace_root`] honours it: `CARGO_MANIFEST_DIR` is baked in
    // at compile time, so a binary built in one checkout and run in another
    // resolves to the one that BUILT it. That is right for a vendored
    // consumer and wrong for every other reader, and this resolver decides
    // where the W3C trees are WRITTEN — so the failure is not a bad lookup
    // but an escape. Measured: running the regeneration procedure inside a
    // linked git worktree (whose `.git` is a file, so no other layer
    // corrects the root) rewrote nine files in the original checkout while
    // reporting success in the worktree, and the paths it embedded in the
    // generated Kotlin were absolute into that other tree.
    //
    // The variable was already the documented escape hatch; it simply did
    // not reach this resolver, which is why pinning it did not help.
    let marker = Path::new(sce_build::w3c_registry::W3C_REGISTRY_RELATIVE_PATH);
    let pinned = std::env::var("SCE_WORKSPACE_ROOT").ok().and_then(|p| {
        let candidate = PathBuf::from(&p);
        if candidate.join(marker).exists() {
            return Some(fs::canonicalize(&candidate).unwrap_or(candidate));
        }
        eprintln!(
            "sce-codegen: SCE_WORKSPACE_ROOT '{p}' does not contain {} — ignoring it",
            marker.display(),
        );
        None
    });

    // Try CARGO_MANIFEST_DIR ancestor, then CWD
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = crate_dir.join("..");
    // The marker is the conformance registry, not `tests/CMakeLists.txt`:
    // probing for the build script made a CMake-less checkout unable to
    // resolve a root at all, which is the same coupling the registry
    // format itself carried.
    let root = if let Some(pinned) = pinned {
        pinned
    } else if candidate.join(marker).exists() {
        fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.to_path_buf())
    } else {
        let cwd = std::env::current_dir().expect("Cannot get CWD");
        if !cwd.join(marker).exists() {
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

/// Load the registered tests from the W3C conformance registry.
///
/// The registry used to be `tests/CMakeLists.txt`, read back with a
/// regex over `sce_generate_static_w3c_test(...)` calls. Reading a build
/// script as data made CMake the source of truth for a fact that is not
/// a build fact, and it meant a repository vendoring SCE without CMake
/// could not enumerate the fixture set at all. The catalog it reads now
/// is the same shape the forge conformance harness has used all along.
fn load_w3c_registry(registry_file: &Path) -> BTreeMap<String, TestInfo> {
    let registry = sce_build::w3c_registry::W3cRegistry::load(registry_file).unwrap_or_else(|e| {
        cli_exit(CliError::ScxmlGenerate {
            stage: "w3c-registry",
            detail: e.to_string(),
        })
    });
    registry
        .fixtures()
        .iter()
        .map(|f| {
            (
                f.id.clone(),
                TestInfo {
                    test_type: f.harness.clone(),
                    comment: f.summary.clone(),
                },
            )
        })
        .collect()
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
    ///
    /// Takes the model because the artifact it writes is a generated
    /// file like any other: it carries a drift header and therefore a
    /// source-traceability marker, and a marker needs the machine's real
    /// location to name.
    fn post_write_parent(
        &self,
        _test_id: &str,
        _test_mod_dir: &Path,
        _input_stem: &str,
        _model: &SCXMLModel,
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

    /// Files that make the emitted tree a package in its own right:
    /// the build manifest, the module root, the harness the generated
    /// tests call into.
    ///
    /// Absolute paths, so a backend can put them wherever its language
    /// expects rather than beneath one of the two generated-code roots.
    /// Empty for an in-repo regeneration — the repository already
    /// carries hand-authored ones — and empty for a backend whose
    /// suite is not a package `sce-codegen` can describe.
    ///
    /// Deliberately separate from the generated-code path: none of
    /// these is derived from an SCXML document, so none carries a
    /// drift header or an `SCE-MAP:` marker, and the traceability
    /// walker skips them for exactly that reason. This method does not
    /// implement the drift contract — it names files that sit outside
    /// it — so it cites no section.
    fn suite_support_files(&self) -> Vec<(PathBuf, String)> {
        Vec::new()
    }

    /// Whether the tree being written is a suite of the caller's own
    /// rather than this repository's committed one.
    ///
    /// Decides one thing beyond the support files: whether a
    /// single-fixture run still writes the module index. In this
    /// repository it must not — the committed index enumerates every
    /// fixture and a one-fixture run would truncate it to one. In an
    /// emitted suite the index describes what the run produced, so a
    /// one-fixture suite listing one fixture is the correct and
    /// complete answer, and withholding it leaves a package whose
    /// crate root names a module that does not exist.
    fn is_standalone_suite(&self) -> bool {
        false
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
    sourcemap_acc: &mut SymbolAccumulator,
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

        if analyzer::can_generate_static(&child_model, &label).is_err() {
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

        // The child's symbols belong in the same directory sidecar as
        // the parent's — one `sce_sourcemap.json` describes the whole
        // emitted directory, not just whichever machine wrote it last.
        collect_sourcemap_symbols(&child_model, sourcemap_acc);

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
    registry_file: &Path,
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

    // Named for what it is read from. The line used to say "C++ test
    // registry" from when the registry was `tests/CMakeLists.txt` and
    // C++ was the only backend it scheduled; it is printed by every
    // backend, and the registry is now a language-neutral catalog.
    let registered = load_w3c_registry(registry_file);
    outln!("W3C conformance registry: {} fixtures", registered.len());

    if list {
        for (tid, info) in &registered {
            let scxml = find_scxml(resources_dir, tid);
            let status = if scxml.is_some() { "OK" } else { "MISSING" };
            let comment_trunc: String = info.comment.chars().take(70).collect();
            outln!(
                "  {tid:6} [{:9}] {status} -- {comment_trunc}",
                info.type_str()
            );
        }
        return;
    }

    let test_ids: Vec<String> = if let Some(tid) = single_test {
        vec![tid.to_string()]
    } else {
        let mut ids: Vec<String> = registered.keys().cloned().collect();
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
                if analyzer::can_generate_static(&model, scxml_str).is_err()
                    && !model.document_rejected
                {
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
                        backend.post_write_parent(
                            test_id,
                            &test_mod_dir,
                            input_stem,
                            &model,
                            &drift_ctx,
                        );

                        // SCE Protocol-Synthesis RFC §synth-5-O — sourcemap
                        // JSON sidecar. Byte-identical across backends
                        // for the same SCXML input. Accumulated so the
                        // child pass below lands in the same sidecar.
                        let mut sourcemap_acc = SymbolAccumulator::new();
                        collect_sourcemap_symbols(&model, &mut sourcemap_acc);

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
                                &mut sourcemap_acc,
                            );
                        }

                        // One sidecar per emitted directory, covering
                        // every machine that landed in it.
                        flush_sourcemap(&sourcemap_acc, &test_mod_dir, &drift_ctx);

                        // Detect pass state and generate test file (if backend supports it)
                        if backend.generates_test_files() {
                            let pass_state = detect_pass_state(&model);
                            if let Some(ref pass) = pass_state {
                                let machine = to_pascal_case(input_stem);
                                let metadata = read_metadata(resources_dir, test_id);
                                let test_type = registered
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
    if single_test.is_none() || backend.is_standalone_suite() {
        let mut all_ids: Vec<String> = generated_static
            .iter()
            .chain(generated_script.iter())
            .cloned()
            .collect();
        all_ids.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
        backend.finalize(&all_ids, &drift_ctx);
    }

    // A suite of the caller's own arrives buildable or it does not
    // arrive at all: the generated tests name a package, and the
    // package is these files. Outside the `single_test` guard above,
    // because unlike the module index they do not enumerate fixtures —
    // a one-fixture suite needs its manifest and its harness exactly as
    // much as a full one.
    for (path, contents) in backend.suite_support_files() {
        let parent = containing_dir(&path);
        if let Err(e) = fs::create_dir_all(&parent) {
            cli_exit(CliError::CreateOutputDir {
                path: parent.display().to_string(),
                source: e,
            });
        }
        write_or_exit(current_error_format(), &path, contents);
        outln!("  Suite support: {}", path.display());
    }

    // Summary
    let total_generated = generated_static.len() + generated_script.len();
    outln!("\n{}", "=".repeat(60));
    outln!("{} W3C Test Generation Summary", backend.language_name());
    outln!("{}", "=".repeat(60));
    outln!("  Generated (pure static):    {}", generated_static.len());
    outln!("  Generated (script engine):  {}", generated_script.len());
    outln!("  Generated (total):          {total_generated}");
    outln!("  Skipped:                    {}", skipped.len());
    outln!("  Failed:                     {}", failed.len());
    if stale_removed > 0 {
        outln!("  Stale removed:              {stale_removed}");
    }
    outln!("  Total:                      {}", test_ids.len());

    if !skipped.is_empty() {
        outln!("\nSkipped:");
        for (tid, reason) in &skipped {
            outln!("  {tid}: {reason}");
        }
    }

    if !failed.is_empty() {
        outln!("\nFailed tests:");
        for (tid, reason) in &failed {
            outln!("  {tid}: {reason}");
        }
    }

    if total_generated > 0 {
        outln!(
            "\nGenerated SM classes: {}",
            backend.sm_output_base().display()
        );
        if !backend.test_in_sm_dir() {
            outln!(
                "Generated test classes: {}",
                backend.test_output_dir().display()
            );
        }
        outln!(
            "\nGenerated test IDs (static): {}",
            generated_static.join(" ")
        );
        if !generated_script.is_empty() {
            outln!(
                "Generated test IDs (script): {}",
                generated_script.join(" ")
            );
        }
    }

    if !failed.is_empty() {
        // Per-test failures are already printed above in human mode.
        // In JSON mode we still emit a single structured summary so
        // consumers see a record (not just a silent non-zero exit) and
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
    /// Crate the generated integration tests name. They live outside
    /// the crate they exercise, so `use`-ing it is not optional.
    suite: SuiteIdentity,
    /// Set when the emitted crate has to stand on its own.
    standalone: Option<StandaloneSuite>,
}

impl RustBackend {
    const RELATIVE_ROOT: &'static str = "backends/rust/tests";

    fn new(output_root: &Path, packaging: &SuitePackaging) -> Self {
        let tests_crate = output_root.join(Self::RELATIVE_ROOT);
        Self {
            sm_base: tests_crate.join("src/generated"),
            test_dir: tests_crate.join("tests"),
            tmpl_dir: sce_build::find_template_dir_for(Language::Rust),
            suite: packaging.identity().clone(),
            standalone: packaging.standalone_at(Self::RELATIVE_ROOT),
        }
    }

    /// Cargo manifest for a suite standing on its own.
    ///
    /// The SCE dependencies are named by path into the checkout that
    /// generated the suite. That is a function of the run's inputs —
    /// `--workspace-root` / `SCE_WORKSPACE_ROOT` / the registry walk —
    /// and never of where the output landed, so two runs writing to
    /// different roots still emit the same manifest.
    ///
    /// The empty `[workspace]` table is not decoration: without it a
    /// suite emitted anywhere beneath a Cargo workspace is claimed by
    /// that workspace, and cargo refuses to build a package its
    /// workspace root does not list.
    fn cargo_manifest(&self, standalone: &StandaloneSuite) -> String {
        let sce = standalone.sce_root.display();
        format!(
            "# GENERATED -- DO NOT EDIT (sce-codegen generate-w3c)\n\
             #\n\
             # W3C SCXML 1.0 conformance suite. The SCE packages below are\n\
             # named by path into the checkout this suite was generated from;\n\
             # re-point them if that checkout moves.\n\
             \n\
             [workspace]\n\
             \n\
             [package]\n\
             name = \"{name}\"\n\
             version = \"0.0.0\"\n\
             edition = \"2021\"\n\
             rust-version = \"1.75\"\n\
             publish = false\n\
             \n\
             [dependencies]\n\
             sce-rust-runtime = {{ path = \"{sce}/backends/rust/runtime\", \
             default-features = false, features = [\"http-send\"] }}\n\
             sce-rust-lua = {{ path = \"{sce}/backends/rust/lua\" }}\n\
             linkme = \"0.3\"\n\
             log = \"0.4\"\n\
             reqwest = {{ version = \"0.12\", features = [\"blocking\"] }}\n\
             serde_json = \"1\"\n",
            name = self.suite.name(),
        )
    }

    /// Crate root. Names the two modules the generated integration
    /// tests reach through, and nothing else: this repository's own
    /// `lib.rs` also declares `integration`, a hand-curated tree the
    /// W3C generator never writes and an emitted suite therefore does
    /// not have.
    fn crate_root(&self) -> String {
        format!(
            "// GENERATED -- DO NOT EDIT (sce-codegen generate-w3c)\n\
             \n\
             //! W3C SCXML 1.0 conformance suite generated by `sce-codegen`.\n\
             //!\n\
             //! - [`generated`]: one module per fixture, each holding the\n\
             //!   state machine compiled from that fixture's SCXML.\n\
             //! - [`harness`]: the runner the generated integration tests in\n\
             //!   `tests/` call into.\n\
             //!\n\
             //! ```bash\n\
             //! cargo test -p {name}\n\
             //! ```\n\
             \n\
             pub mod generated;\n\
             pub mod harness;\n",
            name = self.suite.name(),
        )
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
        model: &SCXMLModel,
        drift_ctx: &DriftContext,
    ) {
        // Suppressions live on the generated `*_sm.rs` itself (see
        // `state_machine.rs.jinja2` header comment); the parent mod.rs no
        // longer needs to redundantly wrap the declaration in `#[allow(...)]`.
        //
        // §synth-5-O traceability — `write_if_changed_drift_aware` prepends the
        // §synth-6.2.6 header, so the ownership-boundary walker requires this
        // file to carry at least one `SCE-MAP:` marker line. It renders from
        // `rust/module_index.rs.jinja2` so the marker names the `_machine`
        // symbol at the machine's own location; the string this replaced was
        // written here by hand and cited line 1 of the SCXML unconditionally,
        // which is the XML declaration, not the `<scxml>` element.
        let mod_content = match sce_build::generator::generate_rust_module_index(
            model,
            &self.tmpl_dir,
            input_stem,
        ) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Failed to render mod.rs for {input_stem}: {e}");
                return;
            }
        };
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
        let timeout_secs = if test_type == "scheduled" || test_type == "http" {
            5
        } else {
            3
        };
        // An integration test compiles as its own crate, so every path
        // into the suite is spelled from outside it. `suite` is what
        // the caller named the suite, defaulting to this repository's
        // own crate — before it was an input, the literal here made the
        // emitted tests compile only inside a checkout carrying that
        // one name.
        let suite = self.suite.rust_module_path();
        // Engine DI parity: instantiate LuaEngine per-test and pass it
        // to `Policy::new(engine)` instead of registering a process-global singleton.
        let policy_ctor = if needs_script {
            format!(
                "    let script_engine: std::sync::Arc<dyn sce_rust_runtime::IScriptEngine> = \
                 std::sync::Arc::new(sce_rust_lua::LuaEngine::new());\n\
                 \x20   {{POLICY_BINDING}} = {suite}::generated::test"
            )
        } else {
            format!("    {{POLICY_BINDING}} = {suite}::generated::test")
        };
        let policy_args = if needs_script {
            "(script_engine)"
        } else {
            "()"
        };
        let is_http = test_type == "http" && uses_http;
        let http_setup = if is_http {
            format!("    {suite}::harness::setup_http_test(&mut engine);\n")
        } else {
            String::new()
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
            format!(
                "\x20   policy.set_basic_http_access_uri({suite}::harness::HTTP_TEST_SERVER_URL);\n"
            )
        } else {
            String::new()
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
             \x20       {suite}::generated::test{test_id}::{machine_name}State::{pass_variant},\n\
             \x20       \"Test {test_id} reached wrong final state\"\n\
             \x20   );\n\
             }}\n"
        )
    }

    fn test_filename(&self, test_id: &str, _input_stem: &str) -> String {
        format!("test_{test_id}.rs")
    }

    fn is_standalone_suite(&self) -> bool {
        self.standalone.is_some()
    }

    fn suite_support_files(&self) -> Vec<(PathBuf, String)> {
        let Some(standalone) = self.standalone.as_ref() else {
            return Vec::new();
        };
        let root = &standalone.package_root;
        vec![
            (root.join("Cargo.toml"), self.cargo_manifest(standalone)),
            (root.join("src/lib.rs"), self.crate_root()),
            // The committed harness itself, compiled into the
            // generator, so an emitted suite cannot carry a stale copy.
            // The only edit is the suite's own name, which the harness
            // spells in the usage example it documents.
            (
                root.join("src/harness.rs"),
                self.suite
                    .rewrite_rust_source(sce_build::w3c_suite::RUST_HARNESS_SOURCE),
            ),
        ]
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
        outln!("Cleaned Rust generated files");
    }
}

// ── GoBackend ──────────────────────────────────────────────────

struct GoBackend {
    sm_base: PathBuf,
    tmpl_dir: PathBuf,
    /// Module the generated tests import their harness from.
    suite: SuiteIdentity,
    /// Set when the emitted module has to stand on its own.
    standalone: Option<StandaloneSuite>,
}

impl GoBackend {
    const RELATIVE_ROOT: &'static str = "backends/go/tests";

    fn new(output_root: &Path, packaging: &SuitePackaging) -> Self {
        let tests_module = output_root.join(Self::RELATIVE_ROOT);
        Self {
            sm_base: tests_module.join("generated"),
            tmpl_dir: sce_build::find_template_dir_for(Language::Go),
            suite: packaging.identity().clone(),
            standalone: packaging.standalone_at(Self::RELATIVE_ROOT),
        }
    }

    /// Module file for a suite standing on its own.
    ///
    /// The SCE modules keep the import paths their own sources declare
    /// — a Go package's import path is baked into every file that
    /// imports it — and are redirected to the generating checkout with
    /// `replace`, which is what the committed module file already does
    /// for its siblings. Only the redirect target moves.
    fn go_mod(&self, standalone: &StandaloneSuite) -> String {
        let sce = standalone.sce_root.display();
        format!(
            "// GENERATED -- DO NOT EDIT (sce-codegen generate-w3c)\n\
             //\n\
             // W3C SCXML 1.0 conformance suite. The SCE modules below are\n\
             // redirected into the checkout this suite was generated from;\n\
             // re-point the replace directives if that checkout moves.\n\
             \n\
             module {module}\n\
             \n\
             go 1.22\n\
             \n\
             require (\n\
             \tgithub.com/newmassrael/sce-go-lua v0.0.0\n\
             \tgithub.com/newmassrael/sce-go-runtime v0.0.0\n\
             )\n\
             \n\
             require github.com/Shopify/go-lua v0.0.0-20221004153744-91867de107cf // indirect\n\
             \n\
             replace (\n\
             \tgithub.com/newmassrael/sce-go-lua => {sce}/backends/go/lua\n\
             \tgithub.com/newmassrael/sce-go-runtime => {sce}/backends/go/runtime\n\
             )\n",
            module = self.suite.go_module_path(),
        )
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
        let is_http = test_type == "http" && uses_http;
        let timeout = if test_type == "scheduled" {
            "5 * time.Second"
        } else {
            "3 * time.Second"
        };

        // The harness is a package inside the suite's own module, so
        // the import path is the module path. Before it was an input,
        // the literal here made the emitted tests importable only from
        // a module carrying this repository's name.
        let harness_import = format!("{}/harness", self.suite.go_module_path());

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
             \tscegotest \"{harness_import}\"\n\
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

    fn is_standalone_suite(&self) -> bool {
        self.standalone.is_some()
    }

    fn suite_support_files(&self) -> Vec<(PathBuf, String)> {
        let Some(standalone) = self.standalone.as_ref() else {
            return Vec::new();
        };
        let root = &standalone.package_root;
        vec![
            (root.join("go.mod"), self.go_mod(standalone)),
            // The one remote dependency the SCE Lua module pulls in
            // needs its checksum, and the redirected SCE modules do not
            // — a `replace` onto a filesystem path is verified by the
            // path, not by go.sum. Shipping the committed sums keeps
            // the emitted module verifiable without a network fetch to
            // rebuild them.
            (
                root.join("go.sum"),
                sce_build::w3c_suite::GO_SUM_SOURCE.to_string(),
            ),
            (
                root.join("harness/harness.go"),
                sce_build::w3c_suite::GO_HARNESS_SOURCE.to_string(),
            ),
        ]
    }

    fn clean(&self) {
        if self.sm_base.exists() {
            fs::remove_dir_all(&self.sm_base).ok();
            outln!("Cleaned: {}", self.sm_base.display());
        }
    }
}

// ── KotlinBackend ──────────────────────────────────────────────

struct KotlinBackend {
    sm_base: PathBuf,
    test_dir: PathBuf,
    tmpl_dir: PathBuf,
    /// Package root the generated machines and JUnit classes sit under.
    suite: SuiteIdentity,
    /// Set when the emitted project has to stand on its own.
    standalone: Option<StandaloneSuite>,
}

impl KotlinBackend {
    const RELATIVE_ROOT: &'static str = "backends/kotlin/tests";

    fn new(output_root: &Path, packaging: &SuitePackaging) -> Self {
        let tests_module = output_root.join(Self::RELATIVE_ROOT);
        let suite = packaging.identity().clone();
        // A Kotlin source tree mirrors its package names as
        // directories, so the package root the caller named decides
        // where these two trees live. Deriving both from one accessor
        // is what keeps the `package` clause and the path from becoming
        // two answers.
        let package_dir = suite.kotlin_package_dir();
        Self {
            sm_base: tests_module.join(format!("src/main/kotlin/{package_dir}/generated")),
            test_dir: tests_module.join(format!("src/test/kotlin/{package_dir}/w3c")),
            tmpl_dir: sce_build::find_template_dir_for(Language::Kotlin),
            suite,
            standalone: packaging.standalone_at(Self::RELATIVE_ROOT),
        }
    }

    /// Gradle settings for a suite standing on its own.
    ///
    /// A composite build rather than a dependency on published
    /// artifacts: SCE's Kotlin runtime is not published anywhere a
    /// vendoring consumer could resolve it from, and the checkout that
    /// generated the suite is by construction present. Gradle
    /// substitutes the `com.sce:…` coordinates below for the included
    /// build's own projects, which carry exactly those group and
    /// artifact names.
    fn gradle_settings(&self, standalone: &StandaloneSuite) -> String {
        let sce = standalone.sce_root.display();
        format!(
            "// GENERATED -- DO NOT EDIT (sce-codegen generate-w3c)\n\
             //\n\
             // W3C SCXML 1.0 conformance suite. The SCE Kotlin projects are\n\
             // reached through a composite build into the checkout this suite\n\
             // was generated from; re-point includeBuild if that checkout moves.\n\
             \n\
             pluginManagement {{\n\
             \x20   repositories {{\n\
             \x20       gradlePluginPortal()\n\
             \x20       mavenCentral()\n\
             \x20   }}\n\
             }}\n\
             \n\
             dependencyResolutionManagement {{\n\
             \x20   repositories {{\n\
             \x20       mavenCentral()\n\
             \x20   }}\n\
             }}\n\
             \n\
             rootProject.name = \"{name}\"\n\
             \n\
             includeBuild(\"{sce}\")\n",
            name = self
                .suite
                .kotlin_package_root()
                .rsplit('.')
                .next()
                .expect("a package root has at least one element"),
        )
    }

    /// Gradle build file for a suite standing on its own.
    ///
    /// Restates the versions the committed build reads from the version
    /// catalog, because a catalog is a property of the build that owns
    /// it and an emitted suite owns none. The end-to-end gate compiles
    /// and runs what this describes, so a version that stops resolving
    /// fails there rather than in a consumer's tree.
    fn gradle_build(&self) -> String {
        format!(
            "// GENERATED -- DO NOT EDIT (sce-codegen generate-w3c)\n\
             \n\
             plugins {{\n\
             \x20   kotlin(\"jvm\") version \"2.1.20\"\n\
             }}\n\
             \n\
             group = \"{root}\"\n\
             version = \"0.0.0\"\n\
             \n\
             dependencies {{\n\
             \x20   implementation(\"com.sce:sce-kotlin-runtime:1.0.0\")\n\
             \x20   implementation(\"com.sce:sce-kotlin-rhino:1.0.0\")\n\
             \x20   implementation(\"com.sce:sce-kotlin-lua:1.0.0\")\n\
             \x20   implementation(\"com.sce:sce-kotlin-quickjs:1.0.0\")\n\
             \x20   implementation(\"org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.1\")\n\
             \n\
             \x20   testImplementation(kotlin(\"test\"))\n\
             \x20   testImplementation(\"org.junit.jupiter:junit-jupiter:5.10.2\")\n\
             }}\n\
             \n\
             kotlin {{\n\
             \x20   jvmToolchain(17)\n\
             }}\n\
             \n\
             tasks.test {{\n\
             \x20   useJUnitPlatform()\n\
             \x20   // The default engine is Rhino, which is pure JVM. Selecting\n\
             \x20   // lua or quickjs additionally needs their JNI libraries on\n\
             \x20   // java.library.path, which the SCE build produces.\n\
             \x20   systemProperty(\"junit.jupiter.execution.timeout.default\", \"10s\")\n\
             }}\n",
            root = self.suite.kotlin_package_root(),
        )
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
        // The Kotlin templates spell this repository's package root.
        // Rewriting on the way out rather than parameterising the
        // template keeps `template-hash` — and therefore every
        // committed generated file in every backend — unmoved by a
        // change that generates no new state machine code.
        Ok(vec![(
            format!("{input_stem}Sm.kt"),
            self.suite.rewrite_kotlin_source(&code),
        )])
    }

    fn process_child(
        &self,
        test_id: &str,
        child_name: &str,
        code: String,
        test_mod_dir: &Path,
        drift_ctx: &DriftContext,
    ) {
        let root = self.suite.kotlin_package_root();
        let parent_package = format!("test{test_id}");
        let child_package = child_name.to_lowercase();
        let code = self.suite.rewrite_kotlin_source(&code);
        let fixed_code = code.replace(
            &format!("package {root}.generated.{child_package}"),
            &format!("package {root}.generated.{parent_package}"),
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
        let root = self.suite.kotlin_package_root();
        let parent_package = format!("test{test_id}");
        let child_class = to_pascal_case(child_name);
        let stub = format!(
            "// GENERATED STUB -- child codegen failed (no-op)\n\
             // SCE-MAP: {child_name}.scxml:1\n\
             package {root}.generated.{parent_package}\n\n\
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
        let is_http = test_type == "http" && uses_http;
        let base_class = if is_http {
            "W3CHttpTestBase"
        } else {
            "W3CTestBase"
        };

        // §scxml-6.2: SCHEDULED tests need longer timeout
        let timeout_override = if test_type == "scheduled" {
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

        // The suite's own package root, which the generated JUnit class
        // and the machines it imports both sit under. Before it was an
        // input, the literal here fixed the emitted tree to this
        // repository's package names.
        let root = self.suite.kotlin_package_root();

        format!(
            "// GENERATED -- DO NOT EDIT (sce-codegen)\n\
             // SCE-MAP: {input_stem}.scxml:1\n\
             package {root}.w3c\n\
             \n\
             import {root}.generated.{sm_package}.{sm_class}Event\n\
             import {root}.generated.{sm_package}.{sm_class}State\n\
             import {root}.generated.{sm_package}.{sm_class}StateMachine\n\
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

    fn is_standalone_suite(&self) -> bool {
        self.standalone.is_some()
    }

    fn suite_support_files(&self) -> Vec<(PathBuf, String)> {
        let Some(standalone) = self.standalone.as_ref() else {
            return Vec::new();
        };
        let root = &standalone.package_root;
        let mut files = vec![
            (
                root.join("settings.gradle.kts"),
                self.gradle_settings(standalone),
            ),
            (root.join("build.gradle.kts"), self.gradle_build()),
        ];
        // The hand-authored Kotlin the suite carries: the JUnit base
        // classes every generated test extends, and the BasicHTTP test
        // server one of them drives. Each lands in its own source set
        // under the package root, mirroring where it sits here — which
        // is also why `clean_stale` already names the two in the
        // generated test directory as files it must not remove.
        let module_root = self
            .standalone
            .as_ref()
            .map(|s| s.package_root.clone())
            .expect("guarded above");
        let package_dir = self.suite.kotlin_package_dir();
        for (source_set, path, source) in sce_build::w3c_suite::KOTLIN_SUITE_SOURCES {
            files.push((
                module_root.join(source_set).join(&package_dir).join(path),
                self.suite.rewrite_kotlin_source(source),
            ));
        }
        files
    }

    fn clean(&self) {
        if self.sm_base.exists() {
            fs::remove_dir_all(&self.sm_base).ok();
            outln!("Cleaned: {}", self.sm_base.display());
        }
        for entry in fs::read_dir(&self.test_dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("Test") && name.ends_with(".kt") {
                fs::remove_file(entry.path()).ok();
            }
        }
        outln!("Cleaned test classes in: {}", self.test_dir.display());
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
                    outln!("  Removed stale SM dir: {name}");
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
                    outln!("  Removed stale test: {name}");
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
    fn new(output_root: &Path) -> Self {
        Self {
            output_dir: output_root.join("build/tests/w3c_static_generated"),
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
            outln!("Cleaned: {}", self.output_dir.display());
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
    /// Set when the emitted tree has to stand on its own.
    standalone: Option<StandaloneSuite>,
}

impl PythonBackend {
    const RELATIVE_ROOT: &'static str = "backends/python/tests";

    fn new(output_root: &Path, packaging: &SuitePackaging) -> Self {
        Self {
            sm_base: output_root.join(Self::RELATIVE_ROOT).join("generated"),
            tmpl_dir: sce_build::find_template_dir_for(Language::Python),
            standalone: packaging.standalone_at(Self::RELATIVE_ROOT),
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
        let (max_ms, tick_ms) = if test_type == "scheduled" {
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
        // The runtime is reached through the suite's conftest, which
        // pytest imports before it collects anything beneath it. The
        // wrapper used to insert that path itself, computing it from
        // its own depth below `backends/python/` — a second answer to
        // the same question, and one that named a directory an emitted
        // suite does not have.
        //
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

    fn is_standalone_suite(&self) -> bool {
        self.standalone.is_some()
    }

    fn suite_support_files(&self) -> Vec<(PathBuf, String)> {
        let Some(standalone) = self.standalone.as_ref() else {
            return Vec::new();
        };
        // The committed conftest, with the one line that assumes SCE's
        // directory layout re-pointed at the checkout this suite was
        // generated from. Everything else in it — the W3C echo server,
        // the `setup_http` fixture — is layout-independent and travels
        // verbatim.
        let conftest = sce_build::w3c_suite::rewrite_python_conftest(
            sce_build::w3c_suite::PYTHON_CONFTEST_SOURCE,
            &standalone.sce_root,
        )
        .unwrap_or_else(|detail| cli_exit(CliError::InvalidSuitePackage { detail }));
        vec![(standalone.package_root.join("conftest.py"), conftest)]
    }

    fn clean(&self) {
        if self.sm_base.exists() {
            fs::remove_dir_all(&self.sm_base).ok();
            outln!("Cleaned: {}", self.sm_base.display());
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
            outln!("{description}");
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

    outln!("{json}");
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
    out_stream(|w| sce_build::requirements_report::emit_requirements_ndjson(&model, w));
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
    out_stream(|w| sce_build::unresolved_check::emit_unresolved_ndjson(&model, w));
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
                route: LanguageRoute::GenerateIntegration,
            },
            "",
        )
    });

    // The refusal routes through `CliError` like every other language
    // refusal rather than through a bare `eprintln!` + `exit(2)`: this
    // one restated its own menu in prose, so `--error-format=json`
    // received nothing structured for the one route whose restriction is
    // real, and the sentence was a third copy of the set to keep current.
    let script_suffix = match lang {
        Language::Rust => "",
        Language::Kotlin => "_kotlin",
        Language::Go => "_go",
        Language::Python => "_python",
        _ => error_format.emit_and_exit(
            &CliError::UnsupportedLanguage {
                lang: lang.canonical_name().to_string(),
                route: LanguageRoute::GenerateIntegration,
            },
            "",
        ),
    };

    let project_root = find_project_root();
    let integration_root = project_root.join("integration_resources");

    let stems: Vec<String> = match stem {
        Some(s) => vec![s.to_string()],
        None => {
            let mut v = Vec::new();
            let entries = std::fs::read_dir(&integration_root).unwrap_or_else(|e| {
                cli_exit(CliError::ReadInput {
                    path: integration_root.display().to_string(),
                    source: e,
                })
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
            cli_exit(CliError::ReadInput {
                path: script.display().to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no regen script for this stem",
                ),
            });
        }
        let status = std::process::Command::new("bash")
            .arg(&script)
            .current_dir(&project_root)
            .status()
            .unwrap_or_else(|e| {
                cli_exit(CliError::ReadInput {
                    path: script.display().to_string(),
                    source: e,
                })
            });
        if !status.success() {
            cli_exit(CliError::ScxmlGenerate {
                stage: "generate-integration",
                detail: format!(
                    "{} failed (exit {})",
                    script.display(),
                    status.code().unwrap_or(-1)
                ),
            });
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
            route: LanguageRoute::GenerateConformance,
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
    let resource_dir = sibling_of_containing_dir(Path::new(manifest_path), "resources");
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
    // covers exactly the SCXML inputs the harness asserts against. It
    // is the directory `render_harness` just read from, so the two
    // cannot disagree about which fixtures the header describes.
    let drift_ctx = DriftContext::compute(&resource_dir, None, None);
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
    outln!("Generated conformance harness: {}", out_path.display());
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
    out_bytes(expanded.as_bytes());
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

/// Emit `names` in the requested shape.
///
/// Shared by both catalogs so a build system reading one reads the other
/// the same way: the format flag is about how the caller parses the
/// output, not about which catalog produced it.
fn emit_fixture_names(names: &[&str], format: &str) {
    match format {
        "plain" => {
            for n in names {
                outln!("{n}");
            }
        }
        "cmake" => outln!("{}", names.join(";")),
        "space" => outln!("{}", names.join(" ")),
        other => cli_exit(CliError::InvalidFormatOption {
            value: other.to_string(),
            expected: "plain|cmake|space".into(),
        }),
    }
}

/// List the W3C statechart conformance registry.
///
/// This is the enumeration path a build system uses when it has no JSON
/// parser — the same role `list-fixtures` has always played for the
/// forge catalog, and what lets `tests/CMakeLists.txt` derive its
/// per-test registrations from the registry instead of being the
/// registry.
fn list_w3c_fixtures(
    manifest_path: &str,
    format: &str,
    harness: Option<&str>,
    language: Option<&str>,
    has_test_vectors_only: bool,
) {
    // The forge-only selectors are refused rather than accepted and
    // ignored: this catalog has no per-language product gate and no
    // test-vector sidecar, so honouring them would answer a question
    // the registry cannot express.
    if language.is_some() || has_test_vectors_only {
        cli_exit(CliError::ScxmlGenerate {
            stage: "list-fixtures",
            detail: "--language and --has-test-vectors do not apply to --catalog w3c: \
                     the statechart registry has no per-language product gate and no \
                     test-vector sidecar. Filter with --harness instead."
                .to_string(),
        });
    }
    let registry = sce_build::w3c_registry::W3cRegistry::load(Path::new(manifest_path))
        .unwrap_or_else(|e| {
            cli_exit(CliError::ScxmlGenerate {
                stage: "w3c-registry",
                detail: e.to_string(),
            })
        });
    if let Some(h) = harness {
        // An unknown harness lists nothing, and "nothing" is
        // indistinguishable from "no fixture uses it" — so refuse
        // instead, or a typo in a build script would silently drop a
        // whole registration group.
        if !registry.harnesses.contains_key(h) {
            let known: Vec<&str> = registry.harnesses.keys().map(String::as_str).collect();
            cli_exit(CliError::ScxmlGenerate {
                stage: "list-fixtures",
                detail: format!(
                    "unknown --harness `{h}`; the registry declares {}",
                    known.join(", ")
                ),
            });
        }
    }
    let names: Vec<&str> = match harness {
        Some(h) => registry.ids_with_harness(h),
        None => registry.fixtures().iter().map(|f| f.id.as_str()).collect(),
    };
    emit_fixture_names(&names, format);
}

fn cmd_list_fixtures(
    manifest_path: &str,
    format: &str,
    language: Option<&str>,
    has_test_vectors_only: bool,
    resource_dir: Option<&str>,
    catalog: &str,
    harness: Option<&str>,
) {
    match catalog {
        "forge" => {}
        "w3c" => {
            return list_w3c_fixtures(
                manifest_path,
                format,
                harness,
                language,
                has_test_vectors_only,
            )
        }
        other => cli_exit(CliError::ScxmlGenerate {
            stage: "list-fixtures",
            detail: format!("unknown --catalog `{other}`; expected forge or w3c"),
        }),
    }
    if harness.is_some() {
        cli_exit(CliError::ScxmlGenerate {
            stage: "list-fixtures",
            detail: "--harness applies to --catalog w3c only; the forge catalog has \
                     no harness axis."
                .to_string(),
        });
    }
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
            None => sibling_of_containing_dir(Path::new(manifest_path), "resources"),
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
                route: LanguageRoute::ListFixtures,
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
        None => sibling_of_containing_dir(Path::new(manifest_path), "resources"),
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
    emit_fixture_names(&names, format);
}

// ── Utility functions ───────────────────────────────────────────

/// Record the `// From:` provenance path against the invocation's
/// [`SOURCE_ROOT`] (delegates to lib). Reading the root here rather than
/// threading it keeps every emit site on one source of truth, matching how
/// the error format and workspace-root override are already handled.
fn resolve_source_path(model: &mut SCXMLModel, scxml_path: &Path) {
    // Whichever declared root actually contains the input wins. The
    // output root is tried first because it is the narrower claim: it is
    // set only when the caller named one, and an input under it is one
    // this run wrote, which the project root may not be able to spell
    // relatively at all.
    let root = OUTPUT_ROOT
        .get()
        .filter(|r| path_is_under(scxml_path, r))
        .cloned()
        .or_else(current_source_root);
    sce_build::resolve_source_path(model, scxml_path.to_str().unwrap_or(""), root.as_deref());
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
    fs::create_dir_all(containing_dir(path)).ok();
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
//   `--symbol <NAME>` — direct sourcemap key lookup.
//   `--pc <ADDR>`     — resolve the address to the function symbol
//                        containing it, then look that symbol up.
//   `--hardfault`     — the same resolution for every newline-separated
//                        address on stdin, one record per frame in the
//                        order the dump lists them.
//
// The address->symbol hop reads the ELF **symbol table**, not a DWARF
// line program. The spec's contract is PC -> symbol -> sourcemap ->
// SCXML file:line, and the SCXML coordinates live in the sourcemap, so
// the line program would only re-derive the generated-language line the
// sourcemap already supersedes. `.symtab` carries `st_value` / `st_size`
// for every emitted function, which is exactly the containment test this
// needs, and it survives a `--strip-debug` image — the shape an MCU
// build ships.
//
// ARM Thumb: a PC harvested from a Cortex-M exception frame carries the
// Thumb bit in bit 0, and Thumb function symbols carry it in `st_value`
// too. Both sides are normalised on ARM so an odd address resolves to
// the same function as its even neighbour; without that, every frame in
// a hardfault dump misses.

fn cmd_addr2sce(
    sourcemap_dir: &str,
    symbol: Option<&str>,
    pc: Option<u64>,
    elf: Option<&str>,
    hardfault: bool,
    error_format: ErrorFormat,
) {
    let _ = error_format;
    let (map, map_path) = load_sourcemap(sourcemap_dir);

    // Mode dispatch. Exactly one of the three arrived: the
    // `addr2sce_mode` group on the subcommand makes any other
    // combination a parse failure, reported as `cli/usage`.
    match (symbol, pc, hardfault) {
        (Some(name), None, false) => addr2sce_resolve_symbol(&map, name, &map_path),
        (None, Some(addr), false) => {
            let symbols = addr2sce_load_symbol_table(elf);
            if !addr2sce_resolve_pc(&map, &symbols, addr, &map_path) {
                cli_exit(CliError::QueryNoMatch {
                    tool: "addr2sce",
                    query: format!("pc {addr:#x}"),
                    searched: map_path.display().to_string(),
                });
            }
        }
        (None, None, true) => {
            let symbols = addr2sce_load_symbol_table(elf);
            let mut frames = 0usize;
            let mut unresolved = 0usize;
            for line in std::io::stdin().lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(e) => cli_exit(CliError::ReadInput {
                        path: "<stdin>".to_string(),
                        source: e,
                    }),
                };
                let trimmed = line.trim();
                // Blank lines keep a pasted dump usable verbatim.
                if trimmed.is_empty() {
                    continue;
                }
                frames += 1;
                let pc = addr2sce_parse_stdin_pc(trimmed);
                if !addr2sce_resolve_pc(&map, &symbols, pc, &map_path) {
                    unresolved += 1;
                }
            }
            if frames == 0 {
                cli_exit(CliError::ReadInput {
                    path: "<stdin>".to_string(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "--hardfault read no addresses",
                    ),
                });
            }
            // A partially-resolved stack is a narrative with a hole in
            // it; the frames that did resolve stay on stdout so the
            // operator sees how far the walk got.
            if unresolved > 0 {
                cli_exit(CliError::QueryNoMatch {
                    tool: "addr2sce",
                    query: format!("{unresolved} of {frames} frame(s)"),
                    searched: map_path.display().to_string(),
                });
            }
        }
        // Unreachable by construction: `addr2sce_mode` is
        // `required(true).multiple(false)`, so clap rejects zero modes
        // and two modes alike before this function is entered.
        _ => unreachable!("clap group `addr2sce_mode` admits exactly one mode"),
    }
}

/// A function symbol's address range, normalised for the target's
/// address convention.
struct FunctionSymbol {
    name: String,
    addr: u64,
    size: u64,
}

/// Every sized function symbol in `elf`, address-sorted.
///
/// `elf` is `Some` whenever this is reached: `--pc` and `--hardfault`
/// both carry `requires = "elf"`, so the parser refuses the invocation
/// that would leave it absent. The argument stays optional in the
/// struct because the third mode, `--symbol`, does not take one.
fn addr2sce_load_symbol_table(elf: Option<&str>) -> Vec<FunctionSymbol> {
    use object::{Object, ObjectSymbol};

    let Some(path) = elf else {
        unreachable!("`--pc` / `--hardfault` declare `requires = \"elf\"`")
    };
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => cli_exit(CliError::ReadInput {
            path: path.to_string(),
            source: e,
        }),
    };
    let file = match object::File::parse(&*bytes) {
        Ok(f) => f,
        Err(e) => cli_exit(CliError::ReadInput {
            path: path.to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
        }),
    };
    // On ARM the Thumb bit lives in bit 0 of a function symbol's value
    // and of any PC that reached the CPU in Thumb state. Clearing it on
    // both sides is what makes the two comparable.
    let thumb = matches!(file.architecture(), object::Architecture::Arm);

    let mut symbols: Vec<FunctionSymbol> = file
        .symbols()
        .filter(|s| s.kind() == object::SymbolKind::Text)
        .filter_map(|s| {
            let name = s.name().ok()?;
            if name.is_empty() {
                return None;
            }
            Some(FunctionSymbol {
                name: name.to_string(),
                addr: if thumb { s.address() & !1 } else { s.address() },
                size: s.size(),
            })
        })
        .collect();
    if symbols.is_empty() {
        cli_exit(CliError::ReadInput {
            path: path.to_string(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "carries no function symbols — a fully stripped image cannot \
                 be attributed; keep `.symtab` in the artifact used for triage",
            ),
        });
    }
    symbols.sort_by_key(|s| s.addr);
    symbols
}

/// Parse a PC written with or without the `0x` prefix.
///
/// One parser, two callers with different failure meanings: as a
/// `value_parser` on `--pc` a bad value is a malformed command line
/// (`cli/usage`, via clap), and on a `--hardfault` stdin line it is
/// unusable input (`cli/read-input`). Sharing the function keeps the
/// two spellings of "what counts as an address" from drifting.
fn parse_pc_address(raw: &str) -> Result<u64, String> {
    let text = raw.trim();
    let digits = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => hex,
        // Bare digits are hex too: every tool that prints a stack dump
        // (gdb, a Cortex-M fault handler, objdump) writes hex, and
        // reading `1024` as decimal would resolve the wrong function
        // silently.
        None => text,
    };
    u64::from_str_radix(digits, 16)
        .map_err(|_| format!("'{raw}' is not a hexadecimal program-counter address"))
}

/// Parse one address off a `--hardfault` stdin line.
fn addr2sce_parse_stdin_pc(raw: &str) -> u64 {
    match parse_pc_address(raw) {
        Ok(v) => v,
        Err(detail) => cli_exit(CliError::ReadInput {
            path: "<stdin>".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, detail),
        }),
    }
}

/// Resolve one PC and print its record. Returns false when the address
/// belongs to no function or the function is absent from the sourcemap.
///
/// Containment is `[addr, addr + size)` on the greatest symbol at or
/// below the PC. A symbol with no size (hand-written assembly labels,
/// linker-script anchors) cannot bound anything, so an address past a
/// sized function's end is reported as a miss rather than attributed to
/// whatever came before it — a crash triage that names the wrong state
/// is worse than one that says it cannot tell.
fn addr2sce_resolve_pc(
    map: &sce_build::forge::sourcemap::Sourcemap,
    symbols: &[FunctionSymbol],
    pc: u64,
    map_path: &Path,
) -> bool {
    // No normalisation here. The symbol side already dropped the Thumb
    // bit, so a function begins at its even address and an odd PC
    // inside it lands in the same range; masking the query as well
    // changed no outcome, which a mutation confirmed before the branch
    // was removed.
    let addr = pc;
    let hit = symbols
        .partition_point(|s| s.addr <= addr)
        .checked_sub(1)
        .map(|i| &symbols[i])
        .filter(|s| s.size > 0 && addr - s.addr < s.size);
    let Some(symbol) = hit else {
        eprintln!("addr2sce: pc {pc:#x} is inside no function symbol");
        return false;
    };
    let Some(entry) = map.symbols.get(&symbol.name) else {
        eprintln!(
            "addr2sce: pc {pc:#x} resolved to symbol '{}' which is absent from {}",
            symbol.name,
            map_path.display()
        );
        return false;
    };
    print_lookup_record(
        sce_build::forge::sourcemap::LookupKind::Addr2Sce,
        map_path,
        &symbol.name,
        entry,
    );
    true
}

/// Read and parse the `sce_sourcemap.json` under `dir`.
///
/// The single load path for both lookup directions. Typed through
/// `sourcemap::from_json` rather than `serde_json::Value`: the shape
/// has a struct, and re-stating its field names at each consumer is
/// how one direction ends up understanding a row the other does not.
fn load_sourcemap(dir: &str) -> (sce_build::forge::sourcemap::Sourcemap, PathBuf) {
    let map_path = Path::new(dir).join("sce_sourcemap.json");
    let raw = match fs::read_to_string(&map_path) {
        Ok(s) => s,
        Err(e) => cli_exit(CliError::ReadInput {
            path: map_path.display().to_string(),
            source: e,
        }),
    };
    match sce_build::forge::sourcemap::from_json(&raw) {
        Ok(m) => (m, map_path),
        Err(e) => cli_exit(CliError::ReadInput {
            path: map_path.display().to_string(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()),
        }),
    }
}

/// Emit one lookup hit as an NDJSON line on stdout.
fn print_lookup_record(
    kind: sce_build::forge::sourcemap::LookupKind,
    map_path: &Path,
    symbol: &str,
    entry: &sce_build::forge::sourcemap::SourceSymbol,
) {
    let record = sce_build::forge::sourcemap::SymbolLookupRecord {
        v: sce_build::forge::sourcemap::SYMBOL_LOOKUP_SCHEMA_VERSION,
        kind: kind.as_str(),
        generator: sce_build::GENERATOR_COMMIT,
        sourcemap: map_path.display().to_string(),
        symbol,
        entry,
    };
    outln!("{}", record.to_line());
}

/// Look `symbol` up in the loaded sourcemap and print the resolved
/// SCXML coordinates as a single JSON line on stdout. Returns
/// process exit 0 on a hit, 1 on a miss (so a CI gate using addr2sce
/// to verify symbol presence can fail loudly).
fn addr2sce_resolve_symbol(
    map: &sce_build::forge::sourcemap::Sourcemap,
    symbol: &str,
    map_path: &Path,
) {
    let Some(entry) = map.symbols.get(symbol) else {
        cli_exit(CliError::QueryNoMatch {
            tool: "addr2sce",
            query: format!("symbol '{symbol}'"),
            searched: map_path.display().to_string(),
        })
    };
    print_lookup_record(
        sce_build::forge::sourcemap::LookupKind::Addr2Sce,
        map_path,
        symbol,
        entry,
    );
}

/// `sce-codegen sce2sym` — resolve SCXML coordinates to the symbols
/// they lowered to, across one or more backends' sidecars.
///
/// Exit 0 on at least one hit, 1 when the query matched nothing in any
/// sourcemap. A miss is a failure rather than an empty success for the
/// same reason `addr2sce`'s is: the caller asked where something went,
/// and "nowhere" is an answer a build gate must be able to fail on.
fn cmd_sce2sym(sourcemap_dirs: &[String], query: sce_build::forge::sourcemap::SymbolQuery<'_>) {
    let mut hits = 0usize;
    for dir in sourcemap_dirs {
        let (map, map_path) = load_sourcemap(dir);
        for (symbol, entry) in sce_build::forge::sourcemap::find_symbols(&map, &query) {
            print_lookup_record(
                sce_build::forge::sourcemap::LookupKind::Sce2Sym,
                &map_path,
                symbol,
                entry,
            );
            hits += 1;
        }
    }
    if hits == 0 {
        cli_exit(CliError::QueryNoMatch {
            tool: "sce2sym",
            query: "the query".to_string(),
            searched: format!("{} sourcemap(s)", sourcemap_dirs.len()),
        });
    }
}
