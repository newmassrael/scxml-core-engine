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
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::auth_policy::Policy;
use crate::claude_code::{AuthorServer, ClaudeCode, ClaudeCodeConfig};
use crate::client_run::capture;
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
    /// How long a shell that found something missing waits before it looks again. A person can
    /// install the client, or what the authoring server needs, while the window is open, and the
    /// executor must not wait for the window to be opened again to notice.
    pub retry: Duration,
}

/// How often a shell that could not host looks again: seldom enough that a missing program is
/// not a process started every moment, often enough that a person who has just installed it
/// is not left waiting.
const RETRY_EVERY: Duration = Duration::from_secs(10);

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
            retry: RETRY_EVERY,
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
    shared: Arc<Shared>,
}

/// What a host is doing now: which client runs, or why none does. Said by the thread that looks
/// for what the executor needs, and read by the shell that started it and by the thread that
/// reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Standing {
    version: Option<String>,
    not_hosted: Option<NotHosted>,
}

impl Standing {
    fn of(found: &Result<ClaudeCode, NotHosted>) -> Standing {
        match found {
            Ok(client) => Standing {
                version: client.version(),
                not_hosted: None,
            },
            Err(why) => Standing {
                version: None,
                not_hosted: Some(why.clone()),
            },
        }
    }
}

/// What the threads of a host share.
struct Shared {
    standing: Mutex<Standing>,
    /// What the runner left queued and could not run, asked of it as often as the report looks:
    /// nothing until there is a runner.
    left: Mutex<Box<dyn Fn() -> Vec<HostWaiting> + Send>>,
}

impl Shared {
    fn new(standing: Standing) -> Shared {
        Shared {
            standing: Mutex::new(standing),
            left: Mutex::new(Box::new(Vec::new)),
        }
    }

    fn standing(&self) -> Standing {
        self.standing
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn set(&self, standing: Standing) {
        *self.standing.lock().unwrap_or_else(|e| e.into_inner()) = standing;
    }

    fn waiting(&self) -> Vec<HostWaiting> {
        (self.left.lock().unwrap_or_else(|e| e.into_inner()))()
    }
}

impl ExecutorHost {
    /// What the client says its version is; none when no client is running.
    pub fn client_version(&self) -> Option<String> {
        self.shared.standing().version
    }

    /// Why nothing is hosted, when nothing is. It can change while the window is open: a host
    /// that could not start goes on looking, and hosts once what it needed is there.
    pub fn not_hosted(&self) -> Option<NotHosted> {
        self.shared.standing().not_hosted
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
    // One word stops everything the host runs, the runner included.
    let shutdown = Cancel::new();
    let mut threads = Vec::new();
    // The first look is made now, so that what it found is in the works folder when this returns.
    let first = find_client(&store, &settings);
    let shared = Arc::new(Shared::new(Standing::of(&first)));
    say(&store, &name, &shared.standing(), &[]);
    let supervising = thread::Builder::new()
        .name("sce-executor".to_string())
        .spawn({
            let (store, shutdown, shared) =
                (Arc::clone(&store), shutdown.clone(), Arc::clone(&shared));
            move || {
                supervise(
                    store,
                    product,
                    settings,
                    connections,
                    shutdown,
                    shared,
                    first,
                )
            }
        });
    match supervising {
        Ok(thread) => threads.push(thread),
        Err(e) => shared.set(Standing {
            version: None,
            not_hosted: Some(NotHosted::NoClaude(format!(
                "a thread for the executor: {e}"
            ))),
        }),
    }
    if let Ok(thread) = thread::Builder::new()
        .name("sce-host-report".to_string())
        .spawn({
            let (store, shutdown, shared) =
                (Arc::clone(&store), shutdown.clone(), Arc::clone(&shared));
            move || report_until_stopped(&store, &shutdown, &name, &shared)
        })
    {
        threads.push(thread);
    }
    ExecutorHost {
        shutdown,
        threads,
        shared,
    }
}

/// Host the executor once what it needs is there, and until told to stop.
///
/// `found` is the first look. What was missing then (a client not yet installed, an authoring
/// server that was not ready) is looked for again every `settings.retry`, because a person can
/// put it right while the window is open and the executor has to notice without the window being
/// opened again. The owner's word that there is to be no executor is the one thing that does not
/// come right: that is not looked for again.
fn supervise<C>(
    store: Arc<WorkStore<C>>,
    product: Arc<dyn Product>,
    settings: HostSettings,
    connections: Option<(ConnectionStore, Policy)>,
    shutdown: Cancel,
    shared: Arc<Shared>,
    found: Result<ClaudeCode, NotHosted>,
) where
    C: Clock + Send + Sync + 'static,
{
    let mut found = found;
    loop {
        let client = match found {
            Ok(client) => client,
            Err(why) => {
                let owners_word = why == NotHosted::Off;
                shared.set(Standing {
                    version: None,
                    not_hosted: Some(why),
                });
                let wait = if owners_word {
                    Duration::MAX
                } else {
                    settings.retry
                };
                if !wait_or_stop(&shutdown, wait) {
                    return;
                }
                found = find_client(&store, &settings);
                continue;
            }
        };
        shared.set(Standing {
            version: client.version(),
            not_hosted: None,
        });
        let directory = connections.map(|(settings_store, policy)| {
            let launch = ClaudeLaunch {
                binary: client.binary().to_path_buf(),
                author: client.author().clone(),
                max_budget_usd: settings.config.max_budget_usd,
            };
            Arc::new(Connections::new(settings_store, policy, Some(launch))) as Arc<dyn Directory>
        });
        let mut runner = Runner::new(
            Arc::clone(&store),
            product,
            Arc::new(client),
            RunnerConfig::named(&settings.name),
        )
        .with_shutdown(shutdown);
        if let Some(directory) = directory {
            runner = runner.with_connections(directory);
        }
        let runner = Arc::new(runner);
        *shared.left.lock().unwrap_or_else(|e| e.into_inner()) = Box::new({
            let runner = Arc::clone(&runner);
            move || waiting_words(&runner.waiting())
        });
        runner.run();
        return;
    }
}

/// Wait for `span`, or until told to stop; whether it was the span that ended.
fn wait_or_stop(stop: &Cancel, span: Duration) -> bool {
    let until = Instant::now().checked_add(span);
    while !stop.is_cancelled() {
        if until.is_some_and(|until| Instant::now() >= until) {
            return true;
        }
        thread::sleep(Duration::from_millis(25));
    }
    false
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
    shared: &Shared,
) {
    // Said once already, by `start`: the next is due a period from then.
    let mut last = Instant::now();
    let mut said = (shared.standing(), shared.waiting());
    while !stop.is_cancelled() {
        let now = (shared.standing(), shared.waiting());
        if now != said || last.elapsed() >= REPORT_EVERY {
            say(store, name, &now.0, &now.1);
            said = now;
            last = Instant::now();
        }
        thread::sleep(Duration::from_millis(25));
    }
}

/// One report. A folder that could not be written to this time is written to at the next.
fn say<C: Clock>(store: &WorkStore<C>, name: &str, standing: &Standing, waiting: &[HostWaiting]) {
    let reason = standing.not_hosted.as_ref().map(ToString::to_string);
    let _ = store.report_host(HostReport {
        name,
        hosting: reason.is_none(),
        reason: reason.as_deref(),
        client_version: standing.version.as_deref(),
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
