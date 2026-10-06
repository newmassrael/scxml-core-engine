// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which ways of signing in the workbench uses, and which it does not.
//!
//! What a provider's terms allow is a fact about a way of reaching its model, not about a
//! connection: the same Claude Code is reached by a subscription, by an API key and by a cloud
//! provider's credential, and the terms say something different of each. So the unit here is a
//! [`Route`], with a [`Status`] each, and the table is a constant of the build:
//!
//! - **allowed**: the provider's documents describe it as a way to use their client;
//! - **conditional**: the documents do not forbid it and do not describe it either, and the
//!   product's owner has decided to take the risk that remains. A release can switch it off;
//! - **forbidden**: the documents forbid it;
//! - **unconfirmed**: nobody has checked, or the documents do not say.
//!
//! A person cannot widen the table, a release can narrow it (a switch only ever turns a route
//! off), and a route that is not in it is `Unlisted`, which is unconfirmed. Only allowed and
//! conditional routes are used.
//!
//! The table is written a second time in `tests/auth_policy.rs`, so that a change to one cannot
//! go unnoticed in the other. What each row rests on is said at the row: the provider's terms as
//! they read, and, where they are silent, the decision the product's owner took.

use std::collections::BTreeSet;

use serde::Serialize;

use crate::connection::{AdapterKind, AuthSource};

/// A way of signing in, or of starting a sign-in, that a provider's terms speak of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Route {
    /// The person signed in to Claude Code with the client's own command (a subscription), and
    /// the workbench runs the client unmodified.
    ClaudeOfficialLogin,
    /// A key of the person's own, from the environment or the client's own login.
    ClaudeApiKey,
    /// The person configured a cloud provider's credential for the client (Amazon Bedrock,
    /// Google Cloud, Microsoft Foundry). The workbench sets none of it up.
    ClaudeCloudProvider,
    /// The workbench opens a terminal and runs the client's own sign-in command for the person.
    ClaudeLoginCommandByApp,
    /// The workbench offers a Claude.ai sign-in screen of its own, or handles the sign-in.
    ClaudeLoginScreenByApp,
    /// A sign-in through Codex's app-server.
    CodexAppServerAuth,
    /// OpenAI's product for apps that sign people in with their ChatGPT plan.
    CodexSignInWithChatGpt,
    /// A key of the person's own, through the CLI's login or `CODEX_API_KEY`.
    CodexCliApiKey,
    /// The person signed in to the Codex CLI with a ChatGPT plan, and the workbench runs
    /// `codex exec` unmodified.
    CodexCliChatGptLogin,
    /// An API key handed to app-server's login.
    CodexAppServerApiKey,
    /// An access token, or a workload identity.
    CodexAccessToken,
    /// A server the person runs.
    LocalServer,
    /// What the table does not name.
    Unlisted,
}

/// What the design says of a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Allowed,
    Conditional,
    Forbidden,
    Unconfirmed,
}

/// Why a route is not used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reason {
    Forbidden,
    Unconfirmed,
    /// A release switched it off.
    SwitchedOff,
}

/// Whether a route is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "decision", rename_all = "kebab-case")]
pub enum Decision {
    Use { status: Status },
    Refuse { status: Status, reason: Reason },
}

/// What a client says of the credential it is using: the part of `claude auth status` and
/// `codex login status` that decides a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Observed {
    /// A plan the person pays for monthly (a Claude subscription, a ChatGPT plan).
    Subscription,
    /// A key that is billed by use.
    ApiKey,
    /// A cloud provider's credential.
    CloudProvider,
    /// An access token or a workload identity.
    AccessToken,
    /// Anything the client said that is none of these.
    Other,
}

