// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The files a relay keeps a secret in: its authority's key, which issues every token it takes, and
//! the private key of its certificate. Each is read only when its owner alone may read it (as
//! `ssh` reads a private key), and is made that way: a key another user of the machine could read
//! is a key that has leaked, and a relay that started anyway would hide it.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Why a secret was not read or made. The text names the file and what is wrong, never what it
/// holds.
#[derive(Debug)]
pub struct SecretError {
    pub kind: SecretErrorKind,
    pub message: String,
}

/// What kind of wrong a [`SecretError`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretErrorKind {
    /// It is not there, or cannot be read.
    Unreadable,
    /// Someone other than its owner may read or change it.
    Exposed,
    /// A file is there already where one was to be made.
    Exists,
    /// It could not be made or written.
    Unwritable,
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SecretError {}

fn error(kind: SecretErrorKind, message: String) -> SecretError {
    SecretError { kind, message }
}

/// The bytes of the secret `what` kept at `path`, when only its owner may read it.
pub fn read(path: &Path, what: &str) -> Result<Vec<u8>, SecretError> {
    let unreadable = |why: io::Error| {
        error(
            SecretErrorKind::Unreadable,
            format!("{what} at {} cannot be read: {why}", path.display()),
        )
    };
    let metadata = fs::metadata(path).map_err(unreadable)?;
    if !metadata.is_file() {
        return Err(error(
            SecretErrorKind::Unreadable,
            format!("{what} at {} is not a file", path.display()),
        ));
    }
    if let Some(mode) = exposed_mode(&metadata) {
        return Err(error(
            SecretErrorKind::Exposed,
            format!(
                "{what} at {} may be read or changed by others (mode {mode:04o}): make it its \
                 owner's alone (chmod 600) and replace it if anyone else could have read it",
                path.display()
            ),
        ));
    }
    fs::read(path).map_err(unreadable)
}

/// Make the file `path` holding `contents`, readable and writable by its owner alone, when there is
/// no file there: a key is never written over. It is on the disk when this returns.
pub fn create(path: &Path, what: &str, contents: &[u8]) -> Result<(), SecretError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    owner_only(&mut options);
    let mut file = options.open(path).map_err(|why| {
        let kind = if why.kind() == io::ErrorKind::AlreadyExists {
            SecretErrorKind::Exists
        } else {
            SecretErrorKind::Unwritable
        };
        error(
            kind,
            format!("{what} cannot be made at {}: {why}", path.display()),
        )
    })?;
    let written = file.write_all(contents).and_then(|()| file.sync_all());
    written.map_err(|why| {
        // What was written is a key nobody may use: it is taken away, and made again.
        let _ = fs::remove_file(path);
        error(
            SecretErrorKind::Unwritable,
            format!("{what} cannot be written at {}: {why}", path.display()),
        )
    })?;
    sync_parent(path);
    Ok(())
}

/// The mode of a file someone other than its owner may read or change, or `None` when only its
/// owner may.
#[cfg(unix)]
fn exposed_mode(metadata: &fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    let mode = metadata.permissions().mode() & 0o7777;
    ((mode & 0o077) != 0).then_some(mode)
}

/// A system with no Unix modes says nothing of who may read a file.
#[cfg(not(unix))]
fn exposed_mode(_metadata: &fs::Metadata) -> Option<u32> {
    None
}

#[cfg(unix)]
fn owner_only(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
}

#[cfg(not(unix))]
fn owner_only(_options: &mut OpenOptions) {}

/// Write the new file's name into its directory durably, when the directory can be opened as a
/// file (on Unix); a key whose name a crash lost is made again.
fn sync_parent(path: &Path) {
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "sce-relay-secret-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_secret_is_made_for_its_owner_alone_and_never_over_another() {
        let dir = scratch("made");
        let path = dir.join("key");
        create(&path, "the key", b"one").unwrap();
        assert_eq!(read(&path, "the key").unwrap(), b"one");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        let again = create(&path, "the key", b"two").unwrap_err();
        assert_eq!(again.kind, SecretErrorKind::Exists);
        assert_eq!(fs::read(&path).unwrap(), b"one", "not written over");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_secret_that_is_not_there_says_which_file() {
        let dir = scratch("absent");
        let path = dir.join("nothing");
        let refused = read(&path, "the authority's key").unwrap_err();
        assert_eq!(refused.kind, SecretErrorKind::Unreadable);
        assert!(
            refused.message.contains("the authority's key")
                && refused.message.contains(&path.display().to_string()),
            "{refused}"
        );
        let refused = read(&dir, "the authority's key").unwrap_err();
        assert!(refused.message.contains("is not a file"), "{refused}");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_secret_others_may_read_or_change_is_refused_and_not_said() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("exposed");
        let path = dir.join("key");
        create(&path, "the key", b"do-not-print-this").unwrap();
        for mode in [0o640, 0o604, 0o620, 0o644] {
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            let refused = read(&path, "the key").unwrap_err();
            assert_eq!(refused.kind, SecretErrorKind::Exposed, "{mode:o}");
            assert!(
                refused.message.contains(&format!("{mode:04o}")),
                "{refused}"
            );
            assert!(!refused.message.contains("do-not-print-this"));
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
        assert_eq!(read(&path, "the key").unwrap(), b"do-not-print-this");
        fs::remove_dir_all(&dir).unwrap();
    }
}
