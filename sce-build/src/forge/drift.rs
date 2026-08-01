// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Generated-source drift detection per spec §synth-6.2.6
//! (`docs/spec/synth/rfc-sce-protocol-synthesis.md` lines 3496-3519).
//!
//! Every emitted file carries a 4-line header:
//!
//! ```text
//! // SCE-GENERATED — DO NOT EDIT
//! // source-hash: <sha256 of sorted input SCXML + deploy.yaml>
//! // template-hash: <sha256 of Cargo.lock + tools/codegen/templates tree>
//! // generated-at: <unix seconds, informational only>
//! ```
//!
//! `sce-codegen verify <out-dir>` recomputes both hashes from the current
//! source + template state and compares against the embedded values.
//! Mismatch fires `forge/source-hash-mismatch`.
//!
//! ## Design decisions
//!
//! - Per-file header is emitted verbatim (this module emits one block; see
//!   `render_header`). Python uses `#` comment prefix; everything else uses
//!   `//`.
//! - Hash shape: BTreeMap<PathBuf, sha256(content)> 2-level hash.
//!   Deterministic via BTreeMap iteration order; sub-file drift localization
//!   debugging-friendly because each file's individual digest is recoverable.
//! - Source set = recursive `**/*.scxml` from input root +
//!   optional `deploy.yaml` raw bytes (pre-XInclude). XInclude expansion is
//!   NOT applied — raw on-disk bytes drive the hash.
//! - `template-hash` = Cargo.lock + recursive sha256 over
//!   `tools/codegen/templates/**/*`. Cargo.lock substitutes for the
//!   spec's "sce-build binary" reference because compiled binary bytes are
//!   linker-non-deterministic.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Sentinel header banner. First line of every emitted file regardless of
/// backend; consumers detect SCE-generated provenance by matching this
/// prefix (`{comment_prefix} SCE-GENERATED`). The em-dash is intentional
/// per spec line 3502 verbatim.
pub const HEADER_BANNER: &str = "SCE-GENERATED \u{2014} DO NOT EDIT";

/// Pair of digests computed from the source + template state. Both are
/// embedded as hex strings in every generated file's §synth-6.2.6 header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DriftHashes {
    pub source_hash: [u8; 32],
    pub template_hash: [u8; 32],
}

impl DriftHashes {
    /// Hex-encoded `source-hash` value (64 lowercase hex chars).
    pub fn source_hex(&self) -> String {
        hex_encode(&self.source_hash)
    }

    /// Hex-encoded `template-hash` value (64 lowercase hex chars).
    pub fn template_hex(&self) -> String {
        hex_encode(&self.template_hash)
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
}

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
}

