// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What can go wrong with a works folder, said so a caller can branch on it.
//!
//! Every variant has a stable `kind` word. A shell that is not Rust (the browser
//! UI, the MCP) reads that word and not the sentence, and the sentence is for a
//! person. The two never share a field.

use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::revision::Revision;

/// A failed operation on a works folder.
#[derive(Debug)]
pub enum StoreError {
    /// A work or revision that is not there.
    NotFound { what: String },
    /// An id that is not a valid work id, so it names no folder.
    InvalidId { id: String },
    /// A title that cannot name a work.
    InvalidTitle { reason: String },
    /// The text was saved from a revision that is no longer the current one.
    ///
    /// Not a failure to retry with the same bytes: somebody else's save, or a
    /// save from another window, came first, and writing over it would lose it.
    /// The caller reads `current`, decides what the two texts become, and asks
    /// again with `current` as the base.
    Conflict {
        base: Option<Revision>,
        current: Option<Revision>,
    },
    /// A text larger than the store accepts as one source, or one model
    /// (`what` says which).
    TooLarge {
        what: &'static str,
        bytes: usize,
        limit: usize,
    },
    /// A file the store wrote is not what it says it is.
    Corrupt { path: PathBuf, reason: String },
    /// Another save held the work's lock for the whole wait.
    Busy { path: PathBuf, waited_ms: u64 },
    /// The operating system refused.
    Io { path: PathBuf, source: io::Error },
}

impl StoreError {
    /// The word a program branches on.
    pub fn kind(&self) -> &'static str {
        match self {
            StoreError::NotFound { .. } => "not-found",
            StoreError::InvalidId { .. } => "invalid-id",
            StoreError::InvalidTitle { .. } => "invalid-title",
            StoreError::Conflict { .. } => "conflict",
            StoreError::TooLarge { .. } => "too-large",
            StoreError::Corrupt { .. } => "corrupt",
            StoreError::Busy { .. } => "busy",
            StoreError::Io { .. } => "io",
        }
    }

    pub(crate) fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        StoreError::Io {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn corrupt(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        StoreError::Corrupt {
            path: path.into(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::NotFound { what } => write!(f, "{what} does not exist"),
            StoreError::InvalidId { id } => {
                write!(
                    f,
                    "`{id}` is not a work id (lowercase letters, digits and hyphens)"
                )
            }
            StoreError::InvalidTitle { reason } => write!(f, "the title cannot be used: {reason}"),
            StoreError::Conflict { base, current } => {
                let show = |r: &Option<Revision>| {
                    r.as_ref()
                        .map_or_else(|| "no revision".to_string(), |r| r.short().to_string())
                };
                write!(
                    f,
                    "the text was saved from {}, and the work is now at {}: nothing was written. \
                     Read the current text, decide what the two become, and save again from it",
                    show(base),
                    show(current)
                )
            }
            StoreError::TooLarge { what, bytes, limit } => {
                write!(
                    f,
                    "the text is {bytes} bytes and a {what} may hold at most {limit}"
                )
            }
            StoreError::Corrupt { path, reason } => {
                write!(
                    f,
                    "{} is not what the store wrote: {reason}",
                    path.display()
                )
            }
            StoreError::Busy { path, waited_ms } => write!(
                f,
                "another save held {} for {waited_ms} ms; nothing was written",
                path.display()
            ),
            StoreError::Io { path, source } => write!(f, "{}: {source}", path.display()),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
