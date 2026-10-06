// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a client's program may be on this computer, whichever client it is.
//!
//! A window started from a menu does not have what a shell's startup files add to the search path
//! (an installer's folder is one), so a person who has a client installed would be told there is
//! none. The places to look are the search path, then the folders the official installers put a
//! program in. What a program is called and how it says what it is belong to each client; the
//! walk over the places, what is passed over, and what is offered once do not, so they are here
//! once.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long a program that may be a client is given to say what it is.
pub(crate) const SAY_WHAT_IT_IS: Duration = Duration::from_secs(15);

/// Where to look for a client: the search path first, then the places the official installers
/// put it.
#[derive(Debug, Clone)]
pub struct Search {
    /// The folders of the search path, in its order.
    pub path: Vec<PathBuf>,
    /// The folders it is installed in, which the search path may not name.
    pub known: Vec<PathBuf>,
    /// How long each program is given to say what it is.
    pub timeout: Duration,
}

impl Search {
    /// Where this process can look: its search path, and the installers' folders under the
    /// person's home.
    pub fn from_environment() -> Self {
        let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .filter(|home| !home.is_empty())
            .map(PathBuf::from);
        Search {
            path: std::env::var_os("PATH")
                .map(|path| std::env::split_paths(&path).collect())
                .unwrap_or_default(),
            known: known_directories(home.as_deref()),
            timeout: SAY_WHAT_IT_IS,
        }
    }
}

/// The folders a client is installed in apart from the search path: under the person's home where
/// the official installers put it, and the system's own places. Without a home folder there are
/// only the ones that need none: a home is not guessed at.
pub fn known_directories(home: Option<&Path>) -> Vec<PathBuf> {
    let mut known = Vec::new();
    if let Some(home) = home {
        known.push(home.join(".local").join("bin"));
        known.push(home.join(".claude").join("local"));
    }
    if cfg!(not(windows)) {
        known.push(PathBuf::from("/usr/local/bin"));
        known.push(PathBuf::from("/opt/homebrew/bin"));
    }
    known
}

/// Where a candidate was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Found {
    SearchPath,
    KnownLocation,
}

/// A program the application found that says it is a client: what it may be told to run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Candidate {
    pub path: PathBuf,
    pub version: String,
    pub found: Found,
}

/// Whether `path` is a file that can be run.
fn is_runnable(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.is_file() && meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        meta.is_file()
    }
}

/// The programs in `search` called `file_name` that say what they are (`says` gives the version
/// of one that says it is the client, and nothing of one that says anything else), in the order
/// they were found: the search path's, then the known locations'. One program reached by two names
/// is offered once, by the first; a folder that is not there, a file that cannot be run, and a
/// program that does not answer in time are passed over and do not stop the search.
pub(crate) fn find_programs(
    search: &Search,
    file_name: &str,
    says: impl Fn(&Path, Duration) -> Option<String>,
) -> Vec<Candidate> {
    let places = search
        .path
        .iter()
        .map(|dir| (dir, Found::SearchPath))
        .chain(search.known.iter().map(|dir| (dir, Found::KnownLocation)));
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut offered = Vec::new();
    for (dir, found) in places {
        let path = dir.join(file_name);
        if !is_runnable(&path) {
            continue;
        }
        // Asked once whatever it is called: a program that is not the client is not asked twice.
        let Ok(real) = fs::canonicalize(&path) else {
            continue;
        };
        if seen.contains(&real) {
            continue;
        }
        seen.push(real);
        if let Some(version) = says(&path, search.timeout) {
            offered.push(Candidate {
                path,
                version,
                found,
            });
        }
    }
    offered
}
