// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a shell does to host the application's own executor: find the pieces, start a runner
//! on a thread of its own, and stop it with the window.
//!
//! The desktop application and the browser shell used while developing the screen both call
//! [`start`], so that what they host cannot differ for a reason that lives in a shell. A shell
//! that cannot find what the executor needs does not fail: it says in words what it looked for
//! ([`NotHosted`]), hosts nothing, and the application shows "no AI connected" and works as it
//! always did. The owner is never made to have Claude Code to use the workbench.
//!
//! The pieces, found the way the product's generator is (the environment, then beside the
//! program, then the search path):
//!
//! | what | the environment | beside the program |
//! |---|---|---|
//! | Claude Code | `SCE_CLAUDE` | on the search path as `claude` |
//! | the authoring server's launcher | `SCE_AUTHOR_MCP` | `sce-author-mcp` |
//! | `sce-work`, for the authoring server | `SCE_WORK` | `sce-work` |
//! | the product, for the authoring server | `SCE_CODEGEN` | `sce-codegen` |
//!
//! `SCE_EXECUTOR=off` hosts nothing, `SCE_CLAUDE_MODEL` names the model a run uses and
//! `SCE_CLAUDE_BUDGET_USD` bounds what one run may cost.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::auth_policy::Policy;
use crate::claude_code::{capture, AuthorServer, ClaudeCode, ClaudeCodeConfig};
use crate::clock::Clock;
use crate::directory::{ClaudeLaunch, Connections};
use crate::figures::{SceCodegen, GENERATOR_ENV};
use crate::installed::Installed;
use crate::review::Product;
use crate::runner::{Cancel, Directory, Generator, Runner, RunnerConfig, Waiting};
use crate::store::{ConnectionStore, HostReport, HostWaiting, WorkStore, WAITING_MAX};

/// Names `SCE_EXECUTOR`, `SCE_CLAUDE`, `SCE_AUTHOR_MCP`, `SCE_WORK`, `SCE_CLAUDE_MODEL`,
/// `SCE_CLAUDE_BUDGET_USD` in the environment.
const EXECUTOR_ENV: &str = "SCE_EXECUTOR";
pub(crate) const CLAUDE_ENV: &str = "SCE_CLAUDE";
const AUTHOR_ENV: &str = "SCE_AUTHOR_MCP";
const WORK_ENV: &str = "SCE_WORK";
const MODEL_ENV: &str = "SCE_CLAUDE_MODEL";
const BUDGET_ENV: &str = "SCE_CLAUDE_BUDGET_USD";

/// What a shell is told about the executor it may host.
#[derive(Debug, Clone)]
pub struct HostSettings {
    /// The name its requests are taken under and the adapter is reported as (`desktop`).
    pub name: String,
    pub enabled: bool,
    pub claude: Option<PathBuf>,
    pub author: Option<PathBuf>,
    pub work: Option<PathBuf>,
    pub codegen: Option<PathBuf>,
    pub config: ClaudeCodeConfig,
}

impl HostSettings {
    /// The settings the environment of this process gives.
    pub fn from_environment(name: &str) -> Self {
        Self::from_lookup(name, |key| std::env::var_os(key))
    }

    /// The settings `get` gives for each name: the environment, or a map a test holds.
    pub fn from_lookup(name: &str, get: impl Fn(&str) -> Option<OsString>) -> Self {
        let text = |key: &str| {
            get(key)
                .filter(|value| !value.is_empty())
                .and_then(|value| value.into_string().ok())
        };
        let path = |key: &str| get(key).filter(|v| !v.is_empty()).map(PathBuf::from);
        let enabled = !text(EXECUTOR_ENV).is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "off" | "0" | "false" | "no"
            )
        });
        let config = ClaudeCodeConfig {
            model: text(MODEL_ENV),
            // A budget is a positive, finite number: anything else is no budget, which a person
            // reads as "I did not say", and not as a limit of zero that stops every run.
            max_budget_usd: text(BUDGET_ENV)
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|dollars| dollars.is_finite() && *dollars > 0.0),
            ..ClaudeCodeConfig::default()
        };
        HostSettings {
            name: name.to_string(),
            enabled,
            claude: path(CLAUDE_ENV),
            author: path(AUTHOR_ENV),
            work: path(WORK_ENV),
            codegen: path(GENERATOR_ENV),
            config,
        }
    }
}

