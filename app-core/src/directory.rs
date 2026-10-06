// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Running for the connection a request was made for.
//!
//! A request is pinned to a connection by its id and the revision it had (`requests::Pin`), and a
//! runner finds the generator that writes for it here. What is done, in this order, and why:
//!
//! 1. **The adapter.** This build can run Claude Code and Codex; it has no adapter for the other
//!    kind yet, and says so, so that a request for it waits with a reason and is not taken by a
//!    client that is not the one the person chose.
//! 2. **The settings, at the revision the request pinned.** A connection changes after a request
//!    is made, and the request is about what it was when it was made. Settings the request was
//!    made with and this computer does not have (a works folder that came from another computer)
//!    are said to be missing, not guessed at.
//! 3. **Who is signed in**, asked the way a generation is run, so that what the screen shows and
//!    what a generation uses are the same login (`claude_code::observe_auth`,
//!    `codex::Codex::observe_login`). Codex is first asked whether it is one this build verified
//!    and has a credential to run on: a login is not worth asking about for a client that may not
//!    be run.
//! 4. **Whether that way of signing in is one the build uses** (`auth_policy`), and why not when
//!    it is not, in words a person acts on.
//! 5. **The generator**, with the model and the limits the request pinned.
//!
//! What the person is told when a request cannot be run is the point of the sentences here: a
//! request that waits for an AI that will not come, with nothing said, is the worst of the ways
//! this can fail.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::auth_policy::{Decision, Observed, Policy, Reason, Route};
use crate::claude_code::{
    claude_version_of, observe_auth, AuthorServer, ClaudeCode, ClaudeCodeConfig, SAY_WHAT_IT_IS,
};
use crate::codex::{Codex, CodexConfig, CodexLaunch};
use crate::connection::{AdapterKind, AuthSource};
use crate::error::StoreError;
use crate::requests::Pin;
use crate::runner::{Directory, Generator, Unrunnable};
use crate::store::{ConnectionStore, StoredConnection};

/// How long what the client said of who is signed in is kept before it is asked again. A runner
/// looks at its requests every few seconds; a client asked every time would be a process spawned
/// for each look, for as long as a person has not signed in.
const OBSERVED_FOR: Duration = Duration::from_secs(5);

/// The Claude Code this computer has, and how it reaches the authoring server.
#[derive(Debug, Clone)]
pub struct ClaudeLaunch {
    pub binary: PathBuf,
    pub author: AuthorServer,
    /// What the environment lets a run spend (`SCE_CLAUDE_BUDGET_USD`). A connection has no word
    /// for money, so this bounds a run made for one as it bounds any other: it is the person's
    /// limit on the application, and it changes nothing a request asked for.
    pub max_budget_usd: Option<f64>,
}

/// The programs a directory can run, asked each time one is needed. A person installs a client
/// while the window is open, and one that was there goes away; a directory that held what was found
/// when it was made would not run for the one and would start the other from a path that is gone.
pub trait Launches: Send + Sync {
    /// The Claude Code that is there now, when one is.
    fn claude(&self) -> Option<ClaudeLaunch>;
    /// The Codex that is there now, when one is.
    fn codex(&self) -> Option<CodexLaunch>;
}

/// Launches that were found once and do not change: what a command line tool or a test has.
#[derive(Debug, Clone, Default)]
pub struct FixedLaunches {
    pub claude: Option<ClaudeLaunch>,
    pub codex: Option<CodexLaunch>,
}

impl Launches for FixedLaunches {
    fn claude(&self) -> Option<ClaudeLaunch> {
        self.claude.clone()
    }

    fn codex(&self) -> Option<CodexLaunch> {
        self.codex.clone()
    }
}

/// Where a runner finds the generator for the connection of a request.
pub struct Connections {
    settings: ConnectionStore,
    policy: Policy,
    launches: Arc<dyn Launches>,
    /// What was last said, of which program, and when.
    observed: Mutex<Option<(PathBuf, Instant, Observation)>>,
    /// What was last found of a Codex and of a way of signing in to it, and when.
    codex_seen: Mutex<Option<CodexSeen>>,
}

/// What a finding about Codex is about (the program, and the credential a connection chose for
/// it), when it was made, and what it was: a Codex that may be run, or why not.
type CodexSeen = ((PathBuf, AuthSource), Instant, Result<Codex, String>);

/// What the client said of who is signed in: the kind of credential, nobody, or why it could not
/// be asked.
type Observation = Result<Option<Observed>, String>;

impl Connections {
    /// A directory that runs the Claude Code `claude` describes, which does not change.
    pub fn new(settings: ConnectionStore, policy: Policy, claude: Option<ClaudeLaunch>) -> Self {
        Self::live(
            settings,
            policy,
            Arc::new(FixedLaunches {
                claude,
                codex: None,
            }),
        )
    }

