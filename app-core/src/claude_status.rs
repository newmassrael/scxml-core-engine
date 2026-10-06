// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a screen says of Claude Code: whether it is installed, who is signed in and how that is
//! billed, and whether the build uses that way of signing in.
//!
//! Three things are kept apart because a screen acts on each differently. The client being there
//! is not somebody being signed in (a found program says nothing of an account), and somebody
//! being signed in is not a way the build uses (the table in `auth_policy` decides that, and a
//! way it does not use is shown as the login it is, not hidden). A client that could not be asked
//! is `unknown` and is said so, and never read as "nobody": a screen that offered a sign-in for a
//! question that failed would send a person to log in again for nothing.
//!
//! The client is asked the way a generation runs it (`claude_code::observe_account`), so that the
//! login shown is the login a generation uses, and only what a screen may show is read out of its
//! answer.

use std::path::Path;

use serde::Serialize;

use crate::auth_policy::{Decision, Policy, Route};
use crate::claude_code::{locate, observe_account, version_of, Search};
use crate::host::CLAUDE_ENV;

/// How a way of signing in is billed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Billing {
    /// Against the limits of a plan the person pays for monthly.
    Subscription,
    /// By use, against an API key.
    Usage,
    /// By a cloud provider, on its own account.
    Provider,
}

impl Billing {
    /// How a route is billed; none for a way that is not one of the three.
    pub fn of(route: Route) -> Option<Billing> {
        match route {
            Route::ClaudeOfficialLogin => Some(Billing::Subscription),
            Route::ClaudeApiKey => Some(Billing::Usage),
            Route::ClaudeCloudProvider => Some(Billing::Provider),
            _ => None,
        }
    }
}

/// Whether the program is there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum Client {
    /// It answered `--version`. `path` is the program that did: the screen of this computer says
    /// which one it asked, and it is not kept anywhere a work is.
    Installed { version: String, path: String },
    /// A file is there and did not answer `--version`: not something to run until it does.
    Unverified,
    /// There is no such program.
    Missing,
}

/// Who is signed in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum AccountState {
    SignedIn {
        route: Route,
        billing: Option<Billing>,
        /// The variable that decided it, when the client said one did. A name and not a value.
        environment: Option<String>,
        /// Whether a generation would use it: the table's answer, as a flag a screen can act on.
        usable: bool,
        decision: Decision,
    },
    SignedOut,
    /// It could not be asked, or what it said was not an answer; says why.
    Unknown {
        reason: String,
    },
}

/// A command a person runs in a terminal, with the billing it signs in for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Guidance {
    pub billing: Billing,
    pub command: &'static str,
}

/// The commands that sign in to Claude Code. Fixed words: the client chooses where it keeps a
/// login, so none carries a path or a variable (a login made somewhere else would not be the one
/// a generation uses), and a plain `claude auth login` signs in with a subscription, so the usage
/// one is named.
pub const SIGN_IN: &[Guidance] = &[
    Guidance {
        billing: Billing::Subscription,
        command: "claude auth login",
    },
    Guidance {
        billing: Billing::Usage,
        command: "claude auth login --console",
    },
];

/// What a screen says of Claude Code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClaudeStatus {
    pub client: Client,
    pub account: AccountState,
    pub sign_in: Vec<Guidance>,
}

/// Ask: where Claude Code is (`named` is where a connection or the environment said it is, when
/// one did; else the first program `search` finds), and who is signed in to it. Starts the client,
/// so only an entrance that may start a program asks.
pub fn read(named: Option<&Path>, policy: &Policy, search: &Search) -> ClaudeStatus {
    let (client, account) = match locate(named, search) {
        Some(binary) if binary.is_file() => match version_of(&binary) {
            Some(version) => (
                Client::Installed {
                    version,
                    path: binary.display().to_string(),
                },
                account_of(&binary, policy),
            ),
            None => (
                Client::Unverified,
                AccountState::Unknown {
                    reason: "Claude Code did not answer `--version`, so it is not asked who is \
                             signed in"
                        .to_string(),
                },
            ),
        },
        _ => (
            Client::Missing,
            AccountState::Unknown {
                reason: format!(
                    "Claude Code was not found: install it, or set {CLAUDE_ENV} to its path"
                ),
            },
        ),
    };
    ClaudeStatus {
        client,
        account,
        sign_in: SIGN_IN.to_vec(),
    }
}

fn account_of(binary: &Path, policy: &Policy) -> AccountState {
    let account = match observe_account(binary) {
        Ok(account) => account,
        Err(reason) => return AccountState::Unknown { reason },
    };
    // Nobody is signed in is a state, and not a route to judge.
    let Some(observed) = account.observed else {
        return AccountState::SignedOut;
    };
    let route = Route::of(
        crate::connection::AdapterKind::ClaudeCode,
        crate::connection::AuthSource::OfficialLogin,
        Some(observed),
    )
    .unwrap_or(Route::Unlisted);
    let decision = policy.decide(route);
    AccountState::SignedIn {
        route,
        billing: Billing::of(route),
        environment: account.environment,
        usable: matches!(decision, Decision::Use { .. }),
        decision,
    }
}
