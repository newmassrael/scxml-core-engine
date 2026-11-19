// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Generated-source drift detection per spec §synth-6.2.6
//! (`docs/spec/synth/rfc-sce-protocol-synthesis.md`).
//!
//! Every emitted file carries a header:
//!
//! ```text
//! // SCE-GENERATED — DO NOT EDIT
//! // source-hash: <sha256 of the file's input closure>
//! // source: <document the file is generated from>
//! ```
//!
//! The header states a fact about the file's own inputs and nothing that can
//! change while the file does not. Its digest is the input closure of the
//! documents the file is generated from ([`InputClosure`], ADR 0094): those
//! documents and every file their generation reads, never a document it does
//! not read. The `source:` lines name the documents, one per line, so the
//! closure can be recomputed from the file alone. Drift is judged on content:
//! `sce-codegen … --assert-unchanged` runs the generation in memory and names
//! every file on disk that is not as it would leave it (see
//! [`GeneratedTree`]). `sce-codegen verify <out-dir>` is the narrower check
//! that needs no generation: it recomputes each file's closure and compares
//! its digest with the embedded value, firing `forge/source-hash-mismatch`.
//!
//! ## What the header does not carry
//!
//! - Another document. Until ADR 0094 the digest covered every `**/*.scxml`
//!   under the input root, which for the `static_datamodel` trees was one
//!   fixture directory: an edit to any fixture re-stamped 753 committed
//!   files whose content did not change, and an import outside the root or a
//!   fragment that is not `.scxml` moved output without moving the header.
//!   The directory walk ([`SourceSet`]) stays as the start of the set form,
//!   for a file really generated from every document of a directory.
//! - A template or generator hash. Until 2026-09 it carried a
//!   `template-hash` over the template tree and `Cargo.lock`, which was
//!   wrong in both directions: an edit to the lock or to any template
//!   re-stamped every committed tree with no byte of output changed, while
//!   an edit to the generator's own code changed output without moving it.
//!   A content comparison has neither defect.
//! - A timestamp. `generated-at` was informational only, pinned to `0` in
//!   every committed tree, and when unpinned it made two runs over the same
//!   inputs differ.
//! - The generator's identity. That is a fact about a run, not a file, and
//!   the run's stdout manifest carries it as `generator`
//!   (`SCE_WIRE_CONTRACTS.md` policy 2). Stamped into a committed file it
//!   would change with every generator commit whether the output did or not.
//!
//! [`prepend_or_replace_header`] still recognises the older four-line header,
//! so a file generated before the change is rewritten in the current shape
//! instead of growing a second header.
//!
//! ## Design decisions
//!
//! - Per-file header is emitted verbatim (this module emits one block; see
//!   `render_header`). Python uses `#` comment prefix; everything else uses
//!   `//`.
//! - Hash shape: BTreeMap<PathBuf, sha256(content)> 2-level hash.
//!   Deterministic via BTreeMap iteration order; sub-file drift localization
//!   debugging-friendly because each file's individual digest is recoverable.
//! - Input closure = the seeds and what their generation reads
//!   ([`crate::import_closure::reads`]), optional `deploy.yaml`, and the
//!   limits, each by its raw bytes (pre-XInclude). XInclude expansion is NOT
//!   applied to the bytes hashed — raw on-disk bytes drive the hash.

use crate::generator_witness::{hash_btreemap, hex_encode, sha256_bytes};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Sentinel header banner. First line of every emitted file regardless of
/// backend; consumers detect SCE-generated provenance by matching this
/// prefix (`{comment_prefix} SCE-GENERATED`). The em-dash is intentional
/// per the spec's own spelling.
pub const HEADER_BANNER: &str = "SCE-GENERATED \u{2014} DO NOT EDIT";

/// The key of the digest, as `source-hash: <hex>`.
const SOURCE_HASH_KEY: &str = "source-hash";

/// The key of each document the digest is a closure of, as `source: <key>`,
/// one line per seed (ADR 0094 section 3).
const SEED_KEY: &str = "source";

/// Keys an earlier header shape wrote and this module no longer does — see
/// the module documentation. Recognised only so that replacing a header
/// removes them.
const RETIRED_HEADER_KEYS: [&str; 2] = ["template-hash", "generated-at"];

/// The facts a §synth-6.2.6 header states about the file it heads.
///
/// A struct rather than a bare digest so that a fact added to the header
/// later is added here, and every site that renders, compares or parses a
/// header gets it without changing shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DriftHeader {
    /// Digest of the file's input closure — see [`InputClosure`].
    pub source_hash: [u8; 32],
    /// The documents the file is generated from, as their closure keys them
    /// ([`InputClosure::seeds`]), sorted: the `source:` lines, which let
    /// `sce-codegen verify` recompute the closure from the file alone.
    pub seeds: Vec<String>,
}

impl DriftHeader {
    /// Hex-encoded `source-hash` value (64 lowercase hex chars).
    pub fn source_hex(&self) -> String {
        hex_encode(&self.source_hash)
    }

    /// The header of a file generated from the seeds of `closure`.
    pub fn of(closure: &InputClosure) -> Self {
        Self {
            source_hash: closure.digest(),
            seeds: closure.seeds().to_vec(),
        }
    }
}

/// I/O failure surface raised by hash computation. Stays narrow so callers
/// can attach `forge/source-hash-mismatch` semantics at the wire boundary
/// without an upstream pipeline-stage taxonomy.
#[derive(Debug, thiserror::Error)]
pub enum DriftHashError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The walk descended more directories than [`MAX_DIRECTORY_DESCENTS`].
    ///
    /// Reported rather than truncated. A partial walk folds to a
    /// well-formed digest that describes a subset of the input, and a
    /// header carrying one is unauditable in the same way the empty-input
    /// digest was: nothing downstream can tell it apart from a complete
    /// one.
    #[error(
        "{root}: §6.2.6 source set exceeds {limit} directories — a directory \
         symlink reaching a sibling multiplies the paths under it; re-point \
         --input-root at a tree without the aliasing, or remove it"
    )]
    WalkLimitExceeded { root: PathBuf, limit: usize },

    /// The tree kept changing while it was being read.
    ///
    /// Separated from [`Self::Io`] because the remedy is different in kind.
    /// An `Io` failure says the root is wrong; this one says the root is
    /// right but is somebody else's output directory. Reporting the latter
    /// as "failed to read <some file>" sent readers looking for a missing
    /// file that is, by then, present again.
    #[error(
        "{root}: §6.2.6 source set changed while it was being read ({attempts} \
         attempt(s)) — the digest would describe no actual state of the tree; \
         point --input-root at a directory the build does not write to"
    )]
    RootNotQuiescent { root: PathBuf, attempts: usize },
}

/// Liveness ceiling on directory descents in one walk.
///
/// This is not a size policy — it is the bound that keeps a pathological
/// input from running forever. Because a directory link naming a sibling
/// contributes under each name that reaches it, a tree of such links has a
/// number of root-relative paths exponential in its depth: n levels of k
/// links each name k^n paths, all of them genuinely distinct inputs under
/// the documented rule. Enumerating them is the honest answer and refusing
/// is the honest failure; silently hashing a prefix of them is neither.
///
/// The value only has to sit above every real source tree and stay finite.
/// The largest input root in this workspace holds 201 directories and the
/// whole repository holds roughly 4,800, so a million is ~200x the widest
/// tree here — reachable by link multiplication and by nothing else.
pub const MAX_DIRECTORY_DESCENTS: usize = 1_000_000;

/// Floor on the ceiling, checked when the crate compiles rather than when a
/// test runs. Lowering the bound near real tree sizes would turn a liveness
/// guard into a size policy and start refusing legitimate input, so the
/// constraint belongs to the constant, not to a caller.
const _: () = assert!(MAX_DIRECTORY_DESCENTS >= 100_000);

/// The resolved §synth-6.2.6 source set behind a `source-hash`: every
/// `**/*.scxml` under the input root, plus `deploy.yaml` when supplied,
/// keyed by root-relative path.
///
/// Held as a set rather than folded straight to a digest so a caller can
/// assert [`covers`](Self::covers) — that the document it was asked to
/// generate actually contributed — before embedding the digest in output.
/// Without that check a source set that silently collected nothing still
/// produces a well-formed 64-hex digest (the empty-input sha256), which a
/// downstream drift check cannot distinguish from a successful hash.
#[derive(Clone, Debug)]
pub struct SourceSet {
    root: PathBuf,
    entries: BTreeMap<PathBuf, [u8; 32]>,
    /// Where `deploy.yaml` was read from, when one contributed.
    ///
    /// The map keys it under the canonical name rather than its real
    /// path — the digest must not move when a build passes the same
    /// file from a different directory. [`contributing_paths`] still has
    /// to name the file a build system can watch, so the real location
    /// is kept here instead of being reconstructed from `root`, which it
    /// need not sit under.
    ///
    /// [`contributing_paths`]: Self::contributing_paths
    deploy_yaml: Option<PathBuf>,
    /// The canonical real path of every contributing file — what
    /// [`covers`](Self::covers) asks about. Resolved once, when the bytes
    /// are taken, rather than per question.
    members: BTreeSet<PathBuf>,
}

impl SourceSet {
    /// Source-set rule (§synth-6.2.6): walks `input_root` recursively for
    /// `**/*.scxml` and hashes each file's raw bytes. If `deploy_yaml` is
    /// provided, its raw bytes are included under the canonical key
    /// `"deploy.yaml"`.
    pub fn collect(input_root: &Path, deploy_yaml: Option<&Path>) -> Result<Self, DriftHashError> {
        Self::collect_within(input_root, deploy_yaml, MAX_DIRECTORY_DESCENTS)
    }

    /// [`collect`](Self::collect) with the descent ceiling supplied rather
    /// than taken from [`MAX_DIRECTORY_DESCENTS`]. Exists so the refusal
    /// can be exercised against a tree small enough to build in a test —
    /// reaching the production ceiling for real would need millions of
    /// directories.
    pub(crate) fn collect_within(
        input_root: &Path,
        deploy_yaml: Option<&Path>,
        descent_limit: usize,
    ) -> Result<Self, DriftHashError> {
        let mut entries: BTreeMap<PathBuf, [u8; 32]> = BTreeMap::new();
        // A standard document is generated from the library it belongs to,
        // which lives in the generator rather than under a directory. Its
        // source set is the library's counterpart of a directory walk — the
        // standard documents under the root's `sce:std/...` prefix — and the
        // standard documents they import, under their `sce:std/...` names,
        // and nothing on disk. Not the whole library: a document added to
        // another corner of it moves no hash here.
        if crate::forge::stdlib::names_standard(input_root) {
            let prefix = format!("{}/", input_root.to_string_lossy().trim_end_matches('/'));
            let seeds = crate::forge::stdlib::documents()
                .map(|(name, _)| name)
                .filter(|name| name.starts_with(&prefix))
                .map(PathBuf::from);
            for (name, content) in crate::forge::stdlib::closure(seeds) {
                entries.insert(PathBuf::from(name), sha256_bytes(content.as_bytes()));
            }
            fold_limits(&mut entries);
            return Ok(Self {
                root: input_root.to_path_buf(),
                entries,
                deploy_yaml: None,
                members: BTreeSet::new(),
            });
        }
        walk_filtered(
            input_root,
            input_root,
            &mut entries,
            &|p| p.extension().is_some_and(|e| e == "scxml"),
            descent_limit,
        )?;
        if let Some(deploy) = deploy_yaml {
            let bytes = fs::read(deploy).map_err(|e| DriftHashError::Io {
                path: deploy.to_path_buf(),
                source: e,
            })?;
            entries.insert(PathBuf::from("deploy.yaml"), sha256_bytes(&bytes));
        }
        // A document that imports from SCE's standard library generates from
        // documents that are not under the root: they are embedded in the
        // generator (`forge::stdlib`). The ones it reaches — its
        // `<sce:import>`s of `sce:std/...` names, and theirs in turn — are
        // folded in under those names, so a changed standard document moves
        // the hash of every tree that reaches it, and of no other tree.
        //
        // ⚠ Until 2026-09-28 this folded in the WHOLE library whenever a
        // document's bytes contained `sce:std/` anywhere — a header comment
        // naming the document itself was enough. The committed Mesh trees
        // under stdlib/mesh import nothing from the library by that name, yet
        // every document added anywhere in it re-stamped all three of them.
        let seeds: Vec<PathBuf> = entries
            .keys()
            .filter(|rel| is_a_document(rel))
            .flat_map(|rel| {
                let file = input_root.join(rel);
                let base = file.parent().map(Path::to_path_buf).unwrap_or_default();
                let content = fs::read_to_string(&file).unwrap_or_default();
                crate::forge::stdlib::import_sources(&content)
                    .into_iter()
                    .map(move |src| crate::forge::stdlib::resolve(&base, &src))
                    .filter(|resolved| crate::forge::stdlib::names_standard(resolved))
                    .collect::<Vec<_>>()
            })
            .collect();
        for (name, content) in crate::forge::stdlib::closure(seeds) {
            entries.insert(PathBuf::from(name), sha256_bytes(content.as_bytes()));
        }
        fold_limits(&mut entries);
        let members = entries
            .keys()
            .filter(|rel| is_a_document(rel))
            .filter(|rel| !crate::forge::stdlib::names_standard(rel))
            .map(|rel| canonical_key(&input_root.join(rel)))
            .chain(deploy_yaml.map(canonical_key))
            .collect();
        Ok(Self {
            root: input_root.to_path_buf(),
            entries,
            deploy_yaml: deploy_yaml.map(Path::to_path_buf),
            members,
        })
    }

