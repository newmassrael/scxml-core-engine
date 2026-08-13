// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//
// Content witness binding the `sce-codegen` binary to the sources it was
// built from. The module's documentation lives on its `pub mod`
// declaration in `lib.rs`, because `sce-build/build.rs` `include!`s this
// file and an inner doc comment cannot survive that expansion.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Individual files in the witness set, relative to the workspace root.
///
/// `build.rs` is here because it *is* generator behaviour: it derives the
/// embedded template registry and the provenance stamps, so editing it
/// changes emitted output without touching a single file under `src/`.
pub const WITNESS_FILES: &[&str] = &["Cargo.lock", "sce-build/Cargo.toml", "sce-build/build.rs"];

/// Directories walked recursively, relative to the workspace root. Every
/// file contributes regardless of extension — the same rule
/// [`crate::forge::drift::compute_template_hash`] applies to the template
/// tree, so a data file added next to the Rust sources cannot enter the
/// binary through `include_str!` without entering the witness too.
pub const WITNESS_TREES: &[&str] = &["sce-build/src"];

/// Value [`crate::GENERATOR_SOURCE_DIGEST`] carries when the build could
/// not read the witness set — a vendored crate or a release tarball with
/// no workspace `Cargo.lock` beside it. Distinct from any real digest, so
/// a consumer can tell "this binary cannot be checked" apart from "this
/// binary disagrees with the tree". Conflating those two is what
/// `gate_blamed_the_author_for_its_own_missing_tool` cost.
pub const DIGEST_UNAVAILABLE: &str = "unavailable";

/// Directory descents allowed before the walk gives up. The witness tree
/// is this repository's own source directory, so the bound exists only to
/// turn a symlink cycle into a named refusal instead of a hang.
const MAX_DESCENTS: usize = 10_000;

/// Why a witness could not be computed. Carries the offending path so the
/// caller reports which member of the set it failed on rather than
/// reporting the set.
#[derive(Debug)]
pub enum WitnessError {
    /// A member of the witness set could not be read.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The walk exceeded [`MAX_DESCENTS`], which for a tree this shape
    /// means a symlink cycle.
    TooDeep { root: PathBuf },
}

impl std::fmt::Display for WitnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WitnessError::Io { path, source } => {
                write!(f, "cannot read witness member {}: {source}", path.display())
            }
            WitnessError::TooDeep { root } => write!(
                f,
                "witness walk under {} exceeded {MAX_DESCENTS} directories — symlink cycle?",
                root.display()
            ),
        }
    }
}

impl std::error::Error for WitnessError {}

/// Every file in the witness set, as paths under `workspace_root`, sorted.
///
/// `build.rs` turns this into one `cargo:rerun-if-changed` line per entry.
/// Watching the set file-by-file is not optional: cargo watches a
/// *directory* by its mtime, which moves when a file is added or removed
/// and not when one is edited — so a directory watch would leave the
/// embedded digest describing a tree that had since changed, and the check
/// would then refuse a binary that was actually current.
pub fn witness_paths(workspace_root: &Path) -> Result<Vec<PathBuf>, WitnessError> {
    Ok(collect(workspace_root)?.into_keys().collect())
}

/// Hex-encoded sha256 over the witness set's contents.
pub fn digest_hex(workspace_root: &Path) -> Result<String, WitnessError> {
    let entries = collect(workspace_root)?;
    let relative: BTreeMap<PathBuf, [u8; 32]> = entries
        .into_iter()
        .map(|(path, hash)| {
            let key = path
                .strip_prefix(workspace_root)
                .unwrap_or(&path)
                .to_path_buf();
            (key, hash)
        })
        .collect();
    Ok(hex_encode(&hash_btreemap(&relative)))
}

/// Absolute path → content hash for every member of the set.
///
/// Keyed by the on-disk path so [`witness_paths`] can hand cargo something
/// it can watch; [`digest_hex`] re-keys by the workspace-relative path so
/// the digest is a function of the tree and not of where it was checked
/// out. A build machine holds this repository under a different prefix
/// than the machine that authored the change, and the two must agree.
fn collect(workspace_root: &Path) -> Result<BTreeMap<PathBuf, [u8; 32]>, WitnessError> {
    let mut entries: BTreeMap<PathBuf, [u8; 32]> = BTreeMap::new();
    for file in WITNESS_FILES {
        let path = workspace_root.join(file);
        entries.insert(path.clone(), hash_file(&path)?);
    }
    for tree in WITNESS_TREES {
        let root = workspace_root.join(tree);
        let mut budget = MAX_DESCENTS;
        walk(&root, &root, &mut entries, &mut budget)?;
    }
    Ok(entries)
}

