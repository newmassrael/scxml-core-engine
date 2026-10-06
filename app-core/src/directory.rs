// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Running for the connection a request was made for.
//!
//! A request is pinned to a connection by its id and the revision it had (`requests::Pin`), and a
//! runner finds the generator that writes for it here. What is done, in this order, and why:
//!
//! 1. **The adapter.** This build can run Claude Code; it has no adapter for the other kinds yet,
//!    and says so, so that a request for one waits with a reason and is not taken by a client
//!    that is not the one the person chose.
//! 2. **The settings, at the revision the request pinned.** A connection changes after a request
//!    is made, and the request is about what it was when it was made. Settings the request was
//!    made with and this computer does not have (a works folder that came from another computer)
//!    are said to be missing, not guessed at.
//! 3. **Who is signed in**, asked the way a generation is run, so that what the screen shows and
//!    what a generation uses are the same login (`claude_code::observe_auth`).
//! 4. **Whether that way of signing in is one the build uses** (`auth_policy`), and why not when
//!    it is not, in words a person acts on.
//! 5. **The generator**, with the model and the limits the request pinned.
//!
//! What the person is told when a request cannot be run is the point of the sentences here: a
//! request that waits for an AI that will not come, with nothing said, is the worst of the ways
//! this can fail.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::auth_policy::{Decision, Observed, Policy, Reason, Route};
use crate::claude_code::{observe_auth, AuthorServer, ClaudeCode, ClaudeCodeConfig};
use crate::connection::AdapterKind;
use crate::error::StoreError;
use crate::requests::Pin;
use crate::runner::{Directory, Generator, Unrunnable};
use crate::store::ConnectionStore;

/// How long what the client said of who is signed in is kept before it is asked again. A runner
/// looks at its requests every few seconds; a client asked every time would be a process spawned
/// for each look, for as long as a person has not signed in.
const OBSERVED_FOR: Duration = Duration::from_secs(5);

/// The Claude Code this computer has, and how it reaches the authoring server.
#[derive(Debug, Clone)]
pub struct ClaudeLaunch {
    pub binary: PathBuf,
    pub author: AuthorServer,
}

/// Where a runner finds the generator for the connection of a request.
pub struct Connections {
    settings: ConnectionStore,
    policy: Policy,
    claude: Option<ClaudeLaunch>,
    observed: Mutex<Option<(Instant, Observation)>>,
}

/// What the client said of who is signed in: the kind of credential, nobody, or why it could not
/// be asked.
type Observation = Result<Option<Observed>, String>;

impl Connections {
    pub fn new(settings: ConnectionStore, policy: Policy, claude: Option<ClaudeLaunch>) -> Self {
        Connections {
            settings,
            policy,
            claude,
            observed: Mutex::new(None),
        }
    }

    /// The Claude Code that runs for `pin`, or why it cannot.
    pub fn claude_for(&self, pin: &Pin) -> Result<ClaudeCode, Unrunnable> {
        if pin.adapter != AdapterKind::ClaudeCode {
            return Err(no_adapter(pin.adapter));
        }
        let stored = match self.settings.read(&pin.connection, Some(&pin.revision)) {
            Ok(Some(stored)) => stored,
            Ok(None) | Err(StoreError::NotFound { .. }) => {
                return Err(Unrunnable::new(format!(
                    "the settings of `{}` that this request was made with are not on this \
                     computer: a request made on another computer cannot be run here, so ask \
                     again here",
                    pin.connection
                )))
            }
            Err(other) => {
                return Err(Unrunnable::new(format!(
                    "the settings of `{}` could not be read: {other}",
                    pin.connection
                )))
            }
        };
        if stored.connection.adapter != pin.adapter {
            return Err(Unrunnable::new(format!(
                "the settings of `{}` at the revision the request names are for another kind of \
                 connection than the request says",
                pin.connection
            )));
        }
        let Some(launch) = &self.claude else {
            return Err(Unrunnable::new(
                "Claude Code was not found on this computer: install it, or set SCE_CLAUDE to its \
                 path",
            ));
        };
        let observed = self.observe(launch).map_err(|e| {
            Unrunnable::new(format!("Claude Code could not say who is signed in: {e}"))
        })?;
        let Some(route) = Route::of(stored.connection.adapter, stored.connection.auth, observed)
        else {
            return Err(Unrunnable::new(
                "nobody is signed in to Claude Code: run `claude auth login` (a subscription) or \
                 `claude auth login --console` (billed by use) in a terminal, then ask again",
            ));
        };
        if let Decision::Refuse { reason, .. } = self.policy.decide(route) {
            return Err(Unrunnable::new(refusal_words(route, reason)));
        }
        let defaults = ClaudeCodeConfig::default();
        let config = ClaudeCodeConfig {
            model: pin.model.clone(),
            max_turns: pin.limits.turns.unwrap_or(defaults.max_turns),
            max_budget_usd: None,
            timeout: pin
                .limits
                .seconds
                .map_or(defaults.timeout, |s| Duration::from_secs(u64::from(s))),
        };
        Ok(ClaudeCode::new(
            launch.binary.clone(),
            launch.author.clone(),
            config,
        ))
    }

    /// Who is signed in, as the client said it a moment ago or says it now.
    fn observe(&self, launch: &ClaudeLaunch) -> Result<Option<Observed>, String> {
        let mut kept = self.observed.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((at, said)) = kept.as_ref() {
            if at.elapsed() < OBSERVED_FOR {
                return said.clone();
            }
        }
        let said = observe_auth(&launch.binary);
        *kept = Some((Instant::now(), said.clone()));
        said
    }
}

impl Directory for Connections {
    fn generator_for(&self, pin: &Pin) -> Result<Arc<dyn Generator>, Unrunnable> {
        match pin.adapter {
            AdapterKind::ClaudeCode => self
                .claude_for(pin)
                .map(|client| Arc::new(client) as Arc<dyn Generator>),
            other => Err(no_adapter(other)),
        }
    }
}

fn no_adapter(adapter: AdapterKind) -> Unrunnable {
    Unrunnable::new(format!(
        "this build has no adapter for `{}` connections yet, so a request for one waits: choose \
         another connection, or ask from an AI client of your own",
        adapter.word()
    ))
}

/// Why a way of signing in is not used, in the words a person reads.
fn refusal_words(route: Route, reason: Reason) -> String {
    let word = serde_json::to_value(route)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default();
    match reason {
        Reason::Forbidden => format!("signing in this way ({word}) is not allowed"),
        Reason::Unconfirmed => format!(
            "signing in this way ({word}) is not one this build uses: its terms have not been \
             checked. Sign in another way, with `claude auth login` or `claude auth login \
             --console`"
        ),
        Reason::SwitchedOff => {
            format!("signing in this way ({word}) is switched off in this build")
        }
    }
}