    /// Folds the set to the `source-hash` value embedded in the header.
    pub fn digest(&self) -> [u8; 32] {
        hash_btreemap(&self.entries)
    }

    /// Root the set was collected from — carried for diagnostics that need
    /// to tell the caller which directory came up short.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Number of contributing documents (`.scxml`, `deploy.yaml`, and the
    /// standard library's documents when the root imports from it).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Is `path` one of the files this set was taken from — so that an
    /// edit to it moves the digest?
    ///
    /// Matched on file identity: the canonical real path, which every
    /// spelling of one file resolves to — a sandbox link name, a
    /// root-relative path, the real path itself. Not on spelling, which
    /// would refuse a link to a member; and not on content, which this
    /// used to be.
    ///
    /// ⚠ Content answered the wrong question. "Are these bytes in the
    /// digest?" is true of any copy with the same bytes, wherever it sits,
    /// and the question the caller asks is whether a later edit to THIS
    /// file will show. A document outside the root that happened to
    /// duplicate one inside it passed, and every edit to it afterwards left
    /// the `source-hash` unmoved — the state `sce-codegen verify` exists
    /// to catch, reached through the check meant to prevent it.
    ///
    /// Returns `false` when `path` does not exist: a file that is not
    /// there cannot be the one the set read.
    pub fn covers(&self, path: &Path) -> bool {
        // A standard document has no file identity; it is named, and the
        // name is what its entry is keyed by.
        if crate::forge::stdlib::names_standard(path) {
            return crate::forge::stdlib::lookup(path).is_some() && self.entries.contains_key(path);
        }
        // Keyed as every document identity is (`crate::confine::canonicalize`):
        // a document a run holds in memory has the key its written file has.
        match crate::confine::canonicalize(path) {
            Ok(real) => self.members.contains(&real),
            Err(_) => false,
        }
    }

    /// Every file whose bytes fed [`digest`](Self::digest), as paths a
    /// build system can watch.
    ///
    /// Exists so a depfile can name the source set without walking for
    /// it a second time. The second walk is what the depfile axis keeps
    /// getting wrong — it dropped the shared `_macros/` family, then the
    /// transitive `<sce:import>` closure, both times because the writer
    /// re-derived a set someone else had already resolved. The set that
    /// decided the hash is the set that must be declared, so it is the
    /// one returned here.
    ///
    /// Declaring these is not optional precision. The fold is total over
    /// the set, so editing *any* member changes the `source-hash` every
    /// artefact of that invocation embeds; a member left undeclared is an
    /// artefact the build reuses with a header that no longer describes
    /// its inputs, which is the state `sce-codegen verify` refuses.
    ///
    /// Order is the map's, so a depfile does not churn between runs.
    pub fn contributing_paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<PathBuf> = self
            .entries
            .keys()
            .filter(|rel| is_a_document(rel))
            // A standard document is in the generator, not on disk: a build
            // system watches the generator for it.
            .filter(|rel| !crate::forge::stdlib::names_standard(rel))
            .map(|rel| self.root.join(rel))
            .collect();
        // Appended from the field rather than joined onto `root`: the
        // canonical map key is a naming rule for the digest, not a
        // location, and the file it names may sit anywhere.
        if let Some(deploy) = &self.deploy_yaml {
            paths.push(deploy.clone());
        }
        paths
    }
}

/// The canonical key the limits a build was given are folded in under
/// (`crate::limits`): not a file, so it is never a path a build system watches.
const LIMITS_ENTRY: &str = "limits";

/// Is `rel` a document of the set, and not the `deploy.yaml` or the limits a build
/// was given, which the digest folds in beside the documents?
fn is_a_document(rel: &Path) -> bool {
    rel != Path::new("deploy.yaml") && rel != Path::new(LIMITS_ENTRY)
}

/// Fold the limits this thread's generator runs under into the set, when it runs
/// under any: a tree built with a bound moved does not pass for the tree built as
/// written, and a tree built as written has the digest it always had.
fn fold_limits(entries: &mut BTreeMap<PathBuf, [u8; 32]>) {
    let limits = crate::limits::current();
    if !limits.is_empty() {
        entries.insert(
            PathBuf::from(LIMITS_ENTRY),
            sha256_bytes(limits.canonical().as_bytes()),
        );
    }
}

/// Digest-only convenience over [`SourceSet::collect`]. Use the set itself
/// wherever the coverage invariant has to be asserted before the digest is
/// embedded; this entry point suits `sce-codegen verify`, which recomputes
/// against values already on disk.
pub fn compute_source_hash(
    input_root: &Path,
    deploy_yaml: Option<&Path>,
) -> Result<[u8; 32], DriftHashError> {
    Ok(SourceSet::collect(input_root, deploy_yaml)?.digest())
}

/// Where an input closure's references are looked for, beside the documents
/// themselves, and the file a run adds to every closure it takes.
#[derive(Clone, Copy, Debug, Default)]
pub struct ClosureOptions<'a> {
    /// The `-I` search path `<xi:include>` and `<sce:use>` fragments are
    /// found on, as the generation is given it.
    pub include_dirs: &'a [PathBuf],
    /// The `--input-root` a candidate or a `src` child is looked for under
    /// first, and the directory a staged copy stands for ([`InputClosure`]).
    pub input_root: Option<&'a Path>,
    /// The `deploy.yaml` the run is given. Every document of the run is
    /// generated under it, so it is in every closure the run takes.
    pub deploy_yaml: Option<&'a Path>,
}

/// The one `source:` a file generated from every document of a directory
/// names: the set form (ADR 0094 section 4).
pub const SET_FORM_SEED: &str = "./";

/// The input closure behind a `source-hash` (ADR 0094): the documents a file
/// is generated from (its seeds), every file their generation reads,
/// transitively ([`crate::import_closure::reads`]), `deploy.yaml` when the run
/// is given one, and the limits when the build moves any ([`fold_limits`]).
///
/// Each member is an entry `(key, sha256 of its raw bytes)`, folded by the
/// fold the directory set uses ([`SourceSet::digest`]). A standard document
/// is keyed by its library name. A file is keyed by its path relative to the
/// anchor, the deepest directory that holds every seed (for one seed, its
/// own directory), with `..` where it lies outside. So the digest is a
/// function of the files' bytes and of where they lie relative to each
/// other, and not of where the checkout sits or of the working directory.
///
/// A **staged copy** hashes as the document it stands for. A build that
/// copies one document into a directory of its own and names the original's
/// directory as `--input-root` (the integration regeneration scripts do)
/// takes the input root as the anchor, and keys what lies beside the copy
/// from the copy's directory; a copy is a seed outside the input root that
/// the input root holds a document of the same name for. Every key is then
/// the key a generation of the original gives it.
///
/// The bytes are read coherently, as the directory walk reads them: each
/// file is witnessed before it is read and checked again once the closure
/// is complete, and a closure that moved while it was read is read again,
/// then refused ([`DriftHashError::RootNotQuiescent`]).
#[derive(Clone, Debug)]
pub struct InputClosure {
    /// The seeds' keys, sorted: the `source:` lines of the header.
    seeds: Vec<String>,
    entries: BTreeMap<PathBuf, [u8; 32]>,
    /// Every file on disk whose bytes fed the digest, for a depfile.
    paths: Vec<PathBuf>,
}

impl InputClosure {
    /// The closure of `seeds`, the documents a file is generated from. A seed
    /// that cannot be read is refused, never left out: a closure without the
    /// document it describes would be a digest of something else.
    pub fn of(seeds: &[&Path], options: ClosureOptions<'_>) -> Result<Self, DriftHashError> {
        let walk = crate::import_closure::WalkOptions {
            include_dirs: options.include_dirs,
            input_root: options.input_root,
        };
        // A child document a run wrote stands for the document it came from
        // ([`recorded_origin`]), whose closure it carries.
        let owned: Vec<PathBuf> = seeds
            .iter()
            .map(|seed| origin_seed(seed, options.input_root))
            .collect();
        let seeds: Vec<&Path> = owned.iter().map(PathBuf::as_path).collect();
        for _ in 0..MAX_SNAPSHOT_ATTEMPTS {
            let mut reader = ClosureReader::new(Anchor::of(&seeds, options.input_root));
            let mut keys: Vec<String> = Vec::with_capacity(seeds.len());
            for seed in &seeds {
                let key = reader.read(seed).map_err(|source| DriftHashError::Io {
                    path: seed.to_path_buf(),
                    source,
                })?;
                keys.push(key.to_string_lossy().into_owned());
            }
            for member in crate::import_closure::reads(&owned, walk) {
                // A member that cannot be read contributes nothing: the
                // generation that reads it reports why in its own words.
                let _ = reader.read(&member);
            }
            reader.read_deploy(options.deploy_yaml)?;
            keys.sort();
            keys.dedup();
            if let Some(closure) = reader.finish(keys) {
                return Ok(closure);
            }
        }
        Err(DriftHashError::RootNotQuiescent {
            root: owned.first().cloned().unwrap_or_default(),
            attempts: MAX_SNAPSHOT_ATTEMPTS,
        })
    }

