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

use crate::claude_code::{AuthorServer, ClaudeCode, ClaudeCodeConfig};
use crate::clock::Clock;
use crate::figures::{SceCodegen, GENERATOR_ENV};
use crate::review::Product;
use crate::runner::{Cancel, Generator, Runner, RunnerConfig};
use crate::store::WorkStore;

/// Names `SCE_EXECUTOR`, `SCE_CLAUDE`, `SCE_AUTHOR_MCP`, `SCE_WORK`, `SCE_CLAUDE_MODEL`,
/// `SCE_CLAUDE_BUDGET_USD` in the environment.
const EXECUTOR_ENV: &str = "SCE_EXECUTOR";
const CLAUDE_ENV: &str = "SCE_CLAUDE";
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

/// Why a shell hosts no executor, in words a person can act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotHosted {
    /// The owner turned it off.
    Off,
    /// No Claude Code was found; says where it looked.
    NoClaude(String),
    /// No launcher for the authoring server was found; says where it looked.
    NoAuthorServer(String),
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
        }
    }
}

impl std::error::Error for NotHosted {}

/// A runner on a thread of its own, for as long as it is held. Dropping it tells the runner to
/// stop, kills a client at work, and waits for the thread: closing the window does not leave a
/// client running.
pub struct ExecutorHost {
    shutdown: Cancel,
    thread: Option<thread::JoinHandle<()>>,
    version: Option<String>,
}

impl ExecutorHost {
    /// What the client says its version is.
    pub fn client_version(&self) -> Option<String> {
        self.version.clone()
    }

    /// Stop now; the same as dropping.
    pub fn stop(self) {
        drop(self);
    }
}

impl Drop for ExecutorHost {
    fn drop(&mut self) {
        self.shutdown.cancel();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Host an executor for `store`, if what it needs can be found.
pub fn start<C>(
    store: Arc<WorkStore<C>>,
    product: Arc<dyn Product>,
    settings: HostSettings,
) -> Result<ExecutorHost, NotHosted>
where
    C: Clock + Send + Sync + 'static,
{
    if !settings.enabled {
        return Err(NotHosted::Off);
    }
    let author = author_server(&settings, store.root())?;
    let config = settings.config.clone();
    let client = match &settings.claude {
        Some(binary) => {
            let client = ClaudeCode::new(binary.clone(), author, config);
            if client.version().is_none() {
                return Err(NotHosted::NoClaude(format!(
                    "{} did not answer --version",
                    binary.display()
                )));
            }
            client
        }
        None => ClaudeCode::find(author, config)
            .ok_or_else(|| NotHosted::NoClaude("`claude` is not on the search path".to_string()))?,
    };
    let version = client.version();
    let runner = Runner::new(
        store,
        product,
        Arc::new(client),
        RunnerConfig::named(settings.name),
    );
    let shutdown = runner.shutdown();
    let thread = thread::Builder::new()
        .name("sce-executor".to_string())
        .spawn(move || runner.run())
        .map_err(|e| NotHosted::NoClaude(format!("a thread for the executor: {e}")))?;
    Ok(ExecutorHost {
        shutdown,
        thread: Some(thread),
        version,
    })
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
