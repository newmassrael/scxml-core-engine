// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a screen says of Codex: whether it is installed, whether this build verified that version,
//! and who is signed in by each of the three ways a connection can take its credential.
//!
//! Four things are kept apart because a screen acts on each differently. The program being there
//! says nothing of an account. This build having verified the version is not somebody being
//! signed in (until a version is verified a generation never runs, whoever is). Somebody being
//! signed in is not a way the build uses (the table in `auth_policy` decides that, and a way it
//! does not use is shown as the login it is, not hidden). And a client that could not be asked is
//! `unknown` and is said so, and never read as nobody: a screen that offered a sign-in for a
//! question that failed would send a person to log in again for nothing.
//!
//! A connection takes its credential from one of three sources, and each is asked the way a
//! generation runs it (`codex::login_of`), so that the login shown is the login a generation
//! uses: the official client's own login, the application's stored one (kept in a folder of the
//! application's, apart from the person's), and a key in the environment. A key is named by its
//! variable and never read into the answer, and nothing else of the environment is.

use std::path::Path;

use serde::Serialize;

use crate::auth_policy::{Decision, Observed, Policy, Route};
use crate::claude_status::{AccountState, Billing, Client};
use crate::client_find::Search;
use crate::codex::{locate, login_of, support_verdict, version_of, NotRunnable};
use crate::codex_environment::API_KEY_VARIABLE;
use crate::codex_support::Support;
use crate::connection::{AdapterKind, AuthSource};
use crate::host::CODEX_ENV;

/// The sources a connection to Codex can take its credential from, in the order a screen lists
/// them.
const SOURCES: [AuthSource; 3] = [
    AuthSource::OfficialLogin,
    AuthSource::AppStore,
    AuthSource::EnvApiKey,
];

/// Whether this build may run the Codex that is there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum SupportState {
    /// A person verified this version against the instructions this build gives it.
    Verified,
    /// It was asked and the answer is no; says why.
    Unverified { reason: String },
    /// It could not be asked (no program, or the program would not list its features).
    Unknown { reason: String },
}

/// Who is signed in by one source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceAccount {
    pub source: AuthSource,
    #[serde(flatten)]
    pub account: AccountState,
}

/// A command a person runs in a terminal, with the billing it signs in for and the folder to run
/// it for, when it is not the client's own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Guidance {
    pub source: AuthSource,
    pub billing: Billing,
    pub command: &'static str,
    /// The folder to start the client with as `CODEX_HOME`, so that the login it makes is the one
    /// a generation uses. None for the official client's own login, which is where the client
    /// keeps it. Said as a folder and not as a shell's words: which shell it is is not known.
    pub home: Option<String>,
}

/// What a screen says of Codex.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CodexStatus {
    pub client: Client,
    pub support: SupportState,
    pub accounts: Vec<SourceAccount>,
    pub sign_in: Vec<Guidance>,
    /// The variable a connection that takes its key from the environment reads it from.
    pub key_variable: &'static str,
}

/// Ask: where Codex is (`named` is where a connection or the environment said it is, when one
/// did; else the first program `search` finds), whether this build verified that version
/// (`support`), and who is signed in by each source (`app_home` is the application's own home for
/// the client, and `environment` is the process's own). Starts the client, so only an entrance
/// that may start a program asks.
pub fn read(
    named: Option<&Path>,
    policy: &Policy,
    support: &Support,
    app_home: &Path,
    environment: &[(String, String)],
    search: &Search,
) -> CodexStatus {
    let binary = locate(named, search).filter(|binary| binary.is_file());
    let (client, version) = match binary.as_deref().map(|binary| (binary, version_of(binary))) {
        Some((binary, Some(version))) => (
            Client::Installed {
                version: version.clone(),
                path: binary.display().to_string(),
            },
            Some(version),
        ),
        Some((_, None)) => (Client::Unverified, None),
        None => (Client::Missing, None),
    };
    let support_state = match (binary.as_deref(), version.as_deref()) {
        (Some(binary), Some(version)) => match support_verdict(binary, version, support) {
            Ok(()) => SupportState::Verified,
            Err(NotRunnable::Unverified(reason)) => SupportState::Unverified { reason },
            Err(NotRunnable::Unasked(reason)) => SupportState::Unknown { reason },
        },
        _ => SupportState::Unknown {
            reason: why_not_asked(&client),
        },
    };
    let accounts = SOURCES
        .iter()
        .map(|&source| SourceAccount {
            source,
            account: account_of(
                source,
                binary.as_deref().filter(|_| version.is_some()),
                &client,
                policy,
                app_home,
                environment,
            ),
        })
        .collect();
    CodexStatus {
        client,
        support: support_state,
        accounts,
        sign_in: sign_in(app_home),
        key_variable: API_KEY_VARIABLE,
    }
}

/// Why nothing was asked of a program that is not an installed Codex.
fn why_not_asked(client: &Client) -> String {
    match client {
        Client::Missing => {
            format!("Codex was not found: install it, or set {CODEX_ENV} to its path")
        }
        _ => "the program did not say it is Codex, so it is not asked anything".to_string(),
    }
}

/// Who is signed in by `source`. `binary` is the Codex to ask, when there is one that says it is.
fn account_of(
    source: AuthSource,
    binary: Option<&Path>,
    client: &Client,
    policy: &Policy,
    app_home: &Path,
    environment: &[(String, String)],
) -> AccountState {
    if source == AuthSource::EnvApiKey {
        // A key in the environment is the credential whatever a stored login says, and it needs
        // no client to be named: its variable is, and its value is not read.
        let set = environment
            .iter()
            .any(|(name, value)| name == API_KEY_VARIABLE && !value.is_empty());
        return if set {
            signed_in(source, None, Some(API_KEY_VARIABLE.to_string()), policy)
        } else {
            AccountState::SignedOut
        };
    }
    let Some(binary) = binary else {
        return AccountState::Unknown {
            reason: why_not_asked(client),
        };
    };
    match login_of(binary, source, app_home, environment) {
        Err(reason) => AccountState::Unknown { reason },
        // Nobody signed in is a state, and not a route to judge.
        Ok(None) => AccountState::SignedOut,
        Ok(Some(observed)) => signed_in(source, Some(observed), None, policy),
    }
}

/// Somebody is signed in by `source`: by what route, how that is billed, and whether the build
/// uses it.
fn signed_in(
    source: AuthSource,
    observed: Option<Observed>,
    environment: Option<String>,
    policy: &Policy,
) -> AccountState {
    let route = Route::of(AdapterKind::Codex, source, observed).unwrap_or(Route::Unlisted);
    let decision = policy.decide(route);
    AccountState::SignedIn {
        route,
        billing: Billing::of(route),
        environment,
        usable: matches!(decision, Decision::Use { .. }),
        decision,
    }
}

/// The commands that sign in to Codex. Fixed words: a plain `codex login` signs in with a ChatGPT
/// plan, so the key one is named. The application's stored login is made in the application's own
/// folder, which is named and not put into a command, because the words that set a variable
/// differ by shell.
pub fn sign_in(app_home: &Path) -> Vec<Guidance> {
    let home = |path: &Path| Some(path.display().to_string());
    let mut guidance = Vec::new();
    for (source, home) in [
        (AuthSource::OfficialLogin, None),
        (AuthSource::AppStore, home(app_home)),
    ] {
        guidance.push(Guidance {
            source,
            billing: Billing::Subscription,
            command: "codex login",
            home: home.clone(),
        });
        guidance.push(Guidance {
            source,
            billing: Billing::Usage,
            command: "codex login --with-api-key",
            home,
        });
    }
    guidance
}