impl HostSettings {
    /// These settings with the programs of an installed bundle where the environment named none: what
    /// an installer carried is what is used, and a developer's word (a variable set) still wins.
    pub fn with_bundle(mut self, bundle: &Installed) -> Self {
        self.author = self.author.or_else(|| bundle.author.clone());
        self.work = self.work.or_else(|| bundle.work.clone());
        self.codegen = self.codegen.or_else(|| bundle.codegen.clone());
        self
    }
}

/// Why a shell hosts no executor, in words a person can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotHosted {
    /// The owner turned it off.
    Off,
    /// No Claude Code was found; says where it looked.
    NoClaude(String),
    /// No launcher for the authoring server was found; says where it looked.
    NoAuthorServer(String),
    /// The launcher is there and the server would not start (no Python, a Python module it needs,
    /// no generator); says what the server itself said when it was asked.
    AuthorServerNotReady(String),
}

impl std::fmt::Display for NotHosted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotHosted::Off => write!(
                f,
                "the application's own executor is off ({EXECUTOR_ENV} says so): requests wait \
                 for an authoring client of your own"
            ),
            NotHosted::NoClaude(tried) => write!(
                f,
                "no Claude Code to write models with ({tried}): install it, or set {CLAUDE_ENV} \
                 to its path; until then requests wait for an authoring client of your own"
            ),
            NotHosted::NoAuthorServer(tried) => write!(
                f,
                "no launcher for the SCE authoring server ({tried}): set {AUTHOR_ENV} to it \
                 (a checkout has scripts/sce_author_mcp.sh, an installed bundle bin/sce-author-mcp)"
            ),
            NotHosted::AuthorServerNotReady(said) => write!(
                f,
                "the SCE authoring server cannot start, so the AI could not check what it writes: \
                 {said}"
            ),
        }
    }
}

impl std::error::Error for NotHosted {}

/// How often a shell says again what it is doing: a third of the time a word counts for, so that
/// one that is late is not read as one that went.
const REPORT_EVERY: Duration = Duration::from_secs(30);

/// What a shell hosts: an executor on a thread of its own, or nothing and the reason. Either way it
/// says so, again and again, where the owner's screen reads it; dropping it stops both threads,
/// kills a client at work and waits for them, so closing the window does not leave a client
/// running.
pub struct ExecutorHost {
    shutdown: Cancel,
    threads: Vec<thread::JoinHandle<()>>,
    version: Option<String>,
    not_hosted: Option<NotHosted>,
}

impl ExecutorHost {
    /// What the client says its version is; none when no client is running.
    pub fn client_version(&self) -> Option<String> {
        self.version.clone()
    }

    /// Why nothing is hosted, when nothing is.
    pub fn not_hosted(&self) -> Option<&NotHosted> {
        self.not_hosted.as_ref()
    }

    /// Stop now; the same as dropping.
    pub fn stop(self) {
        drop(self);
    }
}

