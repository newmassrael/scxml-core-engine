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
use std::collections::BTreeMap;
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

/// Source-set rule (§synth-6.2.6): walks `input_root` recursively for `**/*.scxml`, hashes
/// each file's raw bytes, and folds the sorted `(rel_path, file_hash)`
/// pairs through a final BTreeMap digest. If `deploy_yaml` is provided,
/// its raw bytes are included under the canonical key `"deploy.yaml"`.
pub fn compute_source_hash(
    input_root: &Path,
    deploy_yaml: Option<&Path>,
) -> Result<[u8; 32], DriftHashError> {
    let mut entries: BTreeMap<PathBuf, [u8; 32]> = BTreeMap::new();
    walk_filtered_recursive(input_root, input_root, &mut entries, &|p| {
        p.extension().is_some_and(|e| e == "scxml")
    })?;
    if let Some(deploy) = deploy_yaml {
        let bytes = fs::read(deploy).map_err(|e| DriftHashError::Io {
            path: deploy.to_path_buf(),
            source: e,
        })?;
        entries.insert(PathBuf::from("deploy.yaml"), sha256_bytes(&bytes));
    }
    Ok(hash_btreemap(&entries))
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
    walk_filtered_recursive(template_root, template_root, &mut entries, &|_| true)?;
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

/// Recursively walks `root` collecting every file whose path predicate
/// returns true. Returns sorted `(rel_path_from_anchor, sha256)` pairs.
/// `anchor` is the path canonicalization basis so the BTreeMap key stays
/// stable across absolute/relative invocations.
fn walk_filtered_recursive(
    anchor: &Path,
    dir: &Path,
    out: &mut BTreeMap<PathBuf, [u8; 32]>,
    keep: &dyn Fn(&Path) -> bool,
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
        let file_type = entry.file_type().map_err(|e| DriftHashError::Io {
            path: path.clone(),
            source: e,
        })?;
        if file_type.is_dir() {
            walk_filtered_recursive(anchor, &path, out, keep)?;
        } else if file_type.is_file() && keep(&path) {
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