    /// The set form: the closure of every document under `root`, keyed from
    /// `root`, for a file generated from all of them (an index of the units
    /// a run generated, a harness over a fixture directory). It is the
    /// directory set ([`SourceSet::collect`]) and whatever its documents
    /// reach outside it, or read that is not a document: a set the walk of a
    /// directory alone left unsound.
    pub fn of_directory(root: &Path, options: ClosureOptions<'_>) -> Result<Self, DriftHashError> {
        let walk = crate::import_closure::WalkOptions {
            include_dirs: options.include_dirs,
            input_root: options.input_root,
        };
        for _ in 0..MAX_SNAPSHOT_ATTEMPTS {
            let set = SourceSet::collect(root, options.deploy_yaml)?;
            let documents: Vec<PathBuf> = set
                .contributing_paths()
                .into_iter()
                .filter(|path| set.deploy_yaml.as_deref() != Some(path.as_path()))
                .collect();
            let mut reader = ClosureReader::new(Anchor {
                dir: canonical_key(root),
                staged: None,
            });
            reader.entries = set.entries.clone();
            reader.seen = set.members.clone();
            reader.paths = set.contributing_paths();
            for member in crate::import_closure::reads(&documents, walk) {
                let _ = reader.read(&member);
            }
            if let Some(closure) = reader.finish(vec![SET_FORM_SEED.to_string()]) {
                return Ok(closure);
            }
        }
        Err(DriftHashError::RootNotQuiescent {
            root: root.to_path_buf(),
            attempts: MAX_SNAPSHOT_ATTEMPTS,
        })
    }

    /// The `source-hash` value: the fold of the entries in key order.
    pub fn digest(&self) -> [u8; 32] {
        hash_btreemap(&self.entries)
    }

    /// The seeds' keys, sorted, as the header's `source:` lines state them.
    pub fn seeds(&self) -> &[String] {
        &self.seeds
    }

    /// Whether a document contributed, and not only `deploy.yaml` or the
    /// limits. A set form over a directory that holds no document has
    /// nothing to describe, and is refused by its caller.
    pub fn holds_a_document(&self) -> bool {
        self.entries.keys().any(|key| is_a_document(key.as_path()))
    }

    /// Every file on disk whose bytes fed [`digest`](Self::digest), for a
    /// depfile to declare. A standard document is in the generator, which a
    /// build system watches for it.
    pub fn contributing_paths(&self) -> Vec<PathBuf> {
        self.paths.clone()
    }
}

/// The directory an [`InputClosure`]'s keys are taken from.
struct Anchor {
    /// Canonical.
    dir: PathBuf,
    /// The directory of a staged copy, canonical: what lies in it is keyed
    /// from it, as it would be from the original's directory.
    staged: Option<PathBuf>,
}

impl Anchor {
    fn of(seeds: &[&Path], input_root: Option<&Path>) -> Self {
        let dirs: Vec<PathBuf> = seeds
            .iter()
            .filter(|seed| !crate::forge::stdlib::names_standard(seed))
            .map(|seed| real_parent(seed))
            .collect();
        if let ([seed], [dir], Some(root)) = (seeds, dirs.as_slice(), input_root) {
            let root = canonical_key(root);
            let stands_for_one = !dir.starts_with(&root)
                && seed
                    .file_name()
                    .is_some_and(|name| crate::confine::exists(&root.join(name)));
            if stands_for_one {
                return Self {
                    dir: root,
                    staged: Some(dir.clone()),
                };
            }
        }
        Self {
            dir: deepest_common(&dirs),
            staged: None,
        }
    }

    /// The key of the file whose canonical path is `real`.
    fn key(&self, real: &Path) -> PathBuf {
        if let Some(rel) = self
            .staged
            .as_deref()
            .and_then(|local| real.strip_prefix(local).ok())
        {
            return rel.to_path_buf();
        }
        relative(&self.dir, real)
    }
}

/// The canonical directory of the document `path` names.
fn real_parent(path: &Path) -> PathBuf {
    let real = crate::confine::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    real.parent().map(Path::to_path_buf).unwrap_or_default()
}

/// The deepest directory every one of `dirs` is at or under.
fn deepest_common(dirs: &[PathBuf]) -> PathBuf {
    let mut common: Option<PathBuf> = None;
    for dir in dirs {
        common = Some(match common {
            None => dir.clone(),
            Some(so_far) => so_far
                .components()
                .zip(dir.components())
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| a.as_os_str())
                .collect(),
        });
    }
    common.unwrap_or_default()
}

/// `path` relative to `base`, both canonical: the components past the ones
/// they share, behind one `..` for each component of `base` past them.
fn relative(base: &Path, path: &Path) -> PathBuf {
    let base: Vec<Component> = base.components().collect();
    let parts: Vec<Component> = path.components().collect();
    let shared = base.iter().zip(&parts).take_while(|(a, b)| a == b).count();
    let mut out = PathBuf::new();
    for _ in shared..base.len() {
        out.push("..");
    }
    for part in &parts[shared..] {
        out.push(part.as_os_str());
    }
    out
}

/// The target of the processing instruction that records, at the head of a
/// child document a run writes itself, the document it came from (ADR 0094,
/// owner's answer 2): `<?sce-origin document="parent.scxml"?>`.
///
/// A run writes the document of each machine written in-line in an
/// `<invoke>`'s `<content>` (the `__sce_synth_invoke__` children, and the
/// documents of the SCE Mesh §9.6.6 children a deploy places on a partition
/// of their own), and another run generates from it. Its bytes are a
/// function of the document it was cut from, so a file generated from it is
/// generated from that document, and carries that document's closure; the
/// written child has to say which document that is, since no build system
/// keeps the relation for it.
///
/// A processing instruction is the carrier because it is XML's channel for
/// an instruction to one application (XML 1.0 section 2.6): it is in the
/// document, so the record travels with every copy of it, which a sidecar or
/// a staging record would not; it is no element or attribute, so the
/// document's content and its schema are unchanged and an SCXML processor
/// that does not know it ignores it; and unlike a comment, which an XML
/// processor may discard and which carries no structure, a reader is handed
/// it by target and pseudo-attribute.
pub const ORIGIN_INSTRUCTION: &str = "sce-origin";

/// How many recorded origins [`origin_seed`] follows: a child written by the
/// run of a child that was itself written records the authored document
/// directly, so one step is the rule and the bound only ends a loop.
const MAX_ORIGIN_STEPS: usize = 8;

/// `text`, a child document a run writes itself, with the document it came
/// from recorded ([`ORIGIN_INSTRUCTION`]). `document` is that document as its
/// closure keys it (its `source:`), which a run that generates from the child
/// finds under its `--input-root`, or beside the child.
///
/// The instruction goes on the first line, after an XML declaration when the
/// text has one (which must come first), so every row of the child is the row
/// it was and what is said about the child names the same rows. A document
/// name an instruction could not hold (a `"` or a `?>`) records nothing.
pub fn record_origin(text: &str, document: &str) -> String {
    if document.is_empty() || document.contains('"') || document.contains("?>") {
        return text.to_string();
    }
    let instruction = format!("<?{ORIGIN_INSTRUCTION} document=\"{document}\"?>");
    if text.starts_with("<?xml") {
        if let Some(end) = text.find("?>") {
            let (declaration, rest) = text.split_at(end + 2);
            return format!("{declaration}{instruction}{rest}");
        }
    }
    format!("{instruction}{text}")
}

/// The document a child document records it came from ([`record_origin`]),
/// or `None` for a document that records none: an instruction of the
/// target in its prolog, before its root element, with a `document`
/// pseudo-attribute.
pub fn recorded_origin(text: &str) -> Option<String> {
    let document = roxmltree::Document::parse(text).ok()?;
    let instruction = document
        .root()
        .children()
        .take_while(|node| !node.is_element())
        .filter_map(|node| node.pi())
        .find(|pi| pi.target == ORIGIN_INSTRUCTION)?;
    let (_, after) = instruction.value?.split_once("document=\"")?;
    let (value, _) = after.split_once('"')?;
    (!value.is_empty()).then(|| value.to_string())
}

/// The document `seed` stands for: itself, or, for a child document a run
/// wrote, the document it records it came from, found under `input_root`
/// first and then beside the child. A recorded origin found in neither place
/// leaves the child its own seed: its bytes are a function of the document
/// it came from, so its closure is still sound, but nothing under a root
/// names it for `verify`, which is why a run that generates from a written
/// child names the directory of its origin as `--input-root`.
fn origin_seed(seed: &Path, input_root: Option<&Path>) -> PathBuf {
    let mut current = seed.to_path_buf();
    for _ in 0..MAX_ORIGIN_STEPS {
        if crate::forge::stdlib::names_standard(&current) {
            break;
        }
        let Some(document) = crate::confine::read_to_string(&current)
            .ok()
            .as_deref()
            .and_then(recorded_origin)
        else {
            break;
        };
        // A child of a standard statechart came from the library.
        let standard = Path::new(&document);
        if crate::forge::stdlib::names_standard(standard) {
            if crate::forge::stdlib::lookup(standard).is_some() {
                current = standard.to_path_buf();
            }
            break;
        }
        let beside = current.parent().map(|dir| dir.join(document.as_str()));
        let Some(found) = input_root
            .map(|root| root.join(document.as_str()))
            .into_iter()
            .chain(beside)
            .find(|candidate| candidate != &current && crate::confine::exists(candidate))
        else {
            break;
        };
        current = found;
    }
    current
}

/// One coherent read of a closure's members.
struct ClosureReader {
    anchor: Anchor,
    entries: BTreeMap<PathBuf, [u8; 32]>,
    paths: Vec<PathBuf>,
    /// The canonical path of every file already read.
    seen: BTreeSet<PathBuf>,
    witnesses: Vec<ReadWitness>,
}

impl ClosureReader {
    fn new(anchor: Anchor) -> Self {
        Self {
            anchor,
            entries: BTreeMap::new(),
            paths: Vec::new(),
            seen: BTreeSet::new(),
            witnesses: Vec::new(),
        }
    }

    /// Read the file `path` names into the closure, once, and give its key.
    /// Through [`crate::confine`], so a document a run holds in memory is
    /// read as the run that writes it would read it.
    fn read(&mut self, path: &Path) -> std::io::Result<PathBuf> {
        if crate::forge::stdlib::names_standard(path) {
            let content = crate::forge::stdlib::lookup(path)
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
            let key = PathBuf::from(crate::forge::stdlib::identity(path));
            self.entries
                .entry(key.clone())
                .or_insert_with(|| sha256_bytes(content.as_bytes()));
            return Ok(key);
        }
        let real = crate::confine::canonicalize(path)?;
        let key = self.anchor.key(&real);
        if self.seen.contains(&real) {
            return Ok(key);
        }
        // Witnessed before the read, for the reason the directory walk
        // witnesses first. A document held in memory has no file to stat,
        // and nothing writes it while the run reads it.
        let before = ReadWitness::observe(path);
        let bytes = crate::confine::read(path)?;
        self.entries.insert(key.clone(), sha256_bytes(&bytes));
        self.witnesses.extend(before);
        self.seen.insert(real.clone());
        self.paths.push(real);
        Ok(key)
    }

    /// `deploy.yaml`, under its canonical key, as the directory set takes it.
    fn read_deploy(&mut self, deploy: Option<&Path>) -> Result<(), DriftHashError> {
        let Some(deploy) = deploy else {
            return Ok(());
        };
        let before = ReadWitness::observe(deploy);
        let bytes = fs::read(deploy).map_err(|source| DriftHashError::Io {
            path: deploy.to_path_buf(),
            source,
        })?;
        self.entries
            .insert(PathBuf::from("deploy.yaml"), sha256_bytes(&bytes));
        self.witnesses.extend(before);
        self.paths.push(deploy.to_path_buf());
        Ok(())
    }

    /// The closure, when nothing it read moved while it was read; `None`
    /// when something did, for the caller to read it again.
    fn finish(mut self, seeds: Vec<String>) -> Option<InputClosure> {
        let settled = self
            .witnesses
            .iter()
            .all(|w| ReadWitness::observe(&w.path).as_ref() == Some(w));
        if !settled {
            return None;
        }
        fold_limits(&mut self.entries);
        Some(InputClosure {
            seeds,
            entries: self.entries,
            paths: self.paths,
        })
    }
}