impl Route {
    /// Every route, in the order of the design's table.
    pub const ALL: &'static [Route] = &[
        Route::ClaudeOfficialLogin,
        Route::ClaudeApiKey,
        Route::ClaudeCloudProvider,
        Route::ClaudeLoginCommandByApp,
        Route::ClaudeLoginScreenByApp,
        Route::CodexAppServerAuth,
        Route::CodexSignInWithChatGpt,
        Route::CodexCliApiKey,
        Route::CodexCliChatGptLogin,
        Route::CodexAppServerApiKey,
        Route::CodexAccessToken,
        Route::LocalServer,
        Route::Unlisted,
    ];

    /// What the design says of this route.
    pub fn status(self) -> Status {
        match self {
            Route::ClaudeApiKey
            | Route::ClaudeCloudProvider
            | Route::CodexCliApiKey
            | Route::LocalServer => Status::Allowed,
            Route::ClaudeOfficialLogin | Route::CodexCliChatGptLogin => Status::Conditional,
            Route::ClaudeLoginScreenByApp | Route::CodexAppServerAuth => Status::Forbidden,
            Route::ClaudeLoginCommandByApp
            | Route::CodexSignInWithChatGpt
            | Route::CodexAppServerApiKey
            | Route::CodexAccessToken
            | Route::Unlisted => Status::Unconfirmed,
        }
    }

    /// The route a connection's generations take, from what its client reports. `None` when
    /// nothing is signed in yet, which is a state to say so in, not a route to judge.
    pub fn of(adapter: AdapterKind, auth: AuthSource, observed: Option<Observed>) -> Option<Route> {
        match adapter {
            AdapterKind::Local => Some(Route::LocalServer),
            AdapterKind::ClaudeCode => observed.map(|observed| match observed {
                Observed::Subscription => Route::ClaudeOfficialLogin,
                Observed::ApiKey => Route::ClaudeApiKey,
                Observed::CloudProvider => Route::ClaudeCloudProvider,
                Observed::AccessToken | Observed::Other => Route::Unlisted,
            }),
            // The variable is the credential, whatever a stored login says.
            AdapterKind::Codex if auth == AuthSource::EnvApiKey => Some(Route::CodexCliApiKey),
            AdapterKind::Codex => observed.map(|observed| match observed {
                Observed::Subscription => Route::CodexCliChatGptLogin,
                Observed::ApiKey => Route::CodexCliApiKey,
                Observed::AccessToken => Route::CodexAccessToken,
                Observed::CloudProvider | Observed::Other => Route::Unlisted,
            }),
        }
    }
}

/// Routes a release has switched off. Empty in this build: a route is added here by a change
/// that says, in its message, which document or which answer made it necessary.
const SWITCHED_OFF: &[Route] = &[];

/// The table of routes as a build uses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    switched_off: BTreeSet<Route>,
}

impl Policy {
    /// The policy of this build.
    pub fn shipped() -> Self {
        Policy {
            switched_off: SWITCHED_OFF.iter().copied().collect(),
        }
    }

    /// The same with `routes` switched off. A switch turns a route off and nothing more: the
    /// table is what it is, so a route that was refused is refused for the reason it always was.
    pub fn with_switched_off(mut self, routes: impl IntoIterator<Item = Route>) -> Self {
        self.switched_off.extend(routes);
        self
    }

    /// The routes that are switched off, in the table's order.
    pub fn switched_off(&self) -> Vec<Route> {
        self.switched_off.iter().copied().collect()
    }

    /// Whether `route` is used.
    pub fn decide(&self, route: Route) -> Decision {
        let status = route.status();
        match status {
            Status::Forbidden => Decision::Refuse {
                status,
                reason: Reason::Forbidden,
            },
            Status::Unconfirmed => Decision::Refuse {
                status,
                reason: Reason::Unconfirmed,
            },
            Status::Allowed | Status::Conditional if self.switched_off.contains(&route) => {
                Decision::Refuse {
                    status,
                    reason: Reason::SwitchedOff,
                }
            }
            Status::Allowed | Status::Conditional => Decision::Use { status },
        }
    }
}
