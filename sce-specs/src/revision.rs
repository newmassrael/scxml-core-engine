// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a revision is: the SHA-256 of the exact bytes that were saved.
//!
//! The acceptance record the product already writes pins a specification by the
//! digest of its bytes, and decides what has gone stale by comparing digests. A
//! revision number kept beside that would be a second answer to "which version
//! is this", and the two could disagree. So the revision IS the digest, and
//! whether a result is about the text on screen is a comparison of two of them.

use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Lowercase hexadecimal length of a SHA-256 digest.
const HEX_LENGTH: usize = 64;

/// The digest of one saved text, as 64 lowercase hexadecimal characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Revision(String);

/// Text that is not a revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotARevision(pub String);

impl fmt::Display for NotARevision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "`{}` is not a revision: expected {HEX_LENGTH} lowercase hexadecimal characters",
            self.0
        )
    }
}

impl std::error::Error for NotARevision {}

impl Revision {
    /// The revision of `bytes`.
    pub fn of(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        let mut hex = String::with_capacity(HEX_LENGTH);
        for byte in digest {
            hex.push_str(&format!("{byte:02x}"));
        }
        Revision(hex)
    }

    /// Read a revision someone wrote down.
    pub fn parse(text: &str) -> Result<Self, NotARevision> {
        let well_formed = text.len() == HEX_LENGTH
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if well_formed {
            Ok(Revision(text.to_string()))
        } else {
            Err(NotARevision(text.to_string()))
        }
    }

    /// The full digest.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The first twelve characters, for a person to read. Never a key.
    pub fn short(&self) -> &str {
        &self.0[..12]
    }
}

impl fmt::Display for Revision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Revision {
    type Error = NotARevision;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Revision::parse(&text)
    }
}

impl From<Revision> for String {
    fn from(revision: Revision) -> String {
        revision.0
    }
}