/// Renders the §synth-6.2.6 header: the banner, then `source-hash`, then one
/// `source:` line per seed in the order the header holds them (sorted).
/// `comment_prefix` is `//` for Rust/Cpp/C11/Kotlin/Go and `#` for Python.
/// Caller prepends the result at the very top of each emitted file.
///
/// The output ends in `\n` so subsequent template content starts on a
/// fresh line.
pub fn render_header(header: &DriftHeader, comment_prefix: &str) -> String {
    let mut rendered = format!(
        "{cp} {HEADER_BANNER}\n{cp} {SOURCE_HASH_KEY}: {sh}\n",
        cp = comment_prefix,
        sh = header.source_hex(),
    );
    for seed in &header.seeds {
        rendered.push_str(&format!("{comment_prefix} {SEED_KEY}: {seed}\n"));
    }
    rendered
}

/// Picks the comment prefix (`//` or `#`) by file extension. Drives
/// the per-backend header shape without needing to plumb a language
/// enum through the post-process step.
pub fn comment_prefix_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("py") => "#",
        _ => "//",
    }
}

/// Prepends the §synth-6.2.6 header to a file's content. Idempotent against
/// already-headered content — if `content` already begins with the banner
/// line, the existing header block is replaced rather than duplicated.
/// Idempotence matters for the production pipeline where codegen may run
/// multiple times for the same logical inputs, and where a file is read
/// back and extended (the Rust suite's `mod.rs`).
///
/// The existing block is found by its keys, not counted in lines, so a
/// header of the older four-line shape is replaced as cleanly as the
/// current one. The body after it is kept byte for byte.
pub fn prepend_or_replace_header(
    content: &str,
    header: &DriftHeader,
    comment_prefix: &str,
) -> String {
    let body = strip_header(content).unwrap_or(content);
    format!("{}{body}", render_header(header, comment_prefix))
}

/// `content` without its leading §synth-6.2.6 header, or `None` when it
/// does not lead with one. The header is the banner line and the key lines
/// directly under it, whichever shape wrote them.
fn strip_header(content: &str) -> Option<&str> {
    let (first, mut rest) = split_first_line(content)?;
    if !first.contains(HEADER_BANNER) {
        return None;
    }
    while let Some((line, after)) = split_first_line(rest) {
        if !is_header_key_line(line) {
            break;
        }
        rest = after;
    }
    Some(rest)
}

/// The first line of `text` without its terminator, and everything after
/// the terminator. `None` for empty text.
fn split_first_line(text: &str) -> Option<(&str, &str)> {
    if text.is_empty() {
        return None;
    }
    Some(match text.find('\n') {
        Some(end) => (&text[..end], &text[end + 1..]),
        None => (text, ""),
    })
}

/// Is `line` one of the header's key lines — `source-hash`, `source`, or a
/// key an older shape wrote — under either comment prefix?
fn is_header_key_line(line: &str) -> bool {
    let Some(body) = comment_body(line) else {
        return false;
    };
    [SOURCE_HASH_KEY, SEED_KEY]
        .into_iter()
        .chain(RETIRED_HEADER_KEYS)
        .any(|key| key_value(body, key).is_some())
}

/// The value of a `key: value` comment body, or `None` for another key.
fn key_value<'a>(body: &'a str, key: &str) -> Option<&'a str> {
    body.strip_prefix(key)?.strip_prefix(": ")
}

/// The text of a `// ` or `# ` comment line, or `None` for any other line.
fn comment_body(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix("// ")
        .or_else(|| trimmed.strip_prefix("# "))
}

/// What a generated file's header states, as written. Hex values are
/// returned as-is (lowercase, 64 chars).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedHeader {
    pub source_hash_hex: String,
    /// The `source:` lines, in the order written; empty for a header of a
    /// shape before ADR 0094, which `sce-codegen verify` refuses.
    pub seeds: Vec<String>,
}

/// Parses the §synth-6.2.6 header out of a generated file's content, or
/// `None` when the banner or the `source-hash` line is missing. Accepts
/// either `//` or `#` comment prefix, and every header shape.
///
/// Reads every key line under the banner, however many there are: a file
/// generated from a set of documents has one `source:` line per document.
pub fn parse_embedded_header(content: &str) -> Option<EmbeddedHeader> {
    let mut lines = content.lines();
    // The banner leads the file, behind nothing but comment lines.
    loop {
        if comment_body(lines.next()?)?.starts_with(HEADER_BANNER) {
            break;
        }
    }
    let mut source_hash_hex = None;
    let mut seeds = Vec::new();
    for line in lines {
        let Some(body) = comment_body(line) else {
            break;
        };
        if let Some(value) = key_value(body, SOURCE_HASH_KEY) {
            source_hash_hex = Some(value.trim().to_string());
        } else if let Some(value) = key_value(body, SEED_KEY) {
            seeds.push(value.trim().to_string());
        } else if !RETIRED_HEADER_KEYS
            .iter()
            .any(|key| key_value(body, key).is_some())
        {
            break;
        }
    }
    Some(EmbeddedHeader {
        source_hash_hex: source_hash_hex?,
        seeds,
    })
}

/// A generated output tree, as a check made during generation reads it.
///
/// Ordinarily the disk. An `sce-codegen --assert-unchanged` run writes
/// nothing, so a check it makes has to read the tree that run would have
/// left behind — the files it produced laid over the files already there.
/// Reading the disk instead judges bytes the run never produced: the
/// previous run's, or the hand edit the assertion exists to expose.
pub trait GeneratedTree {
    /// Every file under `dir`, recursively.
    fn files_under(&self, dir: &Path) -> BTreeSet<PathBuf>;

    /// The text of the file at `path`, or `None` when there is none.
    fn read_to_string(&self, path: &Path) -> Option<String>;
}

/// The generated tree on disk.
pub struct OnDisk;

impl GeneratedTree for OnDisk {
    fn files_under(&self, dir: &Path) -> BTreeSet<PathBuf> {
        fn walk(dir: &Path, out: &mut BTreeSet<PathBuf>) {
            let Ok(entries) = fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else {
                    out.insert(path);
                }
            }
        }
        let mut out = BTreeSet::new();
        walk(dir, &mut out);
        out
    }

    fn read_to_string(&self, path: &Path) -> Option<String> {
        fs::read_to_string(path).ok()
    }
}

// ── internal helpers ──────────────────────────────────────────────────

/// What a file looked like when its bytes were taken. Re-stat-ing every
/// witness after the walk is what makes the digest a snapshot rather than
/// a sequence of unrelated reads.
///
/// `(len, mtime)` is the change signal every build system already trusts.
/// `modified()` is not available on every filesystem, so the timestamp is
/// optional and the comparison degrades to length alone rather than
/// refusing to run.
#[derive(Clone, PartialEq, Eq)]
struct ReadWitness {
    path: PathBuf,
    len: u64,
    mtime: Option<std::time::SystemTime>,
}

impl ReadWitness {
    /// Observes `path` as it is right now. `None` when the file cannot be
    /// stat-ed at all, which the caller reads as "changed" — a file that
    /// vanished is exactly the case this exists to catch.
    fn observe(path: &Path) -> Option<Self> {
        let meta = fs::metadata(path).ok()?;
        Some(Self {
            path: path.to_path_buf(),
            len: meta.len(),
            mtime: meta.modified().ok(),
        })
    }
}

/// Re-reads of one source tree allowed before the walk gives up.
///
/// The walk reads files one at a time, so a tree being written while it is
/// read yields bytes from no single state of that tree. Re-walking is how a
/// coherent read is obtained without a filesystem snapshot primitive: read,
/// verify nothing moved, and read again if something did.
///
/// The ceiling exists because a tree under continuous mutation never
/// settles, and looping forever on it would be worse than saying so. Three
/// is enough for the bursty writes a parallel build produces and small
/// enough that a genuinely unstable root is reported promptly.
const MAX_SNAPSHOT_ATTEMPTS: usize = 3;

/// Walks `root` collecting every file whose path predicate returns true.
/// Returns sorted `(rel_path_from_anchor, sha256)` pairs. `anchor` is the
/// path canonicalization basis so the BTreeMap key stays stable across
/// absolute/relative invocations.
///
/// The walk is retried until the files it read are still the files on disk,
/// so the digest describes one state of the tree rather than a blend of
/// several. See [`walk_filtered_coherent`].
fn walk_filtered(
    anchor: &Path,
    root: &Path,
    out: &mut BTreeMap<PathBuf, [u8; 32]>,
    keep: &dyn Fn(&Path) -> bool,
    descent_limit: usize,
) -> Result<(), DriftHashError> {
    walk_filtered_coherent(
        anchor,
        root,
        out,
        keep,
        descent_limit,
        MAX_SNAPSHOT_ATTEMPTS,
        &mut || {},
    )
}

/// [`walk_filtered`] with the attempt ceiling supplied and a hook fired
/// after each completed traversal.
///
/// Both extras exist for the tests: a concurrent writer cannot be scheduled
/// deterministically from outside, so the hook is where a test mutates the
/// tree at the one instant that matters — after the bytes were taken and
/// before they are verified — and the ceiling lets the refusal be reached
/// without looping three times to get there.
pub(crate) fn walk_filtered_coherent(
    anchor: &Path,
    root: &Path,
    out: &mut BTreeMap<PathBuf, [u8; 32]>,
    keep: &dyn Fn(&Path) -> bool,
    descent_limit: usize,
    attempts: usize,
    after_walk: &mut dyn FnMut(),
) -> Result<(), DriftHashError> {
    for _ in 0..attempts.max(1) {
        // Seeded with the root so a link naming the root is recognised as a
        // cycle by the same rule that catches one naming any other ancestor.
        let mut descent: BTreeSet<PathBuf> = BTreeSet::new();
        descent.insert(canonical_key(root));
        let mut budget = DescentBudget {
            root: root.to_path_buf(),
            remaining: descent_limit,
            limit: descent_limit,
        };
        let mut witnesses: Vec<ReadWitness> = Vec::new();

        // Each attempt starts from an empty set: a retry must not inherit
        // digests taken from the state that was already found to have moved.
        out.clear();
        match walk_filtered_recursive(
            anchor,
            root,
            out,
            keep,
            &mut descent,
            &mut budget,
            &mut witnesses,
        ) {
            Ok(()) => {}
            // An entry `read_dir` listed and `fs::read` could not find is the
            // tree moving under the walk, not a broken root — retry it. The
            // root itself going missing is a real failure and falls through.
            Err(e) if is_vanished_entry(&e, root) => continue,
            Err(e) => return Err(e),
        }

        after_walk();

        if witnesses
            .iter()
            .all(|w| ReadWitness::observe(&w.path).as_ref() == Some(w))
        {
            return Ok(());
        }
    }

    Err(DriftHashError::RootNotQuiescent {
        root: root.to_path_buf(),
        attempts: attempts.max(1),
    })
}

/// Did this error come from an entry disappearing mid-walk rather than from
/// the root being wrong? Only the former is worth another attempt.
fn is_vanished_entry(err: &DriftHashError, root: &Path) -> bool {
    match err {
        DriftHashError::Io { path, source } => {
            source.kind() == std::io::ErrorKind::NotFound && path != root
        }
        _ => false,
    }
}

/// Descent allowance for one walk, spent across the whole traversal rather
/// than per branch — the cost being bounded is the total number of
/// `read_dir` calls. Carries the root and the original ceiling so the
/// refusal names both without the recursion threading them separately.
struct DescentBudget {
    root: PathBuf,
    remaining: usize,
    limit: usize,
}