    /// A directory that asks `launches` for the programs it runs each time it needs one.
    pub fn live(settings: ConnectionStore, policy: Policy, launches: Arc<dyn Launches>) -> Self {
        Connections {
            settings,
            policy,
            launches,
            observed: Mutex::new(None),
            codex_seen: Mutex::new(None),
        }
    }

    /// The same, able to run the Codex `launch` describes. The launches become fixed: what the
    /// directory could find of Claude Code when this is called is what it keeps.
    pub fn with_codex(mut self, launch: CodexLaunch) -> Self {
        self.launches = Arc::new(FixedLaunches {
            claude: self.launches.claude(),
            codex: Some(launch),
        });
        self
    }

    /// The settings `pin` was made with, as they were then, or why they cannot be had.
    fn stored_for(&self, pin: &Pin) -> Result<StoredConnection, Unrunnable> {
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
            // What failed is for a log: a reason is said where the works folder is, which is
            // shared, and a store's error names a path of this computer.
            Err(_) => {
                return Err(Unrunnable::new(format!(
                    "the settings of `{}` could not be read, so this request cannot be run",
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
        Ok(stored)
    }

    /// The Claude Code that runs for `pin`, or why it cannot.
    pub fn claude_for(&self, pin: &Pin) -> Result<ClaudeCode, Unrunnable> {
        if pin.adapter != AdapterKind::ClaudeCode {
            return Err(no_adapter(pin.adapter));
        }
        let stored = self.stored_for(pin)?;
        let Some(launch) = self.launches.claude() else {
            return Err(claude_not_found());
        };
        // The program the connection names, when it names one, is what runs: the person chose it
        // among the ones the application found. It is asked again, because a file can be gone or
        // replaced since, and a run is not made with another program in its place.
        let binary = match stored.connection.executable.as_deref() {
            Some(chosen) => {
                let chosen = PathBuf::from(chosen);
                if claude_version_of(&chosen, SAY_WHAT_IT_IS).is_none() {
                    return Err(Unrunnable::new(format!(
                        "the Claude Code that `{}` names is gone or no longer says it is Claude \
                         Code: choose it again under AI connection",
                        pin.connection
                    )));
                }
                chosen
            }
            // The program the host found may have gone since it looked: that is said as it is, and
            // not as a client that could not be asked something.
            None if !launch.binary.is_file() => return Err(claude_not_found()),
            None => launch.binary.clone(),
        };
        // What the client printed is for the settings screen, which shows it to the person who
        // is looking: a reason is said where the works folder is, and is a sentence of ours.
        let observed = self.observe(&binary).map_err(|_| {
            Unrunnable::new(
                "Claude Code could not be asked who is signed in: check it under AI connection, \
                 then ask again",
            )
        })?;
        let Some(route) = Route::of(stored.connection.adapter, stored.connection.auth, observed)
        else {
            return Err(Unrunnable::new(
                "nobody is signed in to Claude Code: run `claude auth login` (a subscription) or \
                 `claude auth login --console` (billed by use) in a terminal, then ask again",
            ));
        };
        if let Decision::Refuse { reason, .. } = self.policy.decide(route) {
            return Err(Unrunnable::new(refusal_words(
                AdapterKind::ClaudeCode,
                route,
                reason,
            )));
        }
        let defaults = ClaudeCodeConfig::default();
        let config = ClaudeCodeConfig {
            model: pin.model.clone(),
            max_turns: pin.limits.turns.unwrap_or(defaults.max_turns),
            max_budget_usd: launch.max_budget_usd,
            timeout: pin
                .limits
                .seconds
                .map_or(defaults.timeout, |s| Duration::from_secs(u64::from(s))),
        };
        Ok(ClaudeCode::new(binary, launch.author.clone(), config))
    }

    /// The Codex that runs for `pin`, or why it cannot. Every sentence of a reason names no path and
    /// nothing Codex printed: it is kept in the works folder, which is shared.
    pub fn codex_for(&self, pin: &Pin) -> Result<Codex, Unrunnable> {
        if pin.adapter != AdapterKind::Codex {
            return Err(no_adapter(pin.adapter));
        }
        let stored = self.stored_for(pin)?;
        let Some(launch) = self.launches.codex() else {
            return Err(codex_not_found());
        };
        let binary = match stored.connection.executable.as_deref() {
            Some(chosen) => PathBuf::from(chosen),
            None if !launch.binary.is_file() => return Err(codex_not_found()),
            None => launch.binary.clone(),
        };
        let ready = self
            .codex_ready(&launch, binary, stored.connection.auth)
            .map_err(Unrunnable::new)?;
        let defaults = CodexConfig::default();
        Ok(ready.with_config(CodexConfig {
            model: pin.model.clone(),
            timeout: pin
                .limits
                .seconds
                .map_or(defaults.timeout, |s| Duration::from_secs(u64::from(s))),
        }))
    }

    /// A Codex that may be run with `auth`, as it was found a moment ago or is found now: a
    /// version a person verified, nothing on that nobody looked at, a credential to give it, and a
    /// way of signing in that the build uses. What is found of one program and one credential is
    /// not what another would give.
    fn codex_ready(
        &self,
        launch: &CodexLaunch,
        binary: PathBuf,
        auth: AuthSource,
    ) -> Result<Codex, String> {
        let key = (binary, auth);
        let mut kept = self.codex_seen.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((of, at, found)) = kept.as_ref() {
            if *of == key && at.elapsed() < OBSERVED_FOR {
                return found.clone();
            }
        }
        let found = self.find_codex(launch, &key.0, auth);
        *kept = Some((key, Instant::now(), found.clone()));
        found
    }

    fn find_codex(
        &self,
        launch: &CodexLaunch,
        binary: &Path,
        auth: AuthSource,
    ) -> Result<Codex, String> {
        let codex = Codex::new(
            binary.to_path_buf(),
            launch.author.clone(),
            CodexConfig::default(),
            auth,
            launch.app_home.clone(),
            launch.support.clone(),
        )
        .with_environment(launch.environment.clone());
        if let Some(why) = codex.why_not() {
            return Err(why);
        }
        // A key from the environment is the credential whatever a stored login says, so nobody is
        // asked who is signed in.
        let observed = if auth == AuthSource::EnvApiKey {
            None
        } else {
            codex.observe_login().map_err(|_| {
                "Codex could not be asked who is signed in: check it, then ask again".to_string()
            })?
        };
        let Some(route) = Route::of(AdapterKind::Codex, auth, observed) else {
            return Err(
                "nobody is signed in to Codex: run `codex login` in a terminal (a ChatGPT plan, or \
                 an API key), then ask again"
                    .to_string(),
            );
        };
        if let Decision::Refuse { reason, .. } = self.policy.decide(route) {
            return Err(refusal_words(AdapterKind::Codex, route, reason));
        }
        Ok(codex)
    }

    /// Who is signed in to `binary`, as it said a moment ago or says now. What was said of
    /// another program is not what this one would say.
    fn observe(&self, binary: &Path) -> Observation {
        let mut kept = self.observed.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((of, at, said)) = kept.as_ref() {
            if of == binary && at.elapsed() < OBSERVED_FOR {
                return said.clone();
            }
        }
        let said = observe_auth(binary);
        *kept = Some((binary.to_path_buf(), Instant::now(), said.clone()));
        said
    }
}

impl Directory for Connections {
    fn generator_for(&self, pin: &Pin) -> Result<Arc<dyn Generator>, Unrunnable> {
        match pin.adapter {
            AdapterKind::ClaudeCode => self
                .claude_for(pin)
                .map(|client| Arc::new(client) as Arc<dyn Generator>),
            AdapterKind::Codex => self
                .codex_for(pin)
                .map(|client| Arc::new(client) as Arc<dyn Generator>),
            other => Err(no_adapter(other)),
        }
    }
}

fn claude_not_found() -> Unrunnable {
    Unrunnable::new(
        "Claude Code was not found on this computer: install it, or set SCE_CLAUDE to its path",
    )
}

fn codex_not_found() -> Unrunnable {
    Unrunnable::new(
        "Codex was not found on this computer: install it, or set SCE_CODEX to its path",
    )
}

fn no_adapter(adapter: AdapterKind) -> Unrunnable {
    Unrunnable::new(format!(
        "this build has no adapter for `{}` connections yet, so a request for one waits: choose \
         another connection, or ask from an AI client of your own",
        adapter.word()
    ))
}

/// Why a way of signing in is not used, in the words a person reads, and what to do instead in the
/// client that was asked.
fn refusal_words(adapter: AdapterKind, route: Route, reason: Reason) -> String {
    let word = serde_json::to_value(route)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default();
    let instead = match adapter {
        AdapterKind::Codex => {
            "with `codex login` (a ChatGPT plan) or `codex login --with-api-key`, or set \
             CODEX_API_KEY and choose that"
        }
        _ => "with `claude auth login` or `claude auth login --console`",
    };
    match reason {
        Reason::Forbidden => format!("signing in this way ({word}) is not allowed"),
        Reason::Unconfirmed => format!(
            "signing in this way ({word}) is not one this build uses: its terms have not been \
             checked. Sign in another way, {instead}"
        ),
        Reason::SwitchedOff => {
            format!("signing in this way ({word}) is switched off in this build")
        }
    }
}
