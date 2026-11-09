// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The two tables of an action cache (ADR 0097, section 3): the action cache,
//! from an action key to the [`ActionResult`] it produced, and the
//! content-addressed store, from a digest to the bytes it names.
//!
//! [`ContentStore`] is the interface both the local store and the fleet's
//! REAPI cache answer, so a caller does not know which one holds an entry.
//! [`LocalStore`] is the local one: a directory of the user's cache, shared by
//! every checkout and worktree of the machine.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use prost::Message;

use super::reapi::{ActionResult, Digest};

/// What an action cache and its content store answer.
pub trait ContentStore {
    /// The digests of `digests` the store does not hold.
    fn find_missing(&self, digests: &[Digest]) -> io::Result<Vec<Digest>>;
    /// The bytes `digest` names, checked against it, or `None` when the store
    /// does not hold them.
    fn read_blob(&self, digest: &Digest) -> io::Result<Option<Vec<u8>>>;
    /// Store `bytes` and return their digest.
    fn write_blob(&self, bytes: &[u8]) -> io::Result<Digest>;
    /// The result recorded for the action whose key is `action`.
    fn action_result(&self, action: &Digest) -> io::Result<Option<ActionResult>>;
    /// Record `result` for `action`. Every blob it names must be stored
    /// first: an entry whose outputs are missing is a miss to every reader.
    fn record_action_result(&self, action: &Digest, result: &ActionResult) -> io::Result<()>;
}

/// The local action cache: `<root>/cas/<hash>` for blobs and
/// `<root>/ac/<hash>` for encoded results, each fanned out under the hash's
/// first two characters.
pub struct LocalStore {
    root: PathBuf,
}

impl LocalStore {
    /// A store rooted at `root`, created on first write.
    pub fn at(root: impl Into<PathBuf>) -> LocalStore {
        LocalStore { root: root.into() }
    }

    /// The machine's store: `$SCE_ACTION_CACHE` when it is set, else
    /// `$XDG_CACHE_HOME/sce/actions`, else `$HOME/.cache/sce/actions`.
    /// `None` when none of the three is set, which is a machine with no
    /// cache directory to share.
    pub fn for_this_machine() -> Option<LocalStore> {
        let from = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
        if let Some(dir) = from("SCE_ACTION_CACHE") {
            return Some(LocalStore::at(PathBuf::from(dir)));
        }
        if let Some(dir) = from("XDG_CACHE_HOME") {
            return Some(LocalStore::at(PathBuf::from(dir).join("sce/actions")));
        }
        from("HOME").map(|home| LocalStore::at(PathBuf::from(home).join(".cache/sce/actions")))
    }