fn walk(
    root: &Path,
    current: &Path,
    out: &mut BTreeMap<PathBuf, [u8; 32]>,
    budget: &mut usize,
) -> Result<(), WitnessError> {
    if *budget == 0 {
        return Err(WitnessError::TooDeep {
            root: root.to_path_buf(),
        });
    }
    *budget -= 1;
    let dir = fs::read_dir(current).map_err(|source| WitnessError::Io {
        path: current.to_path_buf(),
        source,
    })?;
    for entry in dir {
        let entry = entry.map_err(|source| WitnessError::Io {
            path: current.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|source| WitnessError::Io {
            path: path.clone(),
            source,
        })?;
        if kind.is_dir() {
            walk(root, &path, out, budget)?;
        } else {
            let hash = hash_file(&path)?;
            out.insert(path, hash);
        }
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<[u8; 32], WitnessError> {
    let bytes = fs::read(path).map_err(|source| WitnessError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(sha256_bytes(&bytes))
}

/// sha256 of a byte slice.
///
/// Lives here rather than in [`crate::forge::drift`] because `build.rs`
/// `include!`s this file and cannot reach the library — and the drift
/// module needs the identical fold, so stating it twice would put the
/// drift-header `template-hash` one careless edit away from silently
/// moving under every committed generated header.
pub(crate) fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
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
///
/// The length prefix is what makes the fold injective: without it
/// `{"ab" -> h}` and `{"a" -> ..., "b" -> ...}` could produce the same
/// byte stream.
pub(crate) fn hash_btreemap(entries: &BTreeMap<PathBuf, [u8; 32]>) -> [u8; 32] {
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

pub(crate) fn hex_encode(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

// ── tests ─────────────────────────────────────────────────────────────
//
// `#[cfg(test)]` is stripped before name resolution in a build script, so
// this block is invisible to the `include!` in `build.rs` and may use the
// crate's dev-dependencies freely.
#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal workspace carrying one file per member of the witness set.
    fn fake_workspace(dir: &Path) {
        fs::create_dir_all(dir.join("sce-build/src/forge")).unwrap();
        fs::write(dir.join("Cargo.lock"), b"[[package]]\nname = \"x\"\n").unwrap();
        fs::write(dir.join("sce-build/Cargo.toml"), b"[package]\n").unwrap();
        fs::write(dir.join("sce-build/build.rs"), b"fn main() {}\n").unwrap();
        fs::write(dir.join("sce-build/src/lib.rs"), b"pub mod forge;\n").unwrap();
        fs::write(dir.join("sce-build/src/forge/mod.rs"), b"// forge\n").unwrap();
    }

    #[test]
    fn a_one_byte_source_edit_moves_the_digest() {
        let tmp = tempfile::tempdir().unwrap();
        fake_workspace(tmp.path());
        let before = digest_hex(tmp.path()).unwrap();

        fs::write(tmp.path().join("sce-build/src/forge/mod.rs"), b"// forgE\n").unwrap();
        let after = digest_hex(tmp.path()).unwrap();

        assert_ne!(
            before, after,
            "a changed generator source left the witness unmoved"
        );
    }

    /// The property the whole check rests on across a build machine: two
    /// checkouts of the same content at different prefixes must agree.
    ///
    /// The digest is keyed by the workspace-relative path for exactly this
    /// reason. Keyed by the absolute path it would differ between the
    /// authoring machine and the build machine on every run, which is the
    /// shape of false refusal that got the previous mtime-based attempt
    /// reverted — it too was "correct" locally and wrong across the
    /// transfer.
    #[test]
    fn the_digest_does_not_depend_on_where_the_tree_is_checked_out() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        fake_workspace(a.path());
        fake_workspace(b.path());

        assert_ne!(a.path(), b.path(), "the two checkouts must differ in path");
        assert_eq!(
            digest_hex(a.path()).unwrap(),
            digest_hex(b.path()).unwrap(),
            "the witness read the checkout location into the digest"
        );
    }

    /// Deletion, not edit: a source file that vanishes is the direction an
    /// edit-only probe cannot reach, and the fold's length prefix is what
    /// makes it distinguishable at all.
    #[test]
    fn removing_a_source_file_moves_the_digest() {
        let tmp = tempfile::tempdir().unwrap();
        fake_workspace(tmp.path());
        let before = digest_hex(tmp.path()).unwrap();

        fs::remove_file(tmp.path().join("sce-build/src/forge/mod.rs")).unwrap();
        let after = digest_hex(tmp.path()).unwrap();

        assert_ne!(before, after, "a deleted generator source left no trace");
    }

    #[test]
    fn adding_a_source_file_moves_the_digest() {
        let tmp = tempfile::tempdir().unwrap();
        fake_workspace(tmp.path());
        let before = digest_hex(tmp.path()).unwrap();

        fs::write(tmp.path().join("sce-build/src/forge/new.rs"), b"").unwrap();
        let after = digest_hex(tmp.path()).unwrap();

        assert_ne!(
            before, after,
            "an added generator source left the witness unmoved"
        );
    }

    /// The omission that keeps the check from refusing correct builds.
    ///
    /// Native `sce-codegen` reads the Jinja2 tree from disk per run, so an
    /// edited template is already in effect for the binary CMake invokes.
    /// Folding the template tree into this digest would demand a rebuild
    /// that changes nothing — a false refusal, and the exact reason the
    /// mtime attempt had to be reverted. Template drift is covered by the
    /// drift-header `template-hash` instead.
    #[test]
    fn editing_a_template_does_not_move_the_digest() {
        let tmp = tempfile::tempdir().unwrap();
        fake_workspace(tmp.path());
        let templates = tmp.path().join("tools/codegen/templates");
        fs::create_dir_all(&templates).unwrap();
        fs::write(templates.join("state_machine.jinja2"), b"{{ a }}").unwrap();
        let before = digest_hex(tmp.path()).unwrap();

        fs::write(templates.join("state_machine.jinja2"), b"{{ b }}").unwrap();

        assert_eq!(
            before,
            digest_hex(tmp.path()).unwrap(),
            "a template edit demanded a generator rebuild it does not need"
        );
    }

    /// `build.rs` turns this list into the `cargo:rerun-if-changed` set. If
    /// it named fewer files than the digest folds, an edit to the
    /// difference would leave the embedded digest describing a tree that
    /// had moved — and the check would then refuse a current generator.
    #[test]
    fn every_file_the_digest_folds_is_a_file_the_build_watches() {
        let tmp = tempfile::tempdir().unwrap();
        fake_workspace(tmp.path());

        let watched = witness_paths(tmp.path()).unwrap();
        let mut expected: Vec<PathBuf> = [
            "Cargo.lock",
            "sce-build/Cargo.toml",
            "sce-build/build.rs",
            "sce-build/src/lib.rs",
            "sce-build/src/forge/mod.rs",
        ]
        .iter()
        .map(|p| tmp.path().join(p))
        .collect();
        expected.sort();

        assert_eq!(watched, expected);

        // Every watched path must move the digest when it changes —
        // the list is a claim about causation, not a directory listing.
        for path in &watched {
            let before = digest_hex(tmp.path()).unwrap();
            let original = fs::read(path).unwrap();
            let mut edited = original.clone();
            edited.push(b'!');
            fs::write(path, &edited).unwrap();
            assert_ne!(
                before,
                digest_hex(tmp.path()).unwrap(),
                "{} is watched but does not contribute to the digest",
                path.display()
            );
            fs::write(path, &original).unwrap();
        }
    }

    #[test]
    fn a_missing_witness_member_is_named_rather_than_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        fake_workspace(tmp.path());
        fs::remove_file(tmp.path().join("Cargo.lock")).unwrap();

        let err = digest_hex(tmp.path()).expect_err("a vanished member must not fold to a digest");
        assert!(
            err.to_string().contains("Cargo.lock"),
            "error names the set instead of the member: {err}"
        );
    }

    /// The fold is injective over its keys. Without the length prefix
    /// `{"ab" -> h}` and a two-entry map splitting the same bytes would
    /// hash alike, and a rename could pass unnoticed.
    #[test]
    fn the_fold_separates_a_key_from_its_split() {
        let one: BTreeMap<PathBuf, [u8; 32]> =
            [(PathBuf::from("ab"), [7u8; 32])].into_iter().collect();
        let two: BTreeMap<PathBuf, [u8; 32]> = [
            (PathBuf::from("a"), [7u8; 32]),
            (PathBuf::from("b"), [7u8; 32]),
        ]
        .into_iter()
        .collect();

        assert_ne!(hash_btreemap(&one), hash_btreemap(&two));
    }

    /// This repository's own tree is the input the check runs against in
    /// production; a digest that cannot be taken of it is a check that
    /// never runs.
    #[test]
    fn this_repository_yields_a_digest() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let digest = digest_hex(&root).expect("the workspace this crate lives in is the witness");
        assert_eq!(digest.len(), 64, "digest is not a sha256 hex string");
        assert_ne!(digest, DIGEST_UNAVAILABLE);
        assert_eq!(
            digest,
            digest_hex(&root).unwrap(),
            "two reads of one unchanged tree disagreed"
        );
    }
}