impl SourceSet {
    /// Source-set rule (§synth-6.2.6): walks `input_root` recursively for
    /// `**/*.scxml` and hashes each file's raw bytes. If `deploy_yaml` is
    /// provided, its raw bytes are included under the canonical key
    /// `"deploy.yaml"`.
    pub fn collect(input_root: &Path, deploy_yaml: Option<&Path>) -> Result<Self, DriftHashError> {
        let mut entries: BTreeMap<PathBuf, [u8; 32]> = BTreeMap::new();
        walk_filtered(input_root, input_root, &mut entries, &|p| {
            p.extension().is_some_and(|e| e == "scxml")
        })?;
        if let Some(deploy) = deploy_yaml {
            let bytes = fs::read(deploy).map_err(|e| DriftHashError::Io {
                path: deploy.to_path_buf(),
                source: e,
            })?;
            entries.insert(PathBuf::from("deploy.yaml"), sha256_bytes(&bytes));
        }
        Ok(Self {
            root: input_root.to_path_buf(),
            entries,
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

    /// Number of contributing files (`.scxml` plus `deploy.yaml`).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Did `path`'s bytes contribute to this set?
    ///
    /// Matched on content rather than path. The same document reaches the
    /// walk under different names depending on how the build addressed it —
    /// a sandbox link name, a canonicalized real path, a root-relative
    /// path — and comparing those spellings answers a question about path
    /// arithmetic, not the one that matters: whether these bytes are in
    /// the digest. Reading the file back costs one syscall per generate.
    ///
    /// Returns `false` when `path` cannot be read, which is the correct
    /// answer for the caller: bytes it cannot read are bytes it cannot
    /// prove contributed.
    pub fn covers(&self, path: &Path) -> bool {
        let Ok(bytes) = fs::read(path) else {
            return false;
        };
        let digest = sha256_bytes(&bytes);
        self.entries.values().any(|h| *h == digest)
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

/// Template-hash rule (§synth-6.2.6): walks `template_root` recursively for every file
/// (no extension filter — `.jinja2` + `.json` + `.md` + everything else
/// in the template tree contributes), hashes raw bytes, then folds
/// `Cargo.lock` into the same BTreeMap as the binary-identity surrogate.
pub fn compute_template_hash(
    template_root: &Path,
    cargo_lock: &Path,
) -> Result<[u8; 32], DriftHashError> {
    let mut entries: BTreeMap<PathBuf, [u8; 32]> = BTreeMap::new();
    walk_filtered(template_root, template_root, &mut entries, &|_| true)?;
    let lock_bytes = fs::read(cargo_lock).map_err(|e| DriftHashError::Io {
        path: cargo_lock.to_path_buf(),
        source: e,
    })?;
    entries.insert(PathBuf::from("Cargo.lock"), sha256_bytes(&lock_bytes));
    Ok(hash_btreemap(&entries))
}

/// Returns the `generated-at` timestamp value. Defaults to the current
/// unix seconds; honours `SOURCE_DATE_EPOCH` (Linux reproducible-builds
/// convention) when set. Per spec line 3505 the timestamp is
/// "informational only" — it does not feed either hash, so deterministic
/// regeneration via `SOURCE_DATE_EPOCH=0` produces byte-stable output.
pub fn now_utc_seconds() -> u64 {
    if let Ok(s) = std::env::var("SOURCE_DATE_EPOCH") {
        if let Ok(n) = s.parse::<u64>() {
            return n;
        }
    }
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Renders the 4-line §synth-6.2.6 header. `comment_prefix` is `//` for
/// Rust/Cpp/C11/Kotlin/Go and `#` for Python. Caller prepends the result
/// at the very top of each emitted file.
///
/// The output ends in `\n` so subsequent template content starts on a
/// fresh line.
pub fn render_header(hashes: &DriftHashes, generated_at_secs: u64, comment_prefix: &str) -> String {
    format!(
        "{cp} {banner}\n{cp} source-hash: {sh}\n{cp} template-hash: {th}\n{cp} generated-at: {ts}\n",
        cp = comment_prefix,
        banner = HEADER_BANNER,
        sh = hashes.source_hex(),
        th = hashes.template_hex(),
        ts = generated_at_secs,
    )
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
/// already-headered content — if `content` already begins with the
/// banner line, the existing header block is replaced rather than
/// duplicated. Idempotence matters for the production pipeline where
/// codegen may run multiple times for the same logical inputs.
pub fn prepend_or_replace_header(
    content: &str,
    hashes: &DriftHashes,
    generated_at_secs: u64,
    comment_prefix: &str,
) -> String {
    let header = render_header(hashes, generated_at_secs, comment_prefix);
    if has_existing_header(content) {
        // Strip the existing 4-header lines + any single trailing blank
        // line, then prepend the fresh header. The "single trailing
        // blank line" matters because we ourselves emit a `\n` at the
        // end of `render_header`; consecutive runs would otherwise
        // grow a phantom newline.
        let mut lines = content.lines();
        for _ in 0..4 {
            let _ = lines.next();
        }
        let rest = lines.collect::<Vec<_>>().join("\n");
        let suffix_newline = if content.ends_with('\n') { "\n" } else { "" };
        format!("{header}{rest}{suffix_newline}")
    } else {
        format!("{header}{content}")
    }
}

/// Cheap structural check: does the content already lead with the
/// SCE-GENERATED banner? Used by `prepend_or_replace_header` to keep
/// regeneration idempotent.
fn has_existing_header(content: &str) -> bool {
    let Some(first) = content.lines().next() else {
        return false;
    };
    first.contains(HEADER_BANNER)
}

/// Extracted hex strings from a generated file's embedded header. Hex
/// values are returned as-is (lowercase, 64 chars). Returns `None` if
/// the SCE-GENERATED banner is missing or any of the required hash
/// lines fails to parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedHashes {
    pub source_hash_hex: String,
    pub template_hash_hex: String,
}

/// Parses the §synth-6.2.6 header out of a generated file's content. Accepts
/// either `//` or `#` comment prefix. Tolerant of leading shebang line
/// or BOM (skips first line up to 2 if needed).
pub fn parse_embedded_hashes(content: &str) -> Option<EmbeddedHashes> {
    // The 4 header lines live within the first ~6 lines of the file (after
    // an optional shebang for python). Scan with a small window to keep
    // recovery cheap even if a future shape change adds an attribute line
    // before the banner.
    let mut source_hex: Option<String> = None;
    let mut template_hex: Option<String> = None;
    let mut saw_banner = false;
    for line in content.lines().take(12) {
        let trimmed = line.trim_start();
        let body = trimmed
            .strip_prefix("// ")
            .or_else(|| trimmed.strip_prefix("# "))?;
        if body.starts_with(HEADER_BANNER) {
            saw_banner = true;
            continue;
        }
        if let Some(rest) = body.strip_prefix("source-hash: ") {
            source_hex = Some(rest.trim().to_string());
        } else if let Some(rest) = body.strip_prefix("template-hash: ") {
            template_hex = Some(rest.trim().to_string());
        } else if body.starts_with("generated-at: ") {
            break;
        }
        if source_hex.is_some() && template_hex.is_some() && saw_banner {
            break;
        }
    }
    match (saw_banner, source_hex, template_hex) {
        (true, Some(s), Some(t)) => Some(EmbeddedHashes {
            source_hash_hex: s,
            template_hash_hex: t,
        }),
        _ => None,
    }
}

// ── internal helpers ──────────────────────────────────────────────────

/// Walks `root` collecting every file whose path predicate returns true.
/// Returns sorted `(rel_path_from_anchor, sha256)` pairs. `anchor` is the
/// path canonicalization basis so the BTreeMap key stays stable across
/// absolute/relative invocations.
fn walk_filtered(
    anchor: &Path,
    root: &Path,
    out: &mut BTreeMap<PathBuf, [u8; 32]>,
    keep: &dyn Fn(&Path) -> bool,
) -> Result<(), DriftHashError> {
    // Seeded with the root so a link naming the root is recognised as a
    // cycle by the same rule that catches one naming any other ancestor.
    let mut descent: BTreeSet<PathBuf> = BTreeSet::new();
    descent.insert(canonical_key(root));
    walk_filtered_recursive(anchor, root, out, keep, &mut descent)
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
) -> Result<(), DriftHashError> {
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
            let descended = walk_filtered_recursive(anchor, &path, out, keep, descent);
            descent.remove(&key);
            descended?;
        } else if target_type.is_file() && keep(&path) {
            let bytes = fs::read(&path).map_err(|e| DriftHashError::Io {
                path: path.clone(),
                source: e,
            })?;
            let rel = path.strip_prefix(anchor).unwrap_or(&path).to_path_buf();
            out.insert(rel, sha256_bytes(&bytes));
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

fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

/// Deterministic folding of the per-file hashes: emits a fixed-format
/// concatenation of `(len(path_bytes), path_bytes, hash_bytes)` for each
/// entry in BTreeMap order, then sha256s the result. Avoids a serde
/// crate dep and keeps the hash invariant under path-encoding (UTF-8
/// is required because BTreeMap iteration order is byte-stable on
/// `PathBuf`'s lossy str representation).
fn hash_btreemap(entries: &BTreeMap<PathBuf, [u8; 32]>) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for (path, hash) in entries {
        let path_str = path.to_string_lossy();
        let path_bytes = path_str.as_bytes();
        let len = path_bytes.len() as u64;
        hasher.update(len.to_le_bytes());
        hasher.update(path_bytes);
        hasher.update(hash);
    }
    hasher.finalize().into()
}

fn hex_encode(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

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

    #[test]
    fn source_hash_stable_under_insertion_order() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        write_file(root, "a.scxml", b"<scxml/>");
        write_file(root, "sub/b.scxml", b"<scxml/>");
        let h1 = compute_source_hash(root, None).unwrap();

        // Reorder-on-disk is impossible since we re-read the tree; the
        // sort invariant is enforced by BTreeMap, so successive calls
        // over the same tree produce identical hashes regardless of
        // filesystem readdir ordering.
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
    /// each names a distinct set of root-relative paths. The W3C tree does
    /// exactly this — `resources/403a`, `403b` and `403c` all name
    /// `resources/403`.
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

    /// Coverage is content-keyed, so a sandbox link name resolves against a
    /// set collected from the real tree and vice versa — the two spellings
    /// of the same document must both answer "covered".
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

    #[test]
    fn template_hash_changes_when_template_edited() {
        let dir = TempDir::new().unwrap();
        let tpl_root = dir.path().join("templates");
        let lock = dir.path().join("Cargo.lock");
        fs::write(&lock, b"# lock content\n").unwrap();
        write_file(&tpl_root, "state_machine.jinja2", b"hello {{ name }}");
        let h_pre = compute_template_hash(&tpl_root, &lock).unwrap();

        fs::write(tpl_root.join("state_machine.jinja2"), b"hello {{ other }}").unwrap();
        let h_post = compute_template_hash(&tpl_root, &lock).unwrap();
        assert_ne!(h_pre, h_post);
    }

    #[test]
    fn template_hash_changes_when_lock_edited() {
        let dir = TempDir::new().unwrap();
        let tpl_root = dir.path().join("templates");
        fs::create_dir_all(&tpl_root).unwrap();
        let lock = dir.path().join("Cargo.lock");
        fs::write(&lock, b"# v1\n").unwrap();
        let h_pre = compute_template_hash(&tpl_root, &lock).unwrap();

        fs::write(&lock, b"# v2\n").unwrap();
        let h_post = compute_template_hash(&tpl_root, &lock).unwrap();
        assert_ne!(h_pre, h_post);
    }

    #[test]
    fn render_header_emits_4_lines() {
        let hashes = DriftHashes {
            source_hash: [0xaa; 32],
            template_hash: [0xbb; 32],
        };
        let h = render_header(&hashes, 1715731200, "//");
        let lines: Vec<&str> = h.lines().collect();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].contains("SCE-GENERATED"));
        assert!(lines[0].contains("DO NOT EDIT"));
        assert!(lines[1].starts_with("// source-hash: aaaa"));
        assert!(lines[2].starts_with("// template-hash: bbbb"));
        assert_eq!(lines[3], "// generated-at: 1715731200");
        // Header must end with newline so following template content
        // starts on a clean line.
        assert!(h.ends_with('\n'));
    }

    #[test]
    fn render_header_uses_hash_prefix_for_python() {
        let hashes = DriftHashes {
            source_hash: [0; 32],
            template_hash: [0; 32],
        };
        let h = render_header(&hashes, 0, "#");
        let first = h.lines().next().unwrap();
        assert!(first.starts_with("# SCE-GENERATED"));
        assert!(h.contains("# source-hash: "));
        assert!(h.contains("# template-hash: "));
        assert!(h.contains("# generated-at: 0"));
    }

    #[test]
    fn parse_embedded_hashes_round_trip() {
        let hashes = DriftHashes {
            source_hash: [0x12; 32],
            template_hash: [0x34; 32],
        };
        let header = render_header(&hashes, 100, "//");
        // Add some body content after the header so parse only consumes
        // the relevant lines.
        let file_content = format!("{header}\npub mod whatever {{}}\n");
        let parsed = parse_embedded_hashes(&file_content).expect("header parseable");
        assert_eq!(parsed.source_hash_hex, hashes.source_hex());
        assert_eq!(parsed.template_hash_hex, hashes.template_hex());
    }

    #[test]
    fn parse_embedded_hashes_round_trip_python() {
        let hashes = DriftHashes {
            source_hash: [0xab; 32],
            template_hash: [0xcd; 32],
        };
        let header = render_header(&hashes, 200, "#");
        let file_content = format!("{header}def main():\n    pass\n");
        let parsed = parse_embedded_hashes(&file_content).expect("python header parseable");
        assert_eq!(parsed.source_hash_hex, hashes.source_hex());
        assert_eq!(parsed.template_hash_hex, hashes.template_hex());
    }

    #[test]
    fn parse_embedded_hashes_none_when_banner_missing() {
        let bogus = "// just a regular comment\n// source-hash: aaaa\n// template-hash: bbbb\n";
        assert!(parse_embedded_hashes(bogus).is_none());
    }

    #[test]
    fn now_utc_seconds_honors_source_date_epoch() {
        // SAFETY: env var manipulation is process-global; the test sets
        // and immediately restores. Other tests that read this var must
        // tolerate restoration (they read the value once, not racingly).
        let prev = std::env::var("SOURCE_DATE_EPOCH").ok();
        // SAFETY: env var manipulation in tests; isolated per-process by
        // cargo test default thread model.
        unsafe {
            std::env::set_var("SOURCE_DATE_EPOCH", "42");
        }
        assert_eq!(now_utc_seconds(), 42);
        match prev {
            Some(v) => unsafe { std::env::set_var("SOURCE_DATE_EPOCH", v) },
            None => unsafe { std::env::remove_var("SOURCE_DATE_EPOCH") },
        }
    }

    #[test]
    fn prepend_header_idempotent_on_double_run() {
        let hashes = DriftHashes {
            source_hash: [0x33; 32],
            template_hash: [0x44; 32],
        };
        let body = "pub fn foo() {}\n";
        let once = prepend_or_replace_header(body, &hashes, 100, "//");
        let twice = prepend_or_replace_header(&once, &hashes, 100, "//");
        assert_eq!(once, twice, "header injection must be idempotent");
    }

    #[test]
    fn prepend_header_replaces_when_hashes_change() {
        let h1 = DriftHashes {
            source_hash: [0x11; 32],
            template_hash: [0x22; 32],
        };
        let h2 = DriftHashes {
            source_hash: [0x55; 32],
            template_hash: [0x66; 32],
        };
        let body = "pub fn bar() {}\n";
        let first = prepend_or_replace_header(body, &h1, 100, "//");
        let updated = prepend_or_replace_header(&first, &h2, 100, "//");
        let parsed = parse_embedded_hashes(&updated).expect("re-parse after replacement");
        assert_eq!(parsed.source_hash_hex, h2.source_hex());
        assert_eq!(parsed.template_hash_hex, h2.template_hex());
        // Body must still end with the original function definition.
        assert!(updated.contains("pub fn bar() {}"));
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
    fn header_banner_uses_em_dash_per_spec_line_3502() {
        // Drift guard: spec line 3502 spells "SCE-GENERATED — DO NOT EDIT"
        // with a literal em-dash (U+2014). A regex-anchored verifier on
        // the consumer side will fail if this drifts to a hyphen.
        assert!(HEADER_BANNER.contains('\u{2014}'));
        assert_eq!(HEADER_BANNER, "SCE-GENERATED \u{2014} DO NOT EDIT");
    }
}