    /// The directory this store keeps its tables in.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn path(&self, table: &str, digest: &Digest) -> io::Result<PathBuf> {
        if !digest.is_well_formed() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{digest} is not a SHA-256 digest"),
            ));
        }
        Ok(self
            .root
            .join(table)
            .join(&digest.hash[..2])
            .join(&digest.hash))
    }

    /// Write `bytes` to `path` so that a reader sees either no file or the
    /// whole one: into a file beside it that no other writer names (this
    /// process's id and a counter of its writes), then renamed over it.
    fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
        static WRITES: AtomicU64 = AtomicU64::new(0);
        let dir = path.parent().expect("a table path has a parent");
        fs::create_dir_all(dir)?;
        let staged = dir.join(format!(
            ".{}.{}.{}.partial",
            path.file_name()
                .and_then(|n| n.to_str())
                .expect("a table file is named by a hex digest"),
            std::process::id(),
            WRITES.fetch_add(1, Ordering::Relaxed)
        ));
        let written = (|| {
            let mut file = fs::File::create(&staged)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            fs::rename(&staged, path)
        })();
        if written.is_err() {
            let _ = fs::remove_file(&staged);
        }
        written
    }

    fn read(path: &Path) -> io::Result<Option<Vec<u8>>> {
        match fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl ContentStore for LocalStore {
    fn find_missing(&self, digests: &[Digest]) -> io::Result<Vec<Digest>> {
        let mut missing = Vec::new();
        for digest in digests {
            let path = self.path("cas", digest)?;
            let held = match fs::metadata(&path) {
                Ok(meta) => i64::try_from(meta.len()).ok() == Some(digest.size_bytes),
                Err(e) if e.kind() == io::ErrorKind::NotFound => false,
                Err(e) => return Err(e),
            };
            if !held {
                missing.push(digest.clone());
            }
        }
        Ok(missing)
    }

    fn read_blob(&self, digest: &Digest) -> io::Result<Option<Vec<u8>>> {
        let path = self.path("cas", digest)?;
        let Some(bytes) = Self::read(&path)? else {
            return Ok(None);
        };
        // A blob is only ever what its name says. A file that is not (a
        // write cut short by a crash of the machine, a disk error) is
        // removed and reported as absent, so the caller produces it again
        // instead of building from wrong bytes.
        if Digest::of(&bytes) != *digest {
            fs::remove_file(&path)?;
            return Ok(None);
        }
        Ok(Some(bytes))
    }

    fn write_blob(&self, bytes: &[u8]) -> io::Result<Digest> {
        let digest = Digest::of(bytes);
        let path = self.path("cas", &digest)?;
        if !path.exists() {
            Self::write_atomically(&path, bytes)?;
        }
        Ok(digest)
    }

    fn action_result(&self, action: &Digest) -> io::Result<Option<ActionResult>> {
        let path = self.path("ac", action)?;
        let Some(bytes) = Self::read(&path)? else {
            return Ok(None);
        };
        match ActionResult::decode(bytes.as_slice()) {
            Ok(result) => Ok(Some(result)),
            Err(_) => {
                fs::remove_file(&path)?;
                Ok(None)
            }
        }
    }

    fn record_action_result(&self, action: &Digest, result: &ActionResult) -> io::Result<()> {
        let named: Vec<Digest> = result
            .output_files
            .iter()
            .filter_map(|f| f.digest.clone())
            .chain(result.stdout_digest.clone())
            .chain(result.stderr_digest.clone())
            .collect();
        let missing = self.find_missing(&named)?;
        if !missing.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "the result of action {action} names {} blob(s) the store does not hold, \
                     the first {}",
                    missing.len(),
                    missing[0]
                ),
            ));
        }
        Self::write_atomically(&self.path("ac", action)?, &result.encode_to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::super::reapi::OutputFile;
    use super::*;

    #[test]
    fn a_blob_written_is_read_back_by_its_digest() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalStore::at(dir.path());
        let digest = store.write_blob(b"generated").unwrap();
        assert_eq!(digest, Digest::of(b"generated"));
        assert_eq!(
            store.read_blob(&digest).unwrap().as_deref(),
            Some(&b"generated"[..])
        );
        assert!(store.find_missing(&[digest]).unwrap().is_empty());
    }

    /// A blob whose bytes are not what its name says is never handed out.
    #[test]
    fn a_blob_that_no_longer_matches_its_digest_is_absent() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalStore::at(dir.path());
        let digest = store.write_blob(b"generated").unwrap();
        let path = store.path("cas", &digest).unwrap();
        fs::write(&path, b"generatee").unwrap();
        assert_eq!(store.read_blob(&digest).unwrap(), None);
        assert!(!path.exists(), "the wrong bytes were left in the store");
    }

    /// The order the trait requires: outputs first, then the entry. An entry
    /// recorded over missing blobs would be a hit no reader can use.
    #[test]
    fn a_result_naming_a_missing_blob_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalStore::at(dir.path());
        let action = Digest::of(b"action");
        let result = ActionResult {
            output_files: vec![OutputFile {
                path: "out/a.rs".into(),
                digest: Some(Digest::of(b"never stored")),
                is_executable: false,
            }],
            ..ActionResult::default()
        };
        assert!(store.record_action_result(&action, &result).is_err());
        assert_eq!(store.action_result(&action).unwrap(), None);

        store.write_blob(b"never stored").unwrap();
        store.record_action_result(&action, &result).unwrap();
        assert_eq!(store.action_result(&action).unwrap(), Some(result));
    }

    /// A digest is a file name here, so a malformed one could name a path
    /// outside the store.
    #[test]
    fn a_malformed_digest_names_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = LocalStore::at(dir.path());
        let bad = Digest {
            hash: "../../etc/passwd".into(),
            size_bytes: 0,
        };
        assert!(store.read_blob(&bad).is_err());
        assert!(store.action_result(&bad).is_err());
    }
}