impl DescentBudget {
    /// Charges one descent. The error is a function of the root and the
    /// ceiling only — never of how far this particular traversal happened
    /// to get, which would vary with directory iteration order and make
    /// the diagnostic unstable across machines.
    fn charge(&mut self) -> Result<(), DriftHashError> {
        match self.remaining.checked_sub(1) {
            Some(left) => {
                self.remaining = left;
                Ok(())
            }
            None => Err(DriftHashError::WalkLimitExceeded {
                root: self.root.clone(),
                limit: self.limit,
            }),
        }
    }
}

/// Recursive half of [`walk_filtered`]. `descent` carries the canonicalized
/// directories on the path currently being descended, so a directory link
/// that resolves to one of its own ancestors terminates the walk.
///
/// The set is the *current path*, not every directory ever visited, and the
/// distinction decides the digest. Two links can resolve to one directory
/// without either being a cycle — neither lies on the other's descent path,
/// and each names a distinct set of root-relative paths, which is what the
/// source set is keyed by. Suppressing the second one instead makes the
/// surviving name a function of `fs::read_dir` ordering, so the same tree
/// hashes differently on two machines; it also drops the alias from drift
/// detection, since removing it would leave the digest unchanged.
///
/// A link onto an ancestor is the opposite case: every file it reaches is
/// one the walk is already collecting, reachable under unboundedly many
/// spellings. Cutting there is what bounds the walk.
fn walk_filtered_recursive(
    anchor: &Path,
    dir: &Path,
    out: &mut BTreeMap<PathBuf, [u8; 32]>,
    keep: &dyn Fn(&Path) -> bool,
    descent: &mut BTreeSet<PathBuf>,
    budget: &mut DescentBudget,
    witnesses: &mut Vec<ReadWitness>,
) -> Result<(), DriftHashError> {
    budget.charge()?;
    let entries = fs::read_dir(dir).map_err(|e| DriftHashError::Io {
        path: dir.to_path_buf(),
        source: e,
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| DriftHashError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?;
        let path = entry.path();
        let link_type = entry.file_type().map_err(|e| DriftHashError::Io {
            path: path.clone(),
            source: e,
        })?;
        // `entry.file_type()` is an lstat: for a symlink it reports neither
        // dir nor file, so trusting it drops the entry from the source set.
        // Build sandboxes (Bazel execroot, Nix, staged CMake inputs) expose
        // declared inputs as links into the real tree rather than copies, and
        // a source set that drops every entry folds to the empty-input
        // digest — a valid-looking hash the §synth-6.2.6 drift check cannot
        // tell apart from a successful one. Resolve through the link.
        let target_type = if link_type.is_symlink() {
            match fs::metadata(&path) {
                Ok(meta) => meta.file_type(),
                // A dangling link names no bytes, so it contributes nothing
                // by definition. The source-set coverage invariant asserted
                // by `SourceSet::covers` is what catches the case where the
                // dangling link was the input document itself.
                Err(_) => continue,
            }
        } else {
            link_type
        };
        if target_type.is_dir() {
            // Symlinked directories can form cycles. Key the guard on the
            // canonical target so a link onto a directory already being
            // descended terminates instead of recursing.
            let key = canonical_key(&path);
            if !descent.insert(key.clone()) {
                continue;
            }
            let descended =
                walk_filtered_recursive(anchor, &path, out, keep, descent, budget, witnesses);
            descent.remove(&key);
            descended?;
        } else if target_type.is_file() && keep(&path) {
            // Witness before the read, not after. A file replaced *during*
            // its own read leaves a post-read stat that matches the final
            // one, so the swap would go unseen; a pre-read stat does not.
            let before = ReadWitness::observe(&path);
            let bytes = fs::read(&path).map_err(|e| DriftHashError::Io {
                path: path.clone(),
                source: e,
            })?;
            let rel = path.strip_prefix(anchor).unwrap_or(&path).to_path_buf();
            out.insert(rel, sha256_bytes(&bytes));
            match before {
                Some(w) => witnesses.push(w),
                // Read succeeded but the stat did not, so nothing can vouch
                // for these bytes. Treat the attempt as incoherent rather
                // than silently dropping the check for this one file.
                None => {
                    return Err(DriftHashError::Io {
                        path: path.clone(),
                        source: std::io::Error::from(std::io::ErrorKind::NotFound),
                    })
                }
            }
        }
    }
    Ok(())
}

/// Identity a directory is compared by when deciding whether the walk is
/// already inside it. Falls back to the path as addressed when the real
/// path cannot be resolved — a directory whose identity cannot be
/// established is better treated as distinct than silently merged with
/// another.
fn canonical_key(dir: &Path) -> PathBuf {
    fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf())
}