impl Drop for ExecutorHost {
    fn drop(&mut self) {
        self.shutdown.cancel();
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

/// Host an executor for `store` when what it needs can be found, and say what came of it.
///
/// Never fails: that nothing could be hosted is a state a screen shows ("no AI is connected, and
/// here is what to install or set") and not an error that stops the application.
pub fn start<C>(
    store: Arc<WorkStore<C>>,
    product: Arc<dyn Product>,
    settings: HostSettings,
) -> ExecutorHost
where
    C: Clock + Send + Sync + 'static,
{
    start_with(store, product, settings, None)
}

/// The same, for a shell that has the person's settings: the executor then also runs the requests
/// that were made for a connection, for the connections it can run (the settings folder, and
/// the build's table of ways of signing in), and leaves them to the executor of their own
/// connection when it cannot. Without them it runs only the requests nobody chose a connection
/// for, and leaves the rest alone.
pub fn start_with<C>(
    store: Arc<WorkStore<C>>,
    product: Arc<dyn Product>,
    settings: HostSettings,
    connections: Option<(ConnectionStore, Policy)>,
) -> ExecutorHost
where
    C: Clock + Send + Sync + 'static,
{
    let name = settings.name.clone();
    // One word stops everything the host runs: the runner's own, when there is a runner.
    let mut shutdown = Cancel::new();
    let mut threads = Vec::new();
    // What the runner left queued and could not run: asked of it as often as the report looks, and
    // nothing when there is no runner.
    let mut left: Box<dyn Fn() -> Vec<HostWaiting> + Send> = Box::new(Vec::new);
    let (version, not_hosted) = match find_client(&store, &settings) {
        Ok(client) => {
            let version = client.version();
            let directory = connections.map(|(settings, policy)| {
                let launch = ClaudeLaunch {
                    binary: client.binary().to_path_buf(),
                    author: client.author().clone(),
                };
                Arc::new(Connections::new(settings, policy, Some(launch))) as Arc<dyn Directory>
            });
            let mut runner = Runner::new(
                Arc::clone(&store),
                product,
                Arc::new(client),
                RunnerConfig::named(&name),
            );
            if let Some(directory) = directory {
                runner = runner.with_connections(directory);
            }
            shutdown = runner.shutdown();
            let runner = Arc::new(runner);
            left = Box::new({
                let runner = Arc::clone(&runner);
                move || waiting_words(&runner.waiting())
            });
            let spawned = thread::Builder::new()
                .name("sce-executor".to_string())
                .spawn(move || runner.run());
            match spawned {
                Ok(thread) => {
                    threads.push(thread);
                    (version, None)
                }
                Err(e) => (
                    None,
                    Some(NotHosted::NoClaude(format!(
                        "a thread for the executor: {e}"
                    ))),
                ),
            }
        }
        Err(why) => (None, Some(why)),
    };
    // What is said of it, said now (so that it is in the works folder when this returns) and kept
    // said.
    let reason = not_hosted.as_ref().map(ToString::to_string);
    let said = (version.clone(), reason);
    say(&store, &name, &said.0, said.1.as_deref(), &left());
    if let Ok(thread) = thread::Builder::new()
        .name("sce-host-report".to_string())
        .spawn({
            let (store, shutdown, name) = (Arc::clone(&store), shutdown.clone(), name.clone());
            move || {
                report_until_stopped(&store, &shutdown, &name, &said.0, said.1.as_deref(), &*left)
            }
        })
    {
        threads.push(thread);
    }
    ExecutorHost {
        shutdown,
        threads,
        version,
        not_hosted,
    }
}

/// What a shell needs to host an executor, found; or why it is not.
fn find_client<C: Clock>(
    store: &WorkStore<C>,
    settings: &HostSettings,
) -> Result<ClaudeCode, NotHosted> {
    if !settings.enabled {
        return Err(NotHosted::Off);
    }
    let author = author_server(settings, store.root())?;
    let config = settings.config.clone();
    let client = match &settings.claude {
        Some(binary) => {
            let client = ClaudeCode::new(binary.clone(), author.clone(), config);
            if client.version().is_none() {
                return Err(NotHosted::NoClaude(format!(
                    "{} did not answer --version",
                    binary.display()
                )));
            }
            client
        }
        None => ClaudeCode::find(author.clone(), config)
            .ok_or_else(|| NotHosted::NoClaude("`claude` is not on the search path".to_string()))?,
    };
    // Last: a missing client is the plainer thing to say, and the server is asked only when there
    // is a client to give it to.
    ready(&author)?;
    Ok(client)
}

/// Say what the shell is doing now, and again every [`REPORT_EVERY`] or as soon as what it
/// waits for changes, until told to stop: a person who pressed the button is told why nothing
/// happens when it is true and not half a minute later.
fn report_until_stopped<C: Clock>(
    store: &WorkStore<C>,
    stop: &Cancel,
    name: &str,
    version: &Option<String>,
    reason: Option<&str>,
    left: &dyn Fn() -> Vec<HostWaiting>,
) {
    // Said once already, by `start`: the next is due a period from then.
    let mut last = Instant::now();
    let mut said = left();
    while !stop.is_cancelled() {
        let now = left();
        if now != said || last.elapsed() >= REPORT_EVERY {
            say(store, name, version, reason, &now);
            said = now;
            last = Instant::now();
        }
        thread::sleep(Duration::from_millis(25));
    }
}

/// One report. A folder that could not be written to this time is written to at the next.
fn say<C: Clock>(
    store: &WorkStore<C>,
    name: &str,
    version: &Option<String>,
    reason: Option<&str>,
    waiting: &[HostWaiting],
) {
    let _ = store.report_host(HostReport {
        name,
        hosting: reason.is_none(),
        reason,
        client_version: version.as_deref(),
        waiting,
    });
}

/// What the runner left queued, as a shell says it: the oldest [`WAITING_MAX`] of them, each as the
/// ids and the sentence and nothing else.
fn waiting_words(waiting: &[Waiting]) -> Vec<HostWaiting> {
    waiting
        .iter()
        .take(WAITING_MAX)
        .map(|w| HostWaiting {
            work: w.work.as_str().to_string(),
            request: w.request.clone(),
            connection: w.connection.as_str().to_string(),
            reason: w.reason.clone(),
        })
        .collect()
}

/// How the client reaches the authoring server: the launcher, and what the server needs to find
/// the works folder and the product, which are this shell's and no other's.
fn author_server(settings: &HostSettings, works: &Path) -> Result<AuthorServer, NotHosted> {
    let launcher = match &settings.author {
        Some(path) if path.is_file() => path.clone(),
        Some(path) => {
            return Err(NotHosted::NoAuthorServer(format!(
                "{} is not a file",
                path.display()
            )))
        }
        None => found_beside_or_on_path("sce-author-mcp").ok_or_else(|| {
            NotHosted::NoAuthorServer(
                "`sce-author-mcp` is not beside the program or on the search path".to_string(),
            )
        })?,
    };
    let mut env = vec![("SCE_WORKS_DIR".to_string(), works.display().to_string())];
    let work = settings
        .work
        .clone()
        .or_else(|| found_beside_or_on_path("sce-work"));
    if let Some(work) = work {
        env.push((WORK_ENV.to_string(), work.display().to_string()));
    }
    let codegen = settings
        .codegen
        .clone()
        .or_else(|| SceCodegen::discover().map(|found| found.program().to_path_buf()));
    if let Some(codegen) = codegen {
        env.push((GENERATOR_ENV.to_string(), codegen.display().to_string()));
    }
    Ok(AuthorServer {
        command: launcher,
        args: Vec::new(),
        env,
    })
}

/// Ask the authoring server whether it can do its work (`--check`), with the environment the
/// client will give it. From outside, a server that cannot start is an AI that does not answer;
/// asked first, what is missing (Python, a module it needs, the generator) is said in the server's
/// own words where the owner reads it. The server is the authority on what it needs, so nothing is
/// copied here.
fn ready(server: &AuthorServer) -> Result<(), NotHosted> {
    let mut command = std::process::Command::new(&server.command);
    command.arg("--check").envs(server.env.iter().cloned());
    let outcome = capture(command, Duration::from_secs(30));
    match outcome {
        Ok(done) if done.status.success() => Ok(()),
        Ok(done) => {
            let said = if done.stderr.trim().is_empty() {
                done.stdout
            } else {
                done.stderr
            };
            Err(NotHosted::AuthorServerNotReady(said.trim().to_string()))
        }
        Err(why) => Err(NotHosted::NoAuthorServer(format!(
            "{} could not be run: {why}",
            server.command.display()
        ))),
    }
}

/// A program of this installation: beside the running program, else on the search path.
fn found_beside_or_on_path(name: &str) -> Option<PathBuf> {
    let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(&file)))
        .filter(|path| path.is_file());
    beside.or_else(|| {
        let paths = std::env::var_os("PATH")?;
        std::env::split_paths(&paths)
            .map(|dir| dir.join(&file))
            .find(|path| path.is_file())
    })
}
