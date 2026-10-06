// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What reaches Codex of the person's environment, by the credential a connection chose.
//!
//! A run is billed to one credential, and the screen says which: a stored login (the
//! application's own, or the one the person made with the official client) or a key the person
//! put in `CODEX_API_KEY`. So a run is started with that one and without the others. A key that is
//! only in the person's shell does not take over from the login a connection chose, and a login
//! that happens to be stored does not stand in for a key a connection asked the environment for.
//! A connection that chose a key and finds none has a problem with the environment, and is told
//! so, instead of running on something else.
//!
//! Only names are handled here. A value of the person's is never read into what is returned, so
//! there is nothing of it to reach a log; the one thing set is a folder of the application's.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::connection::AuthSource;

/// The variable a connection that chose a key from the environment reads it from.
pub const API_KEY_VARIABLE: &str = "CODEX_API_KEY";

/// The names that carry a credential, or select the way of signing in. The client's 0.159.0
/// install holds more such strings than its documents explain (connector, cloud provider and
/// workload credentials among them), and what each does has not been established: until it is,
/// none of them reaches a run, because the only credentials a connection may use are the ones
/// above.
const CREDENTIALS: &[&str] = &[
    "CODEX_API_KEY",
    "OPENAI_API_KEY",
    "CODEX_ACCESS_TOKEN",
    "CODEX_CONNECTORS_TOKEN",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "AZURE_CLIENT_SECRET",
    "GOOGLE_APPLICATION_CREDENTIALS",
];

/// The families of names that carry a credential: a name a later version adds to one is in it.
const CREDENTIAL_PREFIXES: &[&str] = &["OPENAI_IDENTITY_"];

/// What to do to a run's environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    /// The folder to give the client as its home (`CODEX_HOME`), when it is not the one it would
    /// use itself. Never a person's folder: the application's.
    pub home: Option<PathBuf>,
    /// The names to take away, which is every one that could pick a credential other than the
    /// one the connection chose.
    pub remove: Vec<String>,
}

impl Environment {
    /// Start `command` with this environment: the others' credentials gone, the home set.
    pub fn apply(&self, command: &mut Command) {
        for name in &self.remove {
            command.env_remove(name);
        }
        if let Some(home) = &self.home {
            command.env("CODEX_HOME", home);
        }
    }
}

/// Why a run cannot be given an environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The connection chose a key from the environment, and there is none in it.
    VariableMissing(&'static str),
    /// The connection's source of credentials is not one Codex has.
    NotACodexSource(AuthSource),
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refused::VariableMissing(name) => write!(
                f,
                "this connection takes its key from `{name}` in the environment, and the \
                 application sees none there: a window started from a menu does not have what a \
                 shell's startup files set, so set it where the application is started, or sign \
                 in with the official client and choose that instead"
            ),
            Refused::NotACodexSource(source) => write!(
                f,
                "{source:?} is not a source of credentials for Codex: it takes a stored login of \
                 the application's or of the official client, or a key from the environment"
            ),
        }
    }
}

impl std::error::Error for Refused {}

/// The environment to run Codex in for a connection that chose `auth`, given the names and values
/// the process has (`parent`). `app_home` is the application's own home for the client.
pub fn environment_for(
    auth: AuthSource,
    app_home: &Path,
    parent: &[(String, String)],
) -> Result<Environment, Refused> {
    let (keeps, home) = match auth {
        AuthSource::AppStore => (None, Some(app_home.to_path_buf())),
        // Its own home is the client's: where that is, the person's environment says.
        AuthSource::OfficialLogin => (None, None),
        AuthSource::EnvApiKey => {
            let set = parent
                .iter()
                .any(|(name, value)| name == API_KEY_VARIABLE && !value.is_empty());
            if !set {
                return Err(Refused::VariableMissing(API_KEY_VARIABLE));
            }
            // A stored login is not what this connection chose; the application's folder holds
            // none, so the key is what there is.
            (Some(API_KEY_VARIABLE), Some(app_home.to_path_buf()))
        }
        other => return Err(Refused::NotACodexSource(other)),
    };
    let mut remove: Vec<String> = CREDENTIALS
        .iter()
        .filter(|name| Some(**name) != keeps)
        .map(|name| name.to_string())
        .collect();
    for (name, _) in parent {
        let in_a_family = CREDENTIAL_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix));
        if in_a_family && !remove.contains(name) {
            remove.push(name.clone());
        }
    }
    remove.sort();
    Ok(Environment { home, remove })
}