// The three primitives this module folds its hashes with — `sha256_bytes`,
// `hash_btreemap`, `hex_encode` — live in [`crate::generator_witness`]
// because `sce-build/build.rs` `include!`s that file and cannot reach the
// library. Stating them twice would put the §synth-6.2.6 `source-hash`
// one careless edit away from moving silently under every committed
// generated header in the tree.

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_file(dir: &Path, rel: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content).unwrap();
        path
    }

    #[test]
    fn hex_encode_round_trip() {
        let mut bytes = [0u8; 32];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = i as u8;
        }
        assert_eq!(
            hex_encode(&bytes),
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
        );
    }

    /// Two calls over one unchanged tree agree. That is idempotence, and it
    /// is all this test shows — running the same computation twice cannot
    /// establish that the result is independent of the order the filesystem
    /// lists entries in, because both runs see the same order. The BTreeMap
    /// normalises the order of the keys it is *given*; which keys it is given
    /// is a separate question, and the one that has to be tested by varying
    /// the input. `source_hash_counts_every_alias_of_one_directory` and
    /// `source_hash_of_aliased_directories_ignores_creation_order` cover it.
    #[test]
    fn source_hash_is_idempotent_over_an_unchanged_tree() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "a.scxml", b"<scxml/>");
        write_file(root, "sub/b.scxml", b"<scxml/>");
        let h1 = compute_source_hash(root, None).unwrap();
        let h2 = compute_source_hash(root, None).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn source_hash_excludes_non_scxml() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "a.scxml", b"<scxml/>");
        let h_pre = compute_source_hash(root, None).unwrap();

        // Add a non-scxml file under root — should not affect the hash.
        write_file(root, "noise.txt", b"ignore me");
        write_file(root, "deploy.yaml", b"untouched");
        let h_post = compute_source_hash(root, None).unwrap();
        assert_eq!(
            h_pre, h_post,
            "non-.scxml files must not contribute to source-hash"
        );
    }

    #[test]
    fn source_hash_changes_when_scxml_edited() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let path = write_file(root, "a.scxml", b"<scxml/>");
        let h_pre = compute_source_hash(root, None).unwrap();

        fs::write(&path, b"<scxml version='1.0'/>").unwrap();
        let h_post = compute_source_hash(root, None).unwrap();
        assert_ne!(h_pre, h_post);
    }

    /// A source tree may reach `input_root` through symlinks — build
    /// sandboxes (Bazel execroot, Nix, `cmake` staged inputs) materialise
    /// declared inputs as links into the real tree rather than copies.
    /// `entry.file_type()` does not traverse them, so a walk that trusts it
    /// collects nothing and folds to the empty-input digest, which is a
    /// valid-looking hash a consumer cannot distinguish from success.
    #[test]
    fn source_hash_follows_symlinked_scxml_file() {
        let real = TempDir::new().unwrap();
        let sandbox = TempDir::new().unwrap();
        let target = write_file(real.path(), "doc.scxml", b"<scxml/>");
        std::os::unix::fs::symlink(&target, sandbox.path().join("doc.scxml")).unwrap();

        let linked = compute_source_hash(sandbox.path(), None).unwrap();
        let direct = compute_source_hash(real.path(), None).unwrap();
        assert_eq!(
            linked, direct,
            "a symlinked .scxml must hash identically to the file it points at"
        );
    }

    /// Same traversal defect one level up: a symlinked *directory* under
    /// `input_root` is neither `is_dir()` nor `is_file()` to `lstat`, so
    /// everything beneath it disappears from the source set.
    #[test]
    fn source_hash_follows_symlinked_directory() {
        let real = TempDir::new().unwrap();
        let sandbox = TempDir::new().unwrap();
        write_file(real.path(), "nested/doc.scxml", b"<scxml/>");
        std::os::unix::fs::symlink(real.path().join("nested"), sandbox.path().join("nested"))
            .unwrap();

        let linked = compute_source_hash(sandbox.path(), None).unwrap();
        let direct = compute_source_hash(real.path(), None).unwrap();
        assert_eq!(
            linked, direct,
            "a symlinked subdirectory must contribute the .scxml files beneath it"
        );
    }

    /// A symlink cycle must terminate the walk instead of recursing until
    /// the stack runs out.
    #[test]
    fn source_hash_terminates_on_symlink_cycle() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "nested/doc.scxml", b"<scxml/>");
        // nested/loop -> nested (a directory containing the link itself)
        std::os::unix::fs::symlink(root.join("nested"), root.join("nested/loop")).unwrap();

        let h = compute_source_hash(root, None).unwrap();
        let flat = {
            let plain = TempDir::new().unwrap();
            write_file(plain.path(), "nested/doc.scxml", b"<scxml/>");
            compute_source_hash(plain.path(), None).unwrap()
        };
        assert_eq!(
            h, flat,
            "a cyclic directory link must be descended once and contribute \
             nothing beyond the files already collected"
        );
    }

    /// A link resolving to the root itself is the same class as the case
    /// above — it names no file the walk is not already collecting, only a
    /// second spelling of each, and unboundedly many of them. The cycle
    /// guard has to be seeded with the root for the two to agree.
    #[test]
    fn source_hash_terminates_on_a_link_back_to_the_root() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "doc.scxml", b"<scxml/>");
        std::os::unix::fs::symlink(root, root.join("self")).unwrap();

        let set = SourceSet::collect(root, None).unwrap();
        let keys: Vec<String> = set
            .entries
            .keys()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            keys,
            vec!["doc.scxml".to_string()],
            "a link to the root re-spells files already collected; it must \
             not add entries"
        );
    }

    /// Two links under one root may resolve to the same directory without
    /// either being a cycle: neither is on the other's descent path, and
    /// each names a distinct set of root-relative paths. A build sandbox
    /// does this when it materialises two declared inputs as links into one
    /// directory of the real tree. The W3C corpus (`resources/`) holds no
    /// such link: a checkout without symbolic links holds a text file in its
    /// place, and would hash a different input set than the same commit
    /// checked out with them.
    ///
    /// Suppressing the second and later ones leaks `fs::read_dir` ordering
    /// into the digest: whichever name the filesystem happens to yield
    /// first is the one that keys the entries, so two machines hash the
    /// same tree to different values. It also drops the alias from drift
    /// detection — removing it would leave the digest unchanged.
    #[test]
    fn source_hash_counts_every_alias_of_one_directory() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "real/doc.scxml", b"<scxml/>");
        std::os::unix::fs::symlink(root.join("real"), root.join("alias_a")).unwrap();
        std::os::unix::fs::symlink(root.join("real"), root.join("alias_b")).unwrap();

        let set = SourceSet::collect(root, None).unwrap();
        let keys: Vec<String> = set
            .entries
            .keys()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            keys,
            vec![
                "alias_a/doc.scxml".to_string(),
                "alias_b/doc.scxml".to_string(),
                "real/doc.scxml".to_string(),
            ],
            "every root-relative path naming a .scxml keys its own entry"
        );
    }

    /// Aliases are what make the path count exponential in the depth of the
    /// tree, so the walk needs a ceiling that turns a pathological input
    /// into a bounded failure.
    ///
    /// Built as sibling levels linked forward: each of `l0`, `l1`, `l2`
    /// holds three links to the *next* level, so `l3/doc.scxml` is named by
    /// 3^3 root-relative paths out of five real directories. The links have
    /// to point forward — a link naming its own parent is an ancestor, which
    /// the cycle rule skips, so it would multiply nothing.
    ///
    /// The refusal must be an error, not a truncated set: a partial walk
    /// folds to a digest describing a subset of the input, which is the
    /// unauditable-header failure the empty-set refusal already rules out.
    #[test]
    fn source_hash_refuses_a_tree_that_exceeds_the_descent_ceiling() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "l3/doc.scxml", b"<scxml/>");
        for level in ["l0", "l1", "l2"] {
            fs::create_dir_all(root.join(level)).unwrap();
        }
        for (from, to) in [("l0", "l1"), ("l1", "l2"), ("l2", "l3")] {
            for name in ["a", "b", "c"] {
                std::os::unix::fs::symlink(
                    root.join(to),
                    root.join(from).join(format!("alias_{name}")),
                )
                .unwrap();
            }
        }

        let err = SourceSet::collect_within(root, None, 8)
            .expect_err("a tree past the ceiling must refuse, not truncate");
        match err {
            DriftHashError::WalkLimitExceeded { root: r, limit } => {
                assert_eq!(r, root, "the refusal names the root it was given");
                assert_eq!(
                    limit, 8,
                    "the refusal names the ceiling, not a partial count"
                );
            }
            other => panic!("expected WalkLimitExceeded, got {other:?}"),
        }
    }

    /// The ceiling must not fire on an ordinary tree — a liveness bound that
    /// refuses legitimate input is a size policy. That the constant stays far
    /// above real tree sizes is asserted where it is defined, at compile
    /// time; what this covers is the production entry point actually running
    /// under it rather than around it.
    #[test]
    fn the_descent_ceiling_does_not_fire_on_an_ordinary_tree() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "a.scxml", b"<scxml/>");
        write_file(root, "sub/nested/b.scxml", b"<scxml/>");
        assert!(
            SourceSet::collect(root, None).is_ok(),
            "an ordinary tree must not come near the ceiling"
        );
    }

    /// The digest the aliases produce must not depend on which name the
    /// filesystem yields first. Building the same logical tree with the
    /// links created in the opposite order pins that: under a first-wins
    /// guard the two trees agree only by luck of the two readdir orders.
    #[test]
    fn source_hash_of_aliased_directories_ignores_creation_order() {
        let build = |a_first: bool| {
            let dir = TempDir::new().unwrap();
            let root = dir.path();
            write_file(root, "real/doc.scxml", b"<scxml/>");
            let names = if a_first {
                ["alias_a", "alias_b"]
            } else {
                ["alias_b", "alias_a"]
            };
            for name in names {
                std::os::unix::fs::symlink(root.join("real"), root.join(name)).unwrap();
            }
            compute_source_hash(root, None).unwrap()
        };
        assert_eq!(
            build(true),
            build(false),
            "the source-hash must be a function of the tree, not of the \
             order its entries were created in"
        );
    }

    /// The coverage invariant: the document codegen was handed must be in
    /// the set whose digest gets embedded.
    #[test]
    fn source_set_covers_the_input_document() {
        let dir = TempDir::new().unwrap();
        let doc = write_file(dir.path(), "doc.scxml", b"<scxml/>");
        let set = SourceSet::collect(dir.path(), None).unwrap();
        assert!(set.covers(&doc));
        assert_eq!(set.len(), 1);
    }

    /// A root that imports from the standard library generates from the
    /// documents it reaches, so those are folded in — under their
    /// `sce:std/...` names, never as paths a build system could watch, since
    /// they live in the generator — and no others: a standard document's
    /// edit must not move a tree that never read it.
    #[test]
    fn a_root_folds_in_the_standard_documents_its_imports_reach() {
        let plain = TempDir::new().unwrap();
        write_file(plain.path(), "doc.scxml", b"<scxml/>");
        let plain_set = SourceSet::collect(plain.path(), None).unwrap();
        assert_eq!(plain_set.len(), 1);

        // days_in_month imports is_leap_year by a relative name inside the
        // library: both are what the document reaches.
        let importing = TempDir::new().unwrap();
        let doc = write_file(
            importing.path(),
            "doc.scxml",
            br#"<scxml xmlns:sce="http://sce.dev/ext"><sce:import kind="algorithm" src="sce:std/time/days_in_month.scxml" as="d"/></scxml>"#,
        );
        let set = SourceSet::collect(importing.path(), None).unwrap();
        let reached =
            crate::forge::stdlib::closure([PathBuf::from("sce:std/time/days_in_month.scxml")]);
        assert!(
            reached.contains_key("sce:std/time/is_leap_year.scxml"),
            "the closure follows an import inside the library: {:?}",
            reached.keys().collect::<Vec<_>>()
        );
        assert!(
            reached.len() < crate::forge::stdlib::documents().count(),
            "a document reaches part of the library, not all of it"
        );
        assert_eq!(set.len(), 1 + reached.len());
        assert!(set.covers(&doc));
        assert_eq!(
            set.contributing_paths(),
            vec![importing.path().join("doc.scxml")]
        );
    }

    /// A standard document's source set is the library's counterpart of its
    /// directory — the documents under its `sce:std/...` prefix and what
    /// they import — not the whole library, so a document added elsewhere
    /// in it leaves the hash where it was.
    #[test]
    fn a_standard_root_holds_its_prefix_and_what_it_imports() {
        let set = SourceSet::collect(Path::new("sce:std/time"), None).unwrap();
        let names: Vec<&Path> = set.entries.keys().map(PathBuf::as_path).collect();
        assert!(
            names.contains(&Path::new("sce:std/time/days_in_month.scxml")),
            "{names:?}"
        );
        assert!(
            names
                .iter()
                .all(|n| n.to_string_lossy().starts_with("sce:std/time/")),
            "the time documents import only each other, so nothing else is in: {names:?}"
        );
        assert!(set.covers(Path::new("sce:std/time/days_in_month.scxml")));
    }

    /// Only an `<sce:import>` element imports. A document whose comment
    /// names a standard document — as every document under stdlib/ names
    /// itself in its header — reads nothing from the library, and folding
    /// the library in for it re-stamped a tree for every document added
    /// anywhere in it.
    #[test]
    fn a_standard_name_in_a_comment_folds_nothing_in() {
        let dir = TempDir::new().unwrap();
        write_file(
            dir.path(),
            "doc.scxml",
            b"<!-- sce:std/mesh/envelope_id.scxml: a sixteen-byte id -->\n<scxml/>",
        );
        let set = SourceSet::collect(dir.path(), None).unwrap();
        assert_eq!(set.len(), 1);
    }

    /// Coverage is keyed on file identity, so a sandbox link name resolves
    /// against a set collected from the real tree and vice versa — the two
    /// spellings of the same document must both answer "covered".
    #[test]
    fn source_set_covers_document_addressed_through_a_symlink() {
        let real = TempDir::new().unwrap();
        let sandbox = TempDir::new().unwrap();
        let target = write_file(real.path(), "doc.scxml", b"<scxml/>");
        let link = sandbox.path().join("doc.scxml");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let from_real = SourceSet::collect(real.path(), None).unwrap();
        assert!(from_real.covers(&link), "link name must resolve to covered");
        let from_sandbox = SourceSet::collect(sandbox.path(), None).unwrap();
        assert!(from_sandbox.covers(&target), "real path must resolve too");
    }

    /// The reported failure mode: the walk collects nothing, the digest is
    /// still a well-formed sha256, and only the coverage check can tell.
    #[test]
    fn source_set_rejects_document_outside_the_collected_root() {
        let elsewhere = TempDir::new().unwrap();
        let doc = write_file(elsewhere.path(), "doc.scxml", b"<scxml/>");
        let empty_root = TempDir::new().unwrap();

        let set = SourceSet::collect(empty_root.path(), None).unwrap();
        assert!(set.is_empty());
        assert_eq!(
            hex_encode(&set.digest()),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "an empty source set still folds to a well-formed digest — this \
             is why coverage has to be asserted separately"
        );
        assert!(!set.covers(&doc));
    }

    /// Same bytes, different file: a copy outside the root is not covered.
    /// Were it, every later edit to the copy would leave the digest
    /// unmoved — the coverage check would admit exactly the input whose
    /// changes the `source-hash` cannot see.
    #[test]
    fn source_set_rejects_a_byte_identical_copy_outside_the_root() {
        let root = TempDir::new().unwrap();
        let elsewhere = TempDir::new().unwrap();
        let member = write_file(root.path(), "doc.scxml", b"<scxml/>");
        let copy = write_file(elsewhere.path(), "doc.scxml", b"<scxml/>");

        let set = SourceSet::collect(root.path(), None).unwrap();
        assert!(set.covers(&member), "the file the set read is covered");
        assert!(
            !set.covers(&copy),
            "a byte-identical copy outside the root must not be covered"
        );

        // And the reason it matters: the copy's edit does not reach the digest.
        let before = set.digest();
        fs::write(&copy, b"<scxml name=\"edited\"/>").unwrap();
        let after = SourceSet::collect(root.path(), None).unwrap().digest();
        assert_eq!(before, after, "the copy was never part of the digest");
    }

    #[test]
    fn source_set_rejects_unreadable_document() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "doc.scxml", b"<scxml/>");
        let set = SourceSet::collect(dir.path(), None).unwrap();
        assert!(!set.covers(&dir.path().join("no-such-file.scxml")));
    }

    #[test]
    fn source_hash_includes_deploy_yaml_when_given() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "a.scxml", b"<scxml/>");
        let deploy = write_file(root, "deploy.yaml", b"machines: {}");
        let h_with = compute_source_hash(root, Some(&deploy)).unwrap();
        let h_without = compute_source_hash(root, None).unwrap();
        assert_ne!(
            h_with, h_without,
            "deploy.yaml inclusion must affect source-hash"
        );
    }

    /// A header of the digest `[byte; 32]` over `seeds`.
    fn header_of(byte: u8, seeds: &[&str]) -> DriftHeader {
        DriftHeader {
            source_hash: [byte; 32],
            seeds: seeds.iter().map(|seed| seed.to_string()).collect(),
        }
    }

    #[test]
    fn render_header_emits_the_banner_the_source_hash_and_the_seeds() {
        let header = header_of(0xaa, &["a.scxml"]);
        let h = render_header(&header, "//");
        let lines: Vec<&str> = h.lines().collect();
        assert_eq!(lines.len(), 3, "{h}");
        assert!(lines[0].contains("SCE-GENERATED"));
        assert!(lines[0].contains("DO NOT EDIT"));
        assert_eq!(lines[1], format!("// source-hash: {}", "aa".repeat(32)));
        assert_eq!(lines[2], "// source: a.scxml");
        // Header must end with newline so following template content
        // starts on a clean line.
        assert!(h.ends_with('\n'));
    }

    #[test]
    fn render_header_uses_hash_prefix_for_python() {
        let header = header_of(0, &["a.scxml"]);
        let h = render_header(&header, "#");
        let first = h.lines().next().unwrap();
        assert!(first.starts_with("# SCE-GENERATED"));
        assert!(h.contains("# source-hash: "));
        assert!(h.contains("# source: a.scxml\n"));
    }

    #[test]
    fn parse_embedded_header_round_trip() {
        let header = header_of(0x12, &["a.scxml"]);
        let rendered = render_header(&header, "//");
        // Add some body content after the header so parse only consumes
        // the relevant lines.
        let file_content = format!("{rendered}\npub mod whatever {{}}\n");
        let parsed = parse_embedded_header(&file_content).expect("header parseable");
        assert_eq!(parsed.source_hash_hex, header.source_hex());
        assert_eq!(parsed.seeds, header.seeds);
    }

    #[test]
    fn parse_embedded_header_round_trip_python() {
        let header = header_of(0xab, &["a.scxml"]);
        let rendered = render_header(&header, "#");
        let file_content = format!("{rendered}def main():\n    pass\n");
        let parsed = parse_embedded_header(&file_content).expect("python header parseable");
        assert_eq!(parsed.source_hash_hex, header.source_hex());
        assert_eq!(parsed.seeds, header.seeds);
    }

    /// ADR 0094 test 15: a file generated from a set of documents names each,
    /// and the parser reads them all, past any window a header of two lines
    /// fitted in. A header replaced in place leaves the body as it was.
    #[test]
    fn the_header_round_trips_its_seeds() {
        let seeds: Vec<String> = (0..20).map(|n| format!("set/doc_{n:02}.scxml")).collect();
        let named: Vec<&str> = seeds.iter().map(String::as_str).collect();
        let header = header_of(0x42, &named);
        let body = "pub fn body() {}\n";
        let headed = prepend_or_replace_header(body, &header, "//");
        let parsed = parse_embedded_header(&headed).expect("a long header parses");
        assert_eq!(parsed.seeds, seeds);
        assert_eq!(parsed.source_hash_hex, header.source_hex());

        let fewer = header_of(0x43, &["one.scxml"]);
        assert_eq!(
            prepend_or_replace_header(&headed, &fewer, "//"),
            format!("{}{body}", render_header(&fewer, "//")),
            "every seed line of the old header goes with it"
        );
    }

    /// A header of the shape before ADR 0094 parses with no seeds, which is
    /// how `verify` tells it apart.
    #[test]
    fn a_header_without_seeds_parses_with_none() {
        let two_lines = format!(
            "// {HEADER_BANNER}\n// source-hash: {}\nbody\n",
            "ab".repeat(32)
        );
        let parsed = parse_embedded_header(&two_lines).expect("parses");
        assert!(parsed.seeds.is_empty());
    }

    #[test]
    fn parse_embedded_header_none_when_banner_missing() {
        let bogus = "// just a regular comment\n// source-hash: aaaa\n";
        assert!(parse_embedded_header(bogus).is_none());
    }

    #[test]
    fn parse_embedded_header_reads_the_older_shape() {
        let legacy = format!(
            "// {HEADER_BANNER}\n// source-hash: {s}\n// template-hash: {t}\n\
             // generated-at: 0\npub fn f() {{}}\n",
            s = "12".repeat(32),
            t = "34".repeat(32),
        );
        let parsed = parse_embedded_header(&legacy).expect("legacy header parseable");
        assert_eq!(parsed.source_hash_hex, "12".repeat(32));
    }

    #[test]
    fn prepend_header_idempotent_on_double_run() {
        let header = header_of(0x33, &["a.scxml"]);
        let body = "pub fn foo() {}\n";
        let once = prepend_or_replace_header(body, &header, "//");
        let twice = prepend_or_replace_header(&once, &header, "//");
        assert_eq!(once, twice, "header injection must be idempotent");
    }

    #[test]
    fn prepend_header_replaces_when_the_source_hash_changes() {
        let h1 = header_of(0x11, &["a.scxml"]);
        let h2 = header_of(0x55, &["a.scxml"]);
        let body = "pub fn bar() {}\n";
        let first = prepend_or_replace_header(body, &h1, "//");
        let updated = prepend_or_replace_header(&first, &h2, "//");
        let parsed = parse_embedded_header(&updated).expect("re-parse after replacement");
        assert_eq!(parsed.source_hash_hex, h2.source_hex());
        assert_eq!(updated, format!("{}{body}", render_header(&h2, "//")));
    }

    /// A file generated before the header lost `template-hash` and
    /// `generated-at` is rewritten in the current shape: one header, and
    /// the body exactly as it was. Counting lines instead of reading keys
    /// would leave two stale key lines at the top of the body.
    #[test]
    fn prepend_header_replaces_a_header_of_the_older_shape() {
        let header = header_of(0x77, &["a.scxml"]);
        for prefix in ["//", "#"] {
            let body = format!("{prefix} GENERATED -- DO NOT EDIT (sce-codegen)\nbody line\n");
            let legacy = format!(
                "{prefix} {HEADER_BANNER}\n{prefix} source-hash: {s}\n\
                 {prefix} template-hash: {t}\n{prefix} generated-at: 0\n{body}",
                s = "11".repeat(32),
                t = "22".repeat(32),
            );
            assert_eq!(
                prepend_or_replace_header(&legacy, &header, prefix),
                format!("{}{body}", render_header(&header, prefix)),
                "prefix {prefix}"
            );
        }
    }

    #[test]
    fn comment_prefix_for_path_python_vs_default() {
        assert_eq!(comment_prefix_for_path(Path::new("foo.py")), "#");
        assert_eq!(comment_prefix_for_path(Path::new("foo.rs")), "//");
        assert_eq!(comment_prefix_for_path(Path::new("foo.cpp")), "//");
        assert_eq!(comment_prefix_for_path(Path::new("foo.h")), "//");
        assert_eq!(comment_prefix_for_path(Path::new("foo.kt")), "//");
        assert_eq!(comment_prefix_for_path(Path::new("foo.go")), "//");
        assert_eq!(comment_prefix_for_path(Path::new("foo.c")), "//");
    }

    #[test]
    fn header_banner_uses_the_specs_em_dash() {
        // Drift guard: §6.2.6 spells "SCE-GENERATED — DO NOT EDIT"
        // with a literal em-dash (U+2014). A regex-anchored verifier on
        // the consumer side will fail if this drifts to a hyphen.
        assert!(HEADER_BANNER.contains('\u{2014}'));
        assert_eq!(HEADER_BANNER, "SCE-GENERATED \u{2014} DO NOT EDIT");
    }

    // ── snapshot coherence ────────────────────────────────────────────
    //
    // The walk reads files one at a time. Without a coherence check a tree
    // written while it is read yields a digest assembled from two different
    // states of that tree — wrong, and silent. These tests drive the writer
    // from the `after_walk` hook so the mutation lands at the one instant
    // that matters: after the bytes were taken, before they are verified.
    //
    // Detection is exercised through file LENGTH changes and deletions, both
    // of which are decided by the filesystem's own bookkeeping. Timestamp
    // resolution never enters, so these do not become clock-dependent
    // fixtures; mtime is the additional signal that also covers a
    // same-length swap in production.

    fn scxml_keep(p: &Path) -> bool {
        p.extension().is_some_and(|e| e == "scxml")
    }

    /// Collect through the hooked entry point, running `mutate` once per
    /// completed traversal.
    fn walk_hooked(
        root: &Path,
        attempts: usize,
        mutate: &mut dyn FnMut(),
    ) -> Result<BTreeMap<PathBuf, [u8; 32]>, DriftHashError> {
        let mut out = BTreeMap::new();
        walk_filtered_coherent(
            root,
            root,
            &mut out,
            &scxml_keep,
            MAX_DIRECTORY_DESCENTS,
            attempts,
            mutate,
        )?;
        Ok(out)
    }

    #[test]
    fn quiescent_tree_is_read_in_a_single_attempt() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "a.scxml", b"<scxml/>");
        write_file(root, "nested/b.scxml", b"<scxml id='b'/>");

        let mut traversals = 0usize;
        let out = walk_hooked(root, MAX_SNAPSHOT_ATTEMPTS, &mut || traversals += 1)
            .expect("an untouched tree must not need a retry");

        assert_eq!(out.len(), 2, "both documents contribute");
        assert_eq!(
            traversals, 1,
            "a still tree costs exactly one traversal — the coherence check \
             must not turn every hash into repeated reads"
        );
    }

    #[test]
    fn file_rewritten_after_the_read_is_refused_not_hashed() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "a.scxml", b"<scxml/>");
        let victim = root.join("a.scxml");

        // One attempt, so the first detection is also the verdict.
        let err = walk_hooked(root, 1, &mut || {
            write_file(root, "a.scxml", b"<scxml id='rewritten-and-longer'/>");
        })
        .expect_err("bytes that no longer match the file must not be hashed");

        match err {
            DriftHashError::RootNotQuiescent { root: r, attempts } => {
                assert_eq!(r, root, "the refusal names the root it was given");
                assert_eq!(attempts, 1, "the refusal names the ceiling it hit");
            }
            other => panic!("expected RootNotQuiescent, got {other:?}"),
        }
        assert!(victim.exists(), "the test mutated rather than removed");
    }

    #[test]
    fn file_deleted_after_the_read_is_refused_not_hashed() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "a.scxml", b"<scxml/>");
        write_file(root, "b.scxml", b"<scxml id='b'/>");

        let err = walk_hooked(root, 1, &mut || {
            fs::remove_file(root.join("b.scxml")).unwrap();
        })
        .expect_err("a digest covering a file that is gone describes nothing");

        assert!(
            matches!(err, DriftHashError::RootNotQuiescent { .. }),
            "a vanished contributor is incoherence, not a generic IO fault: {err:?}"
        );
    }

    #[test]
    fn retry_rereads_and_reports_the_settled_state() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "a.scxml", b"<scxml/>");
        let settled = b"<scxml id='settled-content'/>".to_vec();

        // Mutate on the first traversal only, so the second one sees a still
        // tree — the shape a bursty parallel build actually produces.
        let mut traversals = 0usize;
        let settled_for_hook = settled.clone();
        let out = walk_hooked(root, 3, &mut || {
            traversals += 1;
            if traversals == 1 {
                write_file(root, "a.scxml", &settled_for_hook);
            }
        })
        .expect("a tree that settles must be readable");

        assert_eq!(traversals, 2, "exactly one retry was needed");
        assert_eq!(
            out.get(Path::new("a.scxml")),
            Some(&sha256_bytes(&settled)),
            "the retry must re-read; carrying the first attempt's digest \
             forward would report a state that no longer exists"
        );
        assert_eq!(
            out.len(),
            1,
            "each attempt starts from an empty set — a retry must not \
             accumulate entries from the state it rejected"
        );
    }

    #[test]
    fn vanished_entry_is_distinguished_from_a_wrong_root() {
        let root = Path::new("/nonexistent-root");
        let entry = root.join("child.scxml");
        let not_found = || std::io::Error::from(std::io::ErrorKind::NotFound);

        assert!(
            is_vanished_entry(
                &DriftHashError::Io {
                    path: entry,
                    source: not_found(),
                },
                root
            ),
            "an entry below the root going missing is the tree moving — retry"
        );
        assert!(
            !is_vanished_entry(
                &DriftHashError::Io {
                    path: root.to_path_buf(),
                    source: not_found(),
                },
                root
            ),
            "the root itself going missing is a wrong root — no retry can fix it"
        );
        assert!(
            !is_vanished_entry(
                &DriftHashError::Io {
                    path: root.join("child.scxml"),
                    source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
                },
                root
            ),
            "only NotFound is transient; a permission fault must surface"
        );
    }

    // ── input closures (ADR 0094) ─────────────────────────────────────

    const SCXML_OPEN: &str = "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\" \
                              xmlns:sce=\"http://sce.dev/ext\" version=\"1.0\"";

    /// A statechart whose body is `body`.
    fn chart(body: &str) -> String {
        format!("{SCXML_OPEN} initial=\"s\">\n{body}<state id=\"s\"/>\n</scxml>\n")
    }

    /// An `<sce:import>` of `src`.
    fn import(src: &str) -> String {
        format!("<sce:import kind=\"algorithm\" src=\"{src}\" as=\"A\"/>\n")
    }

    /// An algorithm-shaped document whose text carries `mark`, so two marks
    /// are two contents.
    fn algorithm(mark: &str, body: &str) -> String {
        format!("{SCXML_OPEN} sce:kind=\"algorithm\" name=\"{mark}\">\n{body}</scxml>\n")
    }

    fn closure_of(seed: &Path) -> InputClosure {
        InputClosure::of(&[seed], ClosureOptions::default())
            .unwrap_or_else(|e| panic!("the closure of {} is read: {e}", seed.display()))
    }

    /// ADR 0094 test 1, at the library: a file's closure moves with the
    /// documents it reads and not with a sibling. Today's rule, the
    /// directory set, moves both.
    #[test]
    fn a_closure_moves_with_what_it_reads_and_not_with_a_sibling() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let a = write_file(root, "a.scxml", algorithm("a", "").as_bytes());
        let b = write_file(root, "b.scxml", chart("").as_bytes());
        let c = write_file(root, "c.scxml", chart(&import("a.scxml")).as_bytes());
        let (b_before, c_before) = (closure_of(&b).digest(), closure_of(&c).digest());
        let dir_before = compute_source_hash(root, None).unwrap();

        fs::write(&a, algorithm("a_edited", "")).unwrap();

        assert_eq!(
            closure_of(&b).digest(),
            b_before,
            "b reads nothing of a, so an edit to a moves no hash of b"
        );
        assert_ne!(closure_of(&c).digest(), c_before, "c imports a");
        assert_ne!(
            compute_source_hash(root, None).unwrap(),
            dir_before,
            "the directory set moves for every file of the directory: the defect"
        );
        assert_eq!(closure_of(&b).seeds(), ["b.scxml"]);
    }

    /// ADR 0094 test 2: the closure follows an import's own imports.
    #[test]
    fn a_transitive_import_moves_the_importers_closure() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let leaf = write_file(root, "a2.scxml", algorithm("a2", "").as_bytes());
        write_file(
            root,
            "a.scxml",
            algorithm("a", &import("a2.scxml")).as_bytes(),
        );
        let c = write_file(root, "c.scxml", chart(&import("a.scxml")).as_bytes());
        let before = closure_of(&c).digest();

        fs::write(&leaf, algorithm("a2_edited", "")).unwrap();

        assert_ne!(closure_of(&c).digest(), before);
    }

    /// A fragment an `<xi:include>` splices: a container whose one child is
    /// the state `id`. An included root is replaced by its children, so a
    /// root with none is refused (docs/SCE_ACCEPTED_SUBSET.md) and the
    /// document that includes it never reaches a closure.
    fn fragment_of(id: &str) -> Vec<u8> {
        format!(
            "<state xmlns=\"http://www.w3.org/2005/07/scxml\" id=\"container\">\
             <state id=\"{id}\"/></state>\n"
        )
        .into_bytes()
    }

    /// ADR 0094 test 4: an import outside the document's directory and a
    /// fragment that is not `.scxml` are inputs, which the directory set
    /// never read.
    #[test]
    fn an_outside_import_and_a_fragment_that_is_not_scxml_move_the_closure() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let outside = write_file(root, "other/x.scxml", algorithm("x", "").as_bytes());
        let fragment = write_file(root, "doc/frag.xml", &fragment_of("included"));
        let body = format!(
            "{}<xi:include xmlns:xi=\"http://www.w3.org/2001/XInclude\" href=\"frag.xml\"/>\n",
            import("../other/x.scxml")
        );
        let main = write_file(root, "doc/main.scxml", chart(&body).as_bytes());
        let dir_before = compute_source_hash(&root.join("doc"), None).unwrap();
        let before = closure_of(&main).digest();

        fs::write(&outside, algorithm("x_edited", "")).unwrap();
        let after_import = closure_of(&main).digest();
        assert_ne!(after_import, before, "the import outside the directory");
        assert_eq!(
            compute_source_hash(&root.join("doc"), None).unwrap(),
            dir_before,
            "which the directory set does not read: the defect"
        );

        fs::write(&fragment, fragment_of("edited")).unwrap();
        assert_ne!(
            closure_of(&main).digest(),
            after_import,
            "the fragment that is not .scxml"
        );
    }

    /// ADR 0094 test 5: a copy staged outside the input root hashes as the
    /// document it was copied from, with the child the input root holds.
    #[test]
    fn a_staged_copy_hashes_as_the_document_it_was_copied_from() {
        let tracked = TempDir::new().unwrap();
        let stage = TempDir::new().unwrap();
        let doc_text = chart("<state id=\"run\"><invoke src=\"child.scxml\"/></state>\n");
        let original = write_file(tracked.path(), "doc.scxml", doc_text.as_bytes());
        write_file(tracked.path(), "child.scxml", chart("").as_bytes());
        let copy = write_file(stage.path(), "doc.scxml", doc_text.as_bytes());

        let from_original = closure_of(&original);
        let from_copy = InputClosure::of(
            &[copy.as_path()],
            ClosureOptions {
                input_root: Some(tracked.path()),
                ..ClosureOptions::default()
            },
        )
        .unwrap();
        assert_eq!(from_copy.digest(), from_original.digest());
        assert_eq!(from_copy.seeds(), from_original.seeds());
        assert_eq!(from_copy.seeds(), ["doc.scxml"]);
    }

    /// ADR 0094 test 6: a tree copied elsewhere whole has the same closures.
    #[test]
    fn the_closure_does_not_depend_on_where_the_tree_sits() {
        let digests: Vec<[u8; 32]> = (0..2)
            .map(|_| {
                let dir = TempDir::new().unwrap();
                let root = dir.path();
                write_file(root, "shared/a.scxml", algorithm("a", "").as_bytes());
                write_file(root, "doc/b.scxml", algorithm("b", "").as_bytes());
                let body = format!("{}{}", import("../shared/a.scxml"), import("b.scxml"));
                let main = write_file(root, "doc/main.scxml", chart(&body).as_bytes());
                closure_of(&main).digest()
            })
            .collect();
        assert_eq!(digests[0], digests[1]);
    }

    /// ADR 0094 test 12: `deploy.yaml` is in every closure of a run given one.
    #[test]
    fn deploy_yaml_is_in_every_closure_of_a_run_given_one() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let doc = write_file(root, "a.scxml", chart("").as_bytes());
        let deploy = write_file(root, "deploy.yaml", b"machines: {}");
        let with = InputClosure::of(
            &[doc.as_path()],
            ClosureOptions {
                deploy_yaml: Some(&deploy),
                ..ClosureOptions::default()
            },
        )
        .unwrap();
        assert_ne!(with.digest(), closure_of(&doc).digest());
        assert!(with.contributing_paths().contains(&deploy));
    }

    /// A file generated from several documents names each, from the deepest
    /// directory that holds them all.
    #[test]
    fn several_seeds_are_keyed_from_the_directory_that_holds_them() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let left = write_file(root, "left/a.scxml", chart("").as_bytes());
        let right = write_file(root, "right/b.scxml", chart("").as_bytes());
        let both = InputClosure::of(
            &[right.as_path(), left.as_path()],
            ClosureOptions::default(),
        )
        .unwrap();
        assert_eq!(both.seeds(), ["left/a.scxml", "right/b.scxml"]);
    }

    /// A standard document is keyed by its library name, wherever it is read.
    #[test]
    fn a_standard_seed_is_keyed_by_its_library_name() {
        let seed = Path::new("sce:std/time/days_in_month.scxml");
        let closure = closure_of(seed);
        assert_eq!(closure.seeds(), ["sce:std/time/days_in_month.scxml"]);
        assert!(closure.holds_a_document());
        assert!(
            closure.contributing_paths().is_empty(),
            "a standard document is in the generator, not on disk"
        );
    }

    /// A seed that cannot be read is refused, never left out.
    #[test]
    fn a_seed_that_cannot_be_read_is_refused() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("absent.scxml");
        assert!(matches!(
            InputClosure::of(&[missing.as_path()], ClosureOptions::default()),
            Err(DriftHashError::Io { .. })
        ));
    }

    /// The set form holds every document under its directory and what they
    /// reach outside it.
    #[test]
    fn the_set_form_covers_its_directory_and_what_its_documents_reach() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        let outside = write_file(root, "other/x.scxml", algorithm("x", "").as_bytes());
        let inside = write_file(root, "set/b.scxml", chart("").as_bytes());
        write_file(
            root,
            "set/a.scxml",
            chart(&import("../other/x.scxml")).as_bytes(),
        );
        let set = root.join("set");
        let first = InputClosure::of_directory(&set, ClosureOptions::default()).unwrap();
        assert_eq!(first.seeds(), [SET_FORM_SEED]);

        fs::write(&outside, algorithm("x_edited", "")).unwrap();
        let second = InputClosure::of_directory(&set, ClosureOptions::default()).unwrap();
        assert_ne!(second.digest(), first.digest(), "what a document reaches");

        fs::write(&inside, chart("<state id=\"t\"/>\n")).unwrap();
        let third = InputClosure::of_directory(&set, ClosureOptions::default()).unwrap();
        assert_ne!(third.digest(), second.digest(), "any document of the set");

        let empty = TempDir::new().unwrap();
        assert!(
            !InputClosure::of_directory(empty.path(), ClosureOptions::default())
                .unwrap()
                .holds_a_document()
        );
    }

    // ── written children (ADR 0094, owner's answer 2) ─────────────────

    /// The origin a run records in a child it writes reads back, behind an
    /// XML declaration or without one, and adds no line: every row of the
    /// child is the row it was.
    #[test]
    fn a_written_child_records_its_origin_on_its_first_line() {
        for text in [
            "<?xml version=\"1.0\"?>\n\n<scxml xmlns=\"http://www.w3.org/2005/07/scxml\"/>\n",
            "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\"/>\n",
        ] {
            let recorded = record_origin(text, "parent.scxml");
            assert_eq!(recorded_origin(&recorded).as_deref(), Some("parent.scxml"));
            assert_eq!(
                recorded.lines().count(),
                text.lines().count(),
                "the instruction shares the first line: {recorded}"
            );
            assert_eq!(
                recorded_origin(text),
                None,
                "an authored document records none"
            );
        }
        assert_eq!(
            record_origin("<scxml/>", "a\"b.scxml"),
            "<scxml/>",
            "a name the instruction cannot hold records nothing"
        );
    }

    /// A child document a run wrote is seeded by the document it came from:
    /// its closure is that document's, found under the input root, and
    /// beside the child when it stands there. Found in neither place, the
    /// child is its own seed.
    #[test]
    fn a_written_child_is_seeded_by_the_document_it_came_from() {
        let tracked = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        write_file(tracked.path(), "a.scxml", algorithm("a", "").as_bytes());
        let parent = write_file(
            tracked.path(),
            "parent.scxml",
            chart(&import("a.scxml")).as_bytes(),
        );
        let child_text = record_origin(&chart(""), "parent.scxml");
        let child = write_file(
            out.path(),
            "parent__sce_synth_invoke__inner.scxml",
            child_text.as_bytes(),
        );

        let from_parent = closure_of(&parent);
        let from_child = InputClosure::of(
            &[child.as_path()],
            ClosureOptions {
                input_root: Some(tracked.path()),
                ..ClosureOptions::default()
            },
        )
        .unwrap();
        assert_eq!(from_child.digest(), from_parent.digest());
        assert_eq!(from_child.seeds(), ["parent.scxml"]);

        let beside = write_file(tracked.path(), "beside__child.scxml", child_text.as_bytes());
        assert_eq!(closure_of(&beside).digest(), from_parent.digest());

        let alone = closure_of(&child);
        assert_eq!(
            alone.seeds(),
            ["parent__sce_synth_invoke__inner.scxml"],
            "an origin found nowhere leaves the child its own seed"
        );
    }
}
